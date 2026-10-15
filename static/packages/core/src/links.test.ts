import { expect, test } from "bun:test";

import { absolutize, anchorHref, classify, isSafePagePath, rebase } from "./links";

const root = "http://localhost:8000/wiki/";

test("pages are .html files under the root", () => {
  expect(classify("auth.html", "http://localhost:8000/wiki/pages/x.html", root)).toEqual({
    kind: "page",
    path: "pages/auth.html",
    fragment: "",
  });
  expect(classify("../index.html", "http://localhost:8000/wiki/pages/x.html", root)).toEqual({
    kind: "page",
    path: "index.html",
    fragment: "",
  });
});

test("nested paths keep their encoding and fragment", () => {
  const base = "http://localhost:8000/wiki/pages/Project%20X/Auth%20Flow.html";
  expect(classify("../Project%20X.html#id-a%20b", base, root)).toEqual({
    kind: "page",
    path: "pages/Project%20X.html",
    fragment: "id-a%20b",
  });
  expect(classify("#b3", base, root)).toEqual({
    kind: "page",
    path: "pages/Project%20X/Auth%20Flow.html",
    fragment: "b3",
  });
});

test("hermetic assets, attachments, and other sites are not pages", () => {
  const base = "http://localhost:8000/wiki/pages/x.html";
  for (const href of [
    "../_hermetic/style.css",
    "../_hermetic/fonts/LICENSE-Inter.txt",
    "diagram.png",
    "../assets/logo.svg",
    "https://example.com/a.html",
    "http://localhost:9000/wiki/pages/a.html",
    "/other/a.html",
    "mailto:me@example.com",
    "../",
  ]) {
    expect(classify(href, base, root)).toEqual({ kind: "other" });
  }
});

test("absolutize resolves against the page URL", () => {
  const page = "http://h/wiki/pages/Project%20X/Auth%20Flow.html";
  expect(absolutize("../Project%20X.html", page)).toBe("http://h/wiki/pages/Project%20X.html");
  expect(absolutize("#b2", page)).toBe(page + "#b2");
  expect(absolutize("diagram.png", page)).toBe("http://h/wiki/pages/Project%20X/diagram.png");
  expect(absolutize("https://example.com/", page)).toBe("https://example.com/");
  expect(absolutize("mailto:me@example.com", page)).toBe("mailto:me@example.com");
});

test("safe page paths", () => {
  for (const ok of ["index.html", "pages/a.html", "pages/Project%20X/Auth%20Flow.html"]) {
    expect(isSafePagePath(ok)).toBeTrue();
  }
  for (const bad of [
    "",
    "/etc/passwd.html",
    "../x.html",
    "pages/../../x.html",
    "pages//a.html",
    "https://evil.example/a.html",
    "javascript:alert(1).html",
    "pages/a.css",
    "pages/a.html?x",
    "pages\\a.html",
  ]) {
    expect(isSafePagePath(bad)).toBeFalse();
  }
});

test("anchorHref resolves the attribute, for HTML and SVG links alike", () => {
  const anchor = (href: string | null, baseURI: string) => ({
    getAttribute: (name: string) => (name === "href" ? href : null),
    baseURI,
  });
  const base = "http://localhost:8000/wiki/graph.html";
  expect(anchorHref(anchor("pages/Project%20X.html", base))).toBe(
    "http://localhost:8000/wiki/pages/Project%20X.html",
  );
  expect(anchorHref(anchor("../index.html#b2", "http://localhost:8000/wiki/pages/a.html"))).toBe(
    "http://localhost:8000/wiki/index.html#b2",
  );
  expect(anchorHref(anchor("https://example.com/", base))).toBe("https://example.com/");
  expect(anchorHref(anchor(null, base))).toBe(base);
});

test("rebase makes relative links root-relative and keeps absolute ones", () => {
  const page = "http://h/wiki/journals/2026-10-14.html";
  expect(rebase("../pages/plugin%20system.html", page)).toBe("/wiki/pages/plugin%20system.html");
  expect(rebase("#b2", page)).toBe("/wiki/journals/2026-10-14.html#b2");
  expect(rebase("https://example.com/", page)).toBe("https://example.com/");
  expect(rebase("http://other:8000/x.html", page)).toBe("http://other:8000/x.html");
  expect(rebase("http://h/wiki/journals.html", page)).toBe("http://h/wiki/journals.html");
  expect(rebase("//other/x.html", page)).toBe("http://other/x.html");
  expect(rebase("mailto:me@example.com", page)).toBe("mailto:me@example.com");
});
