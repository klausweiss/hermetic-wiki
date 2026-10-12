import { expect, test } from "bun:test";

import { readPanes, withPanes, writePanes } from "./state";

test("round trip keeps order and collapsed state", () => {
  const panes = [
    { path: "pages/redis.html", collapsed: false },
    { path: "pages/Project%20X/Auth%20Flow.html", collapsed: true },
  ];
  const params = writePanes(new URLSearchParams(), panes);
  expect(params.toString()).toBe("p=pages%2Fredis.html&p=%7Epages%2FProject%2520X%2FAuth%2520Flow.html");
  expect(readPanes(new URLSearchParams(params.toString()))).toEqual(panes);
});

test("other parameters are preserved", () => {
  const params = writePanes(new URLSearchParams("view=timeline&p=pages/old.html"), [
    { path: "pages/a.html", collapsed: false },
  ]);
  expect(params.get("view")).toBe("timeline");
  expect(params.getAll("p")).toEqual(["pages/a.html"]);
});

test("invalid and duplicate entries are dropped", () => {
  const params = new URLSearchParams();
  for (const v of ["pages/a.html", "", "~", "../secret.html", "https://x/a.html", "pages/a.html", "~pages/a.html", "pages/b.css", "~pages/c.html"]) {
    params.append("p", v);
  }
  expect(readPanes(params)).toEqual([
    { path: "pages/a.html", collapsed: false },
    { path: "pages/c.html", collapsed: true },
  ]);
});

test("withPanes keeps the fragment", () => {
  const url = withPanes("http://h/wiki/pages/a.html?view=x#b3", [{ path: "index.html", collapsed: false }]);
  expect(url).toBe("http://h/wiki/pages/a.html?view=x&p=index.html#b3");
  expect(withPanes("http://h/wiki/pages/a.html?p=index.html", [])).toBe("http://h/wiki/pages/a.html");
});
