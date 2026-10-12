// Runs inside headless Chromium: executes one query with hermetic's search (the same
// source code the site ships) or with Pagefind, and reports pages, time, and bytes.

import { search } from "../../../static/packages/search/src/query";
import { resolve } from "../../../static/packages/search/src/results";
import { FetchSource } from "../../../static/packages/search/src/source";

/** Page key used to compare engines: site path without `.html`. */
const key = (path: string) => decodeURIComponent(path).replace(/^\/+/, "").replace(/\.html$/, "");

function unique(paths: string[]): string[] {
  return [...new Set(paths.map(key))];
}

interface Run {
  pages: string[];
  coldMs: number;
  warmMs: number;
  bytes: number;
}

async function hermetic(base: string, query: string): Promise<Run> {
  // Count the bytes of every index file fetched for this query (uncompressed).
  let bytes = 0;
  const realFetch = window.fetch;
  window.fetch = async (input, init) => {
    const response = await realFetch(input, init);
    bytes += (await response.clone().arrayBuffer()).byteLength;
    return response;
  };
  try {
    const source = new FetchSource(new URL(base, location.href));
    let t = performance.now();
    const cold = await search(source, query, 20);
    const results = await resolve(source, await source.meta(), cold.hits);
    const coldMs = performance.now() - t;
    t = performance.now();
    await search(source, query, 20);
    const warmMs = performance.now() - t;
    return { pages: unique(results.map((r) => r.path)), coldMs, warmMs, bytes };
  } finally {
    window.fetch = realFetch;
  }
}

interface Pagefind {
  options(o: Record<string, unknown>): Promise<void>;
  init(): Promise<void>;
  search(q: string): Promise<{ results: { data(): Promise<{ url: string }> }[] }>;
}

let pagefind: Pagefind | null = null;

async function pagefindRun(base: string, query: string): Promise<Run> {
  if (!pagefind) {
    pagefind = (await import(/* @vite-ignore */ `${base}pagefind/pagefind.js`)) as Pagefind;
    await pagefind.options({ baseUrl: base });
    await pagefind.init();
  }
  let t = performance.now();
  const found = await pagefind.search(query);
  const data = await Promise.all(found.results.slice(0, 20).map((r) => r.data()));
  const coldMs = performance.now() - t;
  t = performance.now();
  await pagefind.search(query);
  const warmMs = performance.now() - t;
  return { pages: unique(data.map((d) => d.url.slice(base.length - 1))), coldMs, warmMs, bytes: 0 };
}

(window as unknown as { bench: unknown }).bench = { hermetic, pagefind: pagefindRun };
