// Sidebar state (panes, shown or hidden) <-> URL query string. Pure functions: no DOM access.

import { isSafePagePath } from "./links";

export interface Pane {
  /** Root-relative page path, as in `PageLink.path`. */
  path: string;
  collapsed: boolean;
}

/** Query parameter holding one pane per occurrence, top pane first. */
export const PANE_PARAM = "p";
/** Prefix marking a collapsed pane. */
const COLLAPSED = "~";

/** Reads panes from a query string, dropping invalid and duplicate entries. */
export function readPanes(params: URLSearchParams): Pane[] {
  const panes: Pane[] = [];
  const seen = new Set<string>();
  for (const value of params.getAll(PANE_PARAM)) {
    const collapsed = value.startsWith(COLLAPSED);
    const path = collapsed ? value.slice(COLLAPSED.length) : value;
    if (!isSafePagePath(path) || seen.has(path)) {
      continue;
    }
    seen.add(path);
    panes.push({ path, collapsed });
  }
  return panes;
}

/** Returns a copy of `params` with the pane parameters replaced by `panes`. */
export function writePanes(params: URLSearchParams, panes: readonly Pane[]): URLSearchParams {
  const out = new URLSearchParams(params);
  out.delete(PANE_PARAM);
  for (const pane of panes) {
    out.append(PANE_PARAM, (pane.collapsed ? COLLAPSED : "") + pane.path);
  }
  return out;
}

/** `url` with its pane parameters replaced; other parameters and the fragment are kept. */
export function withPanes(url: string, panes: readonly Pane[]): string {
  const u = new URL(url);
  u.search = writePanes(u.searchParams, panes).toString();
  return u.href;
}

/**
 * Query parameter saying whether the sidebar is hidden or shown. Absent means the
 * default for the screen width, so it is only written when the reader overrides that.
 */
export const SIDEBAR_PARAM = "sidebar";
const HIDDEN = "hidden";
const SHOWN = "shown";

/** Whether the URL hides (true) or shows (false) the sidebar; null if it does not say. */
export function readHidden(params: URLSearchParams): boolean | null {
  const value = params.get(SIDEBAR_PARAM);
  return value === HIDDEN ? true : value === SHOWN ? false : null;
}

/** Returns a copy of `params` saying `hidden`, or nothing about it when null. */
export function writeHidden(params: URLSearchParams, hidden: boolean | null): URLSearchParams {
  const out = new URLSearchParams(params);
  out.delete(SIDEBAR_PARAM);
  if (hidden !== null) {
    out.set(SIDEBAR_PARAM, hidden ? HIDDEN : SHOWN);
  }
  return out;
}

/** `url` with its sidebar parameters (panes and visibility) replaced. */
export function withSidebar(url: string, panes: readonly Pane[], hidden: boolean | null): string {
  const u = new URL(url);
  u.search = writeHidden(writePanes(u.searchParams, panes), hidden).toString();
  return u.href;
}

/**
 * `target` with the sidebar state of `from`, for navigating with the sidebar kept as
 * it is. `target` is returned unchanged when `from` has no sidebar state.
 */
export function carrySidebar(target: string, from: string): string {
  const params = new URL(from).searchParams;
  const panes = readPanes(params);
  const hidden = readHidden(params);
  return panes.length === 0 && hidden === null ? target : withSidebar(target, panes, hidden);
}
