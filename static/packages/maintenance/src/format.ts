// How unused assets are shown: sizes, previews, and links to the files.

/** A size in bytes for people: `512 B`, `2.0 KB`, `1.5 MB` (1024-based, one decimal). */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(1)} ${units[unit]}`;
}

const PREVIEWABLE = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg"]);

/** Whether the file at `path` is an image the browser can show as a preview. */
export function previewable(path: string): boolean {
  const dot = path.lastIndexOf(".");
  return dot > path.lastIndexOf("/") && PREVIEWABLE.has(path.slice(dot + 1).toLowerCase());
}

/** The URL of the file at `path` (relative to the notes, `/`-separated) on the site. */
export function fileUrl(root: URL, path: string): string {
  return new URL(path.split("/").map(encodeURIComponent).join("/"), root).href;
}
