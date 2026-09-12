//! Wire types mirroring the TCGLense API's JSON DTOs.
//!
//! These are hand-maintained duplicates of the Rust DTOs the API serialises (the
//! same shapes the SPA consumes via `web/src/lib/api/generated/`). Only the fields
//! the CLI reads/renders are modelled; unknown fields are ignored on deserialize.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A page of results plus the cursor metadata to paginate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub has_more: bool,
}

/// The `{ "data": T }` envelope used by non-paginated list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBody<T> {
    pub data: T,
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub created_at: String,
    pub username: Option<String>,
    pub discriminator: Option<i64>,
    pub handle: Option<String>,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub completion_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicConfig {
    pub maintenance_mode: bool,
    pub turnstile_site_key: Option<String>,
    pub signups_enabled: bool,
    pub signups_disabled_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyRatesResponse {
    pub base: String,
    pub as_of: String,
    pub rates: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: i64,
    pub name: String,
    pub scope: String,
    pub key_prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyList {
    pub data: Vec<ApiKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedApiKey {
    pub id: i64,
    pub name: String,
    pub scope: String,
    pub key: String,
    pub key_prefix: String,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsernameAvailability {
    pub valid: bool,
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub data_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSet {
    pub code: String,
    pub name: String,
    pub set_type: Option<String>,
    pub released_at: Option<String>,
    pub card_count: i64,
    pub icon_svg_uri: Option<String>,
    pub parent_set_code: Option<String>,
    pub has_drops: bool,
    /// What one of the set's groups is called, singular and lowercase — `"drop"` for
    /// Secret Lair, `"treatment"` for a print-treatment gallery. `None` when
    /// `has_drops` is false.
    #[serde(default)]
    pub drop_noun: Option<String>,
    pub has_subtypes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPrices {
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
    /// The etched-foil price, for the printings that ship one. USD only.
    #[serde(default)]
    pub usd_etched: Option<String>,
    pub eur: Option<String>,
    pub tix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardFace {
    pub name: Option<String>,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub oracle_text: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub loyalty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub set_code: String,
    pub set_name: String,
    pub collector_number: String,
    pub rarity: Option<String>,
    pub lang: String,
    pub released_at: Option<String>,
    pub mana_cost: Option<String>,
    pub cmc: Option<f64>,
    pub type_line: Option<String>,
    pub oracle_text: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub loyalty: Option<String>,
    #[serde(default)]
    pub color_identity: Vec<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    pub layout: Option<String>,
    pub prices: CardPrices,
    /// Per-format legality: `"modern"` → `"legal" | "not_legal" | "banned" |
    /// "restricted"`. Absent when the catalog row carries no legality data.
    #[serde(default)]
    pub legalities: Option<BTreeMap<String, String>>,
    pub has_image: bool,
    pub drop_name: Option<String>,
    pub drop_slug: Option<String>,
    /// What `drop_name` names — `"drop"` or `"treatment"` — paired with it.
    #[serde(default)]
    pub drop_noun: Option<String>,
    #[serde(default)]
    pub secret_lair_bonus: bool,
    #[serde(default)]
    pub secret_lair_spend_incentive: bool,
    #[serde(default)]
    pub faces: Vec<CardFace>,
}

/// The single-card route's payload (`GET /api/games/{game}/cards/{id}`): the card as
/// every listing shows it plus the printing-level detail only this route carries —
/// artist, frame, finishes, promo tags, external ids and popularity ranks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetail {
    #[serde(flatten)]
    pub card: Card,
    /// The illustrator credited on the printing; a multi-artist card carries every
    /// name in one string, as printed.
    pub artist: Option<String>,
    /// Scryfall's stable ids for the artist(s) above, one per artist.
    #[serde(default)]
    pub artist_ids: Vec<String>,
    /// The artwork's id — every printing of the same painting shares it.
    pub illustration_id: Option<String>,
    /// The printed flavour text; a multi-faced card's faces are joined by `\n//\n`.
    pub flavor_text: Option<String>,
    /// The printed watermark (a guild or faction mark), if any.
    pub watermark: Option<String>,
    /// The frame layout (`1993`, `2003`, `2015`, `future`, …).
    pub frame: Option<String>,
    /// Frame effects on the printing (`showcase`, `extendedart`, `legendary`, …).
    #[serde(default)]
    pub frame_effects: Vec<String>,
    /// `black` / `white` / `silver` / `gold` / `borderless`.
    pub border_color: Option<String>,
    /// The holofoil security stamp (`oval`, `triangle`, `acorn`, `arena`, …), if any.
    pub security_stamp: Option<String>,
    /// The finishes this printing exists in — `nonfoil` / `foil` / `etched`.
    #[serde(default)]
    pub finishes: Vec<String>,
    /// Scryfall's promo-type tags (`prerelease`, `buyabox`, `sldbonus`, …).
    #[serde(default)]
    pub promo_types: Vec<String>,
    /// The colours of mana this card can produce.
    #[serde(default)]
    pub produced_mana: Vec<String>,
    /// A Battle's printed defense box.
    pub defense: Option<String>,
    /// On the Reserved List — never to be reprinted.
    #[serde(default)]
    pub reserved: bool,
    #[serde(default)]
    pub full_art: bool,
    #[serde(default)]
    pub textless: bool,
    #[serde(default)]
    pub promo: bool,
    /// A variation of another printing in the same set (alternate art, a different
    /// frame).
    #[serde(default)]
    pub variation: bool,
    /// A Story Spotlight card.
    #[serde(default)]
    pub story_spotlight: bool,
    /// Carries the publisher's content warning.
    #[serde(default)]
    pub content_warning: bool,
    /// Popularity rank on EDHREC (lower is more played); `None` when unranked.
    pub edhrec_rank: Option<i64>,
    /// Popularity rank in Penny Dreadful; `None` when unranked.
    pub penny_rank: Option<i64>,
    /// Gatherer multiverse ids — one per face for a double-faced card.
    #[serde(default)]
    pub multiverse_ids: Vec<i64>,
    /// TCGplayer product id of the regular/foil printing.
    pub tcgplayer_id: Option<i64>,
    /// TCGplayer product id of the etched printing, when it is a distinct product.
    pub tcgplayer_etched_id: Option<i64>,
    /// Cardmarket product id.
    pub cardmarket_id: Option<i64>,
    /// Magic Online catalog id of the regular printing.
    pub mtgo_id: Option<i64>,
    /// Magic Online catalog id of the foil printing, when distinct.
    pub mtgo_foil_id: Option<i64>,
    /// MTG Arena id; only an Arena printing carries one.
    pub arena_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub date: String,
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
    #[serde(default)]
    pub usd_etched: Option<String>,
    pub eur: Option<String>,
    pub tix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruling {
    /// Who issued it: `wotc` or `scryfall`.
    pub source: String,
    /// `YYYY-MM-DD`.
    pub published_at: String,
    pub comment: String,
}

/// One community Tagger art tag — what a card's *artwork* depicts. The slug is the
/// value the `art:` search filter matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtTag {
    pub slug: String,
    pub label: String,
    /// Distinct artworks carrying the tag (hierarchy-expanded).
    pub count: i64,
    pub description: Option<String>,
}

/// One glossary entry: a keyword ability, keyword action, or ability word.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyword {
    pub name: String,
    pub slug: String,
    /// `ability` | `action` | `ability_word`.
    pub kind: String,
    /// Plain-English explanation (the official reminder text where one exists).
    pub text: String,
    /// Whether the keyword normally carries a value in card text (`Ward {2}`).
    pub parameterized: bool,
    /// How safely the name can be spotted in rules text: `anywhere` |
    /// `ability_line` | `never`.
    pub match_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestStatus {
    pub status: String,
    pub detail: Option<String>,
    pub sets_imported: i64,
    pub cards_imported: i64,
    pub source_updated_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropGroup {
    pub slug: Option<String>,
    pub title: String,
    pub card_count: i64,
    pub cheapest_prints_usd: Option<String>,
    /// The drop's street date (`YYYY-MM-DD`), derived from its cards; a future
    /// date means the drop hasn't landed yet.
    #[serde(default)]
    pub released_at: Option<String>,
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtypeGroup {
    pub slug: Option<String>,
    pub title: String,
    pub card_count: i64,
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanMatch {
    pub card: Card,
    pub distance: i64,
}

// ---------------------------------------------------------------------------
// Sealed products
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPrices {
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub set_code: String,
    pub set_name: Option<String>,
    pub product_type: String,
    pub url: Option<String>,
    pub has_image: bool,
    pub prices: ProductPrices,
    pub msrp: Option<String>,
    pub released_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPricePoint {
    pub date: String,
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSetRef {
    pub code: String,
    pub name: Option<String>,
    pub product_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFacets {
    pub types: Vec<String>,
    pub sets: Vec<ProductSetRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedProductRef {
    pub product: Product,
    pub membership: String,
    pub foil: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCardEntry {
    pub card: Card,
    pub membership: String,
    pub foil: bool,
    pub exclusive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCardSection {
    pub key: String,
    pub total: i64,
    pub booster_family: Option<String>,
    /// The unlisted box component this section's cards are packed in — also the
    /// `--component` value that pages them; null for a plain section.
    #[serde(default)]
    pub component: Option<String>,
    /// `true` when every card of a plain section arrived through a listed
    /// sub-product, so the same pool is browsable on that product's own page;
    /// always `false` for a component section.
    #[serde(default)]
    pub inherited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductComponent {
    pub kind: String,
    pub name: String,
    pub quantity: i64,
    pub product: Option<Product>,
    pub card: Option<Card>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductContainer {
    pub product: Product,
    pub quantity: i64,
}

// ---------------------------------------------------------------------------
// Combos, sealed expected value / openings, and the release calendar
// ---------------------------------------------------------------------------

/// One card a combo needs, as the card page lists it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboPiece {
    /// The card's gameplay identity (Scryfall `oracle_id`).
    pub oracle_id: String,
    pub name: String,
    /// Copies the combo needs.
    pub quantity: i64,
    /// Whether it has to be in the command zone.
    pub must_be_commander: bool,
    /// A catalog printing to look up — the newest the catalog holds; null when it
    /// holds none (a spoiler, a card set not imported yet).
    #[serde(default)]
    pub card_id: Option<String>,
}

/// One Commander Spellbook combo the viewed card is a piece of. Everything here is
/// the data source's own datum — `url` is the page it came from, the link the
/// source's terms ask for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardCombo {
    /// Commander Spellbook's variant id (`"245-2034-6705"`).
    pub id: String,
    /// The combo's page on Commander Spellbook.
    pub url: String,
    /// Colour identity letters in WUBRG order; empty for colourless.
    #[serde(default)]
    pub identity: Vec<String>,
    /// The step-by-step description.
    pub description: String,
    /// Upstream's popularity counter — higher is more played.
    pub popularity: i64,
    /// Wildcard requirements the app can't evaluate ("A free sacrifice outlet").
    #[serde(default)]
    pub templates: Vec<String>,
    /// What it produces — the results a player cares about, by name.
    #[serde(default)]
    pub produces: Vec<String>,
    /// Upstream's bracket tag (`"C"` casual, `"R"` ruthless, …); null when unset.
    #[serde(default)]
    pub bracket_tag: Option<String>,
    /// Mana to start it, Scryfall-style (`"{6}"`); null when none is needed.
    #[serde(default)]
    pub mana_needed: Option<String>,
    #[serde(default)]
    pub mana_value_needed: Option<i64>,
    /// Prerequisites beyond the pieces themselves; null when none.
    #[serde(default)]
    pub prerequisites: Option<String>,
    /// Every piece, in the combo's own order — the viewed card included.
    #[serde(default)]
    pub pieces: Vec<ComboPiece>,
}

/// The combos a card is a piece of, most-played first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardCombos {
    /// At most 50, by popularity; `total` is exact.
    #[serde(default)]
    pub combos: Vec<CardCombo>,
    pub total: i64,
    /// Where the data comes from, for the attribution the source asks for.
    pub source: String,
    pub source_url: String,
}

/// One card's odds inside a booster: how often it shows up, and what that is worth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackCardOdds {
    pub card: Card,
    pub foil: bool,
    /// The print sheet it comes off.
    pub sheet: String,
    /// Expected copies per pack.
    pub expected_per_pack: f64,
    /// One in this many packs holds it.
    pub one_in: f64,
    pub price_usd: Option<String>,
    /// 2-dp USD this card contributes — per pack inside a pack or slot, per copy of
    /// the product in the product-wide list.
    pub contribution_usd: String,
}

/// One slot of a booster: a print sheet picked from some number of times.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotEv {
    pub sheet: String,
    pub foil: bool,
    /// Average picks off this sheet per pack.
    pub picks: f64,
    /// Distinct cards on the sheet.
    pub card_count: i64,
    pub ev_usd: String,
    /// Share of the sheet's weight that carries a price (0..=1).
    pub priced_share: f64,
    #[serde(default)]
    pub top: Vec<PackCardOdds>,
}

/// One distinct booster a copy of the product opens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEv {
    pub booster_code: String,
    #[serde(default)]
    pub name: Option<String>,
    pub set_code: String,
    /// How many of this booster one copy of the product holds.
    pub quantity: i64,
    pub cards_per_pack: f64,
    pub ev_usd: String,
    /// Share of the pack's contents that carries a price (0..=1).
    pub priced_share: f64,
    #[serde(default)]
    pub slots: Vec<SlotEv>,
    #[serde(default)]
    pub top: Vec<PackCardOdds>,
}

/// The expected value of **one copy** of a sealed product at today's prices — an
/// average over many openings, never a valuation of the copy in front of you.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductEv {
    /// 2-dp USD, summed before rounding — so it can differ by a cent from adding up
    /// the rendered per-pack figures.
    pub ev_usd: String,
    /// Every distinct booster one copy opens, with how many of each.
    #[serde(default)]
    pub packs: Vec<PackEv>,
    /// The biggest expected contributors across the whole copy (at most 12).
    #[serde(default)]
    pub top: Vec<PackCardOdds>,
    /// What qualifies these numbers, generated server-side.
    #[serde(default)]
    pub caveats: Vec<String>,
}

/// One card a simulated opening dealt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenedCard {
    pub card: Card,
    pub foil: bool,
    /// The print sheet it came off.
    pub sheet: String,
    pub price_usd: Option<String>,
}

/// One pack of a simulated opening, with the configuration the dice rolled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenedPack {
    pub booster_code: String,
    #[serde(default)]
    pub name: Option<String>,
    pub set_code: String,
    /// Which of the booster's configurations was rolled.
    pub variant: i64,
    /// 2-dp USD of the cards below (unpriced cards count as $0).
    pub value_usd: String,
    #[serde(default)]
    pub cards: Vec<OpenedCard>,
}

/// One simulated opening: stateless, seeded, and a pure function of its URL — what
/// this run of the dice dealt, never what the product is worth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductOpening {
    /// The seed this opening was rolled with — echoed so a random one can be
    /// replayed or shared.
    pub seed: i64,
    /// How many copies of the product were opened.
    pub copies: i64,
    /// Every pack, in opening order: copy by copy, then the product's pack order.
    #[serde(default)]
    pub packs: Vec<OpenedPack>,
    /// 2-dp USD of everything pulled (unpriced cards count as $0).
    pub value_usd: String,
    /// Pulled cards that had a market price.
    pub priced_count: i64,
    /// Pulled cards that had none.
    pub unpriced_count: i64,
    /// What qualifies this run, generated server-side.
    #[serde(default)]
    pub caveats: Vec<String>,
}

/// A set releasing inside a window, with the precons and sealed products it ships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetRelease {
    pub set: CardSet,
    /// `YYYY-MM-DD`.
    pub released_at: String,
    /// Whether this is the Secret Lair set (its drops are listed separately).
    pub secret_lair: bool,
    #[serde(default)]
    pub precons: Vec<PreconDeck>,
    #[serde(default)]
    pub products: Vec<Product>,
}

/// A Secret Lair drop whose cards release inside a window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretLairDropRelease {
    pub slug: String,
    pub title: String,
    pub set_code: String,
    /// `YYYY-MM-DD`.
    pub released_at: String,
    #[serde(default)]
    pub products: Vec<Product>,
}

/// The release calendar for a window: the sets releasing in it (each with what it
/// ships) and the Secret Lair drops, both date-ascending.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Releases {
    /// The window's first day (`YYYY-MM-DD`), inclusive — as resolved, so a
    /// defaulted request learns what it was answered for.
    pub from: String,
    /// The window's last day (`YYYY-MM-DD`), inclusive.
    pub to: String,
    /// Sets releasing inside the window, date then code ascending.
    #[serde(default)]
    pub sets: Vec<SetRelease>,
    /// Secret Lair drops with cards releasing inside the window, date then title
    /// ascending. Always empty for a game without Secret Lair.
    #[serde(default)]
    pub secret_lair_drops: Vec<SecretLairDropRelease>,
}

// ---------------------------------------------------------------------------
// Collection / wish list
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollectionQuantities {
    pub quantity: i64,
    pub foil_quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionEntry {
    pub card: Card,
    pub quantity: i64,
    pub foil_quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub unique_cards: i64,
    pub total_cards: i64,
    pub total_value_usd: Option<String>,
    pub bulk_value_usd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSet {
    pub code: String,
    pub name: String,
    pub set_type: Option<String>,
    pub released_at: Option<String>,
    pub card_count: i64,
    pub icon_svg_uri: Option<String>,
    pub parent_set_code: Option<String>,
    pub has_drops: bool,
    pub has_subtypes: bool,
    pub owned_cards: i64,
    pub owned_copies: i64,
    pub owned_value_usd: Option<String>,
    pub owned_bulk_value_usd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionValuePoint {
    pub date: String,
    pub value_usd: Option<String>,
    pub sealed_value_usd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionVisibility {
    pub public: bool,
    pub show_value_chart: bool,
    pub show_movers: bool,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WishlistVisibility {
    pub public: bool,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionDropGroup {
    pub slug: Option<String>,
    pub title: String,
    pub card_count: i64,
    pub cards: Vec<CollectionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSubtypeGroup {
    pub slug: Option<String>,
    pub title: String,
    pub card_count: i64,
    pub cards: Vec<CollectionEntry>,
}

// Breakdown ------------------------------------------------------------------

/// Where a user's per-game holdings' value sits (the collection or the wish list —
/// the wish-list twin reads "wanted" for "held"): copies + estimated value by
/// rarity, colour identity, card type and finish, plus the top holdings. Cards
/// only; sealed products have none of these facets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionBreakdown {
    /// The same fold `…/summary` answers over the same rows, so every bucket below
    /// is a slice of exactly this total.
    pub summary: CollectionSummary,
    /// By rarity, in rarity order (`unknown` last). Only non-empty buckets.
    #[serde(default)]
    pub rarity: Vec<BreakdownBucket>,
    /// By colour identity: WUBRG, then `multicolor`, then `colorless`.
    #[serde(default)]
    pub color: Vec<BreakdownBucket>,
    /// By the type line's first card type, most valuable bucket first.
    #[serde(default)]
    pub card_type: Vec<BreakdownBucket>,
    /// By finish: `regular` then `foil`, each counting only that finish's copies.
    #[serde(default)]
    pub finish: Vec<BreakdownBucket>,
    /// The most valuable holdings by held value, highest first (at most ten).
    #[serde(default)]
    pub top: Vec<TopHolding>,
    /// Distinct held cards that contribute nothing to any value here because no
    /// finish they're held in is priced — so a small total is tellable from an
    /// unpriced one.
    pub unpriced_cards: i64,
}

/// One bucket of a breakdown facet: how many distinct held cards and copies file
/// under it, and what they are worth as held.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownBucket {
    /// The bucket's stable key — a rarity, a colour-identity bucket, a card type,
    /// or `regular`/`foil`.
    pub key: String,
    /// Distinct held cards in the bucket (one per holdings row).
    pub cards: i64,
    /// Held copies in the bucket (regular + foil — or, for a finish bucket, that
    /// finish).
    pub copies: i64,
    /// A 2-dp decimal string; `None` when none of the bucket's copies is priced.
    pub value_usd: Option<String>,
}

/// One of the most valuable holdings, ranked by **held** value (price × copies —
/// never a single copy's price, which is the list's `sort=price`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopHolding {
    pub card: Card,
    pub quantity: i64,
    pub foil_quantity: i64,
    /// The held value, a 2-dp decimal string (an unpriced holding never ranks).
    pub value_usd: String,
}

// Movers ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMover {
    pub card: Card,
    pub quantity: i64,
    pub foil_quantity: i64,
    /// Whether the prices are the **foil** finish's — a holding is represented by
    /// the owned finish whose single-copy price moved the most over the window.
    pub foil: bool,
    /// That finish's current price for **one copy** (2-dp USD string) — never
    /// multiplied by the quantities above.
    pub price_now: String,
    /// The same finish's single-copy price at the window baseline.
    pub price_prev: String,
    /// `price_now - price_prev`, signed 2-dp USD string.
    pub change_usd: String,
    /// Percent change; null when `price_prev` is 0.
    pub change_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMoverList {
    pub gainers: Vec<CollectionMover>,
    pub losers: Vec<CollectionMover>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSealedMover {
    pub product: Product,
    pub quantity: i64,
    pub foil_quantity: i64,
    /// Whether the prices are the **foil** finish's — a holding is represented by
    /// the owned finish whose single-copy price moved the most over the window.
    pub foil: bool,
    /// That finish's current price for **one copy** (2-dp USD string) — never
    /// multiplied by the quantities above.
    pub price_now: String,
    /// The same finish's single-copy price at the window baseline.
    pub price_prev: String,
    /// `price_now - price_prev`, signed 2-dp USD string.
    pub change_usd: String,
    /// Percent change; null when `price_prev` is 0.
    pub change_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSealedMoverList {
    pub gainers: Vec<CollectionSealedMover>,
    pub losers: Vec<CollectionSealedMover>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSealedMovers {
    pub as_of: Option<String>,
    pub day_as_of: Option<String>,
    pub day: CollectionSealedMoverList,
    pub week: CollectionSealedMoverList,
    pub month: CollectionSealedMoverList,
    pub year: CollectionSealedMoverList,
    pub two_year: CollectionSealedMoverList,
    pub three_year: CollectionSealedMoverList,
    pub all_time: CollectionSealedMoverList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMovers {
    pub as_of: Option<String>,
    pub day_as_of: Option<String>,
    pub day: CollectionMoverList,
    pub week: CollectionMoverList,
    pub month: CollectionMoverList,
    pub year: CollectionMoverList,
    pub two_year: CollectionMoverList,
    pub three_year: CollectionMoverList,
    pub all_time: CollectionMoverList,
    pub sealed: CollectionSealedMovers,
}

// Sealed-product holdings ----------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductHoldingEntry {
    pub product: Product,
    pub quantity: i64,
    pub foil_quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductHoldingSummary {
    pub unique_products: i64,
    pub total_products: i64,
    pub total_value_usd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductHoldingSet {
    pub code: String,
    pub name: Option<String>,
    pub unique_products: i64,
    pub total_products: i64,
    pub total_value_usd: Option<String>,
}

// Import ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub fetched: i64,
    pub total: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub provider: String,
    pub mode: String,
    pub total_rows: i64,
    pub distinct_cards: i64,
    pub matched_cards: i64,
    pub unmatched_cards: i64,
    pub unmatched_sample: Vec<String>,
    pub regular_copies: i64,
    pub foil_copies: i64,
    pub removed_cards: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportJob {
    pub job_id: i64,
    pub status: String,
    #[serde(default)]
    pub progress: Option<ImportProgress>,
    #[serde(default)]
    pub summary: Option<ImportSummary>,
    #[serde(default)]
    pub error: Option<String>,
}

// Buy list -------------------------------------------------------------------

/// One wanted card printing on a shopping list, as a store's bulk-entry page takes
/// it: the counts plus the printing's TCGplayer product id where TCGplayer lists it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyListCard {
    /// The printing's external (Scryfall) id.
    pub card_id: String,
    pub name: String,
    pub set_code: String,
    pub collector_number: String,
    /// Regular copies wanted.
    pub quantity: i64,
    /// Foil copies wanted.
    pub foil_quantity: i64,
    /// TCGplayer product id of the printing; `None` when TCGplayer doesn't list it.
    #[serde(default)]
    pub tcgplayer_id: Option<i64>,
}

/// One wanted sealed product on a shopping list (its external id is its TCGplayer
/// product id).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyListProduct {
    pub product_id: String,
    pub name: String,
    pub quantity: i64,
    pub foil_quantity: i64,
}

/// A shopping list as bulk-buy rows — the wish list's (`GET /api/wishlist/{game}/
/// buy-list`) and the decks' (`GET /api/decks/{game}/needed/buy-list`) share the
/// shape. Card rows are capped at 500; `truncated` + the totals say what was cut.
/// Sealed products ride only on an unfiltered wish-list request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyList {
    pub cards: Vec<BuyListCard>,
    #[serde(default)]
    pub products: Vec<BuyListProduct>,
    /// Card rows the filters matched, including any beyond the cap.
    pub total_cards: i64,
    /// Sealed-product rows (`0` when the request was filtered).
    pub total_products: i64,
    /// Whether either list was cut at the cap.
    pub truncated: bool,
}

// ---------------------------------------------------------------------------
// Decks
// ---------------------------------------------------------------------------

/// One card in a deck's command zone, as the deck list names it. The external
/// card id travels so a client can link to the printing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCommander {
    pub card_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub id: i64,
    pub game: String,
    pub name: String,
    pub description: Option<String>,
    pub format: Option<String>,
    pub folder_id: Option<i64>,
    pub is_public: bool,
    pub card_count: i64,
    /// The card(s) in the command zone — one for most Commander decks, two for
    /// partners or an Oathbreaker pair; empty for a deck without one.
    #[serde(default)]
    pub commanders: Vec<DeckCommander>,
    /// WUBRG-ordered letters; `[]` is colourless and **null** means there was
    /// nothing to read a colour off — the three-way convention `PreconDeck` uses.
    #[serde(default)]
    pub color_identity: Option<Vec<String>>,
    /// Estimated USD value of everything outside a maybeboard (2-dp decimal
    /// string); null when nothing in the deck is priced — never `"0.00"`.
    #[serde(default)]
    pub value_usd: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSection {
    pub id: i64,
    pub name: String,
    pub position: i64,
    /// Whether the section sits outside the deck proper — its cards are left out
    /// of `summary`, legality, analytics and the needed list.
    #[serde(default)]
    pub is_maybeboard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCardEntry {
    pub card: Card,
    pub section_id: i64,
    pub quantity: i64,
    pub foil_quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckDetail {
    pub id: i64,
    pub game: String,
    pub name: String,
    pub description: Option<String>,
    pub format: Option<String>,
    pub folder_id: Option<i64>,
    pub is_public: bool,
    pub handle: Option<String>,
    /// Aggregates over the deck **proper** — every card outside a maybeboard section.
    pub summary: CollectionSummary,
    /// The same aggregates over the maybeboard sections alone (all-zero, or absent
    /// on a server that predates maybeboards).
    #[serde(default)]
    pub maybeboard_summary: Option<CollectionSummary>,
    pub sections: Vec<DeckSection>,
    pub cards: Vec<DeckCardEntry>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckFolder {
    pub id: i64,
    pub name: String,
    pub deck_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckVisibility {
    pub public: bool,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckImportResponse {
    pub deck: Deck,
    pub provider: String,
    pub total_rows: i64,
    pub matched_cards: i64,
    pub unmatched_cards: i64,
    pub unmatched_sample: Vec<String>,
}

// Deck formats, legality, analytics, goldfish ---------------------------------

/// One legality-tracked format: its Scryfall key, how it's spelled to a human, the
/// select grouping it renders under, and the extra spellings `deck.format` accepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckFormat {
    /// The key used in a card's `legalities` object.
    pub key: String,
    /// Display label — also what's stored in `deck.format` when picked.
    pub label: String,
    /// `constructed` | `commander` | `arena` | `other`.
    pub group: String,
    pub aliases: Vec<String>,
    /// Whether it's one of the most-played formats.
    pub popular: bool,
}

/// One offending card name in a deck (all printings of a name fold into one issue).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckLegalityIssue {
    /// External card id of one printing (for links).
    pub card_id: String,
    pub name: String,
    /// `banned` | `not_legal` | `commander_only` | `off_colour` | `over_limit` |
    /// `restricted`.
    pub status: String,
    /// Total copies across every section and printing.
    pub quantity: i64,
}

/// One deck-wide construction breach, with a ready-to-render sentence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckRuleViolation {
    /// `deck-size` | `sideboard-size` | `command-zone` | `commander-eligibility` |
    /// `colour-identity`.
    pub rule: String,
    /// `error` (illegal as it stands) or `warning` (simply not finished yet).
    pub severity: String,
    pub message: String,
}

/// A deck's verdict against its own format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckLegality {
    /// The legality key the deck's format label normalised to.
    pub format_key: String,
    pub format_label: String,
    /// Sorted most severe first.
    pub issues: Vec<DeckLegalityIssue>,
    pub violations: Vec<DeckRuleViolation>,
    /// Per-printing status for every entry belonging to an offending name.
    #[serde(default)]
    pub card_statuses: BTreeMap<String, String>,
    /// Cards whose catalog row carries no legality data at all.
    pub unknown_count: i64,
    /// No card issues and no error-severity violation.
    pub legal: bool,
}

/// One bar of a distribution (a mana-value bucket, a colour, a card type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckStatItem {
    /// Stable bucket identifier (`"3"`, `"W"`, `"Creature"`).
    pub key: String,
    pub label: String,
    pub count: i64,
    /// Advisory hex swatch for the buckets that have a canonical colour.
    #[serde(default)]
    pub color: Option<String>,
}

/// How many copies of one card **name** a pool holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCardOdds {
    pub name: String,
    pub copies: i64,
}

/// The copy-weighted composition of a set of deck entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckComposition {
    /// Total copies (regular + foil) across every entry.
    pub total_copies: i64,
    /// Distinct printings — a name held in two arts counts twice.
    pub unique_cards: i64,
    pub land_copies: i64,
    /// Copy-weighted mean mana value over nonlands, or null when there are none.
    #[serde(default)]
    pub average_mana_value: Option<f64>,
    /// Nonland copies bucketed by mana value, `0`..`6` then `7+`.
    pub mana_curve: Vec<DeckStatItem>,
    pub colors: Vec<DeckStatItem>,
    pub card_types: Vec<DeckStatItem>,
    /// Copies folded by card name, most-copied first.
    pub card_odds: Vec<DeckCardOdds>,
}

/// The hypergeometric draw odds for one card out of the library pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckDrawOdds {
    pub name: String,
    pub copies: i64,
    pub library_size: i64,
    /// How many cards the `at_least_one` figure assumes were seen.
    pub cards_seen: i64,
    pub at_least_one: f64,
    /// `curve[i]` is P(at least one copy) after seeing `i + 1` cards.
    pub curve: Vec<f64>,
}

/// Everything the deck stats endpoint answers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckAnalytics {
    /// Composition of the deck proper — maybeboard sections excluded.
    pub deck: DeckComposition,
    /// Composition of the library pool the odds are drawn from.
    pub library: DeckComposition,
    pub library_section_ids: Vec<i64>,
    /// The sections the library defaults to: everything that isn't a maybeboard,
    /// a command zone, or a sideboard.
    pub default_library_section_ids: Vec<i64>,
    /// Null only when the library pool is empty.
    #[serde(default)]
    pub odds: Option<DeckDrawOdds>,
}

/// A goldfished hand: what you're holding, what you bottomed, and what's left.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldfishHand {
    /// The seed this hand was shuffled with — echoed so it can be replayed.
    pub seed: i64,
    pub mulligans: i64,
    /// The opening hand size actually dealt (clamped to the library).
    pub opening: i64,
    pub draws: i64,
    /// Cards that still have to go to the bottom before the game starts.
    pub to_bottom: i64,
    /// The hand, in the order the cards were drawn.
    pub hand: Vec<Card>,
    pub bottomed: Vec<Card>,
    pub library_size: i64,
    pub library_total: i64,
    pub section_ids: Vec<i64>,
}

// Commander bracket ---------------------------------------------------------

/// One rung of Wizards' 1–5 Commander bracket ladder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckBracketLevel {
    /// 1–5.
    pub bracket: i64,
    /// The rung's name (`"Upgraded"`).
    pub label: String,
    pub description: String,
}

/// One card counted towards a bracket category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckBracketCard {
    /// External card id of one printing (for links).
    pub card_id: String,
    pub name: String,
    /// Copies of that name across the deck proper (regular + foil, every section).
    pub quantity: i64,
}

/// What the deck holds in one bracket category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckBracketCategory {
    /// `game_changer` | `mass_land_denial` | `extra_turn` | `tutor`.
    pub signal: String,
    pub label: String,
    pub description: String,
    /// Distinct card **names** — a card held in two arts counts once.
    pub count: i64,
    /// Whether this category is what put the estimate where it is.
    pub decisive: bool,
    /// The matched cards in the deck's own order, capped (`count` stays exact).
    pub cards: Vec<DeckBracketCard>,
}

/// Where a deck sits on the Commander bracket ladder, estimated from its cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckBracketEstimate {
    /// Always `commander` — the estimate is null for every other format.
    pub format_key: String,
    pub format_label: String,
    /// The lowest bracket the deck's cards don't rule out: 2, 3 or 4. Never 1 or 5,
    /// both of which are claims about intent a decklist can't settle.
    pub bracket: i64,
    pub label: String,
    pub description: String,
    /// All five rungs, so a client can draw the ladder without its own copy of it.
    pub ladder: Vec<DeckBracketLevel>,
    /// Why the estimate landed where it did, most decisive first.
    pub reasons: Vec<String>,
    /// What the estimate could not see. Never empty — the floor is only meaningful
    /// alongside the reasons it might be too low.
    pub caveats: Vec<String>,
    /// Every category, in a stable order, whether or not the deck holds any.
    pub categories: Vec<DeckBracketCategory>,
    /// Whether the deck also clears the extra bar bracket 1 sets.
    pub exhibition_possible: bool,
}

/// One deck that wants a [`NeededCard`], in the game's cross-deck needed list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeededCardDeck {
    pub id: i64,
    pub name: String,
}

/// A card the caller's decks collectively want more copies of than they own.
/// Scoped to one deck (`?deck_id=`), `required` is what *that* deck wants and
/// `needed` is its share of the shortfall across **all** the caller's decks, so two
/// decks sharing one owned copy are each told they still need one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeededCard {
    pub card: Card,
    /// Total copies the decks want.
    pub required: i64,
    /// Copies already in the collection.
    pub owned: i64,
    /// Shortfall (`required - owned`, floored at zero).
    pub needed: i64,
    pub decks: Vec<NeededCardDeck>,
    /// What the `needed` copies cost **at the printings and finishes the decks
    /// hold**; null when no held finish of the card is priced — never `"0.00"`.
    #[serde(default)]
    pub held_usd: Option<String>,
    /// What they cost at the card's **cheapest printing** anywhere in the catalog;
    /// null when no printing of it is priced.
    #[serde(default)]
    pub cheapest_usd: Option<String>,
}

/// The money and the size of a shopping list, folded once so "12 cards, ~$41" needs
/// no re-summing of the lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeededTotals {
    /// Distinct entries in the list.
    pub cards: i64,
    /// Copies to acquire: the sum of every entry's `needed`.
    pub copies: i64,
    /// The sum of every entry's `held_usd`; null when no entry is priced that way.
    /// While `held_unpriced_cards` is non-zero this is a floor.
    #[serde(default)]
    pub held_usd: Option<String>,
    /// Entries whose `held_usd` is null.
    pub held_unpriced_cards: i64,
    /// The sum of every entry's `cheapest_usd`; null when no entry has a priced
    /// printing at all. While `cheapest_unpriced_cards` is non-zero this is a floor.
    #[serde(default)]
    pub cheapest_usd: Option<String>,
    /// Entries whose `cheapest_usd` is null.
    pub cheapest_unpriced_cards: i64,
}

/// The deck shopping list: the shortfalls, the deck it was scoped to (if any), and
/// the totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeededList {
    /// The shortfalls, by card name.
    pub data: Vec<NeededCard>,
    /// The deck the list was scoped to (`--deck`), or null for the game-wide list.
    #[serde(default)]
    pub deck: Option<NeededCardDeck>,
    pub totals: NeededTotals,
}

// Card → decks, deck → tokens ------------------------------------------------

/// The exact printing behind some of the copies a deck holds of a card — deck
/// proper and maybeboard alike.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDeckPrintingRef {
    pub id: String,
    pub set_code: String,
    pub collector_number: String,
    /// Copies of this exact printing in the deck (regular + foil, **maybeboard
    /// included**) — so the printings span both [`CardDeckRef::quantity`] and
    /// [`CardDeckRef::maybeboard_quantity`], not `quantity` alone.
    pub quantity: i64,
}

/// One of the caller's decks that contains a card — any printing of it — with the
/// copies split between the deck proper and the maybeboard, so "runs it" and "only
/// considering it" read apart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDeckRef {
    pub deck: Deck,
    /// Copies (regular + foil, any printing) in the deck proper — maybeboards excluded.
    pub quantity: i64,
    /// Copies in the deck's maybeboard sections, counted apart from `quantity`.
    pub maybeboard_quantity: i64,
    /// The exact printings behind those copies, most copies first. May sum short of
    /// the totals above (a printing gone from the catalog is simply absent).
    #[serde(default)]
    pub printings: Vec<CardDeckPrintingRef>,
}

/// One of a deck's cards that makes a token, with how many copies the deck runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckTokenSource {
    pub card_id: String,
    pub name: String,
    pub quantity: i64,
}

/// One token (or emblem) a deck makes, and what makes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckToken {
    /// Stable identity across sets: the token printing's oracle id where the catalog
    /// has it, else a name + type-line key.
    pub key: String,
    pub name: String,
    /// The token's printed type line (`Token Creature — Soldier`, `Emblem — Elspeth`).
    #[serde(default)]
    pub type_line: Option<String>,
    /// A printing of the token (the newest one the deck's cards point at); null when
    /// no referenced printing is in the catalog.
    #[serde(default)]
    pub card: Option<Card>,
    /// The deck's cards that make it, by name, capped upstream — `source_count`
    /// stays exact.
    pub sources: Vec<DeckTokenSource>,
    /// How many distinct cards in the deck make it.
    pub source_count: i64,
}

/// The tokens and emblems a deck's cards make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckTokens {
    /// Most-made first, then by name.
    pub tokens: Vec<DeckToken>,
    /// Cards in the deck proper whose catalog row hasn't been checked for tokens
    /// yet; while non-zero the list is a floor, not the whole answer.
    pub unchecked_count: i64,
}

// Combos, mana base, pricing, roles -------------------------------------------

/// One piece of a combo, judged against a deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckComboPiece {
    /// The card's gameplay identity (Scryfall `oracle_id`) — pieces match on this,
    /// so any printing counts.
    pub oracle_id: String,
    pub name: String,
    /// Copies the combo needs.
    pub quantity: i64,
    /// Whether it has to be in the command zone.
    pub must_be_commander: bool,
    /// Whether the deck proper holds the card at all (any zone, any number).
    pub in_deck: bool,
    /// A printing to link to: the deck's own where it holds the card, else the
    /// catalog's newest; null when the catalog holds none.
    #[serde(default)]
    pub card_id: Option<String>,
}

/// One thing a combo still needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckComboMissing {
    /// The card's or template's name.
    pub name: String,
    /// Why it's missing: `card` | `template` | `commander`.
    pub kind: String,
    #[serde(default)]
    pub card_id: Option<String>,
}

/// One Commander Spellbook combo against a deck — the source's own datum (the
/// wire's `ComboSummary`) flattened together with what this deck has and lacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCombo {
    /// Commander Spellbook's variant id (`"245-2034-6705"`).
    pub id: String,
    /// The combo's page on Commander Spellbook — the link the source's terms ask for.
    pub url: String,
    /// Colour identity letters in WUBRG order; empty for colourless.
    #[serde(default)]
    pub identity: Vec<String>,
    /// The step-by-step description.
    pub description: String,
    /// Upstream's popularity counter — higher is more played.
    pub popularity: i64,
    /// Wildcard requirements the app can't evaluate ("A free sacrifice outlet").
    #[serde(default)]
    pub templates: Vec<String>,
    /// What it produces — the results a player cares about, by name.
    #[serde(default)]
    pub produces: Vec<String>,
    /// Upstream's bracket tag (`"C"` casual, `"R"` ruthless, …); null when unset.
    #[serde(default)]
    pub bracket_tag: Option<String>,
    /// Mana to start it, Scryfall-style (`"{6}"`); null when none is needed.
    #[serde(default)]
    pub mana_needed: Option<String>,
    #[serde(default)]
    pub mana_value_needed: Option<i64>,
    /// Prerequisites beyond the pieces themselves; null when none.
    #[serde(default)]
    pub prerequisites: Option<String>,
    /// Every piece, in the combo's own order.
    #[serde(default)]
    pub pieces: Vec<DeckComboPiece>,
    /// What the deck still needs — empty for a combo it can assemble.
    #[serde(default)]
    pub missing: Vec<DeckComboMissing>,
}

/// Everything a deck's combo read says.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCombos {
    /// Combos the deck proper can assemble: fewest pieces first, then most-played.
    /// Capped upstream — `combo_count` is exact.
    pub combos: Vec<DeckCombo>,
    pub combo_count: i64,
    /// Combos exactly one card (or template, or commander swap) away, most-played
    /// first. Capped upstream — `almost_count` is exact.
    pub almost: Vec<DeckCombo>,
    pub almost_count: i64,
    /// Whether any combo data is present at all. `false` means the dataset hasn't
    /// been synced — an empty `combos` is then "unknown", never "none".
    pub available: bool,
    /// Where the data comes from, for the attribution the source asks for.
    pub source: String,
    pub source_url: String,
}

/// One spell counted against a colour — a card whose cost holds hard pips of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckManaDemandCard {
    pub card_id: String,
    pub name: String,
    /// Copies of that name across the deck it's cast from.
    pub quantity: i64,
    /// The cost as read — the front half of a split card.
    pub mana_cost: String,
    /// Pips of this colour in that cost.
    pub pips: i64,
    /// The mana value of that cost — the turn the spell is meant to be cast on.
    pub turn: i64,
    /// The table row this spell was judged as (`"1CC"`), after clamping.
    pub cost_key: String,
    /// Whether the cost also holds hard pips of another colour, which adds one to
    /// the requirement (Karsten's gold-card rule).
    pub gold: bool,
    /// Sources of this colour a deck this size needs to cast it on curve.
    pub sources_needed: i64,
    /// Whether the cost holds an `{X}` — listed, but never allowed to set the
    /// colour's requirement.
    pub x_cost: bool,
    /// Whether the spell had to be clamped onto the table to be judged at all —
    /// then `cost_key` is not the cost's own row.
    pub clamped: bool,
}

/// One card in the library that produces a colour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckManaSource {
    pub card_id: String,
    pub name: String,
    /// Copies of that name in the library — each one is one source.
    pub quantity: i64,
    /// Whether the card is a land; a nonland producer counts the same here, and the
    /// split is reported so a reader can weigh it.
    pub land: bool,
}

/// One colour's ledger: what the deck asks for, what the library gives, the verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckManaColor {
    /// `W` / `U` / `B` / `R` / `G` / `C`.
    pub color: String,
    /// `"White"`, …, `"Colorless"`.
    pub label: String,
    /// Hard pips of this colour across the deck it's cast from, copy-weighted.
    pub pips: i64,
    /// Pips this colour *could* pay (hybrid, twobrid, Phyrexian) — reported, never
    /// counted against the colour.
    pub hybrid_pips: i64,
    /// Distinct card names with a hard pip of this colour.
    pub demand_count: i64,
    /// Those cards, most demanding first, capped upstream.
    #[serde(default)]
    pub demand: Vec<DeckManaDemandCard>,
    /// Sources in the library, copy-weighted: lands and nonland producers alike.
    pub sources: i64,
    pub land_sources: i64,
    pub nonland_sources: i64,
    /// Distinct card names producing this colour.
    pub source_count: i64,
    /// Those cards, lands first, capped upstream.
    #[serde(default)]
    pub source_cards: Vec<DeckManaSource>,
    /// Sources the most demanding spell needs, or null when nothing demands the
    /// colour.
    #[serde(default)]
    pub sources_needed: Option<i64>,
    /// `max(0, sources_needed - sources)`.
    pub shortfall: i64,
    /// `enough` | `short` | `no_demand` | `undecided`.
    pub status: String,
    /// One ready-to-render sentence ("Short 2 black sources").
    pub verdict: String,
}

/// A deck's colour requirements against its sources, judged with Frank Karsten's
/// 2022 tables. Demand is the library plus the command zone; supply is the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckManaBase {
    /// Copies in the deck the spells are cast from: the library plus the command zone.
    pub deck_size: i64,
    /// The table column the deck was judged against: 40, 60, 80 or 99.
    pub table_size: i64,
    /// Copies in the library — the sources' pool.
    pub library_size: i64,
    /// Land copies in the library.
    pub land_count: i64,
    /// One ledger per colour the deck demands or produces, WUBRG-then-colourless.
    pub colors: Vec<DeckManaColor>,
    /// Distinct library cards whose catalog row hasn't been checked for what it
    /// produces; while non-zero every source count is a floor.
    pub unchecked_count: i64,
    /// What the numbers assume. Never empty — a threshold is only honest beside
    /// its model.
    pub caveats: Vec<String>,
    /// Where the thresholds come from.
    pub source: String,
}

/// The cheapest printing of one pricing line's card, held the way the line is held.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckCheapestPrinting {
    /// The printing — its `id` is what the printing swap takes.
    pub card: Card,
    /// What the line would cost as this printing (2-dp USD). Always priced.
    pub price_usd: String,
}

/// One deck row — a printing in a section — priced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckPricingLine {
    /// The printing the deck holds.
    pub card: Card,
    /// The section it sits in — the same printing in two sections is two rows.
    pub section_id: i64,
    pub quantity: i64,
    pub foil_quantity: i64,
    /// What the row is worth as held; null when no finish it holds copies of is
    /// priced — never `"0.00"`.
    #[serde(default)]
    pub price_usd: Option<String>,
    /// The cheapest priced printing at this row's finish split, or null when no
    /// printing of it is priced in every finish the row holds.
    #[serde(default)]
    pub cheapest: Option<DeckCheapestPrinting>,
    /// `price_usd` minus `cheapest.price_usd`; `"0.00"` when the row already holds
    /// the cheapest printing, null when either side is unknown.
    #[serde(default)]
    pub saving_usd: Option<String>,
}

/// The deck's money, line by line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckPricing {
    /// Every row of the deck proper, most expensive first (unpriced rows last).
    /// Maybeboard rows are not listed.
    pub lines: Vec<DeckPricingLine>,
    /// The deck's value as held — identical to the detail's `summary.total_value_usd`;
    /// null when nothing in the deck is priced.
    #[serde(default)]
    pub total_usd: Option<String>,
    /// The value if every line with a known saving were swapped to its cheapest
    /// printing; null whenever `total_usd` is.
    #[serde(default)]
    pub cheapest_total_usd: Option<String>,
    /// The sum of every line's known saving; null whenever `total_usd` is.
    #[serde(default)]
    pub saving_usd: Option<String>,
    /// Rows whose held printing has no price in either finish — while non-zero,
    /// `total_usd` is a floor.
    pub unpriced_count: i64,
    /// Rows a swap would save money on — what "swap all" would touch.
    pub swappable_count: i64,
}

/// One card counted towards a deckbuilding role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckRoleCard {
    pub card_id: String,
    pub name: String,
    /// Copies of that name across the deck proper.
    pub quantity: i64,
}

/// What a deck holds in one role. `role` stays a plain string, so a role the API
/// grows still renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckRoleGroup {
    /// `ramp` | `card_draw` | `removal` | `board_wipe` | `counterspell` | `tutor` |
    /// `recursion` | `protection`.
    pub role: String,
    pub label: String,
    /// What the role counts, and the near-miss it deliberately doesn't.
    pub description: String,
    /// Distinct card **names** in this role — a card held in two arts counts once.
    pub count: i64,
    /// Copies (regular + foil) across those names.
    pub copies: i64,
    /// The matched cards in the deck's own order, capped upstream (`count` stays
    /// exact).
    #[serde(default)]
    pub cards: Vec<DeckRoleCard>,
}

/// How many pieces of each deckbuilding role a deck holds, read off rules text over
/// the deck proper. The roles aren't a partition — a card may fill several, and
/// most creatures and every land fill none.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckRoles {
    /// Every role, in a stable order, whether or not the deck holds any.
    pub roles: Vec<DeckRoleGroup>,
    /// The roles each printing fills, keyed by external card id. **Maybeboards are
    /// in this map** although they're out of every count above.
    #[serde(default)]
    pub card_roles: BTreeMap<String, Vec<String>>,
    /// Distinct card names in the deck proper.
    pub card_count: i64,
    /// Distinct card names that matched no role.
    pub unclassified_count: i64,
}

// Suggestions, diff, add-to-collection ----------------------------------------

/// One owned card the deck could play.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSuggestionCard {
    /// One printing the caller owns — the lowest catalog id among those held.
    pub card: Card,
    /// EDHREC's global popularity rank (1 = most played) — the sort key.
    pub edhrec_rank: i64,
    /// Copies owned across every printing (regular + foil).
    pub owned: i64,
    /// The roles the card fills; empty for a card the grammar can't place.
    #[serde(default)]
    pub roles: Vec<String>,
}

/// What the collection could add to the deck in one role, beside what it holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSuggestionRole {
    pub role: String,
    pub label: String,
    /// What the role counts — the roles read's own wording.
    pub description: String,
    /// Distinct cards in the deck proper already filling this role.
    pub in_deck: i64,
    /// Scanned candidates filling this role.
    pub count: i64,
    /// Those candidates by external card id into [`DeckSuggestions::cards`], most
    /// popular first, capped upstream (`count` stays exact).
    #[serde(default)]
    pub card_ids: Vec<String>,
}

/// Cards in the caller's collection this deck could play: legal in its format,
/// inside its colour identity, not already in it — ranked by EDHREC's **global**
/// popularity (not per-commander synergy, as the caveats say) and grouped by role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSuggestions {
    /// The command-zone cards whose identity the filter used; empty when the
    /// colours are a union over the deck proper.
    #[serde(default)]
    pub commanders: Vec<DeckCommander>,
    /// The identity candidates had to fit inside; `[]` is colourless and null means
    /// there was nothing to read a colour off — then no colour filter applied.
    #[serde(default)]
    pub color_identity: Option<Vec<String>>,
    /// The legality key the deck's format normalised to, or null when the format
    /// isn't a tracked one — then no legality filter applied.
    #[serde(default)]
    pub format_key: Option<String>,
    #[serde(default)]
    pub format_label: Option<String>,
    /// Owned cards (by gameplay identity) that passed every filter — exact.
    pub candidate_count: i64,
    /// How many of those were loaded and classified; equal to `candidate_count`
    /// unless it exceeded the scan cap.
    pub scanned_count: i64,
    /// Every card `top` or a role names, once each — the pool the id lists below
    /// index into.
    pub cards: Vec<DeckSuggestionCard>,
    /// The most popular candidates overall, by external card id into `cards`.
    #[serde(default)]
    pub top: Vec<String>,
    /// Every role, in the roles read's order, whether or not any candidate fills it.
    pub roles: Vec<DeckSuggestionRole>,
    /// Scanned candidates filling no role.
    pub unclassified_count: i64,
    /// What the ranking is and isn't. Never empty.
    pub caveats: Vec<String>,
}

/// One side of a deck comparison, named so the two columns can be captioned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckDiffSide {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub format: Option<String>,
    /// Copies in the deck proper.
    pub total_cards: i64,
}

/// Counts of cards (names, not rows) per kind of change, over the deck-wide fold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckDiffSummary {
    pub added: i64,
    pub removed: i64,
    pub changed: i64,
    pub finish_changed: i64,
    pub unchanged: i64,
}

/// One card that differs, folded across every printing of it and both finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckDiffEntry {
    /// A printing of the card, for the row and the link.
    pub card: Card,
    /// The fold key — the card's name, shared by every printing of it.
    pub name: String,
    /// `added` | `removed` | `changed` | `finish` (only the foil split differs).
    pub change: String,
    /// Copies in the base deck: regular + foil, every printing. `0` when added.
    pub base_quantity: i64,
    /// Copies in the other deck, on the same grain. `0` when removed.
    pub other_quantity: i64,
    /// `other_quantity - base_quantity`: positive when the other deck plays more.
    pub delta: i64,
    /// Foil copies in the base deck, every printing.
    pub base_foil_quantity: i64,
    /// Foil copies in the other deck, every printing.
    pub other_foil_quantity: i64,
}

/// The differences within one section, matched between the two decks by **name**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckDiffSection {
    pub name: String,
    /// Whether the section sits outside the deck proper.
    pub is_maybeboard: bool,
    /// The base deck's section id, or null when only the other deck has it.
    #[serde(default)]
    pub base_section_id: Option<i64>,
    /// The other deck's section id, or null when only the base deck has it.
    #[serde(default)]
    pub other_section_id: Option<i64>,
    /// The cards that differ here — never empty, a quiet section isn't listed.
    pub entries: Vec<DeckDiffEntry>,
    /// Cards held identically (same copies, same finishes) in both decks' copy of
    /// this section.
    pub unchanged: i64,
}

/// Everything two of the caller's decks disagree on. Cards are folded by **name**
/// across every printing and both finishes, so a printing swap is not a change and
/// a split playset is one card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckDiff {
    pub base: DeckDiffSide,
    pub other: DeckDiffSide,
    /// Card counts per kind of change, over `cards`.
    pub summary: DeckDiffSummary,
    /// The deck-wide fold over the deck proper, section-agnostic: a card moved
    /// between sections nets to nothing here.
    pub cards: Vec<DeckDiffEntry>,
    /// The per-section view; a section with nothing to report isn't listed.
    pub sections: Vec<DeckDiffSection>,
}

/// What an add-to-collection did. Deliberately not an import summary: the rows come
/// from the catalog, so the only way one fails to land is its card having left it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionAdd {
    /// Distinct printings whose owned counts went up.
    pub cards: i64,
    /// Regular copies added on top of what was already owned.
    pub regular_copies: i64,
    /// Foil copies added on top of what was already owned.
    pub foil_copies: i64,
    /// Distinct cards that couldn't be added — no longer in the catalog.
    pub skipped_cards: i64,
}

// ---------------------------------------------------------------------------
// Preconstructed decks
// ---------------------------------------------------------------------------

/// Just enough of a precon's face card to label a row: the **external** card id,
/// its name, and whether an image exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconFaceCard {
    pub card_id: String,
    pub name: String,
    pub has_image: bool,
}

/// A published decklist's header: what it is, when it came out, how big it is, and
/// the card that fronts it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconDeck {
    /// URL identity, stable across syncs (`turtle-power-tmc`).
    pub slug: String,
    pub game: String,
    pub name: String,
    /// The set the deck ships with, lowercased (`tmc`).
    pub set_code: String,
    /// That set's display name, when the catalog holds it.
    #[serde(default)]
    pub set_name: Option<String>,
    /// Upstream's category: "Commander Deck", "Secret Lair Drop", "Jumpstart", …
    pub deck_type: String,
    #[serde(default)]
    pub released_at: Option<String>,
    /// Copies in the deck proper (mainboard + command zone).
    pub card_count: i64,
    /// Copies in the sideboard, counted apart from `card_count`.
    pub sideboard_count: i64,
    /// `["W","U"]`, `[]` for colourless, and **null** when there's nothing to read
    /// a colour off — the same three-way convention a deck's colour identity uses.
    #[serde(default)]
    pub color_identity: Option<Vec<String>>,
    /// The deck's commander, else the first card upstream lists; null when that card
    /// is no longer in the catalog.
    #[serde(default)]
    pub face_card: Option<PreconFaceCard>,
    /// Estimated USD value of the deck proper (sideboard excluded, the same grain
    /// as `card_count`), a 2-dp decimal string; null when none of its cards are
    /// priced — never `"0.00"`.
    #[serde(default)]
    pub price_usd: Option<String>,
}

/// One bucket of precons — a set, or a deck type. Grouped pages paginate by
/// *group*, so a group's decks are never split across a page boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconGroup {
    /// The set code (`tmc`) when grouping by set, a slugified deck type
    /// (`commander-deck`) when grouping by type.
    pub slug: String,
    pub title: String,
    /// The set code this group links to; null for a type group.
    #[serde(default)]
    pub set_code: Option<String>,
    /// The set's release date when grouping by set; always null by type.
    #[serde(default)]
    pub released_at: Option<String>,
    pub deck_count: i64,
    pub decks: Vec<PreconDeck>,
}

/// A deck type that actually occurs, with how many decks carry it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconTypeRef {
    #[serde(rename = "type")]
    pub type_: String,
    pub count: i64,
}

/// A set that has precons (code + resolved name + count), for the set filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconSetRef {
    pub code: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub released_at: Option<String>,
    pub count: i64,
}

/// The filter vocabulary for a game's precons: every type and set that has one,
/// published rather than hard-coded (upstream adds categories over time).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconFacets {
    /// Deck types, most decks first.
    pub types: Vec<PreconTypeRef>,
    /// Sets that have precons, newest release first.
    pub sets: Vec<PreconSetRef>,
    /// Total precon decks for the game, before any filter.
    pub total: i64,
}

/// One card of a precon. A row is a **single** finish, unlike a deck card's
/// regular+foil pair, because that is how a published decklist states it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconCardEntry {
    pub card: Card,
    /// `commander` | `main` | `side`.
    pub board: String,
    pub quantity: i64,
    pub foil: bool,
}

/// The full single-precon view: the header, the value summary, every card in board
/// order, and the sealed product that ships it (when the catalog holds one).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreconDeckDetail {
    #[serde(flatten)]
    pub deck: PreconDeck,
    /// The format the deck's *type* states (`Commander Deck` → `commander`), or null
    /// when the type states none — exactly what a copy of it would be judged against.
    #[serde(default)]
    pub format: Option<String>,
    /// Value / copy aggregates over the deck proper (command zone + mainboard).
    pub summary: CollectionSummary,
    /// The same aggregates over the sideboard alone; all-zero when there isn't one.
    pub sideboard_summary: CollectionSummary,
    pub cards: Vec<PreconCardEntry>,
    /// The sealed product this deck ships in, when the catalog holds one.
    #[serde(default)]
    pub product: Option<Product>,
}

/// One preconstructed deck containing a card (any printing, on any board): the
/// deck's browse header plus how the card sits in it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPreconRef {
    pub precon: PreconDeck,
    /// Total copies of the card in the deck — any board, any printing, any finish.
    pub quantity: i64,
    /// `true` when every copy is foil (a foil-only inclusion).
    pub foil: bool,
    /// `true` when a copy sits in the command zone — the card *leads* this deck.
    pub commander: bool,
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// One kind's slice of a catalog search: the top matches, plus whether more
/// matched than the limit let through.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchGroup<T> {
    pub data: Vec<T>,
    pub has_more: bool,
}

impl<T> Default for SearchGroup<T> {
    fn default() -> Self {
        SearchGroup {
            data: Vec::new(),
            has_more: false,
        }
    }
}

/// Everything the catalog knows that matches one query, grouped by kind. Every
/// group carries the same wire shape its own listing does, so the rows render with
/// the tables those listings already use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Distinct card names, each as one representative printing.
    pub cards: SearchGroup<Card>,
    /// Sets by name or exact set code, each as the set list shows it.
    #[serde(default)]
    pub sets: SearchGroup<CardSet>,
    /// Sealed products (boxes, bundles, decks) by name.
    pub products: SearchGroup<Product>,
    /// Preconstructed decks by name.
    pub precons: SearchGroup<PreconDeck>,
    /// Rules keywords by name — never by reminder text.
    pub keywords: SearchGroup<Keyword>,
}

// ---------------------------------------------------------------------------
// Public sharing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicGameSummary {
    pub game: String,
    pub summary: CollectionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicProfile {
    pub username: String,
    pub discriminator: i64,
    pub handle: String,
    pub member_since: String,
    /// Games whose **collection** the owner shares.
    pub games: Vec<PublicGameSummary>,
    /// Games whose **wish list** the owner shares — shared independently of the
    /// collection, so a game can appear in one list, both, or neither.
    #[serde(default)]
    pub wishlists: Vec<PublicGameSummary>,
}

// ---------------------------------------------------------------------------
// Tools — life tracker
// ---------------------------------------------------------------------------

/// One seat in a tracked game: who's sitting there, what they brought, where they
/// are on screen, what they're on, and how the game ended for them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifePlayer {
    pub id: i64,
    /// Seat order within the session, 0-based and gap-free.
    pub position: i64,
    pub name: String,
    pub starting_life: i64,
    pub life: i64,
    /// Screen rotation in degrees (`0`, `90`, `180`, `270`).
    pub rotation: i64,
    /// `none` while the game is active, then `win` / `loss` / `draw`.
    pub result: String,
    pub deck_id: Option<i64>,
    pub deck_name: Option<String>,
    pub commander_card_id: Option<String>,
    pub commander_name: Option<String>,
}

/// A tracked game's header plus its seats — what the session list returns, and
/// what every write echoes back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeSession {
    pub id: i64,
    pub game: String,
    pub name: Option<String>,
    pub format: Option<String>,
    /// The total a new seat in this session starts on.
    pub starting_life: i64,
    /// Seat-placement layout slug: `rows` / `facing` / `facing-solo` / `sides` /
    /// `sides-solo` / `grid` / `pinwheel`.
    pub layout: String,
    /// Which counters beyond life this game tracks, in display order — any of
    /// `commander_damage` / `poison` / `energy` / `experience`. Empty for a game
    /// that only tracks life; `life` is always tracked and never listed.
    #[serde(default)]
    pub counters: Vec<String>,
    /// `active` or `finished`. Only an active session accepts edits.
    pub status: String,
    /// Seats in `position` order.
    pub players: Vec<LifePlayer>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Where one of a seat's counters currently stands — folded out of the history, so
/// only counters a seat has actually moved appear (an absent entry is `0`). A seat's
/// life lives on the seat itself, not here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeCounter {
    /// `poison` / `energy` / `experience` / `commander_damage`.
    pub counter: String,
    pub player_id: i64,
    /// For `commander_damage`, the seat whose commander dealt it; null otherwise.
    #[serde(default)]
    pub source_player_id: Option<i64>,
    pub value: i64,
}

/// One recorded change. `delta` is what the change was; `life_after` is what it left
/// this row's `counter` on (the seat's life, for `life`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeEvent {
    pub id: i64,
    pub player_id: i64,
    pub delta: i64,
    pub life_after: i64,
    /// Which counter moved: `life` / `poison` / `energy` / `experience` /
    /// `commander_damage`. Defaults to `life` on a server that predates counters.
    #[serde(default = "counter_life")]
    pub counter: String,
    /// For `commander_damage`, the seat whose commander dealt it; null otherwise.
    #[serde(default)]
    pub source_player_id: Option<i64>,
    /// `adjust` (relative) or `set` (absolute correction).
    pub kind: String,
    pub created_at: String,
}

fn counter_life() -> String {
    "life".to_string()
}

/// One tracked game in full: its header + seats, plus every recorded life change
/// in the order they happened. Returned by the detail read and by every write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeSessionDetail {
    pub session: LifeSession,
    /// Every non-life counter that has been moved, folded out of `events` — so the
    /// table's full state arrives without replaying the history client-side.
    #[serde(default)]
    pub counters: Vec<LifeCounter>,
    pub events: Vec<LifeEvent>,
}

/// What one life change returned: the seat as it now stands, plus the change that
/// was recorded. This — not the whole session — is what the life endpoint echoes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeChange {
    pub player: LifePlayer,
    /// Where the affected counter now stands, for a change to something other than
    /// `life` (whose value is the seat's own `life`).
    #[serde(default)]
    pub counter: Option<LifeCounter>,
    pub event: LifeEvent,
}

/// A deck's record across finished tracked games.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeDeckRecord {
    pub deck_id: i64,
    pub deck_name: String,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
    /// `wins / games` in `0.0..=1.0`, or null with no games.
    pub win_rate: Option<f64>,
    pub last_played_at: Option<String>,
}
