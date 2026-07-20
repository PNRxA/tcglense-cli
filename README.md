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
your platform, extract it (`tar -xzf …` on Unix), and put `tcglense` on your `PATH`.
On macOS the first launch may be blocked by Gatekeeper — see
[macOS Gatekeeper warning](#macos-gatekeeper-warning) below.

**From source** (needs [Rust](https://rustup.rs/)):

```sh
cargo install --git https://github.com/PNRxA/tcglense-cli   # installs `tcglense`
# or, in a clone:
cargo build --release        # binary at target/release/tcglense
```

### macOS Gatekeeper warning

The macOS release binaries aren't code-signed with an Apple Developer ID or notarized by
Apple. When you download a release archive **in a browser**, macOS tags it with a
`com.apple.quarantine` flag, and on macOS 15 (Sequoia) or later the first launch of the
un-notarized binary is blocked with:

> Apple could not verify “tcglense” is free of malware that may harm your Mac or
> compromise your privacy.

(macOS 14 and earlier word it *“… can't be opened because the developer cannot be
verified.”*) This is Gatekeeper reacting to the missing notarization, not a sign that the
binary is actually unsafe — the source and the release workflow that builds it are public.
Any one of these gets you running:

- **Extract in a terminal (simplest).** The quarantine flag sits on the downloaded
  `.tar.gz`; the command-line `tar` tool doesn't copy it onto the extracted files (only
  Finder's Archive Utility does), so the binary is never quarantined and Gatekeeper stays
  quiet:

  ```sh
  tar -xzf ~/Downloads/tcglense-*-aarch64-apple-darwin.tar.gz
  ./tcglense --version
  ```

- **Already extracted by double-clicking?** Strip the quarantine flag off the binary:

  ```sh
  xattr -d com.apple.quarantine ./tcglense   # or: xattr -c ./tcglense
  ```

  (`xattr -d` prints a harmless “No such xattr” if the flag was never set; `xattr -c`
  clears every extended attribute and never complains.)

- **Prefer clicking?** Try to run `./tcglense` once so it gets blocked, then open **System
  Settings → Privacy & Security**, scroll to **Security**, and click **Open Anyway** next
  to the “tcglense was blocked” message (authenticate, then run it again). On Sequoia the
  old right-click → *Open* shortcut was removed, so this button is the GUI route.

- **Build or self-update instead.** Binaries macOS never sees as “downloaded from the
  internet” are never quarantined, so these skip the warning entirely:
  `cargo install --git https://github.com/PNRxA/tcglense-cli` (compiled locally), and
  `tcglense update`, whose self-updater fetches over HTTPS rather than through a browser.

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
| **Web session** (browser sign-in) | `tcglense login` | Opens your browser to the TCGLense sign-in page — the password is entered on the website, never in the terminal — and captures the result on a temporary `127.0.0.1` loopback listener (the OAuth 2.0 native-app flow, RFC 8252, with PKCE). Stores a short-lived access token **and** the opaque refresh token; the CLI silently refreshes on expiry. This is the full-access credential (it can manage API keys). |
| **API key** (`tcgl_…`) | `tcglense auth key tcgl_…` | Programmatic, per-user; scoped `read` or `read_write`. A read-only key gets `403` on writes. Mint one with `tcglense api-keys create`. Best for headless/CI where no browser is available. |

Credentials are stored in `~/.config/tcglense/config.json` (mode `0600`; override
with `--config <path>` or `$TCGLENSE_CONFIG`). For a one-off call without touching
stored state, pass `--api-key tcgl_…`, `--token <bearer>`, or the matching
`TCGLENSE_API_KEY` / `TCGLENSE_TOKEN` env vars.

```sh
tcglense login                                     # opens the browser to sign in
tcglense login --no-browser                        # print the URL instead (headless / SSH)
tcglense whoami                                    # GET /api/auth/me
tcglense api-keys create "my laptop" --scope read_write --use-key
tcglense status                                    # show base URL + credential
tcglense logout
```

`tcglense login` opens `<base-url>/cli-login`, so it targets whichever origin
`--url` / `TCGLENSE_URL` points at (the production site by default). For local dev,
point it at the web origin that serves the SPA and proxies `/api` (e.g.
`tcglense config url http://localhost:5173`), not the bare API port. On a headless
box, use `--no-browser` and open the printed URL on another device, or authenticate
with an API key instead.

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
tcglense rulings mtg <card-id>
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
tcglense wishlist mtg visibility set true            # share your wish list publicly

# Decks
tcglense decks mtg list
tcglense decks mtg create "Mono-Green Stompy" --format commander
tcglense decks mtg show <deck-id>
tcglense decks mtg card <deck-id> set <card-id> --section <section-id> --qty 1
tcglense decks mtg export <deck-id> --format moxfield-text
tcglense decks mtg needed --mode card                # cards your decks want but you don't own

# Public sharing (reads need no auth; `deck … copy` does)
tcglense public alice-0001 profile
tcglense public alice-0001 collection mtg summary
tcglense public alice-0001 collection mtg products list
tcglense public alice-0001 wishlist mtg summary
tcglense public alice-0001 decks
tcglense public alice-0001 deck <deck-id> copy       # clone a public deck into your own

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
