import { expect, test } from "bun:test";

import { carrySidebar, readHidden, readPanes, withPanes, writeHidden, writePanes } from "./state";

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

test("sidebar visibility round trip", () => {
  expect(readHidden(writeHidden(new URLSearchParams(), true))).toBe(true);
  expect(readHidden(writeHidden(new URLSearchParams(), false))).toBe(false);
  expect(writeHidden(new URLSearchParams("sidebar=hidden&p=a.html"), null).toString()).toBe("p=a.html");
  expect(readHidden(new URLSearchParams("sidebar=bogus"))).toBeNull();
});

test("carrySidebar keeps panes and visibility, or leaves the target alone", () => {
  const from = "http://h/wiki/index.html?p=pages%2Fa.html&sidebar=hidden";
  expect(carrySidebar("http://h/wiki/pages/b.html#x", from)).toBe(
    "http://h/wiki/pages/b.html?p=pages%2Fa.html&sidebar=hidden#x",
  );
  expect(carrySidebar("http://h/wiki/pages/b.html?q=1", "http://h/wiki/index.html")).toBe(
    "http://h/wiki/pages/b.html?q=1",
  );
});
