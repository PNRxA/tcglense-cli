# TCGLense CLI (`tcglense`)

A Rust command-line client **and** interactive TUI for the
[TCGLense](https://github.com/PNRxA/tcglense) API. It covers the whole public API
surface — the card catalog, sealed products, your collection, wish list and decks,
API-key management, and public sharing — through both one-shot commands (for
scripting/piping) and a keyboard-driven terminal UI (for browsing).

- **Stack:** Rust 2024 · [clap](https://docs.rs/clap) (commands) ·
  [reqwest](https://docs.rs/reqwest) + rustls/ring (HTTP) ·
  [ratatui](https://ratatui.rs) (TUI) · [tokio](https://tokio.rs).
- **Talks to** the [TCGLense API](https://github.com/PNRxA/tcglense) — defaults to the
  production origin `https://tcglense.com`; point it at a self-host or local dev with
  `--url` / `TCGLENSE_URL` or `tcglense config url <URL>`.

## Install

**Prebuilt binaries** for Linux, macOS, and Windows are attached to every
[release](https://github.com/PNRxA/tcglense-cli/releases) — download the archive for
your platform, extract, and put `tcglense` on your `PATH`.

**From source** (needs [Rust](https://rustup.rs/)):

```sh
cargo install --git https://github.com/PNRxA/tcglense-cli   # installs `tcglense`
# or, in a clone:
cargo build --release        # binary at target/release/tcglense
```

## Updating

`tcglense` updates itself from the latest GitHub release:

```sh
tcglense update            # download + install the latest release (prompts first)
tcglense update --check    # only report whether a newer version is available
tcglense update -y         # update without the confirmation prompt
tcglense --version         # show the installed version
```

After an ordinary command it also runs a throttled (once/day), best-effort check for a
newer release and prints a one-line notice to stderr when one exists — only to an
interactive terminal (never when output is piped or redirected). Silence it with
`TCGLENSE_NO_UPDATE_CHECK=1`.

## Authentication

The CLI supports **both** auth models the API offers:

| Model | How | Notes |
|-------|-----|-------|
| **Web session** (email + password) | `tcglense login` | Stores a short-lived access token **and** the opaque refresh token; the CLI silently refreshes on expiry. This is the full-access credential (it can manage API keys). |
| **API key** (`tcgl_…`) | `tcglense auth key tcgl_…` | Programmatic, per-user; scoped `read` or `read_write`. A read-only key gets `403` on writes. Mint one with `tcglense api-keys create`. |

Credentials are stored in `~/.config/tcglense/config.json` (mode `0600`; override
with `--config <path>` or `$TCGLENSE_CONFIG`). For a one-off call without touching
stored state, pass `--api-key tcgl_…`, `--token <bearer>`, or the matching
`TCGLENSE_API_KEY` / `TCGLENSE_TOKEN` env vars.

```sh
tcglense config url http://localhost:8080          # optional: point at a self-host / local dev
tcglense login --email you@example.com             # prompts for the password
tcglense whoami                                    # GET /api/auth/me
tcglense api-keys create "my laptop" --scope read_write --use-key
tcglense status                                    # show base URL + credential
tcglense logout
```

## One-shot commands

Every command accepts `--json` for machine-readable output (ideal for `jq`
pipelines); the default is human-friendly tables.

```sh
# Catalog (public — no auth needed)
tcglense games
tcglense sets mtg
tcglense set mtg blb --cards -q 't:creature c:g'
tcglense cards mtg -q 'lightning bolt'
tcglense card mtg <card-id>
tcglense prices mtg <card-id> --range 1y
tcglense prints mtg <card-id>
tcglense sealed mtg <card-id>
tcglense products mtg --set blb --sort price --dir desc
tcglense product mtg <product-id> contents

# Collection (auth required)
tcglense collection mtg summary
tcglense collection mtg list -q 'is:foil'
tcglense collection mtg set <card-id> --qty 4 --foil 1
tcglense collection mtg add <card-id> --qty 1
tcglense collection mtg import --provider archidekt --source <url> --mode merge
tcglense collection mtg export --format archidekt -o backup.csv
tcglense collection mtg movers --window week
tcglense collection mtg products list

# Wish list (mirrors the collection card ops)
tcglense wishlist mtg set <card-id> --qty 1

# Decks
tcglense decks mtg list
tcglense decks mtg create "Mono-Green Stompy" --format commander
tcglense decks mtg show <deck-id>
tcglense decks mtg card <deck-id> set <card-id> --section <section-id> --qty 1
tcglense decks mtg export <deck-id> --format moxfield-text

# Public sharing (no auth)
tcglense public alice-0001 profile
tcglense public alice-0001 collection mtg summary
tcglense public alice-0001 decks

# Server / meta
tcglense health
tcglense server-config
tcglense currencies
tcglense openapi -o openapi.json
```

Run `tcglense <command> --help` for the full option list of any command.

## Interactive TUI

Run `tcglense` with no subcommand (or `tcglense tui`) to launch the terminal UI:

```
tcglense
```

- Pick a game → choose **Browse sets**, **Search cards**, **Collection**,
  **Wish list**, **Decks**, or **Account**.
- `↑`/`↓` (or `j`/`k`) move · `Enter`/`→` open · `Esc`/`←`/`⌫` back · `q` quit · `?` help.
- On card lists: `n`/`p` page, and (signed in) `+`/`-` adjust owned counts, `f`/`F`
  the foil count, `w` add to the wish list.
- On collection/wish-list views: `+`/`-`, `f`/`F`, and `r` to remove a holding.

## Global flags

| Flag | Env | Meaning |
|------|-----|---------|
| `--url <URL>` | `TCGLENSE_URL` | API base URL (default `https://tcglense.com`). |
| `--api-key <tcgl_…>` | `TCGLENSE_API_KEY` | Use an API key for this call (not persisted). |
| `--token <bearer>` | `TCGLENSE_TOKEN` | Use a raw bearer token for this call (not persisted). |
| `--config <path>` | `TCGLENSE_CONFIG` | Config file location. |
| `--json` | — | JSON output instead of tables. |

## Development

```sh
cargo fmt --all
cargo clippy --all-targets
cargo build
cargo test
```

Cutting a release: `./scripts/release.sh` bumps the version, tags `vX.Y.Z`, pushes, and
publishes the GitHub Release, whose workflow builds and attaches the per-platform
binaries.

## License

[MIT](./LICENSE). Part of the [TCGLense](https://github.com/PNRxA/tcglense) project.
