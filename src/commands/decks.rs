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
    },
    /// Check a deck against its own format: offending cards + construction breaches.
    Legality { deck_id: i64 },
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
        DecksCommand::Needed { mode } => needed(ctx, &base, mode).await?,
        DecksCommand::Legality { deck_id } => legality(ctx, &format!("{base}/{deck_id}")).await?,
        DecksCommand::Stats { deck_id, args } => {
            stats(ctx, &format!("{base}/{deck_id}"), args).await?
        }
        DecksCommand::Goldfish { deck_id, args } => {
            goldfish(ctx, &format!("{base}/{deck_id}"), args).await?
        }
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

async fn needed(ctx: &Ctx, base: &str, mode: NeededMode) -> Result<()> {
    let body: DataBody<Vec<NeededCard>> = ctx
        .client
        .get_json(
            &format!("{base}/needed"),
            &[("mode", mode.as_str().to_string())],
        )
        .await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else if body.data.is_empty() {
        println!("Nothing needed — your collection covers every deck.");
    } else {
        let mut t = table(&["Need", "Own", "Want", "Name", "Set", "#", "Decks"]);
        for n in &body.data {
            let decks: Vec<&str> = n.decks.iter().map(|d| d.name.as_str()).collect();
            t.add_row(vec![
                n.needed.to_string(),
                n.owned.to_string(),
                n.required.to_string(),
                output::truncate(&n.card.name, 32),
                n.card.set_code.to_uppercase(),
                n.card.collector_number.clone(),
                output::truncate(&decks.join(", "), 30),
            ]);
        }
        println!("{t}");
        ctx.printer
            .note(format!("{} card(s) needed.", body.data.len()));
    }
    Ok(())
}

// -- legality / analytics / goldfish -----------------------------------------
//
// These three reads exist twice over — once for your own decks under
// `/api/decks/{game}/{deck_id}`, once for a shared one under
// `/api/u/{handle}/decks/{deck_id}` — and are identical bar the base path, so
// each handler takes the deck's base URL and `public.rs` reuses it.

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
