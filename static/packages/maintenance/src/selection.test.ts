import { describe, expect, test } from "bun:test";

import { Selection } from "./selection";

const paths = (n: number) => Array.from({ length: n }, (_, i) => `assets/${i + 1}.png`);

describe("Selection", () => {
  test("select all, and the mixed state", () => {
    const s = new Selection(paths(12));
    expect(s.all).toBe("none");
    s.setAll(true);
    expect(s.size).toBe(12);
    expect(s.all).toBe("all");
    s.set(3, false);
    expect(s.all).toBe("some");
    s.setAll(false);
    expect(s.all).toBe("none");
  });

  test("Shift+click selects the range from the entry clicked last", () => {
    const s = new Selection(paths(8));
    s.set(1, true);
    s.set(5, true, true);
    expect(s.chosen).toEqual(["assets/2.png", "assets/3.png", "assets/4.png", "assets/5.png", "assets/6.png"]);
    // Backwards, clearing.
    s.set(3, false, true);
    expect(s.chosen).toEqual(["assets/2.png", "assets/3.png"]);
  });

  test("a range without an earlier click is one entry", () => {
    const s = new Selection(paths(4));
    s.set(2, true, true);
    expect(s.chosen).toEqual(["assets/3.png"]);
  });

  test("a new list keeps the selection of entries still listed", () => {
    const s = new Selection(["a", "b", "c"]);
    s.set(0, true);
    s.set(2, true);
    s.replace(["b", "c", "d"]);
    expect(s.chosen).toEqual(["c"]);
    expect(s.all).toBe("some");
    // The range starts at `c`, now at index 1.
    s.set(2, true, true);
    expect(s.chosen).toEqual(["c", "d"]);
  });
});
