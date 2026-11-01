#!/usr/bin/env bash
set -euo pipefail

# Posizionati nella root del repository
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

if [ $# -ne 1 ]; then
    echo "Uso: $0 <nuova_versione> (es. 0.1.2)" >&2
    exit 1
fi

NEW_VERSION="$1"

# Valida formato versione (es. 0.1.2 o 0.1.2-beta.1)
if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "Errore: formato versione non valido '$NEW_VERSION'. Usa SemVer (es. 0.1.2)." >&2
    exit 1
fi

# 1. Verifica di essere sul branch master
CURRENT_BRANCH="$(git branch --show-current)"
if [ "$CURRENT_BRANCH" != "master" ]; then
    echo "Errore: lo script deve essere eseguito dal branch 'master' (branch attuale: '$CURRENT_BRANCH')." >&2
    exit 1
fi

# 2. Verifica che il repository sia pulito (nessun file modificato, in stage o untracked)
if [ -n "$(git status --porcelain)" ]; then
    echo "Errore: il repository non è pulito. Committa o annulla le modifiche prima di procedere." >&2
    git status --short >&2
    exit 1
fi

echo "==> Aggiornamento versione a $NEW_VERSION..."

# Aggiorna Cargo.toml (solo nella sezione [package])
sed -i '/^\[package\]/,/^\[/ s/^version = ".*"/version = "'"$NEW_VERSION"'"/' Cargo.toml

# Aggiorna Cargo.lock (blocco del pacchetto git-exp)
sed -i '/^name = "git-exp"$/{n;s/^version = ".*"/version = "'"$NEW_VERSION"'"/;}' Cargo.lock

# Aggiorna .github/workflows/release.yml
if [ -f ".github/workflows/release.yml" ]; then
    sed -i 's/^\([[:space:]]*VERSION:[[:space:]]*\)"[^"]*"/\1"'"$NEW_VERSION"'"/' .github/workflows/release.yml
fi

# Commit e push su master
echo "==> Commit e push su master..."
git add Cargo.toml Cargo.lock .github/workflows/release.yml
git commit -m "CHORE: bump version to $NEW_VERSION"
git push origin master

# Switch al branch publish, allinea con master e fai push
echo "==> Switch a publish, merge di master e push..."
if git show-ref --verify --quiet refs/heads/publish; then
    git checkout publish
else
    git checkout -b publish
fi

git merge --ff-only master
git push origin publish

# Torna a master
echo "==> Ritorno al branch master..."
git checkout master

echo "==> Versione $NEW_VERSION pubblicata con successo!"

