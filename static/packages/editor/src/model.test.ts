import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";

import { type Doc, type Node, type Payload, flatten, fromPayload } from "./model";
import { backspace, cycleTask, indent, insideOpenFence, move, outdent, split } from "./ops";
import { serialize } from "./serialize";

// Payloads written by the Rust parser (tests/edit.rs) for tests/fixtures/editing/*.md.
const FIXTURES = new URL("../test-fixtures/", import.meta.url);

function fixture(name: string): Payload {
  return JSON.parse(readFileSync(new URL(`${name}.json`, FIXTURES), "utf8")) as Payload;
}

/** A payload for `text`, as the server parses it; only for fixtures above. */
function load(name: string): Doc {
  return fromPayload(fixture(name));
}

function find(doc: Doc, body: string): Node {
  const hit = flatten(doc).find((e) => e.node.body === body);
  if (!hit) throw new Error(`no block ${JSON.stringify(body)}`);
  return hit.node;
}

describe("serialize", () => {
  test("every fixture round-trips byte for byte", () => {
    const names = readdirSync(FIXTURES).filter((n) => n.endsWith(".json"));
    expect(names.length >= 9).toBeTrue();
    for (const name of names) {
      const p = fixture(name.replace(/\.json$/, ""));
      expect(serialize(fromPayload(p))).toBe(p.text);
    }
  });

  test("an edit changes only that block's lines", () => {
    const doc = load("irregular");
    find(doc, "b").body = "b changed";
    expect(serialize(doc)).toBe("- a\n      - b changed\n  - c\n   - d\n");
  });

  test("a moved block is indented so that it parses where it was put", () => {
    // `d` becomes a sibling of `c`, so it may not be indented further than `c`.
    const doc = load("irregular");
    outdent(doc, find(doc, "d"), 0);
    expect(serialize(doc)).toBe("- a\n      - b\n  - c\n  - d\n");
    // A block with no previous sibling goes one indentation unit below its parent.
    const tabs = load("tabs");
    indent(tabs, find(tabs, "d"), 0);
    expect(serialize(tabs)).toBe("- a\n\t- b\n\t\t- c\n\t\t- d\n- e\n");
    const fresh = load("four-spaces");
    split(fresh, find(fresh, "c"), 1);
    indent(fresh, flatten(fresh).at(-2)!.node, 0);
    expect(serialize(fresh)).toBe("- a\n    - b\n        - c\n            -\n    - d\n");
  });

  test("blank lines and prose around an edited block stay", () => {
    const doc = load("prose");
    find(doc, "a").body = "A";
    expect(serialize(doc)).toBe("\n\nIntro paragraph\nstill intro\n\n- A\n- b\n\nClosing prose\n\nSecond prose\n");
  });

  test("multi-line bodies are indented to the content column", () => {
    const doc = load("tabs");
    find(doc, "b").body = "b\nsecond line\n\nafter blank";
    expect(serialize(doc)).toBe("- a\n\t- b\n\t  second line\n\n\t  after blank\n\t\t- c\n\t- d\n- e\n");
  });

  test("an empty body is written as a bare bullet", () => {
    const doc = load("basic");
    find(doc, "third").body = "";
    expect(serialize(doc).endsWith("\n-\n")).toBeTrue();
  });

  test("no final newline is kept", () => {
    const doc = load("no-final-newline");
    find(doc, "child").body = "kid";
    expect(serialize(doc)).toBe("- no final newline\n  - kid");
  });

  test("an empty page gets its first block", () => {
    const doc = fromPayload({ exists: false, file: "pages/new.md", version: "missing", text: "", indent: "  ", blocks: [] });
    expect(serialize(doc)).toBe("");
    doc.roots.push({ body: "first note", bullet: true, children: [] });
    expect(serialize(doc)).toBe("- first note\n");
  });
});

describe("outliner operations", () => {
  test("Enter at the end makes a sibling (New sibling)", () => {
    const doc = load("tabs");
    const e = find(doc, "e");
    const f = split(doc, e, 1)!;
    f.node.body = "f";
    expect(serialize(doc)).toBe("- a\n\t- b\n\t\t- c\n\t- d\n- e\n- f\n");
  });

  test("Enter in the middle splits (Split in the middle)", () => {
    const doc = fromPayload(fixture("basic"));
    const third = find(doc, "third");
    third.body = "first second";
    const f = split(doc, third, "first ".length)!;
    expect(f.offset).toBe(0);
    expect(f.node.body).toBe("second");
    expect(serialize(doc).endsWith("- first \n- second\n")).toBeTrue();
  });

  test("Enter on a block with children makes the first child", () => {
    const doc = load("tabs");
    const a = find(doc, "a");
    split(doc, a, 1)!.node.body = "new";
    expect(serialize(doc)).toBe("- a\n\t- new\n\t- b\n\t\t- c\n\t- d\n- e\n");
  });

  test("properties stay with the block they belong to", () => {
    const doc = load("basic");
    const rotate = find(doc, "TODO rotate signing keys\nid:: rotate-keys");
    const f = split(doc, rotate, "TODO rotate".length)!;
    expect(rotate.body).toBe("TODO rotate\nid:: rotate-keys");
    expect(f.node.body).toBe(" signing keys");
  });

  test("Enter inside an open fence does not split", () => {
    expect(insideOpenFence("code:\n```sh\nls", 12)).toBeTrue();
    expect(insideOpenFence("```sh", 5)).toBeTrue();
    expect(insideOpenFence("```sh\nls\n```", 12)).toBeFalse();
    expect(insideOpenFence("```sh\nls\n~~~", 12)).toBeTrue();
    expect(insideOpenFence("plain", 5)).toBeFalse();
    const doc = load("fences");
    const after = find(doc, "after\n~~~\nopen fence until the end");
    expect(split(doc, after, after.body.length)).toBeNull();
  });

  test("Tab and Shift+Tab (Indent and outdent)", () => {
    const doc = fromPayload({
      exists: true, file: "pages/t.md", version: "v", text: "- a\n- b\n- c\n", indent: "  ",
      blocks: [
        { lines: [0, 1], first: 0, depth: 0, bullet: true, anchor: "b1", body: "a", column: 2 },
        { lines: [1, 2], first: 1, depth: 0, bullet: true, anchor: "b2", body: "b", column: 2 },
        { lines: [2, 4], first: 2, depth: 0, bullet: true, anchor: "b3", body: "c", column: 2 },
      ],
    });
    const b = find(doc, "b");
    expect(indent(doc, b, 0)).not.toBeNull();
    expect(serialize(doc)).toBe("- a\n  - b\n- c\n");
    expect(outdent(doc, b, 0)).not.toBeNull();
    expect(serialize(doc)).toBe("- a\n- b\n- c\n");
    expect(indent(doc, find(doc, "a"), 0)).toBeNull();
    expect(outdent(doc, find(doc, "a"), 0)).toBeNull();
  });

  test("Shift+Tab keeps later siblings (Outdent keeps later siblings)", () => {
    const doc = load("four-spaces");
    outdent(doc, find(doc, "b"), 0);
    expect(serialize(doc)).toBe("- a\n    - d\n- b\n    - c\n");
  });

  test("Tab in a tab-indented file uses tabs (Tab-indented file)", () => {
    const doc = load("tabs");
    indent(doc, find(doc, "e"), 0);
    expect(serialize(doc)).toBe("- a\n\t- b\n\t\t- c\n\t- d\n\t- e\n");
  });

  test("prose never moves into the tree", () => {
    const doc = load("prose");
    expect(indent(doc, find(doc, "Second prose"), 0)).toBeNull();
    const intro = find(doc, "Intro paragraph\nstill intro");
    expect(indent(doc, find(doc, "a"), 0)).toBeNull();
    split(doc, intro, intro.body.length)!.node.body = "new";
    expect(serialize(doc)).toBe("\n\nIntro paragraph\nstill intro\n\n- new\n- a\n- b\n\nClosing prose\n\nSecond prose\n");
  });

  test("a block at a new depth never keeps its old indentation", () => {
    const doc = load("tabs");
    outdent(doc, find(doc, "b"), 0);
    move(doc, find(doc, "b"), 0, -1);
    move(doc, find(doc, "b"), 0, -1);
    expect(serialize(doc)).toBe("- b\n\t- c\n- a\n\t- d\n- e\n");
  });

  test("Alt+Shift+Up/Down swap with siblings, children travel along", () => {
    const doc = load("tabs");
    expect(move(doc, find(doc, "a"), 0, -1)).toBeNull();
    move(doc, find(doc, "a"), 0, 1);
    expect(serialize(doc)).toBe("- e\n- a\n\t- b\n\t\t- c\n\t- d\n");
    move(doc, find(doc, "d"), 0, -1);
    expect(serialize(doc)).toBe("- e\n- a\n\t- d\n\t- b\n\t\t- c\n");
  });

  test("Backspace joins with the previous block (Join with previous)", () => {
    const doc = fromPayload({
      exists: true, file: "pages/t.md", version: "v", text: "- one\n- two\n", indent: "  ",
      blocks: [
        { lines: [0, 1], first: 0, depth: 0, bullet: true, anchor: "b1", body: "one", column: 2 },
        { lines: [1, 3], first: 1, depth: 0, bullet: true, anchor: "b2", body: "two", column: 2 },
      ],
    });
    const f = backspace(doc, find(doc, "two"))!;
    expect(f.node.body).toBe("onetwo");
    expect(f.offset).toBe(3);
    expect(serialize(doc)).toBe("- onetwo\n");
    expect(backspace(doc, find(doc, "onetwo"))).toBeNull();
  });

  test("Backspace deletes an empty block, but does not join into a parent", () => {
    const doc = load("empty-bullets");
    const nodes = flatten(doc).map((e) => e.node);
    // `-` (empty, no children) is deleted.
    const f = backspace(doc, nodes[1]!)!;
    expect(f.node.body).toBe("a");
    expect(serialize(doc)).toBe("- a\n- \n  - child of empty\n");
    // The child's previous block is its parent: no join.
    expect(backspace(doc, find(doc, "child of empty"))).toBeNull();
  });

  test("Ctrl+Enter cycles the task marker (Cycle task)", () => {
    const node: Node = { body: "write tests\nmore", bullet: true, children: [] };
    expect(cycleTask(node, 5).offset).toBe(10);
    expect(node.body).toBe("TODO write tests\nmore");
    cycleTask(node, 0);
    expect(node.body).toBe("DOING write tests\nmore");
    cycleTask(node, 0);
    expect(node.body).toBe("DONE write tests\nmore");
    const f = cycleTask(node, 7);
    expect(node.body).toBe("write tests\nmore");
    expect(f.offset).toBe(2);
    const bare: Node = { body: "TODO", bullet: true, children: [] };
    cycleTask(bare, 4);
    expect(bare.body).toBe("DOING ");
  });
});

/**
 * Depth of every bullet line, by the parser's nesting rule: a bullet is a child of the
 * nearest preceding bullet with a smaller indentation column; text at column 0 ends
 * every open bullet. Only for texts without code fences.
 */
function parsedDepths(text: string): number[] {
  const stack: number[] = [];
  const depths: number[] = [];
  let inBullet = false;
  for (const line of text.split("\n")) {
    if (line.trim() === "") continue;
    const ws = /^[ \t]*/.exec(line)![0];
    let col = 0;
    for (const c of ws) col = c === "\t" ? col + 4 - (col % 4) : col + 1;
    const rest = line.slice(ws.length);
    if (rest === "-" || rest.startsWith("- ")) {
      while (stack.length > 0 && stack[stack.length - 1]! >= col) stack.pop();
      depths.push(stack.length);
      stack.push(col);
      inBullet = true;
    } else if (col === 0 && inBullet) {
      stack.length = 0;
      inBullet = false;
    }
  }
  return depths;
}

describe("written text parses back into the same tree", () => {
  test("after random sequences of operations on every fixture", () => {
    let seed = 7;
    const random = (n: number) => {
      seed = (seed * 1103515245 + 12345) % 2147483648;
      return seed % n;
    };
    const names = readdirSync(FIXTURES)
      .filter((n) => n.endsWith(".json") && n !== "fences.json")
      .map((n) => n.replace(/\.json$/, ""));
    for (let round = 0; round < 400; round++) {
      const doc = load(names[random(names.length)]!);
      for (let step = 0; step < 12; step++) {
        const all = flatten(doc);
        if (all.length === 0) break;
        const node = all[random(all.length)]!.node;
        switch (random(7)) {
          case 0: split(doc, node, node.body.split("\n")[0]!.length); break;
          case 1: indent(doc, node, 0); break;
          case 2: outdent(doc, node, 0); break;
          case 3: move(doc, node, 0, random(2) === 0 ? -1 : 1); break;
          case 4: backspace(doc, node); break;
          case 5: cycleTask(node, 0); break;
          default: node.body = `${node.body.split("\n")[0]} x`;
        }
      }
      const expected = flatten(doc).filter((e) => e.node.bullet).map((e) => e.depth);
      expect(parsedDepths(serialize(doc))).toEqual(expected);
    }
  });
});

describe("task labels", () => {
  test("advanceTask cycles TODO → DOING → DONE → TODO and touches only the marker", async () => {
    const { advanceTask } = await import("./ops");
    expect(advanceTask("TODO rotate signing keys")).toBe("DOING rotate signing keys");
    expect(advanceTask("DOING rotate\nowner:: ops\nmore")).toBe("DONE rotate\nowner:: ops\nmore");
    expect(advanceTask("DONE x")).toBe("TODO x");
    expect(advanceTask("TODO")).toBe("DOING");
    expect(advanceTask("TODOS for today")).toBeNull();
    expect(advanceTask("plain")).toBeNull();
    expect(advanceTask("note\nTODO not the first line")).toBeNull();
  });
});
