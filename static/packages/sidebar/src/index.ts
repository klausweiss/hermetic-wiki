// <hermetic-sidebar>: the Logseq-style sidebar. Shift+click on a page link opens that
// page here, stacked newest first; the panes, and whether the sidebar is hidden, are
// kept in the URL.

import {
  type Env,
  icon,
  PAGES_CHANGED,
  type Pane,
  SIDEBAR_CLOSE_TOP,
  SIDEBAR_TOGGLE,
  TARGET_ATTR,
  carrySidebar,
  classify,
  fetchPage,
  readEnv,
  readHidden,
  readPanes,
  whenParsed,
  withSidebar,
} from "@hermetic/core";

/** Layout classes live on <html> so they can be set before the body exists. */
const LAYOUT = document.documentElement.classList;
const NARROW = "(max-width: 899.98px)";

/** Narrow screens hide the sidebar unless asked otherwise, since it would cover the page. */
function hiddenByDefault(): boolean {
  return matchMedia(NARROW).matches;
}

/** Whether the sidebar starts hidden: as the URL says, else the default for the width. */
function initiallyHidden(): boolean {
  return readHidden(new URLSearchParams(location.search)) ?? hiddenByDefault();
}

/**
 * Sets the sidebar's layout classes before the page is first painted, from what the
 * element will show once it is set up: hidden as the URL says or on narrow screens,
 * empty unless the URL lists panes (which only open over http). Without this the page
 * would first paint at full width and then shift to make room for the sidebar.
 */
export function applyInitialLayout(): void {
  const hidden = initiallyHidden();
  const interactive = location.protocol === "http:" || location.protocol === "https:";
  const empty = !interactive || readPanes(new URLSearchParams(location.search)).length === 0;
  LAYOUT.toggle("sidebar-open", !hidden);
  LAYOUT.toggle("sidebar-hidden", hidden);
  LAYOUT.toggle("sidebar-empty", empty);
}

interface PaneView {
  pane: Pane;
  fragment: string;
  element: HTMLElement;
  body: HTMLElement;
  title: HTMLAnchorElement;
  collapse: HTMLButtonElement;
  loaded: boolean;
}

export class HermeticSidebar extends HTMLElement {
  private env: Env | null = null;
  private list!: HTMLElement;
  private placeholder!: HTMLElement;
  private closeAllButton!: HTMLButtonElement;
  /** One control for hiding and showing, fixed in the same place in both states. */
  private toggle!: HTMLButtonElement;
  private observer!: IntersectionObserver;
  /** Top pane first. */
  private views: PaneView[] = [];
  private sidebarHidden = initiallyHidden();

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  get isEmpty(): boolean {
    return this.views.length === 0;
  }

  /** Current panes, top first. */
  get panes(): Pane[] {
    return this.views.map((v) => ({ ...v.pane }));
  }

  /** Opens `path` on top, or moves it to the top if it is already open. */
  open(path: string, fragment = ""): void {
    if (!this.env?.interactive) {
      return;
    }
    const existing = this.views.find((v) => v.pane.path === path);
    if (existing) {
      this.views = [existing, ...this.views.filter((v) => v !== existing)];
      existing.pane.collapsed = false;
      existing.fragment = fragment;
      this.list.prepend(existing.element);
      if (existing.loaded) {
        scrollToTarget(existing);
      }
    } else {
      this.add({ path, collapsed: false }, fragment);
    }
    this.setHidden(false);
    this.persist();
    this.list.scrollTop = 0;
  }

  closeTop(): void {
    const top = this.views[0];
    if (top) {
      this.close(top);
    }
  }

  toggleHidden(): void {
    this.setHidden(!this.sidebarHidden);
  }

  private setup(): void {
    if (this.env) {
      return;
    }
    const env = readEnv();
    this.env = env;

    const aside = document.createElement("aside");
    aside.className = "sidebar";
    aside.setAttribute("aria-label", "Sidebar");
    const bar = document.createElement("div");
    bar.className = "sidebar-bar";
    const label = document.createElement("span");
    label.textContent = "Sidebar";
    this.closeAllButton = button("Close all", "Close every pane", () => this.closeAll());
    bar.append(label, this.closeAllButton);

    this.placeholder = document.createElement("p");
    this.placeholder.className = "sidebar-placeholder";
    this.placeholder.textContent = env.interactive
      ? "Shift+click a link to open its page here, next to the one you are reading."
      : "Pages open here with Shift+click when the site is served over http:// " +
        "(run `hermetic serve` in the notes folder). Browsers do not allow it for " +
        "pages opened directly from disk.";

    this.list = document.createElement("div");
    this.list.className = "panes";
    aside.append(bar, this.placeholder, this.list);

    this.toggle = button("", "Show or hide the sidebar (\\)", () => this.toggleHidden());
    this.toggle.classList.add("corner-button", "sidebar-toggle");
    this.append(aside, this.toggle);

    // Panes are fetched only when they come near the visible part of the sidebar.
    this.observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const view = this.views.find((v) => v.element === entry.target);
          if (entry.isIntersecting && view && !view.pane.collapsed) {
            void this.load(view);
          }
        }
      },
      { root: this.list, rootMargin: "400px 0px" },
    );

    document.addEventListener(SIDEBAR_CLOSE_TOP, () => this.closeTop());
    document.addEventListener(SIDEBAR_TOGGLE, () => this.toggleHidden());
    document.addEventListener(PAGES_CHANGED, (e) => {
      const detail = (e as CustomEvent<{ changed: string[]; removed: string[] }>).detail;
      this.onPagesChanged(detail.changed, detail.removed);
    });

    if (env.interactive) {
      // Over file:// the browser does not let us fetch other pages, so links keep
      // their default behavior and the placeholder explains why.
      for (const pane of readPanes(new URLSearchParams(location.search)).reverse()) {
        this.add(pane, "");
      }
      this.persist();
      document.addEventListener("click", (e) => this.onClick(e));
    }
    this.render();
    // Width changes animate only from now on, i.e. in response to the reader, never
    // while the page loads.
    requestAnimationFrame(() => requestAnimationFrame(() => LAYOUT.add("sidebar-animate")));
  }

  private onClick(e: MouseEvent): void {
    if (!this.env || e.defaultPrevented || e.button !== 0 || e.ctrlKey || e.metaKey || e.altKey) {
      return;
    }
    const a = e.target instanceof Element ? e.target.closest<HTMLAnchorElement>("a[href]") : null;
    if (!a || (a.target && a.target !== "_self") || a.hasAttribute("download")) {
      return;
    }
    const link = classify(a.href, location.href, this.env.root.href);
    if (link.kind !== "page") {
      return;
    }
    if (e.shiftKey) {
      e.preventDefault();
      this.open(link.path, link.fragment);
    } else {
      // A normal navigation, carrying the sidebar's state to the next page.
      const href = carrySidebar(a.href, location.href);
      if (href !== a.href) {
        e.preventDefault();
        location.href = href;
      }
    }
  }

  private add(pane: Pane, fragment: string): void {
    const element = document.createElement("section");
    element.className = "pane";

    const header = document.createElement("header");
    header.className = "pane-header";
    const collapse = button("", "Collapse or expand", () => this.toggleCollapsed(view));
    collapse.classList.add("pane-collapse");
    const title = document.createElement("a");
    title.className = "pane-title";
    title.href = new URL(pane.path, this.env?.root).href;
    title.textContent = decodeURIComponent(pane.path.replace(/\.html$/, ""));
    const close = button("×", "Close", () => this.close(view));
    close.classList.add("pane-close");
    header.append(collapse, title, close);

    const body = document.createElement("div");
    body.className = "pane-body";
    body.textContent = "Loading…";
    element.append(header, body);

    const view: PaneView = { pane, fragment, element, body, title, collapse, loaded: false };
    this.views.unshift(view);
    this.list.prepend(element);
    this.observer.observe(element);
    this.render();
  }

  /** Live reload: refresh panes whose pages changed, close panes whose pages are gone. */
  private onPagesChanged(changed: string[], removed: string[]): void {
    for (const view of [...this.views]) {
      const path = decodePath(view.pane.path);
      if (removed.includes(path)) {
        this.close(view);
      } else if (changed.includes(path)) {
        view.loaded = false;
        if (!view.pane.collapsed) {
          void this.load(view, false);
        }
      }
    }
  }

  private async load(view: PaneView, scroll = true): Promise<void> {
    if (view.loaded || !this.env) {
      return;
    }
    view.loaded = true;
    const url = new URL(view.pane.path, this.env.root);
    if (view.fragment) {
      url.hash = view.fragment;
    }
    const page = await fetchPage(url);
    if (!page) {
      // The page no longer exists: drop the pane silently.
      this.close(view);
      return;
    }
    view.title.textContent = page.title;
    view.body.replaceChildren(page.article);
    if (scroll) {
      scrollToTarget(view);
    }
  }

  private toggleCollapsed(view: PaneView): void {
    view.pane.collapsed = !view.pane.collapsed;
    if (!view.pane.collapsed) {
      void this.load(view);
    }
    this.render();
    this.persist();
  }

  private close(view: PaneView): void {
    this.observer.unobserve(view.element);
    view.element.remove();
    this.views = this.views.filter((v) => v !== view);
    this.render();
    this.persist();
  }

  private closeAll(): void {
    for (const view of [...this.views]) {
      this.close(view);
    }
  }

  private setHidden(hidden: boolean): void {
    this.sidebarHidden = hidden;
    this.render();
    this.persist();
  }

  private render(): void {
    if (!this.env) {
      return;
    }
    LAYOUT.toggle("sidebar-open", !this.sidebarHidden);
    LAYOUT.toggle("sidebar-hidden", this.sidebarHidden);
    // While empty the sidebar is a narrow strip; it widens when a pane opens.
    LAYOUT.toggle("sidebar-empty", this.isEmpty);
    this.placeholder.hidden = !this.isEmpty;
    this.closeAllButton.hidden = this.isEmpty;
    // An icon: show (panel opening) or hide (panel closing), with the number of open
    // panes while they are hidden.
    const label = !this.sidebarHidden
      ? "Hide the sidebar (\\)"
      : this.isEmpty
        ? "Show the sidebar (\\)"
        : `Show the sidebar, ${this.views.length} open (\\)`;
    this.toggle.replaceChildren(icon(this.sidebarHidden ? "panel-right-open" : "panel-right-close"));
    if (this.sidebarHidden && !this.isEmpty) {
      const count = document.createElement("span");
      count.className = "corner-count";
      count.textContent = String(this.views.length);
      this.toggle.append(count);
    }
    this.toggle.setAttribute("aria-label", label);
    this.toggle.title = label;
    this.toggle.setAttribute("aria-expanded", String(!this.sidebarHidden));
    for (const view of this.views) {
      view.element.classList.toggle("collapsed", view.pane.collapsed);
      view.collapse.textContent = view.pane.collapsed ? "▸" : "▾";
      view.collapse.setAttribute("aria-expanded", String(!view.pane.collapsed));
    }
  }

  /**
   * Mirrors the panes, and whether the sidebar is hidden when that differs from the
   * default, into the URL without adding a history entry.
   */
  private persist(): void {
    if (!this.env?.interactive) {
      // Over file:// the state stays out of the URL, like the panes it would carry.
      return;
    }
    const hidden = this.sidebarHidden === hiddenByDefault() ? null : this.sidebarHidden;
    history.replaceState(history.state, "", withSidebar(location.href, this.panes, hidden));
  }
}

/** A pane path as the server names site paths: percent-decoded. */
function decodePath(path: string): string {
  try {
    return decodeURIComponent(path);
  } catch {
    return path;
  }
}

function scrollToTarget(view: PaneView): void {
  if (view.fragment) {
    view.body.querySelector(`[${TARGET_ATTR}]`)?.scrollIntoView({ block: "start" });
  }
}

function button(text: string, title: string, onClick: () => void): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.textContent = text;
  b.title = title;
  b.addEventListener("click", onClick);
  return b;
}

export function define(name = "hermetic-sidebar"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticSidebar);
  }
}
