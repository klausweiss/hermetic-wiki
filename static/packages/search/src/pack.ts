// Numbers packed by `src/search/index.rs` (`pack`): five bits per character, low bits
// first, the sixth bit set on every character but the last.

const DIGITS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";
const VALUE = new Map([...DIGITS].map((c, i) => [c, i]));

export function unpack(packed: string): number[] {
  const out: number[] = [];
  let n = 0;
  let scale = 1;
  for (const c of packed) {
    const v = VALUE.get(c) ?? 0;
    n += (v & 31) * scale;
    if (v & 32) {
      scale *= 32;
    } else {
      out.push(n);
      n = 0;
      scale = 1;
    }
  }
  return out;
}
