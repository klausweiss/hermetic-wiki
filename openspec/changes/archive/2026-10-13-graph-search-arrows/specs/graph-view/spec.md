# Spec Delta

## MODIFIED Requirements

### Requirement: Search in the graph
When the script runs, the graph SHALL offer a search box, opened with a control next to the zoom controls or with `Ctrl+F` (`Cmd+F` on macOS) on the graph page. Pressing `Ctrl+F` again while the search box has focus SHALL open the browser's own find instead. A node SHALL match when every word of the query appears in its page name, ignoring case and diacritics as site search does. While the query is not empty, matching nodes SHALL be emphasized with their labels shown, the rest de-emphasized, and the number of matches shown. The down arrow SHALL select the next match and the up arrow the previous one, wrapping around at either end, centring the selected match, zooming in until its label is readable, and highlighting its neighbours. `Enter` SHALL open the selected match's page; with no match selected, it SHALL open the match when there is exactly one, and otherwise select the first. `Escape` SHALL clear the query, close the search box, and return to the view from before the search. Searching SHALL work over `file://` as well. On other pages, `Ctrl+F` SHALL keep the browser's behavior.

#### Scenario: Matching nodes
- **WHEN** the graph has the pages `Project X/Auth Flow`, `auth`, and `redis`, and the user types `auth` into the graph search
- **THEN** `Project X/Auth Flow` and `auth` are emphasized with their labels shown, `redis` is dimmed, and the box reports 2 matches

#### Scenario: Several words and accents
- **WHEN** the user types `flow AUTH` or `zurich`
- **THEN** `Project X/Auth Flow` matches the first query and `Zürich` the second

#### Scenario: Going through matches
- **WHEN** a query has two matches and the user presses the down arrow twice, then the up arrow
- **THEN** each press centres the selected match with its label readable: the first, the second, then the first again

#### Scenario: Wrapping around
- **WHEN** the last of three matches is selected and the user presses the down arrow
- **THEN** the first match is selected

#### Scenario: Open a match
- **WHEN** a match is selected and the user presses `Enter`
- **THEN** that match's page opens

#### Scenario: Enter with one match
- **WHEN** a query has exactly one match, none is selected, and the user presses `Enter`
- **THEN** that match's page opens

#### Scenario: Enter with several matches
- **WHEN** a query has several matches, none is selected, and the user presses `Enter`
- **THEN** the first match is selected and centred, and no page opens

#### Scenario: Escape
- **WHEN** the user has searched and selected a match, then presses `Escape`
- **THEN** the search box closes, no node is dimmed, and the graph shows the view it had before the search

#### Scenario: Browser find stays reachable
- **WHEN** the graph search box has focus and the user presses `Ctrl+F`
- **THEN** the browser's own find opens

#### Scenario: Other pages
- **WHEN** the user presses `Ctrl+F` on a journal page
- **THEN** the browser's own find opens
