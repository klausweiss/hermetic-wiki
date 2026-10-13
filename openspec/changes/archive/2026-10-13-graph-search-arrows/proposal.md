# Proposal

## Why

The graph search steps through matches with Enter and Shift+Enter and opens one with Ctrl+Enter. The site search dialog uses the arrow keys to move and Enter to open, and the graph search should work the same way, so one habit covers both.

## What Changes

- In the graph search box, the down and up arrows move to the next and previous match (wrapping around), centring it as Enter does today.
- Enter opens the selected match's page. With no match selected yet, Enter opens the match when there is exactly one, and otherwise selects the first.
- **BREAKING** (keys only): Shift+Enter and Ctrl+Enter no longer do anything special in the graph search.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `graph-view`: "Search in the graph": arrow keys move between matches, Enter opens the selected one.

## Impact

- `static/packages/graph/src/index.ts` (the search box's key handling) and `static/dist/app.js`.
- `src/help/pages.txt` (the Graph section names the keys).
