// Fetching another generated page and extracting its content for reuse in this one.

import { absolutize } from "./links";

/** Attribute marking the element a fragment pointed at, after ids were removed. */
export const TARGET_ATTR = "data-hermetic-target";

export interface FetchedPage {
  /** The page's `main > article`, adopted into this document. */
  article: HTMLElement;
  /** The page title without the site name. */
  title: string;
}

/**
 * Fetches `url` and returns its article with every `href`/`src` made absolute (so links
 * keep pointing where they did on the original page) and ids removed (so they cannot
 * clash with this page's anchors). The element the URL's fragment pointed at is
 * marked with `TARGET_ATTR`. Returns null when the page is missing or has no article.
 */
export async function fetchPage(url: URL): Promise<FetchedPage | null> {
  let response: Response;
  try {
    response = await fetch(url.href, { credentials: "same-origin" });
  } catch {
    return null;
  }
  if (!response.ok) {
    return null;
  }
  const doc = new DOMParser().parseFromString(await response.text(), "text/html");
  const article = doc.querySelector<HTMLElement>("main > article");
  if (!article) {
    return null;
  }
  const pageUrl = url.href.split("#")[0] ?? url.href;
  for (const attr of ["href", "src"]) {
    for (const el of article.querySelectorAll(`[${attr}]`)) {
      el.setAttribute(attr, absolutize(el.getAttribute(attr) ?? "", pageUrl));
    }
  }
  const fragment = decodeURIComponent(url.hash.slice(1));
  if (fragment) {
    article.querySelector(`[id="${CSS.escape(fragment)}"]`)?.setAttribute(TARGET_ATTR, "");
  }
  for (const el of article.querySelectorAll("[id]")) {
    el.removeAttribute("id");
  }
  const title = (doc.title.split(" · ")[0] ?? doc.title).trim();
  return { article: document.adoptNode(article), title };
}
