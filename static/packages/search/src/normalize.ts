// Turning text into search words. `src/search/normalize.rs` must do exactly the same;
// `tests/fixtures/search/normalize.tsv` holds the shared vectors.

/** Letters that NFKD does not decompose, folded to their usual base letters. */
const FOLD: Readonly<Record<string, string>> = {
  "ł": "l",
  "đ": "d",
  "ð": "d",
  "ø": "o",
  "ß": "ss",
  "æ": "ae",
  "œ": "oe",
  "ı": "i",
  "þ": "th",
};

const MARK = /\p{M}/u;
const WORD = /[\p{Alphabetic}\p{N}]+/gu;

/**
 * Folds text for matching: NFKD, combining marks removed, lowercased character by
 * character, and the letters NFKD keeps (`ł`, `ß`, `æ`, ...) folded to base letters.
 */
export function foldText(text: string): string {
  let out = "";
  for (const c of text.normalize("NFKD")) {
    if (MARK.test(c)) {
      continue;
    }
    for (const lower of c.toLowerCase()) {
      out += FOLD[lower] ?? lower;
    }
  }
  return out;
}

/** The search words of `text`: maximal runs of letters and digits of the folded text. */
export function words(text: string): string[] {
  return foldText(text).match(WORD) ?? [];
}
