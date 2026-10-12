import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

import { type Payload, flatten, fromPayload } from "./model";
import { addId, blockId, ensureId, newId } from "./refs";
import { serialize } from "./serialize";

const fixture = (name: string) =>
  fromPayload(JSON.parse(readFileSync(new URL(`../test-fixtures/${name}.json`, import.meta.url), "utf8")) as Payload);

describe("block ids", () => {
  test("generated ids are 8 characters from a-z and 0-9", () => {
    for (let i = 0; i < 50; i++) expect(/^[a-z0-9]{8}$/.test(newId())).toBeTrue();
    expect(newId(() => new Uint8Array([0, 1, 25, 26, 35, 36, 255, 7]))).toBe("abz09adh");
  });

  test("Block without id: the id line is added and nothing else changes", () => {
    const doc = fixture("basic");
    const node = flatten(doc).find((e) => e.node.body === "decided to use [[JWT]]\nstatus:: accepted")!.node;
    addId(node, "k3x9q2ab");
    expect(serialize(doc)).toBe(
      "- decided to use [[JWT]]\n  status:: accepted\n  id:: k3x9q2ab\n  - TODO rotate signing keys\n    id:: rotate-keys\n  - DONE write the ADR\n- second topic\n  continues here\n\n  after a blank line\n- third\n",
    );
    expect(blockId(node)).toBe("k3x9q2ab");
  });

  test("the id line goes right after the first line, at the content column", () => {
    const doc = fixture("tabs");
    const node = flatten(doc).find((e) => e.node.body === "c")!.node;
    addId(node, "abcdefgh");
    expect(serialize(doc)).toBe("- a\n\t- b\n\t\t- c\n\t\t  id:: abcdefgh\n\t- d\n- e\n");
    const multi = fixture("basic");
    const second = flatten(multi).find((e) => e.node.body.startsWith("second topic"))!.node;
    addId(second, "zz");
    expect(serialize(multi).includes("- second topic\n  id:: zz\n  continues here\n")).toBeTrue();
  });

  test("Block with id: nothing changes", () => {
    const doc = fixture("basic");
    const node = flatten(doc).find((e) => e.node.body.startsWith("TODO rotate"))!.node;
    expect(ensureId(node)).toEqual({ id: "rotate-keys", added: false });
    expect(serialize(doc)).toBe(serialize(fixture("basic")));
  });
});
