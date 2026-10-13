// The calendar strip at the top of the timeline: one small month grid per month with
// entries, oldest on the left, linking each journal day to its page. Months without entries
// are collapsed into a narrow gap marker, so years of journaling stay one row high; the
// strip scrolls sideways and starts at the newest month.

import { type Day, type Month, daysInMonth, firstWeekday, gapLabel, layout } from "./months";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const WEEKDAYS = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

export function calendar(days: Day[], today = new Date()): HTMLElement {
  const nav = document.createElement("nav");
  nav.className = "calendar";
  nav.setAttribute("aria-label", "Journal calendar");
  const todayKey = isoDate(today.getFullYear(), today.getMonth() + 1, today.getDate());
  for (const item of layout(days)) {
    if (item.kind === "gap") {
      const gap = document.createElement("div");
      gap.className = "calendar-gap";
      gap.title = `No entries for ${gapLabel(item.months)}`;
      gap.textContent = gapLabel(item.months, true);
      nav.append(gap);
    } else {
      nav.append(month(item, todayKey));
    }
  }
  return nav;
}

function month(m: Month, todayKey: string): HTMLElement {
  const box = document.createElement("div");
  box.className = "calendar-month";
  box.setAttribute("role", "group");
  const name = `${MONTHS[m.month - 1]} ${m.year}`;
  box.setAttribute("aria-label", name);
  const title = document.createElement("div");
  title.className = "calendar-title";
  title.textContent = name;
  const grid = document.createElement("div");
  grid.className = "calendar-grid";
  const first = firstWeekday(m.year, m.month);
  for (let i = 0; i < first; i++) {
    grid.append(document.createElement("span"));
  }
  for (let d = 1; d <= daysInMonth(m.year, m.month); d++) {
    const date = isoDate(m.year, m.month, d);
    const href = m.days.get(d);
    const cell = document.createElement(href ? "a" : "span");
    if (cell instanceof HTMLAnchorElement && href) {
      cell.href = href;
      cell.className = "has-entry";
    }
    if (date === todayKey) {
      cell.classList.add("today");
    }
    cell.title = `${date} ${WEEKDAYS[(first + d - 1) % 7]}`;
    cell.textContent = String(d);
    grid.append(cell);
  }
  box.append(title, grid);
  return box;
}

function isoDate(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

/** Scrolls the strip to its newest (rightmost) month. */
export function scrollToNewest(nav: HTMLElement): void {
  nav.scrollLeft = nav.scrollWidth;
}
