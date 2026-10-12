// The shared vectors also checked by the Rust tests: both sides must agree exactly.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import { words } from "./normalize";
import { unpack } from "./pack";
import { stem } from "./stem";

const fixture = (name: string) =>
  readFileSync(new URL(`../../../../tests/fixtures/search/${name}`, import.meta.url), "utf8")
    .split("\n")
    .filter((l) => l !== "" && !l.startsWith("##"));

test("normalization vectors", () => {
  const rows = fixture("normalize.tsv");
  expect(rows.length >= 50).toBeTrue();
  for (const row of rows) {
    const [input = "", expected = ""] = row.split("\t");
    const text = input.replaceAll("\\t", "\t").replaceAll("\\n", "\n");
    expect(words(text)).toEqual(expected.split(" ").filter((w) => w !== ""));
  }
});

test("Porter2 vectors", () => {
  const rows = fixture("porter2.tsv");
  expect(rows.length >= 5000).toBeTrue();
  const failures = rows.filter((row) => {
    const [word = "", expected = ""] = row.split("\t");
    return stem(word) !== expected;
  });
  expect(failures).toEqual([]);
});

test("unpacking matches the Rust packing", () => {
  // Same numbers as the Rust test of `pack`.
  expect(unpack("01VW1eVWWWW1")).toEqual([0, 1, 31, 32, 1000, 1 << 20]);
});
