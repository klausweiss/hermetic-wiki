# Proposal

## Why

While writing, the notes should be viewable live, with all JS enhancements (multi-pane, merged journal, search) that need `http://`, without running a separate web server or rebuilding by hand.

## What Changes

- Add `hermetic serve <dir>`: serves the same output `hermetic build` produces, from the same single binary, with no configuration.
- Watch the source directory and rebuild incrementally: a changed file re-renders itself, the pages whose linked references or tasks it affects, and the affected index entries.
- Live reload of open pages after a rebuild.
- Binds to localhost by default.

## Capabilities

### New Capabilities
- `serve-mode`: the serve command, file watching, incremental rebuild, live reload, and network binding defaults.

### Modified Capabilities
- `cli`: adds the `serve` command.

## Impact

- Depends on `static-build` and `search`.
- New dependencies: a file watcher (e.g. `notify`) and an HTTP server (to be chosen in design; async runtime may already be required by `search`).
