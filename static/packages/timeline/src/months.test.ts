import { expect, test } from "bun:test";

import { daysInMonth, firstWeekday, gapLabel, layout } from "./months";

const day = (date: string) => ({ date, href: `journals/${date}.html` });

test("days are grouped by month, oldest first", () => {
  const items = layout([day("2026-10-12"), day("2026-10-01"), day("2026-09-30")]);
  expect(items.map((i) => (i.kind === "month" ? `${i.year}-${i.month}` : `gap ${i.months}`))).toEqual([
    "2026-9",
    "2026-10",
  ]);
  const october = items[1];
  expect(october?.kind === "month" && [...october.days.keys()].sort()).toEqual([1, 12]);
  expect(october?.kind === "month" && october.days.get(12)).toBe("journals/2026-10-12.html");
});

test("skipped months become one gap, across years", () => {
  const items = layout([day("2026-02-01"), day("2024-11-05"), day("2024-12-31")]);
  expect(items.map((i) => (i.kind === "month" ? `${i.year}-${i.month}` : `gap ${i.months}`))).toEqual([
    "2024-11",
    "2024-12",
    "gap 13",
    "2026-2",
  ]);
});

test("text that is not a date is ignored", () => {
  expect(layout([day("someday"), day("2026-1-1")])).toEqual([]);
});

test("month shape", () => {
  expect(daysInMonth(2024, 2)).toBe(29);
  expect(daysInMonth(2026, 2)).toBe(28);
  expect(daysInMonth(2026, 12)).toBe(31);
  // 2026-10-01 is a Thursday; 2026-06-01 a Monday; 2026-11-01 a Sunday.
  expect(firstWeekday(2026, 10)).toBe(3);
  expect(firstWeekday(2026, 6)).toBe(0);
  expect(firstWeekday(2026, 11)).toBe(6);
});

test("gap labels", () => {
  expect(gapLabel(1)).toBe("1 month");
  expect(gapLabel(5)).toBe("5 months");
  expect(gapLabel(12)).toBe("1 year");
  expect(gapLabel(14)).toBe("1 year 2 months");
  expect(gapLabel(25)).toBe("2 years 1 month");
  expect(gapLabel(3, true)).toBe("3 mo");
  expect(gapLabel(14, true)).toBe("1 yr 2 mo");
});
