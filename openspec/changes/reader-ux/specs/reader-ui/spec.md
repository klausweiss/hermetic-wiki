# Spec Delta

## Purpose

Defines the reading experience of the generated site: visual design, theming, responsive layout, keyboard shortcuts, and how the bundled script enhances pages without ever degrading the plain HTML experience.

## ADDED Requirements

### Requirement: Readable typography
Pages SHALL use the embedded Inter for text and JetBrains Mono for code, with body text of at least 16px, line height of at least 1.5, and a content measure of at most about 75 characters per line on wide screens.

#### Scenario: Wide screen reading
- **WHEN** a page is viewed on a 1440px-wide window without the sidebar
- **THEN** body text lines do not exceed about 75 characters and use Inter

### Requirement: Themes
Pages SHALL follow the system's light or dark preference by default. When the script runs, it SHALL offer a toggle cycling between system, light, and dark, and SHALL remember the choice in the browser so it applies to every page of the site. Without the script, pages SHALL still follow the system preference.

#### Scenario: Dark system preference
- **WHEN** the operating system prefers a dark appearance and the user has not chosen a theme
- **THEN** pages render with the dark palette, with or without JavaScript

#### Scenario: Manual choice persists
- **WHEN** the user switches to light theme on one page and opens another page of the site
- **THEN** the other page renders in the light theme

### Requirement: Responsive layout
Pages SHALL be usable from 320px wide upward without horizontal page scrolling; wide content such as tables and code blocks SHALL scroll within themselves.

#### Scenario: Phone width
- **WHEN** a page with a wide table is viewed 360px wide
- **THEN** the page does not scroll horizontally and the table scrolls within its own box

### Requirement: Progressive enhancement
The bundled script SHALL only add behavior to pages that are already complete without it. It SHALL activate features that fetch other pages (opening sidebar panes, the timeline) only when the page is served over `http:` or `https:`; over `file:` it SHALL leave link behavior unchanged and offer no control that cannot work, explaining in the sidebar instead why it stays empty. Controls that need the script SHALL only be added by the script, so they never appear when it does not run. The script SHALL load no other resource from outside the output directory.

#### Scenario: Opened from disk with JavaScript enabled
- **WHEN** a page is opened via `file://` with JavaScript enabled and the user Shift+clicks a page link
- **THEN** the browser's default behavior happens, no pane opens, no timeline control is shown, and the sidebar explains that opening pages there needs the site to be served over `http://`

#### Scenario: JavaScript disabled
- **WHEN** a page is served over `http://` with JavaScript disabled
- **THEN** it shows no theme toggle, timeline toggle, or sidebar, and all links work as plain links

### Requirement: Keyboard shortcuts
When the script runs, it SHALL provide these shortcuts, ignored while typing in a text field: `g` then `h` opens the home page, `g` then `j` the journal list, `g` then `p` all pages, `g` then `t` open tasks; `Escape` closes the topmost sidebar pane; `\` shows or hides the sidebar; `?` shows a list of the shortcuts.

#### Scenario: Go to journals
- **WHEN** the user presses `g` then `j` on any page
- **THEN** the journal list page opens

#### Scenario: Typing is not intercepted
- **WHEN** focus is in a text input and the user types `g j`
- **THEN** the characters are typed and no navigation happens
