// hermetic's browser script: applies the reader's theme and the sidebar layout before the
// page is painted, and
// defines every element. The generated HTML already contains the elements; without
// this script they are inert and every page still works.

import { define as defineLiveReload } from "@hermetic/live-reload";
import { define as defineShortcuts } from "@hermetic/shortcuts";
import { applyInitialLayout, define as defineSidebar } from "@hermetic/sidebar";
import { applyStoredTheme, define as defineThemeToggle } from "@hermetic/theme-toggle";
import { define as defineTimeline } from "@hermetic/timeline";

// Both run before the page is first painted: no theme flash, no layout shift.
applyStoredTheme();
applyInitialLayout();
defineThemeToggle();
defineSidebar();
defineTimeline();
defineShortcuts();
defineLiveReload();
