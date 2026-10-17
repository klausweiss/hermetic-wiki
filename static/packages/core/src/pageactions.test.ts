import { describe, expect, test } from "bun:test";

import { addPageAction, pageActions } from "./pageactions";

describe("page actions", () => {
  test("added in order, and withdrawn", () => {
    const run = () => {};
    const removeA = addPageAction({ key: "a", what: "A", run });
    addPageAction({ key: "b", what: "B", run });
    expect(pageActions().map((a) => a.key)).toEqual(["a", "b"]);
    removeA();
    removeA();
    expect(pageActions().map((a) => a.key)).toEqual(["b"]);
  });
});
