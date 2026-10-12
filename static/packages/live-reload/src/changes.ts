// Matching server change notifications to pages. Pure functions: no DOM access.

/** What `hermetic serve` sends after a rebuild: site paths, not percent-encoded. */
export interface Changes {
  changed: string[];
  removed: string[];
}

/** Parses an event payload; null for anything that is not a change notification. */
export function parseChanges(data: string): Changes | null {
  try {
    const value: unknown = JSON.parse(data);
    if (typeof value !== "object" || value === null) {
      return null;
    }
    const { changed, removed } = value as Record<string, unknown>;
    const strings = (v: unknown): v is string[] => Array.isArray(v) && v.every((s) => typeof s === "string");
    return strings(changed) && strings(removed) ? { changed, removed } : null;
  } catch {
    return null;
  }
}

/**
 * The site path of the page at `pathname` (a URL path, percent-encoded), relative to the
 * site root's path, decoded like the server's paths. Directory URLs mean `index.html`.
 */
export function sitePath(pathname: string, rootPathname: string): string | null {
  if (!pathname.startsWith(rootPathname)) {
    return null;
  }
  let path: string;
  try {
    path = decodeURIComponent(pathname.slice(rootPathname.length));
  } catch {
    return null;
  }
  return path === "" || path.endsWith("/") ? `${path}index.html` : path;
}

/** Whether a site path changed or disappeared. */
export function affects(changes: Changes, path: string): boolean {
  return changes.changed.includes(path) || changes.removed.includes(path);
}
