//! Shared engine for the collection + wish-list surfaces. They are independent
//! tables that share the same wire shapes and route shape (only the base path and
//! the batch-count route name differ), so the card- and product-holding operations
//! live here once, parameterised by a [`Surface`].

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};

use super::{CardExportFormat, Ctx, push_flag, push_opt};
use crate::models::*;
use crate::output::{self, collection_summary, collection_table, product_holdings_table, table};

/// Identifies one holdings surface (collection or wish list) for a game.
pub struct Surface {
    /// e.g. `/api/collection/mtg`.
    pub base: String,
    /// Batch card-count route leaf: `owned` (collection) or `counts` (wish list).
    pub batch_route: &'static str,
    /// Batch product-count route leaf.
    pub product_batch_route: &'static str,
    /// Column label for the primary count: `Owned` or `Wanted`.
    pub noun: &'static str,
}

/// Which per-card counter the copy bounds read.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HoldingsFinish {
    /// Regular + foil copies together (the default).
    Any,
    /// Regular copies only — and at least one must be held.
    Regular,
    /// Foil copies only — and at least one must be held.
    Foil,
}

impl HoldingsFinish {
    pub fn as_str(self) -> &'static str {
        match self {
            HoldingsFinish::Any => "any",
            HoldingsFinish::Regular => "regular",
            HoldingsFinish::Foil => "foil",
        }
    }
}

/// The copy-count filters every holdings card listing takes (the list, the card
/// export, the per-set drop/sub-type groupings and the wish-list buy list).
#[derive(Debug, Clone, Copy, Args)]
pub struct CopyFilter {
    /// Keep only cards held in at least this many copies (of the `--finish`
    /// counter).
    #[arg(long, value_name = "N")]
    pub min_copies: Option<i64>,
    /// Keep only cards held in at most this many copies (of the `--finish`
    /// counter).
    #[arg(long, value_name = "N")]
    pub max_copies: Option<i64>,
    /// Which counter the copy bounds read: any (regular + foil, default), regular,
    /// or foil — the latter two also require at least one copy of that finish.
    #[arg(long, value_enum)]
    pub finish: Option<HoldingsFinish>,
}

impl CopyFilter {
    /// Push whichever bounds were given onto a query vec.
    pub fn push(&self, q: &mut Vec<(&'static str, String)>) {
        push_opt(q, "min_copies", &self.min_copies);
        push_opt(q, "max_copies", &self.max_copies);
        if let Some(f) = self.finish {
            q.push(("finish", f.as_str().to_string()));
        }
    }
}

/// The full filter set of a holdings card search — the search itself plus the
/// copy-count bounds. Shared by `list`, `export-cards` and the wish list's
/// `buy-list`, which all honour exactly the same parameters.
#[derive(Debug, Clone, Args)]
pub struct ListFilter {
    /// Scryfall-style search filter.
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    /// Scope to one set code.
    #[arg(long)]
    pub set: Option<String>,
    /// With `--set`, span the set's whole group.
    #[arg(long)]
    pub related: bool,
    /// Sort key: updated | quantity | name | rarity | released | cmc | price.
    #[arg(long)]
    pub sort: Option<String>,
    /// Direction: asc | desc.
    #[arg(long)]
    pub dir: Option<String>,
    #[command(flatten)]
    pub copies: CopyFilter,
}

impl ListFilter {
    /// Push every filter that was given onto a query vec.
    pub fn push(&self, q: &mut Vec<(&'static str, String)>) {
        push_opt(q, "q", &self.query);
        push_opt(q, "set", &self.set);
        push_opt(q, "sort", &self.sort);
        push_opt(q, "dir", &self.dir);
        push_flag(q, "include_related", self.related);
        self.copies.push(q);
    }
}

/// Product-holding subcommands, identical between the two surfaces.
#[derive(Debug, Subcommand)]
pub enum ProductHoldingCommand {
    /// List held sealed products.
    List {
        #[arg(long)]
        set: Option<String>,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// Show the counts for one product.
    Get { product_id: String },
    /// Set absolute counts for one product (both zero removes it).
    Set {
        product_id: String,
        #[arg(long, default_value_t = 0)]
        qty: i64,
        #[arg(long, default_value_t = 0)]
        foil: i64,
    },
    /// Aggregate summary of held products.
    Summary,
    /// Per-set aggregate tiles.
    Sets,
    /// Batch counts for the given product ids.
    Counts { ids: Vec<String> },
}

// -- card holdings ----------------------------------------------------------

pub async fn list(
    ctx: &Ctx,
    s: &Surface,
    filter: ListFilter,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    filter.push(&mut q);
    push_opt(&mut q, "page", &page);
    push_opt(&mut q, "page_size", &page_size);
    let page: Page<CollectionEntry> = ctx.client.get_json(&s.base, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&page)?;
    } else {
        collection_table(&page.data, s.noun);
        ctx.printer.note(format!(
            "page {} · {} cards total{}",
            page.page,
            page.total,
            if page.has_more {
                " · more (--page)"
            } else {
                ""
            }
        ));
    }
    Ok(())
}

pub async fn get(ctx: &Ctx, s: &Surface, card_id: &str) -> Result<()> {
    let path = format!("{}/cards/{}", s.base, card_id);
    let q: CollectionQuantities = ctx.client.get_json(&path, &[]).await?;
    print_quantities(ctx, &q);
    Ok(())
}

pub async fn set(ctx: &Ctx, s: &Surface, card_id: &str, qty: i64, foil: i64) -> Result<()> {
    let path = format!("{}/cards/{}", s.base, card_id);
    let body = serde_json::json!({ "quantity": qty, "foil_quantity": foil });
    let q: CollectionQuantities = ctx.client.put_json(&path, body).await?;
    if !ctx.printer.json {
        if q.quantity == 0 && q.foil_quantity == 0 {
            ctx.printer.note("Removed.");
        } else {
            ctx.printer.note(format!(
                "Set to {} regular / {} foil.",
                q.quantity, q.foil_quantity
            ));
        }
    } else {
        ctx.printer.json(&q)?;
    }
    Ok(())
}

pub async fn add(ctx: &Ctx, s: &Surface, card_id: &str, qty: i64, foil: i64) -> Result<()> {
    let path = format!("{}/cards/{}", s.base, card_id);
    let current: CollectionQuantities = ctx.client.get_json(&path, &[]).await?;
    set(
        ctx,
        s,
        card_id,
        current.quantity + qty,
        current.foil_quantity + foil,
    )
    .await
}

/// `bulk_max` is the per-unit price cutoff (USD cents) that splits the bulk
/// subtotal out of the total; only the public summaries document it, so the
/// collection/wish-list callers pass `None`.
pub async fn summary(
    ctx: &Ctx,
    s: &Surface,
    set: Option<String>,
    related: bool,
    bulk_max: Option<i64>,
) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "set", &set);
    push_flag(&mut q, "include_related", related);
    push_opt(&mut q, "bulk_max_cents", &bulk_max);
    let path = format!("{}/summary", s.base);
    let summary: CollectionSummary = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&summary)?;
    } else {
        collection_summary(&summary);
    }
    Ok(())
}

/// Per-set aggregate tiles. `bulk_max` is the per-unit price cutoff (USD cents,
/// default $1) that splits each tile's bulk subtotal out of its value.
pub async fn sets(ctx: &Ctx, s: &Surface, bulk_max: Option<i64>) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "bulk_max_cents", &bulk_max);
    let path = format!("{}/sets", s.base);
    let body: DataBody<Vec<CollectionSet>> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else {
        let mut t = table(&["Code", "Name", "Cards", "Copies", "Value", "Bulk"]);
        for cs in &body.data {
            t.add_row(vec![
                cs.code.to_uppercase(),
                output::truncate(&cs.name, 36),
                cs.owned_cards.to_string(),
                cs.owned_copies.to_string(),
                output::price(&cs.owned_value_usd),
                output::price(&cs.owned_bulk_value_usd),
            ]);
        }
        println!("{t}");
    }
    Ok(())
}

/// Where the holdings' value sits: copies + value by rarity, colour identity, card
/// type and finish, plus the ten most valuable holdings by *held* value. `bulk_max`
/// is the per-unit price cutoff (USD cents, default $1) the embedded summary's bulk
/// subtotal splits on. Authed surfaces only — the public mirrors don't expose it.
pub async fn breakdown(ctx: &Ctx, s: &Surface, bulk_max: Option<i64>) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "bulk_max_cents", &bulk_max);
    let path = format!("{}/breakdown", s.base);
    let b: CollectionBreakdown = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&b)?;
        return Ok(());
    }
    collection_summary(&b.summary);
    facet_table("By rarity", &b.rarity);
    facet_table("By colour identity", &b.color);
    facet_table("By card type", &b.card_type);
    facet_table("By finish", &b.finish);
    if !b.top.is_empty() {
        println!("\nTop holdings (by held value):");
        let mut t = table(&["Name", "Set", "#", "Qty", "Foil", "Held value"]);
        for h in &b.top {
            t.add_row(vec![
                output::truncate(&h.card.name, 32),
                h.card.set_code.to_uppercase(),
                h.card.collector_number.clone(),
                h.quantity.to_string(),
                h.foil_quantity.to_string(),
                format!("${}", h.value_usd),
            ]);
        }
        println!("{t}");
    }
    if b.unpriced_cards > 0 {
        println!(
            "\n{} card(s) add nothing above — no finish they're held in is priced.",
            b.unpriced_cards
        );
    }
    Ok(())
}

/// One breakdown facet as a small table, skipped when the facet is empty.
fn facet_table(label: &str, buckets: &[BreakdownBucket]) {
    if buckets.is_empty() {
        return;
    }
    println!("\n{label}:");
    let mut t = table(&["Key", "Cards", "Copies", "Value"]);
    for b in buckets {
        t.add_row(vec![
            b.key.clone(),
            b.cards.to_string(),
            b.copies.to_string(),
            output::price(&b.value_usd),
        ]);
    }
    println!("{t}");
}

pub async fn set_drops(
    ctx: &Ctx,
    s: &Surface,
    code: &str,
    query: Option<String>,
    copies: CopyFilter,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "q", &query);
    copies.push(&mut q);
    push_opt(&mut q, "page", &page);
    push_opt(&mut q, "page_size", &page_size);
    let path = format!("{}/sets/{}/drops", s.base, code);
    let page: Page<CollectionDropGroup> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&page)?;
    } else {
        for g in &page.data {
            println!("\n== {} ({} cards) ==", g.title, g.card_count);
            collection_table(&g.cards, s.noun);
        }
    }
    Ok(())
}

pub async fn set_subtypes(
    ctx: &Ctx,
    s: &Surface,
    code: &str,
    query: Option<String>,
    copies: CopyFilter,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "q", &query);
    copies.push(&mut q);
    push_opt(&mut q, "page", &page);
    push_opt(&mut q, "page_size", &page_size);
    let path = format!("{}/sets/{}/subtypes", s.base, code);
    let page: Page<CollectionSubtypeGroup> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&page)?;
    } else {
        for g in &page.data {
            println!("\n== {} ({} cards) ==", g.title, g.card_count);
            collection_table(&g.cards, s.noun);
        }
    }
    Ok(())
}

/// Export the whole result set of a holdings card search as a `.txt` deck-list —
/// the browse's mirror of the catalog's card-search export, with the real held
/// counts on each line so the file round-trips through the text importer.
pub async fn export_cards(
    ctx: &Ctx,
    s: &Surface,
    filter: ListFilter,
    format: CardExportFormat,
    output: Option<PathBuf>,
) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    filter.push(&mut q);
    let path = format!("{}/cards/export", s.base);
    super::export_text(ctx, &path, q, format, output).await
}

pub async fn batch_counts(ctx: &Ctx, s: &Surface, ids: Vec<String>) -> Result<()> {
    let path = format!("{}/{}", s.base, s.batch_route);
    let body = serde_json::json!({ "ids": ids });
    let resp: DataBody<BTreeMap<String, CollectionQuantities>> =
        ctx.client.post_json(&path, body).await?;
    print_counts_map(ctx, &resp.data);
    Ok(())
}

// -- product holdings -------------------------------------------------------

pub async fn products(ctx: &Ctx, s: &Surface, cmd: ProductHoldingCommand) -> Result<()> {
    let base = format!("{}/products", s.base);
    match cmd {
        ProductHoldingCommand::List {
            set,
            page,
            page_size,
        } => products_list(ctx, s, set, page, page_size).await?,
        ProductHoldingCommand::Get { product_id } => {
            let q: CollectionQuantities = ctx
                .client
                .get_json(&format!("{base}/{product_id}"), &[])
                .await?;
            print_quantities(ctx, &q);
        }
        ProductHoldingCommand::Set {
            product_id,
            qty,
            foil,
        } => {
            let body = serde_json::json!({ "quantity": qty, "foil_quantity": foil });
            let q: CollectionQuantities = ctx
                .client
                .put_json(&format!("{base}/{product_id}"), body)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&q)?;
            } else if q.quantity == 0 && q.foil_quantity == 0 {
                ctx.printer.note("Removed.");
            } else {
                ctx.printer
                    .note(format!("Set to {} / {} foil.", q.quantity, q.foil_quantity));
            }
        }
        ProductHoldingCommand::Summary => products_summary(ctx, s).await?,
        ProductHoldingCommand::Sets => products_sets(ctx, s).await?,
        ProductHoldingCommand::Counts { ids } => {
            let body = serde_json::json!({ "ids": ids });
            let resp: DataBody<BTreeMap<String, CollectionQuantities>> = ctx
                .client
                .post_json(&format!("{base}/{}", s.product_batch_route), body)
                .await?;
            print_counts_map(ctx, &resp.data);
        }
    }
    Ok(())
}

// -- product read helpers (shared with the public, read-only surfaces) -------

pub async fn products_list(
    ctx: &Ctx,
    s: &Surface,
    set: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "set", &set);
    push_opt(&mut q, "page", &page);
    push_opt(&mut q, "page_size", &page_size);
    let page: Page<ProductHoldingEntry> = ctx
        .client
        .get_json(&format!("{}/products", s.base), &q)
        .await?;
    if ctx.printer.json {
        ctx.printer.json(&page)?;
    } else {
        product_holdings_table(&page.data);
        ctx.printer.note(format!(
            "page {} · {} products total{}",
            page.page,
            page.total,
            if page.has_more {
                " · more (--page)"
            } else {
                ""
            }
        ));
    }
    Ok(())
}

pub async fn products_summary(ctx: &Ctx, s: &Surface) -> Result<()> {
    let summary: ProductHoldingSummary = ctx
        .client
        .get_json(&format!("{}/products/summary", s.base), &[])
        .await?;
    if ctx.printer.json {
        ctx.printer.json(&summary)?;
    } else {
        println!("Unique products : {}", summary.unique_products);
        println!("Total products  : {}", summary.total_products);
        println!(
            "Total value     : {}",
            output::price(&summary.total_value_usd)
        );
    }
    Ok(())
}

pub async fn products_sets(ctx: &Ctx, s: &Surface) -> Result<()> {
    let body: DataBody<Vec<ProductHoldingSet>> = ctx
        .client
        .get_json(&format!("{}/products/sets", s.base), &[])
        .await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else {
        let mut t = table(&["Set", "Name", "Unique", "Total", "Value"]);
        for ps in &body.data {
            t.add_row(vec![
                ps.code.to_uppercase(),
                output::dash(&ps.name),
                ps.unique_products.to_string(),
                ps.total_products.to_string(),
                output::price(&ps.total_value_usd),
            ]);
        }
        println!("{t}");
    }
    Ok(())
}

// -- shared printing --------------------------------------------------------

fn print_quantities(ctx: &Ctx, q: &CollectionQuantities) {
    if ctx.printer.json {
        let _ = ctx.printer.json(q);
    } else {
        println!("regular: {}   foil: {}", q.quantity, q.foil_quantity);
    }
}

fn print_counts_map(ctx: &Ctx, map: &BTreeMap<String, CollectionQuantities>) {
    if ctx.printer.json {
        let _ = ctx.printer.json(map);
    } else if map.is_empty() {
        println!("(none held)");
    } else {
        let mut t = table(&["ID", "Regular", "Foil"]);
        for (id, q) in map {
            t.add_row(vec![
                id.clone(),
                q.quantity.to_string(),
                q.foil_quantity.to_string(),
            ]);
        }
        println!("{t}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copies(min: Option<i64>, max: Option<i64>, finish: Option<HoldingsFinish>) -> CopyFilter {
        CopyFilter {
            min_copies: min,
            max_copies: max,
            finish,
        }
    }

    #[test]
    fn copy_filter_pushes_only_what_was_given() {
        let mut q: Vec<(&str, String)> = Vec::new();
        copies(None, None, None).push(&mut q);
        assert!(q.is_empty());

        let mut q: Vec<(&str, String)> = Vec::new();
        copies(Some(4), Some(9), Some(HoldingsFinish::Foil)).push(&mut q);
        assert_eq!(
            q,
            vec![
                ("min_copies", "4".to_string()),
                ("max_copies", "9".to_string()),
                ("finish", "foil".to_string()),
            ]
        );
    }

    #[test]
    fn list_filter_carries_the_search_and_the_copy_bounds() {
        let f = ListFilter {
            query: Some("is:foil".into()),
            set: Some("blb".into()),
            related: true,
            sort: Some("price".into()),
            dir: None,
            copies: copies(Some(1), None, Some(HoldingsFinish::Regular)),
        };
        let mut q: Vec<(&str, String)> = Vec::new();
        f.push(&mut q);
        assert_eq!(
            q,
            vec![
                ("q", "is:foil".to_string()),
                ("set", "blb".to_string()),
                ("sort", "price".to_string()),
                ("include_related", "true".to_string()),
                ("min_copies", "1".to_string()),
                ("finish", "regular".to_string()),
            ]
        );
    }
}
