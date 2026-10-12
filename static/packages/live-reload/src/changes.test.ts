import { expect, test } from "bun:test";

import { affects, parseChanges, sitePath } from "./changes";

test("site paths are decoded and relative to the root", () => {
  expect(sitePath("/pages/Project%20X/Auth%20Flow.html", "/")).toBe("pages/Project X/Auth Flow.html");
  expect(sitePath("/wiki/pages/Zu%CC%88rich.html", "/wiki/")).toBe("pages/Zürich.html");
  expect(sitePath("/", "/")).toBe("index.html");
  expect(sitePath("/wiki/", "/wiki/")).toBe("index.html");
  expect(sitePath("/other/a.html", "/wiki/")).toBeNull();
  expect(sitePath("/%E0%A4%A.html", "/")).toBeNull();
});

test("change payloads", () => {
  expect(parseChanges('{"changed":["a.html"],"removed":[]}')).toEqual({ changed: ["a.html"], removed: [] });
  for (const bad of ["", "null", "[]", '{"changed":[1],"removed":[]}', '{"changed":[]}', "nope"]) {
    expect(parseChanges(bad)).toBeNull();
  }
});

test("affects", () => {
  const c = { changed: ["pages/a.html"], removed: ["pages/b.html"] };
  expect(affects(c, "pages/a.html")).toBeTrue();
  expect(affects(c, "pages/b.html")).toBeTrue();
  expect(affects(c, "pages/c.html")).toBeFalse();
});
