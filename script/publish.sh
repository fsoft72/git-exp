#!/usr/bin/env bash
set -euo pipefail

# Move to the repository root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new_version> (e.g. 0.1.2)" >&2
    exit 1
fi

NEW_VERSION="$1"

# Validate version format (e.g. 0.1.2 or 0.1.2-beta.1)
if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "Error: invalid version format '$NEW_VERSION'. Expected SemVer (e.g. 0.1.2)." >&2
    exit 1
fi

# 1. Ensure current branch is master
CURRENT_BRANCH="$(git branch --show-current)"
if [ "$CURRENT_BRANCH" != "master" ]; then
    echo "Error: script must be run from 'master' branch (current branch: '$CURRENT_BRANCH')." >&2
    exit 1
fi

# 2. Ensure working tree is clean (no modified, staged, or untracked files)
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: working tree is not clean. Commit or discard changes before proceeding." >&2
    git status --short >&2
    exit 1
fi

echo "==> Updating version to $NEW_VERSION..."

# Update Cargo.toml ([package] section only)
sed -i '/^\[package\]/,/^\[/ s/^version = ".*"/version = "'"$NEW_VERSION"'"/' Cargo.toml

# Update Cargo.lock (git-exp package block)
sed -i '/^name = "git-exp"$/{n;s/^version = ".*"/version = "'"$NEW_VERSION"'"/;}' Cargo.lock

# Update .github/workflows/release.yml
if [ -f ".github/workflows/release.yml" ]; then
    sed -i 's/^\([[:space:]]*VERSION:[[:space:]]*\)"[^"]*"/\1"'"$NEW_VERSION"'"/' .github/workflows/release.yml
fi

# Commit and push to master
echo "==> Committing and pushing to master..."
git add Cargo.toml Cargo.lock .github/workflows/release.yml
git commit -m "CHORE: bump version to $NEW_VERSION"
git push origin master

# Switch to publish branch, fast-forward merge master, and push
echo "==> Switching to publish, merging master, and pushing..."
if git show-ref --verify --quiet refs/heads/publish; then
    git checkout publish
else
    git checkout -b publish
fi

git merge --ff-only master
git push origin publish

# Return to master
echo "==> Returning to master branch..."
git checkout master

echo "==> Version $NEW_VERSION published successfully!"
