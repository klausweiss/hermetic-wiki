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

/** Dispatched on `document` (by the `/` shortcut) to open search with its field focused. */
export const SEARCH_FOCUS = "hermetic-search-focus";

/** Dispatched on `document` (by `Ctrl+K`) to open search, or close it when it is open. */
export const SEARCH_TOGGLE = "hermetic-search-toggle";

/**
 * Set on `<html>` by <hermetic-editor> while the page is open for editing. Then
 * <hermetic-live-reload> does not reload when the page itself changes, and includes it
 * in `PAGES_CHANGED` instead, so the editor can update the page in place.
 */
export const EDITING_ATTR = "data-hermetic-editing";

/** Dispatched on `document` (by the `g d` shortcut) to open today's journal. */
export const OPEN_TODAY = "hermetic-open-today";

/** Dispatched on `document` (by the `, t` shortcut) to switch between light and dark. */
export const THEME_SWITCH = "hermetic-theme-switch";

/** The theme the page shows: the chosen one, or the system's when none is chosen. */
export function shownTheme(): "light" | "dark" {
  const chosen = document.documentElement.dataset["theme"];
  if (chosen === "light" || chosen === "dark") return chosen;
  return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** Dispatched on `document` (by the `p` shortcut) to edit the page's properties. */
export const EDIT_PROPERTIES = "hermetic-edit-properties";
