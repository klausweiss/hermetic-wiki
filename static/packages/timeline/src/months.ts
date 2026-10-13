// The calendar's layout: journal days grouped into months, oldest first, with the runs of
// months without entries collapsed into gaps. No DOM, so it can be tested on its own.

/** A journal day and the link to its page. */
export interface Day {
  /** `YYYY-MM-DD`. */
  date: string;
  href: string;
}

export interface Month {
  kind: "month";
  year: number;
  /** 1–12. */
  month: number;
  /** Links by day of the month. */
  days: Map<number, string>;
}

/** Months without entries between two months with entries. */
export interface Gap {
  kind: "gap";
  months: number;
}

export type Item = Month | Gap;

const DATE = /^(\d{4})-(\d{2})-(\d{2})$/;

/** Months with entries, oldest first, separated by gaps where months were skipped. */
export function layout(days: Day[]): Item[] {
  const byMonth = new Map<number, Month>();
  for (const { date, href } of days) {
    const m = DATE.exec(date.trim());
    if (!m) {
      continue;
    }
    const [year, month, day] = [Number(m[1]), Number(m[2]), Number(m[3])];
    const key = year * 12 + (month - 1);
    let entry = byMonth.get(key);
    if (!entry) {
      entry = { kind: "month", year, month, days: new Map() };
      byMonth.set(key, entry);
    }
    entry.days.set(day, href);
  }
  const items: Item[] = [];
  let prev: number | null = null;
  for (const key of [...byMonth.keys()].sort((a, b) => a - b)) {
    if (prev !== null && key - prev > 1) {
      items.push({ kind: "gap", months: key - prev - 1 });
    }
    items.push(byMonth.get(key)!);
    prev = key;
  }
  return items;
}

/** Days in a month (`month` 1–12). */
export function daysInMonth(year: number, month: number): number {
  return new Date(Date.UTC(year, month, 0)).getUTCDate();
}

/** Column of the month's first day in a week starting on Monday: 0 (Monday) to 6 (Sunday). */
export function firstWeekday(year: number, month: number): number {
  return (new Date(Date.UTC(year, month - 1, 1)).getUTCDay() + 6) % 7;
}

/** How long a gap lasted: "3 months", "1 year 2 months", or with `short`, "3 mo", "1 yr 2 mo". */
export function gapLabel(months: number, short = false): string {
  const years = Math.floor(months / 12);
  const rest = months % 12;
  const part = (n: number, unit: string, abbr: string): string =>
    short ? `${n} ${abbr}` : `${n} ${unit}${n === 1 ? "" : "s"}`;
  return [years ? part(years, "year", "yr") : "", rest ? part(rest, "month", "mo") : ""].filter(Boolean).join(" ");
}
