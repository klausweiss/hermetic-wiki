// <hermetic-shortcuts>: keyboard shortcuts and a help dialog listing them. It talks to
// the sidebar only through events and the URL, so it works with or without one.

import {
  EDITING_ATTR,
  EDIT_PROPERTIES,
  OPEN_TODAY,
  SEARCH_FOCUS,
  SEARCH_TOGGLE,
  SIDEBAR_CLOSE_TOP,
  SIDEBAR_TOGGLE,
  THEME_SWITCH,
  carrySidebar,
  icon,
  readEnv,
  shownTheme,
  whenParsed,
} from "@hermetic/core";

interface Shortcut {
  /**
   * The keys, written `Mod+Shift+Z` (pressed together), `g h` (one after the other),
   * `Tab / Shift+Tab` (alternatives), with `{text}` for words that are not keys.
   * `Mod` is ⌘ on Apple platforms and Ctrl elsewhere.
   */
  keys: string;
  what: string;
  sidebar?: true;
  search?: true;
  /** Listed only on the graph page. */
  graph?: true;
  /** Listed only where editing is available (hermetic serve, same machine). */
  editing?: true;
}

interface Group {
  title: string;
  shortcuts: readonly Shortcut[];
}

const APPLE = /Mac|iPhone|iPad/.test(navigator.platform);
/** How keys are labelled: Apple's symbols there, names elsewhere. */
const KEY_LABELS: Readonly<Record<string, string>> = APPLE
  ? { Mod: "⌘", Shift: "⇧", Alt: "⌥", Ctrl: "⌃" }
  : { Mod: "Ctrl" };

// Formatting before editing: the two columns then come out about even.
const HELP: readonly Group[] = [
  {
    title: "Search and view",
    shortcuts: [
      { keys: "/", what: "Open search", search: true },
      { keys: "Mod+K", what: "Open or close search, even while typing", search: true },
      { keys: "Mod+F", what: "Search the graph (again: the browser's find)", graph: true },
      { keys: "Shift+{click}", what: "Open a page in the sidebar", sidebar: true },
      { keys: "Esc", what: "Close the top sidebar pane", sidebar: true },
      { keys: "\\", what: "Show or hide the sidebar", sidebar: true },
      { keys: "?", what: "Show these shortcuts" },
    ],
  },
  {
    title: "Formatting and history",
    shortcuts: [
      { keys: "Mod+B", what: "Bold", editing: true },
      { keys: "Mod+I", what: "Italic", editing: true },
      { keys: "Mod+Shift+S", what: "Strikethrough", editing: true },
      { keys: "Mod+E", what: "Inline code", editing: true },
      { keys: "Mod+V", what: "Paste a URL over selected text to link it", editing: true },
      { keys: "Mod+Z", what: "Undo, across blocks", editing: true },
      { keys: "Mod+Shift+Z", what: "Redo", editing: true },
    ],
  },
  {
    title: "Editing",
    shortcuts: [
      { keys: "{Click a block}", what: "Edit it", editing: true },
      { keys: "p", what: "Edit the page's properties", editing: true },
      { keys: "Enter", what: "New block below", editing: true },
      { keys: "Shift+Enter", what: "Line break", editing: true },
      { keys: "Tab / Shift+Tab", what: "Indent / outdent the block", editing: true },
      { keys: "Alt+Shift+↑ / ↓", what: "Move the block up / down", editing: true },
      { keys: "Backspace", what: "At the start: join with the block above", editing: true },
      { keys: "Mod+Enter", what: "Cycle TODO / DOING / DONE", editing: true },
      { keys: "[[ / ((", what: "Link a page / reference a block", editing: true },
      { keys: "Esc", what: "Stop editing", editing: true },
    ],
  },
];

/** `keys` (see Shortcut) as keycaps, with "then" and "or" between steps and choices. */
export function keycaps(keys: string): HTMLElement {
  const out = document.createElement("span");
  out.className = "keys";
  const word = (cls: string, text: string) => {
    const span = document.createElement("span");
    span.className = cls;
    span.textContent = text;
    return span;
  };
  keys.split(" / ").forEach((choice, i) => {
    if (i > 0) out.append(word("keys-sep", "or"));
    // Spaces inside {text} do not separate steps.
    (choice.match(/(?:\{[^}]*\}|[^\s{])+/g) ?? []).forEach((step, j) => {
      if (j > 0) out.append(word("keys-sep", "then"));
      const chord = document.createElement("span");
      chord.className = "chord";
      for (const part of step.match(/\{[^}]*\}|[^+{]+|\+(?=\+|$)/g) ?? []) {
        if (part.startsWith("{")) {
          chord.append(word("keys-text", part.slice(1, -1)));
        } else {
          const kbd = document.createElement("kbd");
          kbd.textContent = KEY_LABELS[part] ?? part;
          chord.append(kbd);
        }
      }
      out.append(chord);
    });
  });
  return out;
}

/** An action of a key menu: pressing the menu's key and then the action's key runs it. */
interface Action {
  key: string;
  what: string;
  /** Shown after the description, e.g. what the action changes from and to. */
  detail?: () => string;
  available?: () => boolean;
  run: () => void;
}

/** A menu opened by a key, listing the actions a second key runs. */
interface Menu {
  key: string;
  title: string;
  actions: readonly Action[];
}

/** Opens a page of the site, carrying the sidebar's state (mirrored in the URL) along. */
function go(path: string): () => void {
  return () => {
    location.href = carrySidebar(new URL(path, readEnv().root).href, location.href);
  };
}

/** How long a menu's key waits for the second key before showing the menu. */
const MENU_DELAY_MS = 300;

const MENUS: readonly Menu[] = [
  {
    key: "g",
    title: "Go to",
    actions: [
      { key: "h", what: "Home", run: go("index.html") },
      { key: "j", what: "Journals", run: go("journals.html") },
      {
        key: "d",
        what: "Today's journal",
        // The editor opens (and creates) it where it runs.
        available: () => document.querySelector("hermetic-editor") !== null,
        run: () => document.dispatchEvent(new Event(OPEN_TODAY)),
      },
      { key: "p", what: "All pages", run: go("pages.html") },
      { key: "t", what: "Open tasks", run: go("tasks.html") },
      { key: "g", what: "Graph", run: go("graph.html") },
    ],
  },
  {
    key: ",",
    title: "Actions",
    actions: [
      {
        key: "t",
        what: "Dark / light theme",
        detail: () => (shownTheme() === "dark" ? "dark → light" : "light → dark"),
        available: () => hasThemeButton(),
        run: () => document.dispatchEvent(new Event(THEME_SWITCH)),
      },
    ],
  },
];

const available = (menu: Menu) => menu.actions.filter((a) => a.available?.() ?? true);

export class HermeticShortcuts extends HTMLElement {
  private ready = false;
  private help: HTMLDialogElement | null = null;
  private menu: HTMLDialogElement | null = null;
  /** The menu shown in `menu`. */
  private shown: Menu | null = null;
  /** A menu whose key was just pressed, shown only if no second key follows soon. */
  private pendingMenu: { menu: Menu; timer: number } | null = null;

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  private setup(): void {
    if (this.ready) {
      return;
    }
    this.ready = true;
    addEventListener("keydown", (e) => this.onKey(e));
  }

  private onKey(e: KeyboardEvent): void {
    // Ctrl+K / Cmd+K opens or closes search, even from a text field (such as the search
    // field itself), and replaces the browser's own binding (Firefox's web search) only
    // where there is search to open.
    if (
      !e.defaultPrevented &&
      (e.ctrlKey || e.metaKey) &&
      !e.altKey &&
      !e.shiftKey &&
      e.key.toLowerCase() === "k" &&
      hasSearch()
    ) {
      e.preventDefault();
      document.dispatchEvent(new Event(SEARCH_TOGGLE));
      return;
    }
    if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) {
      return;
    }
    // The second key of a menu pressed before the menu shows: run its action at once.
    if (this.pendingMenu) {
      if (["Shift", "Control", "Alt", "Meta"].includes(e.key)) return;
      const { menu } = this.pendingMenu;
      this.cancelMenu();
      const action = available(menu).find((a) => a.key === e.key);
      if (action || e.key === "Escape") {
        e.preventDefault();
        action?.run();
        return;
      }
    }
    const menu = MENUS.find((m) => m.key === e.key);
    if (menu) {
      if (!document.querySelector("dialog[open]") && available(menu).length > 0) {
        e.preventDefault();
        // Shown after a pause, so that typing both keys from memory does not flash it.
        const timer = window.setTimeout(() => {
          this.pendingMenu = null;
          if (!document.querySelector("dialog[open]")) this.openMenu(menu);
        }, MENU_DELAY_MS);
        this.pendingMenu = { menu, timer };
      }
      return;
    }
    switch (e.key) {
      case "Escape":
        if (!this.help?.open) {
          document.dispatchEvent(new Event(SIDEBAR_CLOSE_TOP));
        }
        break;
      case "/":
        // Only when a search box is shown; otherwise the key keeps its default.
        if (hasSearch()) {
          e.preventDefault();
          document.dispatchEvent(new Event(SEARCH_FOCUS));
        }
        break;
      case "p":
        // Only on a page open for editing; the editor shows the properties dialog.
        if (document.documentElement.hasAttribute(EDITING_ATTR) && !document.querySelector("dialog[open]")) {
          e.preventDefault();
          document.dispatchEvent(new Event(EDIT_PROPERTIES));
        }
        break;
      case "\\":
        e.preventDefault();
        document.dispatchEvent(new Event(SIDEBAR_TOGGLE));
        break;
      case "?":
        e.preventDefault();
        this.help ??= this.helpDialog();
        this.help.showModal();
        this.help.focus();
        break;
    }
  }

  private cancelMenu(): void {
    if (this.pendingMenu) clearTimeout(this.pendingMenu.timer);
    this.pendingMenu = null;
  }

  /**
   * A key menu (`g`, `,`): shows that its key was pressed and lists the actions a second
   * key runs. That key (or a click on an action) runs it; any other key, Escape, or a
   * click outside closes the menu.
   */
  private openMenu(menu: Menu): void {
    const dialog = (this.menu ??= this.menuDialog());
    this.shown = menu;
    dialog.setAttribute("aria-label", menu.title);
    const head = dialog.querySelector(".leader-head")!;
    const title = document.createElement("span");
    title.className = "leader-title";
    title.textContent = menu.title;
    const hint = document.createElement("span");
    hint.textContent = "then press a key";
    head.replaceChildren(keycaps(menu.key), title, hint);
    dialog.querySelector("ul")!.replaceChildren(
      ...available(menu).map((a) => {
        const li = document.createElement("li");
        const button = document.createElement("button");
        button.type = "button";
        button.className = "leader-action";
        const what = document.createElement("span");
        what.className = "leader-what";
        what.textContent = a.what;
        button.append(keycaps(a.key), what);
        if (a.detail) {
          const detail = document.createElement("span");
          detail.className = "leader-detail";
          detail.textContent = a.detail();
          button.append(detail);
        }
        button.addEventListener("click", () => {
          dialog.close();
          a.run();
        });
        li.append(button);
        return li;
      }),
    );
    dialog.showModal();
    dialog.focus();
  }

  private menuDialog(): HTMLDialogElement {
    const dialog = document.createElement("dialog");
    dialog.className = "leader-dialog";
    dialog.tabIndex = -1;
    const panel = document.createElement("div");
    panel.className = "leader-panel";
    const head = document.createElement("div");
    head.className = "leader-head";
    panel.append(head, document.createElement("ul"));
    dialog.append(panel);
    dialog.addEventListener("keydown", (e) => {
      // A key reaching the dialog after it closed (focus still on it) is the page's.
      if (!dialog.open) return;
      if (e.key === "Escape" || e.key === "Tab" || e.ctrlKey || e.metaKey || e.altKey) return;
      if (["Shift", "Control", "Alt", "Meta", "Enter", " "].includes(e.key)) return;
      e.preventDefault();
      e.stopPropagation();
      dialog.close();
      const menu = this.shown;
      if (menu) available(menu).find((a) => a.key === e.key)?.run();
    });
    // The dialog itself has no padding: a click on it, not on the panel, is on the backdrop.
    dialog.addEventListener("click", (e) => {
      if (e.target === dialog) dialog.close();
    });
    this.append(dialog);
    return dialog;
  }

  private helpDialog(): HTMLDialogElement {
    const withSidebar = readEnv().interactive && document.querySelector("hermetic-sidebar") !== null;
    const withEditing = readEnv().interactive && document.querySelector("hermetic-editor") !== null;
    const withGraph = document.querySelector("main hermetic-graph[data-ready]") !== null;
    const shown = (s: Shortcut) =>
      !(
        (s.sidebar && !withSidebar) ||
        (s.search && !hasSearch()) ||
        (s.editing && !withEditing) ||
        (s.graph && !withGraph)
      );
    const dialog = document.createElement("dialog");
    dialog.className = "shortcuts";
    dialog.setAttribute("aria-labelledby", "shortcuts-title");
    // Focusable itself, to take the focus on opening instead of its close button.
    dialog.tabIndex = -1;
    const header = document.createElement("header");
    const title = document.createElement("h2");
    title.id = "shortcuts-title";
    title.textContent = "Keyboard shortcuts";
    const close = document.createElement("button");
    close.type = "button";
    close.className = "shortcuts-close";
    close.setAttribute("aria-label", "Close");
    close.append(icon("x"));
    close.addEventListener("click", () => dialog.close());
    header.append(title, close);
    // The key menus first, set apart: each its key and title, then its actions' keys.
    const menus = document.createElement("div");
    menus.className = "shortcut-menus";
    for (const menu of MENUS) {
      const actions = available(menu);
      if (actions.length === 0) continue;
      const card = document.createElement("section");
      card.className = "shortcut-menu";
      const heading = document.createElement("h3");
      const key = keycaps(menu.key);
      key.classList.add("menu-key");
      const title = document.createElement("span");
      title.textContent = menu.title;
      const hint = document.createElement("span");
      hint.className = "keys-sep";
      hint.textContent = "then";
      heading.append(key, title, hint);
      const list = document.createElement("dl");
      for (const a of actions) {
        const row = document.createElement("div");
        const dt = document.createElement("dt");
        dt.append(keycaps(a.key));
        const dd = document.createElement("dd");
        dd.textContent = a.what;
        row.append(dt, dd);
        list.append(row);
      }
      card.append(heading, list);
      menus.append(card);
    }
    const groups = document.createElement("div");
    groups.className = "shortcut-groups";
    for (const group of HELP) {
      const items = group.shortcuts.filter(shown);
      if (items.length === 0) continue;
      const section = document.createElement("section");
      const heading = document.createElement("h3");
      heading.textContent = group.title;
      const list = document.createElement("dl");
      for (const s of items) {
        const row = document.createElement("div");
        const dt = document.createElement("dt");
        dt.textContent = s.what;
        const dd = document.createElement("dd");
        dd.append(keycaps(s.keys));
        row.append(dt, dd);
        list.append(row);
      }
      section.append(heading, list);
      groups.append(section);
    }
    // A click on the backdrop (the dialog element itself, outside its content) closes it.
    dialog.addEventListener("click", (e) => {
      if (e.target === dialog) dialog.close();
    });
    dialog.append(header, ...(menus.childElementCount > 0 ? [menus] : []), groups);
    this.append(dialog);
    return dialog;
  }
}

function hasThemeButton(): boolean {
  return document.querySelector("hermetic-theme-toggle .theme-button") !== null;
}

/** The search button renders only where search can work (over http). */
function hasSearch(): boolean {
  return document.querySelector("hermetic-search .search-button") !== null;
}

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}

export function define(name = "hermetic-shortcuts"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticShortcuts);
  }
}
