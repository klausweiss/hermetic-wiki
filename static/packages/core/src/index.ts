// @hermetic/core: logic shared by hermetic's components. No UI, no custom elements.

export { EDITING_ATTR, EDIT_PROPERTIES, OPEN_TODAY, PAGES_CHANGED, SEARCH_FOCUS, SEARCH_TOGGLE, SIDEBAR_CLOSE_TOP, SIDEBAR_TOGGLE, THEME_SWITCH, shownTheme, whenParsed } from "./dom";
export { type Env, readEnv } from "./env";
export { type FetchedPage, TARGET_ATTR, fetchPage } from "./fetchpage";
export { type Anchor, type Link, type PageLink, absolutize, anchorHref, classify, isSafePagePath } from "./links";
export {
  PANE_PARAM,
  type Pane,
  SIDEBAR_PARAM,
  carrySidebar,
  readHidden,
  readPanes,
  withPanes,
  withSidebar,
  writeHidden,
  writePanes,
} from "./state";
export { type IconName, icon } from "./icons";
