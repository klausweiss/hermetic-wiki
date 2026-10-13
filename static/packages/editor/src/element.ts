// <hermetic-editor>: inserted by `hermetic serve` into pages it serves to a browser on
// the same machine. It turns the page's own blocks into a Logseq-style outliner: click
// a block to edit its markdown, use the outliner keys, and changes are saved to the
// note file. The page is updated in place from each rebuild instead of reloading.

import { EDITING_ATTR, EDIT_PROPERTIES, OPEN_TODAY, PAGES_CHANGED, carrySidebar, icon, readEnv, whenParsed } from "@hermetic/core";

import { Conflict, type Refused, SaveQueue, type SaveState, SourceApi } from "./api";
import { type Doc, type Node, type Payload, flatten, fromPayload, newNode } from "./model";
import { MARKERS, applied, toggle } from "./format";
import * as ops from "./ops";
import { pageProps, setPageProps } from "./props";
import { editProperties } from "./propsdialog";
import { sameText, serialize } from "./serialize";

/** How long after the last keystroke a change is saved. */
export const IDLE_SAVE_MS = 1000;
/** How long "Saved" stays in the corner; "Not saved" stays for as long as it is true. */
export const SAVED_SHOWN_MS = 6000;
/** sessionStorage key: the site path whose first block to edit once it has loaded. */
const START_KEY = "hermetic-editor-start";
/** How long after it is recorded a page starts editing when it loads. */
const START_MS = 5000;

interface Rendered {
  /** The body the server rendered this element for. */
  body: string;
  el: HTMLElement;
  className: string;
}

interface Active {
  node: Node;
  area: HTMLTextAreaElement;
}

/** Extension points for autocomplete and the bullet menu (set up by other modules). */
export interface EditorPlugin {
  /** Called on every input in the block being edited. */
  input?(editor: HermeticEditor, area: HTMLTextAreaElement): void;
  /** Called first on keydown; returning true means the key was handled. */
  key?(editor: HermeticEditor, e: KeyboardEvent, area: HTMLTextAreaElement): boolean;
  /** Called when editing of a block ends. */
  end?(editor: HermeticEditor): void;
  /** Adds controls to a block's element (e.g. a bullet menu). */
  decorate?(editor: HermeticEditor, li: HTMLLIElement, node: Node): void;
}

const plugins: EditorPlugin[] = [];

export function addPlugin(plugin: EditorPlugin): void {
  plugins.push(plugin);
}

export class HermeticEditor extends HTMLElement {
  root!: URL;
  api!: SourceApi;
  /** Site path of the page being edited; empty when the page is not editable. */
  path = "";
  doc: Doc | null = null;
  private queue: SaveQueue | null = null;
  /** The page's text as last loaded or saved. */
  private stored = "";
  private rendered = new WeakMap<Node, Rendered>();
  private nodes = new WeakMap<Element, Node>();
  private active: Active | null = null;
  private idle = 0;
  private statusFade = 0;
  private section: HTMLElement | null = null;
  private status: HTMLElement | null = null;
  private ready = false;

  connectedCallback(): void {
    whenParsed(() => void this.setup());
  }

  private async setup(): Promise<void> {
    const env = readEnv();
    if (this.ready || !env.interactive) return;
    this.ready = true;
    this.root = env.root;
    this.api = new SourceApi(env.root);
    this.addHeaderControls();
    this.watchTaskLists();
    document.addEventListener(OPEN_TODAY, () => this.openTodayNow());

    const path = sitePath(location.pathname, env.root.pathname);
    const article = document.querySelector<HTMLElement>("main > article.page");
    if (article?.dataset.view === "home") this.offerHomePage(article);
    if (!path || !article || article.classList.contains("index")) return;
    let payload: Payload;
    try {
      payload = await this.api.get(path);
    } catch {
      return; // not an editable page
    }
    this.path = path;
    let section = article.querySelector<HTMLElement>(":scope > section.content");
    const rendered = section;
    if (!section) {
      section = document.createElement("section");
      section.className = "content";
      const empty = article.querySelector(":scope > p.empty");
      if (empty) empty.replaceWith(section);
      else article.querySelector(":scope > header")?.after(section);
    }
    this.section = section;
    section.addEventListener("click", (e) => this.onClick(e));
    section.addEventListener("keydown", (e) => {
      if ((e.key === "Enter" || e.key === " ") && (e.target as Element).matches(".task-marker[role=button]")) {
        e.preventDefault();
        this.labelClicked(e.target as HTMLElement);
      }
    });
    // A label click must not take the focus from the block being edited.
    section.addEventListener("mousedown", (e) => {
      if ((e.target as Element).closest(".task-marker[role=button]")) e.preventDefault();
    });
    document.documentElement.setAttribute(EDITING_ATTR, "");
    document.addEventListener(EDIT_PROPERTIES, () => this.editProperties());

    this.queue = new SaveQueue(this.api, path, payload.version, {
      text: () => serialize(this.doc!),
      saved: (p, text) => this.onSaved(p, text),
      conflict: (p, text) => void this.onConflict(p, text),
      failed: (e, _text) => this.onFailed(e),
      state: (s) => this.showState(s),
    });
    this.resync(payload, rendered);
    // The status stays empty, and so hidden, until the first edit.

    document.addEventListener(PAGES_CHANGED, (e) => {
      const { changed, removed } = (e as CustomEvent<{ changed: string[]; removed: string[] }>).detail;
      if ([...changed, ...removed].includes(this.path)) void this.refresh();
      this.queue?.retryNow();
    });
    addEventListener("beforeunload", (e) => {
      if (this.unsaved()) {
        e.preventDefault();
        e.returnValue = "";
      }
    });
    addEventListener("online", () => this.queue?.retryNow());
    // Return starts editing at the end of the page, when nothing else takes the key.
    addEventListener("keydown", (e) => {
      if (e.key !== "Enter" || e.defaultPrevented || e.isComposing || this.active || !this.doc) return;
      if (e.shiftKey || e.ctrlKey || e.metaKey || e.altKey || isControl(e.target)) return;
      if (document.querySelector("dialog[open]")) return;
      e.preventDefault();
      const all = flatten(this.doc);
      const last = all[all.length - 1]?.node;
      if (last) {
        this.edit(last, last.body.length);
      } else {
        this.startFirst();
      }
    });

    try {
      // Kept for a few seconds rather than removed, so a page that is loaded twice in a
      // row (e.g. reloaded by the browser) still starts editing.
      const start = JSON.parse(sessionStorage.getItem(START_KEY) ?? "null") as { path: string; at: number } | null;
      if (start?.path === path && Date.now() - start.at < START_MS) this.startFirst();
    } catch {
      // Storage unavailable: just do not start editing.
    }
  }

  /** Whether changes are not yet stored in the file. */
  unsaved(): boolean {
    return (this.doc !== null && !sameText(serialize(this.doc), this.stored)) || (this.queue?.dirty ?? false);
  }

  // --- Header: "Today" and the save status ------------------------------------------

  private addHeaderControls(): void {
    const nav = document.querySelector(".site-header .site-nav");
    if (!nav || nav.querySelector(".today")) return;
    const today = document.createElement("a");
    today.className = "today";
    today.textContent = "Today";
    const path = `journals/${localDate(new Date())}.html`;
    today.href = new URL(path, this.root).href;
    today.addEventListener("click", (e) => {
      if (e.button !== 0 || e.shiftKey || e.ctrlKey || e.metaKey || e.altKey) return;
      e.preventDefault();
      void this.openToday(path);
    });
    this.status = document.createElement("span");
    this.status.className = "edit-status";
    this.status.setAttribute("role", "status");
    nav.prepend(today);
    nav.querySelector("hermetic-search")?.before(this.status);
  }

  /** On the default home page: "Write a home page" creates the root `index.md`. */
  private offerHomePage(article: HTMLElement): void {
    const hint = article.querySelector<HTMLElement>("p.hint");
    if (!hint || article.querySelector(".write-home")) return;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "write-home";
    button.textContent = "Write a home page";
    button.addEventListener("click", async () => {
      button.disabled = true;
      startAfterLoad("index.html");
      holdLiveReload();
      try {
        await this.api.put("index.html", "missing", "-\n");
      } catch (e) {
        if (!(e instanceof Conflict)) {
          startAfterLoad(null);
          button.disabled = false;
          button.textContent = "Could not create index.md; try again";
          return;
        }
      }
      location.reload();
    });
    const p = document.createElement("p");
    p.className = "hint";
    p.append(button, " It is saved as index.md in the notes folder and replaces this page.");
    hint.replaceWith(p);
  }

  /** Opens today's journal (the "Today" control and the `g d` keys). */
  openTodayNow(): void {
    void this.openToday(`journals/${localDate(new Date())}.html`);
  }

  private async openToday(path: string): Promise<void> {
    try {
      // Start writing only in a day without content; otherwise just show it. A day
      // without a file gets one with the first save that has content.
      const p = await this.api.get(path);
      if (p.blocks.every((b) => b.body.trim() === "")) startAfterLoad(path);
    } catch {
      // Open it anyway; the page tells what is wrong.
    }
    location.href = carrySidebar(new URL(path, this.root).href, location.href);
  }

  private showState(state: SaveState): void {
    if (!this.status || !this.path) return;
    this.status.dataset.state = state;
    this.status.textContent = state === "saved" ? "Saved" : state === "saving" ? "Saving…" : "Not saved";
    this.status.classList.remove("faded");
    clearTimeout(this.statusFade);
    if (state === "saved") {
      const status = this.status;
      this.statusFade = window.setTimeout(() => status.classList.add("faded"), SAVED_SHOWN_MS);
    }
  }

  // --- Loading and rendering ------------------------------------------------------------

  /**
   * Replaces the outline with `payload`. Rendered block bodies come from `html` (the
   * page's content section from the build) by anchor, or are kept from the previous
   * outline where a block's text is unchanged. Editing continues in the block at the
   * same position.
   */
  private resync(payload: Payload, html: Element | null): void {
    this.showPageMenu(payload);
    const before = this.doc ? flatten(this.doc).map((e) => e.node) : [];
    const activeIndex = this.active ? before.indexOf(this.active.node) : -1;
    this.doc = fromPayload(payload);
    // A journal day emptied while being written in has no file and no blocks: keep the
    // empty block being edited.
    if (this.doc.roots.length === 0 && this.active) this.doc.roots.push(newNode());
    this.stored = payload.text;
    if (this.queue) this.queue.version = payload.version;
    const after = flatten(this.doc).map((e) => e.node);
    after.forEach((node, i) => {
      const li = node.anchor ? html?.querySelector(`li.block[id="${CSS.escape(node.anchor)}"]`) : null;
      const body = li?.querySelector<HTMLElement>(":scope > .block-body");
      if (li && body) {
        this.rendered.set(node, { body: node.body, el: body, className: li.className });
        return;
      }
      const old = before[i] ? this.rendered.get(before[i]!) : undefined;
      if (old && old.body === node.body) this.rendered.set(node, old);
    });
    if (this.active) {
      const node = after[Math.min(activeIndex, after.length - 1)];
      if (node && activeIndex >= 0) {
        const { area } = this.active;
        const offset = Math.min(area.selectionStart, node.body.length);
        this.active.node = node;
        if (area.value !== node.body) area.value = node.body;
        this.render();
        this.focusAt(offset);
        return;
      }
      this.active = null;
    }
    this.render();
  }

  /** Re-reads the page after a rebuild, unless there are changes not yet saved. */
  private async refresh(): Promise<void> {
    if (!this.doc || this.unsaved()) return;
    const url = location.href.split("#")[0]!;
    const [payload, html] = await Promise.all([
      this.api.get(this.path).catch(() => null),
      fetch(url, { cache: "no-store" })
        .then((r) => (r.ok ? r.text() : null))
        .catch(() => null),
    ]);
    if (!payload || this.unsaved()) return;
    const fetched = html ? new DOMParser().parseFromString(html, "text/html").querySelector("main > article") : null;
    this.resync(payload, fetched?.querySelector(":scope > section.content") ?? null);
    if (fetched) this.replaceSections(fetched);
  }

  /** Replaces everything after the content section (references, tasks, …) with `article`'s. */
  private replaceSections(article: Element): void {
    const section = this.section;
    if (!section) return;
    while (section.nextSibling) section.nextSibling.remove();
    const fresh = article.querySelector(":scope > section.content") ?? article.querySelector(":scope > p.empty");
    let next = fresh?.nextSibling ?? null;
    while (next) {
      const node = next;
      next = next.nextSibling;
      section.parentElement!.append(document.adoptNode(node));
    }
  }

  render(): void {
    const doc = this.doc;
    const section = this.section;
    if (!doc || !section) return;
    if (doc.roots.length === 0) {
      const start = document.createElement("p");
      start.className = "empty editor-start";
      start.textContent = "Click here to start writing.";
      section.replaceChildren(start);
      return;
    }
    section.replaceChildren(this.tree(doc.roots));
    armLabels(section, ":scope li.block > .block-body > .task-marker");
  }

  private tree(nodes: Node[]): HTMLUListElement {
    const ul = document.createElement("ul");
    ul.className = "blocks";
    for (const node of nodes) {
      const li = document.createElement("li");
      const r = this.rendered.get(node);
      const fresh = r && r.body === node.body;
      li.className = fresh ? r.className : "block";
      if (fresh && node.anchor) li.id = node.anchor;
      if (this.active?.node === node) {
        li.classList.add("editing");
        li.append(this.active.area);
      } else if (fresh) {
        li.append(r.el);
      } else {
        const body = document.createElement("div");
        body.className = "block-body pending";
        body.textContent = node.body;
        li.append(body);
      }
      for (const p of plugins) p.decorate?.(this, li, node);
      if (node.children.length > 0) li.append(this.tree(node.children));
      this.nodes.set(li, node);
      ul.append(li);
    }
    return ul;
  }

  // --- The page menu ----------------------------------------------------------------------

  /**
   * The page's actions menu: a small menu button in the page header with "Page
   * properties" and, while the page has a file, "Delete page".
   */
  private showPageMenu(payload: Payload): void {
    const header = document.querySelector("main > article.page > header.page-header");
    if (!header) return;
    let menu = header.querySelector<HTMLElement>(".page-menu");
    if (!menu) {
      menu = document.createElement("div");
      menu.className = "page-menu";
      const toggle = document.createElement("button");
      toggle.type = "button";
      toggle.className = "page-menu-button";
      toggle.setAttribute("aria-label", "Page actions");
      toggle.title = "Page actions";
      toggle.setAttribute("aria-haspopup", "menu");
      toggle.setAttribute("aria-expanded", "false");
      toggle.append(icon("menu"));
      const items = document.createElement("div");
      items.className = "page-menu-items";
      items.setAttribute("role", "menu");
      items.hidden = true;
      const props = document.createElement("button");
      props.type = "button";
      props.className = "page-props-item";
      props.setAttribute("role", "menuitem");
      props.setAttribute("aria-keyshortcuts", "p");
      const key = document.createElement("kbd");
      key.textContent = "p";
      props.append(icon("list"), "Page properties", key);
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "page-delete";
      remove.setAttribute("role", "menuitem");
      remove.append(icon("trash-2"), "Delete page");
      items.append(props, remove);
      menu.append(toggle, items);
      header.append(menu);

      const close = (focusToggle: boolean) => {
        items.hidden = true;
        toggle.setAttribute("aria-expanded", "false");
        if (focusToggle) toggle.focus();
      };
      toggle.addEventListener("click", () => {
        const open = items.hidden;
        items.hidden = !open;
        toggle.setAttribute("aria-expanded", String(open));
        if (open) props.focus();
      });
      props.addEventListener("click", () => {
        close(false);
        this.editProperties();
      });
      remove.addEventListener("click", () => {
        close(false);
        void this.confirmDelete();
      });
      menu.addEventListener("keydown", (e) => {
        if (e.key === "Escape" && !items.hidden) {
          e.preventDefault();
          e.stopPropagation();
          close(true);
        } else if ((e.key === "ArrowDown" || e.key === "ArrowUp") && !items.hidden) {
          e.preventDefault();
          const shown = [...items.querySelectorAll<HTMLButtonElement>("button:not([hidden])")];
          const at = shown.indexOf(document.activeElement as HTMLButtonElement);
          const by = e.key === "ArrowDown" ? 1 : -1;
          shown[(at + by + shown.length) % shown.length]?.focus();
        }
      });
      document.addEventListener("click", (e) => {
        if (!items.hidden && !menu!.contains(e.target as globalThis.Node)) close(false);
      });
    }
    menu.dataset.file = payload.file;
    menu.querySelector<HTMLElement>(".page-delete")!.hidden = !payload.exists;
  }

  private async confirmDelete(): Promise<void> {
    const file = document.querySelector<HTMLElement>(".page-menu")?.dataset.file ?? "";
    const count = Number(document.querySelector("section.linked-refs h2 .count")?.textContent ?? "0") || 0;
    const dialog = document.createElement("dialog");
    dialog.className = "confirm-delete";
    const heading = document.createElement("h2");
    heading.textContent = "Delete this page?";
    const what = document.createElement("p");
    what.append("The file ");
    const code = document.createElement("code");
    code.textContent = file;
    what.append(code, " will be moved to .trash/ in the notes folder; move it back to restore the page.");
    const refs = document.createElement("p");
    refs.textContent =
      count === 0
        ? "No block on another page references it."
        : `${count} block${count === 1 ? "" : "s"} on other pages reference${count === 1 ? "s" : ""} it; the page stays, without content, for ${count === 1 ? "that reference" : "those references"}.`;
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.textContent = "Cancel";
    cancel.autofocus = true;
    const confirm = document.createElement("button");
    confirm.type = "button";
    confirm.className = "danger";
    confirm.textContent = "Move to trash";
    const error = document.createElement("p");
    error.className = "error";
    error.hidden = true;
    const buttons = document.createElement("div");
    buttons.className = "dialog-buttons";
    buttons.append(cancel, confirm);
    dialog.append(heading, what, refs, error, buttons);
    this.append(dialog);
    dialog.addEventListener("close", () => dialog.remove());
    cancel.addEventListener("click", () => dialog.close());
    confirm.addEventListener("click", async () => {
      confirm.disabled = true;
      this.end(false);
      try {
        await this.api.delete(this.path, this.queue?.version ?? "");
        location.href = new URL("index.html", this.root).href;
      } catch (e) {
        error.hidden = false;
        error.textContent =
          e instanceof Conflict
            ? "The page changed since it was loaded; nothing was deleted. Reload and try again."
            : "The page could not be deleted; nothing was changed.";
        confirm.disabled = false;
      }
    });
    dialog.showModal();
  }

  // --- Page properties ------------------------------------------------------------------

  /** The properties dialog; its rows become the page's first block when it closes. */
  private editProperties(): void {
    if (!this.doc || document.querySelector("dialog[open]")) return;
    this.end(true);
    editProperties(this, pageProps(this.doc), (rows) => {
      if (!this.doc || !setPageProps(this.doc, rows)) return;
      this.render();
      this.saveNow();
    });
  }

  // --- Task labels ----------------------------------------------------------------------

  /** A label of one of this page's own blocks was clicked. */
  private labelClicked(label: HTMLElement): void {
    const li = label.closest("li.block");
    const node = li ? this.nodes.get(li) : undefined;
    if (node) this.advanceOwn(node);
  }

  /** Moves a block of this page to its next task state, keeping its rendering. */
  private advanceOwn(node: Node): boolean {
    const body = ops.advanceTask(node.body);
    if (body === null) return false;
    const r = this.rendered.get(node);
    const fresh = r !== undefined && r.body === node.body;
    node.body = body;
    if (this.active?.node === node) this.active.area.value = body;
    if (r && fresh) {
      r.body = body;
      const state = taskState(body);
      const label = r.el.querySelector<HTMLElement>(":scope > .task-marker");
      if (label && state) showState(label, state);
      r.className = r.className.replace(/\btask-(todo|doing|done)\b/, `task-${state}`);
    }
    // Rendering moves the text field of the block being edited; keep its focus.
    const area = this.active?.area;
    const [start, end] = area ? [area.selectionStart, area.selectionEnd] : [0, 0];
    const focused = area !== undefined && document.activeElement === area;
    this.render();
    if (area && focused) {
      area.focus({ preventScroll: true });
      area.setSelectionRange(start, end);
    }
    this.saveNow();
    return true;
  }

  /**
   * Labels in task lists (the open-tasks pages and each page's "Open tasks" section)
   * move the task they belong to, found through the item's link to its block.
   */
  private watchTaskLists(): void {
    const main = document.querySelector("main");
    if (!main) return;
    const lists = ":scope ul.tasks > li > .task-marker";
    armLabels(main, lists);
    const act = (label: HTMLElement) => void this.advanceListed(label);
    main.addEventListener("click", (e) => {
      const label = (e.target as Element).closest<HTMLElement>("ul.tasks > li > .task-marker[role=button]");
      if (label) act(label);
    });
    main.addEventListener("keydown", (e) => {
      const target = e.target as Element;
      if ((e.key === "Enter" || e.key === " ") && target.matches("ul.tasks > li > .task-marker[role=button]")) {
        e.preventDefault();
        act(target as HTMLElement);
      }
    });
    document.addEventListener(PAGES_CHANGED, () => armLabels(main, lists));
  }

  private async advanceListed(label: HTMLElement): Promise<void> {
    const link = label.parentElement?.querySelector<HTMLAnchorElement>("a.task-source");
    if (!link || label.dataset.busy) return;
    const url = new URL(link.href);
    const path = sitePath(url.pathname, this.root.pathname);
    const anchor = decodeURIComponent(url.hash.slice(1));
    if (!path || !anchor || url.origin !== this.root.origin) return;
    const before = taskState(label.textContent ?? "");
    if (path === this.path && this.doc) {
      const node = flatten(this.doc).find((e) => e.node.anchor === anchor)?.node;
      if (node && this.advanceOwn(node)) showState(label, taskState(node.body)!);
      return;
    }
    label.dataset.busy = "1";
    if (before) showState(label, NEXT[before]);
    try {
      const ok = await advanceRemote(this.api, path, anchor);
      if (!ok && before) showState(label, before);
    } catch {
      if (before) showState(label, before);
    } finally {
      delete label.dataset.busy;
    }
  }

  // --- Editing ------------------------------------------------------------------------

  private onClick(e: MouseEvent): void {
    const target = e.target as Element;
    const label = target.closest<HTMLElement>(".task-marker[role=button]");
    if (label) {
      this.labelClicked(label);
      return;
    }
    if (target.closest("a, button, textarea, input, .block-menu")) return;
    if (target.closest(".editor-start")) {
      this.doc!.roots.push(newNode());
      this.edit(this.doc!.roots[0]!, 0);
      return;
    }
    const li = target.closest("li.block");
    const node = li ? this.nodes.get(li) : undefined;
    if (!node || this.active?.node === node) return;
    // Do not take a click that ends a text selection.
    if (String(getSelection() ?? "").length > 0) return;
    this.edit(node, node.body.length);
  }

  private startFirst(): void {
    if (!this.doc) return;
    if (this.doc.roots.length === 0) this.doc.roots.push(newNode());
    this.edit(this.doc.roots[0]!, 0);
  }

  /** Starts (or moves) editing to `node`, with the cursor at `offset`. */
  edit(node: Node, offset: number): void {
    if (this.active && this.active.node !== node) this.end(true);
    if (!this.active) {
      const area = document.createElement("textarea");
      area.className = "block-editor";
      area.rows = 1;
      area.spellcheck = true;
      area.setAttribute("aria-label", "Block text");
      area.addEventListener("input", () => this.onInput(area));
      area.addEventListener("keydown", (e) => this.onKey(e, area));
      area.addEventListener("blur", () => {
        // Let clicks on lists or menus that keep editing take focus back first.
        setTimeout(() => {
          if (this.active?.area === area && document.activeElement !== area && !this.contains(document.activeElement)) {
            this.end(true);
          }
        }, 150);
      });
      this.active = { node, area };
    }
    this.active.node = node;
    this.active.area.value = node.body;
    this.render();
    this.focusAt(offset);
  }

  private focusAt(offset: number): void {
    const area = this.active?.area;
    if (!area) return;
    area.focus({ preventScroll: true });
    area.setSelectionRange(offset, offset);
    autosize(area);
    area.scrollIntoView({ block: "nearest" });
  }

  /** Ends editing; the block shows as rendered (or as pending text until the rebuild). */
  end(save: boolean): void {
    if (!this.active) return;
    this.active = null;
    for (const p of plugins) p.end?.(this);
    this.render();
    if (save) this.saveNow();
  }

  get editing(): Active | null {
    return this.active;
  }

  private onInput(area: HTMLTextAreaElement): void {
    if (!this.active) return;
    this.active.node.body = area.value;
    autosize(area);
    for (const p of plugins) p.input?.(this, area);
    this.changed();
  }

  /** Something in the outline changed: save once typing pauses. */
  changed(): void {
    this.showState("unsaved");
    clearTimeout(this.idle);
    this.idle = window.setTimeout(() => this.saveNow(), IDLE_SAVE_MS);
  }

  saveNow(): void {
    clearTimeout(this.idle);
    if (!this.queue || !this.doc) return;
    if (sameText(serialize(this.doc), this.stored) && !this.queue.dirty) {
      this.showState("saved");
      return;
    }
    void this.queue.request();
  }

  /** Applies an outliner operation's result: re-render, keep editing, save. */
  private apply(focus: ops.Focus | null): boolean {
    if (!focus) return false;
    this.edit(focus.node, focus.offset);
    this.saveNow();
    return true;
  }

  private onKey(e: KeyboardEvent, area: HTMLTextAreaElement): void {
    if (e.isComposing || !this.active || !this.doc) return;
    for (const p of plugins) {
      if (p.key?.(this, e, area)) return;
    }
    const doc = this.doc;
    const node = this.active.node;
    node.body = area.value;
    const start = area.selectionStart;
    const collapsed = start === area.selectionEnd;
    const mod = e.ctrlKey || e.metaKey;
    const marker = mod && !e.altKey ? formatMarker(e.key.toLowerCase(), e.shiftKey) : null;
    if (marker) {
      e.preventDefault();
      format(area, marker);
      return;
    }
    if (e.key === "Enter" && mod && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      this.apply(ops.cycleTask(node, start));
    } else if (e.key === "Enter" && !e.shiftKey && !e.altKey && !mod) {
      const focus = ops.split(doc, node, start);
      if (focus) {
        e.preventDefault();
        this.apply(focus);
      }
    } else if (e.key === "Tab" && !e.altKey && !mod) {
      e.preventDefault();
      this.apply(e.shiftKey ? ops.outdent(doc, node, start) : ops.indent(doc, node, start));
    } else if ((e.key === "ArrowUp" || e.key === "ArrowDown") && e.altKey && e.shiftKey && !mod) {
      e.preventDefault();
      this.apply(ops.move(doc, node, start, e.key === "ArrowUp" ? -1 : 1));
    } else if (e.key === "Backspace" && collapsed && start === 0 && !mod && !e.altKey) {
      const focus = ops.backspace(doc, node);
      if (focus) {
        e.preventDefault();
        this.apply(focus);
      }
    } else if ((e.key === "ArrowUp" || e.key === "ArrowDown") && collapsed && !e.altKey && !e.shiftKey && !mod) {
      const up = e.key === "ArrowUp";
      const lineStart = area.value.lastIndexOf("\n", start - 1) + 1;
      const onEdge = up ? lineStart === 0 : area.value.indexOf("\n", start) < 0;
      const other = onEdge ? ops.neighbor(doc, node, up ? -1 : 1) : null;
      if (other) {
        e.preventDefault();
        const column = start - lineStart;
        const lastLine = other.body.lastIndexOf("\n") + 1;
        const firstLen = other.body.split("\n")[0]!.length;
        this.edit(other, up ? Math.min(lastLine + column, other.body.length) : Math.min(column, firstLen));
        this.saveNow();
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      area.blur();
      this.end(true);
    }
  }

  // --- Saving results -------------------------------------------------------------------

  private onSaved(payload: Payload, text: string): void {
    // If nothing changed since this save started, take the server's view of the
    // blocks (text typed as `- …` becomes separate blocks), then the rendered page: the
    // response comes after the rebuild, whose change notice may have arrived while
    // this save was still running (and was then skipped).
    if (this.doc && serialize(this.doc) === text) {
      this.resync(payload, null);
      // After the save queue has finished (this runs inside it).
      setTimeout(() => void this.refresh(), 0);
    } else {
      this.stored = text;
    }
    this.clearNotice("failed");
  }

  private async onConflict(payload: Payload, refused: string): Promise<void> {
    this.active = null;
    this.resync(payload, null);
    this.showNotice(
      "conflict",
      "This page was changed elsewhere, so your last change was not saved. The page now shows the saved version; your text is below.",
      refused,
    );
    await this.refresh();
  }

  private onFailed(e: Refused): void {
    this.showNotice("failed", `The server refused to save this page (status ${e.status}). Your changes are kept in this tab.`, null);
  }

  private showNotice(kind: string, message: string, text: string | null): void {
    this.clearNotice(kind);
    const box = document.createElement("div");
    box.className = `edit-notice edit-${kind}`;
    box.setAttribute("role", "alert");
    const p = document.createElement("p");
    p.textContent = message;
    box.append(p);
    if (text !== null) {
      const area = document.createElement("textarea");
      area.readOnly = true;
      area.value = text;
      area.rows = Math.min(10, text.split("\n").length);
      const copy = document.createElement("button");
      copy.type = "button";
      copy.textContent = "Copy";
      copy.addEventListener("click", () => {
        area.select();
        void navigator.clipboard?.writeText(text).catch(() => document.execCommand("copy"));
      });
      box.append(area, copy);
    }
    const dismiss = document.createElement("button");
    dismiss.type = "button";
    dismiss.textContent = "Dismiss";
    dismiss.addEventListener("click", () => box.remove());
    box.append(dismiss);
    this.section?.before(box);
  }

  private clearNotice(kind: string): void {
    document.querySelector(`.edit-notice.edit-${kind}`)?.remove();
  }
}

/** The marker a formatting key toggles: Ctrl/Cmd + B, I, E, or Shift+S. */
function formatMarker(key: string, shift: boolean): string | null {
  if (shift) return key === "s" ? MARKERS.strike : null;
  return key === "b" ? MARKERS.bold : key === "i" ? MARKERS.italic : key === "e" ? MARKERS.code : null;
}

/**
 * Toggles `marker` around the selection. The change goes through the browser's text
 * insertion, so its own undo (Ctrl+Z) reverts it; where that is unavailable the text is
 * replaced directly (without undo).
 */
function format(area: HTMLTextAreaElement, marker: string): void {
  const edit = toggle(area.value, area.selectionStart, area.selectionEnd, marker);
  const expected = applied(area.value, edit);
  area.setSelectionRange(edit.start, edit.end);
  const inserted =
    edit.text === "" ? document.execCommand("delete") : document.execCommand("insertText", false, edit.text);
  if (!inserted || area.value !== expected) {
    area.value = expected;
    area.dispatchEvent(new Event("input", { bubbles: true }));
  }
  area.setSelectionRange(edit.selStart, edit.selEnd);
}

function autosize(area: HTMLTextAreaElement): void {
  area.style.height = "auto";
  area.style.height = `${area.scrollHeight}px`;
}

type TaskState = "todo" | "doing" | "done";
const NEXT: Record<TaskState, TaskState> = { todo: "doing", doing: "done", done: "todo" };

/** The task state a body (or a label's text) starts with. */
function taskState(text: string): TaskState | null {
  const m = /^(TODO|DOING|DONE)(?=\s|$)/.exec(text);
  return m ? (m[1]!.toLowerCase() as TaskState) : null;
}

function showState(label: HTMLElement, state: TaskState): void {
  label.textContent = state.toUpperCase();
  label.classList.remove("task-todo", "task-doing", "task-done");
  label.classList.add(`task-${state}`);
  label.title = `${state.toUpperCase()}: click for ${NEXT[state].toUpperCase()}`;
}

/** Makes task labels matching `selector` under `root` controls. */
function armLabels(root: Element, selector: string): void {
  for (const label of root.querySelectorAll<HTMLElement>(selector)) {
    if (label.getAttribute("role") === "button") continue;
    label.setAttribute("role", "button");
    label.tabIndex = 0;
    const state = taskState(label.textContent ?? "");
    if (state) label.title = `${state.toUpperCase()}: click for ${NEXT[state].toUpperCase()}`;
  }
}

/**
 * Moves the task at `anchor` on another page to its next state, in that page's file.
 * Retries once when the file changed meanwhile; false when the block is no task there.
 */
async function advanceRemote(api: SourceApi, path: string, anchor: string): Promise<boolean> {
  for (let attempt = 0; ; attempt++) {
    const payload = await api.get(path);
    const doc = fromPayload(payload);
    const node = flatten(doc).find((e) => e.node.anchor === anchor)?.node;
    const body = node ? ops.advanceTask(node.body) : null;
    if (!node || body === null) return false;
    node.body = body;
    try {
      await api.put(path, payload.version, serialize(doc));
      return true;
    } catch (e) {
      if (!(e instanceof Conflict) || attempt > 0) throw e;
    }
  }
}

/**
 * Keeps live reload from reloading this page while an action that writes a note
 * navigates away by itself: the write's rebuild would otherwise reload the page first
 * and cut the action off (live reload hands changes to the editor instead).
 */
function holdLiveReload(): void {
  document.documentElement.setAttribute(EDITING_ATTR, "");
}

/** Records (or clears) the page whose first block to edit once it has loaded. */
function startAfterLoad(path: string | null): void {
  try {
    if (path === null) sessionStorage.removeItem(START_KEY);
    else sessionStorage.setItem(START_KEY, JSON.stringify({ path, at: Date.now() }));
  } catch {
    // Without storage, the page opens without editing.
  }
}

/** Whether key presses on `target` belong to it (fields, links, buttons, …). */
function isControl(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.isContentEditable ||
    ["INPUT", "TEXTAREA", "SELECT", "BUTTON", "A", "SUMMARY"].includes(target.tagName) ||
    target.getAttribute("role") === "button"
  );
}

/** Today's date in the browser's time zone, as `YYYY-MM-DD`. */
export function localDate(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** The site path of the page at `pathname`, relative to the root's path (see live-reload). */
export function sitePath(pathname: string, rootPathname: string): string | null {
  if (!pathname.startsWith(rootPathname)) return null;
  let path: string;
  try {
    path = decodeURIComponent(pathname.slice(rootPathname.length));
  } catch {
    return null;
  }
  return path === "" || path.endsWith("/") ? `${path}index.html` : path;
}

export function define(name = "hermetic-editor"): void {
  if (!customElements.get(name)) customElements.define(name, HermeticEditor);
}
