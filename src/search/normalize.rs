//! Turning text into search words. `static/packages/search/src/normalize.ts` must do
//! exactly the same; `tests/fixtures/search/normalize.tsv` holds the shared vectors.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Letters that NFKD does not decompose, folded to their usual base letters.
fn fold(c: char, out: &mut String) {
    match c {
        'ł' => out.push('l'),
        'đ' | 'ð' => out.push('d'),
        'ø' => out.push('o'),
        'ß' => out.push_str("ss"),
        'æ' => out.push_str("ae"),
        'œ' => out.push_str("oe"),
        'ı' => out.push('i'),
        'þ' => out.push_str("th"),
        c => out.push(c),
    }
}

/// Folds text for matching: NFKD, combining marks removed, lowercased character by
/// character, and the letters NFKD keeps (`ł`, `ß`, `æ`, ...) folded to base letters.
pub fn fold_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.nfkd().filter(|c| !is_combining_mark(*c)) {
        for lower in c.to_lowercase() {
            fold(lower, &mut out);
        }
    }
    out
}

/// The search words of `text`: maximal runs of letters and digits of the folded text.
pub fn words(text: &str) -> Vec<String> {
    fold_text(text)
        .split(|c: char| !(c.is_alphabetic() || c.is_numeric()))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_vectors() {
        let vectors = include_str!("../../tests/fixtures/search/normalize.tsv");
        let mut checked = 0;
        for line in vectors.lines().filter(|l| !l.starts_with("##")) {
            let (input, expected) = line.split_once('\t').expect("tab-separated");
            let input = input.replace("\\t", "\t").replace("\\n", "\n");
            let expected: Vec<&str> = expected.split(' ').filter(|w| !w.is_empty()).collect();
            assert_eq!(words(&input), expected, "{input:?}");
            checked += 1;
        }
        assert!(checked >= 50);
    }
}
