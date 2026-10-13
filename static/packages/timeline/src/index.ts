// <hermetic-timeline>: wraps the journal list (`ul.days` links, newest first) and adds a
// timeline view (the default) showing full journal days, loaded as the reader scrolls, under a
// calendar of the days with entries. Without the script the element is inert and the list
// inside it shows as usual.

import { fetchPage, readEnv, whenParsed } from "@hermetic/core";

import { calendar, scrollToNewest } from "./calendar";

const VIEW_PARAM = "view";
const TIMELINE = "timeline";
const LIST = "list";
const BATCH = 5;
/** Batches loaded to fill the screen before the reader has scrolled at all. */
const MAX_UNSCROLLED_BATCHES = 2;

export class HermeticTimeline extends HTMLElement {
  private ready = false;

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  private setup(): void {
    // Fetching day pages is not possible over file://; keep the plain list there.
    if (this.ready || !readEnv().interactive) {
      return;
    }
    this.ready = true;
    const days = [...this.querySelectorAll<HTMLAnchorElement>("ul.days a")];
    if (days.length === 0) {
      return;
    }

    const toggle = document.createElement("div");
    toggle.className = "view-toggle";
    toggle.setAttribute("role", "group");
    toggle.setAttribute("aria-label", "Journal view");
    const listButton = viewButton("List");
    const timelineButton = viewButton("Timeline");
    toggle.append(listButton, timelineButton);
    // Beside the page title, where it takes no height of its own.
    const header = this.parentElement?.querySelector(":scope > .page-header");
    if (header) {
      header.append(toggle);
    } else {
      this.prepend(toggle);
    }

    const timeline = document.createElement("div");
    timeline.className = "timeline";
    const cal = calendar(days.map((a) => ({ date: a.textContent ?? "", href: a.href })));
    timeline.append(cal);
    // The strip can only be scrolled once it is laid out, so the first time it is shown.
    let calendarShown = false;
    const sentinel = document.createElement("div");
    sentinel.className = "timeline-sentinel";
    timeline.append(sentinel);
    this.append(timeline);

    let next = 0;
    let loading = false;
    let batches = 0;
    let scrolled = false;
    addEventListener("scroll", () => (scrolled = true), { once: true, passive: true });

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          void loadBatch();
        }
      },
      { rootMargin: "600px 0px" },
    );

    const loadBatch = async (): Promise<void> => {
      if (loading || next >= days.length || (!scrolled && batches >= MAX_UNSCROLLED_BATCHES)) {
        return;
      }
      loading = true;
      batches += 1;
      const batch = days.slice(next, next + BATCH);
      next += batch.length;
      const pages = await Promise.all(batch.map((a) => fetchPage(new URL(a.href))));
      batch.forEach((a, i) => sentinel.before(daySection(a, pages[i]?.article ?? null)));
      loading = false;
      if (next >= days.length) {
        const end = document.createElement("p");
        end.className = "timeline-end";
        end.textContent = "This is the beginning of the journal.";
        sentinel.replaceWith(end);
        observer.disconnect();
        return;
      }
      // Re-check: the sentinel may still be visible after a short batch.
      observer.unobserve(sentinel);
      observer.observe(sentinel);
    };

    const show = (view: string): void => {
      const isTimeline = view === TIMELINE;
      this.classList.toggle("show-timeline", isTimeline);
      listButton.setAttribute("aria-pressed", String(!isTimeline));
      timelineButton.setAttribute("aria-pressed", String(isTimeline));
      // The timeline is the default; only the list is named in the URL.
      const url = new URL(location.href);
      if (isTimeline) {
        url.searchParams.delete(VIEW_PARAM);
        observer.observe(sentinel);
        if (!calendarShown) {
          calendarShown = true;
          scrollToNewest(cal);
        }
      } else {
        url.searchParams.set(VIEW_PARAM, LIST);
        observer.unobserve(sentinel);
      }
      history.replaceState(history.state, "", url.href);
    };

    listButton.addEventListener("click", () => show(LIST));
    timelineButton.addEventListener("click", () => show(TIMELINE));
    show(new URL(location.href).searchParams.get(VIEW_PARAM) === LIST ? LIST : TIMELINE);
  }
}

function viewButton(text: string): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.textContent = text;
  return b;
}

/** One day: a heading linking to the day page, then the day's own blocks. */
function daySection(link: HTMLAnchorElement, article: HTMLElement | null): HTMLElement {
  const section = document.createElement("section");
  section.className = "timeline-day";
  const heading = document.createElement("h2");
  const a = document.createElement("a");
  a.href = link.href;
  a.textContent = link.textContent ?? "";
  heading.append(a);
  const weekday = link.parentElement?.querySelector(".weekday")?.cloneNode(true);
  if (weekday) {
    heading.append(" ", weekday);
  }
  section.append(heading);
  const content = article?.querySelector("section.content");
  if (content) {
    section.append(content);
  } else {
    const missing = document.createElement("p");
    missing.className = "empty";
    missing.textContent = "This day could not be loaded.";
    section.append(missing);
  }
  return section;
}

export function define(name = "hermetic-timeline"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticTimeline);
  }
}
