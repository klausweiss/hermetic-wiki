// <hermetic-graph>: wraps the graph view's SVG (written by hermetic, laid out at build
// time) and adds pan and zoom (wheel, drag, pinch, buttons), labels that appear as the
// reader zooms in, and highlighting of a node's neighbours. Without the script the SVG
// shows the whole graph fitted to the page, and its nodes are plain links.
//
// Panning and zooming need no other page, so they also work over file://.

import { icon, whenParsed } from "@hermetic/core";

import { FIT, type View, centreOn, labelLevel, maxZoom, pan, transform, zoomAt } from "./view";

const SVG_NS = "http://www.w3.org/2000/svg";
/** Zoom factor of the zoom buttons. */
const STEP = 1.5;
/** Pointer movement, in screen pixels, after which a press becomes a drag. */
const DRAG_PX = 4;
/** How long after the last wheel event the graph counts as still moving. */
const WHEEL_IDLE_MS = 150;

interface GraphNode {
  a: SVGAElement;
  /** Centre, in graph units. */
  x: number;
  y: number;
  /** Indices of the connected nodes. */
  near: number[];
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
      };
    }
    this.highlight = document.createElementNS(SVG_NS, "path");
    this.highlight.classList.add("graph-highlight");
    world.insertBefore(this.highlight, world.querySelector(".graph-nodes"));
    this.append(this.controls());
    this.dataset.ready = "";

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
    box.setAttribute("aria-label", "Graph zoom");
    const button = (label: string, name: "zoom-in" | "zoom-out" | "scan", act: () => void) => {
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
    return box;
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
