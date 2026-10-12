# Spec Delta

## MODIFIED Requirements

### Requirement: Themes
Pages SHALL follow the system's light or dark preference by default. When the script runs, it SHALL offer a toggle cycling between system, light, and dark: an icon button showing the current theme, with an accessible name stating the current and the next theme, fixed in the top-right corner next to the sidebar's show/hide control (or in the corner itself where there is none), never overlapping the header's links, search box, or the sidebar's controls, and SHALL remember the choice in the browser so it applies to every page of the site. Without the script, pages SHALL still follow the system preference.

#### Scenario: Dark system preference
- **WHEN** the operating system prefers a dark appearance and the user has not chosen a theme
- **THEN** pages render with the dark palette, with or without JavaScript

#### Scenario: Manual choice persists
- **WHEN** the user switches to light theme on one page and opens another page of the site
- **THEN** the other page renders in the light theme

#### Scenario: Corner controls
- **WHEN** a page is shown at any width from 320px up, with the sidebar shown or hidden
- **THEN** the theme button sits directly left of the sidebar's control in the top-right corner, neither moves when the sidebar is toggled, and neither covers a header link or the search box
