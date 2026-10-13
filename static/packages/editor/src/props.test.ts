import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import { type Doc, type Payload, fromPayload } from "./model";
import { cleanKey, pageProps, propsNode, setPageProps } from "./props";
import { serialize } from "./serialize";

// Payloads written by the Rust parser (tests/edit.rs) for tests/fixtures/editing/*.md.
function load(name: string): Doc {
  const url = new URL(`../test-fixtures/${name}.json`, import.meta.url);
  return fromPayload(JSON.parse(readFileSync(url, "utf8")) as Payload);
}

describe("page properties", () => {
  test("are read from a first block of only property lines", () => {
    expect(pageProps(load("page-props"))).toEqual([
      { key: "alias", value: "auth, [[login]]" },
      { key: "tags", value: "security" },
    ]);
    expect(propsNode(load("basic"))).toBeNull();
    expect(pageProps(load("prose"))).toEqual([]);
  });

  test("unchanged properties leave the file as it was", () => {
    const doc = load("page-props");
    expect(setPageProps(doc, pageProps(doc))).toBeFalse();
    expect(serialize(doc)).toBe("alias:: auth, [[login]]\ntags::  security\n\n- first\n  - child\n");
  });

  test("are rewritten in place, empty keys left out", () => {
    const doc = load("page-props");
    const changed = setPageProps(doc, [
      { key: "alias", value: "auth" },
      { key: "", value: "dropped" },
      { key: "status", value: "" },
    ]);
    expect(changed).toBeTrue();
    expect(serialize(doc)).toBe("alias:: auth\nstatus::\n\n- first\n  - child\n");
  });

  test("are added at the top of a page without them", () => {
    const doc = load("basic");
    const text = serialize(doc);
    setPageProps(doc, [{ key: "tags", value: "x" }]);
    expect(serialize(doc)).toBe(`tags:: x\n\n${text}`);
    expect(pageProps(doc)).toEqual([{ key: "tags", value: "x" }]);
  });

  test("are added before prose, kept apart by a blank line", () => {
    const doc = load("prose");
    setPageProps(doc, [{ key: "tags", value: "x" }]);
    expect(serialize(doc).startsWith("\n\ntags:: x\n\nIntro paragraph\n")).toBeTrue();
  });

  test("are added to an empty page", () => {
    const doc: Doc = { roots: [], indent: "  ", head: [], finalNewline: true };
    setPageProps(doc, [{ key: "alias", value: "a" }]);
    expect(serialize(doc)).toBe("alias:: a\n");
  });

  test("removing every property removes the block", () => {
    const doc = load("page-props");
    setPageProps(doc, [{ key: "", value: "" }]);
    expect(serialize(doc)).toBe("- first\n  - child\n");
  });

  test("keys keep only the characters a property key may have", () => {
    expect(cleanKey("my key:")).toBe("mykey");
    expect(cleanKey("due_date-2")).toBe("due_date-2");
  });

  test("values stay on one line", () => {
    const doc = load("page-props");
    setPageProps(doc, [{ key: "note", value: " two\nlines " }]);
    expect(pageProps(doc)).toEqual([{ key: "note", value: "two lines" }]);
  });
});
