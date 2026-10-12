// <hermetic-live-reload>: inserted by `hermetic serve` into the pages it serves (built
// sites never contain it). After each rebuild the server lists the pages that changed:
// this page reloads only if it is one of them, keeping its scroll position; otherwise
// other components (the sidebar) are told, so they can refresh what they show.

import { PAGES_CHANGED, readEnv, whenParsed } from "@hermetic/core";

import { affects, parseChanges, sitePath } from "./changes";

const SCROLL_KEY = "hermetic-live-reload-scroll:";

export class HermeticLiveReload extends HTMLElement {
  private source: EventSource | null = null;

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  disconnectedCallback(): void {
    this.source?.close();
    this.source = null;
  }

  private setup(): void {
    const env = readEnv();
    if (this.source || !env.interactive) {
      return;
    }
    const path = sitePath(location.pathname, env.root.pathname);
    if (path) {
      restoreScroll(path);
    }
    // EventSource reconnects by itself when the server goes away and comes back.
    this.source = new EventSource(new URL("_hermetic/events", env.root));
    this.source.addEventListener("message", (e) => {
      const changes = parseChanges(e.data);
      if (!changes) {
        return;
      }
      if (path && affects(changes, path)) {
        saveScroll(path);
        location.reload();
      } else {
        document.dispatchEvent(new CustomEvent(PAGES_CHANGED, { detail: changes }));
      }
    });
  }
}

function saveScroll(path: string): void {
  try {
    sessionStorage.setItem(SCROLL_KEY + path, String(scrollY));
  } catch {
    // Without storage the browser's own scroll restoration still applies.
  }
}

function restoreScroll(path: string): void {
  try {
    const saved = sessionStorage.getItem(SCROLL_KEY + path);
    if (saved !== null) {
      sessionStorage.removeItem(SCROLL_KEY + path);
      scrollTo(0, Number(saved));
    }
  } catch {
    // Storage unavailable: nothing to restore.
  }
}

export function define(name = "hermetic-live-reload"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticLiveReload);
  }
}
