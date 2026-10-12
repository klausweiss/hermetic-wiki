// Evaluating a query against the index: candidate terms, BM25F scoring, all words
// required (falling back to any word), typo tolerance, and ranking boosts.

import { words } from "./normalize";
import { unpack } from "./pack";
import { type IndexSource, type Meta, type Term, shardKey } from "./source";
import { stem } from "./stem";

/** Field weights: [page names, block text]. */
export const FIELD_WEIGHT: readonly [number, number] = [3.0, 1.0];
/** How much a candidate term counts relative to an exact match. */
export const WEIGHT = { exact: 1.0, stem: 0.9, prefixBase: 0.5, prefixLength: 0.4, typo: 0.4 } as const;
/** Boost for the most recent journal day, scaled down linearly to 0 for the oldest. */
export const RECENCY_BOOST = 0.15;
/** Words shorter than this are not matched as prefixes. */
const MIN_PREFIX = 2;
/** Words shorter than this get no typo tolerance. */
const MIN_TYPO = 4;

export interface Hit {
  doc: number;
  score: number;
  /** Index terms that matched, for highlighting. */
  matched: Set<string>;
  /** Not every query word matched. */
  partial: boolean;
  /** Found only by typo tolerance. */
  approximate: boolean;
  /** A page whose name is exactly the query. */
  exactName: boolean;
}

interface Candidate {
  term: Term;
  weight: number;
}

interface WordScore {
  score: number;
  terms: Set<string>;
  /** Matched the page name exactly (field 0, exact term). */
  exactName: boolean;
}

/** Whether `a` and `b` differ by one insertion, deletion, substitution, or swap. */
export function withinOneEdit(a: string, b: string): boolean {
  if (a === b) return false;
  const x = [...a];
  const y = [...b];
  if (Math.abs(x.length - y.length) > 1) return false;
  let i = 0;
  while (i < x.length && i < y.length && x[i] === y[i]) i++;
  if (x.length === y.length) {
    const rest = (k: number) => x.slice(k).join("") === y.slice(k).join("");
    return rest(i + 1) || (x[i] === y[i + 1] && x[i + 1] === y[i] && rest(i + 2));
  }
  const [long, short] = x.length > y.length ? [x, y] : [y, x];
  return long.slice(i + 1).join("") === short.slice(i).join("");
}

function candidates(word: string, terms: Term[], typo: boolean): Candidate[] {
  const wordStem = stem(word);
  const out: Candidate[] = [];
  for (const term of terms) {
    const [t, s] = term;
    let weight = 0;
    if (typo) {
      if (word.length >= MIN_TYPO && withinOneEdit(word.slice(2), t.slice(2)) && word.slice(0, 2) === t.slice(0, 2)) {
        weight = WEIGHT.typo;
      }
    } else if (t === word) {
      weight = WEIGHT.exact;
    } else if ((s || t) === wordStem) {
      weight = WEIGHT.stem;
    } else if (word.length >= MIN_PREFIX && t.startsWith(word)) {
      weight = WEIGHT.prefixBase + (WEIGHT.prefixLength * word.length) / t.length;
    }
    if (weight > 0) out.push({ term, weight });
  }
  return out;
}

/** Best BM25F contribution of one query word, per document. */
function scoreWord(meta: Meta, cands: Candidate[]): Map<number, WordScore> {
  const out = new Map<number, WordScore>();
  for (const { term, weight } of cands) {
    const numbers = unpack(term[2]);
    const df = numbers.length / 3;
    const idf = Math.log(1 + (meta.n_docs - df + 0.5) / (df + 0.5));
    let doc = 0;
    for (let i = 0; i < numbers.length; i += 3) {
      doc += numbers[i] ?? 0;
      const tfField = numbers[i + 1] ?? 0;
      const len = numbers[i + 2] ?? 0;
      const field = (tfField % 2) as 0 | 1;
      const tf = Math.floor(tfField / 2) * FIELD_WEIGHT[field];
      const avg = meta.avg_len[field] || 1;
      const score = (idf * tf * (meta.k1 + 1)) / (tf + meta.k1 * (1 - meta.b + (meta.b * len) / avg)) * weight;
      const exactName = field === 0 && weight === WEIGHT.exact;
      const best = out.get(doc);
      if (!best) {
        out.set(doc, { score, terms: new Set([term[0]]), exactName });
      } else {
        best.terms.add(term[0]);
        best.exactName ||= exactName;
        best.score = Math.max(best.score, score);
      }
    }
  }
  return out;
}

/** Name length of page documents, from their name-field postings. */
function nameLengths(cands: Candidate[]): Map<number, number> {
  const out = new Map<number, number>();
  for (const { term } of cands) {
    const numbers = unpack(term[2]);
    let doc = 0;
    for (let i = 0; i < numbers.length; i += 3) {
      doc += numbers[i] ?? 0;
      if ((numbers[i + 1] ?? 0) % 2 === 0) out.set(doc, numbers[i + 2] ?? 0);
    }
  }
  return out;
}

function evaluate(meta: Meta, queryWords: string[], shards: Map<string, Term[]>, typo: boolean): Hit[] {
  const perWord: Map<number, WordScore>[] = [];
  const lengths = new Map<number, number>();
  for (const word of queryWords) {
    const cands = candidates(word, shards.get(shardKey(word)) ?? [], typo);
    perWord.push(scoreWord(meta, cands));
    for (const [doc, len] of nameLengths(cands)) lengths.set(doc, len);
  }
  const combine = (docs: Iterable<number>, partial: boolean): Hit[] => {
    const hits: Hit[] = [];
    for (const doc of docs) {
      let score = 0;
      const matched = new Set<string>();
      let exact = true;
      for (const scores of perWord) {
        const s = scores.get(doc);
        if (!s) {
          exact = false;
          continue;
        }
        score += s.score;
        s.terms.forEach((t) => matched.add(t));
        exact &&= s.exactName;
      }
      const exactName = !partial && !typo && exact && lengths.get(doc) === queryWords.length;
      if (meta.journal_docs) {
        const [first, last] = meta.journal_docs;
        if (doc >= first && doc <= last && last > first) {
          score *= 1 + (RECENCY_BOOST * (doc - first)) / (last - first);
        }
      }
      hits.push({ doc, score, matched, partial, approximate: typo, exactName });
    }
    return hits;
  };
  const [first, ...rest] = perWord;
  if (!first) return [];
  const all = [...first.keys()].filter((doc) => rest.every((m) => m.has(doc)));
  if (all.length > 0) return combine(all, false);
  const any = new Set<number>();
  perWord.forEach((m) => m.forEach((_, doc) => any.add(doc)));
  return combine(any, queryWords.length > 1);
}

export interface SearchResult {
  words: string[];
  hits: Hit[];
}

/** Runs a query, loading only the shards its words need. */
export async function search(source: IndexSource, query: string, limit: number): Promise<SearchResult> {
  const queryWords = [...new Set(words(query))];
  if (queryWords.length === 0) return { words: [], hits: [] };
  const meta = await source.meta();
  const existing = new Set(meta.shards);
  const shards = new Map<string, Term[]>();
  await Promise.all(
    [...new Set(queryWords.map(shardKey))]
      .filter((key) => existing.has(key))
      .map(async (key) => shards.set(key, await source.shard(key))),
  );
  let hits = evaluate(meta, queryWords, shards, false);
  if (hits.length === 0) hits = evaluate(meta, queryWords, shards, true);
  hits.sort((a, b) => Number(b.exactName) - Number(a.exactName) || b.score - a.score || a.doc - b.doc);
  return { words: queryWords, hits: hits.slice(0, limit) };
}
