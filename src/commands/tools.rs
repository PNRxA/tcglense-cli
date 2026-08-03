//! Tools: the life tracker (`/api/tools/{game}/life/…`).
//!
//! A *session* is one tracked game — a header (name, format, layout, starting life,
//! which counters it tracks) plus its seats, and a full history of every change.
//!
//! Life is always tracked; a game may also track `commander_damage`, `poison`,
//! `energy` and `experience`. Those counters are never stored — the API folds them
//! out of the history, so there is exactly one writer for every number in a game,
//! and [`LifeSessionDetail::counters`] is where the table's derived state arrives.
//!
//! Most writes echo the whole [`LifeSessionDetail`] back, because one change can
//! re-derive many rows (an undo re-folds a seat's chain; removing a seat renumbers
//! the rest). Three do not, and the response type has to match or the write lands
//! but the CLI reports a decode failure: editing the session returns the bare
//! [`LifeSession`], editing a seat returns the bare [`LifePlayer`], and a change
//! returns a [`LifeChange`] — the seat it moved, the counter it left, and the event.

use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};

use super::{Ctx, push_opt};
use crate::models::*;
use crate::output::{self, table};

/// A counter a game can track beyond life. `life` is always tracked and is never
/// one of these — it's what a life change moves when no counter is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Counter {
    /// Damage from one seat's commander to another; every change names its source.
    #[value(name = "commander_damage", alias = "commander-damage", alias = "cmd")]
    CommanderDamage,
    #[value(name = "poison")]
    Poison,
    #[value(name = "energy")]
    Energy,
    #[value(name = "experience", alias = "exp")]
    Experience,
}

impl Counter {
    fn as_str(self) -> &'static str {
        match self {
            Counter::CommanderDamage => "commander_damage",
            Counter::Poison => "poison",
            Counter::Energy => "energy",
            Counter::Experience => "experience",
        }
    }
}

#[derive(Debug, Args)]
pub struct LifeArgs {
    pub game: String,
    #[command(subcommand)]
    pub command: LifeCommand,
}

#[derive(Debug, Subcommand)]
pub enum LifeCommand {
    /// List your tracked games, most-recently-started first.
    List {
        /// Narrow to `active` or `finished` games.
        #[arg(long)]
        status: Option<String>,
        /// How many to return (1..=200; default 50).
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Show one tracked game in full: seats plus the whole life history.
    Show { session_id: i64 },
    /// Start a tracked game (or rematch an earlier one with `--from`).
    Start {
        /// A seat, repeatable in seating order. Either a bare name (`Alice`) or a
        /// name followed by comma-separated attributes:
        /// `deck=<deck-id>`, `commander=<card-id>`, `rotation=0|90|180|270`,
        /// `life=<starting life>` — e.g. `--player 'Alice,deck=12,rotation=90'`.
        /// A seat may name a deck or a commander, not both.
        #[arg(long = "player", value_name = "SPEC")]
        players: Vec<String>,
        /// Rematch: copy the seats, decks, rotations, layout and format of an
        /// earlier game of yours. Explicit options still win over the copy.
        #[arg(long, value_name = "SESSION_ID")]
        from: Option<i64>,
        /// Label for the game.
        #[arg(long)]
        name: Option<String>,
        /// Format the table is playing (e.g. `commander`).
        #[arg(long)]
        format: Option<String>,
        /// Seat layout: rows | facing | facing-solo | sides | sides-solo | grid |
        /// pinwheel (default: fits the player count).
        #[arg(long)]
        layout: Option<String>,
        /// A counter to track beyond life, repeatable. Omit to take the rematched
        /// game's counters, else the format's default (a Commander pod opens with
        /// the damage matrix on).
        #[arg(long = "counter", value_enum, value_name = "COUNTER")]
        counters: Vec<Counter>,
        /// The total each seat starts on.
        #[arg(long)]
        starting_life: Option<i64>,
    },
    /// Relabel a tracked game (works on a finished one too). Each option is
    /// optional; pass an empty string to clear `--name` / `--format`.
    Update {
        session_id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        format: Option<String>,
        /// Seat layout: rows | facing | facing-solo | sides | sides-solo | grid |
        /// pinwheel.
        #[arg(long)]
        layout: Option<String>,
        /// Replace the tracked-counter set, repeatable. Values already recorded
        /// against a counter you drop are kept, not deleted.
        #[arg(long = "counter", value_enum, value_name = "COUNTER")]
        counters: Vec<Counter>,
        /// Stop tracking every counter but life.
        #[arg(long, conflicts_with = "counters")]
        no_counters: bool,
    },
    /// Delete a tracked game, its seats and its whole life history.
    Delete { session_id: i64 },
    /// Record the result and close the game out.
    Finish {
        session_id: i64,
        /// The seat that won (every other seat takes a loss).
        #[arg(long, value_name = "PLAYER_ID", conflicts_with = "draw")]
        winner: Option<i64>,
        /// Record a draw for the whole table.
        #[arg(long)]
        draw: bool,
    },
    /// Seat another player at the table.
    AddPlayer {
        session_id: i64,
        #[arg(long)]
        name: Option<String>,
        /// One of your decks for the game.
        #[arg(long, value_name = "DECK_ID", conflicts_with = "commander")]
        deck: Option<i64>,
        /// The commander they're playing, for a deck you don't have.
        #[arg(long, value_name = "CARD_ID")]
        commander: Option<String>,
        /// Screen rotation: 0 | 90 | 180 | 270.
        #[arg(long)]
        rotation: Option<i64>,
        /// The total this seat starts on (default: the session's).
        #[arg(long)]
        starting_life: Option<i64>,
    },
    /// Replace a seat's name, deck/commander link and rotation. This is a full
    /// replace: an omitted `--deck`/`--commander` unlinks what was there, and an
    /// omitted `--rotation` seats the player upright.
    UpdatePlayer {
        session_id: i64,
        player_id: i64,
        #[arg(long)]
        name: String,
        #[arg(long, value_name = "DECK_ID", conflicts_with = "commander")]
        deck: Option<i64>,
        #[arg(long, value_name = "CARD_ID")]
        commander: Option<String>,
        /// Screen rotation: 0 | 90 | 180 | 270 (default 0).
        #[arg(long)]
        rotation: Option<i64>,
    },
    /// Take a seat off the table (their history goes with them).
    RemovePlayer { session_id: i64, player_id: i64 },
    /// Set the seat order — exactly the session's seat ids, each once.
    Reorder {
        session_id: i64,
        #[arg(required = true, value_name = "PLAYER_ID")]
        player_ids: Vec<i64>,
    },
    /// Move one of a seat's numbers and record it in the history.
    Adjust {
        session_id: i64,
        player_id: i64,
        /// A relative change, e.g. `-3`.
        #[arg(long, allow_negative_numbers = true, conflicts_with = "life")]
        delta: Option<i64>,
        /// An absolute correction — the value the counter is actually on.
        #[arg(long, allow_negative_numbers = true)]
        life: Option<i64>,
        /// Which counter to move; omit for the seat's life total. The game has to
        /// be tracking it (see `life start --counter`).
        #[arg(long, value_enum, value_name = "COUNTER")]
        counter: Option<Counter>,
        /// For `commander_damage`, the seat whose commander dealt it (required
        /// there, refused for every other counter).
        #[arg(long = "from", value_name = "PLAYER_ID")]
        source: Option<i64>,
    },
    /// Undo one recorded change, from anywhere in the history.
    Undo { session_id: i64, event_id: i64 },
    /// Per-deck win/loss record across your finished tracked games.
    Records {
        /// Narrow to a single deck.
        #[arg(long, value_name = "DECK_ID")]
        deck: Option<i64>,
    },
}

pub async fn life(ctx: &Ctx, args: LifeArgs) -> Result<()> {
    let base = format!("/api/tools/{}/life", args.game);
    match args.command {
        LifeCommand::List { status, limit } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "status", &status);
            push_opt(&mut q, "limit", &limit);
            let body: DataBody<Vec<LifeSession>> =
                ctx.client.get_json(&format!("{base}/sessions"), &q).await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else if body.data.is_empty() {
                println!("No tracked games.");
            } else {
                sessions_table(&body.data);
            }
            Ok(())
        }

        LifeCommand::Show { session_id } => {
            let detail: LifeSessionDetail = ctx
                .client
                .get_json(&format!("{base}/sessions/{session_id}"), &[])
                .await?;
            report(ctx, &detail, true)
        }

        LifeCommand::Start {
            players,
            from,
            name,
            format,
            layout,
            counters,
            starting_life,
        } => {
            if players.is_empty() && from.is_none() {
                bail!(
                    "give the table with --player (repeatable), or --from <session-id> to rematch"
                );
            }
            let seats: Vec<serde_json::Value> = players
                .iter()
                .map(|spec| parse_player_spec(spec))
                .collect::<Result<_>>()?;
            let body = serde_json::json!({
                "players": seats,
                "from_session_id": from,
                "name": name,
                "format": format,
                "layout": layout,
                "counters": counter_list(&counters),
                "starting_life": starting_life,
            });
            let detail: LifeSessionDetail = ctx
                .client
                .post_json(&format!("{base}/sessions"), body)
                .await?;
            ctx.printer
                .note(format!("Started tracked game {}.", detail.session.id));
            report(ctx, &detail, false)
        }

        LifeCommand::Update {
            session_id,
            name,
            format,
            layout,
            counters,
            no_counters,
        } => {
            // An empty `--counter` list means "leave the set alone"; turning them
            // all off is the explicit `--no-counters`.
            let counters = if no_counters {
                Some(Vec::new())
            } else {
                counter_list(&counters)
            };
            if name.is_none() && format.is_none() && layout.is_none() && counters.is_none() {
                bail!(
                    "provide at least one of --name / --format / --layout / --counter / --no-counters"
                );
            }
            let body = serde_json::json!({
                "name": name,
                "format": format,
                "layout": layout,
                "counters": counters,
            });
            // Editing the session echoes the bare session, not the detail envelope.
            let session: LifeSession = ctx
                .client
                .put_json(&format!("{base}/sessions/{session_id}"), body)
                .await?;
            report_session(ctx, &session)
        }

        LifeCommand::Delete { session_id } => {
            ctx.client
                .delete(&format!("{base}/sessions/{session_id}"))
                .await?;
            ctx.printer
                .note(format!("Deleted tracked game {session_id}."));
            Ok(())
        }

        LifeCommand::Finish {
            session_id,
            winner,
            draw,
        } => {
            if winner.is_none() && !draw {
                bail!("say who won with --winner <player-id>, or --draw for a drawn game");
            }
            let body = serde_json::json!({ "winner_player_id": winner });
            let detail: LifeSessionDetail = ctx
                .client
                .post_json(&format!("{base}/sessions/{session_id}/finish"), body)
                .await?;
            report(ctx, &detail, false)
        }

        LifeCommand::AddPlayer {
            session_id,
            name,
            deck,
            commander,
            rotation,
            starting_life,
        } => {
            let body = serde_json::json!({
                "name": name,
                "deck_id": deck,
                "commander_card_id": commander,
                "rotation": rotation,
                "starting_life": starting_life,
            });
            let detail: LifeSessionDetail = ctx
                .client
                .post_json(&format!("{base}/sessions/{session_id}/players"), body)
                .await?;
            report(ctx, &detail, false)
        }

        LifeCommand::UpdatePlayer {
            session_id,
            player_id,
            name,
            deck,
            commander,
            rotation,
        } => {
            let body = serde_json::json!({
                "name": name,
                "deck_id": deck,
                "commander_card_id": commander,
                "rotation": rotation.unwrap_or(0),
            });
            // Editing a seat echoes just that seat.
            let seat: LifePlayer = ctx
                .client
                .put_json(
                    &format!("{base}/sessions/{session_id}/players/{player_id}"),
                    body,
                )
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&seat)?;
            } else {
                players_table(std::slice::from_ref(&seat));
            }
            Ok(())
        }

        LifeCommand::RemovePlayer {
            session_id,
            player_id,
        } => {
            let detail: LifeSessionDetail = ctx
                .client
                .delete_json(&format!("{base}/sessions/{session_id}/players/{player_id}"))
                .await?;
            report(ctx, &detail, false)
        }

        LifeCommand::Reorder {
            session_id,
            player_ids,
        } => {
            let body = serde_json::json!({ "player_ids": player_ids });
            let detail: LifeSessionDetail = ctx
                .client
                .put_json(
                    &format!("{base}/sessions/{session_id}/players/reorder"),
                    body,
                )
                .await?;
            report(ctx, &detail, false)
        }

        LifeCommand::Adjust {
            session_id,
            player_id,
            delta,
            life,
            counter,
            source,
        } => {
            if delta.is_none() == life.is_none() {
                bail!("pass exactly one of --delta <change> or --life <total>");
            }
            // 21 damage is per commander, so a sourceless damage row couldn't
            // decide anything; every other counter refuses a source outright.
            match (counter, source) {
                (Some(Counter::CommanderDamage), None) => {
                    bail!(
                        "commander_damage needs --from <player-id> — the seat whose commander dealt it"
                    );
                }
                (c, Some(_)) if c != Some(Counter::CommanderDamage) => {
                    bail!("--from only applies to --counter commander_damage");
                }
                _ => {}
            }
            let body = serde_json::json!({
                "delta": delta,
                "life": life,
                "counter": counter.map(Counter::as_str),
                "source_player_id": source,
            });
            // A change echoes only the seat it moved, the counter it left, and the
            // event recorded.
            let change: LifeChange = ctx
                .client
                .post_json(
                    &format!("{base}/sessions/{session_id}/players/{player_id}/life"),
                    body,
                )
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&change)?;
            } else {
                let e = &change.event;
                println!(
                    "{} (seat {}) {}: {} → {}   ({:+} · {} · event {})",
                    change.player.name,
                    change.player.id,
                    counter_label(e),
                    e.life_after - e.delta,
                    e.life_after,
                    e.delta,
                    e.kind,
                    e.id
                );
            }
            Ok(())
        }

        LifeCommand::Undo {
            session_id,
            event_id,
        } => {
            let detail: LifeSessionDetail = ctx
                .client
                .delete_json(&format!("{base}/sessions/{session_id}/events/{event_id}"))
                .await?;
            report(ctx, &detail, true)
        }

        LifeCommand::Records { deck } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "deck_id", &deck);
            let body: DataBody<Vec<LifeDeckRecord>> =
                ctx.client.get_json(&format!("{base}/decks"), &q).await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else if body.data.is_empty() {
                println!("No decks played yet.");
            } else {
                records_table(&body.data);
            }
            Ok(())
        }
    }
}

// -- request helpers --------------------------------------------------------

/// Turn a repeated `--counter` into the wire list, de-duplicated in the order
/// given. No flags at all is `None` — "leave the tracked set to the server".
fn counter_list(counters: &[Counter]) -> Option<Vec<&'static str>> {
    if counters.is_empty() {
        return None;
    }
    let mut out: Vec<&'static str> = Vec::new();
    for c in counters {
        if !out.contains(&c.as_str()) {
            out.push(c.as_str());
        }
    }
    Some(out)
}

/// Parse a `--player` spec: a name, optionally followed by comma-separated
/// `key=value` attributes (`deck`, `commander`, `rotation`, `life`). A bare
/// leading token is the seat's name, so `Alice`, `Alice,deck=12` and
/// `name=Alice,deck=12` all describe the same seat. Only the fields given are
/// sent, so the server fills the rest in (an unnamed seat becomes `Player {n}`).
fn parse_player_spec(spec: &str) -> Result<serde_json::Value> {
    let mut seat = serde_json::Map::new();
    for (i, part) in spec.split(',').enumerate() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key, value)) = part.split_once('=') else {
            if i == 0 {
                seat.insert("name".into(), part.into());
                continue;
            }
            bail!("player spec `{spec}`: `{part}` is not a key=value attribute");
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "name" => {
                seat.insert("name".into(), value.into());
            }
            "deck" => {
                seat.insert("deck_id".into(), parse_num(spec, key, value)?.into());
            }
            "commander" => {
                seat.insert("commander_card_id".into(), value.into());
            }
            "rotation" => {
                seat.insert("rotation".into(), parse_num(spec, key, value)?.into());
            }
            "life" => {
                seat.insert("starting_life".into(), parse_num(spec, key, value)?.into());
            }
            other => bail!(
                "player spec `{spec}`: unknown attribute `{other}` \
                 (expected name / deck / commander / rotation / life)"
            ),
        }
    }
    if seat.contains_key("deck_id") && seat.contains_key("commander_card_id") {
        bail!("player spec `{spec}`: a seat may name a deck or a commander, not both");
    }
    Ok(serde_json::Value::Object(seat))
}

fn parse_num(spec: &str, key: &str, value: &str) -> Result<i64> {
    value
        .parse::<i64>()
        .map_err(|_| anyhow::anyhow!("player spec `{spec}`: `{key}` wants a number, got `{value}`"))
}

// -- rendering --------------------------------------------------------------

/// Render a session write/read. `with_history` also prints the life history (the
/// detail reads); the writes print just the table's new state.
fn report(ctx: &Ctx, detail: &LifeSessionDetail, with_history: bool) -> Result<()> {
    if ctx.printer.json {
        return ctx.printer.json(detail);
    }
    print_session(&detail.session);
    if !detail.counters.is_empty() {
        counters_table(&detail.session.players, &detail.counters);
    }
    if with_history {
        if detail.events.is_empty() {
            println!("(no changes recorded yet)");
        } else {
            events_table(&detail.session.players, &detail.events);
        }
    }
    Ok(())
}

/// Render a bare session — the shape the session edit echoes back, which carries
/// no history to print.
fn report_session(ctx: &Ctx, session: &LifeSession) -> Result<()> {
    if ctx.printer.json {
        return ctx.printer.json(session);
    }
    print_session(session);
    Ok(())
}

fn print_session(s: &LifeSession) {
    println!(
        "Game {} · {} · {} · layout {} · started {}",
        s.id, s.game, s.status, s.layout, s.started_at
    );
    println!(
        "  name: {}   format: {}   starting life: {}{}",
        s.name.as_deref().unwrap_or("—"),
        s.format.as_deref().unwrap_or("—"),
        s.starting_life,
        match &s.finished_at {
            Some(f) => format!("   finished: {f}"),
            None => String::new(),
        }
    );
    if !s.counters.is_empty() {
        println!("  tracking: life, {}", s.counters.join(", "));
    }
    players_table(&s.players);
}

/// Where every non-life counter stands, folded out of the history by the API. A
/// commander-damage row is per source seat, so it names who dealt it.
fn counters_table(players: &[LifePlayer], counters: &[LifeCounter]) {
    let mut t = table(&["Seat", "Counter", "From", "Value"]);
    for c in counters {
        t.add_row(vec![
            seat_name(players, c.player_id),
            c.counter.clone(),
            match c.source_player_id {
                Some(id) => seat_name(players, id),
                None => "—".to_string(),
            },
            c.value.to_string(),
        ]);
    }
    println!("{t}");
}

/// A seat's name, or its bare id once it has left the table (the history outlives
/// the seat).
fn seat_name(players: &[LifePlayer], player_id: i64) -> String {
    players
        .iter()
        .find(|p| p.id == player_id)
        .map(|p| output::truncate(&p.name, 20))
        .unwrap_or_else(|| format!("#{player_id}"))
}

/// What an event moved, naming the source seat for commander damage.
fn counter_label(e: &LifeEvent) -> String {
    match e.source_player_id {
        Some(id) => format!("{} from #{id}", e.counter),
        None => e.counter.clone(),
    }
}

fn sessions_table(sessions: &[LifeSession]) {
    let mut t = table(&["ID", "Name", "Status", "Format", "Seats", "Started"]);
    for s in sessions {
        t.add_row(vec![
            s.id.to_string(),
            output::truncate(s.name.as_deref().unwrap_or("—"), 28),
            s.status.clone(),
            output::dash(&s.format),
            s.players.len().to_string(),
            s.started_at.clone(),
        ]);
    }
    println!("{t}");
}

fn players_table(players: &[LifePlayer]) {
    let mut t = table(&[
        "Seat", "ID", "Name", "Life", "Start", "Playing", "Rot", "Result",
    ]);
    for p in players {
        t.add_row(vec![
            p.position.to_string(),
            p.id.to_string(),
            output::truncate(&p.name, 24),
            p.life.to_string(),
            p.starting_life.to_string(),
            output::truncate(&playing(p), 32),
            format!("{}°", p.rotation),
            p.result.clone(),
        ]);
    }
    println!("{t}");
}

/// What a seat brought: their linked deck, else their commander, else nothing.
fn playing(p: &LifePlayer) -> String {
    if let Some(deck) = &p.deck_name {
        match p.deck_id {
            Some(id) => format!("{deck} (deck {id})"),
            None => deck.clone(),
        }
    } else if let Some(cmd) = &p.commander_name {
        cmd.clone()
    } else {
        "—".to_string()
    }
}

fn events_table(players: &[LifePlayer], events: &[LifeEvent]) {
    let mut t = table(&[
        "ID", "Seat", "Counter", "From", "Kind", "Delta", "After", "When",
    ]);
    for e in events {
        t.add_row(vec![
            e.id.to_string(),
            seat_name(players, e.player_id),
            e.counter.clone(),
            match e.source_player_id {
                Some(id) => seat_name(players, id),
                None => "—".to_string(),
            },
            e.kind.clone(),
            format!("{:+}", e.delta),
            e.life_after.to_string(),
            e.created_at.clone(),
        ]);
    }
    println!("{t}");
}

fn records_table(records: &[LifeDeckRecord]) {
    let mut t = table(&[
        "Deck",
        "Name",
        "Games",
        "W",
        "L",
        "D",
        "Win rate",
        "Last played",
    ]);
    for r in records {
        t.add_row(vec![
            r.deck_id.to_string(),
            output::truncate(&r.deck_name, 32),
            r.games.to_string(),
            r.wins.to_string(),
            r.losses.to_string(),
            r.draws.to_string(),
            r.win_rate
                .map(|w| format!("{:.0}%", w * 100.0))
                .unwrap_or_else(|| "—".to_string()),
            output::dash(&r.last_played_at),
        ]);
    }
    println!("{t}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_spec_is_a_name() {
        let seat = parse_player_spec("Alice").unwrap();
        assert_eq!(seat["name"], "Alice");
        assert_eq!(seat.as_object().unwrap().len(), 1);
    }

    #[test]
    fn attributes_follow_the_name() {
        let seat = parse_player_spec("Alice, deck=12, rotation=90, life=40").unwrap();
        assert_eq!(seat["name"], "Alice");
        assert_eq!(seat["deck_id"], 12);
        assert_eq!(seat["rotation"], 90);
        assert_eq!(seat["starting_life"], 40);
    }

    #[test]
    fn name_may_be_given_as_an_attribute() {
        let seat = parse_player_spec("name=Bob,commander=abc-123").unwrap();
        assert_eq!(seat["name"], "Bob");
        assert_eq!(seat["commander_card_id"], "abc-123");
    }

    #[test]
    fn an_empty_spec_leaves_every_field_to_the_server() {
        let seat = parse_player_spec("").unwrap();
        assert!(seat.as_object().unwrap().is_empty());
    }

    #[test]
    fn deck_and_commander_are_mutually_exclusive() {
        assert!(parse_player_spec("Alice,deck=1,commander=abc").is_err());
    }

    #[test]
    fn no_counter_flags_leave_the_tracked_set_alone() {
        assert!(counter_list(&[]).is_none());
    }

    #[test]
    fn counters_keep_their_order_and_de_duplicate() {
        let out = counter_list(&[
            Counter::Poison,
            Counter::CommanderDamage,
            Counter::Poison,
            Counter::Experience,
        ])
        .unwrap();
        assert_eq!(out, vec!["poison", "commander_damage", "experience"]);
    }

    #[test]
    fn unknown_attributes_and_bad_numbers_are_rejected() {
        assert!(parse_player_spec("Alice,colour=red").is_err());
        assert!(parse_player_spec("Alice,deck=twelve").is_err());
        // A bare token is only a name in first position.
        assert!(parse_player_spec("Alice,Bob").is_err());
    }
}
