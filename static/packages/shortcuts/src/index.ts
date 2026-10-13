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
  carrySidebar,
  readEnv,
  whenParsed,
} from "@hermetic/core";

const GO_TARGETS: Readonly<Record<string, string>> = {
  h: "index.html",
  j: "journals.html",
  p: "pages.html",
  t: "tasks.html",
  g: "graph.html",
};

interface Shortcut {
  keys: string;
  what: string;
  sidebar?: true;
  search?: true;
  /** Listed only on the graph page. */
  graph?: true;
  /** Listed only where editing is available (hermetic serve, same machine). */
  editing?: true;
}

const APPLE = /Mac|iPhone|iPad/.test(navigator.platform);
/** Ctrl+K, or Cmd+K on Apple platforms, as the help dialog names it. */
const MOD_K = APPLE ? "⌘ K" : "Ctrl + K";
const MOD_ENTER = APPLE ? "⌘ Enter" : "Ctrl + Enter";
const MOD_F = APPLE ? "⌘ F" : "Ctrl + F";
/** The modifier for editing keys, as the help dialog writes it before a key. */
const MOD = APPLE ? "⌘" : "Ctrl +";

const HELP: readonly Shortcut[] = [
  { keys: "g h", what: "Home" },
  { keys: "g j", what: "Journals" },
  { keys: "g p", what: "All pages" },
  { keys: "g t", what: "Open tasks" },
  { keys: "g g", what: "Graph" },
  { keys: "g d", what: "Today's journal", editing: true },
  { keys: "/", what: "Open search", search: true },
  { keys: MOD_K, what: "Open or close search, even while typing", search: true },
  { keys: MOD_F, what: "Search the graph (again: the browser's find)", graph: true },
  { keys: "Shift + click", what: "Open a page in the sidebar", sidebar: true },
  { keys: "Esc", what: "Close the top sidebar pane", sidebar: true },
  { keys: "\\", what: "Show or hide the sidebar", sidebar: true },
  { keys: "?", what: "Show these shortcuts" },
  { keys: "p", what: "Edit the page's properties", editing: true },
  { keys: "Click a block", what: "Edit it", editing: true },
  { keys: "Enter", what: "New block below (Shift + Enter: line break)", editing: true },
  { keys: "Tab / Shift + Tab", what: "Indent / outdent the block", editing: true },
  { keys: "Alt + Shift + ↑ / ↓", what: "Move the block up / down", editing: true },
  { keys: "Backspace at the start", what: "Join with the block above (or delete an empty one)", editing: true },
  { keys: MOD_ENTER, what: "Cycle TODO / DOING / DONE", editing: true },
  { keys: "[[ and ((", what: "Link a page / reference a block", editing: true },
  { keys: `${MOD} B / ${MOD} I`, what: "Bold / italic (also on selected text)", editing: true },
  { keys: `${MOD} Shift S / ${MOD} E`, what: "Strikethrough / inline code", editing: true },
  { keys: "Esc", what: "Stop editing", editing: true },
];

/** How long after `g` the second key is accepted. */
const CHORD_MS = 1500;

export class HermeticShortcuts extends HTMLElement {
  private ready = false;
  private help: HTMLDialogElement | null = null;
  private pendingG = 0;

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
    const chord = Date.now() - this.pendingG < CHORD_MS;
    this.pendingG = 0;
    const target = chord ? GO_TARGETS[e.key] : undefined;
    // `g d`: today's journal, which the editor opens (and creates) where it runs.
    if (chord && e.key === "d" && document.querySelector("hermetic-editor")) {
      e.preventDefault();
      document.dispatchEvent(new Event(OPEN_TODAY));
      return;
    }
    if (target) {
      e.preventDefault();
      // The sidebar's state is mirrored in the URL; carry it along.
      location.href = carrySidebar(new URL(target, readEnv().root).href, location.href);
      return;
    }
    switch (e.key) {
      case "g":
        this.pendingG = Date.now();
        break;
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
        break;
    }
  }

  private helpDialog(): HTMLDialogElement {
    const withSidebar = readEnv().interactive && document.querySelector("hermetic-sidebar") !== null;
    const withEditing = readEnv().interactive && document.querySelector("hermetic-editor") !== null;
    const withGraph = document.querySelector("main hermetic-graph[data-ready]") !== null;
    const dialog = document.createElement("dialog");
    dialog.className = "shortcuts";
    const title = document.createElement("h2");
    title.textContent = "Keyboard shortcuts";
    const list = document.createElement("dl");
    for (const s of HELP) {
      if ((s.sidebar && !withSidebar) || (s.search && !hasSearch()) || (s.editing && !withEditing) || (s.graph && !withGraph)) {
        continue;
      }
      const dt = document.createElement("dt");
      dt.textContent = s.keys;
      const dd = document.createElement("dd");
      dd.textContent = s.what;
      list.append(dt, dd);
    }
    const close = document.createElement("button");
    close.type = "button";
    close.textContent = "Close";
    close.addEventListener("click", () => dialog.close());
    dialog.append(title, list, close);
    this.append(dialog);
    return dialog;
  }
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
