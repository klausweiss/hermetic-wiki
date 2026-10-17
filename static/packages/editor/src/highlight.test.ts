import { describe, expect, test } from "bun:test";

import { highlight } from "./highlight";

/** The highlighted runs as `cls:text`, plain text as is. */
function runs(body: string): string[] {
  const segments = highlight(body);
  expect(segments.map((s) => s.text).join("")).toBe(body);
  return segments.map((s) => (s.cls ? `${s.cls}:${s.text}` : s.text));
}

describe("edit highlighting", () => {
  test("Page links, tags and block references", () => {
    expect(runs("see [[auth]] #backend ((ttl-bug)).")).toEqual([
      "see ",
      "hl-mark:[[",
      "hl-ref:auth",
      "hl-mark:]]",
      " ",
      "hl-ref:#backend",
      " ",
      "hl-mark:((",
      "hl-ref:ttl-bug",
      "hl-mark:))",
      ".",
    ]);
    expect(runs("kicked off #[[Project X]]")).toEqual(["kicked off ", "hl-mark:#[[", "hl-ref:Project X", "hl-mark:]]"]);
  });

  test("Tags and URLs leave trailing punctuation out", () => {
    expect(runs("fixed #backend, at https://x.dev/a.")).toEqual([
      "fixed ",
      "hl-ref:#backend",
      ", at ",
      "hl-link:https://x.dev/a",
      ".",
    ]);
  });

  test("A fragment or a word is not a tag", () => {
    expect(runs("a.html#x")).toEqual(["a.html#x"]);
  });

  test("Emphasis markers", () => {
    expect(runs("**bold** *em* ~~gone~~ snake_case_name")).toEqual([
      "hl-mark:**",
      "hl-strong:bold",
      "hl-mark:**",
      " ",
      "hl-mark:*",
      "hl-em:em",
      "hl-mark:*",
      " ",
      "hl-mark:~~",
      "hl-strike:gone",
      "hl-mark:~~",
      " snake_case_name",
    ]);
  });

  test("Bold and italic together, and nested", () => {
    expect(runs("***both***")).toEqual(["hl-mark:***", "hl-strong hl-em:both", "hl-mark:***"]);
    expect(runs("***a** b*")).toEqual(["hl-mark:*", "hl-mark:**", "hl-strong hl-em:a", "hl-mark:**", "hl-em: b", "hl-mark:*"]);
    expect(runs("***a* b**")).toEqual(["hl-mark:**", "hl-mark:*", "hl-em hl-strong:a", "hl-mark:*", "hl-strong: b", "hl-mark:**"]);
    expect(runs("**bold _em_ [[x]]**")).toEqual([
      "hl-mark:**",
      "hl-strong:bold ",
      "hl-mark:_",
      "hl-em hl-strong:em",
      "hl-mark:_",
      "hl-strong: ",
      "hl-mark:[[",
      "hl-ref hl-strong:x",
      "hl-mark:]]",
      "hl-mark:**",
    ]);
  });

  test("Nothing is recognized in code or after a backslash", () => {
    expect(runs("write `[[auth]]` or \\[[auth]]")).toEqual([
      "write ",
      "hl-code:`[[auth]]`",
      " or ",
      "hl-mark:\\[",
      "[auth]]",
    ]);
  });

  test("Tasks, properties and headings", () => {
    expect(runs("TODO ship it\nid:: abc\n## Notes")).toEqual([
      "hl-todo:TODO",
      " ship it",
      "\n",
      "hl-prop:id::",
      " abc",
      "\n",
      "hl-mark:##",
      " Notes",
    ]);
  });

  test("Fenced code", () => {
    expect(runs("```\n[[x]]\n```\n[[y]]")).toEqual([
      "hl-code:```",
      "\n",
      "hl-code:[[x]]",
      "\n",
      "hl-code:```",
      "\n",
      "hl-mark:[[",
      "hl-ref:y",
      "hl-mark:]]",
    ]);
  });
});
