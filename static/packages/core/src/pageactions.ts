// Actions a page offers in the `.` key menu ("This page"), added by the elements on it.
// Kept on the global object, so elements in separately loaded bundles share one list.

/** An action of the `.` menu: pressing `.` and then `key` runs it. */
export interface PageAction {
  key: string;
  what: string;
  /** Shown after the description, e.g. what the action changes from and to. */
  detail?: () => string;
  available?: () => boolean;
  run: () => void;
}

const KEY = Symbol.for("hermetic.page-actions");

function list(): PageAction[] {
  const g = globalThis as { [KEY]?: PageAction[] };
  return (g[KEY] ??= []);
}

/** Offers `action` in the `.` menu; the returned function withdraws it. */
export function addPageAction(action: PageAction): () => void {
  list().push(action);
  return () => {
    const actions = list();
    const i = actions.indexOf(action);
    if (i >= 0) actions.splice(i, 1);
  };
}

/** The actions the page offers now, in the order they were added. */
export function pageActions(): readonly PageAction[] {
  return [...list()];
}
