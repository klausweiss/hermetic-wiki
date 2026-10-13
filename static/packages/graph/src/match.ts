// Matching page names against a graph search query.

import { foldText } from "@hermetic/search/normalize";

/** The folded words of a query; an empty list matches nothing. */
export function queryWords(query: string): string[] {
  return foldText(query).split(/\s+/).filter(Boolean);
}

/**
 * Whether a page name matches: every query word appears in it, ignoring case and
 * diacritics as site search does. `folded` is the name passed through `foldText`.
 */
export function matches(folded: string, words: readonly string[]): boolean {
  return words.length > 0 && words.every((w) => folded.includes(w));
}
