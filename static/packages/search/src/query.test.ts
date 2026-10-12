// Query scenarios from the `search` spec, over the index the Rust indexer writes for
// `tests/fixtures/search-notes` (kept current by `tests/search_index.rs`).
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import { search, withinOneEdit } from "./query";
import { highlight, resolve, title, type Result } from "./results";
import type { Chunk, IndexSource, Meta, Term } from "./source";

const dir = new URL("../test-index/", import.meta.url);
const read = <T>(path: string): T => JSON.parse(readFileSync(new URL(path, dir), "utf8")) as T;

const source: IndexSource = {
  meta: async () => read<Meta>("meta.json"),
  shard: async (key) => read<Term[]>(`t/${key}.json`),
  chunk: async (n) => read<Chunk>(`d/${n}.json`),
};

async function results(query: string, limit = 20): Promise<Result[]> {
  const { hits } = await search(source, query, limit);
  return resolve(source, await source.meta(), hits);
}

test("prefix while typing", async () => {
  const r = await results("authen");
  expect(r[0]?.preview).toBe("authentication with tokens");
  expect(r[0]?.matched.has("authentication")).toBeTrue();
});

test("all words required", async () => {
  const r = await results("redis ttl");
  expect(r.map((x) => x.preview)).toEqual(["the redis ttl was wrong"]);
  expect(r[0]?.partial).toBeFalse();
  expect(r[0]?.breadcrumb).toEqual(["authentication with tokens"]);
});

test("partial matches when no result has every word", async () => {
  const r = await results("kafka zeppelin");
  expect(r.length > 0).toBeTrue();
  expect(r.every((x) => x.partial)).toBeTrue();
});

test("typo", async () => {
  const r = await results("kafla");
  expect(r.length > 0).toBeTrue();
  expect(r.every((x) => x.approximate)).toBeTrue();
  expect(r.some((x) => x.path === "pages/kafka.html")).toBeTrue();
});

test("diacritics", async () => {
  for (const q of ["zazolc", "gesla", "ZAŻÓŁĆ"]) {
    const r = await results(q);
    expect(r[0]?.preview).toBe("Zażółć gęślą jaźń");
  }
});

test("inflection", async () => {
  const r = await results("tested");
  expect(r[0]?.preview).toBe("we were testing the parser");
});

test("exact page name first", async () => {
  const r = await results("kafka");
  expect(r[0]?.kind).toBe("page");
  expect(r[0]?.path).toBe("pages/kafka.html");
  expect(r.length > 3).toBeTrue();
});

test("recency breaks ties", async () => {
  const r = await results("deploy freeze");
  expect(r.slice(0, 2).map((x) => x.path)).toEqual(["journals/2026-10-01.html", "journals/2025-01-10.html"]);
  expect(r[0]?.title).toBe("2026-10-01 Thursday");
});

test("block result for a linked journal block", async () => {
  const r = await results("refresh");
  expect(r).toHaveLength(1);
  expect(r[0]?.path).toBe("journals/2026-10-12.html");
  expect(r[0]?.anchor).toBe("b2");
  expect(r[0]?.breadcrumb).toEqual(["standup"]);
});

test("highlighting", () => {
  const segs = highlight("Zażółć gęślą jaźń", new Set(["gesla"]));
  expect(segs).toEqual([
    { text: "Zażółć ", mark: false },
    { text: "gęślą", mark: true },
    { text: " jaźń", mark: false },
  ]);
});

test("edit distance", () => {
  expect(withinOneEdit("kafla", "kafka")).toBeTrue();
  expect(withinOneEdit("kafak", "kafka")).toBeTrue();
  expect(withinOneEdit("kafk", "kafka")).toBeTrue();
  expect(withinOneEdit("kafka", "kafka")).toBeFalse();
  expect(withinOneEdit("kxfla", "kafka")).toBeFalse();
  expect(title("2026-10-12")).toBe("2026-10-12 Monday");
  expect(title("kafka")).toBe("kafka");
});
