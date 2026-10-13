// <hermetic-search> (the header's search button and dialog) and <hermetic-search-results>
// (search.html). Both render nothing until defined, and only over http(s): fetching the
// index needs it.

import { PAGES_CHANGED, SEARCH_FOCUS, SEARCH_TOGGLE, icon, readEnv, whenParsed } from "@hermetic/core";

import { search } from "./query";
import { type Result, highlight, href, resolve } from "./results";
import { FetchSource } from "./source";

const APPLE = /Mac|iPhone|iPad/.test(navigator.platform);

/** Results shown in the search dialog, and on the results page. */
export const BOX_RESULTS = 8;
export const PAGE_RESULTS = 200;
const DEBOUNCE_MS = 80;

let shared: FetchSource | null = null;

/** One index loader per page, forgotten when `hermetic serve` rebuilds the index. */
function sourceFor(root: URL): FetchSource {
  if (!shared) {
    const source = new FetchSource(root);
    document.addEventListener(PAGES_CHANGED, (e) => {
      const { changed, removed } = (e as CustomEvent<{ changed: string[]; removed: string[] }>).detail;
      if ([...changed, ...removed].some((p) => p.startsWith("search/"))) {
        source.invalidate();
        document.dispatchEvent(new Event(INDEX_CHANGED));
      }
    });
    shared = source;
  }
  return shared;
}

/** Dispatched on `document` after the index was invalidated, so open views re-run. */
const INDEX_CHANGED = "hermetic-search-index-changed";

async function find(root: URL, query: string, limit: number): Promise<Result[]> {
  const source = sourceFor(root);
  const { hits } = await search(source, query, limit);
  return resolve(source, await source.meta(), hits);
}

/** A result as a link: page title, breadcrumb, and preview with matched words marked. */
export function renderResult(root: URL, r: Result, tag: "a" = "a"): HTMLAnchorElement {
  const a = document.createElement(tag);
  a.className = `search-result search-result-${r.kind}`;
  a.href = href(root, r.path, r.anchor);
  const head = document.createElement("span");
  head.className = "search-result-head";
  const titleEl = document.createElement("span");
  titleEl.className = "search-result-title";
  titleEl.textContent = r.title;
  head.append(titleEl);
  if (r.breadcrumb.length > 0) {
    const crumbs = document.createElement("span");
    crumbs.className = "search-result-crumbs";
    crumbs.textContent = r.breadcrumb.join(" › ");
    head.append(crumbs);
  }
  a.append(head);
  const preview = document.createElement("span");
  preview.className = "search-result-preview";
  if (r.kind === "page") {
    for (const seg of highlight(r.title, r.matched)) appendSegment(preview, seg.text, seg.mark);
  } else {
    for (const seg of highlight(r.preview, r.matched)) appendSegment(preview, seg.text, seg.mark);
  }
  a.append(preview);
  return a;
}

function appendSegment(parent: HTMLElement, text: string, mark: boolean): void {
  if (mark) {
    const m = document.createElement("mark");
    m.textContent = text;
    parent.append(m);
  } else {
    parent.append(text);
  }
}

function note(results: Result[], query: string): string | null {
  if (results.length === 0) return `No results for “${query}”.`;
  if (results.every((r) => r.approximate)) return `No exact matches for “${query}”; showing similar words.`;
  if (results.every((r) => r.partial)) return `No page contains every word of “${query}”; showing pages with some of them.`;
  return null;
}

/** The search box in the header, with results as you type. */
export class HermeticSearch extends HTMLElement {
  private input: HTMLInputElement | null = null;
  private dialog!: HTMLDialogElement;
  /** What had focus before the dialog opened, focused again when it closes. */
  private returnFocus: HTMLElement | null = null;
  private list!: HTMLElement;
  private root!: URL;
  private selected = -1;
  private seq = 0;
  private timer = 0;

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  private setup(): void {
    const env = readEnv();
    if (this.input || !env.interactive) return;
    this.root = env.root;

    const button = document.createElement("button");
    button.type = "button";
    button.className = "search-button";
    const label = `Search (/ or ${APPLE ? "⌘ K" : "Ctrl + K"})`;
    button.setAttribute("aria-label", label);
    button.title = label;
    button.setAttribute("aria-haspopup", "dialog");
    button.append(icon("search"));
    button.addEventListener("click", () => this.open());

    // A modal dialog: in the top layer wherever it sits, the rest of the page inert while
    // it is open, and Escape closes it.
    const dialog = document.createElement("dialog");
    dialog.className = "search-dialog";
    dialog.setAttribute("aria-label", "Search the notes");
    const panel = document.createElement("div");
    panel.className = "search-panel";
    const form = document.createElement("form");
    form.role = "search";
    form.method = "get";
    form.action = new URL("search.html", env.root).href;
    const field = document.createElement("div");
    field.className = "search-field";
    const input = document.createElement("input");
    input.type = "search";
    input.name = "q";
    input.placeholder = "Search the notes";
    input.autocomplete = "off";
    input.setAttribute("aria-label", "Search the notes");
    input.setAttribute("aria-controls", "hermetic-search-list");
    input.setAttribute("aria-autocomplete", "list");
    input.value = new URL(location.href).pathname.endsWith("/search.html")
      ? (new URL(location.href).searchParams.get("q") ?? "")
      : "";
    field.append(icon("search"), input);
    this.list = document.createElement("div");
    this.list.id = "hermetic-search-list";
    this.list.className = "search-dropdown";
    this.list.setAttribute("role", "listbox");
    this.list.hidden = true;
    form.append(field, this.list);
    panel.append(form);
    dialog.append(panel);
    this.append(button, dialog);
    this.dialog = dialog;
    this.input = input;

    input.addEventListener("input", () => {
      clearTimeout(this.timer);
      this.timer = window.setTimeout(() => void this.update(), DEBOUNCE_MS);
    });
    input.addEventListener("keydown", (e) => this.onKey(e));
    form.addEventListener("submit", (e) => {
      const option = this.options()[this.selected];
      if (option) {
        e.preventDefault();
        option.click();
      } else if (!input.value.trim()) {
        e.preventDefault();
      }
    });
    // The dialog itself has no padding: a click on it, not on the panel, is on the backdrop.
    dialog.addEventListener("click", (e) => {
      if (e.target === dialog) this.closeDialog();
    });
    // Opening a result closes the dialog, also when Shift+click opens it in the sidebar.
    this.list.addEventListener("click", (e) => {
      if (e.target instanceof Element && e.target.closest("a")) this.closeDialog();
    });
    dialog.addEventListener("close", () => {
      this.close();
      const back = this.returnFocus;
      this.returnFocus = null;
      if (back?.isConnected) back.focus();
    });
    document.addEventListener(SEARCH_FOCUS, () => this.open());
    document.addEventListener(SEARCH_TOGGLE, () => (dialog.open ? this.closeDialog() : this.open()));
    document.addEventListener(INDEX_CHANGED, () => {
      if (!this.list.hidden) void this.update();
    });
  }

  /** The results and, after them, the link to all results: what the arrow keys move through. */
  private options(): HTMLAnchorElement[] {
    return [...this.list.querySelectorAll<HTMLAnchorElement>("a.search-result, a.search-all")];
  }

  private async update(): Promise<void> {
    const query = this.input?.value.trim() ?? "";
    const seq = ++this.seq;
    if (!query) {
      this.close();
      return;
    }
    let results: Result[];
    try {
      results = await find(this.root, query, BOX_RESULTS);
    } catch {
      results = [];
    }
    if (seq !== this.seq) return; // a newer query is running
    this.list.replaceChildren();
    const message = note(results, query);
    if (message) {
      const p = document.createElement("p");
      p.className = "search-note";
      p.textContent = message;
      this.list.append(p);
    }
    results.forEach((r, i) => {
      const a = renderResult(this.root, r);
      a.setAttribute("role", "option");
      a.id = `hermetic-search-option-${i}`;
      a.tabIndex = -1;
      this.list.append(a);
    });
    if (results.length > 0) {
      const all = document.createElement("a");
      all.className = "search-all";
      all.href = `${new URL("search.html", this.root).href}?q=${encodeURIComponent(query)}`;
      all.textContent = "All results →";
      all.setAttribute("role", "option");
      all.id = "hermetic-search-option-all";
      all.tabIndex = -1;
      this.list.append(all);
    }
    this.select(-1);
    this.list.hidden = false;
  }

  private select(i: number): void {
    const options = this.options();
    this.selected = options.length === 0 ? -1 : Math.max(-1, Math.min(i, options.length - 1));
    options.forEach((o, k) => o.setAttribute("aria-selected", String(k === this.selected)));
    const active = options[this.selected];
    if (active) {
      this.input?.setAttribute("aria-activedescendant", active.id);
      active.scrollIntoView({ block: "nearest" });
    } else {
      this.input?.removeAttribute("aria-activedescendant");
    }
  }

  private onKey(e: KeyboardEvent): void {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      this.list.hidden = false;
      this.select(this.selected + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      this.select(this.selected - 1);
    } else if (e.key === "Escape") {
      // A search field would spend the first Escape on clearing itself; close instead,
      // keeping the query for the next time search opens.
      e.preventDefault();
      this.closeDialog();
    }
  }

  private open(): void {
    if (!this.input) return;
    if (!this.dialog.open) {
      this.returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      this.dialog.showModal();
      if (this.input.value.trim() && this.list.childElementCount > 0) this.list.hidden = false;
    }
    this.input.focus();
    this.input.select();
  }

  private closeDialog(): void {
    if (this.dialog.open) this.dialog.close();
  }

  private close(): void {
    this.list.hidden = true;
    this.select(-1);
  }
}

/** The full list of results on search.html. */
export class HermeticSearchResults extends HTMLElement {
  private ready = false;

  connectedCallback(): void {
    whenParsed(() => void this.setup());
  }

  private async setup(): Promise<void> {
    const env = readEnv();
    if (this.ready || !env.interactive) return;
    this.ready = true;
    const unavailable = document.querySelector<HTMLElement>(".search-unavailable");
    if (unavailable) unavailable.hidden = true;
    const render = async () => {
      const query = (new URL(location.href).searchParams.get("q") ?? "").trim();
      this.replaceChildren();
      if (!query) {
        const p = document.createElement("p");
        p.className = "empty";
        p.textContent = "Type in the search box above to search the notes.";
        this.append(p);
        return;
      }
      let results: Result[];
      try {
        results = await find(env.root, query, PAGE_RESULTS);
      } catch {
        results = [];
      }
      const heading = document.createElement("p");
      heading.className = "search-summary";
      heading.textContent =
        note(results, query) ??
        `${results.length === PAGE_RESULTS ? `First ${PAGE_RESULTS}` : results.length} result${results.length === 1 ? "" : "s"} for “${query}”`;
      const list = document.createElement("div");
      list.className = "search-list";
      for (const r of results) list.append(renderResult(env.root, r));
      this.append(heading, list);
    };
    document.addEventListener(INDEX_CHANGED, () => void render());
    await render();
  }
}

export function define(): void {
  if (!customElements.get("hermetic-search")) customElements.define("hermetic-search", HermeticSearch);
  if (!customElements.get("hermetic-search-results")) {
    customElements.define("hermetic-search-results", HermeticSearchResults);
  }
}
