//! Preconstructed-deck commands: the published decklists that ship with a game's
//! sets — Commander decks, Planeswalker / Challenger / Starter decks, Jumpstart
//! themes, intro packs. Browse them flat or bucketed, read one in full, and copy
//! one into your own decks.
//!
//! The per-deck reads (`legality`, `bracket`, `stats`, `goldfish`) are computed by
//! the same core over the published list as over a deck of your own, so they reuse
//! the handlers in [`super::decks`] with the precon's base path — the same way
//! `public.rs` reuses them for a shared deck. The one write lives on the *decks*
//! surface (`POST /api/decks/{game}/precons/{slug}/copy`), because what it creates
//! is a deck.

use anyhow::Result;
use clap::{Args, Subcommand};

use super::{Ctx, decks, page_footer, push_flag, push_opt};
use crate::models::*;
use crate::output::{self, table};

#[derive(Debug, Args)]
pub struct PreconsArgs {
    /// Game id slug, e.g. `mtg`.
    pub game: String,
    #[command(subcommand)]
    pub command: PreconsCommand,
}

/// The browse filters, identical on the flat list and the grouped one.
#[derive(Debug, Args)]
pub struct PreconFilters {
    /// Name substring; every word must match.
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    /// Set code, e.g. `tmc`.
    #[arg(long)]
    pub set: Option<String>,
    /// With --set, span the set's whole group (root + related sub-sets).
    #[arg(long)]
    pub related: bool,
    /// Deck type, e.g. `Commander Deck` — `facets` lists the vocabulary.
    #[arg(long = "type")]
    pub type_: Option<String>,
    /// Sort key: released (default, newest first) | name.
    #[arg(long)]
    pub sort: Option<String>,
    #[arg(long)]
    pub page: Option<u32>,
    #[arg(long)]
    pub page_size: Option<u32>,
}

impl PreconFilters {
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q: Vec<(&'static str, String)> = Vec::new();
        push_opt(&mut q, "q", &self.query);
        push_opt(&mut q, "set", &self.set);
        push_flag(&mut q, "include_related", self.related);
        push_opt(&mut q, "type", &self.type_);
        push_opt(&mut q, "sort", &self.sort);
        push_opt(&mut q, "page", &self.page);
        push_opt(&mut q, "page_size", &self.page_size);
        q
    }
}

#[derive(Debug, Subcommand)]
pub enum PreconsCommand {
    /// List the game's preconstructed decks, newest first.
    List {
        #[command(flatten)]
        filters: PreconFilters,
    },
    /// List them bucketed by set (default) or by deck type — a page is a page of
    /// groups, so a group is never split across a boundary.
    Groups {
        /// Bucket by `set` (default) or `type`.
        #[arg(long)]
        group: Option<String>,
        #[command(flatten)]
        filters: PreconFilters,
    },
    /// Show the deck types and sets that have precons, with counts.
    Facets,
    /// Show one precon in full: header, value, every card, and the product it ships in.
    Show { slug: String },
    /// Check the deck against the format its type states.
    Legality { slug: String },
    /// Estimate where a Commander precon sits on the 1–5 bracket ladder.
    Bracket { slug: String },
    /// Composition (curve, colours, types) plus the draw odds for one card.
    Stats {
        slug: String,
        #[command(flatten)]
        args: decks::StatsArgs,
    },
    /// Shuffle the library and deal a sample opening hand.
    Goldfish {
        slug: String,
        #[command(flatten)]
        args: decks::GoldfishArgs,
    },
    /// Copy the precon into your own decks (auth required; starts private + loose).
    Copy { slug: String },
}

pub async fn run(ctx: &Ctx, args: PreconsArgs) -> Result<()> {
    let game = args.game;
    let base = format!("/api/games/{game}/precons");
    match args.command {
        PreconsCommand::List { filters } => {
            let page: Page<PreconDeck> = ctx.client.get_json(&base, &filters.query()).await?;
            if ctx.printer.json {
                ctx.printer.json(&page)?;
            } else {
                if page.data.is_empty() {
                    println!("No preconstructed decks match.");
                } else {
                    precons_table(&page.data);
                }
                page_footer(ctx, page.page, page.total, page.has_more, "decks");
            }
        }
        PreconsCommand::Groups { group, filters } => {
            let mut q = filters.query();
            push_opt(&mut q, "group", &group);
            let page: Page<PreconGroup> =
                ctx.client.get_json(&format!("{base}/groups"), &q).await?;
            if ctx.printer.json {
                ctx.printer.json(&page)?;
            } else {
                if page.data.is_empty() {
                    println!("No preconstructed decks match.");
                }
                for g in &page.data {
                    println!(
                        "\n== {} ({} deck{}{}) ==",
                        g.title,
                        g.deck_count,
                        if g.deck_count == 1 { "" } else { "s" },
                        match &g.released_at {
                            Some(d) => format!(", {d}"),
                            None => String::new(),
                        }
                    );
                    precons_table(&g.decks);
                }
                page_footer(ctx, page.page, page.total, page.has_more, "groups");
            }
        }
        PreconsCommand::Facets => {
            let body: DataBody<PreconFacets> =
                ctx.client.get_json(&format!("{base}/facets"), &[]).await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else {
                print_facets(&body.data);
            }
        }
        PreconsCommand::Show { slug } => {
            let d: PreconDeckDetail = ctx.client.get_json(&format!("{base}/{slug}"), &[]).await?;
            if ctx.printer.json {
                ctx.printer.json(&d)?;
            } else {
                print_precon_detail(&d);
            }
        }
        PreconsCommand::Legality { slug } => {
            decks::legality(ctx, &format!("{base}/{slug}")).await?
        }
        PreconsCommand::Bracket { slug } => decks::bracket(ctx, &format!("{base}/{slug}")).await?,
        PreconsCommand::Stats { slug, args } => {
            decks::stats(ctx, &format!("{base}/{slug}"), args).await?
        }
        PreconsCommand::Goldfish { slug, args } => {
            decks::goldfish(ctx, &format!("{base}/{slug}"), args).await?
        }
        PreconsCommand::Copy { slug } => {
            let d: DeckDetail = ctx
                .client
                .post_json(
                    &format!("/api/decks/{game}/precons/{slug}/copy"),
                    serde_json::json!({}),
                )
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&d)?;
            } else {
                println!(
                    "Copied '{}' into your decks as deck {} ({} cards, private).",
                    d.name, d.id, d.summary.total_cards
                );
            }
        }
    }
    Ok(())
}

// -- rendering ---------------------------------------------------------------

fn precons_table(decks: &[PreconDeck]) {
    let mut t = table(&["Slug", "Name", "Set", "Type", "Colours", "Cards", "Side"]);
    for d in decks {
        t.add_row(vec![
            output::truncate(&d.slug, 28),
            output::truncate(&d.name, 32),
            d.set_code.to_uppercase(),
            output::truncate(&d.deck_type, 18),
            colours(&d.color_identity),
            d.card_count.to_string(),
            d.sideboard_count.to_string(),
        ]);
    }
    println!("{t}");
}

/// A deck's colour identity as WUBRG letters — `C` for a deliberately colourless
/// deck, `—` when there was nothing to read a colour off at all.
fn colours(identity: &Option<Vec<String>>) -> String {
    match identity {
        None => "—".to_string(),
        Some(c) if c.is_empty() => "C".to_string(),
        Some(c) => c.join(""),
    }
}

fn print_facets(f: &PreconFacets) {
    println!("{} preconstructed deck(s).", f.total);
    if !f.types.is_empty() {
        let mut t = table(&["Type", "Decks"]);
        for ty in &f.types {
            t.add_row(vec![output::truncate(&ty.type_, 34), ty.count.to_string()]);
        }
        println!("{t}");
    }
    if !f.sets.is_empty() {
        let mut t = table(&["Set", "Name", "Released", "Decks"]);
        for s in &f.sets {
            t.add_row(vec![
                s.code.to_uppercase(),
                output::truncate(s.name.as_deref().unwrap_or("—"), 40),
                output::dash(&s.released_at),
                s.count.to_string(),
            ]);
        }
        println!("{t}");
    }
}

fn print_precon_detail(d: &PreconDeckDetail) {
    let h = &d.deck;
    println!("{}  [{}]", h.name, h.slug);
    println!(
        "  {} · {} · {}",
        h.deck_type,
        h.set_code.to_uppercase(),
        h.set_name.as_deref().unwrap_or("—")
    );
    println!(
        "  format: {}  ·  colours: {}  ·  released: {}",
        d.format.as_deref().unwrap_or("—"),
        colours(&h.color_identity),
        h.released_at.as_deref().unwrap_or("—")
    );
    println!(
        "  cards: {}  ·  value: {}",
        d.summary.total_cards,
        output::price(&d.summary.total_value_usd)
    );
    // The header's counts (and `summary`) cover the deck proper, so a sideboard is
    // only ever reported apart from it — it can't inflate the deck's value.
    if d.sideboard_summary.total_cards > 0 {
        println!(
            "  sideboard: {} cards · {}",
            d.sideboard_summary.total_cards,
            output::price(&d.sideboard_summary.total_value_usd)
        );
    }
    if let Some(fc) = &h.face_card {
        println!("  face card: {} [{}]", fc.name, fc.card_id);
    }
    if let Some(p) = &d.product {
        println!(
            "  ships in: {} [{}] · {}",
            p.name,
            p.id,
            output::price(&p.prices.usd)
        );
    }
    // `cards` arrives in board order; the API's own board keys get a heading each.
    for (board, label) in [
        ("commander", "Command zone"),
        ("main", "Deck"),
        ("side", "Sideboard"),
    ] {
        print_board(d, board, label);
    }
    // Anything the API grows beyond the three known boards still gets shown.
    for board in d
        .cards
        .iter()
        .map(|c| c.board.as_str())
        .filter(|b| !matches!(*b, "commander" | "main" | "side"))
        .collect::<std::collections::BTreeSet<_>>()
    {
        print_board(d, board, board);
    }
}

fn print_board(d: &PreconDeckDetail, board: &str, label: &str) {
    let cards: Vec<&PreconCardEntry> = d.cards.iter().filter(|c| c.board == board).collect();
    if cards.is_empty() {
        return;
    }
    let count: i64 = cards.iter().map(|c| c.quantity).sum();
    println!("\n== {label} ({count}) ==");
    let mut t = table(&["Qty", "Foil", "Name", "Set", "#", "USD"]);
    for c in cards {
        t.add_row(vec![
            c.quantity.to_string(),
            if c.foil { "yes" } else { "" }.to_string(),
            output::truncate(&c.card.name, 34),
            c.card.set_code.to_uppercase(),
            c.card.collector_number.clone(),
            output::price(if c.foil {
                &c.card.prices.usd_foil
            } else {
                &c.card.prices.usd
            }),
        ]);
    }
    println!("{t}");
}

#[cfg(test)]
mod tests {
    use super::colours;

    #[test]
    fn colours_render_the_three_way_identity() {
        assert_eq!(colours(&None), "—");
        assert_eq!(colours(&Some(vec![])), "C");
        assert_eq!(colours(&Some(vec!["W".to_string(), "U".to_string()])), "WU");
    }
}
