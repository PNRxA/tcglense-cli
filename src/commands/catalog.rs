//! Public catalog commands: games, sets, the release calendar, cards, combos,
//! prices, prints, sealed products (including their expected value and simulated
//! openings), scan, and image download.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Subcommand};

use super::{CardExportFormat, Ctx, page_footer, precons, push_flag, push_opt};
use crate::models::*;
use crate::output::{
    self, card_detail, card_detail_extras, cards_table, games_table, prices_table, products_table,
    sets_table, table,
};

// -- arg types --------------------------------------------------------------

#[derive(Debug, Args)]
pub struct SetsArgs {
    pub game: String,
}

#[derive(Debug, Args)]
pub struct SetArgs {
    pub game: String,
    pub code: String,
    /// List the set's cards instead of its metadata.
    #[arg(long)]
    pub cards: bool,
    /// List the set's Secret Lair drops (requires a drop-grouped set).
    #[arg(long)]
    pub drops: bool,
    /// List the set's cards grouped by sub-type (treatment).
    #[arg(long)]
    pub subtypes: bool,
    /// Scryfall-style filter query.
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    /// With --drops, keep only drops whose title matches (case-insensitive).
    #[arg(long, value_name = "TITLE")]
    pub drop: Option<String>,
    /// Span the set's whole group (root + related tokens/promos/decks).
    #[arg(long)]
    pub related: bool,
    /// With --cards, sort key: number | name | rarity | released | cmc | price.
    #[arg(long)]
    pub sort: Option<String>,
    /// Direction: asc | desc.
    #[arg(long)]
    pub dir: Option<String>,
    #[arg(long)]
    pub page: Option<u32>,
    #[arg(long)]
    pub page_size: Option<u32>,
}

#[derive(Debug, Args)]
pub struct CardsArgs {
    pub game: String,
    /// Scryfall-style search query.
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    /// Exact-name filter ("printings of this name").
    #[arg(long)]
    pub name: Option<String>,
    /// Scope to a single set code.
    #[arg(long)]
    pub set: Option<String>,
    /// With --set, span the set's whole group.
    #[arg(long)]
    pub related: bool,
    /// Sort key: name | number | rarity | released | cmc | price.
    #[arg(long)]
    pub sort: Option<String>,
    /// Direction: asc | desc.
    #[arg(long)]
    pub dir: Option<String>,
    #[arg(long)]
    pub page: Option<u32>,
    #[arg(long)]
    pub page_size: Option<u32>,
    /// Show only the first rows of the same search, skipping the (expensive)
    /// total — a quick peek rather than a page to turn. Not available with --set,
    /// and there is no page to turn, so --page / --page-size are rejected.
    #[arg(long, conflicts_with_all = ["page", "page_size"])]
    pub preview: bool,
    /// With --preview, rows to return (clamped to 1..=25 by the server; default 8).
    #[arg(long, requires = "preview", conflicts_with_all = ["page", "page_size"])]
    pub limit: Option<u32>,
}

#[derive(Debug, Args)]
pub struct CardArgs {
    pub game: String,
    pub id: String,
}

#[derive(Debug, Args)]
pub struct CombosArgs {
    pub game: String,
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ReleasesArgs {
    pub game: String,
    /// First day of the window, `YYYY-MM-DD`, inclusive (default: today).
    #[arg(long)]
    pub from: Option<String>,
    /// Last day of the window, `YYYY-MM-DD`, inclusive (default: `from` + 90 days;
    /// the window spans at most 366 days).
    #[arg(long)]
    pub to: Option<String>,
}

#[derive(Debug, Args)]
pub struct CardNamesArgs {
    pub game: String,
    pub query: String,
    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub game: String,
    /// Words every match's name must contain (any order, case-insensitive).
    pub query: String,
    /// Max matches per group (clamped to 1..=10; default 5).
    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Debug, Args)]
pub struct PricesArgs {
    pub game: String,
    pub id: String,
    /// Window: 7d | 30d | 1y | 2y | 3y | all (default: full daily series).
    #[arg(long)]
    pub range: Option<String>,
}

#[derive(Debug, Args)]
pub struct PrintsArgs {
    pub game: String,
    pub id: String,
}

#[derive(Debug, Args)]
pub struct SealedArgs {
    pub game: String,
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RulingsArgs {
    pub game: String,
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ScanArgs {
    pub game: String,
    /// A file containing the 32-byte fingerprint (raw bytes or hex text).
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// The 32-byte fingerprint as a 64-char hex string.
    #[arg(long)]
    pub hex: Option<String>,
    /// How many candidate matches to return.
    #[arg(long)]
    pub top_k: Option<i64>,
}

#[derive(Debug, Args)]
pub struct ProductsArgs {
    pub game: String,
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    #[arg(long)]
    pub set: Option<String>,
    #[arg(long = "type")]
    pub type_: Option<String>,
    /// Sort key: name | price | released.
    #[arg(long)]
    pub sort: Option<String>,
    /// Direction: asc | desc.
    #[arg(long)]
    pub dir: Option<String>,
    #[arg(long)]
    pub page: Option<u32>,
    #[arg(long)]
    pub page_size: Option<u32>,
    /// Show the game's product facets (types + sets) instead of a product list.
    #[arg(long)]
    pub facets: bool,
}

#[derive(Debug, Args)]
pub struct ProductArgs {
    pub game: String,
    pub id: String,
    #[command(subcommand)]
    pub command: Option<ProductCommand>,
}

#[derive(Debug, Subcommand)]
pub enum ProductCommand {
    /// Price history.
    Prices {
        #[arg(long)]
        range: Option<String>,
    },
    /// Structural composition ("what's in the box").
    Contents,
    /// Parent products that contain this one.
    Containers,
    /// Cards the product contains / can yield.
    Cards {
        /// Restrict to one display section: contains | exclusive | booster | variable.
        #[arg(long)]
        section: Option<String>,
        /// Page the cards packed in one unlisted box component instead (a
        /// `component` value from `sections`); a name matching none is an empty page.
        #[arg(long)]
        component: Option<String>,
        /// Scryfall-style filter narrowing the product's cards.
        #[arg(short = 'q', long)]
        query: Option<String>,
        /// Sort key: name | rarity | cmc | price | … (re-orders within each section).
        #[arg(long)]
        sort: Option<String>,
        /// Direction: asc | desc.
        #[arg(long)]
        dir: Option<String>,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        page_size: Option<u32>,
    },
    /// The non-empty card display sections + counts.
    Sections {
        /// Scryfall-style filter — the manifest narrows to matching sections + counts.
        #[arg(short = 'q', long)]
        query: Option<String>,
    },
    /// Expected value of one copy at today's prices, per booster and per sheet.
    Ev,
    /// Simulate opening the product — stateless and seeded, so the same seed
    /// deals the same packs.
    Open {
        /// Roll seed; omit for a fresh random one (the result echoes it back).
        #[arg(long)]
        seed: Option<i64>,
        /// How many copies of the product to open (default 1).
        #[arg(long)]
        copies: Option<i64>,
    },
}

#[derive(Debug, Args)]
pub struct IngestArgs {
    pub game: String,
}

#[derive(Debug, Args)]
pub struct FormatsArgs {
    pub game: String,
    /// Only the most-played formats.
    #[arg(long)]
    pub popular: bool,
}

#[derive(Debug, Args)]
pub struct KeywordsArgs {
    pub game: String,
    /// Show each entry's full explanation instead of a one-line table.
    #[arg(long)]
    pub full: bool,
}

#[derive(Debug, Args)]
pub struct ArtTagsArgs {
    pub game: String,
    /// Substring to match tag slugs/labels against; omit for the game's full tag list.
    #[arg(short = 'q', long, conflicts_with = "card")]
    pub query: Option<String>,
    /// Max suggestions for a `-q` lookup (clamped to 1..=50; ignored without `-q`).
    #[arg(long, conflicts_with = "card")]
    pub limit: Option<u32>,
    /// Show the art tags on one card's artwork instead of the game's tag list.
    #[arg(long, value_name = "CARD_ID")]
    pub card: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub command: ExportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    /// Export a whole card-search result set as a `.txt` deck-list.
    Cards {
        game: String,
        /// Scryfall-style search filter.
        #[arg(short = 'q', long)]
        query: Option<String>,
        /// Exact-name filter (matched literally).
        #[arg(long)]
        name: Option<String>,
        /// Sort key: name | number | rarity | released | cmc | price.
        #[arg(long)]
        sort: Option<String>,
        /// Direction: asc | desc.
        #[arg(long)]
        dir: Option<String>,
        #[arg(long, value_enum, default_value_t = CardExportFormat::Text)]
        format: CardExportFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Export a set's card-search result set as a `.txt` deck-list.
    Set {
        game: String,
        code: String,
        /// Scryfall-style search filter.
        #[arg(short = 'q', long)]
        query: Option<String>,
        /// Span the set's whole group (root + related sub-sets).
        #[arg(long)]
        related: bool,
        /// Sort key: number | name | rarity | released | cmc | price.
        #[arg(long)]
        sort: Option<String>,
        /// Direction: asc | desc.
        #[arg(long)]
        dir: Option<String>,
        #[arg(long, value_enum, default_value_t = CardExportFormat::Text)]
        format: CardExportFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
pub struct ImageArgs {
    #[command(subcommand)]
    pub command: ImageCommand,
}

#[derive(Debug, Subcommand)]
pub enum ImageCommand {
    /// Download a card image.
    Card {
        game: String,
        id: String,
        /// small | normal | large | png | art_crop.
        #[arg(long)]
        size: Option<String>,
        /// Face index for double-faced cards.
        #[arg(long)]
        face: Option<u32>,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Download a sealed-product image.
    Product {
        game: String,
        id: String,
        /// normal | small.
        #[arg(long)]
        size: Option<String>,
        #[arg(short, long)]
        output: PathBuf,
    },
}

// -- handlers ---------------------------------------------------------------

pub async fn games(ctx: &Ctx) -> Result<()> {
    let body: DataBody<Vec<Game>> = ctx.client.get_json("/api/games", &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else {
        games_table(&body.data);
    }
    Ok(())
}

pub async fn sets(ctx: &Ctx, args: SetsArgs) -> Result<()> {
    let path = format!("/api/games/{}/sets", args.game);
    let body: DataBody<Vec<CardSet>> = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else {
        sets_table(&body.data);
        ctx.printer.note(format!("{} sets.", body.data.len()));
    }
    Ok(())
}

/// The release calendar for a window: the sets landing inside it (with the precons
/// and sealed products they ship) and the Secret Lair drops, both date-ascending.
pub async fn releases(ctx: &Ctx, args: ReleasesArgs) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "from", &args.from);
    push_opt(&mut q, "to", &args.to);
    let path = format!("/api/games/{}/releases", args.game);
    let r: Releases = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        return ctx.printer.json(&r);
    }
    println!("Releases {} → {}", r.from, r.to);
    if r.sets.is_empty() && r.secret_lair_drops.is_empty() {
        println!("Nothing releases in this window.");
        return Ok(());
    }

    if !r.sets.is_empty() {
        let mut t = table(&[
            "Date", "Code", "Name", "Type", "Cards", "Precons", "Products",
        ]);
        for s in &r.sets {
            t.add_row(vec![
                s.released_at.clone(),
                s.set.code.to_uppercase(),
                output::truncate(&s.set.name, 40),
                output::dash(&s.set.set_type),
                s.set.card_count.to_string(),
                s.precons.len().to_string(),
                s.products.len().to_string(),
            ]);
        }
        println!("{t}");
        // What each set ships, named — the counts above say how many, not which.
        for s in &r.sets {
            if s.precons.is_empty() && s.products.is_empty() {
                continue;
            }
            println!("\n== {} ({}) ==", s.set.name, s.set.code.to_uppercase());
            if !s.precons.is_empty() {
                let names: Vec<&str> = s.precons.iter().map(|p| p.name.as_str()).collect();
                println!("  Precons : {}", output::truncate(&names.join(", "), 100));
            }
            if !s.products.is_empty() {
                let names: Vec<&str> = s.products.iter().map(|p| p.name.as_str()).collect();
                println!("  Products: {}", output::truncate(&names.join(", "), 100));
            }
        }
    }

    if !r.secret_lair_drops.is_empty() {
        println!("\n== Secret Lair drops ==");
        let mut t = table(&["Date", "Title", "Slug", "Products"]);
        for d in &r.secret_lair_drops {
            t.add_row(vec![
                d.released_at.clone(),
                output::truncate(&d.title, 44),
                d.slug.clone(),
                d.products.len().to_string(),
            ]);
        }
        println!("{t}");
    }
    ctx.printer.note(format!(
        "\n{} set(s) · {} Secret Lair drop(s).",
        r.sets.len(),
        r.secret_lair_drops.len()
    ));
    Ok(())
}

pub async fn set(ctx: &Ctx, args: SetArgs) -> Result<()> {
    let base = format!("/api/games/{}/sets/{}", args.game, args.code);
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "q", &args.query);
    push_opt(&mut q, "page", &args.page);
    push_opt(&mut q, "page_size", &args.page_size);

    if args.drops {
        push_opt(&mut q, "drop", &args.drop);
        let page: Page<DropGroup> = ctx.client.get_json(&format!("{base}/drops"), &q).await?;
        if ctx.printer.json {
            ctx.printer.json(&page)?;
        } else {
            for g in &page.data {
                println!(
                    "\n== {} ({} cards, {}{}) ==",
                    g.title,
                    g.card_count,
                    output::price(&g.cheapest_prints_usd),
                    match &g.released_at {
                        Some(d) => format!(", {d}"),
                        None => String::new(),
                    }
                );
                cards_table(&g.cards);
            }
            page_footer(ctx, page.page, page.total, page.has_more, "drops");
        }
    } else if args.subtypes {
        let page: Page<SubtypeGroup> = ctx.client.get_json(&format!("{base}/subtypes"), &q).await?;
        if ctx.printer.json {
            ctx.printer.json(&page)?;
        } else {
            for g in &page.data {
                println!("\n== {} ({} cards) ==", g.title, g.card_count);
                cards_table(&g.cards);
            }
            page_footer(ctx, page.page, page.total, page.has_more, "sub-types");
        }
    } else if args.cards {
        push_flag(&mut q, "include_related", args.related);
        push_opt(&mut q, "sort", &args.sort);
        push_opt(&mut q, "dir", &args.dir);
        let page: Page<Card> = ctx.client.get_json(&format!("{base}/cards"), &q).await?;
        print_card_page(ctx, page);
    } else {
        let set: CardSet = ctx.client.get_json(&base, &[]).await?;
        if ctx.printer.json {
            ctx.printer.json(&set)?;
        } else {
            sets_table(std::slice::from_ref(&set));
            println!(
                "drops: {}   sub-types: {}   parent: {}",
                set.has_drops,
                set.has_subtypes,
                set.parent_set_code.as_deref().unwrap_or("—")
            );
        }
    }
    Ok(())
}

pub async fn cards(ctx: &Ctx, args: CardsArgs) -> Result<()> {
    if args.preview {
        return cards_preview(ctx, args).await;
    }
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "q", &args.query);
    push_opt(&mut q, "sort", &args.sort);
    push_opt(&mut q, "dir", &args.dir);
    push_opt(&mut q, "page", &args.page);
    push_opt(&mut q, "page_size", &args.page_size);

    let path = if let Some(set) = &args.set {
        push_flag(&mut q, "include_related", args.related);
        format!("/api/games/{}/sets/{}/cards", args.game, set)
    } else {
        push_opt(&mut q, "name", &args.name);
        format!("/api/games/{}/cards", args.game)
    };
    let page: Page<Card> = ctx.client.get_json(&path, &q).await?;
    print_card_page(ctx, page);
    Ok(())
}

/// The first `--limit` rows of the same search the listing pages through, without
/// the `COUNT(*)` a page's total costs — a peek, not a page to turn. `has_more` is
/// honest (the API over-fetches one row) but there is no total and no page two.
async fn cards_preview(ctx: &Ctx, args: CardsArgs) -> Result<()> {
    if args.set.is_some() {
        bail!("--preview is not available with --set; drop --set (or use `set <code> --cards`)");
    }
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "q", &args.query);
    push_opt(&mut q, "name", &args.name);
    push_opt(&mut q, "sort", &args.sort);
    push_opt(&mut q, "dir", &args.dir);
    push_opt(&mut q, "limit", &args.limit);
    let path = format!("/api/games/{}/cards/preview", args.game);
    let group: SearchGroup<Card> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        return ctx.printer.json(&group);
    }
    if group.data.is_empty() {
        println!("No matches.");
        return Ok(());
    }
    cards_table(&group.data);
    ctx.printer.note(format!(
        "showing {}{}",
        group.data.len(),
        if group.has_more {
            " · more match — use the paged listing"
        } else {
            ""
        }
    ));
    Ok(())
}

pub async fn card(ctx: &Ctx, args: CardArgs) -> Result<()> {
    let path = format!("/api/games/{}/cards/{}", args.game, args.id);
    let detail: CardDetail = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&detail)?;
    } else {
        card_detail(&detail.card);
        card_detail_extras(&detail);
    }
    Ok(())
}

pub async fn card_names(ctx: &Ctx, args: CardNamesArgs) -> Result<()> {
    let mut q: Vec<(&str, String)> = vec![("q", args.query)];
    push_opt(&mut q, "limit", &args.limit);
    let path = format!("/api/games/{}/card-names", args.game);
    let body: DataBody<Vec<String>> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else {
        for n in &body.data {
            println!("{n}");
        }
    }
    Ok(())
}

pub async fn prices(ctx: &Ctx, args: PricesArgs) -> Result<()> {
    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "range", &args.range);
    let path = format!("/api/games/{}/cards/{}/prices", args.game, args.id);
    let body: DataBody<Vec<PricePoint>> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else if body.data.is_empty() {
        println!("No price history in range.");
    } else {
        prices_table(&body.data);
    }
    Ok(())
}

pub async fn prints(ctx: &Ctx, args: PrintsArgs) -> Result<()> {
    let path = format!("/api/games/{}/cards/{}/prints", args.game, args.id);
    let body: DataBody<Vec<Card>> = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else if body.data.is_empty() {
        println!("No other printings.");
    } else {
        cards_table(&body.data);
    }
    Ok(())
}

pub async fn rulings(ctx: &Ctx, args: RulingsArgs) -> Result<()> {
    let path = format!("/api/games/{}/cards/{}/rulings", args.game, args.id);
    let body: DataBody<Vec<Ruling>> = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else if body.data.is_empty() {
        println!("No rulings.");
    } else {
        let mut t = table(&["Date", "Source", "Ruling"]);
        for r in &body.data {
            t.add_row(vec![
                r.published_at.clone(),
                r.source.clone(),
                output::truncate(&r.comment, 88),
            ]);
        }
        println!("{t}");
    }
    Ok(())
}

/// The Commander Spellbook combos a card is a piece of, most-played first. Keyed
/// by the card's gameplay identity, so every printing answers the same list; the
/// source's attribution rides under the table because its terms ask for it.
pub async fn combos(ctx: &Ctx, args: CombosArgs) -> Result<()> {
    let path = format!("/api/games/{}/cards/{}/combos", args.game, args.id);
    let r: CardCombos = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        return ctx.printer.json(&r);
    }
    if r.combos.is_empty() {
        println!("No combos (or no combo data synced).");
    } else {
        let mut t = table(&["Pieces", "Produces", "Popularity", "Bracket", "ID"]);
        for c in &r.combos {
            let pieces: Vec<String> = c
                .pieces
                .iter()
                .map(|p| {
                    let qty = if p.quantity > 1 {
                        format!("{}× ", p.quantity)
                    } else {
                        String::new()
                    };
                    let cmdr = if p.must_be_commander { " (cmdr)" } else { "" };
                    format!("{qty}{}{cmdr}", p.name)
                })
                // A template ("A free sacrifice outlet") is a requirement, not a
                // card to go and buy — bracket it, as the deck surface does.
                .chain(c.templates.iter().map(|t| format!("[{t}]")))
                .collect();
            t.add_row(vec![
                output::truncate(&pieces.join(" + "), 56),
                output::truncate(&c.produces.join(", "), 36),
                c.popularity.to_string(),
                output::dash(&c.bracket_tag),
                c.id.clone(),
            ]);
        }
        println!("{t}");
        ctx.printer.note(format!(
            "{} combo(s) total{}.",
            r.total,
            if r.total > r.combos.len() as i64 {
                format!(" · {} listed (the API caps the list)", r.combos.len())
            } else {
                String::new()
            }
        ));
    }
    // The data source's terms require the attribution, combos or not.
    ctx.printer
        .note(format!("Combo data: {} · {}", r.source, r.source_url));
    Ok(())
}

pub async fn keywords(ctx: &Ctx, args: KeywordsArgs) -> Result<()> {
    let path = format!("/api/games/{}/keywords", args.game);
    let body: DataBody<Vec<Keyword>> = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
        return Ok(());
    }
    if body.data.is_empty() {
        println!("No curated glossary for this game yet.");
        return Ok(());
    }
    if args.full {
        for k in &body.data {
            println!(
                "{}  [{}{}]",
                k.name,
                k.kind,
                if k.parameterized {
                    ", takes a value"
                } else {
                    ""
                }
            );
            println!("  {}\n", k.text);
        }
    } else {
        keywords_table(&body.data);
        ctx.printer.note(format!(
            "{} entries (--full for each explanation).",
            body.data.len()
        ));
    }
    Ok(())
}

fn keywords_table(entries: &[Keyword]) {
    let mut t = table(&["Name", "Kind", "Slug", "Explanation"]);
    for k in entries {
        t.add_row(vec![
            k.name.clone(),
            k.kind.clone(),
            k.slug.clone(),
            output::truncate(&k.text, 72),
        ]);
    }
    println!("{t}");
}

/// One search across everything the catalog names — cards (one per distinct
/// name), sets, sealed products, preconstructed decks and rules keywords — each
/// group capped at `--limit` and flagging whether more matched than fit.
pub async fn search(ctx: &Ctx, args: SearchArgs) -> Result<()> {
    let mut q: Vec<(&str, String)> = vec![("q", args.query)];
    push_opt(&mut q, "limit", &args.limit);
    let r: SearchResults = ctx
        .client
        .get_json(&format!("/api/games/{}/search", args.game), &q)
        .await?;
    if ctx.printer.json {
        return ctx.printer.json(&r);
    }
    // Each group renders with the table its own listing uses.
    let shown = [
        search_group("Cards", &r.cards, cards_table),
        search_group("Sets", &r.sets, sets_table),
        search_group("Sealed products", &r.products, products_table),
        search_group("Preconstructed decks", &r.precons, precons::precons_table),
        search_group("Keywords", &r.keywords, keywords_table),
    ];
    if !shown.contains(&true) {
        println!("No matches.");
    }
    Ok(())
}

/// Render one group of a search answer under a heading — nothing at all when it
/// is empty — and say whether anything was shown.
fn search_group<T>(label: &str, group: &SearchGroup<T>, render: fn(&[T])) -> bool {
    if group.data.is_empty() {
        return false;
    }
    println!(
        "\n== {label} ({}{}) ==",
        group.data.len(),
        if group.has_more {
            ", more matched — raise --limit or narrow the query"
        } else {
            ""
        }
    );
    render(&group.data);
    true
}

/// The formats this game tracks deck legality for — the spellings `decks … create
/// --format` accepts, so a format can be checked without hard-coding the list.
pub async fn formats(ctx: &Ctx, args: FormatsArgs) -> Result<()> {
    let path = format!("/api/games/{}/formats", args.game);
    let body: DataBody<Vec<DeckFormat>> = ctx.client.get_json(&path, &[]).await?;
    let formats: Vec<&DeckFormat> = body
        .data
        .iter()
        .filter(|f| !args.popular || f.popular)
        .collect();
    if ctx.printer.json {
        ctx.printer.json(&formats)?;
    } else if formats.is_empty() {
        println!("No legality-tracked formats for this game.");
    } else {
        let mut t = table(&["Label", "Key", "Group", "Popular", "Also accepts"]);
        for f in &formats {
            t.add_row(vec![
                f.label.clone(),
                f.key.clone(),
                f.group.clone(),
                if f.popular { "yes" } else { "" }.to_string(),
                output::truncate(&f.aliases.join(", "), 40),
            ]);
        }
        println!("{t}");
        ctx.printer.note(format!("{} formats.", formats.len()));
    }
    Ok(())
}

pub async fn art_tags(ctx: &Ctx, args: ArtTagsArgs) -> Result<()> {
    let (path, q) = match &args.card {
        Some(id) => (
            format!("/api/games/{}/cards/{}/art-tags", args.game, id),
            Vec::new(),
        ),
        None => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "q", &args.query);
            push_opt(&mut q, "limit", &args.limit);
            (format!("/api/games/{}/art-tags", args.game), q)
        }
    };
    let body: DataBody<Vec<ArtTag>> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else if body.data.is_empty() {
        println!("No art tags.");
    } else {
        let mut t = table(&["Slug", "Label", "Artworks", "Description"]);
        for tag in &body.data {
            t.add_row(vec![
                tag.slug.clone(),
                output::truncate(&tag.label, 36),
                tag.count.to_string(),
                output::truncate(tag.description.as_deref().unwrap_or("—"), 48),
            ]);
        }
        println!("{t}");
        ctx.printer.note(format!(
            "{} tags · filter cards with `art:<slug>`.",
            body.data.len()
        ));
    }
    Ok(())
}

pub async fn export(ctx: &Ctx, args: ExportArgs) -> Result<()> {
    let (path, q, format, output) = match args.command {
        ExportCommand::Cards {
            game,
            query,
            name,
            sort,
            dir,
            format,
            output,
        } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "q", &query);
            push_opt(&mut q, "name", &name);
            push_opt(&mut q, "sort", &sort);
            push_opt(&mut q, "dir", &dir);
            (format!("/api/games/{game}/cards/export"), q, format, output)
        }
        ExportCommand::Set {
            game,
            code,
            query,
            related,
            sort,
            dir,
            format,
            output,
        } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "q", &query);
            push_flag(&mut q, "include_related", related);
            push_opt(&mut q, "sort", &sort);
            push_opt(&mut q, "dir", &dir);
            (
                format!("/api/games/{game}/sets/{code}/cards/export"),
                q,
                format,
                output,
            )
        }
    };
    super::export_text(ctx, &path, q, format, output).await
}

pub async fn sealed(ctx: &Ctx, args: SealedArgs) -> Result<()> {
    let path = format!("/api/games/{}/cards/{}/sealed", args.game, args.id);
    let body: DataBody<Vec<SealedProductRef>> = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&body.data)?;
    } else if body.data.is_empty() {
        println!("Not found in any sealed product.");
    } else {
        let mut t = table(&["Membership", "Foil", "Product", "Set", "USD"]);
        for r in &body.data {
            t.add_row(vec![
                r.membership.clone(),
                if r.foil { "yes" } else { "" }.to_string(),
                output::truncate(&r.product.name, 40),
                r.product.set_code.to_uppercase(),
                output::price(&r.product.prices.usd),
            ]);
        }
        println!("{t}");
    }
    Ok(())
}

pub async fn scan(ctx: &Ctx, args: ScanArgs) -> Result<()> {
    let bytes = read_fingerprint(&args)?;
    let numbers: Vec<u16> = bytes.iter().map(|b| *b as u16).collect();
    let body = serde_json::json!({
        "fingerprints": [numbers],
        "top_k": args.top_k,
    });
    let path = format!("/api/games/{}/scan", args.game);
    let resp: ScanResponseLocal = ctx.client.post_json(&path, body).await?;
    if ctx.printer.json {
        ctx.printer.json(&resp.data)?;
    } else if resp.data.is_empty() {
        println!("No match within the confidence radius.");
    } else {
        let mut t = table(&["Distance", "ID", "Name", "Set", "#"]);
        for m in &resp.data {
            t.add_row(vec![
                m.distance.to_string(),
                output::truncate(&m.card.id, 12),
                output::truncate(&m.card.name, 34),
                m.card.set_code.to_uppercase(),
                m.card.collector_number.clone(),
            ]);
        }
        println!("{t}");
    }
    Ok(())
}

pub async fn products(ctx: &Ctx, args: ProductsArgs) -> Result<()> {
    if args.facets {
        let path = format!("/api/games/{}/products/facets", args.game);
        let body: DataBody<ProductFacets> = ctx.client.get_json(&path, &[]).await?;
        if ctx.printer.json {
            ctx.printer.json(&body.data)?;
        } else {
            println!("Types: {}", body.data.types.join(", "));
            let mut t = table(&["Set", "Name", "Products"]);
            for s in &body.data.sets {
                t.add_row(vec![
                    s.code.to_uppercase(),
                    output::dash(&s.name),
                    s.product_count.to_string(),
                ]);
            }
            println!("{t}");
        }
        return Ok(());
    }

    let mut q: Vec<(&str, String)> = Vec::new();
    push_opt(&mut q, "q", &args.query);
    push_opt(&mut q, "set", &args.set);
    push_opt(&mut q, "type", &args.type_);
    push_opt(&mut q, "sort", &args.sort);
    push_opt(&mut q, "dir", &args.dir);
    push_opt(&mut q, "page", &args.page);
    push_opt(&mut q, "page_size", &args.page_size);
    let path = format!("/api/games/{}/products", args.game);
    let page: Page<Product> = ctx.client.get_json(&path, &q).await?;
    if ctx.printer.json {
        ctx.printer.json(&page)?;
    } else {
        products_table(&page.data);
        page_footer(ctx, page.page, page.total, page.has_more, "products");
    }
    Ok(())
}

pub async fn product(ctx: &Ctx, args: ProductArgs) -> Result<()> {
    let base = format!("/api/games/{}/products/{}", args.game, args.id);
    match args.command {
        None => {
            let p: Product = ctx.client.get_json(&base, &[]).await?;
            if ctx.printer.json {
                ctx.printer.json(&p)?;
            } else {
                product_detail(&p);
            }
        }
        Some(ProductCommand::Prices { range }) => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "range", &range);
            let body: DataBody<Vec<ProductPricePoint>> =
                ctx.client.get_json(&format!("{base}/prices"), &q).await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else {
                let mut t = table(&["Date", "USD", "Foil"]);
                for p in &body.data {
                    t.add_row(vec![
                        p.date.clone(),
                        output::price(&p.usd),
                        output::price(&p.usd_foil),
                    ]);
                }
                println!("{t}");
            }
        }
        Some(ProductCommand::Contents) => {
            let body: DataBody<Vec<ProductComponent>> = ctx
                .client
                .get_json(&format!("{base}/contents"), &[])
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else {
                let mut t = table(&["Kind", "Qty", "Name", "Links to"]);
                for c in &body.data {
                    let link = c
                        .product
                        .as_ref()
                        .map(|p| format!("product {}", p.id))
                        .or_else(|| c.card.as_ref().map(|cd| format!("card {}", cd.id)))
                        .unwrap_or_default();
                    t.add_row(vec![
                        c.kind.clone(),
                        c.quantity.to_string(),
                        output::truncate(&c.name, 40),
                        link,
                    ]);
                }
                println!("{t}");
            }
        }
        Some(ProductCommand::Containers) => {
            let body: DataBody<Vec<ProductContainer>> = ctx
                .client
                .get_json(&format!("{base}/containers"), &[])
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else {
                let mut t = table(&["Qty", "Parent product", "ID", "USD"]);
                for c in &body.data {
                    t.add_row(vec![
                        c.quantity.to_string(),
                        output::truncate(&c.product.name, 40),
                        c.product.id.clone(),
                        output::price(&c.product.prices.usd),
                    ]);
                }
                println!("{t}");
            }
        }
        Some(ProductCommand::Cards {
            section,
            component,
            query,
            sort,
            dir,
            page,
            page_size,
        }) => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "section", &section);
            push_opt(&mut q, "component", &component);
            push_opt(&mut q, "q", &query);
            push_opt(&mut q, "sort", &sort);
            push_opt(&mut q, "dir", &dir);
            push_opt(&mut q, "page", &page);
            push_opt(&mut q, "page_size", &page_size);
            let page: Page<ProductCardEntry> =
                ctx.client.get_json(&format!("{base}/cards"), &q).await?;
            if ctx.printer.json {
                ctx.printer.json(&page)?;
            } else {
                let mut t = table(&["Membership", "Excl", "Foil", "Name", "Set", "#"]);
                for e in &page.data {
                    t.add_row(vec![
                        e.membership.clone(),
                        if e.exclusive { "yes" } else { "" }.to_string(),
                        if e.foil { "yes" } else { "" }.to_string(),
                        output::truncate(&e.card.name, 32),
                        e.card.set_code.to_uppercase(),
                        e.card.collector_number.clone(),
                    ]);
                }
                println!("{t}");
                page_footer(ctx, page.page, page.total, page.has_more, "cards");
            }
        }
        Some(ProductCommand::Sections { query }) => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "q", &query);
            let body: DataBody<Vec<ProductCardSection>> = ctx
                .client
                .get_json(&format!("{base}/cards/sections"), &q)
                .await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else {
                let mut t = table(&[
                    "Section",
                    "Cards",
                    "Booster family",
                    "Component",
                    "Inherited",
                ]);
                for s in &body.data {
                    t.add_row(vec![
                        s.key.clone(),
                        s.total.to_string(),
                        output::dash(&s.booster_family),
                        output::dash(&s.component),
                        if s.inherited { "yes" } else { "" }.to_string(),
                    ]);
                }
                println!("{t}");
            }
        }
        Some(ProductCommand::Ev) => {
            let body: DataBody<Option<ProductEv>> =
                ctx.client.get_json(&format!("{base}/ev"), &[]).await?;
            if ctx.printer.json {
                ctx.printer.json(&body.data)?;
            } else {
                match &body.data {
                    // `data: null` — not a 404 — is the answer for a product with
                    // no booster data at all.
                    None => println!(
                        "No expected value — the product has no booster data (not a booster, or randomised contents)."
                    ),
                    Some(ev) => print_product_ev(ev),
                }
            }
        }
        Some(ProductCommand::Open { seed, copies }) => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "seed", &seed);
            push_opt(&mut q, "copies", &copies);
            let opening: ProductOpening = ctx.client.get_json(&format!("{base}/open"), &q).await?;
            if ctx.printer.json {
                ctx.printer.json(&opening)?;
            } else {
                print_opening(ctx, &opening);
            }
        }
    }
    Ok(())
}

/// One copy's expected value: the headline, then each booster it opens with the
/// sheets behind it, the biggest contributors across the copy, and the caveats
/// (which the API generates and asks callers to show).
fn print_product_ev(ev: &ProductEv) {
    println!("EV ${} per copy", ev.ev_usd);
    for p in &ev.packs {
        println!(
            "\n{}× {} ({}) · ${}/pack · {:.1} cards · priced share {:.1}%",
            p.quantity,
            p.name.as_deref().unwrap_or(&p.booster_code),
            p.set_code.to_uppercase(),
            p.ev_usd,
            p.cards_per_pack,
            p.priced_share * 100.0
        );
        if p.slots.is_empty() {
            continue;
        }
        let mut t = table(&["Sheet", "Foil", "Picks", "Cards", "EV", "Priced"]);
        for s in &p.slots {
            t.add_row(vec![
                output::truncate(&s.sheet, 32),
                if s.foil { "yes" } else { "" }.to_string(),
                format!("{:.2}", s.picks),
                s.card_count.to_string(),
                format!("${}", s.ev_usd),
                format!("{:.1}%", s.priced_share * 100.0),
            ]);
        }
        println!("{t}");
    }
    if !ev.top.is_empty() {
        println!("\nTop contributors (per copy):");
        println!("{}", odds_table(&ev.top));
    }
    for c in &ev.caveats {
        println!("· {c}");
    }
}

/// The odds table shared by the expected-value views: what a card is, how often it
/// shows up, and what it adds.
fn odds_table(odds: &[PackCardOdds]) -> comfy_table::Table {
    let mut t = table(&[
        "Name",
        "Set",
        "#",
        "Foil",
        "Sheet",
        "1 in",
        "Price",
        "Contribution",
    ]);
    for o in odds {
        t.add_row(vec![
            output::truncate(&o.card.name, 32),
            o.card.set_code.to_uppercase(),
            o.card.collector_number.clone(),
            if o.foil { "yes" } else { "" }.to_string(),
            output::truncate(&o.sheet, 24),
            format!("{:.1}", o.one_in),
            output::price(&o.price_usd),
            format!("${}", o.contribution_usd),
        ]);
    }
    t
}

/// One simulated opening, pack by pack. The seed rides in the footer because the
/// run is a pure function of it — the same seed deals the same packs.
fn print_opening(ctx: &Ctx, o: &ProductOpening) {
    println!(
        "seed {} · {} cop(y/ies) · {} pack(s) · ${} pulled ({} priced, {} unpriced)",
        o.seed,
        o.copies,
        o.packs.len(),
        o.value_usd,
        o.priced_count,
        o.unpriced_count
    );
    for (i, p) in o.packs.iter().enumerate() {
        println!(
            "\n== Pack {} · {} ({}) · variant {} · ${} ==",
            i + 1,
            p.name.as_deref().unwrap_or(&p.booster_code),
            p.set_code.to_uppercase(),
            p.variant,
            p.value_usd
        );
        let mut t = table(&["Name", "Set", "#", "Rarity", "Foil", "Sheet", "USD"]);
        for c in &p.cards {
            t.add_row(vec![
                output::truncate(&c.card.name, 32),
                c.card.set_code.to_uppercase(),
                c.card.collector_number.clone(),
                output::dash(&c.card.rarity),
                if c.foil { "yes" } else { "" }.to_string(),
                output::truncate(&c.sheet, 24),
                output::price(&c.price_usd),
            ]);
        }
        println!("{t}");
    }
    if !o.caveats.is_empty() {
        println!();
        for c in &o.caveats {
            println!("· {c}");
        }
    }
    ctx.printer
        .note(format!("Replay this opening with --seed {}.", o.seed));
}

pub async fn ingest(ctx: &Ctx, args: IngestArgs) -> Result<()> {
    let path = format!("/api/games/{}/status", args.game);
    let s: IngestStatus = ctx.client.get_json(&path, &[]).await?;
    if ctx.printer.json {
        ctx.printer.json(&s)?;
    } else {
        println!("status         : {}", s.status);
        if let Some(d) = &s.detail {
            println!("detail         : {d}");
        }
        println!("sets_imported  : {}", s.sets_imported);
        println!("cards_imported : {}", s.cards_imported);
        println!(
            "source_updated : {}",
            s.source_updated_at.as_deref().unwrap_or("—")
        );
        println!(
            "finished_at    : {}",
            s.finished_at.as_deref().unwrap_or("—")
        );
    }
    Ok(())
}

pub async fn image(ctx: &Ctx, args: ImageArgs) -> Result<()> {
    let (path, mut q, output): (String, Vec<(&str, String)>, PathBuf) = match args.command {
        ImageCommand::Card {
            game,
            id,
            size,
            face,
            output,
        } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "size", &size);
            push_opt(&mut q, "face", &face);
            (format!("/api/games/{game}/cards/{id}/image"), q, output)
        }
        ImageCommand::Product {
            game,
            id,
            size,
            output,
        } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            push_opt(&mut q, "size", &size);
            (format!("/api/games/{game}/products/{id}/image"), q, output)
        }
    };
    let bytes = ctx.client.get_bytes(&path, &q).await?;
    q.clear();
    std::fs::write(&output, &bytes)?;
    ctx.printer.note(format!(
        "Wrote {} bytes to {}.",
        bytes.len(),
        output.display()
    ));
    Ok(())
}

// -- helpers ----------------------------------------------------------------

fn print_card_page(ctx: &Ctx, page: Page<Card>) {
    if ctx.printer.json {
        let _ = ctx.printer.json(&page);
    } else {
        cards_table(&page.data);
        page_footer(ctx, page.page, page.total, page.has_more, "cards");
    }
}

fn product_detail(p: &Product) {
    println!("{}  [{}]", p.name, p.id);
    println!(
        "  {} · {} · {}",
        p.set_code.to_uppercase(),
        p.set_name.as_deref().unwrap_or("—"),
        p.product_type
    );
    println!(
        "  USD {} · Foil {} · MSRP {}",
        output::price(&p.prices.usd),
        output::price(&p.prices.usd_foil),
        output::price(&p.msrp)
    );
    if let Some(r) = &p.released_at {
        println!("  Released: {r}");
    }
    if let Some(u) = &p.url {
        println!("  {u}");
    }
}

fn read_fingerprint(args: &ScanArgs) -> Result<Vec<u8>> {
    let bytes = if let Some(hex) = &args.hex {
        decode_hex(hex)?
    } else if let Some(file) = &args.file {
        let raw = std::fs::read(file)?;
        if raw.len() == 32 {
            raw
        } else {
            // Treat as hex/whitespace text.
            let text = String::from_utf8_lossy(&raw);
            decode_hex(&text)?
        }
    } else {
        bail!("provide the fingerprint with --file <path> or --hex <64-char hex>");
    };
    if bytes.len() != 32 {
        bail!(
            "a fingerprint must be exactly 32 bytes (got {})",
            bytes.len()
        );
    }
    Ok(bytes)
}

fn decode_hex(s: &str) -> Result<Vec<u8>> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        bail!("hex fingerprint must have an even number of digits");
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| anyhow::anyhow!("invalid hex: {e}"))
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct ScanResponseLocal {
    data: Vec<ScanMatch>,
}

#[cfg(test)]
mod tests {
    use super::decode_hex;

    #[test]
    fn decode_hex_parses_and_ignores_whitespace() {
        let out = decode_hex("00 ff 10\n2a").unwrap();
        assert_eq!(out, vec![0x00, 0xff, 0x10, 0x2a]);
    }

    #[test]
    fn decode_hex_rejects_odd_length() {
        assert!(decode_hex("abc").is_err());
    }

    #[test]
    fn decode_hex_rejects_non_hex() {
        assert!(decode_hex("zz").is_err());
    }
}
