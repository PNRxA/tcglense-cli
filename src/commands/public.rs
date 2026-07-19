//! Public, handle-keyed surfaces: another user's profile, shared collection, shared
//! wish list, and public decks. The reads are unauthenticated (no credential is
//! required or sent); the one write — copying a public deck into your own decks —
//! needs a credential.

use anyhow::Result;
use clap::{Args, Subcommand};

use super::Ctx;
use super::holdings::{self, Surface};
use crate::models::*;
use crate::output::{decks_table, table};

#[derive(Debug, Args)]
pub struct PublicArgs {
    /// The owner's public handle, e.g. `alice-0001`.
    pub handle: String,
    #[command(subcommand)]
    pub command: PublicCommand,
}

#[derive(Debug, Subcommand)]
pub enum PublicCommand {
    /// Show the owner's public profile (handle + per-game summaries).
    Profile,
    /// Read the owner's public collection for a game.
    Collection {
        game: String,
        #[command(subcommand)]
        command: PublicHoldingsCommand,
    },
    /// Read the owner's public wish list for a game.
    Wishlist {
        game: String,
        #[command(subcommand)]
        command: PublicHoldingsCommand,
    },
    /// List the owner's public decks (across games).
    Decks,
    /// Show one public deck (or, with `copy`, clone it into your own decks).
    Deck {
        deck_id: i64,
        #[command(subcommand)]
        command: Option<PublicDeckCommand>,
    },
}

#[derive(Debug, Subcommand)]
pub enum PublicDeckCommand {
    /// Copy this public deck into your own decks (auth required; starts private).
    Copy,
}

/// The read-only holdings surface shared by the public collection and wish list
/// (both are handle-keyed and expose the same reads; only the base path differs).
#[derive(Debug, Subcommand)]
pub enum PublicHoldingsCommand {
    /// List held cards.
    List {
        #[arg(short = 'q', long)]
        query: Option<String>,
        #[arg(long)]
        set: Option<String>,
        #[arg(long)]
        related: bool,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Value / copy summary.
    Summary {
        #[arg(long)]
        set: Option<String>,
        #[arg(long)]
        related: bool,
    },
    /// Per-set aggregates.
    Sets,
    /// Held cards in a drop-grouped set, grouped by drop.
    Drops {
        code: String,
        #[arg(short = 'q', long)]
        query: Option<String>,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Held cards in a set, grouped by sub-type.
    Subtypes {
        code: String,
        #[arg(short = 'q', long)]
        query: Option<String>,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Which of the given card ids the owner holds.
    Owned { ids: Vec<String> },
    /// Read the owner's held sealed products.
    Products {
        #[command(subcommand)]
        command: PublicProductsCommand,
    },
}

/// The read-only sealed-product views on a public holdings surface.
#[derive(Debug, Subcommand)]
pub enum PublicProductsCommand {
    /// List held sealed products.
    List {
        #[arg(long)]
        set: Option<String>,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Per-set aggregate tiles.
    Sets,
    /// Aggregate summary of held products.
    Summary,
}

pub async fn run(ctx: &Ctx, args: PublicArgs) -> Result<()> {
    let handle = &args.handle;
    match args.command {
        PublicCommand::Profile => {
            let p: PublicProfile = ctx
                .client
                .get_json(&format!("/api/u/{handle}"), &[])
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&p)?;
            } else {
                println!("{} (member since {})", p.handle, p.member_since);
                let mut t = table(&["Game", "Cards", "Copies", "Value"]);
                for g in &p.games {
                    t.add_row(vec![
                        g.game.clone(),
                        g.summary.unique_cards.to_string(),
                        g.summary.total_cards.to_string(),
                        crate::output::price(&g.summary.total_value_usd),
                    ]);
                }
                println!("{t}");
            }
        }
        PublicCommand::Collection { game, command } => {
            let s = Surface {
                base: format!("/api/u/{handle}/{game}"),
                batch_route: "owned",
                product_batch_route: "owned",
                noun: "Owned",
            };
            run_holdings(ctx, &s, command).await?
        }
        PublicCommand::Wishlist { game, command } => {
            let s = Surface {
                base: format!("/api/u/{handle}/wishlist/{game}"),
                batch_route: "owned",
                product_batch_route: "owned",
                noun: "Wanted",
            };
            run_holdings(ctx, &s, command).await?
        }
        PublicCommand::Decks => {
            let body: DataBody<Vec<Deck>> = ctx
                .client
                .get_json(&format!("/api/u/{handle}/decks"), &[])
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else if body.data.is_empty() {
                println!("No public decks.");
            } else {
                decks_table(&body.data);
            }
        }
        PublicCommand::Deck { deck_id, command } => match command {
            None => {
                let d: DeckDetail = ctx
                    .client
                    .get_json(&format!("/api/u/{handle}/decks/{deck_id}"), &[])
                    .await?;
                if ctx.printer.json {
                    ctx.printer.json(&d)?;
                } else {
                    println!("{} by {}", d.name, d.handle.as_deref().unwrap_or("?"));
                    println!(
                        "  format: {}  ·  cards: {}",
                        d.format.as_deref().unwrap_or("—"),
                        d.summary.total_cards
                    );
                    let mut t = table(&["Qty", "Foil", "Name", "Set"]);
                    for c in &d.cards {
                        t.add_row(vec![
                            c.quantity.to_string(),
                            c.foil_quantity.to_string(),
                            crate::output::truncate(&c.card.name, 34),
                            c.card.set_code.to_uppercase(),
                        ]);
                    }
                    println!("{t}");
                }
            }
            Some(PublicDeckCommand::Copy) => {
                let d: DeckDetail = ctx
                    .client
                    .post_json(
                        &format!("/api/u/{handle}/decks/{deck_id}/copy"),
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
        },
    }
    Ok(())
}

/// Dispatch a read on a public holdings surface (collection or wish list).
async fn run_holdings(ctx: &Ctx, s: &Surface, command: PublicHoldingsCommand) -> Result<()> {
    match command {
        PublicHoldingsCommand::List {
            query,
            set,
            related,
            page,
            page_size,
        } => holdings::list(ctx, s, query, set, related, None, None, page, page_size).await,
        PublicHoldingsCommand::Summary { set, related } => {
            holdings::summary(ctx, s, set, related).await
        }
        PublicHoldingsCommand::Sets => holdings::sets(ctx, s).await,
        PublicHoldingsCommand::Drops {
            code,
            query,
            page,
            page_size,
        } => holdings::set_drops(ctx, s, &code, query, page, page_size).await,
        PublicHoldingsCommand::Subtypes {
            code,
            query,
            page,
            page_size,
        } => holdings::set_subtypes(ctx, s, &code, query, page, page_size).await,
        PublicHoldingsCommand::Owned { ids } => holdings::batch_counts(ctx, s, ids).await,
        PublicHoldingsCommand::Products { command } => match command {
            PublicProductsCommand::List {
                set,
                page,
                page_size,
            } => holdings::products_list(ctx, s, set, page, page_size).await,
            PublicProductsCommand::Sets => holdings::products_sets(ctx, s).await,
            PublicProductsCommand::Summary => holdings::products_summary(ctx, s).await,
        },
    }
}
