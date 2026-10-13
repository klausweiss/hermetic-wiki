import { expect, test } from "bun:test";

import { isEmpty, sameText } from "./serialize";

test("texts without content", () => {
  for (const text of ["", "\n", "-\n", "- \n", "  -  \r\n\n", "\n-"]) expect(isEmpty(text)).toBe(true);
  for (const text of ["- a\n", "-\n-\n", "a\n", "- \n  - \n", "title:: x\n"]) expect(isEmpty(text)).toBe(false);
  expect(sameText("", "-\n")).toBe(true);
  expect(sameText("- a\n", "- a")).toBe(false);
});
