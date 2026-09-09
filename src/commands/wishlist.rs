//! Wish-list commands: the collection's "want" twin. Rides the same shared holdings
//! engine, minus import/sync/export/movers/value-history (a wish list has nothing to
//! import and no value chart). It keeps its own public-sharing visibility, which —
//! unlike the collection's — has no value-chart/movers toggles.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use super::holdings::{self, CopyFilter, ListFilter, ProductHoldingCommand, Surface};
use super::{CardExportFormat, Ctx};
use crate::models::{BuyList, WishlistVisibility};
use crate::output;

#[derive(Debug, Args)]
pub struct WishlistArgs {
    pub game: String,
    #[command(subcommand)]
    pub command: WishlistCommand,
}

#[derive(Debug, Subcommand)]
pub enum WishlistCommand {
    /// List wanted cards.
    List {
        #[command(flatten)]
        filter: ListFilter,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Show wanted counts for one card.
    Get { card_id: String },
    /// Set the absolute wanted counts for a card (both zero removes it).
    Set {
        card_id: String,
        #[arg(long, default_value_t = 0)]
        qty: i64,
        #[arg(long, default_value_t = 0)]
        foil: i64,
    },
    /// Increment the wanted counts for a card.
    Add {
        card_id: String,
        #[arg(long, default_value_t = 1)]
        qty: i64,
        #[arg(long, default_value_t = 0)]
        foil: i64,
    },
    /// Remove a card from the wish list.
    Remove { card_id: String },
    /// Wish-list value / copy summary.
    Summary {
        #[arg(long)]
        set: Option<String>,
        #[arg(long)]
        related: bool,
    },
    /// Per-set wanted aggregates.
    Sets {
        /// Per-unit bulk price cutoff in USD cents (default $1) — splits each set
        /// tile's bulk subtotal out of its value.
        #[arg(long, value_name = "CENTS")]
        bulk_max: Option<i64>,
    },
    /// Where the wish list's value sits: copies + value by rarity, colour, card
    /// type and finish, plus the most valuable wants.
    Breakdown {
        /// Per-unit bulk price cutoff in USD cents (default $1) — splits the bulk
        /// subtotal out of the embedded summary's total.
        #[arg(long, value_name = "CENTS")]
        bulk_max: Option<i64>,
    },
    /// Wanted cards in a drop-grouped set, grouped by drop.
    Drops {
        code: String,
        /// Scryfall-style search filter within the set.
        #[arg(short = 'q', long)]
        query: Option<String>,
        #[command(flatten)]
        copies: CopyFilter,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Wanted cards in a set, grouped by sub-type.
    Subtypes {
        code: String,
        /// Scryfall-style search filter within the set.
        #[arg(short = 'q', long)]
        query: Option<String>,
        #[command(flatten)]
        copies: CopyFilter,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Batch wanted counts for the given card ids.
    Counts { ids: Vec<String> },
    /// The shopping list: wanted cards as bulk-buy rows — name, set, collector
    /// number, the wanted counts, and the printing's TCGplayer product id. An
    /// unfiltered request (no `-q`, `--set` or copy-count filter) is "buy the whole
    /// list" and carries the wanted sealed products too. Capped at 500 card rows.
    BuyList {
        #[command(flatten)]
        filter: ListFilter,
    },
    /// Export a wanted-card search as a `.txt` deck-list (the same filters as
    /// `list`) — a shopping list that pastes straight into the importers.
    ExportCards {
        #[command(flatten)]
        filter: ListFilter,
        #[arg(long, value_enum, default_value_t = CardExportFormat::Text)]
        format: CardExportFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Manage wanted sealed products.
    Products {
        #[command(subcommand)]
        command: ProductHoldingCommand,
    },
    /// Manage public sharing of this wish list.
    Visibility {
        #[command(subcommand)]
        command: WishlistVisibilityCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum WishlistVisibilityCommand {
    /// Show whether the wish list is public (and its share handle).
    Show,
    /// Make the wish list public (`true`) or private (`false`).
    Set {
        #[arg(action = clap::ArgAction::Set)]
        public: bool,
    },
}

pub async fn run(ctx: &Ctx, args: WishlistArgs) -> Result<()> {
    let s = Surface {
        base: format!("/api/wishlist/{}", args.game),
        batch_route: "counts",
        product_batch_route: "counts",
        noun: "Wanted",
    };
    match args.command {
        WishlistCommand::List {
            filter,
            page,
            page_size,
        } => holdings::list(ctx, &s, filter, page, page_size).await,
        WishlistCommand::Get { card_id } => holdings::get(ctx, &s, &card_id).await,
        WishlistCommand::Set { card_id, qty, foil } => {
            holdings::set(ctx, &s, &card_id, qty, foil).await
        }
        WishlistCommand::Add { card_id, qty, foil } => {
            holdings::add(ctx, &s, &card_id, qty, foil).await
        }
        WishlistCommand::Remove { card_id } => holdings::set(ctx, &s, &card_id, 0, 0).await,
        WishlistCommand::Summary { set, related } => {
            holdings::summary(ctx, &s, set, related, None).await
        }
        WishlistCommand::Sets { bulk_max } => holdings::sets(ctx, &s, bulk_max).await,
        WishlistCommand::Breakdown { bulk_max } => holdings::breakdown(ctx, &s, bulk_max).await,
        WishlistCommand::Drops {
            code,
            query,
            copies,
            page,
            page_size,
        } => holdings::set_drops(ctx, &s, &code, query, copies, page, page_size).await,
        WishlistCommand::Subtypes {
            code,
            query,
            copies,
            page,
            page_size,
        } => holdings::set_subtypes(ctx, &s, &code, query, copies, page, page_size).await,
        WishlistCommand::Counts { ids } => holdings::batch_counts(ctx, &s, ids).await,
        WishlistCommand::BuyList { filter } => buy_list(ctx, &s, filter).await,
        WishlistCommand::ExportCards {
            filter,
            format,
            output,
        } => holdings::export_cards(ctx, &s, filter, format, output).await,
        WishlistCommand::Products { command } => holdings::products(ctx, &s, command).await,
        WishlistCommand::Visibility { command } => visibility(ctx, &s, command).await,
    }
}

/// The shopping list: the wanted cards the same filters as `list` match, as
/// bulk-buy rows. Not paginated — the API caps the card rows at 500 and says so on
/// the envelope. An unfiltered request also carries the wanted sealed products.
async fn buy_list(ctx: &Ctx, s: &Surface, filter: ListFilter) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    filter.push(&mut q);
    let path = format!("{}/buy-list", s.base);
    let list: BuyList = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&list)?;
    } else {
        output::buy_list(&list, &ctx.printer);
    }
    Ok(())
}

async fn visibility(ctx: &Ctx, s: &Surface, cmd: WishlistVisibilityCommand) -> Result<()> {
    let path = format!("{}/visibility", s.base);
    let v: WishlistVisibility = match cmd {
        WishlistVisibilityCommand::Show => ctx.client.get_json(&path, &[]).await?,
        WishlistVisibilityCommand::Set { public } => {
            let body = serde_json::json!({ "public": public });
            ctx.client.put_json(&path, body).await?
        }
    };
    if ctx.printer.json {
        ctx.printer.json(&v)?;
    } else {
        println!(
            "public: {}  ·  handle: {}",
            v.public,
            v.handle.as_deref().unwrap_or("(set a username first)")
        );
    }
    Ok(())
}
