// Syntax colouring for the block being edited: a mirror of the textarea's text, drawn
// behind the (transparent) textarea. Styles in the mirror must keep every glyph's width,
// so they change colour, decoration or stroke, never weight, style, size or spacing.

/** A run of text and the class it is drawn with (none for plain text). */
export interface Segment {
  text: string;
  cls: string | null;
}

const TASK = /^(TODO|DOING|DONE)(?=\s|$)/;
const PROPERTY = /^([ \t]*)([A-Za-z0-9_-]+::)(?=\s|$)/;
const LINE_MARK = /^([ \t]*)(#{1,6}(?=\s)|[-*+](?=\s)|\d+[.)](?=\s)|>)/;
const FENCE = /^[ \t]*(```|~~~)/;
const TRAILING = /[.,;:!?)]+$/;

type Rule = (text: string, at: number) => Segment[] | null;

/** `open` + inner + `close`, the markers drawn as `mark` and the inner text as `cls`. */
function wrapped(open: string, inner: string, close: string, cls: string): Segment[] {
  return [
    { text: open, cls: "hl-mark" },
    { text: inner, cls },
    { text: close, cls: "hl-mark" },
  ];
}

/**
 * Emphasis: `open` + inner + `close`, the inner text highlighted on its own (so emphasis
 * nests) and each of its runs also drawn as `cls`.
 */
function emphasis(open: string, inner: string, close: string, cls: string): Segment[] {
  const runs: Segment[] = [];
  inline(inner, runs);
  return [
    { text: open, cls: "hl-mark" },
    ...runs.map((r) => ({ text: r.text, cls: r.cls === "hl-mark" ? r.cls : r.cls ? `${r.cls} ${cls}` : cls })),
    { text: close, cls: "hl-mark" },
  ];
}

function sticky(re: RegExp, build: (m: RegExpExecArray) => Segment[]): Rule {
  const y = new RegExp(re.source, "y");
  return (text, at) => {
    y.lastIndex = at;
    const m = y.exec(text);
    return m ? build(m) : null;
  };
}

/** Text that may end with punctuation that is not part of it (tags and URLs). */
function trimmed(word: string, cls: string): Segment[] {
  const tail = TRAILING.exec(word)?.[0] ?? "";
  const body = word.slice(0, word.length - tail.length);
  return tail ? [{ text: body, cls }, { text: tail, cls: null }] : [{ text: body, cls }];
}

const wordBefore = (text: string, at: number) => at > 0 && /\w/.test(text[at - 1]!);
const spaceBefore = (text: string, at: number) => at === 0 || /\s/.test(text[at - 1]!);

const RULES: Rule[] = [
  sticky(/\\[\s\S]/, (m) => [{ text: m[0], cls: "hl-mark" }]),
  sticky(/(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/, (m) => [{ text: m[0], cls: "hl-code" }]),
  sticky(/(#?\[\[)([^[\]\n]*)(\]\])/, (m) => wrapped(m[1]!, m[2]!, m[3]!, "hl-ref")),
  sticky(/(\(\()([^()\s]+)(\)\))/, (m) => wrapped(m[1]!, m[2]!, m[3]!, "hl-ref")),
  sticky(/!?\[[^\]\n]*\]\([^)\s]*\)/, (m) => [{ text: m[0], cls: "hl-link" }]),
  (text, at) => {
    if (!spaceBefore(text, at)) return null;
    const m = /^#[^\s#]+/.exec(text.slice(at));
    return m ? trimmed(m[0], "hl-ref") : null;
  },
  (text, at) => {
    if (wordBefore(text, at)) return null;
    const m = /^https?:\/\/[^\s<]+/.exec(text.slice(at));
    return m ? trimmed(m[0], "hl-link") : null;
  },
  // `***both***`, then bold and italic whose text may hold the other (`***a** b*`).
  sticky(/(\*\*\*|___)(?!\s)([^\n]*?)(?<!\s)\1/, (m) => emphasis(m[1]!, m[2]!, m[1]!, "hl-strong hl-em")),
  sticky(/\*\*(?!\s)((?:\*[^*\n]+?\*|[^*\n])+?)(?<!\s)\*\*/, (m) => emphasis("**", m[1]!, "**", "hl-strong")),
  sticky(/\*(?!\s)((?:\*\*[^*\n]+?\*\*|[^*\n])+?)(?<!\s)\*(?!\*)/, (m) => emphasis("*", m[1]!, "*", "hl-em")),
  sticky(/__(?![\s_])([^\n]*?)(?<!\s)__/, (m) => emphasis("__", m[1]!, "__", "hl-strong")),
  sticky(/~~(?!\s)([^\n]*?)(?<!\s)~~/, (m) => emphasis("~~", m[1]!, "~~", "hl-strike")),
  (text, at) => {
    if (wordBefore(text, at)) return null;
    const m = /^_([^_\s]|[^_\s][^_\n]*?[^_\s])_(?!\w)/.exec(text.slice(at));
    return m ? emphasis("_", m[1]!, "_", "hl-em") : null;
  },
];

/** Inline syntax of one line's text. */
function inline(text: string, out: Segment[]): void {
  let plain = "";
  for (let i = 0; i < text.length; ) {
    let found: Segment[] | null = null;
    for (const rule of RULES) if ((found = rule(text, i))) break;
    if (!found) {
      plain += text[i++];
      continue;
    }
    for (const s of found) {
      i += s.text.length;
      if (!s.cls) {
        plain += s.text;
      } else if (s.text) {
        if (plain) out.push({ text: plain, cls: null });
        plain = "";
        out.push(s);
      }
    }
  }
  if (plain) out.push({ text: plain, cls: null });
}

/** The block's text cut into coloured runs; their texts join back to `body`. */
export function highlight(body: string): Segment[] {
  const out: Segment[] = [];
  let fence: string | null = null;
  body.split("\n").forEach((line, n) => {
    if (n > 0) out.push({ text: "\n", cls: null });
    const f = FENCE.exec(line);
    if (fence || f) {
      out.push({ text: line, cls: "hl-code" });
      if (f && (!fence || f[1] === fence)) fence = fence ? null : f[1]!;
      return;
    }
    let rest = line;
    const prop = PROPERTY.exec(rest);
    const task = n === 0 ? TASK.exec(rest) : null;
    const mark = LINE_MARK.exec(rest);
    if (prop) {
      out.push({ text: prop[1]! + prop[2]!, cls: "hl-prop" });
      rest = rest.slice(prop[0].length);
    } else if (task) {
      out.push({ text: task[1]!, cls: `hl-${task[1]!.toLowerCase()}` });
      rest = rest.slice(task[0].length);
    } else if (mark) {
      out.push({ text: mark[0], cls: "hl-mark" });
      rest = rest.slice(mark[0].length);
    }
    inline(rest, out);
  });
  return out;
}

/**
 * Wraps `area` with a mirror drawn behind it, kept in step with its text whether it
 * changes by typing or by setting `value`. Returns the wrapper to place in the page.
 */
export function highlighted(area: HTMLTextAreaElement): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = "block-editor-wrap";
  const mirror = document.createElement("div");
  mirror.className = "block-editor-mirror";
  mirror.setAttribute("aria-hidden", "true");
  wrap.append(mirror, area);

  const sync = () => {
    mirror.replaceChildren(
      ...highlight(area.value).map((s) => {
        if (!s.cls) return s.text;
        const span = document.createElement("span");
        span.className = s.cls;
        span.textContent = s.text;
        return span;
      }),
    );
  };
  const value = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!;
  Object.defineProperty(area, "value", {
    configurable: true,
    get(this: HTMLTextAreaElement) {
      return value.get!.call(this);
    },
    set(this: HTMLTextAreaElement, v: string) {
      value.set!.call(this, v);
      sync();
    },
  });
  area.addEventListener("input", sync);
  return wrap;
}

/** Where the character at `offset` of the mirror's text is drawn. */
function charRect(mirror: Element, offset: number): DOMRect | null {
  const walker = document.createTreeWalker(mirror, NodeFilter.SHOW_TEXT);
  for (let text = walker.nextNode(); text; text = walker.nextNode()) {
    const length = text.textContent!.length;
    if (offset < length) {
      const range = document.createRange();
      range.setStart(text, offset);
      range.setEnd(text, offset + 1);
      return range.getBoundingClientRect();
    }
    offset -= length;
  }
  return null;
}

/**
 * Whether the characters at offsets `a` and `b` of a highlighted area's text are drawn
 * on the same line, counting lines the text wraps onto. `wrap` is what `highlighted` returned.
 */
export function sameLine(wrap: HTMLElement, a: number, b: number): boolean {
  if (a === b) return true;
  const mirror = wrap.querySelector(".block-editor-mirror");
  const ra = mirror && charRect(mirror, a);
  const rb = mirror && charRect(mirror, b);
  if (!ra || !rb) return true;
  return ra.top < rb.bottom && rb.top < ra.bottom;
}
