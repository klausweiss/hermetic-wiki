# Spec Delta

## MODIFIED Requirements

### Requirement: Live reload
Pages served by `hermetic serve` SHALL receive, after every rebuild, the list of site paths whose content changed or which were removed. A tab SHALL reload only when its own page is in that list, keeping its scroll position and open sidebar panes, unless the page is open for editing, in which case it SHALL update in place as `browser-editing` requires; a sidebar pane whose page is in the list SHALL refresh its content in place, and a pane whose page was removed SHALL close. When the number of open tasks changed, every open tab SHALL show the new number on its open-tasks link (see "Site navigation" in `static-site-output`) without reloading. When the connection to the server is lost, the page SHALL keep working and reconnect when the server is back. Built sites (`hermetic build`) SHALL contain no live-reload behavior.

#### Scenario: Editing the page being read
- **WHEN** `pages/auth.html` is open on another device (where it is not open for editing), scrolled halfway, and the author edits `pages/auth.md`
- **THEN** the tab reloads and returns to about the same scroll position

#### Scenario: Editing the page open for editing
- **WHEN** `pages/auth.html` is open on the same machine (open for editing), scrolled halfway, and the author edits `pages/auth.md` in another editor
- **THEN** the tab does not reload, shows the new content, and keeps its scroll position

#### Scenario: Pages that are not editable still reload
- **WHEN** `journals.html` is open on the same machine and a new journal day is saved
- **THEN** the tab reloads and lists the new day

#### Scenario: Editing an unrelated note
- **WHEN** `pages/auth.html` is open and the author edits a note that neither links to `auth` nor is linked from it
- **THEN** the tab does not reload

#### Scenario: Adding a task elsewhere
- **WHEN** `pages/auth.html` is open showing 3 open tasks on its open-tasks link, and the author adds a `TODO` block to a note that neither links to `auth` nor is linked from it
- **THEN** the tab does not reload and its open-tasks link shows 4

#### Scenario: Pane refresh
- **WHEN** `journals/2026-10-12.html` is open with a `redis` pane and the author edits `pages/redis.md`
- **THEN** the pane shows the new content and the main page does not reload

#### Scenario: Built site has no live reload
- **WHEN** the output of `hermetic build` is served by another web server
- **THEN** its pages make no live-reload connections
