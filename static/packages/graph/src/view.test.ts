import { expect, test } from "bun:test";

import { FIT, MIN_ZOOM, centreOn, labelLevel, maxZoom, pan, transform, zoomAt } from "./view";

/** Where the view shows the drawing's point p. */
const screen = (v: { x: number; y: number; k: number }, p: [number, number]) => [
  Math.round((p[0] * v.k + v.x) * 1e6) / 1e6,
  Math.round((p[1] * v.k + v.y) * 1e6) / 1e6,
];

test("zooming keeps the point under the pointer in place", () => {
  const v = zoomAt(FIT, 100, 50, 2, 10);
  expect(v.k).toBe(2);
  expect(screen(v, [100, 50])).toEqual([100, 50]);
  const w = zoomAt(pan(v, 30, -20), 10, 20, 1.5, 10);
  // The drawing's point under (10, 20) before the zoom...
  const p: [number, number] = [(10 - (v.x + 30)) / v.k, (20 - (v.y - 20)) / v.k];
  // ...is still there after it.
  expect(screen(w, p)).toEqual([10, 20]);
});

test("zoom is limited", () => {
  expect(zoomAt(FIT, 0, 0, 0.01, 10).k).toBe(MIN_ZOOM);
  expect(zoomAt(FIT, 0, 0, 100, 10).k).toBe(10);
  // A maximum below the fitted view still allows the fitted view.
  expect(zoomAt(FIT, 0, 0, 2, 0.5).k).toBe(1);
});

test("zooming in and out again returns to the same view", () => {
  const v = zoomAt(zoomAt(FIT, 40, 60, 1.5, 10), 40, 60, 1 / 1.5, 10);
  expect(transform(v)).toBe(transform(FIT));
});

test("maximum zoom gives room between nodes", () => {
  expect(maxZoom(1)).toBe(4);
  // A big graph shrunk to a tenth: zooming up to 200 px between connected nodes.
  expect(Math.round(maxZoom(0.1) * 100) / 100).toBe(33.33);
});

test("labels appear as nodes move apart on screen", () => {
  expect(labelLevel(0.1, 1)).toBe(0);
  expect(labelLevel(0.1, 1.5 ** 4)).toBe(1);
  expect(labelLevel(0.1, 8)).toBe(2);
  expect(labelLevel(1, 2)).toBe(3);
});

test("centring a point", () => {
  const v = centreOn({ x: 5, y: 5, k: 2 }, 10, 20, 100, 100);
  expect(screen(v, [10, 20])).toEqual([100, 100]);
});

test("transform attribute", () => {
  expect(transform({ x: 1.234, y: -5, k: 1 / 3 })).toBe("translate(1.23 -5) scale(0.3333)");
});
