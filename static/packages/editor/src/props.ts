// Page properties: the `key:: value` lines of a page's first block when it has nothing
// else (see the parser's `is_properties_only`), read and written as a list of rows for
// the properties dialog.

import type { Doc, Node } from "./model";

export interface Property {
  key: string;
  value: string;
}

/** A property line as the parser reads it: key, `::`, then nothing or whitespace. */
const LINE = /^([A-Za-z0-9_-]+)::(?:[ \t](.*))?$/;
/** Characters a property key may not contain. */
const NOT_KEY = /[^A-Za-z0-9_-]/g;

/** A key with the characters the parser would not accept removed. */
export function cleanKey(key: string): string {
  return key.replace(NOT_KEY, "");
}

/** The block holding the page's properties: the first block, when it has only property lines. */
export function propsNode(doc: Doc): Node | null {
  const first = doc.roots[0];
  if (!first || first.body === "") return null;
  return first.body.split("\n").every((l) => LINE.test(l)) ? first : null;
}

export function pageProps(doc: Doc): Property[] {
  const node = propsNode(doc);
  if (!node) return [];
  return node.body.split("\n").map((l) => {
    const m = LINE.exec(l)!;
    return { key: m[1]!, value: (m[2] ?? "").trim() };
  });
}

/**
 * Writes `rows` as the page's properties; rows without a key are left out. Changes the
 * properties block in place, adds one at the top of the page, or removes it when no
 * property is left. Returns whether the outline changed.
 */
export function setPageProps(doc: Doc, rows: readonly Property[]): boolean {
  const props = rows
    .map((r) => ({ key: cleanKey(r.key), value: r.value.replace(/\s+/g, " ").trim() }))
    .filter((r) => r.key !== "");
  const before = pageProps(doc);
  if (props.length === before.length && props.every((p, i) => p.key === before[i]!.key && p.value === before[i]!.value)) {
    return false;
  }
  const body = props.map((p) => (p.value === "" ? `${p.key}::` : `${p.key}:: ${p.value}`)).join("\n");
  const node = propsNode(doc);
  if (node && props.length === 0) {
    doc.roots.splice(0, 1, ...node.children);
  } else if (node) {
    node.body = body;
  } else {
    // At the top of the file without a bullet, as page properties are usually written,
    // and followed by a blank line so that text below it stays a block of its own.
    const created: Node = { body, bullet: false, children: [] };
    if (doc.roots.length > 0) created.origin = { lines: [], trail: [""], depth: 0, body: "" };
    doc.roots.unshift(created);
  }
  return true;
}
