// The Snowball English ("Porter2") stemmer: a line-for-line port of
// `src/search/stem.rs` (itself ported from Snowball's `algorithms/english.sbl`).
// `tests/fixtures/search/porter2.tsv` holds the shared vectors.

const PLAIN = /^[a-z]+$/;

const EXCEPTION1: Readonly<Record<string, string>> = {
  skis: "ski",
  skies: "sky",
  idly: "idl",
  gently: "gentl",
  ugly: "ugli",
  early: "earli",
  only: "onli",
  singly: "singl",
  sky: "sky",
  news: "news",
  howe: "howe",
  atlas: "atlas",
  cosmos: "cosmos",
  bias: "bias",
  andes: "andes",
};

const isV = (c: string | undefined): boolean => c !== undefined && "aeiouy".includes(c);

/** `gopast v gopast non-v` from `i`: the index after the first non-vowel after a vowel. */
function afterVowelConsonant(w: string[], i: number): number | null {
  let j = i;
  while (j < w.length && !isV(w[j])) j++;
  if (j >= w.length) return null;
  j++;
  while (j < w.length && isV(w[j])) j++;
  if (j >= w.length) return null;
  return j + 1;
}

const PREFIXES = ["gener", "commun", "arsen", "past", "univers", "later", "emerg", "organ", "inter"];

function regions(w: string[]): [number, number] {
  const n = w.length;
  const word = w.join("");
  let p1: number | null = null;
  for (const p of PREFIXES) {
    if (word.startsWith(p) && (p1 === null || p.length > p1)) p1 = p.length;
  }
  p1 ??= afterVowelConsonant(w, 0);
  if (p1 === null) return [n, n];
  return [p1, afterVowelConsonant(w, p1) ?? n];
}

const endsWith = (w: string[], s: string): boolean => w.length >= s.length && w.slice(w.length - s.length).join("") === s;

function longest(w: string[], suffixes: readonly string[]): string | null {
  let best: string | null = null;
  for (const s of suffixes) {
    if (endsWith(w, s) && (best === null || s.length > best.length)) best = s;
  }
  return best;
}

function replace(w: string[], suffixLen: number, by: string): void {
  w.splice(w.length - suffixLen, suffixLen, ...by);
}

function shortV(w: string[]): boolean {
  const n = w.length;
  if (endsWith(w, "past")) return true;
  const lastOk = (c: string | undefined) => c !== undefined && !isV(c) && c !== "w" && c !== "x" && c !== "Y";
  return (n >= 3 && lastOk(w[n - 1]) && isV(w[n - 2]) && !isV(w[n - 3])) || (n === 2 && !isV(w[1]) && isV(w[0]));
}

function step1a(w: string[]): void {
  const suffix = longest(w, ["sses", "ied", "ies", "s", "us", "ss"]);
  if (suffix === null) return;
  const start = w.length - suffix.length;
  if (suffix === "sses") replace(w, 4, "ss");
  else if (suffix === "ied" || suffix === "ies") replace(w, 3, start >= 2 ? "i" : "ie");
  else if (suffix === "s") {
    if (start >= 1 && w.slice(0, start - 1).some(isV)) w.length = start;
  }
}

const DOUBLES = ["bb", "dd", "ff", "gg", "mm", "nn", "pp", "rr", "tt"];

function step1b(w: string[], p1: number): void {
  const suffix = longest(w, ["eed", "eedly", "ed", "edly", "ingly", "ing"]);
  if (suffix === null) return;
  const start = w.length - suffix.length;
  if (suffix === "eed" || suffix === "eedly") {
    const pre = w.slice(0, start).join("");
    if (start >= p1 && pre !== "proc" && pre !== "exc" && pre !== "succ") replace(w, suffix.length, "ee");
    return;
  }
  if (suffix === "ing") {
    const pre = w.slice(0, start);
    const s = longest(pre, ["y", "inn", "out", "cann", "herr", "earr", "even"]);
    if (s === "y" && pre.length === 2 && !isV(pre[0])) {
      w.length = 1;
      w.push("i", "e");
      return;
    }
    if (s !== null && s !== "y" && pre.length === s.length) return;
  }
  if (!w.slice(0, start).some(isV)) return;
  w.length = start;
  if (endsWith(w, "at") || endsWith(w, "bl") || endsWith(w, "iz")) {
    w.push("e");
    return;
  }
  if (DOUBLES.some((d) => endsWith(w, d))) {
    const first = w[0];
    if (!(w.length === 3 && (first === "a" || first === "e" || first === "o"))) w.pop();
    return;
  }
  if (w.length === p1 && shortV(w)) w.push("e");
}

function step1c(w: string[]): void {
  const n = w.length;
  if (n >= 3 && (w[n - 1] === "y" || w[n - 1] === "Y") && !isV(w[n - 2])) w[n - 1] = "i";
}

const STEP2: readonly [string, string][] = [
  ["tional", "tion"],
  ["enci", "ence"],
  ["anci", "ance"],
  ["abli", "able"],
  ["entli", "ent"],
  ["izer", "ize"],
  ["ization", "ize"],
  ["ational", "ate"],
  ["ation", "ate"],
  ["ator", "ate"],
  ["alism", "al"],
  ["aliti", "al"],
  ["alli", "al"],
  ["fulness", "ful"],
  ["ousli", "ous"],
  ["ousness", "ous"],
  ["iveness", "ive"],
  ["iviti", "ive"],
  ["biliti", "ble"],
  ["bli", "ble"],
  ["ogist", "og"],
  ["ogi", "og"],
  ["fulli", "ful"],
  ["lessli", "less"],
  ["li", ""],
];

function longestRule(w: string[], rules: readonly [string, string][]): [string, string] | null {
  let best: [string, string] | null = null;
  for (const rule of rules) {
    if (endsWith(w, rule[0]) && (best === null || rule[0].length > best[0].length)) best = rule;
  }
  return best;
}

function step2(w: string[], p1: number): void {
  const rule = longestRule(w, STEP2);
  if (rule === null) return;
  const [suffix, by] = rule;
  const start = w.length - suffix.length;
  if (start < p1) return;
  const before = w[start - 1];
  if (suffix === "ogi" && before !== "l") return;
  if (suffix === "li" && (before === undefined || !"cdeghkmnrt".includes(before))) return;
  replace(w, suffix.length, by);
}

const STEP3: readonly [string, string][] = [
  ["tional", "tion"],
  ["ational", "ate"],
  ["alize", "al"],
  ["icate", "ic"],
  ["iciti", "ic"],
  ["ical", "ic"],
  ["ful", ""],
  ["ness", ""],
  ["ative", ""],
];

function step3(w: string[], p1: number, p2: number): void {
  const rule = longestRule(w, STEP3);
  if (rule === null) return;
  const [suffix, by] = rule;
  const start = w.length - suffix.length;
  if (start < p1 || (suffix === "ative" && start < p2)) return;
  replace(w, suffix.length, by);
}

const STEP4 = [
  "al", "ance", "ence", "er", "ic", "able", "ible", "ant", "ement", "ment",
  "ent", "ism", "ate", "iti", "ous", "ive", "ize", "ion",
];

function step4(w: string[], p2: number): void {
  const suffix = longest(w, STEP4);
  if (suffix === null) return;
  const start = w.length - suffix.length;
  if (start < p2) return;
  if (suffix === "ion" && !(w[start - 1] === "s" || w[start - 1] === "t")) return;
  w.length = start;
}

function step5(w: string[], p1: number, p2: number): void {
  const n = w.length;
  const start = n - 1;
  if (w[start] === "e") {
    if (start >= p2 || (start >= p1 && !shortV(w.slice(0, start)))) w.length = start;
  } else if (w[start] === "l") {
    if (start >= p2 && w[start - 1] === "l") w.length = start;
  }
}

/** Stems a folded word. Words that are not plain `a`-`z` are returned unchanged. */
export function stem(word: string): string {
  if (!PLAIN.test(word)) return word;
  const exception = EXCEPTION1[word];
  if (exception !== undefined) return exception;
  if (word.length < 3) return word;
  const w = [...word];
  let yFound = false;
  for (let i = 0; i < w.length; i++) {
    if (w[i] === "y" && (i === 0 || isV(w[i - 1]))) {
      w[i] = "Y";
      yFound = true;
    }
  }
  const [p1, p2] = regions(w);
  step1a(w);
  step1b(w, p1);
  step1c(w);
  step2(w, p1);
  step3(w, p1, p2);
  step4(w, p2);
  step5(w, p1, p2);
  const out = w.join("");
  return yFound ? out.replaceAll("Y", "y") : out;
}
