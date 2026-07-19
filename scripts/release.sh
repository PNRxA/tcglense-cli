#!/usr/bin/env bash
#
# Cut a tcglense-cli release.
#
# Prompts for a version, bumps it in Cargo.toml (+ Cargo.lock), commits, tags
# `vX.Y.Z`, pushes, and publishes a GitHub Release. Publishing the release fires the
# "Release" workflow (.github/workflows/release.yml), which builds the per-platform
# binaries and attaches them to the release.
#
# Run from anywhere:  ./scripts/release.sh
#
# Prerequisites: a clean working tree, and git / cargo / gh on PATH with `gh`
# authenticated (`gh auth login`).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

red()  { printf '\033[31m%s\033[0m\n' "$*" >&2; }
bold() { printf '\033[1m%s\033[0m\n' "$*"; }
die()  { red "Error: $*"; exit 1; }

require() { command -v "$1" >/dev/null 2>&1 || die "'$1' is not installed or not on PATH."; }
require git
require cargo
require gh

git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "not inside a git repository."
gh auth status >/dev/null 2>&1 || die "gh is not authenticated. Run: gh auth login"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[ "$BRANCH" = "main" ] || {
  red "You are on '$BRANCH', not 'main'."
  read -r -p "Continue anyway? [y/N] " reply
  [ "$reply" = "y" ] || [ "$reply" = "Y" ] || die "aborted."
}

[ -z "$(git status --porcelain)" ] || die "working tree is not clean. Commit or stash first."

echo "-> Fetching from origin..."
git fetch --quiet --tags origin
if git rev-parse --verify --quiet "origin/main" >/dev/null; then
  behind="$(git rev-list --count "HEAD..origin/main")"
  [ "$behind" -eq 0 ] || die "HEAD is $behind commit(s) behind origin/main. Pull/rebase first."
fi

# --- Current version (from the [package] table of Cargo.toml) --------------------
current_version="$(
  awk '
    /^\[/ { in_pkg = ($0 == "[package]") }
    in_pkg && /^version[[:space:]]*=/ {
      match($0, /"[^"]*"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
    }
  ' Cargo.toml
)"
[ -n "$current_version" ] || die "could not read the current version from Cargo.toml."

bold "tcglense-cli release"
echo "  Current version: $current_version"
echo

read -r -p "New version (X.Y.Z, without a leading 'v'): " VERSION
VERSION="${VERSION#v}"
if ! printf '%s' "$VERSION" \
  | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'; then
  die "'$VERSION' is not valid semver (expected X.Y.Z or X.Y.Z-prerelease, no leading zeros)."
fi

TAG="v$VERSION"
PRERELEASE=false
case "$VERSION" in *-*) PRERELEASE=true ;; esac

git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && die "tag $TAG already exists locally."
[ -z "$(git ls-remote --tags origin "$TAG")" ] || die "tag $TAG already exists on origin."

echo
bold "About to:"
echo "  1. Bump Cargo.toml to $VERSION (and Cargo.lock)"
echo "  2. Commit + tag $TAG on main and push"
echo "  3. Publish GitHub Release $TAG$( $PRERELEASE && printf ' (pre-release)' ) — builds + attaches binaries"
echo
read -r -p "Proceed? [y/N] " reply
[ "$reply" = "y" ] || [ "$reply" = "Y" ] || die "aborted (no changes made)."

# The locked version of just this package (not a dep that shares the version string).
lock_pkg_version() {
  awk '
    /^\[\[package\]\]/ { name = "" }
    /^name = / { name = $0 }
    name == "name = \"tcglense-cli\"" && /^version = / {
      match($0, /"[^"]*"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
    }
  ' Cargo.lock
}

echo "-> Bumping Cargo.toml..."
tmp="$(mktemp)"
awk -v ver="$VERSION" '
  /^\[/ { in_pkg = ($0 == "[package]") }
  in_pkg && /^version[[:space:]]*=/ && !done { print "version = \"" ver "\""; done = 1; next }
  { print }
' Cargo.toml > "$tmp" && mv "$tmp" Cargo.toml

echo "-> Updating Cargo.lock..."
cargo update --quiet --package tcglense-cli
if [ "$(lock_pkg_version)" != "$VERSION" ]; then
  tmp="$(mktemp)"
  awk -v ver="$VERSION" '
    /^\[\[package\]\]/ { pkg = 1; name = "" }
    pkg && /^name = / { name = $0 }
    pkg && /^version = / && name == "name = \"tcglense-cli\"" { print "version = \"" ver "\""; pkg = 0; next }
    { print }
  ' Cargo.lock > "$tmp" && mv "$tmp" Cargo.lock
fi
[ "$(lock_pkg_version)" = "$VERSION" ] || die "failed to update Cargo.lock to $VERSION."

echo "-> Committing, tagging, pushing..."
git add Cargo.toml Cargo.lock
git commit --quiet -m "chore(release): $TAG"
git tag -a "$TAG" -m "Release $TAG"
git push --quiet origin HEAD
git push --quiet origin "$TAG"

echo "-> Publishing GitHub Release $TAG..."
release_args=(--title "$TAG" --generate-notes)
$PRERELEASE && release_args+=(--prerelease)
gh release create "$TAG" "${release_args[@]}"

echo
bold "Released $TAG 🎉"
repo_slug="$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)"
[ -n "$repo_slug" ] && echo "  Release: https://github.com/$repo_slug/releases/tag/$TAG"
echo "  The 'Release' workflow is now building the per-platform binaries."
