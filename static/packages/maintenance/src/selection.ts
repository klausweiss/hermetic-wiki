// Which unused assets are selected, by path, kept apart from the DOM so it can be tested.

export type AllState = "none" | "some" | "all";

export class Selection {
  private readonly selected = new Set<string>();
  /** Index of the entry clicked last, where a Shift+click range starts. */
  private anchor: number | null = null;

  constructor(private paths: readonly string[] = []) {}

  has(path: string): boolean {
    return this.selected.has(path);
  }

  /** The selected paths, in list order. */
  get chosen(): string[] {
    return this.paths.filter((p) => this.selected.has(p));
  }

  get size(): number {
    return this.selected.size;
  }

  get all(): AllState {
    if (this.selected.size === 0) return "none";
    return this.selected.size === this.paths.length ? "all" : "some";
  }

  /**
   * Sets entry `index` to `on`. With `range`, every entry between it and the entry
   * clicked last is set the same way. The entry becomes the start of the next range.
   */
  set(index: number, on: boolean, range = false): void {
    const from = range && this.anchor !== null ? Math.min(this.anchor, index) : index;
    const to = range && this.anchor !== null ? Math.max(this.anchor, index) : index;
    for (let i = from; i <= to; i++) {
      const path = this.paths[i];
      if (path === undefined) continue;
      if (on) this.selected.add(path);
      else this.selected.delete(path);
    }
    this.anchor = index;
  }

  /** Selects every entry, or none. */
  setAll(on: boolean): void {
    this.selected.clear();
    if (on) for (const p of this.paths) this.selected.add(p);
    this.anchor = null;
  }

  /** A new list: entries still listed stay selected, the others are forgotten. */
  replace(paths: readonly string[]): void {
    const listed = new Set(paths);
    for (const p of [...this.selected]) if (!listed.has(p)) this.selected.delete(p);
    const anchored = this.anchor === null ? undefined : this.paths[this.anchor];
    this.paths = paths;
    const at = anchored === undefined ? -1 : paths.indexOf(anchored);
    this.anchor = at < 0 ? null : at;
  }
}
