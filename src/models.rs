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
    pub has_subtypes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPrices {
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
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
    #[serde(default)]
    pub secret_lair_bonus: bool,
    #[serde(default)]
    pub secret_lair_spend_incentive: bool,
    #[serde(default)]
    pub faces: Vec<CardFace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub date: String,
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
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
pub struct CollectionSource {
    pub provider: String,
    pub external_id: String,
    pub url: String,
    pub last_synced_at: Option<String>,
    pub smart: bool,
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

// Movers ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMover {
    pub card: Card,
    pub quantity: i64,
    pub foil_quantity: i64,
    pub value_now: String,
    pub value_prev: String,
    pub change_usd: String,
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
    pub value_now: String,
    pub value_prev: String,
    pub change_usd: String,
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

// Import / sync --------------------------------------------------------------

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
    pub stopped_early: bool,
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

// ---------------------------------------------------------------------------
// Decks
// ---------------------------------------------------------------------------

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

/// One deck that wants a [`NeededCard`], in the game's cross-deck needed list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeededCardDeck {
    pub id: i64,
    pub name: String,
}

/// A card the caller's decks collectively want more copies of than they own.
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
