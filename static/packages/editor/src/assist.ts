// Helpers around the block being edited: `[[` and `((` completion lists built on the
// search index, and the bullet menu with "Copy reference".

import { PAGES_CHANGED } from "@hermetic/core";
import { FetchSource, type Result, resolve, search } from "@hermetic/search";

import { type HermeticEditor, addPlugin } from "./element";
import { type Node as Block, flatten } from "./model";
import { ensureId, ensureRemoteId } from "./refs";

const DEBOUNCE_MS = 80;
const ITEMS = 8;

interface Item {
  label: string;
  detail: string;
  /** The text to insert in place of the trigger and query. */
  text: () => Promise<string>;
}

interface List {
  start: number;
  items: Item[];
  selected: number;
  el: HTMLElement;
}

let source: FetchSource | null = null;
let list: List | null = null;
let timer = 0;
let seq = 0;

function sourceFor(root: URL): FetchSource {
  if (!source) {
    const s = new FetchSource(root);
    document.addEventListener(PAGES_CHANGED, (e) => {
      const { changed, removed } = (e as CustomEvent<{ changed: string[]; removed: string[] }>).detail;
      if ([...changed, ...removed].some((p) => p.startsWith("search/"))) s.invalidate();
    });
    source = s;
  }
  return source;
}

/** The completion the text before the cursor asks for, if any. */
export function trigger(before: string): { kind: "page" | "block"; start: number; query: string } | null {
  const page = /\[\[([^[\]\n]*)$/.exec(before);
  if (page) return { kind: "page", start: page.index, query: page[1]! };
  const block = /\(\(([^()\n]*)$/.exec(before);
  if (block) return { kind: "block", start: block.index, query: block[1]! };
  return null;
}

async function find(editor: HermeticEditor, query: string, limit: number): Promise<Result[]> {
  const s = sourceFor(editor.root);
  const { hits } = await search(s, query, limit);
  return resolve(s, await s.meta(), hits);
}

async function pageItems(editor: HermeticEditor, query: string): Promise<Item[]> {
  const typed = query.trim();
  if (!typed) return [];
  const pages = (await find(editor, typed, 40).catch(() => [])).filter((r) => r.kind === "page").slice(0, ITEMS);
  // A page found by an alias links by that alias, marked as one.
  const items: Item[] = pages.map((r) => {
    const name = r.alias || r.name;
    return { label: name, detail: r.alias ? `Alias of ${r.name}` : "", text: async () => `[[${name}]]` };
  });
  if (!pages.some((r) => (r.alias || r.name).toLowerCase() === typed.toLowerCase())) {
    items.push({ label: typed, detail: "New page", text: async () => `[[${typed}]]` });
  }
  return items;
}

async function blockItems(editor: HermeticEditor, query: string): Promise<Item[]> {
  if (!query.trim()) return [];
  const blocks = (await find(editor, query.trim(), 40).catch(() => [])).filter((r) => r.kind === "block").slice(0, ITEMS);
  return blocks.map((r) => ({
    label: r.preview,
    detail: [r.title, ...r.breadcrumb].join(" › "),
    text: async () => `((${await idOf(editor, r.path, r.anchor)}))`,
  }));
}

/** The id of a block, giving it one (on this page or on its own page) if needed. */
async function idOf(editor: HermeticEditor, path: string, anchor: string): Promise<string> {
  if (path === editor.path && editor.doc) {
    const node = flatten(editor.doc).find((e) => e.node.anchor === anchor)?.node;
    if (node) return idHere(editor, node);
  }
  return ensureRemoteId(editor.api, path, anchor);
}

/** The id of a block of the page being edited; a new id is saved with the page. */
function idHere(editor: HermeticEditor, node: Block): string {
  const { id, added } = ensureId(node);
  if (added) {
    if (editor.editing?.node === node) editor.editing.area.value = node.body;
    editor.render();
    editor.changed();
  }
  return id;
}

function close(): void {
  list?.el.remove();
  list = null;
  seq++;
}

function show(editor: HermeticEditor, area: HTMLTextAreaElement, start: number, items: Item[]): void {
  list?.el.remove();
  if (items.length === 0) {
    list = null;
    return;
  }
  const el = document.createElement("div");
  el.className = "complete-list";
  el.setAttribute("role", "listbox");
  items.forEach((item, i) => {
    const option = document.createElement("div");
    option.className = "complete-item";
    option.setAttribute("role", "option");
    const label = document.createElement("span");
    label.className = "complete-label";
    label.textContent = item.label;
    option.append(label);
    if (item.detail) {
      const detail = document.createElement("span");
      detail.className = "complete-detail";
      detail.textContent = item.detail;
      option.append(detail);
    }
    // Keep the focus (and the cursor) in the text field.
    option.addEventListener("mousedown", (e) => e.preventDefault());
    option.addEventListener("click", () => void choose(editor, area, i));
    el.append(option);
  });
  const rect = area.getBoundingClientRect();
  el.style.left = `${Math.max(8, Math.min(rect.left, innerWidth - 320))}px`;
  el.style.top = `${rect.bottom + 4}px`;
  editor.append(el);
  list = { start, items, selected: 0, el };
  select(0);
}

function select(i: number): void {
  if (!list) return;
  list.selected = (i + list.items.length) % list.items.length;
  [...list.el.children].forEach((c, k) => c.setAttribute("aria-selected", String(k === list!.selected)));
}

async function choose(editor: HermeticEditor, area: HTMLTextAreaElement, i: number): Promise<void> {
  const current = list;
  const item = current?.items[i];
  if (!current || !item) return;
  close();
  let text: string;
  try {
    text = await item.text();
  } catch {
    return;
  }
  const end = area.selectionStart;
  area.value = area.value.slice(0, current.start) + text + area.value.slice(end);
  const at = current.start + text.length;
  area.focus();
  area.setSelectionRange(at, at);
  area.dispatchEvent(new Event("input", { bubbles: true }));
}

addPlugin({
  input(editor, area) {
    // Not while text is selected, as after wrapping it in `[[`..`]]`.
    const t = area.selectionStart === area.selectionEnd ? trigger(area.value.slice(0, area.selectionStart)) : null;
    clearTimeout(timer);
    if (!t) {
      close();
      return;
    }
    const mine = ++seq;
    timer = window.setTimeout(async () => {
      const items = await (t.kind === "page" ? pageItems(editor, t.query) : blockItems(editor, t.query));
      if (mine === seq && editor.editing?.area === area) show(editor, area, t.start, items);
    }, DEBOUNCE_MS);
  },
  key(editor, e, area) {
    if (!list) return false;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      select(list.selected + (e.key === "ArrowDown" ? 1 : -1));
      return true;
    }
    if ((e.key === "Enter" || e.key === "Tab") && !e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      void choose(editor, area, list.selected);
      return true;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      close();
      return true;
    }
    return false;
  },
  end() {
    close();
  },
  decorate(editor, li, node) {
    if (!node.bullet) return;
    const handle = document.createElement("button");
    handle.type = "button";
    handle.className = "block-handle";
    handle.title = "Block actions";
    handle.setAttribute("aria-label", "Block actions");
    handle.setAttribute("aria-haspopup", "menu");
    handle.addEventListener("click", (e) => {
      e.stopPropagation();
      openMenu(editor, handle, node);
    });
    li.prepend(handle);
  },
});

let menu: HTMLElement | null = null;

function closeMenu(): void {
  menu?.remove();
  menu = null;
}

function openMenu(editor: HermeticEditor, handle: HTMLElement, node: Block): void {
  closeMenu();
  const el = document.createElement("div");
  el.className = "block-menu";
  el.setAttribute("role", "menu");
  const copy = document.createElement("button");
  copy.type = "button";
  copy.setAttribute("role", "menuitem");
  copy.textContent = "Copy reference";
  copy.addEventListener("click", async () => {
    const ref = `((${idHere(editor, node)}))`;
    try {
      await navigator.clipboard.writeText(ref);
      copy.textContent = `Copied ${ref}`;
    } catch {
      copy.textContent = ref;
    }
    setTimeout(closeMenu, 900);
  });
  el.append(copy);
  const rect = handle.getBoundingClientRect();
  el.style.left = `${rect.left}px`;
  el.style.top = `${rect.bottom + 4}px`;
  editor.append(el);
  menu = el;
  copy.focus();
  const away = (e: Event) => {
    if (e instanceof KeyboardEvent ? e.key === "Escape" : !el.contains(e.target as Node)) {
      closeMenu();
      document.removeEventListener("mousedown", away, true);
      document.removeEventListener("keydown", away, true);
    }
  };
  document.addEventListener("mousedown", away, true);
  document.addEventListener("keydown", away, true);
}
