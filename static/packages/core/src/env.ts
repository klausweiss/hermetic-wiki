// Where the site root is, and whether fetch-based features may run.

export interface Env {
  /** Absolute URL of the site root, ending in `/`. */
  root: URL;
  /**
   * True over http(s). Over file:// browsers block fetching other pages, so the
   * sidebar and timeline stay off and links keep their default behavior.
   */
  interactive: boolean;
}

export function readEnv(): Env {
  const meta = document.querySelector<HTMLMetaElement>('meta[name="hermetic-root"]');
  const rel = meta?.content || ".";
  const root = new URL(rel.endsWith("/") ? rel : `${rel}/`, location.href);
  const interactive = location.protocol === "http:" || location.protocol === "https:";
  return { root, interactive };
}
