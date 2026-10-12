// Turning hits into results to show: page, breadcrumb, preview, and highlighting.

import { words } from "./normalize";
import type { Hit } from "./query";
import type { Chunk, IndexSource, Meta } from "./source";

export interface Result {
  kind: "page" | "block";
  /** Site path of the page, not percent-encoded. */
  path: string;
  /** Block anchor, "" for page results. */
  anchor: string;
  /** The page's name. */
  name: string;
  title: string;
  breadcrumb: string[];
  preview: string;
  matched: Set<string>;
  partial: boolean;
  approximate: boolean;
}

const CRUMB_CHARS = 40;
const WEEKDAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/** Shortens text at a word boundary, like the indexer does. */
export function truncate(text: string, max: number): string {
  const chars = [...text];
  if (chars.length <= max) return text;
  let cut = chars.slice(0, max).join("");
  const space = cut.lastIndexOf(" ");
  if (space > max / 2) cut = cut.slice(0, space);
  return `${cut.trimEnd()}…`;
}

/** Page names as titles: journal dates get their weekday, like page headings. */
export function title(name: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(name);
  if (!m) return name;
  const day = new Date(Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3])));
  return Number.isNaN(day.getTime()) ? name : `${name} ${WEEKDAYS[day.getUTCDay()]}`;
}

export async function resolve(source: IndexSource, meta: Meta, hits: Hit[]): Promise<Result[]> {
  const chunks = new Map<number, Promise<Chunk>>();
  const entry = async (doc: number) => {
    const n = Math.floor(doc / meta.doc_chunk);
    let chunk = chunks.get(n);
    if (!chunk) {
      chunk = source.chunk(n);
      chunks.set(n, chunk);
    }
    const c = await chunk;
    const e = c.docs[doc % meta.doc_chunk];
    if (!e) throw new Error(`document ${doc} is missing from the index`);
    const page = c.pages[e[0]] ?? ["", ""];
    return { path: page[0], name: page[1], anchor: e[1], parent: e[2], preview: e[3] };
  };
  return Promise.all(
    hits.map(async (hit) => {
      const e = await entry(hit.doc);
      const breadcrumb: string[] = [];
      let doc = hit.doc;
      let parent = e.parent;
      while (parent > 0) {
        doc -= parent;
        const p = await entry(doc);
        breadcrumb.unshift(truncate(p.preview, CRUMB_CHARS));
        parent = p.parent;
      }
      return {
        kind: e.anchor === "" ? ("page" as const) : ("block" as const),
        path: e.path,
        anchor: e.anchor,
        name: e.name,
        title: title(e.name),
        breadcrumb,
        preview: e.preview,
        matched: hit.matched,
        partial: hit.partial,
        approximate: hit.approximate,
      };
    }),
  );
}

export interface Segment {
  text: string;
  mark: boolean;
}

/** Splits text into segments, marking the words whose normalized form matched. */
export function highlight(text: string, matched: Set<string>): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  for (const m of text.matchAll(/[\p{Alphabetic}\p{N}\p{M}]+/gu)) {
    const start = m.index;
    const word = m[0];
    if (words(word).some((w) => matched.has(w))) {
      if (start > last) out.push({ text: text.slice(last, start), mark: false });
      out.push({ text: word, mark: true });
      last = start + word.length;
    }
  }
  if (last < text.length) out.push({ text: text.slice(last), mark: false });
  return out;
}

/** The URL path of a site path: each segment percent-encoded. */
export function href(root: URL, path: string, anchor: string): string {
  const url = new URL(path.split("/").map(encodeURIComponent).join("/"), root);
  if (anchor) url.hash = anchor;
  return url.href;
}
