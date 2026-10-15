// Deterministic benchmark corpora.
//
// The relevance corpus is this repository's own OpenSpec documents (main specs and
// every change's proposal and design), each turned into a page of bullet blocks, plus
// three months of journals in English and Polish that reference them. A few journal
// days carry unique entries so that journal queries have a known answer.
//
// The performance corpus is 5,000 generated journal days.

import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const REPO = new URL("../../../", import.meta.url).pathname;

/** A small deterministic PRNG (mulberry32). */
function random(seed: number): () => number {
  let a = seed;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Markdown document → bullet blocks: headings top-level, everything else under them. */
export function toBlocks(markdown: string): string {
  const out: string[] = [];
  let fence = false;
  let paragraph: string[] = [];
  const flush = () => {
    if (paragraph.length > 0) out.push(`  - ${paragraph.join(" ")}`);
    paragraph = [];
  };
  for (const raw of markdown.split("\n")) {
    const line = raw.trimEnd();
    if (line.trimStart().startsWith("```")) {
      fence = !fence;
      flush();
      continue;
    }
    if (fence || line.startsWith("<!--")) continue;
    const heading = /^#{1,6}\s+(.*)$/.exec(line);
    const item = /^\s*(?:[-*]|\d+\.)\s+(.*)$/.exec(line);
    if (heading) {
      flush();
      out.push(`- ${heading[1]}`);
    } else if (item) {
      flush();
      out.push(`  - ${item[1]}`);
    } else if (line.trim() === "") {
      flush();
    } else {
      paragraph.push(line.trim());
    }
  }
  flush();
  if (out.length > 0 && !out[0]!.startsWith("- ")) out.unshift("- notes");
  return `${out.join("\n")}\n`;
}

function writePage(notes: string, name: string, markdown: string): void {
  const file = join(notes, "pages", `${name}.md`);
  mkdirSync(join(file, ".."), { recursive: true });
  writeFileSync(file, toBlocks(markdown));
}

/** Names of the documents turned into pages, for journal references. */
function documents(notes: string): string[] {
  const names: string[] = [];
  const specs = join(REPO, "openspec/specs");
  for (const cap of readdirSync(specs).filter((d) => !d.startsWith(".")).sort()) {
    writePage(notes, `spec/${cap}`, readFileSync(join(specs, cap, "spec.md"), "utf8"));
    names.push(`spec/${cap}`);
  }
  const changes = join(REPO, "openspec/changes");
  const dirs = [
    ...readdirSync(join(changes, "archive")).filter((d) => !d.startsWith(".")).map((d) => join("archive", d)),
    ...readdirSync(changes).filter((d) => d !== "archive" && !d.startsWith(".")),
  ].sort();
  for (const dir of dirs) {
    const name = dir.replace(/^archive\//, "").replace(/^\d{4}-\d{2}-\d{2}-/, "");
    for (const kind of ["proposal", "design"]) {
      const file = join(changes, dir, `${kind}.md`);
      if (existsSync(file)) {
        writePage(notes, `${kind}/${name}`, readFileSync(file, "utf8"));
        names.push(`${kind}/${name}`);
      }
    }
  }
  return names;
}

/** Journal entries with nothing searchable except their page references. */
const FILLERS = [
  "Standup: omówiliśmy postępy w [[{p}]]",
  "Przejrzałem [[{p}]] razem z zespołem",
  "Reviewed [[{p}]] with the team",
  "Notatki ze spotkania o [[{p}]]",
  "TODO wrócić do [[{p}]]",
  "Read through [[{p}]] again",
  "Krótka rozmowa o [[{p}]]",
];

/** Unique entries: the known answers to journal queries. */
export const SPECIAL_DAYS: Record<string, string> = {
  "2026-07-20": "- Zażółć gęślą jaźń: testujemy polskie znaki\n",
  "2026-08-15": "- Decyzja: budujemy własną wyszukiwarkę zamiast Pagefind\n  - mniej zależności, mniejszy plik binarny\n",
  "2026-09-03": "- Przebudowa serwera trwa za długo, trzeba ją przyspieszyć [[spec/serve-mode]]\n",
};

export function relevanceCorpus(notes: string): void {
  rmSync(notes, { recursive: true, force: true });
  mkdirSync(join(notes, "journals"), { recursive: true });
  const pages = documents(notes);
  const rnd = random(42);
  const start = Date.UTC(2026, 6, 1);
  for (let i = 0; i < 92; i++) {
    const day = new Date(start + i * 86_400_000).toISOString().slice(0, 10);
    const lines: string[] = [];
    const count = 2 + Math.floor(rnd() * 3);
    for (let k = 0; k < count; k++) {
      const template = FILLERS[Math.floor(rnd() * FILLERS.length)]!;
      const page = pages[Math.floor(rnd() * pages.length)]!;
      lines.push(`- ${template.replace("{p}", page)}`);
    }
    writeFileSync(join(notes, "journals", `${day}.md`), (SPECIAL_DAYS[day] ?? "") + `${lines.join("\n")}\n`);
  }
}

export function performanceCorpus(notes: string): void {
  rmSync(notes, { recursive: true, force: true });
  mkdirSync(join(notes, "journals"), { recursive: true });
  const start = Date.UTC(2013, 0, 1);
  for (let i = 0; i < 5000; i++) {
    const day = new Date(start + i * 86_400_000).toISOString().slice(0, 10);
    writeFileSync(
      join(notes, "journals", `${day}.md`),
      `- standup about [[project ${i % 50}]] and #topic${i % 200}\n` +
        `  - TODO follow up with [[person ${i % 30}]] about redis ttl ${i % 7 === 0 ? "and the deploy freeze" : ""}\n` +
        `- notes\n  more **notes** with \`code\`\n`,
    );
  }
}
