import { expect, test } from "bun:test";

import { COUNT_PATH, mentionsCount, parseCount, parseStored, storageKey } from "./count";

test("valid counts", () => {
  expect(parseCount('{"count":3}')).toBe(3);
  expect(parseCount('{"count":0}')).toBe(0);
});

test("malformed or missing counts", () => {
  for (const bad of ["", "nope", "null", "3", "[]", "{}", '{"count":"3"}', '{"count":-1}', '{"count":1.5}']) {
    expect(parseCount(bad)).toBeNull();
  }
});

test("change lists", () => {
  expect(mentionsCount({ changed: [COUNT_PATH, "tasks.html"], removed: [] })).toBeTrue();
  expect(mentionsCount({ changed: [], removed: [COUNT_PATH] })).toBeTrue();
  expect(mentionsCount({ changed: ["pages/a.html", "tasks.html"], removed: [] })).toBeFalse();
});

test("remembered counts", () => {
  expect(parseStored("12")).toBe(12);
  expect(parseStored("0")).toBe(0);
  for (const bad of [null, "", "-1", "1.5", "12a", " 3", "9999999999999999"]) {
    expect(parseStored(bad)).toBeNull();
  }
});

test("storage keys differ per site", () => {
  expect(storageKey(new URL("https://example.com/a/"))).not.toBe(storageKey(new URL("https://example.com/b/")));
});
