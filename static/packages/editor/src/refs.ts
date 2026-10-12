// Block ids for references: reading a block's id, and giving a block a generated one.

import { Conflict, type SourceApi } from "./api";
import { type Node, flatten, fromPayload } from "./model";
import { serialize } from "./serialize";

const PROPERTY = /^[A-Za-z0-9_-]+::(\s|$)/;
const ID = /^id::[ \t]+(\S+)[ \t]*$/i;
const ID_CHARS = "abcdefghijklmnopqrstuvwxyz0123456789";

/** A random id: 8 characters from a–z and 0–9. */
export function newId(random: (n: number) => Uint8Array = (n) => crypto.getRandomValues(new Uint8Array(n))): string {
  return Array.from(random(8), (b) => ID_CHARS[b % ID_CHARS.length]).join("");
}

/** Index of the line after the block's property lines (which follow its first line). */
function propertyEnd(lines: string[]): number {
  let i = lines.length > 0 && PROPERTY.test(lines[0]!) ? 0 : 1;
  while (i < lines.length && PROPERTY.test(lines[i]!)) i++;
  return Math.min(i, lines.length);
}

/** The block's `id::` property, if it has a usable one. */
export function blockId(node: Node): string | null {
  const lines = node.body.split("\n");
  for (const line of lines.slice(0, propertyEnd(lines))) {
    const m = ID.exec(line);
    if (m && !/[()]/.test(m[1]!)) return m[1]!;
  }
  return null;
}

/**
 * Adds `id:: <id>` after the block's first line and its property lines. When the block
 * is otherwise unchanged, the line is also added to its original lines (indented like
 * its other property lines, or to its content column), so saving changes nothing else.
 */
export function addId(node: Node, id: string): void {
  const before = node.body;
  const lines = before.split("\n");
  const at = propertyEnd(lines);
  lines.splice(at, 0, `id:: ${id}`);
  node.body = lines.join("\n");
  const o = node.origin;
  if (o && o.body === before && o.lines.length === lines.length - 1 && at > 0) {
    const previous = o.lines[at - 1]!;
    const space = /^[ \t]*/.exec(previous)![0];
    const indent = at - 1 === 0 && node.bullet ? `${space}  ` : space;
    o.lines.splice(at, 0, `${indent}id:: ${id}`);
    o.body = node.body;
  }
}

/** The id of `node`, giving it a new one if needed (the caller saves). */
export function ensureId(node: Node): { id: string; added: boolean } {
  const existing = blockId(node);
  if (existing) return { id: existing, added: false };
  const id = newId();
  addId(node, id);
  return { id, added: true };
}

/**
 * The id of the block at `anchor` on another page, adding one to that page's file when
 * the block has none. Retries once when the page changed meanwhile.
 */
export async function ensureRemoteId(api: SourceApi, path: string, anchor: string): Promise<string> {
  if (anchor.startsWith("id-")) return anchor.slice(3);
  for (let attempt = 0; ; attempt++) {
    const payload = await api.get(path);
    const doc = fromPayload(payload);
    const node = flatten(doc).find((e) => e.node.anchor === anchor)?.node;
    if (!node) throw new Error(`no block ${anchor} on ${path}`);
    const { id, added } = ensureId(node);
    if (!added) return id;
    try {
      await api.put(path, payload.version, serialize(doc));
      return id;
    } catch (e) {
      if (!(e instanceof Conflict) || attempt > 0) throw e;
    }
  }
}
