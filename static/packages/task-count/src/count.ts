// The open-task count written by `hermetic build` and `hermetic serve` (see `src/site/mod.rs`).

/** The count's file, relative to the site root: `{"count":N}`. */
export const COUNT_PATH = "_hermetic/open-tasks.json";

/** The count in the file's text, or null when it is not a valid count file. */
export function parseCount(text: string): number | null {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return null;
  }
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const { count } = value as Record<string, unknown>;
  return typeof count === "number" && Number.isSafeInteger(count) && count >= 0 ? count : null;
}

/** Whether a rebuild's change list includes the count file. */
export function mentionsCount(changes: { changed: string[]; removed: string[] }): boolean {
  return changes.changed.includes(COUNT_PATH) || changes.removed.includes(COUNT_PATH);
}
