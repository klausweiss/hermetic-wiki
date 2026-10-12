// The search benchmark: relevance of hermetic's search against Pagefind on judged
// queries, plus index size, bytes loaded per query, and latency on 5,000 journals.
// Development only: it downloads Pagefind and uses a headless browser.

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { chromium } from "playwright-core";

import { words } from "../../../static/packages/search/src/normalize";
import { stem } from "../../../static/packages/search/src/stem";
import { performanceCorpus } from "./corpus";

const HERE = new URL("..", import.meta.url).pathname;
const REPO = join(HERE, "../..");
const BIN = join(REPO, "target/release/hermetic");
const OUT = join(HERE, "out");
const PAGEFIND = "pagefind@1.5.2";
const PERF_QUERIES = ["redis ttl", "standup project", "follow up person", "deploy freeze", "topic12"];
const LATENCY_MS = 50;
const QUERY_BYTES = 300_000;

interface Query {
  q: string;
  category: string;
  relevant: string[];
}

interface Run {
  pages: string[];
  coldMs: number;
  warmMs: number;
  bytes: number;
}

function run(cmd: string, args: string[]): void {
  const r = spawnSync(cmd, args, { stdio: ["ignore", "ignore", "inherit"] });
  if (r.status !== 0) throw new Error(`${cmd} ${args.join(" ")} failed`);
}

function files(dir: string): string[] {
  return readdirSync(dir, { recursive: true }).map(String).filter((f) => statSync(join(dir, f)).isFile());
}

function size(dir: string, filter: (f: string) => boolean = () => true): number {
  return files(dir).filter(filter).reduce((sum, f) => sum + statSync(join(dir, f)).size, 0);
}

/** Every judged page must exist and contain the query's words (except typo queries). */
function checkJudgments(queries: Query[]): string[] {
  const problems: string[] = [];
  for (const { q, category, relevant } of queries) {
    for (const page of relevant) {
      const source = join(HERE, "corpus", page.replace(/\.html$/, ".md"));
      if (!existsSync(source)) {
        problems.push(`"${q}": judged page ${page} does not exist`);
        continue;
      }
      if (category === "typo") continue;
      const text = new Set(words(readFileSync(source, "utf8")).flatMap((w) => [w, stem(w)]));
      const missing = words(q).filter((w) => ![...text].some((t) => t === stem(w) || t.startsWith(w)));
      if (missing.length > 0) problems.push(`"${q}": ${page} lacks ${missing.join(", ")}`);
    }
  }
  return problems;
}

function metrics(queries: Query[], runs: Run[]): { success5: number; mrr: number } {
  let success = 0;
  let rr = 0;
  queries.forEach((query, i) => {
    const relevant = new Set(query.relevant.map((p) => p.replace(/\.html$/, "")));
    const rank = (runs[i]?.pages ?? []).findIndex((p) => relevant.has(p));
    if (rank >= 0 && rank < 5) success++;
    if (rank >= 0) rr += 1 / (rank + 1);
  });
  return { success5: success / queries.length, mrr: rr / queries.length };
}

async function main(): Promise<void> {
  if (!existsSync(BIN)) throw new Error(`${BIN} is missing: run cargo build --release`);
  const queries = JSON.parse(readFileSync(join(HERE, "queries.json"), "utf8")) as Query[];
  const problems = checkJudgments(queries);
  if (problems.some((p) => p.includes("does not exist"))) throw new Error(problems.join("\n"));
  for (const p of problems) console.warn(`warning: ${p}`);

  rmSync(OUT, { recursive: true, force: true });
  mkdirSync(OUT, { recursive: true });
  console.log("building the corpora…");
  run(BIN, ["build", join(HERE, "corpus"), join(OUT, "site")]);
  performanceCorpus(join(OUT, "perf-notes"));
  run(BIN, ["build", join(OUT, "perf-notes"), join(OUT, "perf-site")]);

  // Pagefind indexes a copy restricted to each page's own title and content, the same
  // text hermetic indexes (no linked references or other repeated sections).
  console.log(`indexing with ${PAGEFIND}…`);
  cpSync(join(OUT, "site"), join(OUT, "pf-site"), { recursive: true });
  for (const f of files(join(OUT, "pf-site")).filter((f) => f.endsWith(".html"))) {
    const path = join(OUT, "pf-site", f);
    const html = readFileSync(path, "utf8")
      .replace('<header class="page-header">', '<header class="page-header" data-pagefind-body>')
      .replace('<section class="content">', '<section class="content" data-pagefind-body>');
    writeFileSync(path, html);
  }
  run("bunx", [PAGEFIND, "--site", join(OUT, "pf-site"), "--silent"]);

  const built = await Bun.build({ entrypoints: [join(HERE, "src/browser.ts")], outdir: OUT, target: "browser" });
  if (!built.success) throw new Error(built.logs.join("\n"));
  writeFileSync(join(OUT, "bench.html"), '<!doctype html><meta charset="utf-8"><script type="module" src="/browser.js"></script>\n');

  const mounts: [string, string][] = [
    ["/h/", join(OUT, "site")],
    ["/p/", join(OUT, "pf-site")],
    ["/perf/", join(OUT, "perf-site")],
    ["/", OUT],
  ];
  const server = Bun.serve({
    port: 0,
    async fetch(req) {
      const path = decodeURIComponent(new URL(req.url).pathname);
      for (const [prefix, dir] of mounts) {
        if (path.startsWith(prefix)) {
          const file = Bun.file(join(dir, path.slice(prefix.length)));
          return (await file.exists()) ? new Response(file) : new Response("not found", { status: 404 });
        }
      }
      return new Response("not found", { status: 404 });
    },
  });

  const env = { ...process.env };
  delete env.LD_LIBRARY_PATH; // Nix profiles can point browsers at incompatible libraries.
  const browser = await chromium.launch({ env });
  const page = await browser.newPage();
  await page.goto(`http://localhost:${server.port}/bench.html`);
  await page.waitForFunction(() => "bench" in window);
  const call = (engine: "hermetic" | "pagefind", base: string, q: string): Promise<Run> =>
    page.evaluate(
      ([e, b, query]) => (window as unknown as { bench: Record<string, (b: string, q: string) => Promise<Run>> }).bench[e]!(b, query),
      [engine, base, q] as const,
    );

  const hermeticRuns: Run[] = [];
  const pagefindRuns: Run[] = [];
  for (const { q } of queries) {
    hermeticRuns.push(await call("hermetic", "/h/", q));
    pagefindRuns.push(await call("pagefind", "/p/", q));
  }
  const perfRuns: (Run & { q: string })[] = [];
  for (const q of PERF_QUERIES) perfRuns.push({ q, ...(await call("hermetic", "/perf/", q)) });
  await browser.close();
  server.stop();

  const h = metrics(queries, hermeticRuns);
  const p = metrics(queries, pagefindRuns);
  const pct = (x: number) => `${(x * 100).toFixed(1)}%`;
  console.log("\nper query (rank of first relevant page; - = not found in top 20)");
  queries.forEach((query, i) => {
    const rank = (runs: Run[]) => {
      const rel = new Set(query.relevant.map((r) => r.replace(/\.html$/, "")));
      const k = (runs[i]?.pages ?? []).findIndex((x) => rel.has(x));
      return k < 0 ? "-" : String(k + 1);
    };
    console.log(`  ${query.category.padEnd(10)} ${JSON.stringify(query.q).padEnd(34)} hermetic ${rank(hermeticRuns).padStart(2)}   pagefind ${rank(pagefindRuns).padStart(2)}`);
  });
  const mdBytes = size(join(HERE, "corpus"));
  const hIndex = size(join(OUT, "site/search"));
  const pIndex = size(join(OUT, "pf-site/pagefind"), (f) => /^(index|fragment|filter)\/|pagefind-entry\.json$/.test(f));
  const perfMd = size(join(OUT, "perf-notes"));
  const perfIndex = size(join(OUT, "perf-site/search"));
  const maxWarm = Math.max(...perfRuns.map((r) => r.warmMs));
  const maxBytes = Math.max(...perfRuns.map((r) => r.bytes));
  console.log(`
                      hermetic     pagefind
  success@5           ${pct(h.success5).padEnd(12)} ${pct(p.success5)}
  MRR                 ${h.mrr.toFixed(3).padEnd(12)} ${p.mrr.toFixed(3)}
  index (corpus)      ${(hIndex / 1024).toFixed(0).padEnd(9)} KB ${(pIndex / 1024).toFixed(0)} KB (markdown ${(mdBytes / 1024).toFixed(0)} KB)

  5,000 journals (markdown ${(perfMd / 1024).toFixed(0)} KB, index ${(perfIndex / 1024).toFixed(0)} KB)`);
  for (const r of perfRuns) {
    console.log(`    ${JSON.stringify(r.q).padEnd(20)} loaded ${(r.bytes / 1024).toFixed(0).padStart(4)} KB, cold ${r.coldMs.toFixed(1).padStart(6)} ms, warm ${r.warmMs.toFixed(1).padStart(5)} ms`);
  }
  writeFileSync(join(OUT, "results.json"), JSON.stringify({ hermetic: h, pagefind: p, hermeticRuns, pagefindRuns, perfRuns }, null, 1));

  const failures: string[] = [];
  if (h.success5 < p.success5) failures.push("success@5 below Pagefind");
  if (h.mrr < p.mrr) failures.push("MRR below Pagefind");
  if (maxWarm >= LATENCY_MS) failures.push(`query latency ${maxWarm.toFixed(1)} ms ≥ ${LATENCY_MS} ms`);
  if (maxBytes >= QUERY_BYTES) failures.push(`a query loaded ${maxBytes} bytes ≥ ${QUERY_BYTES}`);
  if (failures.length > 0) {
    console.error(`\nFAILED: ${failures.join("; ")}`);
    process.exit(1);
  }
  console.log("\nOK: hermetic meets every target");
}

await main();
