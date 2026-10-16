// The outliner's operations, as pure changes to a Doc. Each returns where editing
// continues (a node and a cursor offset in its body), or null when it does not apply.

import { type Doc, type Node, flatten, locate, newNode } from "./model";

export interface Focus {
  node: Node;
  offset: number;
}

const PROPERTY = /^[A-Za-z0-9_-]+::(\s|$)/;
const FENCE = /^\s*(`{3,}|~{3,})/;

/** Whether `offset` lies inside a code fence that is still open at that point. */
export function insideOpenFence(body: string, offset: number): boolean {
  let open: string | null = null;
  // Every line up to the cursor, including the part of its own line before it.
  for (const line of body.slice(0, offset).split("\n")) {
    const m = FENCE.exec(line);
    if (!m) continue;
    const run = m[1]!;
    if (open === null) open = run;
    else if (run[0] === open[0] && run.length >= open.length && line.trim() === run) open = null;
  }
  return open !== null;
}

/**
 * Enter: the text after the cursor becomes a new block directly below (first child if
 * the block has children, else next sibling). Property lines right after the first
 * line stay with the block when the cursor is on its first line.
 */
export function split(doc: Doc, node: Node, offset: number): Focus | null {
  if (insideOpenFence(node.body, offset)) return null;
  const place = locate(doc, node);
  if (!place) return null;
  const lines = node.body.split("\n");
  const firstEnd = lines[0]!.length;
  let keep: string;
  let move: string;
  if (offset <= firstEnd) {
    let props = 1;
    while (props < lines.length && PROPERTY.test(lines[props]!)) props++;
    keep = [lines[0]!.slice(0, offset), ...lines.slice(1, props)].join("\n");
    move = [lines[0]!.slice(offset), ...lines.slice(props)].join("\n");
  } else {
    keep = node.body.slice(0, offset);
    move = node.body.slice(offset);
    if (move.startsWith("\n")) move = move.slice(1);
  }
  node.body = keep;
  const created = newNode(move);
  if (node.children.length > 0) node.children.unshift(created);
  else place.siblings.splice(place.index + 1, 0, created);
  return { node: created, offset: 0 };
}

/** Tab: become the last child of the previous sibling. */
export function indent(doc: Doc, node: Node, offset: number): Focus | null {
  const place = locate(doc, node);
  if (!place || place.index === 0 || !node.bullet) return null;
  const prev = place.siblings[place.index - 1]!;
  if (!prev.bullet) return null;
  place.siblings.splice(place.index, 1);
  prev.children.push(node);
  return { node, offset };
}

/** Shift+Tab: become the next sibling of the parent; later siblings stay where they are. */
export function outdent(doc: Doc, node: Node, offset: number): Focus | null {
  const place = locate(doc, node);
  if (!place || !place.parent || !node.bullet) return null;
  const outer = locate(doc, place.parent)!;
  place.siblings.splice(place.index, 1);
  outer.siblings.splice(outer.index + 1, 0, node);
  return { node, offset };
}

/** Alt+Shift+Up/Down: swap with the previous or next sibling. */
export function move(doc: Doc, node: Node, offset: number, by: -1 | 1): Focus | null {
  const place = locate(doc, node);
  if (!place) return null;
  const other = place.index + by;
  if (other < 0 || other >= place.siblings.length) return null;
  place.siblings[place.index] = place.siblings[other]!;
  place.siblings[other] = node;
  return { node, offset };
}

/** The block before or after `node` in document order. */
export function neighbor(doc: Doc, node: Node, by: -1 | 1): Node | null {
  const all = flatten(doc);
  const i = all.findIndex((e) => e.node === node);
  return all[i + by]?.node ?? null;
}

/**
 * Backspace at the start of a block: delete it when it is empty and has no children,
 * otherwise append it to the previous block when that block has no children.
 */
export function backspace(doc: Doc, node: Node): Focus | null {
  const place = locate(doc, node);
  const prev = neighbor(doc, node, -1);
  if (!place || !prev) return null;
  if (node.body === "" && node.children.length === 0) {
    place.siblings.splice(place.index, 1);
    return { node: prev, offset: prev.body.length };
  }
  // Text outside bullets cannot have children.
  if (prev.children.length > 0 || (!prev.bullet && node.children.length > 0)) return null;
  const offset = prev.body.length;
  prev.body += node.body;
  prev.children = node.children;
  place.siblings.splice(place.index, 1);
  return { node: prev, offset };
}

const MARKERS = ["TODO", "DOING", "DONE"] as const;

/** Ctrl/Cmd+Enter: none → TODO → DOING → DONE → none. Keeps the cursor on the same text. */
export function cycleTask(node: Node, offset: number): Focus {
  const nl = node.body.indexOf("\n");
  const first = nl < 0 ? node.body : node.body.slice(0, nl);
  const rest = nl < 0 ? "" : node.body.slice(nl);
  const current = MARKERS.findIndex((m) => first === m || first.startsWith(`${m} `));
  const oldPrefix = current < 0 ? "" : first.length === MARKERS[current]!.length ? MARKERS[current]! : `${MARKERS[current]} `;
  const text = first.slice(oldPrefix.length);
  const next = current + 1 < MARKERS.length ? `${MARKERS[current + 1]} ` : "";
  node.body = next + text + rest;
  const shifted = offset - oldPrefix.length + next.length;
  return { node, offset: Math.max(next.length, Math.min(shifted, node.body.length)) };
}

/** A click on a task label: TODO → DOING → DONE → TODO. Null when the block is no task. */
export function advanceTask(body: string): string | null {
  const nl = body.indexOf("\n");
  const first = nl < 0 ? body : body.slice(0, nl);
  const current = MARKERS.findIndex((m) => first === m || first.startsWith(`${m} `));
  if (current < 0) return null;
  const next = MARKERS[(current + 1) % MARKERS.length]!;
  return next + body.slice(MARKERS[current]!.length);
}

/** Where an image goes: a block and a cursor offset in its body, or the end of its text
 * (`offset` null), or a new last top-level block (`node` null or no longer in the doc). */
export interface Insertion {
  node: Node | null;
  offset: number | null;
}

/** The inserted range of a block's body. */
export interface Inserted {
  node: Node;
  start: number;
  end: number;
}

/**
 * The end of a block's text before the property lines that end it (the first line
 * always counts as text).
 */
export function textEnd(body: string): number {
  const lines = body.split("\n");
  let last = lines.length - 1;
  while (last > 0 && PROPERTY.test(lines[last]!)) last--;
  return lines.slice(0, last + 1).join("\n").length;
}

/**
 * Inserts image markdown at `at`: at the cursor offset as is, or at the end of the
 * block's text after a space. `more` marks a further image of the same paste or drop,
 * which goes on its own line. Without the block, it starts a new last top-level block.
 */
export function insertImage(doc: Doc, at: Insertion, markdown: string, more: boolean): Inserted {
  const node = at.node && locate(doc, at.node) ? at.node : null;
  if (!node) {
    const created = newNode(markdown);
    doc.roots.push(created);
    return { node: created, start: 0, end: markdown.length };
  }
  const body = node.body;
  let offset: number;
  let text: string;
  if (more) {
    offset = Math.min(at.offset ?? body.length, body.length);
    text = `\n${markdown}`;
  } else if (at.offset === null) {
    offset = textEnd(body);
    const before = body.slice(0, offset);
    text = before === "" || /\s$/.test(before) ? markdown : ` ${markdown}`;
  } else {
    offset = Math.max(0, Math.min(at.offset, body.length));
    text = markdown;
  }
  node.body = body.slice(0, offset) + text + body.slice(offset);
  return { node, start: offset, end: offset + text.length };
}
