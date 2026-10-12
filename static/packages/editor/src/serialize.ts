// Writing an outline back to file text so that parsing it gives the same tree.
//
// A node at its original depth keeps its original indentation whenever that still
// places it correctly: its bullet must be indented further than its parent's and (when
// it has a previous sibling) no further than that sibling's, which is how the parser
// decides nesting.
// Then an unchanged node is copied from its original lines byte for byte, and a node
// whose text changed is written from its body at that indentation. Otherwise it takes
// the indentation of its previous sibling, or its parent's plus the file's unit.
// Blank lines after a node travel with it.

import type { Doc, Node } from "./model";

/** Columns a tab advances to (as in the parser). */
const TAB_WIDTH = 4;

export function serialize(doc: Doc): string {
  const out: string[] = [...doc.head];
  const walk = (nodes: Node[], depth: number, parentPrefix: string | null) => {
    let prev: string | null = null;
    for (const node of nodes) {
      const o = node.origin;
      let prefix = "";
      if (node.bullet) {
        const original = o && o.depth === depth ? leadingSpace(o.lines[0] ?? "") : null;
        prefix =
          original !== null && fits(original, parentPrefix, prev)
            ? original
            : (prev ?? (parentPrefix === null ? "" : parentPrefix + doc.indent));
      }
      if (o && o.body === node.body && o.depth === depth && (!node.bullet || prefix === leadingSpace(o.lines[0] ?? ""))) {
        out.push(...o.lines);
      } else {
        out.push(...render(node, prefix));
      }
      if (o) out.push(...o.trail);
      // Text outside bullets ends every open bullet: the next bullet starts afresh.
      prev = node.bullet ? prefix : null;
      walk(node.children, depth + 1, prefix);
    }
  };
  walk(doc.roots, 0, null);
  if (out.length === 0) return "";
  return out.join("\n") + (doc.finalNewline ? "\n" : "");
}

/** Whether a bullet indented by `prefix` nests under `parent` and after `prev`. */
function fits(prefix: string, parent: string | null, prev: string | null): boolean {
  const col = width(prefix);
  if (parent !== null && col <= width(parent)) return false;
  if (prev !== null && col > width(prev)) return false;
  return true;
}

function width(prefix: string): number {
  let col = 0;
  for (const c of prefix) col = c === "\t" ? col + TAB_WIDTH - (col % TAB_WIDTH) : col + 1;
  return col;
}

function leadingSpace(line: string): string {
  return /^[ \t]*/.exec(line)![0];
}

/** A block's lines from its body: `- ` and continuation lines at its content column. */
export function render(node: Node, prefix: string): string[] {
  const lines = node.body.split("\n");
  if (!node.bullet) return lines;
  return lines.map((line, i) => {
    if (i === 0) return line === "" ? `${prefix}-` : `${prefix}- ${line}`;
    return line === "" ? "" : `${prefix}  ${line}`;
  });
}
