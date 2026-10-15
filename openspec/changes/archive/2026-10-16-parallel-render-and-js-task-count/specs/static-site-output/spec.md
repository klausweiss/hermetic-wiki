# Spec Delta

## MODIFIED Requirements

### Requirement: Site navigation
Every page SHALL include a header whose site title links to the home page and which links to the journal list, the open-tasks page, and the graph view, and SHALL use the `--title` value as the site title when given, or the notes directory's name otherwise. When the script runs and can load the site's data files, the link to the open-tasks page SHALL show the number of open tasks in the graph, unless it is zero. The number SHALL NOT be part of the page's HTML, so that a change in the number of open tasks alone changes no page other than the open-tasks list; without the script, and where the data file cannot be loaded (such as a page opened via `file://`), the link SHALL be shown without a number. Every page SHALL also include a site footer with a link to the all-pages list and the text "Generated with hermetic", followed by the commit of hermetic's source the program was built from when that is known. The word "hermetic" SHALL link to hermetic's project page, `https://github.com/klausweiss/hermetic-wiki`. Links SHALL be visually distinct from the footer's text. Without the script the footer SHALL appear after the page's content, and with the script at the bottom of the sidebar (see `multi-pane`). The footer SHALL contain nothing that changes between builds of the same notes by the same program. Each page SHALL contain exactly one `<main>` element holding all page-specific content, and SHALL declare the relative path from itself to the output root in a `<meta name="hermetic-root">` element.

#### Scenario: Header on every page
- **WHEN** any generated page is opened
- **THEN** its site title links to `index.html`, and it links to `journals.html`, `tasks.html`, and `graph.html` via correct relative paths

#### Scenario: Open-tasks count shown by the script
- **WHEN** the notes contain 3 open tasks and a generated page is opened over HTTP with JavaScript enabled
- **THEN** the link to `tasks.html` shows the number 3

#### Scenario: No count without the script
- **WHEN** the notes contain 3 open tasks and a generated page is opened with JavaScript disabled, or via `file://`
- **THEN** the link to `tasks.html` is shown and works, without a number

#### Scenario: No count when there are no open tasks
- **WHEN** the notes contain no open tasks and a generated page is opened with JavaScript enabled
- **THEN** the link to `tasks.html` shows no number

#### Scenario: Ticking off a task changes only the pages that show it
- **WHEN** the notes are built, a `TODO` block in `pages/auth.md` that references no page is changed to `DONE`, and the notes are built again
- **THEN** among the HTML files, only `pages/auth.html` and the open-tasks list files differ between the two builds

#### Scenario: Footer on every page
- **WHEN** `pages/Project X/Auth Flow.html` is opened with JavaScript disabled
- **THEN** after the page's content there is a footer with a link to `../../pages.html` and the text "Generated with hermetic" with "hermetic" linking to `https://github.com/klausweiss/hermetic-wiki`

#### Scenario: Commit in the footer
- **WHEN** hermetic was built from a git checkout at commit `473790c`
- **THEN** every page's footer reads "Generated with hermetic 473790c"

#### Scenario: Root path declared
- **WHEN** `pages/Project X/Auth Flow.html` is inspected
- **THEN** it contains `<meta name="hermetic-root" content="../..">`
