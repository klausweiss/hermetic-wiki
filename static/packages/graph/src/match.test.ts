import { expect, test } from "bun:test";

import { foldText } from "@hermetic/search/normalize";

import { matches, queryWords } from "./match";

const names = ["Project X/Auth Flow", "auth", "redis", "Zürich", "Łódź notes"];
const find = (query: string) => names.filter((n) => matches(foldText(n), queryWords(query)));

test("a word matches anywhere in the name", () => {
  expect(find("auth")).toEqual(["Project X/Auth Flow", "auth"]);
  expect(find("dis")).toEqual(["redis"]);
});

test("every word must match, in any order and case", () => {
  expect(find("flow AUTH")).toEqual(["Project X/Auth Flow"]);
  expect(find("auth redis")).toEqual([]);
});

test("diacritics are ignored both ways", () => {
  expect(find("zurich")).toEqual(["Zürich"]);
  expect(find("ZÜRICH")).toEqual(["Zürich"]);
  expect(find("lodz")).toEqual(["Łódź notes"]);
});

test("an empty query matches nothing", () => {
  expect(find("")).toEqual([]);
  expect(find("   ")).toEqual([]);
});

test("slashes and other punctuation are part of names", () => {
  expect(find("x/auth")).toEqual(["Project X/Auth Flow"]);
});
