// The search index files written by `src/search/index.rs`, and loading them.

/** `search/meta.json`. */
export interface Meta {
  version: number;
  /** Changes whenever any index file changes; used to bypass stale caches. */
  id: string;
  n_docs: number;
  /** Average field lengths: [page names, block text]. */
  avg_len: [number, number];
  k1: number;
  b: number;
  /** First and last document of file-backed journal days, in date order. */
  journal_docs: [number, number] | null;
  doc_chunk: number;
  /** Hex keys of the existing shards. */
  shards: string[];
}

/** One term of a shard: the word, its stem ("" when equal), and packed postings. */
export type Term = [term: string, stem: string, postings: string];

/** `search/d/<n>.json`: documents with their pages. */
export interface Chunk {
  pages: [path: string, name: string][];
  /** [page index, anchor ("" for page documents), parent delta, preview] */
  docs: [number, string, number, string][];
}

/** Where index files come from: the network in the browser, files in tests. */
export interface IndexSource {
  meta(): Promise<Meta>;
  shard(key: string): Promise<Term[]>;
  chunk(n: number): Promise<Chunk>;
}

/** Hex of the UTF-8 bytes of the first two characters: the shard holding `word`. */
export function shardKey(word: string): string {
  const prefix = [...word].slice(0, 2).join("");
  return [...new TextEncoder().encode(prefix)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** Loads index files over HTTP, caching them until the index changes. */
export class FetchSource implements IndexSource {
  private readonly base: URL;
  private metaPromise: Promise<Meta> | null = null;
  private readonly cache = new Map<string, Promise<unknown>>();

  constructor(root: URL) {
    this.base = new URL("search/", root);
  }

  /** Forgets everything loaded, e.g. after `hermetic serve` rebuilt the index. */
  invalidate(): void {
    this.metaPromise = null;
    this.cache.clear();
  }

  meta(): Promise<Meta> {
    this.metaPromise ??= fetchJson<Meta>(new URL("meta.json", this.base), "no-store");
    return this.metaPromise;
  }

  shard(key: string): Promise<Term[]> {
    return this.cached(`t/${key}.json`);
  }

  chunk(n: number): Promise<Chunk> {
    return this.cached(`d/${n}.json`);
  }

  private cached<T>(path: string): Promise<T> {
    let p = this.cache.get(path) as Promise<T> | undefined;
    if (!p) {
      p = this.meta().then((meta) => {
        const url = new URL(path, this.base);
        // The index id makes a new URL whenever the index changes.
        url.searchParams.set("v", meta.id);
        return fetchJson<T>(url, "default");
      });
      this.cache.set(path, p);
    }
    return p;
  }
}

async function fetchJson<T>(url: URL, cache: RequestCache): Promise<T> {
  const response = await fetch(url.href, { cache });
  if (!response.ok) {
    throw new Error(`cannot load ${url.pathname}: ${response.status}`);
  }
  return (await response.json()) as T;
}
