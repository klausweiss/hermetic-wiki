# Proposal

## Why

The theme toggle was a text button ("Theme: system") at the end of the header row. It overflowed that row, and it sat apart from the sidebar's show/hide control, which already lives in the top-right corner.

## What Changes

- The theme toggle becomes an icon button: sun (light), moon (dark), or sun-moon (system). It sits fixed in the top-right corner, just left of the sidebar's control; where there is no sidebar control, it takes the corner itself.
- The sidebar's show/hide control becomes an icon button too (panel open/close). While the sidebar is hidden and panes are open, it shows the number of panes. Both keep their place whether the sidebar is shown or hidden.
- Both have accessible names that say what they do (for example "Theme: system (switch to light)" and "Show the sidebar, 1 open (\)").
- The header and the sidebar's top bar keep the corner free. The header does so only on screens narrow enough for it to reach the corner.
- The icons are Lucide (ISC license); its license text ships in every site as `_hermetic/LICENSE-Lucide.txt`, like the fonts' licenses.

## Capabilities

### New Capabilities

### Modified Capabilities
- `reader-ui`: where the theme control is and how it is presented.
- `static-site-output`: the Lucide license text is shipped with the embedded assets.

## Impact

- Frontend: `core` (`icons.ts`), `theme-toggle`, `sidebar`, `style.css`; `static/dist/app.js` rebuilt.
- Rust: `src/site/assets.rs` embeds `static/icons/LICENSE-Lucide.txt`; the file-list tests include it.
