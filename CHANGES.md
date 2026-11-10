# Changes

## Unreleased

- Git graph rows show the commit date (`YYYY-MM-DD HH:MM:SS`) after the author name; author and date are right aligned in their own column so they no longer overlap the subject. On a narrow sidebar (text area under 400 px) the author is hidden and only the date is shown
- The SCM sidebar graph section sits below the changes scroll area (single scrollbar per area, no nested scrolling) and cannot widen the sidebar
- The Source Control Graph section of the SCM sidebar is now a scrollable box filling the rest of the sidebar height (no empty area below it) showing the whole history with paged loading; before it stopped at 28 rows with no way to scroll

## 0.2.0 - 2026-11-10

- Bumped version to `0.2.0` in `Cargo.toml`, `Cargo.lock` and the release workflow

- Added a collapsed `IGNORED (n)` section in the Source Control sidebar listing paths ignored by `.gitignore` in grey (whole ignored folders are listed once, read-only)
- The watcher no longer triggers a refresh when only ignored files change: touched paths are queued and checked with one `git check-ignore --stdin` call per refresh interval (paths inside `.git` always count)
- The filesystem watcher no longer scans folders ignored by `.gitignore`: the list comes from `git ls-files --others --ignored --exclude-standard --directory` (new folders are checked with `git check-ignore`); the hardcoded `node_modules`/`target` names are only a fallback when git is unavailable
- GITXP-3: the Source Control Graph panel shows the whole history: commits are loaded in pages of 200 as the user scrolls near the end, and the list is virtualized (only visible rows are rendered)
- GITXP-2: opening big repositories is fast again: the filesystem watcher is built on a background thread, skips `node_modules`, `target` and `.git/objects|logs` (7.6 s down to 13 ms on a 32k files repo), and an "Opening repository..." busy label is shown while the first refresh runs
- GITXP-1: added an "Open Folder" button at the bottom left of the status bar; it opens a folder picker, closes the current project (tabs, selection) and shows files and git of the selected folder

## 0.1.1 - 2026-11-01

- Bumped version to `0.1.1` in `Cargo.toml`, `Cargo.lock` and the release workflow

## 0.1.0 - 2026-11-01

- Added `README.md` describing the project (English), including the vibe-coded note
- Added MIT `LICENSE`
