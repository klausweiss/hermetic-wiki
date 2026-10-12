//! The Snowball English ("Porter2") stemmer, ported from Snowball's
//! `algorithms/english.sbl`. `static/packages/search/src/stem.ts` is the same port in
//! TypeScript; `tests/fixtures/search/porter2.tsv` holds the shared vectors.
//!
//! Snowball's backward mode reads suffixes right to left, so in a condition like
//! "non-v_WXY v non-v" the first test applies to the *last* letter.

/// Stems a folded word. Words that are not plain `a`-`z` are returned unchanged.
pub fn stem(word: &str) -> String {
    if word.is_empty() || !word.bytes().all(|b| b.is_ascii_lowercase()) {
        return word.to_owned();
    }
    if let Some(s) = exception1(word) {
        return s.to_owned();
    }
    if word.len() < 3 {
        return word.to_owned();
    }
    let mut w: Vec<u8> = word.bytes().collect();

    // prelude: initial y, and y after a vowel, are consonants (Y).
    let mut y_found = false;
    for i in 0..w.len() {
        if w[i] == b'y' && (i == 0 || is_v(w[i - 1])) {
            w[i] = b'Y';
            y_found = true;
        }
    }

    let (p1, p2) = regions(&w);
    step_1a(&mut w);
    step_1b(&mut w, p1);
    step_1c(&mut w);
    step_2(&mut w, p1);
    step_3(&mut w, p1, p2);
    step_4(&mut w, p2);
    step_5(&mut w, p1, p2);

    if y_found {
        for b in &mut w {
            if *b == b'Y' {
                *b = b'y';
            }
        }
    }
    String::from_utf8(w).expect("ASCII")
}

fn is_v(b: u8) -> bool {
    matches!(b, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
}

fn exception1(word: &str) -> Option<&'static str> {
    Some(match word {
        "skis" => "ski",
        "skies" => "sky",
        "idly" => "idl",
        "gently" => "gentl",
        "ugly" => "ugli",
        "early" => "earli",
        "only" => "onli",
        "singly" => "singl",
        "sky" => "sky",
        "news" => "news",
        "howe" => "howe",
        "atlas" => "atlas",
        "cosmos" => "cosmos",
        "bias" => "bias",
        "andes" => "andes",
        _ => return None,
    })
}

/// `gopast v gopast non-v` from `i`: the index after the first non-vowel that follows a
/// vowel, if any.
fn after_vowel_consonant(w: &[u8], i: usize) -> Option<usize> {
    let vowel = i + w[i..].iter().position(|&b| is_v(b))?;
    let consonant = vowel + 1 + w[vowel + 1..].iter().position(|&b| !is_v(b))?;
    Some(consonant + 1)
}

/// R1 and R2 start positions (`len` when empty).
fn regions(w: &[u8]) -> (usize, usize) {
    const PREFIXES: [&[u8]; 9] = [
        b"gener", b"commun", b"arsen", b"past", b"univers", b"later", b"emerg", b"organ", b"inter",
    ];
    let n = w.len();
    let prefix = PREFIXES
        .iter()
        .filter(|p| w.starts_with(p))
        .map(|p| p.len())
        .max();
    let Some(p1) = prefix.or_else(|| after_vowel_consonant(w, 0)) else {
        return (n, n);
    };
    let p2 = after_vowel_consonant(w, p1).unwrap_or(n);
    (p1, p2)
}

/// The longest of `suffixes` that `w` ends with.
fn longest<'s>(w: &[u8], suffixes: &[&'s [u8]]) -> Option<&'s [u8]> {
    suffixes
        .iter()
        .filter(|s| w.ends_with(s))
        .max_by_key(|s| s.len())
        .copied()
}

fn replace(w: &mut Vec<u8>, suffix_len: usize, with: &[u8]) {
    w.truncate(w.len() - suffix_len);
    w.extend_from_slice(with);
}

/// A short syllable at the end of `w`.
fn short_v(w: &[u8]) -> bool {
    let n = w.len();
    if w.ends_with(b"past") {
        return true;
    }
    let last_ok = |b: u8| !is_v(b) && !matches!(b, b'w' | b'x' | b'Y');
    (n >= 3 && last_ok(w[n - 1]) && is_v(w[n - 2]) && !is_v(w[n - 3]))
        || (n == 2 && !is_v(w[1]) && is_v(w[0]))
}

fn step_1a(w: &mut Vec<u8>) {
    let Some(suffix) = longest(w, &[b"sses", b"ied", b"ies", b"s", b"us", b"ss"]) else {
        return;
    };
    let start = w.len() - suffix.len();
    match suffix {
        b"sses" => replace(w, 4, b"ss"),
        b"ied" | b"ies" => replace(w, 3, if start >= 2 { b"i" } else { b"ie" }),
        // Delete if a vowel occurs before the letter preceding the s.
        b"s" if start >= 1 && w[..start - 1].iter().any(|&b| is_v(b)) => w.truncate(start),
        _ => {} // us, ss
    }
}

fn step_1b(w: &mut Vec<u8>, p1: usize) {
    let Some(suffix) = longest(w, &[b"eed", b"eedly", b"ed", b"edly", b"ingly", b"ing"]) else {
        return;
    };
    let start = w.len() - suffix.len();
    match suffix {
        b"eed" | b"eedly" => {
            if start >= p1 && !matches!(&w[..start], b"proc" | b"exc" | b"succ") {
                replace(w, suffix.len(), b"ee");
            }
            return;
        }
        b"ing" => {
            let pre = &w[..start];
            match longest(
                pre,
                &[b"y", b"inn", b"out", b"cann", b"herr", b"earr", b"even"],
            ) {
                // dying -> die, lying -> lie, tying -> tie, vying -> vie
                Some(b"y") if pre.len() == 2 && !is_v(pre[0]) => {
                    w.truncate(1);
                    w.extend_from_slice(b"ie");
                    return;
                }
                // inning, outing, canning, herring, earring, evening stay as they are.
                Some(s) if s != b"y" && pre.len() == s.len() => return,
                _ => {}
            }
        }
        _ => {}
    }
    // ed, edly, ing, ingly: delete if the preceding part contains a vowel.
    if !w[..start].iter().any(|&b| is_v(b)) {
        return;
    }
    w.truncate(start);
    if w.ends_with(b"at") || w.ends_with(b"bl") || w.ends_with(b"iz") {
        w.push(b'e');
        return;
    }
    const DOUBLES: [&[u8]; 9] = [
        b"bb", b"dd", b"ff", b"gg", b"mm", b"nn", b"pp", b"rr", b"tt",
    ];
    if DOUBLES.iter().any(|d| w.ends_with(d)) {
        // Keep the double in words like "add", "egg", "off".
        if !(w.len() == 3 && matches!(w[0], b'a' | b'e' | b'o')) {
            w.pop();
        }
        return;
    }
    if w.len() == p1 && short_v(w) {
        w.push(b'e');
    }
}

fn step_1c(w: &mut [u8]) {
    let n = w.len();
    if n >= 3 && matches!(w[n - 1], b'y' | b'Y') && !is_v(w[n - 2]) {
        w[n - 1] = b'i';
    }
}

fn step_2(w: &mut Vec<u8>, p1: usize) {
    const RULES: [(&[u8], &[u8]); 25] = [
        (b"tional", b"tion"),
        (b"enci", b"ence"),
        (b"anci", b"ance"),
        (b"abli", b"able"),
        (b"entli", b"ent"),
        (b"izer", b"ize"),
        (b"ization", b"ize"),
        (b"ational", b"ate"),
        (b"ation", b"ate"),
        (b"ator", b"ate"),
        (b"alism", b"al"),
        (b"aliti", b"al"),
        (b"alli", b"al"),
        (b"fulness", b"ful"),
        (b"ousli", b"ous"),
        (b"ousness", b"ous"),
        (b"iveness", b"ive"),
        (b"iviti", b"ive"),
        (b"biliti", b"ble"),
        (b"bli", b"ble"),
        (b"ogist", b"og"),
        (b"ogi", b"og"),
        (b"fulli", b"ful"),
        (b"lessli", b"less"),
        (b"li", b""),
    ];
    let Some((suffix, with)) = RULES
        .iter()
        .filter(|(s, _)| w.ends_with(s))
        .max_by_key(|(s, _)| s.len())
    else {
        return;
    };
    let start = w.len() - suffix.len();
    if start < p1 {
        return;
    }
    match *suffix {
        b"ogi" if start == 0 || w[start - 1] != b'l' => {}
        b"li" if start == 0 || !b"cdeghkmnrt".contains(&w[start - 1]) => {}
        _ => replace(w, suffix.len(), with),
    }
}

fn step_3(w: &mut Vec<u8>, p1: usize, p2: usize) {
    const RULES: [(&[u8], &[u8]); 9] = [
        (b"tional", b"tion"),
        (b"ational", b"ate"),
        (b"alize", b"al"),
        (b"icate", b"ic"),
        (b"iciti", b"ic"),
        (b"ical", b"ic"),
        (b"ful", b""),
        (b"ness", b""),
        (b"ative", b""),
    ];
    let Some((suffix, with)) = RULES
        .iter()
        .filter(|(s, _)| w.ends_with(s))
        .max_by_key(|(s, _)| s.len())
    else {
        return;
    };
    let start = w.len() - suffix.len();
    if start < p1 || (*suffix == b"ative" && start < p2) {
        return;
    }
    replace(w, suffix.len(), with);
}

fn step_4(w: &mut Vec<u8>, p2: usize) {
    const SUFFIXES: [&[u8]; 18] = [
        b"al", b"ance", b"ence", b"er", b"ic", b"able", b"ible", b"ant", b"ement", b"ment", b"ent",
        b"ism", b"ate", b"iti", b"ous", b"ive", b"ize", b"ion",
    ];
    let Some(suffix) = longest(w, &SUFFIXES) else {
        return;
    };
    let start = w.len() - suffix.len();
    if start < p2 {
        return;
    }
    if suffix == b"ion" && !(start > 0 && matches!(w[start - 1], b's' | b't')) {
        return;
    }
    w.truncate(start);
}

fn step_5(w: &mut Vec<u8>, p1: usize, p2: usize) {
    let n = w.len();
    match w.last() {
        Some(b'e') => {
            let start = n - 1;
            if start >= p2 || (start >= p1 && !short_v(&w[..start])) {
                w.truncate(start);
            }
        }
        Some(b'l') => {
            let start = n - 1;
            if start >= p2 && start > 0 && w[start - 1] == b'l' {
                w.truncate(start);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_vectors() {
        let vectors = include_str!("../../tests/fixtures/search/porter2.tsv");
        let mut failures = Vec::new();
        let mut checked = 0;
        for line in vectors.lines().filter(|l| !l.starts_with("##")) {
            let (word, expected) = line.split_once('\t').expect("tab-separated");
            if stem(word) != expected {
                failures.push(format!("{word}: got {}, want {expected}", stem(word)));
            }
            checked += 1;
        }
        assert!(
            failures.is_empty(),
            "{} of {checked} failed:\n{}",
            failures.len(),
            failures[..failures.len().min(30)].join("\n")
        );
        assert!(checked >= 5000);
    }

    #[test]
    fn examples() {
        for (w, s) in [
            ("testing", "test"),
            ("tested", "test"),
            ("tests", "test"),
            ("dying", "die"),
            ("inning", "inning"),
            ("generously", "generous"),
            ("news", "news"),
            ("żółć", "żółć"),
            ("ab", "ab"),
        ] {
            assert_eq!(stem(w), s, "{w}");
        }
    }
}
