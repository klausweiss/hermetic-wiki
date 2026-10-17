// Undo and redo for the page being edited, across blocks and outline operations. Entries
// are whole states of the page (taken before each change); a run of typing is one entry.

/** Where the cursor was: a block (by its position in the outline) and a selection. */
export interface Where {
  index: number;
  start: number;
  end: number;
}

export interface Entry<S> {
  state: S;
  where: Where | null;
}

/** The typing an input was: its kind and where the cursor is after it. */
export interface Typing {
  kind: "insert" | "delete";
  index: number;
  caret: number;
}

/** Typing joins the previous entry unless it pauses this long. */
export const TYPING_PAUSE_MS = 1500;
const LIMIT = 500;

export class History<S> {
  private undos: Entry<S>[] = [];
  private redos: Entry<S>[] = [];
  private typing: (Typing & { at: number }) | null = null;

  /**
   * A change was made; `before` is the state it was made from. Typing of the same kind
   * that continues at the cursor the last typing left, without a pause, joins its entry.
   */
  record(before: () => Entry<S>, typing: Typing | null, now: number): void {
    const last = this.typing;
    let joins = false;
    if (typing && last) {
      const w = before().where;
      joins =
        w !== null &&
        last.kind === typing.kind &&
        w.index === last.index &&
        w.start === w.end &&
        w.start === last.caret &&
        now - last.at < TYPING_PAUSE_MS;
    }
    if (!joins) {
      this.undos.push(before());
      if (this.undos.length > LIMIT) this.undos.shift();
    }
    this.redos = [];
    this.typing = typing ? { ...typing, at: now } : null;
  }

  /** Ends the current run of typing: what follows is a new entry. */
  close(): void {
    this.typing = null;
  }

  /** The state to go back to, given the current one (kept for redo). */
  undo(current: () => Entry<S>): Entry<S> | null {
    return this.step(this.undos, this.redos, current);
  }

  redo(current: () => Entry<S>): Entry<S> | null {
    return this.step(this.redos, this.undos, current);
  }

  clear(): void {
    this.undos = [];
    this.redos = [];
    this.typing = null;
  }

  private step(from: Entry<S>[], to: Entry<S>[], current: () => Entry<S>): Entry<S> | null {
    const entry = from.pop();
    if (!entry) return null;
    to.push(current());
    this.typing = null;
    return entry;
  }
}

/** Which history key `e` is: Ctrl/Cmd+Z undoes, Ctrl/Cmd+Shift+Z and Ctrl+Y redo. */
export function historyKey(e: KeyboardEvent): "undo" | "redo" | null {
  if (e.altKey || !(e.ctrlKey || e.metaKey)) return null;
  const key = e.key.toLowerCase();
  if (key === "z") return e.shiftKey ? "redo" : "undo";
  if (key === "y" && e.ctrlKey && !e.metaKey && !e.shiftKey) return "redo";
  return null;
}
