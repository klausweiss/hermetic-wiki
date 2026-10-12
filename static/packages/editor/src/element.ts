// <hermetic-editor>: inserted by `hermetic serve` into pages it serves to a browser on
// the same machine. It turns the page's own blocks into a Logseq-style outliner: click
// a block to edit its markdown, use the outliner keys, and changes are saved to the
// note file. The page is updated in place from each rebuild instead of reloading.

import { EDITING_ATTR, PAGES_CHANGED, readEnv, readPanes, whenParsed, withPanes } from "@hermetic/core";

import { Conflict, type Refused, SaveQueue, type SaveState, SourceApi } from "./api";
import { type Doc, type Node, type Payload, flatten, fromPayload, newNode } from "./model";
import * as ops from "./ops";
import { serialize } from "./serialize";

/** How long after the last keystroke a change is saved. */
export const IDLE_SAVE_MS = 1000;
/** sessionStorage key: the site path whose first block to edit once it has loaded. */
const START_KEY = "hermetic-editor-start";

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

    const path = sitePath(location.pathname, env.root.pathname);
    const article = document.querySelector<HTMLElement>("main > article.page");
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
    document.documentElement.setAttribute(EDITING_ATTR, "");

    this.queue = new SaveQueue(this.api, path, payload.version, {
      text: () => serialize(this.doc!),
      saved: (p, text) => this.onSaved(p, text),
      conflict: (p, text) => void this.onConflict(p, text),
      failed: (e, _text) => this.onFailed(e),
      state: (s) => this.showState(s),
    });
    this.resync(payload, rendered);
    this.showState("saved");

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

    try {
      if (sessionStorage.getItem(START_KEY) === path) {
        sessionStorage.removeItem(START_KEY);
        this.startFirst();
      }
    } catch {
      // Storage unavailable: just do not start editing.
    }
  }

  /** Whether changes are not yet stored in the file. */
  unsaved(): boolean {
    return (this.doc !== null && serialize(this.doc) !== this.stored) || (this.queue?.dirty ?? false);
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

  private async openToday(path: string): Promise<void> {
    try {
      const p = await this.api.get(path);
      if (!p.exists) {
        try {
          await this.api.put(path, p.version, "-\n");
        } catch (e) {
          if (!(e instanceof Conflict)) throw e; // created meanwhile: just open it
        }
      }
      sessionStorage.setItem(START_KEY, path);
    } catch {
      // Open it anyway; the page tells what is wrong.
    }
    const panes = readPanes(new URLSearchParams(location.search));
    location.href = withPanes(new URL(path, this.root).href, panes);
  }

  private showState(state: SaveState): void {
    if (!this.status || !this.path) return;
    this.status.dataset.state = state;
    this.status.textContent = state === "saved" ? "Saved" : state === "saving" ? "Saving…" : "Not saved";
  }

  // --- Loading and rendering ------------------------------------------------------------

  /**
   * Replaces the outline with `payload`. Rendered block bodies come from `html` (the
   * page's content section from the build) by anchor, or are kept from the previous
   * outline where a block's text is unchanged. Editing continues in the block at the
   * same position.
   */
  private resync(payload: Payload, html: Element | null): void {
    const before = this.doc ? flatten(this.doc).map((e) => e.node) : [];
    const activeIndex = this.active ? before.indexOf(this.active.node) : -1;
    this.doc = fromPayload(payload);
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

  // --- Editing ------------------------------------------------------------------------

  private onClick(e: MouseEvent): void {
    const target = e.target as Element;
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
    if (serialize(this.doc) === this.stored && !this.queue.dirty) {
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

function autosize(area: HTMLTextAreaElement): void {
  area.style.height = "auto";
  area.style.height = `${area.scrollHeight}px`;
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
