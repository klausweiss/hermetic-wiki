// <hermetic-theme-toggle>: switches between system, light, and dark theme, remembered
// in the browser. `applyStoredTheme()` must run before the page is painted. The button
// is an icon in the top-right corner, next to the sidebar's control (see style.css).

import { type IconName, icon, whenParsed } from "@hermetic/core";

type Theme = "system" | "light" | "dark";

const KEY = "hermetic-theme";
const ORDER: readonly Theme[] = ["system", "light", "dark"];
const ICONS: Record<Theme, IconName> = { system: "sun-moon", light: "sun", dark: "moon" };

function load(): Theme {
  try {
    const value = localStorage.getItem(KEY);
    return value === "light" || value === "dark" ? value : "system";
  } catch {
    return "system";
  }
}

function save(theme: Theme): void {
  try {
    if (theme === "system") {
      localStorage.removeItem(KEY);
    } else {
      localStorage.setItem(KEY, theme);
    }
  } catch {
    // Storage may be unavailable (private mode, file://); the choice then lasts for this page.
  }
}

function apply(theme: Theme): void {
  const root = document.documentElement;
  if (theme === "system") {
    delete root.dataset["theme"];
  } else {
    root.dataset["theme"] = theme;
  }
}

/** Applies the stored theme. Runs before the page is painted, so there is no flash. */
export function applyStoredTheme(): void {
  apply(load());
}

/** A button that cycles system → light → dark. Renders nothing until defined. */
export class HermeticThemeToggle extends HTMLElement {
  private button: HTMLButtonElement | null = null;

  connectedCallback(): void {
    whenParsed(() => this.setup());
  }

  private setup(): void {
    if (this.button) {
      return;
    }
    const button = document.createElement("button");
    button.type = "button";
    button.className = "corner-button theme-button";
    let theme = load();
    const show = (): void => {
      const next = ORDER[(ORDER.indexOf(theme) + 1) % ORDER.length] ?? "system";
      const label = `Theme: ${theme} (switch to ${next})`;
      button.setAttribute("aria-label", label);
      button.title = label;
      button.dataset.theme = theme;
      button.replaceChildren(icon(ICONS[theme]));
    };
    show();
    button.addEventListener("click", () => {
      theme = ORDER[(ORDER.indexOf(theme) + 1) % ORDER.length] ?? "system";
      apply(theme);
      save(theme);
      show();
    });
    this.button = button;
    this.append(button);
  }
}

export function define(name = "hermetic-theme-toggle"): void {
  if (!customElements.get(name)) {
    customElements.define(name, HermeticThemeToggle);
  }
}
