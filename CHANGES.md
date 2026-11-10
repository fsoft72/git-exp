# Changes

## Unreleased

- GITXP-2: opening big repositories is fast again: the filesystem watcher is built on a background thread, skips `node_modules`, `target` and `.git/objects|logs` (7.6 s down to 13 ms on a 32k files repo), and an "Opening repository..." busy label is shown while the first refresh runs
- GITXP-1: added an "Open Folder" button at the bottom left of the status bar; it opens a folder picker, closes the current project (tabs, selection) and shows files and git of the selected folder

## 0.1.1 - 2026-11-01

- Bumped version to `0.1.1` in `Cargo.toml`, `Cargo.lock` and the release workflow

## 0.1.0 - 2026-11-01

- Added `README.md` describing the project (English), including the vibe-coded note
- Added MIT `LICENSE`
