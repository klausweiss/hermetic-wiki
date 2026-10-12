// A page's outline as the editor holds it between saves, built from the server's source
// payload. Nodes remember the exact lines they were parsed from, so a save rewrites
// only what changed (see serialize.ts).

/** One block as `GET /_hermetic/source` describes it. */
export interface PayloadBlock {
  /** 0-based line range owned by the block (the ranges partition the file's lines). */
  lines: [number, number];
  /** Line of the block's first content line. */
  first: number;
  depth: number;
  bullet: boolean;
  /** Its anchor on the rendered page: `b<n>` or `id-<id>`. */
  anchor: string;
  /** Its text as the editor shows it: de-indented, without the bullet marker. */
  body: string;
  column: number;
}

/** The source payload of one page. */
export interface Payload {
  exists: boolean;
  version: string;
  /** The file's text with LF line endings. */
  text: string;
  /** Indentation unit for one level of nesting. */
  indent: string;
  blocks: PayloadBlock[];
}

/** Where a node came from, to write it back unchanged when nothing about it changed. */
export interface Origin {
  /** Its content lines exactly as in the file. */
  lines: string[];
  /** Blank lines that followed its content (they move with it). */
  trail: string[];
  depth: number;
  body: string;
}

export interface Node {
  body: string;
  /** False for text outside bullets, which always stays top-level. */
  bullet: boolean;
  children: Node[];
  origin?: Origin;
  anchor?: string;
}

export interface Doc {
  roots: Node[];
  indent: string;
  /** Lines before the first block (blank lines at the top of the file). */
  head: string[];
  /** Whether the file ends with a line ending. */
  finalNewline: boolean;
}

export function fromPayload(p: Payload): Doc {
  const finalNewline = p.text === "" || p.text.endsWith("\n");
  const lines = (finalNewline ? p.text.slice(0, -1) : p.text).split("\n");
  if (p.text === "") lines.length = 0;
  const doc: Doc = { roots: [], indent: p.indent, head: [], finalNewline };
  const stack: Node[] = [];
  p.blocks.forEach((b, i) => {
    if (i === 0) doc.head = lines.slice(0, b.first);
    const count = b.body.split("\n").length;
    const end = Math.min(b.lines[1], lines.length);
    const node: Node = {
      body: b.body,
      bullet: b.bullet,
      children: [],
      anchor: b.anchor,
      origin: {
        lines: lines.slice(b.first, b.first + count),
        trail: lines.slice(b.first + count, end),
        depth: b.depth,
        body: b.body,
      },
    };
    stack.length = b.depth;
    const parent = stack[b.depth - 1];
    (parent ? parent.children : doc.roots).push(node);
    stack.push(node);
  });
  return doc;
}

/** A new, empty bullet block. */
export function newNode(body = ""): Node {
  return { body, bullet: true, children: [] };
}

/** Every node in document order, with its depth. */
export function flatten(doc: Doc): { node: Node; depth: number }[] {
  const out: { node: Node; depth: number }[] = [];
  const walk = (nodes: Node[], depth: number) => {
    for (const node of nodes) {
      out.push({ node, depth });
      walk(node.children, depth + 1);
    }
  };
  walk(doc.roots, 0);
  return out;
}

export interface Place {
  parent: Node | null;
  siblings: Node[];
  index: number;
}

/** Where a node sits in the tree. */
export function locate(doc: Doc, target: Node): Place | null {
  const find = (parent: Node | null, siblings: Node[]): Place | null => {
    for (let index = 0; index < siblings.length; index++) {
      const node = siblings[index]!;
      if (node === target) return { parent, siblings, index };
      const inner = find(node, node.children);
      if (inner) return inner;
    }
    return null;
  };
  return find(null, doc.roots);
}
