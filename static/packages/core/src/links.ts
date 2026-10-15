// Classifying links and rewriting URLs. Pure functions: no DOM access.

/** A link to a generated page of this site, by its path relative to the site root. */
export interface PageLink {
  kind: "page";
  /** Root-relative path, percent-encoded as in the URL (e.g. `pages/Project%20X.html`). */
  path: string;
  /** Fragment without `#`, or "" when there is none. */
  fragment: string;
}

export type Link = PageLink | { kind: "other" };

/**
 * Classifies `href` (resolved against `base`) relative to the site root URL `root`.
 * Pages are same-origin `.html` files under the root, outside hermetic's own `_hermetic/`.
 * Everything else (attachments, other sites, `mailto:`) is "other".
 */
export function classify(href: string, base: string, root: string): Link {
  let url: URL;
  let rootUrl: URL;
  try {
    url = new URL(href, base);
    rootUrl = new URL(root);
  } catch {
    return { kind: "other" };
  }
  if (url.protocol !== rootUrl.protocol || url.host !== rootUrl.host) {
    return { kind: "other" };
  }
  if (!url.pathname.startsWith(rootUrl.pathname)) {
    return { kind: "other" };
  }
  const path = url.pathname.slice(rootUrl.pathname.length);
  if (path === "" || path.startsWith("_hermetic/") || !path.endsWith(".html")) {
    return { kind: "other" };
  }
  return { kind: "page", path, fragment: url.hash.slice(1) };
}

/** Makes an attribute URL absolute against the URL of the page it came from. */
export function absolutize(value: string, pageUrl: string): string {
  try {
    return new URL(value, pageUrl).href;
  } catch {
    return value;
  }
}

/**
 * Rewrites an attribute URL so it keeps pointing at the same place from another page of the
 * site. Relative URLs (the wiki's own links) become root-relative (`/pages/x.html`) rather
 * than absolute, so they do not read as links leaving the site (`[href^="http://"]`); URLs
 * written with a scheme or host stay absolute, even when they point at this site.
 */
export function rebase(value: string, pageUrl: string): string {
  const url = absolutize(value, pageUrl);
  if (/^[a-z][a-z0-9+.-]*:/i.test(value.trim()) || value.trim().startsWith("//")) {
    return url;
  }
  try {
    const parsed = new URL(url);
    return parsed.pathname + parsed.search + parsed.hash;
  } catch {
    return url;
  }
}

/** The parts of an `<a>` element (HTML or SVG) that say where it links to. */
export interface Anchor {
  getAttribute(name: string): string | null;
  baseURI: string;
}

/**
 * The absolute URL a link points to. Works for SVG links too, whose `href` property is
 * not a string, by resolving the attribute the way the browser does when it is followed.
 */
export function anchorHref(a: Anchor): string {
  return absolutize(a.getAttribute("href") ?? "", a.baseURI);
}

/** Whether a root-relative path is safe to request: no scheme, no `..`, ends in `.html`. */
export function isSafePagePath(path: string): boolean {
  if (path === "" || path.startsWith("/") || /^[a-z][a-z0-9+.-]*:/i.test(path)) {
    return false;
  }
  if (/[?#\\]/.test(path) || !path.endsWith(".html")) {
    return false;
  }
  return path.split("/").every((s) => s !== "" && s !== "." && s !== "..");
}
