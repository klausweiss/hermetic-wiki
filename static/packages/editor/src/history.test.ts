import { describe, expect, test } from "bun:test";

import { History, TYPING_PAUSE_MS, type Typing } from "./history";

/** An entry for `state`, with the cursor at `caret` in block 0. */
const at = (state: string, caret: number) => () => ({ state, where: { index: 0, start: caret, end: caret } });
const typed = (kind: Typing["kind"], caret: number): Typing => ({ kind, index: 0, caret });

describe("undo history", () => {
  test("A run of typing is one entry", () => {
    const h = new History<string>();
    h.record(at("", 0), typed("insert", 1), 0);
    h.record(at("a", 1), typed("insert", 2), 100);
    h.record(at("ab", 2), typed("insert", 3), 200);
    expect(h.undo(at("abc", 3))?.state).toBe("");
    expect(h.undo(at("", 0))).toBeNull();
    expect(h.redo(at("", 0))?.state).toBe("abc");
  });

  test("Moving the cursor, a pause, or deleting after typing starts a new entry", () => {
    const h = new History<string>();
    h.record(at("", 0), typed("insert", 1), 0);
    h.record(at("a", 0), typed("insert", 1), 100); // typed at the start: the cursor moved
    h.record(at("ba", 1), typed("insert", 2), 100 + TYPING_PAUSE_MS);
    h.record(at("bca", 2), typed("delete", 1), 200 + TYPING_PAUSE_MS);
    expect(h.undo(at("ba", 1))?.state).toBe("bca");
    expect(h.undo(at("bca", 2))?.state).toBe("ba");
    expect(h.undo(at("ba", 1))?.state).toBe("a");
    expect(h.undo(at("a", 0))?.state).toBe("");
  });

  test("Other changes are entries of their own, and end typing", () => {
    const h = new History<string>();
    h.record(at("", 0), typed("insert", 1), 0);
    h.record(at("a", 1), null, 10);
    h.record(at("a|", 1), typed("insert", 2), 20);
    expect(h.undo(at("a|b", 2))?.state).toBe("a|");
    expect(h.undo(at("a|", 1))?.state).toBe("a");
  });

  test("A new change after undoing drops the redo entries", () => {
    const h = new History<string>();
    h.record(at("", 0), null, 0);
    expect(h.undo(at("a", 1))?.state).toBe("");
    h.record(at("", 0), null, 10);
    expect(h.redo(at("b", 1))).toBeNull();
  });
});
