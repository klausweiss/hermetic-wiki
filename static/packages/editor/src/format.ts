// Toggling inline markdown markers (`**`, `*`, `~~`, `` ` ``) around a selection or at the
// cursor. Pure: computes the replacement and the selection afterwards.

/** Replace `start..end` of the text with `text`, then select `selStart..selEnd`. */
export interface Edit {
  start: number;
  end: number;
  text: string;
  selStart: number;
  selEnd: number;
}

export const MARKERS = { bold: "**", italic: "*", strike: "~~", code: "`" } as const;

/** Length of the run of `ch` ending at `at` (going left) or starting at `at` (going right). */
function run(value: string, at: number, ch: string, dir: -1 | 1): number {
  let n = 0;
  for (let i = dir < 0 ? at - 1 : at; i >= 0 && i < value.length && value[i] === ch; i += dir) n++;
  return n;
}

/**
 * Whether runs of the marker's character on both sides contain the marker. For `*`, bold
 * needs runs of at least two and italic needs odd runs (`*` or `***`), so that italic
 * is never taken for half of a bold marker.
 */
function present(marker: string, before: number, after: number): boolean {
  const n = marker.length;
  if (before < n || after < n) return false;
  if (marker === "*") return before % 2 === 1 && after % 2 === 1;
  return true;
}

export function toggle(value: string, start: number, end: number, marker: string): Edit {
  const n = marker.length;
  const ch = marker[0]!;
  if (start === end) {
    // An empty pair around the cursor is removed; otherwise one is inserted.
    if (present(marker, run(value, start, ch, -1), run(value, start, ch, 1))) {
      return { start: start - n, end: start + n, text: "", selStart: start - n, selEnd: start - n };
    }
    return { start, end, text: marker + marker, selStart: start + n, selEnd: start + n };
  }
  // Leave surrounding whitespace of a selection outside the markers.
  const selected = value.slice(start, end);
  const s = start + (selected.length - selected.trimStart().length);
  const e = end - (selected.length - selected.trimEnd().length);
  if (s >= e) return { start, end, text: selected, selStart: start, selEnd: end };
  const inner = value.slice(s, e);
  if (present(marker, run(value, s, ch, -1), run(value, e, ch, 1))) {
    return { start: s - n, end: e + n, text: inner, selStart: s - n, selEnd: e - n };
  }
  if (inner.length >= 2 * n && present(marker, run(inner, 0, ch, 1), run(inner, inner.length, ch, -1))) {
    return { start: s, end: e, text: inner.slice(n, -n), selStart: s, selEnd: e - 2 * n };
  }
  return { start: s, end: e, text: marker + inner + marker, selStart: s + n, selEnd: e + n };
}

/** The text after applying an edit. */
export function applied(value: string, edit: Edit): string {
  return value.slice(0, edit.start) + edit.text + value.slice(edit.end);
}
