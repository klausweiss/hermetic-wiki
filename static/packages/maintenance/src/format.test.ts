import { describe, expect, test } from "bun:test";

import { fileUrl, formatSize, previewable } from "./format";

describe("formatSize", () => {
  test("bytes, then 1024-based units with one decimal", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(1023)).toBe("1023 B");
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(2048 + 512)).toBe("2.5 KB");
    expect(formatSize(1.5 * 1024 * 1024)).toBe("1.5 MB");
    expect(formatSize(3 * 1024 ** 3)).toBe("3.0 GB");
  });
});

describe("previewable", () => {
  test("images the browser shows, by extension in any case", () => {
    for (const p of ["a.png", "b.JPG", "c.jpeg", "d.gif", "e.webp", "pages/f.svg"]) expect(previewable(p)).toBe(true);
    for (const p of ["a.pdf", "png", "dir.png/file", "a.png.txt"]) expect(previewable(p)).toBe(false);
  });
});

describe("fileUrl", () => {
  test("each segment percent-encoded", () => {
    const root = new URL("http://127.0.0.1:8000/");
    expect(fileUrl(root, "assets/my shot#1.png")).toBe("http://127.0.0.1:8000/assets/my%20shot%231.png");
  });
});
