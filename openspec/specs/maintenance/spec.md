# maintenance Specification

## Purpose

Gives the author one place in `hermetic serve` for housekeeping of the notes folder, starting with finding attachments that nothing uses and moving them to the trash.

## Requirements

### Requirement: Maintenance page
The server SHALL serve a maintenance page at `/maintenance.html` where editing is available (see "Where editing is available" in `browser-editing`). It SHALL use the site's page layout (header, footer, sidebar). Its title SHALL be "Maintenance". It SHALL list every maintenance action with its name, a one-sentence description, and a link to the action's own page. The only action SHALL be "Unused assets", linking to `/maintenance/unused-assets.html`. Maintenance pages SHALL NOT be part of the site's files: `hermetic build` SHALL NOT write them, they SHALL NOT appear in the all-pages list, the search index, or the graph, and requests from other devices SHALL get 404. Where editing is available, the site footer SHALL contain a link "Maintenance" to the maintenance page, after the link to the all-pages list. Built sites and pages served to other devices SHALL NOT contain it.

#### Scenario: Open from the footer
- **WHEN** the author opens any page from `hermetic serve` on the same machine and chooses "Maintenance" in the footer
- **THEN** the maintenance page opens and lists "Unused assets" with its description and a link to it

#### Scenario: Another device
- **WHEN** a phone requests `/maintenance.html` or `/maintenance/unused-assets.html` from `hermetic serve --public`
- **THEN** the response is 404, and the phone's pages have no "Maintenance" link in the footer

#### Scenario: Built site
- **WHEN** a site is produced by `hermetic build`
- **THEN** it contains no `maintenance.html` and no `maintenance/` directory, and its footer has no "Maintenance" link

### Requirement: Unused assets
An attachment (see "Attachments" in `static-site-output`) SHALL be unused when both of these hold:
- No inline markdown image or link in a block's content or a property value of any note resolves to it. Resolution SHALL follow "Attachment links" in `static-site-output`, ignoring any `#fragment`.
- Its file name does not appear in the text of any note file as a whole name (not as part of a longer name such as `used.png` for `d.png`), neither as written nor percent-encoded. This keeps files that raw HTML or reference-style links use.

Attachments that the build skips (symbolic links, and files whose path a generated page takes) SHALL NOT be listed. The rule SHALL apply to attachments in `journals/`, `pages/`, and `assets/` alike.

#### Scenario: Image removed from its block
- **WHEN** `assets/shot_1_0.png` was pasted into `journals/2026_10_17.md` and the image markdown was then deleted from the block
- **THEN** `assets/shot_1_0.png` is unused

#### Scenario: Still shown
- **WHEN** `pages/auth.md` contains `![flow](auth-flow.png)` and `pages/auth-flow.png` exists
- **THEN** `pages/auth-flow.png` is not unused

#### Scenario: Linked, not shown
- **WHEN** a journal contains `[spec](../assets/spec.pdf#page=2)`
- **THEN** `assets/spec.pdf` is not unused

#### Scenario: Raw HTML
- **WHEN** a page contains `<img src="../assets/diagram.svg">` and nothing else names `assets/diagram.svg`
- **THEN** `assets/diagram.svg` is not unused, because its file name appears in the note

#### Scenario: Part of a longer name
- **WHEN** `assets/d.png` exists and the only note text containing `d.png` is `![u](../assets/used.png)`
- **THEN** `assets/d.png` is unused

#### Scenario: Name with spaces
- **WHEN** `assets/my shot.png` exists and a note contains only the text `my%20shot.png` in an HTML attribute
- **THEN** `assets/my shot.png` is not unused

### Requirement: Unused assets page
`/maintenance/unused-assets.html` SHALL be served where editing is available, with the title "Unused assets". It SHALL list every unused asset, ordered by path, showing:
- a small preview for PNG, JPEG, GIF, WebP, and SVG files,
- the path relative to the notes folder, linking to the file,
- the file's size in human-readable units.

Above the list it SHALL show the number of unused assets and their total size. With no unused assets, it SHALL say so instead of showing an empty list.

Each entry SHALL have a checkbox. A "Select all" checkbox SHALL select or clear every entry, and SHALL show a mixed state when only some are selected. Shift+click on an entry's checkbox SHALL set every entry between it and the previously clicked one to the clicked one's new state. `.` then `a` (see the page menu in `reader-ui`) SHALL select every entry, or clear them all when all are selected. `.` then `d` SHALL do what "Move to trash" does, when at least one entry is selected. A "Move to trash" button SHALL show the number of selected entries, and SHALL be disabled when none is selected.

Choosing "Move to trash" SHALL ask for confirmation in a dialog. The dialog SHALL state the number of files and their total size, list their paths, and say that they are moved to `.trash/` in the notes folder. Nothing SHALL change unless the author confirms. After a confirmed move, the page SHALL report how many files were moved and the trash folder they were moved to. It SHALL name any file that was not moved and the reason, and show the updated list. When the notes change (live reload), the list SHALL be updated, keeping the selection of entries still listed.

#### Scenario: Clean up two pasted images
- **WHEN** `assets/a.png` and `assets/b.png` are unused, the author selects both, chooses "Move to trash (2)", and confirms
- **THEN** both files are under `.trash/<timestamp>/assets/`, neither is in `assets/` any more, and the page reports 2 files moved and lists neither

#### Scenario: Cancel
- **WHEN** the author selects files, chooses "Move to trash", and cancels the confirmation
- **THEN** no file is moved and the selection is kept

#### Scenario: Select all
- **WHEN** the page lists 12 unused assets and the author checks "Select all"
- **THEN** all 12 are selected and the button reads "Move to trash (12)"

#### Scenario: Select all from the keyboard
- **WHEN** the page lists 3 unused assets, none selected, and the author presses `.` then `a`, then `.` then `a` again
- **THEN** all 3 are selected after the first press and none after the second, also while a checkbox has the focus

#### Scenario: Move to trash from the keyboard
- **WHEN** two entries are selected and the author presses `.` then `d`
- **THEN** the confirmation for those 2 files opens, and nothing is moved until the author confirms

#### Scenario: Range selection
- **WHEN** the author checks the 2nd entry and then Shift+clicks the 6th
- **THEN** entries 2 through 6 are selected

#### Scenario: Used again in the meantime
- **WHEN** the page lists `assets/a.png` and the author pastes `![a](../assets/a.png)` into a journal in another tab
- **THEN** the list updates and no longer shows `assets/a.png`

#### Scenario: Nothing to clean
- **WHEN** every attachment is used
- **THEN** the page says that there are no unused assets and shows no "Move to trash" button

### Requirement: Maintenance API
The server SHALL offer the unused assets at `/_hermetic/unused-assets`.

`GET` SHALL return, as JSON, the unused assets in the order of "Unused assets page", each with its path relative to the notes folder (with `/` separators) and its size in bytes.

`POST` SHALL take a JSON body `{"paths": [...]}` of paths relative to the notes folder. A body that is not such JSON SHALL get 400, and a body larger than 1 MiB SHALL get 413. The server SHALL move to the trash each listed path that is an unused asset at the time of the request, after the checks of "Safe writes" in `browser-editing`. All files moved by one request SHALL go to the same `.trash/<timestamp>/` folder, each under its path relative to the notes folder. Each other path SHALL be left alone and reported as skipped, with a reason: either it is not an unused asset, or the move failed. A path SHALL never be used to reach a file that is not an unused asset, including through `..`, absolute paths, or symbolic links. The response SHALL be 200 and SHALL be sent after the site has been rebuilt without the moved files. It SHALL give, as JSON, the moved paths, the trash folder relative to the notes folder, and the skipped paths with their reasons.

#### Scenario: List
- **WHEN** the page requests `GET /_hermetic/unused-assets` and `assets/a.png` (2048 bytes) is the only unused asset
- **THEN** the response is JSON listing `assets/a.png` with size 2048

#### Scenario: Not unused
- **WHEN** a `POST` names `pages/auth-flow.png`, which `pages/auth.md` shows, and `pages/auth.md`
- **THEN** both are reported as skipped and neither file is moved

#### Scenario: Path tricks
- **WHEN** a `POST` names `assets/../pages/auth.md`, `/etc/passwd`, or `assets/link.png`, which is a symbolic link
- **THEN** each is reported as skipped and no file is moved or read

#### Scenario: One trash folder per request
- **WHEN** a `POST` moves `assets/a.png` and `pages/sub/b.png`
- **THEN** they are at `.trash/<t>/assets/a.png` and `.trash/<t>/pages/sub/b.png` with the same `<t>`, and `pages/sub/` is removed if this left it empty
