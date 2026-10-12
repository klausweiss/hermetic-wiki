// <hermetic-shortcuts>: keyboard shortcuts and a help dialog listing them. It talks to
// the sidebar only through events and the URL, so it works with or without one.

import {
  SEARCH_FOCUS,
  SIDEBAR_CLOSE_TOP,
  SIDEBAR_TOGGLE,
  readEnv,
  readPanes,
  whenParsed,
  withPanes,
} from "@hermetic/core";

const GO_TARGETS: Readonly<Record<string, string>> = {
  h: "index.html",
  j: "journals.html",
  p: "pages.html",
  t: "tasks.html",
};

interface Shortcut {
  keys: string;
  what: string;
  sidebar?: true;
  search?: true;
}

/** Ctrl+K, or Cmd+K on Apple platforms, as the help dialog names it. */
const MOD_K = /Mac|iPhone|iPad/.test(navigator.platform) ? "⌘ K" : "Ctrl + K";

const HELP: readonly Shortcut[] = [
  { keys: "g h", what: "Home" },
  { keys: "g j", what: "Journals" },
  { keys: "g p", what: "All pages" },
  { keys: "g t", what: "Open tasks" },
  { keys: "/", what: "Search", search: true },
  { keys: MOD_K, what: "Search, even while typing", search: true },
  { keys: "Shift + click", what: "Open a page in the sidebar", sidebar: true },
  { keys: "Esc", what: "Close the top sidebar pane", sidebar: true },
  { keys: "\\", what: "Show or hide the sidebar", sidebar: true },
  { keys: "?", what: "Show these shortcuts" },
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
    // Ctrl+K / Cmd+K works even in a text field, and replaces the browser's own binding
    // (Firefox's web search) only when there is a search box to focus.
    if (
      !e.defaultPrevented &&
      (e.ctrlKey || e.metaKey) &&
      !e.altKey &&
      !e.shiftKey &&
      e.key.toLowerCase() === "k" &&
      hasSearch()
    ) {
      e.preventDefault();
      document.dispatchEvent(new Event(SEARCH_FOCUS));
      return;
    }
    if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) {
      return;
    }
    const chord = Date.now() - this.pendingG < CHORD_MS;
    this.pendingG = 0;
    const target = chord ? GO_TARGETS[e.key] : undefined;
    if (target) {
      e.preventDefault();
      // Open sidebar panes are mirrored in the URL; carry them along.
      const panes = readPanes(new URLSearchParams(location.search));
      location.href = withPanes(new URL(target, readEnv().root).href, panes);
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
    const dialog = document.createElement("dialog");
    dialog.className = "shortcuts";
    const title = document.createElement("h2");
    title.textContent = "Keyboard shortcuts";
    const list = document.createElement("dl");
    for (const s of HELP) {
      if ((s.sidebar && !withSidebar) || (s.search && !hasSearch())) {
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

/** The search box renders only where it can work (over http). */
function hasSearch(): boolean {
  return document.querySelector("hermetic-search input") !== null;
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
