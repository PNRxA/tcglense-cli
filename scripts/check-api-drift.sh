#!/usr/bin/env bash
# Detect drift between the live TCGLense API surface and what this CLI is known
# to cover.
#
# The API publishes a versioned OpenAPI document at `/api/openapi.json`. This
# script fetches it, extracts every operation as "METHOD /path", and diffs that
# set against the committed baseline in `scripts/api-endpoints.txt` (the surface
# the CLI implements as of the last update).
#
#   - Operations in the API but NOT in the baseline  -> the CLI may be MISSING a
#     command for a newly added endpoint. Wire it up (see src/commands/*.rs and
#     src/cli.rs), then refresh the baseline.
#   - Operations in the baseline but NOT in the API   -> an endpoint was removed
#     or renamed upstream; the CLI command for it is now dead.
#
# Exit status: 0 when the sets match, 1 when they differ (so it doubles as a CI
# check), 2 on a usage/fetch error.
#
# Usage:
#   scripts/check-api-drift.sh [BASE_URL]        # default https://tcglense.com
#   scripts/check-api-drift.sh --update [BASE_URL]   # rewrite the baseline to match live
#
# Requires: curl, jq.

set -euo pipefail

update=0
if [[ "${1:-}" == "--update" ]]; then
  update=1
  shift
fi

base_url="${1:-https://tcglense.com}"
base_url="${base_url%/}"
spec_url="$base_url/api/openapi.json"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
baseline="$here/api-endpoints.txt"

for tool in curl jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: '$tool' is required but not installed." >&2
    exit 2
  fi
done

spec="$(curl -fsSL --max-time 45 "$spec_url")" || {
  echo "error: could not fetch $spec_url" >&2
  exit 2
}

version="$(printf '%s' "$spec" | jq -r '.info.version // "?"')"

# Extract "METHOD /path" for every operation, sorted and de-duplicated.
live="$(printf '%s' "$spec" | jq -r '
  .paths | to_entries[] | .key as $p
  | (.value | keys[] | select(test("^(get|post|put|delete|patch)$")) | ascii_upcase) as $m
  | "\($m) \($p)"' | LC_ALL=C sort -u)"

if [[ "$update" == "1" ]]; then
  printf '%s\n' "$live" > "$baseline"
  echo "Updated $baseline to match live API (v$version): $(printf '%s\n' "$live" | grep -c . ) operations."
  exit 0
fi

if [[ ! -f "$baseline" ]]; then
  echo "error: baseline $baseline not found; create it with: $0 --update" >&2
  exit 2
fi

# comm needs sorted inputs; the baseline is committed sorted, `live` is sorted above.
added="$(LC_ALL=C comm -13 <(LC_ALL=C sort -u "$baseline") <(printf '%s\n' "$live"))"
removed="$(LC_ALL=C comm -23 <(LC_ALL=C sort -u "$baseline") <(printf '%s\n' "$live"))"

echo "TCGLense API $spec_url (v$version)"
echo "baseline: $(grep -c . "$baseline") operations · live: $(printf '%s\n' "$live" | grep -c .) operations"

drift=0

if [[ -n "$added" ]]; then
  drift=1
  echo
  echo "== In the API but MISSING from the CLI baseline (add a command, then re-run with --update) =="
  printf '%s\n' "$added" | sed 's/^/  + /'
fi

if [[ -n "$removed" ]]; then
  drift=1
  echo
  echo "== In the CLI baseline but GONE from the API (removed/renamed upstream) =="
  printf '%s\n' "$removed" | sed 's/^/  - /'
fi

if [[ "$drift" == "0" ]]; then
  echo
  echo "No drift: the CLI covers every documented API operation."
  exit 0
fi

echo
echo "Drift detected. After reconciling, refresh the baseline: $0 --update"
exit 1
