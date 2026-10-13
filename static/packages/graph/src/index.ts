// <hermetic-graph>: wraps the graph view's SVG (written by hermetic, laid out at build
// time) and adds pan and zoom (wheel, drag, pinch, buttons), labels that appear as the
// reader zooms in, and highlighting of a node's neighbours. Without the script the SVG
// shows the whole graph fitted to the page, and its nodes are plain links.
//
// It also adds a search over node names, opened with Ctrl+F (Cmd+F) on the graph page:
// the arrow keys move between the matches and Enter opens one.
// Panning, zooming, and searching need no other page, so they also work over file://.

import { type IconName, anchorHref, carrySidebar, icon, whenParsed } from "@hermetic/core";
import { foldText } from "@hermetic/search/normalize";

import { matches, queryWords } from "./match";
import { FIT, SPACING, type View, centreOn, labelLevel, maxZoom, pan, transform, zoomAt } from "./view";

const SVG_NS = "http://www.w3.org/2000/svg";
/** Zoom factor of the zoom buttons. */
const STEP = 1.5;
/** Pointer movement, in screen pixels, after which a press becomes a drag. */
const DRAG_PX = 4;
/** How long after the last wheel event the graph counts as still moving. */
const WHEEL_IDLE_MS = 150;
/** Screen distance between connected nodes at which every label shows (`labelLevel` 3). */
const ALL_LABELS_PX = 70;
const APPLE = /Mac|iPhone|iPad/.test(navigator.platform);

interface GraphNode {
  a: SVGAElement;
  /** Centre, in graph units. */
  x: number;
  y: number;
  /** Indices of the connected nodes. */
  near: number[];
  /** The page name, folded for search. */
  folded: string;
}

interface Point {
  x: number;
  y: number;
}

export class HermeticGraph extends HTMLElement {
  private ready = false;
  private svg!: SVGSVGElement;
  private world!: SVGGElement;
  private highlight!: SVGPathElement;
  private nodes: GraphNode[] = [];
  private view: View = FIT;
  /** Screen pixels per graph unit at the fitted view. */
  private scale = 1;
  /** Pointers currently pressed, at their last position (client coordinates). */
  private pointers = new Map<number, Point>();
  /** Where the current single-pointer press started. */
  private pressedAt: Point | null = null;
  private dragging = false;
  private suppressClick = false;
  private focused: number | null = null;
  private frame = 0;
  private wheelTimer = 0;
  private searchBox!: HTMLElement;
  private searchInput!: HTMLInputElement;
  private searchCount!: HTMLOutputElement;
  private searchButton!: HTMLButtonElement;
  /** Indices of the nodes matching the query, in document order. */
  private found: number[] = [];
  /** Position in `found` of the current match, or -1. */
  private current = -1;
  /** The view when the search box was opened, restored on Escape. */
  private beforeSearch: View | null = null;

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  private setup(): void {
    const svg = this.querySelector<SVGSVGElement>("svg.graph");
    const world = svg?.querySelector<SVGGElement>(".graph-world");
    if (this.ready || !svg || !world) {
      return;
    }
    this.ready = true;
    this.svg = svg;
    this.world = world;
    for (const a of svg.querySelectorAll<SVGAElement>(".graph-nodes a")) {
      const circle = a.querySelector("circle");
      this.nodes[Number(a.dataset.i)] = {
        a,
        x: Number(circle?.getAttribute("cx")),
        y: Number(circle?.getAttribute("cy")),
        near: (a.dataset.n ?? "").split(" ").filter(Boolean).map(Number),
        folded: foldText(a.querySelector("title")?.textContent ?? ""),
      };
    }
    this.highlight = document.createElementNS(SVG_NS, "path");
    this.highlight.classList.add("graph-highlight");
    world.insertBefore(this.highlight, world.querySelector(".graph-nodes"));
    this.append(this.controls(), this.search());
    this.dataset.ready = "";
    // Ctrl+F searches the graph only where the graph is the page, not in a sidebar pane.
    if (this.closest("main")) {
      document.addEventListener("keydown", (e) => this.onDocumentKey(e));
    }

    svg.addEventListener("wheel", (e) => this.onWheel(e), { passive: false });
    svg.addEventListener("pointerdown", (e) => this.onPointerDown(e));
    svg.addEventListener("pointermove", (e) => this.onPointerMove(e));
    svg.addEventListener("pointerup", (e) => this.onPointerUp(e));
    svg.addEventListener("pointercancel", (e) => this.onPointerUp(e));
    // Capture phase: a drag ending on a node must not open it (nor a sidebar pane).
    svg.addEventListener("click", (e) => this.onClick(e), { capture: true });
    svg.addEventListener("pointerover", (e) => this.onPointerOver(e));
    svg.addEventListener("pointerout", (e) => this.onPointerOut(e));
    svg.addEventListener("focusin", (e) => this.onFocusIn(e));
    svg.addEventListener("focusout", () => this.unhighlight());
    new ResizeObserver(() => this.measure()).observe(svg);
    this.measure();
  }

  private controls(): HTMLElement {
    const box = document.createElement("div");
    box.className = "graph-controls";
    box.setAttribute("role", "group");
    box.setAttribute("aria-label", "Graph controls");
    const button = (label: string, name: IconName, act: () => void) => {
      const b = document.createElement("button");
      b.type = "button";
      b.setAttribute("aria-label", label);
      b.title = label;
      b.append(icon(name));
      b.addEventListener("click", act);
      box.append(b);
    };
    button("Zoom in", "zoom-in", () => this.zoomCentre(STEP));
    button("Zoom out", "zoom-out", () => this.zoomCentre(1 / STEP));
    button("Fit graph", "scan", () => this.setView(FIT));
    button(`Search the graph (${APPLE ? "⌘ F" : "Ctrl + F"})`, "search", () => this.openSearch());
    this.searchButton = box.lastElementChild as HTMLButtonElement;
    return box;
  }

  private search(): HTMLElement {
    const box = document.createElement("div");
    box.className = "graph-search";
    box.setAttribute("role", "search");
    box.hidden = true;
    const input = document.createElement("input");
    input.type = "search";
    input.placeholder = "Find pages in the graph";
    input.setAttribute("aria-label", "Find pages in the graph");
    input.setAttribute("aria-keyshortcuts", APPLE ? "Meta+F" : "Control+F");
    input.addEventListener("input", () => this.runSearch());
    input.addEventListener("keydown", (e) => this.onSearchKey(e));
    const count = document.createElement("output");
    count.setAttribute("aria-live", "polite");
    box.append(icon("search"), input, count);
    this.searchBox = box;
    this.searchInput = input;
    this.searchCount = count;
    return box;
  }

  private onDocumentKey(e: KeyboardEvent): void {
    const mod = APPLE ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
    if (!mod || e.altKey || e.shiftKey || e.key.toLowerCase() !== "f" || e.defaultPrevented) {
      return;
    }
    // Pressed again in the search box: the browser's own find. Behind another modal
    // dialog (site search, the shortcut list), the graph is out of reach: leave it too.
    if (document.activeElement === this.searchInput || document.querySelector("dialog:modal")) {
      return;
    }
    e.preventDefault();
    this.openSearch();
  }

  private openSearch(): void {
    if (this.searchBox.hidden) {
      this.beforeSearch = this.view;
      this.searchBox.hidden = false;
    }
    this.searchInput.focus();
    this.searchInput.select();
  }

  private closeSearch(): void {
    this.searchInput.value = "";
    this.runSearch();
    this.searchBox.hidden = true;
    if (this.beforeSearch) {
      this.setView(this.beforeSearch);
      this.beforeSearch = null;
    }
    this.searchButton.focus();
  }

  private runSearch(): void {
    for (const i of this.found) {
      this.nodes[i]?.a.classList.remove("match", "current");
    }
    this.unhighlight();
    const words = queryWords(this.searchInput.value);
    this.found = [];
    this.nodes.forEach((node, i) => {
      if (matches(node.folded, words)) {
        this.found.push(i);
        node.a.classList.add("match");
      }
    });
    this.current = -1;
    if (words.length > 0) {
      this.svg.dataset.search = "";
    } else {
      delete this.svg.dataset.search;
    }
    this.showCount();
  }

  private showCount(): void {
    const n = this.found.length;
    this.searchCount.value =
      this.searchInput.value.trim() === ""
        ? ""
        : n === 0
          ? "No pages"
          : this.current >= 0
            ? `${this.current + 1} of ${n}`
            : n === 1
              ? "1 page"
              : `${n} pages`;
  }

  private onSearchKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      this.closeSearch();
    } else if ((e.key === "ArrowDown" || e.key === "ArrowUp") && this.found.length > 0) {
      // Like the site search's results: the arrows move, wrapping around at either end.
      e.preventDefault();
      const n = this.found.length;
      const down = e.key === "ArrowDown";
      this.goToMatch(this.current < 0 ? (down ? 0 : n - 1) : (this.current + (down ? 1 : n - 1)) % n);
    } else if (e.key === "Enter" && this.found.length > 0) {
      e.preventDefault();
      if (this.current < 0 && this.found.length > 1) {
        // Several matches and none chosen: show the first rather than open a page unseen.
        this.goToMatch(0);
        return;
      }
      const node = this.nodes[this.found[Math.max(this.current, 0)]!];
      if (node) {
        // Like a click on the node: the sidebar's panes come along.
        location.href = carrySidebar(anchorHref(node.a), location.href);
      }
    }
  }

  /** Centres the match at `index` in `found`, zoomed in far enough to read every label. */
  private goToMatch(index: number): void {
    const previous = this.nodes[this.found[this.current] ?? -1];
    previous?.a.classList.remove("current");
    this.current = index;
    const i = this.found[index]!;
    const node = this.nodes[i]!;
    node.a.classList.add("current");
    this.highlightNode(i);
    const k = Math.min(Math.max(this.view.k, ALL_LABELS_PX / (SPACING * this.scale)), maxZoom(this.scale));
    const r = this.svg.getBoundingClientRect();
    const c = this.toSvg(r.left + r.width / 2, r.top + r.height / 2);
    this.setView(centreOn({ ...this.view, k }, node.x, node.y, c.x, c.y));
    this.showCount();
  }

  /** Re-reads how big the fitted graph is on screen. */
  private measure(): void {
    const m = this.svg.getScreenCTM();
    if (m && m.a > 0) {
      this.scale = m.a;
      this.render();
    }
  }

  private setView(view: View): void {
    this.view = view;
    if (!this.frame) {
      this.frame = requestAnimationFrame(() => {
        this.frame = 0;
        this.render();
      });
    }
  }

  private render(): void {
    this.world.setAttribute("transform", transform(this.view));
    // One screen pixel in graph units, for labels that keep their size on screen.
    this.svg.style.setProperty("--u", String(1 / (this.scale * this.view.k)));
    this.dataset.zoom = String(labelLevel(this.scale, this.view.k));
  }

  /** A point in client coordinates, in the SVG's viewBox units. */
  private toSvg(x: number, y: number): Point {
    const m = this.svg.getScreenCTM();
    return m ? { x: (x - m.e) / m.a, y: (y - m.f) / m.d } : { x, y };
  }

  private zoomAround(clientX: number, clientY: number, factor: number): void {
    const p = this.toSvg(clientX, clientY);
    this.setView(zoomAt(this.view, p.x, p.y, factor, maxZoom(this.scale)));
  }

  private zoomCentre(factor: number): void {
    const r = this.svg.getBoundingClientRect();
    this.zoomAround(r.left + r.width / 2, r.top + r.height / 2, factor);
  }

  private onWheel(e: WheelEvent): void {
    e.preventDefault();
    const lines = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1;
    // Trackpad pinches arrive as wheel events with Ctrl held and small deltas.
    const speed = e.ctrlKey ? 0.01 : 0.002;
    this.zoomAround(e.clientX, e.clientY, Math.exp(-e.deltaY * lines * speed));
    this.dataset.moving = "";
    clearTimeout(this.wheelTimer);
    this.wheelTimer = window.setTimeout(() => {
      if (!this.dragging) {
        delete this.dataset.moving;
      }
    }, WHEEL_IDLE_MS);
  }

  private onPointerDown(e: PointerEvent): void {
    if (e.pointerType === "mouse" && e.button !== 0) {
      return;
    }
    this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (this.pointers.size === 1) {
      this.pressedAt = { x: e.clientX, y: e.clientY };
    } else {
      // A second finger: pinch. Both pointers are captured so the gesture survives
      // leaving the drawing.
      this.startDrag();
      for (const id of this.pointers.keys()) {
        this.capture(id);
      }
    }
  }

  private onPointerMove(e: PointerEvent): void {
    const last = this.pointers.get(e.pointerId);
    if (!last) {
      return;
    }
    const now = { x: e.clientX, y: e.clientY };
    if (this.pointers.size === 1) {
      const from = this.pressedAt ?? last;
      if (!this.dragging && Math.hypot(now.x - from.x, now.y - from.y) > DRAG_PX) {
        // Captured only once it is a drag: capturing on press would send a plain click
        // to the SVG instead of the node's link.
        this.startDrag();
        this.capture(e.pointerId);
      }
      if (this.dragging) {
        this.setView(pan(this.view, (now.x - last.x) / this.scale, (now.y - last.y) / this.scale));
      }
    } else if (this.pointers.size === 2) {
      const other = [...this.pointers.entries()].find(([id]) => id !== e.pointerId)?.[1];
      if (other) {
        const before = { mid: midpoint(last, other), d: Math.hypot(last.x - other.x, last.y - other.y) };
        const after = { mid: midpoint(now, other), d: Math.hypot(now.x - other.x, now.y - other.y) };
        const moved = pan(this.view, (after.mid.x - before.mid.x) / this.scale, (after.mid.y - before.mid.y) / this.scale);
        const p = this.toSvg(after.mid.x, after.mid.y);
        const factor = before.d > 0 ? after.d / before.d : 1;
        this.setView(zoomAt(moved, p.x, p.y, factor, maxZoom(this.scale)));
      }
    }
    this.pointers.set(e.pointerId, now);
  }

  private onPointerUp(e: PointerEvent): void {
    if (!this.pointers.delete(e.pointerId)) {
      return;
    }
    if (this.pointers.size === 1) {
      // Lifting one finger of a pinch continues as a pan from the other one.
      this.pressedAt = null;
    } else if (this.pointers.size === 0) {
      this.pressedAt = null;
      if (this.dragging) {
        this.dragging = false;
        delete this.dataset.moving;
        // The click that may follow this pointerup ends the drag; it opens nothing.
        this.suppressClick = true;
        setTimeout(() => (this.suppressClick = false), 0);
      }
    }
  }

  private onClick(e: MouseEvent): void {
    if (this.suppressClick) {
      e.preventDefault();
      e.stopPropagation();
      this.suppressClick = false;
    }
  }

  private startDrag(): void {
    this.dragging = true;
    this.dataset.moving = "";
    this.unhighlight();
  }

  private capture(pointerId: number): void {
    try {
      this.svg.setPointerCapture(pointerId);
    } catch {
      // The pointer is already gone; nothing to capture.
    }
  }

  private nodeAt(target: EventTarget | null): number | null {
    const a = target instanceof Element ? target.closest<SVGAElement>(".graph-nodes a") : null;
    return a ? Number(a.dataset.i) : null;
  }

  private onPointerOver(e: PointerEvent): void {
    const i = this.nodeAt(e.target);
    if (i !== null && !this.dragging) {
      this.highlightNode(i);
    }
  }

  private onPointerOut(e: PointerEvent): void {
    if (this.nodeAt(e.relatedTarget) === null && document.activeElement?.closest(".graph-nodes a") == null) {
      this.unhighlight();
    }
  }

  private onFocusIn(e: FocusEvent): void {
    const i = this.nodeAt(e.target);
    if (i === null) {
      return;
    }
    this.highlightNode(i);
    // Keep the focused node in view when tabbing through the graph.
    const node = this.nodes[i];
    const r = this.svg.getBoundingClientRect();
    const m = this.svg.getScreenCTM();
    if (node && m) {
      const sx = (node.x * this.view.k + this.view.x) * m.a + m.e;
      const sy = (node.y * this.view.k + this.view.y) * m.d + m.f;
      const margin = 24;
      if (sx < r.left + margin || sx > r.right - margin || sy < r.top + margin || sy > r.bottom - margin) {
        const c = this.toSvg(r.left + r.width / 2, r.top + r.height / 2);
        this.setView(centreOn(this.view, node.x, node.y, c.x, c.y));
      }
    }
  }

  private highlightNode(i: number): void {
    if (this.focused === i) {
      return;
    }
    this.unhighlight();
    const node = this.nodes[i];
    if (!node) {
      return;
    }
    this.focused = i;
    this.svg.dataset.focus = "";
    node.a.classList.add("hl");
    let d = "";
    for (const j of node.near) {
      const other = this.nodes[j];
      if (other) {
        other.a.classList.add("hl");
        d += `M${node.x} ${node.y}L${other.x} ${other.y}`;
      }
    }
    this.highlight.setAttribute("d", d);
  }

  private unhighlight(): void {
    if (this.focused === null) {
      return;
    }
    const node = this.nodes[this.focused];
    for (const j of [this.focused, ...(node?.near ?? [])]) {
      this.nodes[j]?.a.classList.remove("hl");
    }
    this.focused = null;
    delete this.svg.dataset.focus;
    this.highlight.removeAttribute("d");
  }
}

function midpoint(a: Point, b: Point): Point {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

export function define(name = "hermetic-graph"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticGraph);
  }
}
