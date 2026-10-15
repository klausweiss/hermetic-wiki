// <hermetic-task-count>: the number of open tasks, inside the site navigation's "Open
// tasks" link. The number is not part of the HTML, so that a change to it alone changes
// no page but the task lists: it is loaded from `_hermetic/open-tasks.json`, only over
// http(s), and loaded again when `hermetic serve` rebuilds it. Until the first load
// finishes it shows the last count this browser saw, faded, or an empty badge on a first
// visit, so the link does not change width when the number arrives. Renders nothing for
// zero, over `file://`, or when the file cannot be loaded.

import { PAGES_CHANGED, readEnv } from "@hermetic/core";

import { COUNT_PATH, mentionsCount, parseCount, parseStored, storageKey } from "./count";

const connected = new Set<HermeticTaskCount>();
/** One request per page, shared by every instance; null once the count file changed. */
let loading: Promise<number | null> | null = null;
let listening = false;

function load(root: URL): Promise<number | null> {
  loading ??= fetch(new URL(COUNT_PATH, root), { cache: "no-store" }).then(
    async (response) => (response.ok ? parseCount(await response.text()) : null),
    () => null,
  );
  return loading;
}

/** The last count this browser saw for the site; storage may be unavailable. */
function remembered(root: URL): number | null {
  try {
    return parseStored(localStorage.getItem(storageKey(root)));
  } catch {
    return null;
  }
}

function remember(root: URL, count: number): void {
  try {
    localStorage.setItem(storageKey(root), String(count));
  } catch {
    // Without storage the next page starts from an empty badge.
  }
}

function listen(): void {
  if (listening) {
    return;
  }
  listening = true;
  document.addEventListener(PAGES_CHANGED, (e) => {
    if (mentionsCount((e as CustomEvent<{ changed: string[]; removed: string[] }>).detail)) {
      loading = null;
      for (const el of connected) {
        el.update();
      }
    }
  });
}

export class HermeticTaskCount extends HTMLElement {
  /** Incremented per update, so that an older response cannot replace a newer one. */
  private generation = 0;

  connectedCallback(): void {
    connected.add(this);
    listen();
    const env = readEnv();
    if (env.interactive && !this.hasChildNodes()) {
      this.renderPlaceholder(remembered(env.root));
    }
    this.update();
  }

  disconnectedCallback(): void {
    connected.delete(this);
  }

  update(): void {
    const env = readEnv();
    if (!env.interactive) {
      return;
    }
    const generation = ++this.generation;
    void load(env.root).then((count) => {
      if (generation === this.generation) {
        this.render(count);
      }
      if (count !== null) {
        remember(env.root, count);
      }
    });
  }

  private render(count: number | null): void {
    if (count === null || count === 0) {
      this.replaceChildren();
      return;
    }
    const badge = document.createElement("span");
    badge.className = "count";
    badge.textContent = String(count);
    this.replaceChildren(" ", badge);
  }

  /**
   * The badge until the count is loaded: the remembered count, faded and hidden from
   * assistive technology since it may be out of date; nothing when it was zero; an empty
   * badge one digit wide (a figure space) when there is none.
   */
  private renderPlaceholder(count: number | null): void {
    if (count === 0) {
      return;
    }
    const badge = document.createElement("span");
    badge.className = "count count-pending";
    badge.setAttribute("aria-hidden", "true");
    badge.textContent = count === null ? "\u2007" : String(count);
    this.replaceChildren(" ", badge);
  }
}

export function define(name = "hermetic-task-count"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticTaskCount);
  }
}
