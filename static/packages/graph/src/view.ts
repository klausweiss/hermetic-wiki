// Pan and zoom of the graph, in the SVG's viewBox units: a point p of the drawing is shown
// at p·k + (x, y). The identity view is the whole graph, fitted by the viewBox.

export interface View {
  x: number;
  y: number;
  k: number;
}

export const FIT: View = { x: 0, y: 0, k: 1 };

/** How far out the reader may zoom, relative to the fitted graph. */
export const MIN_ZOOM = 1 / 1.5;

/** Distance between connected nodes in graph units (`SPACING` in src/site/graph_layout.rs). */
export const SPACING = 60;

/**
 * The view zoomed by `factor` around the point (px, py), which stays where it is on
 * screen; the zoom is kept between `MIN_ZOOM` and `max`.
 */
export function zoomAt(view: View, px: number, py: number, factor: number, max: number): View {
  const k = Math.min(Math.max(view.k * factor, MIN_ZOOM), Math.max(max, 1));
  const f = k / view.k;
  return { x: px - (px - view.x) * f, y: py - (py - view.y) * f, k };
}

export function pan(view: View, dx: number, dy: number): View {
  return { x: view.x + dx, y: view.y + dy, k: view.k };
}

/**
 * The highest zoom: connected nodes about 200 screen pixels apart, given how many screen
 * pixels one viewBox unit takes at the fitted view; never less than 4×.
 */
export function maxZoom(pxPerUnit: number): number {
  return Math.max(4, 200 / (SPACING * pxPerUnit));
}

/**
 * How many label tiers beyond the first to show (0–3), from the screen distance between
 * connected nodes: labels of smaller nodes only appear when there is room for them.
 */
export function labelLevel(pxPerUnit: number, k: number): number {
  const spacing = SPACING * pxPerUnit * k;
  return spacing < 25 ? 0 : spacing < 45 ? 1 : spacing < 70 ? 2 : 3;
}

/** The view that shows the point (px, py) of the drawing at (cx, cy), zoom unchanged. */
export function centreOn(view: View, px: number, py: number, cx: number, cy: number): View {
  return { x: cx - px * view.k, y: cy - py * view.k, k: view.k };
}

/** The SVG `transform` attribute for a view. */
export function transform(view: View): string {
  return `translate(${round(view.x)} ${round(view.y)}) scale(${round(view.k, 1e4)})`;
}

function round(v: number, by = 100): number {
  return Math.round(v * by) / by;
}
