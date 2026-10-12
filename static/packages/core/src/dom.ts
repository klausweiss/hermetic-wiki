// Helpers for custom elements defined before the document body is parsed.

/**
 * Runs `fn` once the document has been parsed. hermetic's script runs in <head>, so
 * elements are upgraded while their children are still missing; setup waits for them.
 */
export function whenParsed(fn: () => void): void {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", fn, { once: true });
  } else {
    fn();
  }
}

/**
 * Events through which components talk without depending on each other: dispatched on
 * `document` by whoever wants the sidebar to act, handled by <hermetic-sidebar>.
 */
export const SIDEBAR_CLOSE_TOP = "hermetic-sidebar-close-top";
export const SIDEBAR_TOGGLE = "hermetic-sidebar-toggle";

/**
 * Dispatched on `document` by <hermetic-live-reload> when pages other than the current
 * one changed; `detail` is `{ changed: string[], removed: string[] }` (site paths).
 */
export const PAGES_CHANGED = "hermetic-pages-changed";

/** Dispatched on `document` (by the `/` shortcut) to focus the search box. */
export const SEARCH_FOCUS = "hermetic-search-focus";

/**
 * Set on `<html>` by <hermetic-editor> while the page is open for editing. Then
 * <hermetic-live-reload> does not reload when the page itself changes, and includes it
 * in `PAGES_CHANGED` instead, so the editor can update the page in place.
 */
export const EDITING_ATTR = "data-hermetic-editing";

/** Dispatched on `document` (by the `g d` shortcut) to open today's journal. */
export const OPEN_TODAY = "hermetic-open-today";
