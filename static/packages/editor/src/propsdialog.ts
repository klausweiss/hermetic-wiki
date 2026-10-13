// The page properties dialog (the `p` key, or the page menu): one row per property,
// with a key field, a value field and a delete button, edited from the keyboard. Up and
// Down move between rows, Tab through a row's key, value and delete button, Ctrl+Enter
// (or Down on the last row) adds a row. Enter (or "Save") hands the rows back; Escape
// (or "Cancel") closes without changes.

import { icon } from "@hermetic/core";

import { type Property, cleanKey } from "./props";

const COLUMNS = ["key", "value", "delete"] as const;
type Column = (typeof COLUMNS)[number];

const APPLE = /Mac|iPhone|iPad/.test(navigator.platform);
const MOD_ENTER = APPLE ? "⌘ Enter" : "Ctrl + Enter";

export function editProperties(host: HTMLElement, initial: readonly Property[], save: (rows: Property[]) => void): void {
  const dialog = document.createElement("dialog");
  dialog.className = "page-props";
  const heading = document.createElement("h2");
  heading.textContent = "Page properties";
  const rows = document.createElement("div");
  rows.className = "prop-rows";
  const error = document.createElement("p");
  error.className = "error";
  error.hidden = true;
  const hint = document.createElement("p");
  hint.className = "prop-hint";
  hint.textContent = `↑ ↓ move · Tab key / value / delete · ${MOD_ENTER} new property · Enter save · Esc cancel`;
  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.textContent = "Cancel";
  const finish = document.createElement("button");
  finish.type = "button";
  finish.textContent = "Save";
  const buttons = document.createElement("div");
  buttons.className = "dialog-buttons";
  buttons.append(cancel, finish);
  dialog.append(heading, rows, error, hint, buttons);

  const all = () => [...rows.querySelectorAll<HTMLElement>(":scope > .prop-row")];
  const field = (row: Element, column: Column) => row.querySelector<HTMLElement>(`.prop-${column}`)!;
  const input = (row: Element, column: "key" | "value") => field(row, column) as HTMLInputElement;
  const isEmpty = (row: Element) => input(row, "key").value === "" && input(row, "value").value.trim() === "";
  const focus = (row: Element, column: Column, at: "start" | "end" = "end") => {
    const el = field(row, column);
    el.focus();
    if (el instanceof HTMLInputElement) {
      const pos = at === "start" ? 0 : el.value.length;
      el.setSelectionRange(pos, pos);
    }
  };
  const disarm = (button: HTMLElement) => {
    delete button.dataset.armed;
    button.title = "Delete property (press twice)";
    button.querySelector(".prop-confirm")?.remove();
  };
  /** Removes `row`; focus goes to the delete button of the row below (or above). */
  const removeRow = (row: HTMLElement) => {
    const list = all();
    const index = list.indexOf(row);
    row.remove();
    const next = list[index + 1] ?? list[index - 1];
    if (next) focus(next, "delete");
    else focus(addRow({ key: "", value: "" }), "key");
  };
  const addRow = (p: Property, after: Element | null = null): HTMLElement => {
    const row = document.createElement("div");
    row.className = "prop-row";
    const key = document.createElement("input");
    key.className = "prop-key";
    key.value = p.key;
    key.placeholder = "key";
    key.setAttribute("aria-label", "Property key");
    key.spellcheck = false;
    key.autocomplete = "off";
    const sep = document.createElement("span");
    sep.className = "prop-sep";
    sep.textContent = "::";
    const value = document.createElement("input");
    value.className = "prop-value";
    value.value = p.value;
    value.placeholder = "value";
    value.setAttribute("aria-label", "Property value");
    value.autocomplete = "off";
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "prop-delete";
    remove.setAttribute("aria-label", "Delete property");
    remove.append(icon("trash-2"));
    disarm(remove);
    // Deleting takes two presses (or clicks): the first one arms the button.
    remove.addEventListener("click", () => {
      if ("armed" in remove.dataset) removeRow(row);
      else {
        remove.dataset.armed = "";
        remove.title = "Press again to delete";
        const confirm = document.createElement("span");
        confirm.className = "prop-confirm";
        confirm.textContent = "Delete?";
        remove.append(confirm);
      }
    });
    remove.addEventListener("blur", () => disarm(remove));
    row.append(key, sep, value, remove);
    if (after) after.after(row);
    else rows.append(row);
    return row;
  };

  for (const p of initial) addRow(p);
  if (initial.length === 0) addRow({ key: "", value: "" });

  // Keys may hold only letters, digits, `-` and `_` (what makes a property line).
  rows.addEventListener("input", (e) => {
    const el = e.target as HTMLInputElement;
    if (!el.classList.contains("prop-key")) return;
    el.removeAttribute("aria-invalid");
    const clean = cleanKey(el.value);
    if (clean !== el.value) {
      const pos = cleanKey(el.value.slice(0, el.selectionStart ?? el.value.length)).length;
      el.value = clean;
      el.setSelectionRange(pos, pos);
    }
  });

  rows.addEventListener("keydown", (e) => {
    const el = e.target as HTMLElement;
    const row = el.closest<HTMLElement>(".prop-row");
    if (!row || e.isComposing) return;
    const column = COLUMNS.find((c) => el.classList.contains(`prop-${c}`))!;
    const text = el instanceof HTMLInputElement ? el : null;
    const mod = e.ctrlKey || e.metaKey;
    const plain = !mod && !e.altKey && !e.shiftKey;
    const list = all();
    const index = list.indexOf(row);
    if (e.key === "Tab" && !mod && !e.altKey) {
      e.preventDefault();
      const by = e.shiftKey ? -1 : 1;
      focus(row, COLUMNS[(COLUMNS.indexOf(column) + by + COLUMNS.length) % COLUMNS.length]!);
    } else if ((e.key === "ArrowUp" || e.key === "ArrowDown") && plain) {
      e.preventDefault();
      const next = list[index + (e.key === "ArrowUp" ? -1 : 1)];
      if (next) focus(next, column);
      else if (e.key === "ArrowDown" && !isEmpty(row)) focus(addRow({ key: "", value: "" }, row), "key");
    } else if (e.key === "Enter" && mod && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      focus(addRow({ key: "", value: "" }, row), "key");
    } else if (e.key === "Enter" && plain && column !== "delete") {
      e.preventDefault();
      close(true);
    } else if (e.key === "Backspace" && plain && text && text.selectionStart === 0 && text.selectionEnd === 0) {
      // At the start of a value: to its key. In an empty row: remove it.
      if (column === "value" && text.value === "") {
        e.preventDefault();
        focus(row, "key");
      } else if (column === "key" && isEmpty(row) && list.length > 1) {
        e.preventDefault();
        row.remove();
        const prev = list[index - 1];
        if (prev) focus(prev, "value");
        else focus(list[index + 1]!, "key", "start");
      }
    } else if (column === "key" && (e.key === ":" || e.key === " ") && !mod && !e.altKey) {
      // Typing `key:: value` (or `key value`) flows into the value field.
      e.preventDefault();
      focus(row, "value");
    }
  });

  let saving = false;
  /** Closes the dialog: saving the rows, unless a row has a value but no key. */
  const close = (save: boolean) => {
    const nameless = all().find((r) => input(r, "key").value === "" && input(r, "value").value.trim() !== "");
    if (save && nameless) {
      input(nameless, "key").setAttribute("aria-invalid", "true");
      error.hidden = false;
      error.textContent = "Give each property a key, or clear its value.";
      focus(nameless, "key");
      return;
    }
    saving = save;
    dialog.close();
  };
  cancel.addEventListener("click", () => close(false));
  finish.addEventListener("click", () => close(true));
  dialog.addEventListener("close", () => {
    const result = all().map((r) => ({ key: input(r, "key").value, value: input(r, "value").value }));
    dialog.remove();
    if (saving) save(result);
  });

  host.append(dialog);
  dialog.showModal();
  const first = all()[0]!;
  focus(first, input(first, "key").value === "" ? "key" : "value");
}
