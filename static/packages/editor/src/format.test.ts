import { describe, expect, test } from "bun:test";

import { MARKERS, applied, toggle } from "./format";

/** Applies a toggle to text with `[` `]` marking the selection (or `|` the cursor). */
function press(marked: string, marker: string): string {
  const cursor = marked.indexOf("|");
  const start = cursor >= 0 ? cursor : marked.indexOf("[");
  const value = marked.replace(/[[\]|]/g, "");
  const end = cursor >= 0 ? cursor : marked.indexOf("]") - 1;
  const edit = toggle(value, start, end, marker);
  const out = applied(value, edit);
  return edit.selStart === edit.selEnd
    ? `${out.slice(0, edit.selStart)}|${out.slice(edit.selStart)}`
    : `${out.slice(0, edit.selStart)}[${out.slice(edit.selStart, edit.selEnd)}]${out.slice(edit.selEnd)}`;
}

describe("formatting keys", () => {
  test("Bold a selection", () => {
    expect(press("hello [world]", MARKERS.bold)).toBe("hello **[world]**");
  });

  test("Toggle off: markers just outside or inside the selection", () => {
    expect(press("hello **[world]**", MARKERS.bold)).toBe("hello [world]");
    expect(press("hello [**world**]", MARKERS.bold)).toBe("hello [world]");
  });

  test("Italic inside bold, and back", () => {
    expect(press("hello **[world]**", MARKERS.italic)).toBe("hello ***[world]***");
    expect(press("hello ***[world]***", MARKERS.italic)).toBe("hello **[world]**");
    expect(press("hello ***[world]***", MARKERS.bold)).toBe("hello *[world]*");
    expect(press("hello *[world]*", MARKERS.bold)).toBe("hello ***[world]***");
    expect(press("hello *[world]*", MARKERS.italic)).toBe("hello [world]");
  });

  test("No selection: insert a pair, or remove an empty one", () => {
    expect(press("note |", MARKERS.bold)).toBe("note **|**");
    expect(press("note **|**", MARKERS.bold)).toBe("note |");
    expect(press("a `|` b", MARKERS.code)).toBe("a | b");
    expect(press("x|", MARKERS.strike)).toBe("x~~|~~");
  });

  test("Strikethrough and code", () => {
    expect(press("[gone]", MARKERS.strike)).toBe("~~[gone]~~");
    expect(press("~~[gone]~~", MARKERS.strike)).toBe("[gone]");
    expect(press("run [ls -la] now", MARKERS.code)).toBe("run `[ls -la]` now");
    expect(press("run `[ls -la]` now", MARKERS.code)).toBe("run [ls -la] now");
  });

  test("Whitespace around a selection stays outside the markers", () => {
    expect(press("say[ hello ]there", MARKERS.bold)).toBe("say **[hello]** there");
    expect(press("a[   ]b", MARKERS.bold)).toBe("a[   ]b");
  });

  test("Multi-line selections wrap as a whole", () => {
    expect(press("[one\ntwo]", MARKERS.italic)).toBe("*[one\ntwo]*");
  });
});
