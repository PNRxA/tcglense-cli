# CLAUDE.md

Guidance for working in this repo.

## What this is

`tcglense` — a Rust CLI + TUI client for the [TCGLense](https://github.com/PNRxA/tcglense)
API. One-shot subcommands (clap) for scripting plus a ratatui TUI. It talks to the
public TCGLense HTTP JSON API (default origin `https://tcglense.com`).

## Layout

- `src/cli.rs` — the clap surface: the top-level `Command` enum. Each variant's args
  live next to its handler in `src/commands/<domain>.rs`.
- `src/commands/mod.rs` — `dispatch()` maps each `Command` to its handler.
- `src/commands/` — `catalog` (games/sets/cards/products/scan/images), `collection`,
  `wishlist`, `decks`, `public`, `auth`, `misc` (health/config/openapi/update).
- `src/commands/holdings.rs` — a **shared engine** for the collection + wish-list
  surfaces, parameterised by a `Surface { base, batch_route, product_batch_route }`.
  Its paths are built by string concatenation off `base` (e.g. `{base}/cards/{id}`,
  `{base}/sets/{code}/drops`). The public collection/wish-list surfaces reuse it too.
- `src/client.rs` — thin async HTTP layer (auth header, one-shot 401 refresh).
- `src/models.rs` — wire types.

Because many request paths are assembled dynamically (the holdings engine, and the
`decks.rs` base concat), a plain grep for a literal path will under-count coverage —
trace the `format!(...)` construction when auditing what the CLI calls.

## Dev

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo build --locked
cargo test --locked
```

CI (`.github/workflows/ci.yml`) runs fmt + clippy + build + test. Releases are cut with
`scripts/release.sh` (bumps the version, tags, publishes the GitHub Release, whose
workflow builds and attaches per-platform binaries).

## Staying in sync with the API

The CLI aims to cover the whole documented API surface. The API publishes a versioned
OpenAPI document at `/api/openapi.json`; `scripts/api-endpoints.txt` is the committed
baseline of the operations the CLI covers (as of the last update).

**Run the drift check to see whether the API has grown past the CLI:**

```sh
scripts/check-api-drift.sh                 # diff live prod API against the baseline
scripts/check-api-drift.sh http://localhost:5173   # check against a local/self-host API
scripts/check-api-drift.sh --update        # rewrite the baseline to match live
```

It fetches the live spec, diffs its `METHOD /path` operation set against the baseline,
and lists any operations the API has that the CLI baseline lacks (a likely **missing
command**) or that the baseline has but the API dropped. It exits non-zero on drift, so
it can gate CI or run on a schedule. Requires `curl` + `jq`.

When it flags a new endpoint: add the command (an arg struct + handler in the right
`src/commands/*.rs`, a `Command` variant in `src/cli.rs`, a dispatch arm in
`src/commands/mod.rs`, a wire type in `src/models.rs` if needed, and a README example),
then re-run with `--update` to refresh the baseline. A good template is any small
read command — e.g. `catalog::rulings` (`GET /api/games/{game}/cards/{id}/rulings`).
