#!/usr/bin/env bash
# Detect drift between the live TCGLense API surface and what this CLI is known
# to cover.
#
# The API publishes a versioned OpenAPI document at `/api/openapi.json`. This
# script fetches it, flattens it to one line per addressable thing, and diffs that
# set against the committed baseline in `scripts/api-endpoints.txt` (the surface
# the CLI implements as of the last update).
#
# Three line shapes, so drift *within* an operation is caught too — an endpoint
# that grows a filter or a body field is a gap in the CLI just as surely as a
# whole new route (that is how the life tracker's counters and the deck
# maybeboard were missed while the route list matched exactly):
#
#   GET /api/games/{game}/cards           the operation
#   GET /api/games/{game}/cards ?sort     a documented query parameter
#   POST /api/decks/{game} +folder_id     a documented JSON request-body field
#
#   - Lines in the API but NOT in the baseline  -> the CLI may be MISSING a
#     command, a flag, or a body field. Wire it up (see src/commands/*.rs and
#     src/cli.rs), then refresh the baseline.
#   - Lines in the baseline but NOT in the API   -> it was removed or renamed
#     upstream; whatever the CLI sends for it is now dead.
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

# Flatten the spec to "METHOD /path", "METHOD /path ?query-param" and
# "METHOD /path +body-field" lines, sorted and de-duplicated. Body schemas are
# usually a $ref into components, so resolve one hop before reading properties;
# a non-object body (the CSV/text uploads) simply contributes no field lines.
#
# A parameter counts as a query parameter when the path template has no
# `{placeholder}` for it, rather than when it says `in: query` — the generator
# labels some genuine query parameters `in: path` (the goldfish/stats options),
# and trusting the label would silently drop them from the diff.
live="$(printf '%s' "$spec" | jq -r '
  . as $root
  | .paths | to_entries[] | .key as $p
  | .value | to_entries[] | select(.key | test("^(get|post|put|delete|patch)$"))
  | (.key | ascii_upcase) as $m | .value as $op
  | "\($m) \($p)",
    ($op.parameters // [] | .[] as $pr
      | select($p | contains("{" + $pr.name + "}") | not)
      | "\($m) \($p) ?\($pr.name)"),
    ($op.requestBody.content // {} | to_entries[] | .value.schema as $s
      | (if ($s | has("$ref"))
         then ($root | getpath($s["$ref"] | ltrimstr("#/") | split("/")))
         else $s end)
      | (.properties // {}) | keys[] | "\($m) \($p) +\(.)")
  ' | LC_ALL=C sort -u)"

# "132 operations (287 lines with parameters)" — count the bare operation lines
# separately from the parameter/body lines hanging off them.
summarise() {
  local lines="$1"
  local total ops
  total="$(printf '%s\n' "$lines" | grep -c .)"
  ops="$(printf '%s\n' "$lines" | grep -cv ' [?+]' || true)"
  echo "$ops operations ($total lines with parameters)"
}

if [[ "$update" == "1" ]]; then
  printf '%s\n' "$live" > "$baseline"
  echo "Updated $baseline to match live API (v$version): $(summarise "$live")."
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
echo "baseline: $(summarise "$(cat "$baseline")") · live: $(summarise "$live")"

drift=0

if [[ -n "$added" ]]; then
  drift=1
  echo
  echo "== In the API but MISSING from the CLI baseline (add a command or flag, then re-run with --update) =="
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
  echo "No drift: the CLI covers every documented API operation, query parameter and body field."
  exit 0
fi

echo
echo "Drift detected. After reconciling, refresh the baseline: $0 --update"
exit 1
