//! Deck commands: a container surface (many decks per game) with folders, sections,
//! per-card edits, import/export, and public sharing.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};

use super::{Ctx, push_opt};
use crate::models::*;
use crate::output::{self, cards_table, decks_table, table};

#[derive(Debug, Args)]
pub struct DecksArgs {
    pub game: String,
    #[command(subcommand)]
    pub command: DecksCommand,
}

#[derive(Debug, Subcommand)]
pub enum DecksCommand {
    /// List your decks.
    List,
    /// Show one deck in full (sections + cards).
    Show { deck_id: i64 },
    /// Create a new deck (seeded with default sections).
    Create {
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        folder: Option<i64>,
    },
    /// Replace a deck's editable metadata.
    Update {
        deck_id: i64,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        format: Option<String>,
    },
    /// Delete a deck.
    Delete { deck_id: i64 },
    /// Import a deck from a provider URL/id or an uploaded file.
    Import {
        #[arg(long, value_enum)]
        provider: DeckProvider,
        /// A public deck URL or id (live import).
        #[arg(long, conflicts_with = "file")]
        source: Option<String>,
        /// A deck-list file to upload.
        #[arg(long, conflicts_with = "source")]
        file: Option<PathBuf>,
        /// Uploaded file format (with --file).
        #[arg(long, value_enum, default_value_t = FileFormat::Csv)]
        file_format: FileFormat,
        /// Name for the new deck.
        #[arg(long)]
        name: Option<String>,
        /// Keep generic Mainboard rows exactly (don't auto-file by type).
        #[arg(long)]
        no_auto_categorize: bool,
    },
    /// Export a deck as a provider-shaped list.
    Export {
        deck_id: i64,
        #[arg(long, value_enum, default_value_t = DeckExportFormat::Archidekt)]
        format: DeckExportFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Manage deck folders.
    Folders {
        #[command(subcommand)]
        command: FolderCommand,
    },
    /// File a deck under a folder (or loosen it with no id).
    MoveToFolder {
        deck_id: i64,
        folder_id: Option<i64>,
    },
    /// Manage a deck's sections.
    Sections {
        deck_id: i64,
        #[command(subcommand)]
        command: SectionCommand,
    },
    /// Edit a card within a deck.
    Card {
        deck_id: i64,
        #[command(subcommand)]
        command: DeckCardCommand,
    },
    /// Enable/disable public sharing of a deck.
    Visibility {
        deck_id: i64,
        #[arg(action = clap::ArgAction::Set)]
        public: bool,
    },
    /// List cards your decks collectively want more copies of than you own.
    Needed {
        /// `card` counts any printing of a gameplay card; `printing` reports the
        /// exact missing printing.
        #[arg(long, value_enum, default_value_t = NeededMode::Card)]
        mode: NeededMode,
        /// Scope the list to one deck: what *it* still needs, as its share of the
        /// shortfall across every deck (a copy two decks share is never counted as
        /// owned by both).
        #[arg(long, value_name = "DECK_ID")]
        deck: Option<i64>,
    },
    /// The needed list as bulk-buy rows — what a store's bulk-entry page takes.
    BuyList {
        /// `card` counts any printing of a gameplay card; `printing` reports the
        /// exact missing printing.
        #[arg(long, value_enum, default_value_t = NeededMode::Card)]
        mode: NeededMode,
        /// Scope the rows to one deck, exactly as `needed --deck` does.
        #[arg(long, value_name = "DECK_ID")]
        deck: Option<i64>,
    },
    /// Duplicate one of your decks — same sections and cards, private, same folder.
    Copy { deck_id: i64 },
    /// Add every card of the deck to your collection. Not idempotent: it adds on top
    /// of what you own, so a second run adds a second copy.
    AddToCollection { deck_id: i64 },
    /// Check a deck against its own format: offending cards + construction breaches.
    Legality { deck_id: i64 },
    /// Estimate where a Commander deck sits on the 1–5 bracket ladder.
    Bracket { deck_id: i64 },
    /// Composition (curve, colours, types) plus the draw odds for one card.
    Stats {
        deck_id: i64,
        #[command(flatten)]
        args: StatsArgs,
    },
    /// Shuffle the library and deal a sample opening hand.
    Goldfish {
        deck_id: i64,
        #[command(flatten)]
        args: GoldfishArgs,
    },
    /// The tokens and emblems the deck's cards make — what to bring besides the deck.
    Tokens { deck_id: i64 },
    /// The combos the deck can assemble, and the ones it's one card away from.
    Combos { deck_id: i64 },
    /// The deck's colour requirements against the sources its library produces.
    Mana { deck_id: i64 },
    /// Where the deck's value is: every row priced, with its cheapest printing.
    Pricing { deck_id: i64 },
    /// Ramp, draw, removal, wipes, counters, tutors, recursion and protection counts.
    Roles { deck_id: i64 },
    /// Cards you own that this deck could play, grouped by the role they fill.
    Suggestions { deck_id: i64 },
    /// Compare two of your decks, card by card and section by section.
    Diff {
        /// The base deck.
        deck_id: i64,
        /// The deck to compare it with.
        other_id: i64,
    },
    /// List your decks that contain a card (any printing of it), latest edit first.
    Containing {
        /// External card id.
        card_id: String,
    },
}

/// Options shared by the private and public deck-analytics reads.
#[derive(Debug, Args)]
pub struct StatsArgs {
    /// Section ids to use as the shuffled library (comma-separated). Omit for the
    /// default selection — everything that isn't a maybeboard, command zone or
    /// sideboard; pass an empty value for none.
    #[arg(long, value_name = "IDS", allow_hyphen_values = true)]
    pub sections: Option<String>,
    /// Card *name* to compute draw odds for (default: the most-copied card).
    #[arg(long, value_name = "NAME")]
    pub card: Option<String>,
    /// How many cards the headline probability assumes were seen (default 7).
    #[arg(long, value_name = "N")]
    pub cards_seen: Option<i64>,
}

/// Options shared by the private and public goldfish reads. The whole hand is a
/// function of these, so the same values always deal the same cards.
#[derive(Debug, Args)]
pub struct GoldfishArgs {
    /// Shuffle seed; omit for a fresh random one (the result echoes it back).
    #[arg(long)]
    pub seed: Option<i64>,
    /// How many London mulligans were taken — each costs a card to the bottom.
    #[arg(long)]
    pub mulligans: Option<i64>,
    /// Card ids put on the bottom (comma-separated), at most one per mulligan.
    #[arg(long, value_name = "CARD_IDS")]
    pub bottom: Option<String>,
    /// Cards drawn after the opening hand (the draw step).
    #[arg(long)]
    pub draws: Option<i64>,
    /// Opening hand size (default 7).
    #[arg(long)]
    pub opening: Option<i64>,
    /// Section ids to shuffle (comma-separated); omit for the default library.
    #[arg(long, value_name = "IDS", allow_hyphen_values = true)]
    pub sections: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum NeededMode {
    Card,
    Printing,
}

impl NeededMode {
    fn as_str(self) -> &'static str {
        match self {
            NeededMode::Card => "card",
            NeededMode::Printing => "printing",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum FolderCommand {
    List,
    Create { name: String },
    Rename { folder_id: i64, name: String },
    Delete { folder_id: i64 },
}

#[derive(Debug, Subcommand)]
pub enum SectionCommand {
    /// Add a custom section.
    Add {
        name: String,
        /// File it as a maybeboard: its cards sit outside the deck proper and are
        /// left out of the summary, legality, analytics and the needed list.
        #[arg(long)]
        maybeboard: bool,
    },
    /// Rename, reposition and/or flip a section's maybeboard flag.
    Update {
        section_id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        position: Option<i64>,
        /// Move the section in or out of the maybeboard (`true`/`false`).
        #[arg(long, action = clap::ArgAction::Set)]
        maybeboard: Option<bool>,
    },
    /// Set the full section order.
    Reorder { section_ids: Vec<i64> },
    /// Delete a section (its cards move to the first remaining one).
    Delete { section_id: i64 },
}

#[derive(Debug, Subcommand)]
pub enum DeckCardCommand {
    /// Set a card's absolute counts in a section (both zero removes it there).
    Set {
        card_id: String,
        #[arg(long)]
        section: i64,
        #[arg(long, default_value_t = 0)]
        qty: i64,
        #[arg(long, default_value_t = 0)]
        foil: i64,
    },
    /// Move a card between two sections.
    Move {
        card_id: String,
        #[arg(long)]
        from: i64,
        #[arg(long)]
        to: i64,
    },
    /// Swap a card for another printing in a section.
    Printing {
        card_id: String,
        #[arg(long)]
        to_card: String,
        #[arg(long)]
        section: i64,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DeckProvider {
    Archidekt,
    Moxfield,
}

impl DeckProvider {
    fn as_str(self) -> &'static str {
        match self {
            DeckProvider::Archidekt => "archidekt",
            DeckProvider::Moxfield => "moxfield",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FileFormat {
    Csv,
    Text,
}

impl FileFormat {
    fn as_str(self) -> &'static str {
        match self {
            FileFormat::Csv => "csv",
            FileFormat::Text => "text",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DeckExportFormat {
    Archidekt,
    Moxfield,
    #[value(name = "moxfield-text")]
    MoxfieldText,
}

impl DeckExportFormat {
    fn as_str(self) -> &'static str {
        match self {
            DeckExportFormat::Archidekt => "archidekt",
            DeckExportFormat::Moxfield => "moxfield",
            DeckExportFormat::MoxfieldText => "moxfield-text",
        }
    }
}

pub async fn run(ctx: &Ctx, args: DecksArgs) -> Result<()> {
    let base = format!("/api/decks/{}", args.game);
    match args.command {
        DecksCommand::List => {
            let body: DataBody<Vec<Deck>> = ctx.client.get_json(&base, &[]).await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else if body.data.is_empty() {
                println!("No decks.");
            } else {
                decks_table(&body.data);
            }
        }
        DecksCommand::Show { deck_id } => {
            let deck: DeckDetail = ctx
                .client
                .get_json(&format!("{base}/{deck_id}"), &[])
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&deck)?;
            } else {
                print_deck_detail(&deck);
            }
        }
        DecksCommand::Create {
            name,
            description,
            format,
            folder,
        } => {
            let body = serde_json::json!({
                "name": name,
                "description": description,
                "format": format,
                "folder_id": folder,
            });
            let deck: DeckDetail = ctx.client.post_json(&base, body).await?;
            if ctx.printer.json {
                ctx.printer.json(&deck)?;
            } else {
                println!("Created deck '{}' (id {}).", deck.name, deck.id);
            }
        }
        DecksCommand::Update {
            deck_id,
            name,
            description,
            format,
        } => {
            let body = serde_json::json!({
                "name": name,
                "description": description,
                "format": format,
            });
            let deck: Deck = ctx
                .client
                .put_json(&format!("{base}/{deck_id}"), body)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&deck)?;
            } else {
                println!("Updated deck '{}'.", deck.name);
            }
        }
        DecksCommand::Delete { deck_id } => {
            ctx.client.delete(&format!("{base}/{deck_id}")).await?;
            ctx.printer.note(format!("Deleted deck {deck_id}."));
        }
        DecksCommand::Import {
            provider,
            source,
            file,
            file_format,
            name,
            no_auto_categorize,
        } => {
            let (source_val, contents_val, format_val) = match (source, file) {
                (Some(src), None) => (Some(src), None, None),
                (None, Some(path)) => {
                    let text = std::fs::read_to_string(&path)?;
                    (None, Some(text), Some(file_format.as_str()))
                }
                (Some(_), Some(_)) => bail!("provide only one of --source or --file"),
                (None, None) => bail!("provide --source <url/id> or --file <path>"),
            };
            let body = serde_json::json!({
                "provider": provider.as_str(),
                "source": source_val,
                "contents": contents_val,
                "format": format_val,
                "name": name,
                "auto_categorize": !no_auto_categorize,
            });
            let resp: DeckImportResponse = ctx
                .client
                .post_json(&format!("{base}/import"), body)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&resp)?;
            } else {
                println!(
                    "Imported '{}' (id {}): {} rows, {} matched, {} unmatched.",
                    resp.deck.name,
                    resp.deck.id,
                    resp.total_rows,
                    resp.matched_cards,
                    resp.unmatched_cards
                );
                if !resp.unmatched_sample.is_empty() {
                    println!("  unmatched e.g. {}", resp.unmatched_sample.join(", "));
                }
            }
        }
        DecksCommand::Export {
            deck_id,
            format,
            output,
        } => {
            let csv = ctx
                .client
                .get_text(
                    &format!("{base}/{deck_id}/export"),
                    &[("format", format.as_str().to_string())],
                )
                .await?;
            match output {
                Some(p) => {
                    std::fs::write(&p, csv.as_bytes())?;
                    ctx.printer.note(format!("Wrote deck to {}.", p.display()));
                }
                None => print!("{csv}"),
            }
        }
        DecksCommand::Folders { command } => folders(ctx, &base, command).await?,
        DecksCommand::MoveToFolder { deck_id, folder_id } => {
            let body = serde_json::json!({ "folder_id": folder_id });
            let deck: Deck = ctx
                .client
                .put_json(&format!("{base}/{deck_id}/folder"), body)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&deck)?;
            } else {
                match deck.folder_id {
                    Some(f) => println!("Deck '{}' filed under folder {f}.", deck.name),
                    None => println!("Deck '{}' loosened (no folder).", deck.name),
                }
            }
        }
        DecksCommand::Sections { deck_id, command } => {
            sections(ctx, &base, deck_id, command).await?
        }
        DecksCommand::Card { deck_id, command } => deck_card(ctx, &base, deck_id, command).await?,
        DecksCommand::Needed { mode, deck } => needed(ctx, &base, mode, deck).await?,
        DecksCommand::BuyList { mode, deck } => {
            let mut q: Vec<(&'static str, String)> = vec![("mode", mode.as_str().to_string())];
            push_opt(&mut q, "deck_id", &deck);
            let b: BuyList = ctx
                .client
                .get_json(&format!("{base}/needed/buy-list"), &q)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&b)?;
            } else {
                output::buy_list(&b, &ctx.printer);
            }
        }
        DecksCommand::Copy { deck_id } => {
            let d: Deck = ctx
                .client
                .post_json(&format!("{base}/{deck_id}/copy"), serde_json::json!({}))
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&d)?;
            } else {
                println!(
                    "Copied '{}' as deck {} (private, same folder).",
                    d.name, d.id
                );
            }
        }
        DecksCommand::AddToCollection { deck_id } => {
            add_to_collection(ctx, &format!("{base}/{deck_id}/collection")).await?
        }
        DecksCommand::Legality { deck_id } => legality(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Bracket { deck_id } => bracket(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Stats { deck_id, args } => {
            stats(ctx, &format!("{base}/{deck_id}"), args).await?
        }
        DecksCommand::Goldfish { deck_id, args } => {
            goldfish(ctx, &format!("{base}/{deck_id}"), args).await?
        }
        DecksCommand::Tokens { deck_id } => tokens(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Combos { deck_id } => combos(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Mana { deck_id } => mana(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Pricing { deck_id } => pricing(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Roles { deck_id } => roles(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Suggestions { deck_id } => {
            suggestions(ctx, &format!("{base}/{deck_id}")).await?
        }
        DecksCommand::Diff { deck_id, other_id } => {
            diff(ctx, &format!("{base}/{deck_id}"), other_id).await?
        }
        DecksCommand::Containing { card_id } => containing(ctx, &base, &card_id).await?,
        DecksCommand::Visibility { deck_id, public } => {
            let body = serde_json::json!({ "public": public });
            let v: DeckVisibility = ctx
                .client
                .put_json(&format!("{base}/{deck_id}/visibility"), body)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&v)?;
            } else {
                println!(
                    "public: {}  ·  handle: {}",
                    v.public,
                    v.handle.as_deref().unwrap_or("(set a username first)")
                );
            }
        }
    }
    Ok(())
}

async fn folders(ctx: &Ctx, base: &str, cmd: FolderCommand) -> Result<()> {
    match cmd {
        FolderCommand::List => {
            let body: DataBody<Vec<DeckFolder>> =
                ctx.client.get_json(&format!("{base}/folders"), &[]).await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else {
                let mut t = table(&["ID", "Name", "Decks"]);
                for f in &body.data {
                    t.add_row(vec![
                        f.id.to_string(),
                        f.name.clone(),
                        f.deck_count.to_string(),
                    ]);
                }
                println!("{t}");
            }
        }
        FolderCommand::Create { name } => {
            let body = serde_json::json!({ "name": name });
            let f: DeckFolder = ctx
                .client
                .post_json(&format!("{base}/folders"), body)
                .await?;
            ctx.printer
                .note(format!("Created folder '{}' (id {}).", f.name, f.id));
            if ctx.printer.json {
                ctx.printer.json(&f)?;
            }
        }
        FolderCommand::Rename { folder_id, name } => {
            let body = serde_json::json!({ "name": name });
            let f: DeckFolder = ctx
                .client
                .put_json(&format!("{base}/folders/{folder_id}"), body)
                .await?;
            ctx.printer
                .note(format!("Renamed folder {} to '{}'.", f.id, f.name));
        }
        FolderCommand::Delete { folder_id } => {
            ctx.client
                .delete(&format!("{base}/folders/{folder_id}"))
                .await?;
            ctx.printer.note(format!(
                "Deleted folder {folder_id} (its decks were ungrouped)."
            ));
        }
    }
    Ok(())
}

async fn sections(ctx: &Ctx, base: &str, deck_id: i64, cmd: SectionCommand) -> Result<()> {
    let sbase = format!("{base}/{deck_id}/sections");
    match cmd {
        SectionCommand::Add { name, maybeboard } => {
            let body = serde_json::json!({ "name": name, "is_maybeboard": maybeboard });
            let s: DeckSection = ctx.client.post_json(&sbase, body).await?;
            ctx.printer.note(format!(
                "Added {}section '{}' (id {}).",
                if s.is_maybeboard { "maybeboard " } else { "" },
                s.name,
                s.id
            ));
        }
        SectionCommand::Update {
            section_id,
            name,
            position,
            maybeboard,
        } => {
            let body = serde_json::json!({
                "name": name,
                "position": position,
                "is_maybeboard": maybeboard,
            });
            let s: DeckSection = ctx
                .client
                .put_json(&format!("{sbase}/{section_id}"), body)
                .await?;
            ctx.printer.note(format!(
                "Updated section '{}' (position {}{}).",
                s.name,
                s.position,
                if s.is_maybeboard { ", maybeboard" } else { "" }
            ));
        }
        SectionCommand::Reorder { section_ids } => {
            let body = serde_json::json!({ "section_ids": section_ids });
            let out: DataBody<Vec<DeckSection>> = ctx
                .client
                .put_json(&format!("{sbase}/reorder"), body)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&out.data)?;
            } else {
                let names: Vec<String> = out.data.iter().map(|s| s.name.clone()).collect();
                println!("New order: {}", names.join(" → "));
            }
        }
        SectionCommand::Delete { section_id } => {
            ctx.client.delete(&format!("{sbase}/{section_id}")).await?;
            ctx.printer.note(format!("Deleted section {section_id}."));
        }
    }
    Ok(())
}

async fn deck_card(ctx: &Ctx, base: &str, deck_id: i64, cmd: DeckCardCommand) -> Result<()> {
    let cbase = format!("{base}/{deck_id}/cards");
    let result: CollectionQuantities = match cmd {
        DeckCardCommand::Set {
            card_id,
            section,
            qty,
            foil,
        } => {
            let body = serde_json::json!({
                "quantity": qty,
                "foil_quantity": foil,
                "section_id": section,
            });
            ctx.client
                .put_json(&format!("{cbase}/{card_id}"), body)
                .await?
        }
        DeckCardCommand::Move { card_id, from, to } => {
            let body = serde_json::json!({ "from_section_id": from, "to_section_id": to });
            ctx.client
                .put_json(&format!("{cbase}/{card_id}/move"), body)
                .await?
        }
        DeckCardCommand::Printing {
            card_id,
            to_card,
            section,
        } => {
            let body = serde_json::json!({ "new_card_id": to_card, "section_id": section });
            ctx.client
                .put_json(&format!("{cbase}/{card_id}/printing"), body)
                .await?
        }
    };
    if ctx.printer.json {
        ctx.printer.json(&result)?;
    } else {
        println!(
            "Card now {} / {} foil in section.",
            result.quantity, result.foil_quantity
        );
    }
    Ok(())
}

/// The shortfall across every deck of a game, or — with `--deck` — one deck's share
/// of it. The two money columns price the same list twice: at the printings the
/// decks actually run, and at each card's cheapest printing anywhere.
async fn needed(ctx: &Ctx, base: &str, mode: NeededMode, deck: Option<i64>) -> Result<()> {
    let mut q: Vec<(&'static str, String)> = vec![("mode", mode.as_str().to_string())];
    push_opt(&mut q, "deck_id", &deck);
    let body: NeededList = ctx.client.get_json(&format!("{base}/needed"), &q).await?;
    if ctx.printer.json {
        return ctx.printer.json(&body);
    }
    if let Some(d) = &body.deck {
        println!(
            "Scoped to '{}' [{}] — its share of the shortfall across every deck.",
            d.name, d.id
        );
    }
    if body.data.is_empty() {
        println!("Nothing needed — your collection covers every deck.");
        return Ok(());
    }
    let mut t = table(&[
        "Need", "Own", "Want", "Name", "Set", "#", "Held", "Cheapest", "Decks",
    ]);
    for n in &body.data {
        let decks: Vec<&str> = n.decks.iter().map(|d| d.name.as_str()).collect();
        t.add_row(vec![
            n.needed.to_string(),
            n.owned.to_string(),
            n.required.to_string(),
            output::truncate(&n.card.name, 32),
            n.card.set_code.to_uppercase(),
            n.card.collector_number.clone(),
            output::price(&n.held_usd),
            output::price(&n.cheapest_usd),
            output::truncate(&decks.join(", "), 24),
        ]);
    }
    println!("{t}");
    let totals = &body.totals;
    ctx.printer.note(format!(
        "{} card(s) · {} copies · {} at the printings your decks run · {} at each card's cheapest.",
        totals.cards,
        totals.copies,
        output::price(&totals.held_usd),
        output::price(&totals.cheapest_usd)
    ));
    // A total is only ever a floor while some entry has no price to add to it.
    if totals.held_unpriced_cards > 0 || totals.cheapest_unpriced_cards > 0 {
        ctx.printer.note(format!(
            "  {} unpriced as held · {} with no priced printing at all — the totals are floors.",
            totals.held_unpriced_cards, totals.cheapest_unpriced_cards
        ));
    }
    Ok(())
}

/// Add every card of a deck to the caller's collection — the deck proper plus its
/// sideboard, maybeboards skipped. Shared by the own-deck, public-deck and precon
/// writes, which differ only in the path. **Not idempotent by design**: it adds on
/// top of what's owned, so a second call records a second copy of the deck.
pub async fn add_to_collection(ctx: &Ctx, path: &str) -> Result<()> {
    let a: CollectionAdd = ctx.client.post_json(path, serde_json::json!({})).await?;
    if ctx.printer.json {
        return ctx.printer.json(&a);
    }
    println!(
        "Added {} printing(s) ({} regular, {} foil) to your collection.",
        a.cards, a.regular_copies, a.foil_copies
    );
    if a.skipped_cards > 0 {
        println!(
            "  {} card(s) skipped — no longer in the catalog.",
            a.skipped_cards
        );
    }
    Ok(())
}

/// The caller's decks that hold any printing of a card, most recently updated
/// first. The copies are split between the deck proper and the maybeboard, so a
/// deck that only *considers* the card is named without claiming it runs it.
async fn containing(ctx: &Ctx, base: &str, card_id: &str) -> Result<()> {
    let body: DataBody<Vec<CardDeckRef>> = ctx
        .client
        .get_json(&format!("{base}/containing/{card_id}"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&body.data);
    }
    if body.data.is_empty() {
        println!("None of your decks contain this card.");
        return Ok(());
    }
    let mut t = table(&[
        "ID",
        "Deck",
        "Commander",
        "Format",
        "Qty",
        "Maybeboard",
        "Printings (all boards)",
    ]);
    for r in &body.data {
        // Which exact printings the copies are, so a deck running a *different*
        // printing than the one asked about reads as such. These counts span both
        // boards, so they total `Qty` + `Maybeboard`, not `Qty` alone.
        let printings: Vec<String> = r
            .printings
            .iter()
            .map(|p| {
                format!(
                    "{}× {} {}",
                    p.quantity,
                    p.set_code.to_uppercase(),
                    p.collector_number
                )
            })
            .collect();
        t.add_row(vec![
            r.deck.id.to_string(),
            output::truncate(&r.deck.name, 30),
            output::truncate(&commanders(&r.deck), 24),
            output::dash(&r.deck.format),
            r.quantity.to_string(),
            r.maybeboard_quantity.to_string(),
            output::truncate(&printings.join(", "), 36),
        ]);
    }
    println!("{t}");
    ctx.printer.note(format!("{} deck(s).", body.data.len()));
    Ok(())
}

/// A deck's command zone by name (`Thrasios & Tymna`), `—` when it has none.
fn commanders(d: &Deck) -> String {
    if d.commanders.is_empty() {
        return "—".to_string();
    }
    d.commanders
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(" & ")
}

// -- legality / bracket / analytics / goldfish / tokens / combos / mana /
// -- pricing / roles ---------------------------------------------------------
//
// These reads exist several times over — once for your own decks under
// `/api/decks/{game}/{deck_id}`, once for a shared one under
// `/api/u/{handle}/decks/{deck_id}`, and once more over a published decklist under
// `/api/games/{game}/precons/{slug}` — and are identical bar the base path, so each
// handler takes the deck's base URL and `public.rs` / `precons.rs` reuse it.

/// A deck's verdict against its own format. `data` is null when the format isn't
/// one legality is tracked for — that means "nothing to evaluate", not "illegal".
pub async fn legality(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let body: DataBody<Option<DeckLegality>> = ctx
        .client
        .get_json(&format!("{deck_base}/legality"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&body.data);
    }
    let Some(l) = body.data else {
        println!("This deck's format isn't one legality is tracked for.");
        return Ok(());
    };
    println!(
        "{} [{}] — {}",
        l.format_label,
        l.format_key,
        if l.legal { "LEGAL" } else { "NOT LEGAL" }
    );
    if l.unknown_count > 0 {
        println!(
            "  {} card(s) carry no legality data (not counted against the deck).",
            l.unknown_count
        );
    }
    if !l.violations.is_empty() {
        println!();
        for v in &l.violations {
            println!("  [{}] {}: {}", v.severity, v.rule, v.message);
        }
    }
    if !l.issues.is_empty() {
        println!();
        let mut t = table(&["Status", "Qty", "Card", "ID"]);
        for i in &l.issues {
            t.add_row(vec![
                i.status.clone(),
                i.quantity.to_string(),
                output::truncate(&i.name, 40),
                output::truncate(&i.card_id, 12),
            ]);
        }
        println!("{t}");
    }
    if l.legal && l.issues.is_empty() && l.violations.is_empty() {
        println!("  No issues.");
    }
    Ok(())
}

/// Where the deck sits on Wizards' 1–5 Commander bracket ladder, estimated from the
/// cards it holds. `data` is null outside Commander — the one format the ladder is
/// defined for — which is "no ladder to place it on", not "bracket 0".
pub async fn bracket(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let body: DataBody<Option<DeckBracketEstimate>> = ctx
        .client
        .get_json(&format!("{deck_base}/bracket"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&body.data);
    }
    let Some(b) = body.data else {
        println!("No bracket — the ladder is only defined for Commander decks.");
        return Ok(());
    };
    println!("Bracket {} — {}  [{}]", b.bracket, b.label, b.format_label);
    println!("  {}", b.description);
    if b.exhibition_possible {
        println!(
            "  Also clears bracket 1's bar: no Game Changers, mass land denial or extra turns."
        );
    }
    if !b.reasons.is_empty() {
        println!("\nWhy:");
        for r in &b.reasons {
            println!("  · {r}");
        }
    }
    // Every category comes back whether or not the deck holds any; the empty ones
    // are the estimate's silence, already said in the reasons.
    let held: Vec<&DeckBracketCategory> = b.categories.iter().filter(|c| c.count > 0).collect();
    if held.is_empty() {
        println!("\nNo Game Changers, mass land denial, extra turns or tutors found.");
    } else {
        println!();
        let mut t = table(&["Category", "Cards", "Decisive", "Examples"]);
        for c in held {
            let names: Vec<String> = c
                .cards
                .iter()
                .map(|c| {
                    if c.quantity > 1 {
                        format!("{}× {}", c.quantity, c.name)
                    } else {
                        c.name.clone()
                    }
                })
                .collect();
            t.add_row(vec![
                c.label.clone(),
                c.count.to_string(),
                if c.decisive { "yes" } else { "" }.to_string(),
                output::truncate(&names.join(", "), 52),
            ]);
        }
        println!("{t}");
    }
    if !b.caveats.is_empty() {
        println!("\nThe estimate is a floor — it couldn't see:");
        for c in &b.caveats {
            println!("  · {c}");
        }
    }
    println!("\nLadder:");
    for l in &b.ladder {
        println!(
            "  {} {}. {} — {}",
            if l.bracket == b.bracket { "→" } else { " " },
            l.bracket,
            l.label,
            l.description
        );
    }
    Ok(())
}

/// The deck's copy-weighted composition, the same fold over the shuffled library,
/// and the draw-odds curve for one card.
pub async fn stats(ctx: &Ctx, deck_base: &str, args: StatsArgs) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "sections", &args.sections);
    push_opt(&mut q, "card", &args.card);
    push_opt(&mut q, "cards_seen", &args.cards_seen);
    let a: DeckAnalytics = ctx
        .client
        .get_json(&format!("{deck_base}/stats"), &q)
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&a);
    }
    print_composition("Deck", &a.deck);
    print_composition("Library", &a.library);
    println!(
        "\nLibrary sections: {}",
        join_ids(&a.library_section_ids, "(none)")
    );
    match &a.odds {
        None => println!("\nNo draw odds — the library pool is empty."),
        Some(o) => {
            println!(
                "\nDraw odds for {} — {} cop{} in {} cards:",
                o.name,
                o.copies,
                if o.copies == 1 { "y" } else { "ies" },
                o.library_size
            );
            println!(
                "  {:.1}% to see at least one after {} cards.",
                o.at_least_one * 100.0,
                o.cards_seen
            );
            // The API returns the whole curve so a slider can be scrubbed; a
            // terminal wants a few checkpoints, not thirty rows.
            let mut t = table(&["Cards seen", "P(≥1)"]);
            for seen in [1usize, 7, 10, 15, 20, 30] {
                if let Some(p) = o.curve.get(seen - 1) {
                    t.add_row(vec![seen.to_string(), format!("{:.1}%", p * 100.0)]);
                }
            }
            println!("{t}");
        }
    }
    Ok(())
}

fn print_composition(label: &str, c: &DeckComposition) {
    println!(
        "\n== {label} ==  {} copies · {} unique · {} lands · avg MV {}",
        c.total_copies,
        c.unique_cards,
        c.land_copies,
        c.average_mana_value
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "—".to_string())
    );
    print_distribution("Curve", &c.mana_curve);
    print_distribution("Colours", &c.colors);
    print_distribution("Types", &c.card_types);
}

/// One distribution as a single line of `bucket:count` pairs — a terminal reads
/// that faster than three more bordered tables.
fn print_distribution(label: &str, items: &[DeckStatItem]) {
    let shown: Vec<String> = items
        .iter()
        .filter(|i| i.count > 0)
        .map(|i| format!("{} {}", i.label, i.count))
        .collect();
    if shown.is_empty() {
        return;
    }
    println!("  {label:<8}: {}", shown.join(" · "));
}

/// Shuffle the library and deal a sample opening hand.
pub async fn goldfish(ctx: &Ctx, deck_base: &str, args: GoldfishArgs) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "seed", &args.seed);
    push_opt(&mut q, "mulligans", &args.mulligans);
    push_opt(&mut q, "bottom", &args.bottom);
    push_opt(&mut q, "draws", &args.draws);
    push_opt(&mut q, "opening", &args.opening);
    push_opt(&mut q, "sections", &args.sections);
    let h: GoldfishHand = ctx
        .client
        .get_json(&format!("{deck_base}/goldfish"), &q)
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&h);
    }
    println!(
        "seed {} · opening {} · mulligans {} · draws {} · {} of {} left in the library",
        h.seed, h.opening, h.mulligans, h.draws, h.library_size, h.library_total
    );
    println!("sections: {}", join_ids(&h.section_ids, "(none)"));
    if h.hand.is_empty() {
        println!("\n(no cards — the library is empty)");
    } else {
        println!();
        cards_table(&h.hand);
    }
    if !h.bottomed.is_empty() {
        println!("\nBottomed:");
        cards_table(&h.bottomed);
    }
    if h.to_bottom > 0 {
        ctx.printer.note(format!(
            "\n{} more card(s) still to go to the bottom — name them with --bottom <card-ids>.",
            h.to_bottom
        ));
    }
    ctx.printer.note(format!(
        "Replay this hand with --seed {} (plus the same options).",
        h.seed
    ));
    Ok(())
}

/// The tokens and emblems the deck's cards make — what a player brings to a game
/// besides the deck — each with the cards that make it. Read off the catalog's
/// per-card token relations, never inferred from rules text, and scoped to the
/// deck proper (a maybeboard card sends you looking for nothing).
pub async fn tokens(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let body: DeckTokens = ctx
        .client
        .get_json(&format!("{deck_base}/tokens"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&body);
    }
    if body.tokens.is_empty() {
        println!(
            "{}",
            if body.unchecked_count > 0 {
                "No tokens found yet."
            } else {
                "This deck makes no tokens."
            }
        );
    } else {
        let mut t = table(&["Token", "Type", "Set", "#", "Makers", "Made by"]);
        for tok in &body.tokens {
            let mut makers: Vec<String> = tok
                .sources
                .iter()
                .map(|s| {
                    if s.quantity > 1 {
                        format!("{}× {}", s.quantity, s.name)
                    } else {
                        s.name.clone()
                    }
                })
                .collect();
            // `sources` is capped upstream; `source_count` is the exact figure.
            if tok.source_count > tok.sources.len() as i64 {
                makers.push("…".to_string());
            }
            let (set, number) = match &tok.card {
                Some(c) => (c.set_code.to_uppercase(), c.collector_number.clone()),
                None => ("—".to_string(), "—".to_string()),
            };
            t.add_row(vec![
                output::truncate(&tok.name, 26),
                output::truncate(tok.type_line.as_deref().unwrap_or("—"), 28),
                set,
                number,
                tok.source_count.to_string(),
                output::truncate(&makers.join(", "), 40),
            ]);
        }
        println!("{t}");
    }
    if body.unchecked_count > 0 {
        ctx.printer.note(format!(
            "{} card(s) haven't been checked for tokens yet — the list is a floor, not the whole answer.",
            body.unchecked_count
        ));
    }
    Ok(())
}

/// The Commander Spellbook combos the deck proper can assemble, and the ones it is
/// exactly one card (or template, or commander swap) away from. The attribution
/// footer is printed with every answer — the source's terms ask for the link.
pub async fn combos(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let c: DeckCombos = ctx
        .client
        .get_json(&format!("{deck_base}/combos"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&c);
    }
    // `available: false` is "unknown", not "none" — saying "no combos" there would
    // be a claim the data can't support.
    if !c.available {
        println!("No combo data has been synced for this game — nothing to say either way.");
        println!("Data from {} ({}).", c.source, c.source_url);
        return Ok(());
    }
    if c.combos.is_empty() {
        println!("This deck can't assemble any known combo.");
    } else {
        println!("Assembled:");
        let mut t = table(&["Pieces", "Produces", "Played", "Bracket", "URL"]);
        for combo in &c.combos {
            t.add_row(vec![
                output::truncate(&combo_pieces(combo), 44),
                output::truncate(&combo.produces.join(", "), 30),
                combo.popularity.to_string(),
                output::dash(&combo.bracket_tag),
                combo.url.clone(),
            ]);
        }
        println!("{t}");
    }
    if !c.almost.is_empty() {
        println!("\nOne card away:");
        let mut t = table(&["Missing", "Combo", "Produces", "Played", "URL"]);
        for combo in &c.almost {
            t.add_row(vec![
                output::truncate(&combo_missing(combo), 28),
                output::truncate(&combo_pieces(combo), 38),
                output::truncate(&combo.produces.join(", "), 26),
                combo.popularity.to_string(),
                combo.url.clone(),
            ]);
        }
        println!("{t}");
    }
    ctx.printer.note(format!(
        "{} assembled · {} one card away.",
        c.combo_count, c.almost_count
    ));
    ctx.printer
        .note(format!("Data from {} ({}).", c.source, c.source_url));
    Ok(())
}

/// A combo's pieces as one cell — `2× Name`, `Name (commander)` — in the combo's
/// own order, plus the wildcard templates it also needs.
fn combo_pieces(c: &DeckCombo) -> String {
    let mut parts: Vec<String> = c
        .pieces
        .iter()
        .map(|p| {
            let qty = if p.quantity > 1 {
                format!("{}× ", p.quantity)
            } else {
                String::new()
            };
            let zone = if p.must_be_commander {
                " (commander)"
            } else {
                ""
            };
            format!("{qty}{}{zone}", p.name)
        })
        .collect();
    parts.extend(c.templates.iter().map(|t| format!("[{t}]")));
    parts.join(" + ")
}

/// What one "almost" combo still needs, with why each piece is missing (a template
/// can't be evaluated at all; a commander miss is a card the deck holds elsewhere).
fn combo_missing(c: &DeckCombo) -> String {
    c.missing
        .iter()
        .map(|m| match m.kind.as_str() {
            "template" => format!("{} (any)", m.name),
            "commander" => format!("{} (as commander)", m.name),
            _ => m.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// The deck's colour requirements against its sources, judged with Frank Karsten's
/// tables: demand is the library plus the command zone, supply the library alone.
pub async fn mana(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let m: DeckManaBase = ctx
        .client
        .get_json(&format!("{deck_base}/mana"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&m);
    }
    println!(
        "{} cards cast from · {} in the library · {} lands · judged against the {}-card column",
        m.deck_size, m.library_size, m.land_count, m.table_size
    );
    let mut t = table(&[
        "Colour",
        "Pips",
        "Sources",
        "Needed",
        "Shortfall",
        "Verdict",
    ]);
    for c in &m.colors {
        t.add_row(vec![
            c.label.clone(),
            c.pips.to_string(),
            // The split is what a reader weighs: 36 lands and 2 rocks is not the
            // same 38 sources as the other way round.
            format!("{} ({}+{})", c.sources, c.land_sources, c.nonland_sources),
            c.sources_needed
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".to_string()),
            if c.shortfall > 0 {
                c.shortfall.to_string()
            } else {
                String::new()
            },
            output::truncate(&c.verdict, 34),
        ]);
    }
    println!("{t}");
    for c in &m.colors {
        if c.demand.is_empty() {
            continue;
        }
        let shown: Vec<String> = c
            .demand
            .iter()
            .take(4)
            .map(|d| format!("{} {} (needs {})", d.name, d.mana_cost, d.sources_needed))
            .collect();
        let more = c.demand_count - shown.len() as i64;
        println!(
            "  {} wants: {}{}",
            c.label,
            shown.join(" · "),
            if more > 0 {
                format!(" · +{more} more")
            } else {
                String::new()
            }
        );
    }
    println!("\nWhat the numbers assume:");
    for cav in &m.caveats {
        println!("  · {cav}");
    }
    if m.unchecked_count > 0 {
        ctx.printer.note(format!(
            "{} library card(s) haven't been checked for what they produce — every source count is a floor.",
            m.unchecked_count
        ));
    }
    ctx.printer.note(format!("Thresholds from {}.", m.source));
    Ok(())
}

/// Where the deck's value is: every row of the deck proper priced as held, beside
/// the cheapest printing of the same card at the row's own finish split.
pub async fn pricing(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let p: DeckPricing = ctx
        .client
        .get_json(&format!("{deck_base}/pricing"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&p);
    }
    if p.lines.is_empty() {
        println!("This deck has no cards to price.");
        return Ok(());
    }
    let mut t = table(&[
        "Qty", "Foil", "Name", "Set", "#", "Held", "Cheapest", "Saving",
    ]);
    for l in &p.lines {
        t.add_row(vec![
            l.quantity.to_string(),
            l.foil_quantity.to_string(),
            output::truncate(&l.card.name, 32),
            l.card.set_code.to_uppercase(),
            l.card.collector_number.clone(),
            output::price(&l.price_usd),
            output::price(&l.cheapest.as_ref().map(|c| c.price_usd.clone())),
            output::price(&l.saving_usd),
        ]);
    }
    println!("{t}");
    println!(
        "\nTotal {}  ·  at the cheapest printings {}  ·  saving {}",
        output::price(&p.total_usd),
        output::price(&p.cheapest_total_usd),
        output::price(&p.saving_usd)
    );
    if p.swappable_count > 0 {
        ctx.printer.note(format!(
            "{} row(s) would save money on a printing swap.",
            p.swappable_count
        ));
    }
    if p.unpriced_count > 0 {
        ctx.printer.note(format!(
            "{} row(s) unpriced in every finish they hold — the total is a floor.",
            p.unpriced_count
        ));
    }
    Ok(())
}

/// How many pieces of each deckbuilding role the deck holds. The roles are not a
/// partition of the deck — a card can fill several, and most creatures and every
/// land fill none — so the unclassified count is printed rather than inferred.
pub async fn roles(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let r: DeckRoles = ctx
        .client
        .get_json(&format!("{deck_base}/roles"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&r);
    }
    let mut t = table(&["Role", "Cards", "Copies", "Examples"]);
    for g in &r.roles {
        let names: Vec<String> = g
            .cards
            .iter()
            .map(|c| {
                if c.quantity > 1 {
                    format!("{}× {}", c.quantity, c.name)
                } else {
                    c.name.clone()
                }
            })
            .collect();
        t.add_row(vec![
            g.label.clone(),
            g.count.to_string(),
            g.copies.to_string(),
            output::truncate(&names.join(", "), 52),
        ]);
    }
    println!("{t}");
    ctx.printer.note(format!(
        "{} card(s) in the deck · {} fill no role (the roles aren't a partition).",
        r.card_count, r.unclassified_count
    ));
    Ok(())
}

/// The cards in the caller's collection this deck could play. Owner-only — it reads
/// the collection, so there's no public or precon mirror of it.
async fn suggestions(ctx: &Ctx, deck_base: &str) -> Result<()> {
    let s: DeckSuggestions = ctx
        .client
        .get_json(&format!("{deck_base}/suggestions"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&s);
    }
    println!(
        "{}  ·  colours {}  ·  {} owned candidate(s), {} scanned",
        s.format_label.as_deref().unwrap_or("no tracked format"),
        identity(&s.color_identity),
        s.candidate_count,
        s.scanned_count
    );
    if !s.commanders.is_empty() {
        println!(
            "  commanders: {}",
            s.commanders
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(" & ")
        );
    }
    // `top` and every role's `card_ids` index into `cards`, which carries each card
    // once however many roles it fills.
    let by_id: HashMap<&str, &DeckSuggestionCard> =
        s.cards.iter().map(|c| (c.card.id.as_str(), c)).collect();
    if s.cards.is_empty() {
        println!("\nNothing in your collection fits this deck.");
        return Ok(());
    }
    if !s.top.is_empty() {
        println!("\nMost played overall:");
        let mut t = table(&["Rank", "Name", "Set", "#", "Own", "Roles"]);
        for id in &s.top {
            let Some(c) = by_id.get(id.as_str()) else {
                continue;
            };
            t.add_row(vec![
                c.edhrec_rank.to_string(),
                output::truncate(&c.card.name, 32),
                c.card.set_code.to_uppercase(),
                c.card.collector_number.clone(),
                c.owned.to_string(),
                output::truncate(&c.roles.join(", "), 30),
            ]);
        }
        println!("{t}");
    }
    println!();
    for r in &s.roles {
        if r.count == 0 {
            continue;
        }
        let names: Vec<&str> = r
            .card_ids
            .iter()
            .filter_map(|id| by_id.get(id.as_str()).map(|c| c.card.name.as_str()))
            .take(5)
            .collect();
        println!(
            "{}: you have {} · you own {} more — {}",
            r.label,
            r.in_deck,
            r.count,
            output::truncate(&names.join(", "), 60)
        );
    }
    if s.unclassified_count > 0 {
        println!(
            "\n{} scanned candidate(s) fill no role at all.",
            s.unclassified_count
        );
    }
    println!("\nWhat this ranking is:");
    for c in &s.caveats {
        println!("  · {c}");
    }
    Ok(())
}

/// What changed between two of the caller's decks. Cards fold by **name** across
/// every printing and both finishes, so a printing swap is not a change.
async fn diff(ctx: &Ctx, deck_base: &str, other_id: i64) -> Result<()> {
    let d: DeckDiff = ctx
        .client
        .get_json(&format!("{deck_base}/diff/{other_id}"), &[])
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&d);
    }
    println!(
        "{} [{}] vs {} [{}]",
        d.base.name, d.base.id, d.other.name, d.other.id
    );
    println!(
        "  {} cards vs {} cards",
        d.base.total_cards, d.other.total_cards
    );
    println!(
        "  {} added · {} removed · {} changed · {} finish-only · {} unchanged",
        d.summary.added,
        d.summary.removed,
        d.summary.changed,
        d.summary.finish_changed,
        d.summary.unchanged
    );
    if d.cards.is_empty() {
        println!("\nThe two decks hold the same cards.");
    } else {
        println!();
        print_diff_entries(&d.cards);
    }
    // Only sections with something to report are listed, so each one gets a block.
    for s in &d.sections {
        println!(
            "\n== {}{} ==  ({} card(s) held identically)",
            s.name,
            if s.is_maybeboard {
                "  [maybeboard]"
            } else {
                ""
            },
            s.unchanged
        );
        print_diff_entries(&s.entries);
    }
    Ok(())
}

fn print_diff_entries(entries: &[DeckDiffEntry]) {
    let mut t = table(&["Change", "Name", "Base", "Other", "Δ"]);
    for e in entries {
        // A `finish` row differs only in the regular/foil split, so the counts are
        // only readable with the foil half spelled out.
        let split = e.change == "finish";
        t.add_row(vec![
            e.change.clone(),
            output::truncate(&e.name, 34),
            diff_qty(e.base_quantity, e.base_foil_quantity, split),
            diff_qty(e.other_quantity, e.other_foil_quantity, split),
            format!("{:+}", e.delta),
        ]);
    }
    println!("{t}");
}

fn diff_qty(quantity: i64, foil: i64, split: bool) -> String {
    if split {
        format!("{quantity} ({foil} foil)")
    } else {
        quantity.to_string()
    }
}

/// A colour identity as WUBRG letters — `C` for a deliberately colourless deck, `—`
/// when there was nothing to read a colour off at all.
fn identity(colors: &Option<Vec<String>>) -> String {
    match colors {
        None => "—".to_string(),
        Some(c) if c.is_empty() => "C".to_string(),
        Some(c) => c.join(""),
    }
}

fn join_ids(ids: &[i64], empty: &str) -> String {
    if ids.is_empty() {
        return empty.to_string();
    }
    ids.iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_deck_detail(d: &DeckDetail) {
    println!("{}  [id {}]", d.name, d.id);
    println!(
        "  format: {}  ·  cards: {}  ·  value: {}  ·  public: {}",
        d.format.as_deref().unwrap_or("—"),
        d.summary.total_cards,
        output::price(&d.summary.total_value_usd),
        d.is_public
    );
    // The deck's own totals exclude the maybeboard, so report it separately when
    // there's anything being considered.
    if let Some(m) = &d.maybeboard_summary
        && m.total_cards > 0
    {
        println!(
            "  maybeboard: {} cards · {}",
            m.total_cards,
            output::price(&m.total_value_usd)
        );
    }
    if let Some(desc) = &d.description
        && !desc.is_empty()
    {
        println!("  {desc}");
    }
    let mut by_section: HashMap<i64, Vec<&DeckCardEntry>> = HashMap::new();
    for c in &d.cards {
        by_section.entry(c.section_id).or_default().push(c);
    }
    for section in &d.sections {
        let cards = by_section.get(&section.id);
        let count: i64 = cards
            .map(|cs| cs.iter().map(|c| c.quantity + c.foil_quantity).sum())
            .unwrap_or(0);
        if count == 0 {
            continue;
        }
        println!(
            "\n== {} ({count}){} ==",
            section.name,
            if section.is_maybeboard {
                "  [maybeboard]"
            } else {
                ""
            }
        );
        if let Some(cards) = cards {
            let mut t = table(&["Qty", "Foil", "Name", "Set", "#"]);
            for c in cards {
                t.add_row(vec![
                    c.quantity.to_string(),
                    c.foil_quantity.to_string(),
                    output::truncate(&c.card.name, 34),
                    c.card.set_code.to_uppercase(),
                    c.card.collector_number.clone(),
                ]);
            }
            println!("{t}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{diff_qty, identity};

    #[test]
    fn identity_renders_the_three_way_colour_identity() {
        assert_eq!(identity(&None), "—");
        assert_eq!(identity(&Some(vec![])), "C");
        assert_eq!(
            identity(&Some(vec!["W".to_string(), "B".to_string()])),
            "WB"
        );
    }

    #[test]
    fn diff_quantities_spell_out_the_foil_half_only_for_a_finish_change() {
        assert_eq!(diff_qty(4, 1, false), "4");
        assert_eq!(diff_qty(4, 1, true), "4 (1 foil)");
    }
}
