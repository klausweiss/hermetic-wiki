// <hermetic-unused-assets>: on the maintenance page of `hermetic serve`, lists the
// attachments nothing uses, lets the author choose some or all of them, and moves the
// chosen ones to the trash after a confirmation. The list is loaded again whenever the
// notes change.

import { PAGES_CHANGED, addPageAction, readEnv, whenParsed } from "@hermetic/core";

import { type Asset, MaintenanceApi, type TrashResult } from "./api";
import { fileUrl, formatSize, previewable } from "./format";
import { Selection } from "./selection";

const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
const total = (assets: readonly Asset[]) => assets.reduce((sum, a) => sum + a.size, 0);

export class HermeticUnusedAssets extends HTMLElement {
  private ready = false;
  private api!: MaintenanceApi;
  private root!: URL;
  private assets: Asset[] = [];
  private selection = new Selection();
  /** Incremented per load, so that an older response cannot replace a newer one. */
  private generation = 0;
  private summary = document.createElement("p");
  private toolbar = document.createElement("div");
  private selectAll = document.createElement("input");
  private trashButton = document.createElement("button");
  private result = document.createElement("div");
  private list = document.createElement("ul");
  private withdraw: (() => void) | null = null;

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  disconnectedCallback(): void {
    this.withdraw?.();
    this.withdraw = null;
  }

  private setup(): void {
    const env = readEnv();
    if (this.ready || !env.interactive) return;
    this.ready = true;
    this.root = env.root;
    this.api = new MaintenanceApi(env.root);
    document.querySelector<HTMLElement>(".maintenance-unavailable")?.setAttribute("hidden", "");

    this.className = "unused-assets";
    this.summary.className = "unused-summary";
    this.result.className = "unused-result";
    this.result.setAttribute("role", "status");
    this.list.className = "unused-list";
    this.toolbar.className = "unused-toolbar";
    const all = document.createElement("label");
    all.className = "unused-select-all";
    this.selectAll.type = "checkbox";
    this.selectAll.addEventListener("change", () => {
      this.selection.setAll(this.selectAll.checked);
      this.sync();
    });
    all.append(this.selectAll, "Select all");
    this.trashButton.type = "button";
    this.trashButton.className = "danger";
    this.trashButton.addEventListener("click", () => this.confirm());
    this.toolbar.append(all, this.trashButton);
    this.append(this.summary, this.result, this.toolbar, this.list);

    const withdraw = [
      addPageAction({
        key: "a",
        what: "Select all / none",
        detail: () => (this.selection.all === "all" ? "all → none" : "none → all"),
        available: () => this.assets.length > 0 && !this.toolbar.hidden,
        run: () => this.toggleAll(),
      }),
      addPageAction({
        key: "d",
        what: "Move the selected files to the trash",
        detail: () => plural(this.selection.size, "file"),
        available: () => this.selection.size > 0,
        run: () => this.confirm(),
      }),
    ];
    this.withdraw = () => withdraw.forEach((w) => w());
    document.addEventListener(PAGES_CHANGED, () => void this.load());
    void this.load();
  }

  private async load(): Promise<void> {
    const generation = ++this.generation;
    let assets: Asset[];
    try {
      assets = await this.api.list();
    } catch {
      if (generation !== this.generation) return;
      this.summary.textContent = "The list of unused assets could not be loaded. Is hermetic serve still running?";
      return;
    }
    if (generation !== this.generation) return;
    this.assets = assets;
    this.selection.replace(assets.map((a) => a.path));
    this.render();
  }

  private render(): void {
    const n = this.assets.length;
    this.summary.textContent =
      n === 0
        ? "There are no unused assets: every file in journals/, pages/, and assets/ is linked to or shown by a note."
        : `${plural(n, "file")} (${formatSize(total(this.assets))}) that no note links to or shows.`;
    this.toolbar.hidden = n === 0;
    this.list.hidden = n === 0;
    this.list.replaceChildren(...this.assets.map((a, i) => this.row(a, i)));
    this.sync();
  }

  private row(asset: Asset, index: number): HTMLLIElement {
    const li = document.createElement("li");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.setAttribute("aria-label", `Select ${asset.path}`);
    box.dataset.path = asset.path;
    // Shift is read from the click; `change` follows it with the new state.
    let range = false;
    box.addEventListener("click", (e) => {
      range = e.shiftKey;
    });
    box.addEventListener("change", () => {
      this.selection.set(index, box.checked, range);
      range = false;
      this.sync();
    });
    const preview = document.createElement("span");
    preview.className = "unused-preview";
    const url = fileUrl(this.root, asset.path);
    if (previewable(asset.path)) {
      const img = document.createElement("img");
      img.loading = "lazy";
      img.alt = "";
      img.src = url;
      preview.append(img);
    }
    const link = document.createElement("a");
    link.className = "unused-path";
    link.href = url;
    link.textContent = asset.path;
    const size = document.createElement("span");
    size.className = "unused-size";
    size.textContent = formatSize(asset.size);
    li.append(box, preview, link, size);
    return li;
  }

  /** Selects every entry, or none when all are selected (`.` then `a`). */
  private toggleAll(): void {
    this.selection.setAll(this.selection.all !== "all");
    this.sync();
  }

  /** Shows the selection: the checkboxes, "Select all", and the button. */
  private sync(): void {
    for (const box of this.list.querySelectorAll<HTMLInputElement>("input[type=checkbox]")) {
      box.checked = this.selection.has(box.dataset.path ?? "");
    }
    const state = this.selection.all;
    this.selectAll.checked = state === "all";
    this.selectAll.indeterminate = state === "some";
    const n = this.selection.size;
    this.trashButton.textContent = `Move to trash (${n})`;
    this.trashButton.disabled = n === 0;
  }

  private confirm(): void {
    const paths = this.selection.chosen;
    if (paths.length === 0 || document.querySelector("dialog[open]")) return;
    const chosen = this.assets.filter((a) => paths.includes(a.path));
    const dialog = document.createElement("dialog");
    dialog.className = "confirm-delete confirm-trash";
    const heading = document.createElement("h2");
    heading.textContent = `Move ${plural(paths.length, "file")} to the trash?`;
    const what = document.createElement("p");
    what.textContent = `${plural(paths.length, "file")} (${formatSize(total(chosen))}) will be moved to .trash/ in the notes folder; move them back to restore them.`;
    const files = document.createElement("ul");
    files.className = "confirm-files";
    for (const p of paths) {
      const li = document.createElement("li");
      const code = document.createElement("code");
      code.textContent = p;
      li.append(code);
      files.append(li);
    }
    const error = document.createElement("p");
    error.className = "error";
    error.hidden = true;
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.textContent = "Cancel";
    cancel.autofocus = true;
    const ok = document.createElement("button");
    ok.type = "button";
    ok.className = "danger";
    ok.textContent = "Move to trash";
    const buttons = document.createElement("div");
    buttons.className = "dialog-buttons";
    buttons.append(cancel, ok);
    dialog.append(heading, what, files, error, buttons);
    this.append(dialog);
    dialog.addEventListener("close", () => dialog.remove());
    cancel.addEventListener("click", () => dialog.close());
    ok.addEventListener("click", async () => {
      ok.disabled = true;
      try {
        const result = await this.api.trash(paths);
        dialog.close();
        this.report(result);
        await this.load();
      } catch {
        error.hidden = false;
        error.textContent = "The files could not be moved; nothing was changed.";
        ok.disabled = false;
      }
    });
    dialog.showModal();
  }

  private report(result: TrashResult): void {
    const lines: HTMLElement[] = [];
    const moved = document.createElement("p");
    if (result.trashed.length > 0) {
      moved.append(`Moved ${plural(result.trashed.length, "file")} to `);
      const code = document.createElement("code");
      code.textContent = `${result.trash}/`;
      moved.append(code, ".");
    } else {
      moved.textContent = "No file was moved.";
    }
    lines.push(moved);
    if (result.skipped.length > 0) {
      const head = document.createElement("p");
      head.textContent = `${plural(result.skipped.length, "file was", "files were")} not moved:`;
      const list = document.createElement("ul");
      for (const s of result.skipped) {
        const li = document.createElement("li");
        const code = document.createElement("code");
        code.textContent = s.path;
        li.append(code, ` (${s.reason})`);
        list.append(li);
      }
      lines.push(head, list);
    }
    this.result.replaceChildren(...lines);
  }
}
