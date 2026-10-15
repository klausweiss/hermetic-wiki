// <hermetic-task-count>: the number of open tasks, inside the site navigation's "Open
// tasks" link. The number is not part of the HTML, so that a change to it alone changes
// no page but the task lists: it is loaded from `_hermetic/open-tasks.json`, only over
// http(s), and loaded again when `hermetic serve` rebuilds it. Renders nothing for zero,
// over `file://`, or when the file cannot be loaded.

import { PAGES_CHANGED, readEnv } from "@hermetic/core";

import { COUNT_PATH, mentionsCount, parseCount } from "./count";

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
}

export function define(name = "hermetic-task-count"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticTaskCount);
  }
}
