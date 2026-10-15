import { expect, test } from "bun:test";

import { COUNT_PATH, mentionsCount, parseCount } from "./count";

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
