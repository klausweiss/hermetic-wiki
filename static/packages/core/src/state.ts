// Sidebar panes <-> URL query string. Pure functions: no DOM access.

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
