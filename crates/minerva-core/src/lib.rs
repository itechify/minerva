//! Minerva core: the single test seam.
//!
//! Inputs are plain data (title captures and Status Bar captures or their readings, the game window, the clock, the player's actions and hotkeys); the output
//! is a [`PanelState`] holding everything the overlay renders. No OS or UI dependencies
//! live here.

mod bests_window;
pub mod bug_report;
mod deal_finder;
mod exp_bar;
mod game_data;
mod grind_tracker;
mod hotkey_settings;
mod item_card;
mod item_data;
mod item_lookup;
mod ink;
mod item_tooltip;
mod leaderboards;
mod loader;
mod map_data;
mod map_index;
mod map_mark;
mod map_preview;
mod market_prices;
mod matcher;
mod minimap_title;
pub mod meowdb;
mod mob_card;
pub mod osms;
pub mod osms_items;
mod pace;
mod panel;
mod personal_best;
mod player_shop;
mod pictures;
mod portal_overrides;
mod portals;
mod prep;
mod recogniser;
mod search;
mod session;
mod settings;
mod setup;
mod shown_map;
mod status_bar;
mod tracker;
mod travel_links;
mod ui_size;
mod updates;
mod world;

pub use bests_window::{PersonalBestsCharacter, PersonalBestsMap, PersonalBestsVisit, PersonalBestsWindow};
pub use bug_report::{BugReport, ReportAbout};
pub use deal_finder::{DealBadge, DealBadges, DealFinderView, DealGrade, DealItem, Verdict};
pub use exp_bar::{ExpBar, ExpBarPlace, ExpBarReader, ExpBarReading, ExpFigures};
pub use game_data::{GameData, Refresh};
pub use grind_tracker::{ClientOffset, GrindTrackerWindow};
pub use hotkey_settings::{KeyPress, RebindProblem, SettingsNotice, SettingsView};
pub use item_card::{
    Edge, ItemCard, ItemCardChoice, ItemCardView, MarketEdit, MarketLine, NameProblem, ShopPrice, ShownItemCard,
};
pub use market_prices::{MARKET_PRICES_VERSION, MarketImport, MarketPrice, SavedMarketLine, SavedMarketPrices};
pub use item_lookup::{ItemChoice, ItemHit, ItemLookup, ItemLookupLine, OurWindow};
pub use item_tooltip::{ItemCapture, ItemTooltip, ItemTooltipReader, ItemTooltipReading};
pub use leaderboards::{
    LeaderboardAnswer, LeaderboardCharacter, LeaderboardEntry, LeaderboardStatus, LeaderboardToggle, LeaderboardUpload,
    RejectedEntry, RejectedListing, RejectedReason, read_leaderboard_answer,
};
pub use map_data::{
    HiddenPortal, MapData, MapDataStatus, MapSummary, Mob, PerParty, Portal, PortalKind,
};
pub use map_index::{Fetched, MapDataSource, MapEntry, SourceError};
pub use map_mark::MapMark;
pub use map_preview::{Landing, Leads, MapPreview, PicturePoint, PortalMark, PreviewImage, SpawnMark};
pub use mob_card::{DamageRange, Elements, HpPerExp, MobCard, MobDetails};
pub use portal_overrides::PortalOverride;
pub use panel::{
    Action, CollapsedSections, DEFAULT_PANEL_WIDTH, Foreground, GameWindow, InsideOffset, Layer, MIN_PANEL_WIDTH, PanelSizes,
    PlacementMode, ScreenPoint,
    ScreenRect, Section, Size,
};
pub use personal_best::{BESTS_VERSION, CharacterBests, MapBests, SavedBests, VisitBest};
pub use player_shop::{ListedPrice, PlayerShopReader, ShopCapture, ShopHover, ShopList, ShopReading, ShopRow};
pub use search::{
    DensityTier, Destination, HuntingTarget, ItemMatch, ItemsMissing, MapMatch, Matches, MobChoice, MobMatch, RouteStop,
    SearchResult, SearchState, SearchView, Trip,
};
pub use session::{Character, Deaths, ExpRate, ExpTracking, Forecast, PacePoint, PaceTrace, TrackingState};
pub use settings::{Hotkey, Hotkeys, Settings};
pub use setup::{SetupReason, SetupState};
pub use shown_map::{Candidate, Candidates, LevelRange, Outline};
pub use status_bar::{STATUS_BAR_STRIP, StatusBar, StatusBarCapture, StatusBarReader, StatusBarReading};
pub use updates::{
    Build, DownloadAnswer, QuitItem, UpdateAnswer, UpdateJob, UpdateNotice, UpdateOffer, UpdateStage, UpdateStatus, UpdateStep,
    Updates,
};
pub use world::{Distance, Step, Travel};

use bests_window::BestsWindow;
use bug_report::{Event, MapFacts, Outcome, Press, RecordedSource, Recorder, Region};
use deal_finder::DealFinder;
use game_data::Refreshes;
use grind_tracker::GrindTracker;
use hotkey_settings::HotkeySettings;
use item_card::{Editing, ItemCards, MESOWATCH_URL, Pressed};
use loader::{Held, Load, Loaded, Loader};
use market_prices::MarketPrices;
use item_lookup::ItemLookups;
use leaderboards::Leaderboards;
use search::{Search, Searched};
use shown_map::ShownMap;
use matcher::{Match, Matcher, Snap};
use pictures::Pictures;
use panel::Panel;
use portal_overrides::PORTAL_OVERRIDES;
use session::Session;
use setup::Setup;
pub use recogniser::{TitleCapture, TitleLines, TitleReader, TitleReading};
use tracker::Tracker;
use updates::{Chosen, Updater};
use serde::{Deserialize, Serialize};
use osms::MapSet;
use osms_items::ItemSet;
use std::collections::{BTreeMap, VecDeque};

/// A title capture's image or a Status Bar's, or the game's whole client area for
/// Item Lookup: tightly packed RGBA, top-down rows.
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Capture time in milliseconds (any monotonic origin).
    pub at_ms: u64,
}

/// A rectangle in physical pixels, relative to the game window's client area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The map the panel shows: a real map from the map list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMap {
    /// The 9-digit map ID, zero padding kept.
    pub id: String,
    pub street_name: String,
    pub map_name: String,
    /// The map's MeowDB page, which the title and the open-page hotkey open.
    pub url: String,
    /// Its Map Mark, by OSMS's name for it (see [`PanelState::map_marks`]); `None` for a map
    /// without one. While it isn't settled which of the maps sharing a title the player is
    /// on, the one they all share, or none.
    pub mark: Option<String>,
}

/// Search's credit (ADR 0008): its results and routes are worked out from OSMS Data
/// Explorer's data, and open no pages, so MeowDB isn't credited.
pub const OSMS_ONLY_CREDIT: &str = "Data: OSMS Data Explorer (osmsdataexplorer.com)";

/// The panel's credit (ADR 0008): its data is OSMS Data Explorer's, and its title and
/// mobs' names open NiaMeowDB's pages.
pub const OSMS_AND_MEOWDB_CREDIT: &str =
    "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)";

/// The Minerva site, the player manual, which Help & Setup's About & Credits opens.
pub const SITE_URL: &str = "https://owlofminerva.app/";

/// The FAQ's answer on a Minimap Title Minerva can't find, which the panel links to while it
/// can't ([`PanelState::title_lost`]).
pub const TITLE_HELP_URL: &str = "https://owlofminerva.app/faq/#minimap-not-found";

/// Minerva's source and releases, which About & Credits opens.
pub const GITHUB_URL: &str = "https://github.com/itechify/minerva";

/// Minerva's Discord server, which About & Credits opens: an invite that never expires.
pub const DISCORD_URL: &str = "https://discord.gg/NnbnVWkRrF";

/// Whether minimap titles can be recognised as real maps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Recognition {
    /// OSMS's file set hasn't arrived yet (it is asked for at startup).
    #[default]
    Starting,
    Ready,
    /// OSMS hasn't been reached for over 7 days, so titles are snapped to a copy of its
    /// file set saved at `saved_at_unix_ms` (ms since the Unix epoch).
    SavedCopy {
        #[serde(rename = "savedAtUnixMs")]
        saved_at_unix_ms: u64,
    },
    /// OSMS couldn't be reached and no file set is saved, so readings can't be snapped to
    /// real maps. The map list is asked for again every minute.
    Unavailable,
    /// OSMS's file set doesn't read, and there is no earlier one to fall back on:
    /// Minerva needs an update.
    FormatChanged,
}

/// Everything the overlay renders.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    pub recognition: Recognition,
    /// The last map the core was confident about.
    pub map: Option<CurrentMap>,
    /// The latest frame didn't confirm the map shown (unreadable, low confidence, or
    /// a different map not yet confirmed).
    pub not_sure: bool,
    /// The maps sharing the shown map's title, when the core couldn't tell which of
    /// them the player is on; `None` when it could.
    pub candidates: Option<Candidates>,
    /// The shown map's Map Summary, mobs and Hidden Portals from OSMS's file set, or why they
    /// aren't there.
    pub data: MapDataStatus,
    /// The shown map's Map Preview: its picture and every portal on it, marked. `None`
    /// with no map shown, on a map without a picture, and while it isn't settled which
    /// of the maps sharing a title the player is on.
    pub map_preview: Option<MapPreview>,
    /// [`OSMS_AND_MEOWDB_CREDIT`] whenever a map is shown.
    pub credit: Option<&'static str>,
    /// The Mob Card of each mob the panel lists, in its order; empty while it lists none.
    pub mob_cards: Vec<MobCard>,
    /// Every Map Mark, by OSMS's name for it, which each map shown anywhere names its own
    /// by (the panel's title, Hidden Portals, the Map Preview, search, the route, the map
    /// setup found and the Personal Bests window); empty until OSMS's file set is read.
    pub map_marks: BTreeMap<String, MapMark>,
    /// Expanded to the full panel, or collapsed to a small icon.
    pub expanded: bool,
    /// The sections the player collapsed to their headings.
    pub collapsed_sections: CollapsedSections,
    /// A visible game window was found (not minimised).
    pub game_found: bool,
    /// Where the player wants the panel: inside, docked or detached.
    pub placement_mode: PlacementMode,
    /// The expanded panel's width (CSS px, logical), as the player set it by dragging
    /// a bottom corner: never under [`MIN_PANEL_WIDTH`]. The shell sizes the window to it.
    pub panel_width: u32,
    /// Where the overlay goes on screen; `None` while there is no game window, unless
    /// the panel is detached. Inside the game it never covers the Minimap Title where it was found, the Status
    /// Bar's Character or its EXP figures: the overlay shows in screen captures, ours
    /// included.
    pub placement: Option<ScreenRect>,
    /// Whether the overlay shows, on top of everything, or is hidden: it shows only with
    /// the game, while the game or one of our windows is in front.
    pub layer: Layer,
    /// The player can drag the panel by its header (inside or detached). The overlay is
    /// clickable whenever it shows, and never takes the keyboard on a click (ADR 0001).
    pub draggable: bool,
    /// First-run setup and re-setup.
    pub setup: SetupState,
    /// Where on screen to capture for the Minimap Title (physical px, ADR 0021): where it was
    /// last found, while it is there, or the game's whole client area while it is looked for;
    /// only while there is a game window and the overlay can show (see [`Layer`]).
    pub title_capture: Option<ScreenRect>,
    /// `title_capture` is the whole client area, as the title is looked for: the shell
    /// captures it every [`TITLE_SEARCH_MS`], not every [`TITLE_READ_MS`].
    pub title_searching: bool,
    /// Where our windows show on screen, as placed here: the overlay while it shows, the Grind
    /// Tracker, the Personal Bests window, Item Lookup's alert, the Item Card and the Deal
    /// Badges. A capture of the screen holds them, and the Minimap Title is never read in them
    /// ([`TitleCapture::ours`]): the panel's own map name and NOT SURE pass for a minimap.
    #[serde(skip_serializing)]
    pub our_windows: Vec<ScreenRect>,
    /// No Minimap Title has been found for [`TITLE_LOST_AFTER_MS`]: the minimap may be
    /// closed. Not while a title is found, even one that doesn't read.
    pub title_lost: bool,
    /// Read the EXP figures in each capture of `status_bar_capture` (ADR 0017), and where
    /// they were last found; `None` while the Grind Tracker is off.
    pub exp_figures: Option<ExpFigures>,
    /// What the EXP bar showed when its figures last read whole; `None` while the Grind
    /// Tracker is off or nothing has read yet.
    pub exp_bar: Option<ExpBar>,
    /// EXP tracking: the Session's state, Time to Level, rates and deaths.
    pub exp_tracking: ExpTracking,
    /// Where on screen to capture for the Status Bar (physical px): a strip of
    /// [`STATUS_BAR_STRIP`] rows across the bottom of the game's client area, while there is a
    /// game window and the overlay can show (see [`Layer`]); `None` otherwise.
    pub status_bar_capture: Option<ScreenRect>,
    /// What the Status Bar showed in the last capture of it; `None` while it isn't found.
    pub status_bar: Option<StatusBar>,
    /// The Character playing, as the Status Bar names them, whom the Session is for; `None`
    /// until a name is read, and after a switch to a Character whose name can't be read.
    pub character: Option<Character>,
    /// The Grind Tracker's window, which shows `exp_tracking`.
    pub grind_tracker: GrindTrackerWindow,
    /// The Personal Bests window, with its rows: sent to that window alone, not with the
    /// overlay's state.
    #[serde(skip_serializing)]
    pub personal_bests: PersonalBestsWindow,
    /// The uploads to the Map Leaderboards under way (#110), for the shell to make: each
    /// shown Character's whole set of entries, or the deletion of one taken off. The shell
    /// makes each once, by its id, and tells the answer to [`Minerva::on_leaderboard_answer`].
    /// Not sent to the overlay.
    #[serde(skip_serializing)]
    pub leaderboard_uploads: Vec<LeaderboardUpload>,
    /// The title of the game window to look for: "MapleStory", or one the player
    /// picked by hand.
    pub game_window_title: String,
    /// The global hotkeys to register, as the overlay names them.
    pub hotkeys: Hotkeys,
    /// Hotkeys the shell couldn't register, so they do nothing, to tell the player.
    pub unavailable_hotkeys: Vec<String>,
    /// Those of them that don't name a key combination; the rest another app has.
    pub invalid_hotkeys: Vec<String>,
    /// The shell keeps the hotkeys registered only while this is set: while the game or
    /// one of our windows is in front. Other apps keep their own Ctrl+Shift shortcuts.
    /// Not while the player rebinds one: the combination they press must reach the overlay.
    pub hotkeys_live: bool,
    /// The settings view, which takes over the panel while it is open; `None` while it
    /// is closed. Like the search view, it has the keyboard while it is open.
    pub settings_view: Option<SettingsView>,
    /// The player closed first-run setup without going through its warning, so the app
    /// quits.
    pub quit: bool,
    /// Search, over every map in OSMS's file set and every item in its item files.
    pub search: SearchState,
    /// What the latest Item Lookup came to, for a few seconds; `None` once it has gone.
    pub item_lookup: Option<ItemLookupLine>,
    /// Where its alert goes on screen: at the top centre of the game window, clear of the
    /// captured regions; `None` while there is none, no game window, or another app in
    /// front (see [`Layer`]).
    pub item_lookup_placement: Option<ScreenRect>,
    /// The Item Card showing; `None` while there is none.
    pub item_card: Option<ShownItemCard>,
    /// Where the Item Card goes on screen: at the pointer, inside the game window and clear
    /// of the captured regions; `None` while there is none, no game window, or another app
    /// in front (see [`Layer`]).
    pub item_card_placement: Option<ScreenRect>,
    /// The Deal Finder (ADR 0014), as the panel shows it.
    pub deal_finder: DealFinderView,
    /// Where on screen to capture for the Deal Finder (physical px): the game's client area,
    /// while it is on, and only while the overlay can show (see [`Layer`]); `None` otherwise.
    pub deal_finder_capture: Option<ScreenRect>,
    /// The Deal Badges beside the Player Shop's listings, and where their window goes; `None`
    /// while the Deal Finder is off, no shop was found, or the overlay can't show, and while
    /// an Item Card or Item Lookup press's capture is taken, which it would be in.
    pub deal_badges: Option<DealBadges>,
    /// The settings view's Game Data: when OSMS's data was fetched, and the player's Refresh.
    pub game_data: GameData,
    /// Updates (ADR 0019): whether a newer version is out, for the tray and Help & Setup, and
    /// the check under way for the shell to run. Nothing of it shows on the panels.
    pub updates: Updates,
}

impl PanelState {
    /// Where the overlay shows on screen: its placement, while it isn't hidden (see
    /// [`Layer`]). It shows in screen captures, ours included.
    pub(crate) fn panel_showing(&self) -> Option<ScreenRect> {
        self.placement.filter(|_| self.layer != Layer::Hidden)
    }
}

/// What a hotkey asks the shell to do at once, beyond rendering the panel state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Open this page in the player's browser (the open-page hotkey, a click on a listed
    /// mob's name or on a map's in the Personal Bests window, or About & Credits' link to
    /// the Minerva site).
    OpenInBrowser { url: String },
    /// Bring the overlay, with the search view already open, back in front (the search
    /// hotkey): the game may have come in front of it. Our own window only (ADR 0001).
    RaiseOverlay,
    /// Item Lookup (ADR 0007) or the Item Card (ADR 0010): read the pointer's position and
    /// capture the game's whole client area now, while the Item Tooltip shows, for
    /// [`Minerva::on_item_capture`] (or [`Minerva::on_item_tooltip_reading`] with what an
    /// [`ItemTooltipReader`] reads in it, or `None` if the capture failed). The Item Card
    /// isn't placed meanwhile: it must be out of the capture.
    CaptureItemTooltip,
}

/// How often the shell captures [`PanelState::title_capture`] where the Minimap Title was
/// found (ADR 0021)...
pub const TITLE_READ_MS: u64 = 333;
/// ...and how often while it looks for it in the whole client area: a player with the minimap
/// closed doesn't pay for a search three times a second.
pub const TITLE_SEARCH_MS: u64 = 1000;
/// With no Minimap Title found for this long, the panel says it can't find it
/// ([`PanelState::title_lost`]).
pub const TITLE_LOST_AFTER_MS: u64 = 10_000;
/// A title found where it was, but read as no map for this long, is looked for afresh in the
/// whole client area.
pub const TITLE_UNREAD_SEARCH_MS: u64 = 3_000;

/// How often the shell captures [`PanelState::deal_finder_capture`] for the Deal Finder:
/// a listing's badge follows the player's scrolling within half a second.
pub const DEAL_FINDER_CAPTURE_MS: u64 = 400;

/// Something the player can do with a hotkey or with the matching icon at the top of
/// the panel, or with the panel's settings icon, an icon at the top of the Grind
/// Tracker's window or About & Credits' link (which have no hotkeys). Either way it goes
/// through [`Minerva::press`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PanelControl {
    /// Collapse the panel to its icon, or expand it again.
    TogglePanel,
    /// Switch placement: inside, docked right, docked left, detached.
    CyclePlacement,
    /// Open the shown map's MeowDB page in the browser.
    OpenMeowdbPage,
    /// Help & Setup: open setup again.
    Setup,
    /// Show the next of the maps sharing the shown map's title.
    ChooseCandidate,
    /// Open the search view.
    Search,
    /// Open the settings view, to rebind the hotkeys.
    Settings,
    /// Open the Grind Tracker's window, or close it.
    ToggleGrindTracker,
    /// The Grind Tracker window's reset button: start the Session afresh, back to waiting
    /// for an EXP change.
    ResetSession,
    /// The Grind Tracker window's Undo, after a reset: bring the old Session back.
    UndoReset,
    /// The × on the Grind Tracker's Best row: remove the shown map's Personal Best the row
    /// shows, this Visit's or the best of the earlier Visits.
    RemoveBest,
    /// The Grind Tracker window's ×: close it.
    CloseGrindTracker,
    /// The Grind Tracker window's Personal Bests button, or its Best row's "Best": open
    /// the Personal Bests window, or close it.
    TogglePersonalBests,
    /// The Personal Bests window's ×: close it.
    ClosePersonalBests,
    /// About & Credits' link in Help & Setup: open the Minerva site in the browser.
    OpenSite,
    /// The panel's line saying it can't find the Minimap Title ([`PanelState::title_lost`]):
    /// open the FAQ's answer on it in the browser.
    OpenTitleHelp,
    /// About & Credits' GitHub link: open Minerva's source in the browser.
    OpenGithub,
    /// About & Credits' Discord link: open Minerva's Discord in the browser.
    OpenDiscord,
    /// Help & Setup's "What's new": open the newer version's release page in the browser.
    /// Nothing while no newer version is known.
    OpenRelease,
    /// The tray's "Update to X.Y.Z…", or Help & Setup's Update: choose to update to the newer
    /// version. A build that can't install opens its release page instead; one that can
    /// opens Help & Setup, which asks first while a Session is running (ADR 0019).
    Update,
    /// Turn the Deal Finder on or off.
    ToggleDealFinder,
}

/// The most the Deal Margin can be, in percent.
const MAX_DEAL_MARGIN: u32 = 99;

/// The game window's title, unless the player picked another by hand.
pub const GAME_WINDOW_TITLE: &str = "MapleStory";

/// This Minerva's version, which every file set it fetches is saved with: the four files
/// that hold the version agree (`docs/releasing.md`).
pub const MINERVA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// How long a file set is used before it is fetched afresh: a day. Game data changes only
/// on patch days, and a day is soon enough to catch one; the player's Refresh catches it
/// sooner. At startup, a set this version saved under a day ago by the wall clock is used
/// as it is, but never one another version saved: a new version may read a set the old one
/// couldn't. While running, the set is fetched afresh every day by capture time, however
/// old the one held, so a set used at one daily ask is always fetched afresh at the next.
const SET_RECHECK_MS: u64 = 24 * 60 * 60 * 1000;

/// With no file set at all, how long to wait before asking again after OSMS couldn't be
/// reached: never on every frame.
const SET_RETRY_MS: u64 = 60_000;

/// A file set saved longer ago than this (wall clock) is shown as a Saved Copy, with its
/// date: game data changes only on patch days, so a younger one says nothing.
const SAVED_COPY_AFTER_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// The Minerva core: feed it frames, render its [`PanelState`].
pub struct Minerva {
    /// Reads frames fed to [`Minerva::on_frame`]; loaded on the first one, so a shell
    /// that reads on its own thread never loads it twice.
    reader: Option<TitleReader>,
    /// Reads frames fed to [`Minerva::on_exp_bar_frame`], and the EXP figures in captures fed
    /// to [`Minerva::on_status_bar_capture`]; loaded on the first one.
    exp_bar_reader: Option<ExpBarReader>,
    /// What the EXP bar showed when its figures last read whole.
    exp_bar: Option<ExpBar>,
    /// Where the EXP figures were last read in a Status Bar capture (found there, or where
    /// they had been), relative to the game's client area, and that area's size then:
    /// where to read them while the Status Bar can't be found, until the client area
    /// changes size, and what our windows keep clear of.
    exp_figures_at: Option<(Size, Rect)>,
    /// The Session measured from the EXP bar's readings.
    session: Session,
    /// The Grind Tracker's window.
    grind_tracker: GrindTracker,
    /// The Personal Bests window.
    bests_window: BestsWindow,
    /// The data source, whose answers a Bug Report writes down.
    source: RecordedSource,
    /// OSMS's file set, once read: every map's data, the map list and the shown map's
    /// figures among it, and when to ask for it next.
    maps: Loader<MapSet>,
    /// The player's Refresh of OSMS's file set and item files.
    refreshes: Refreshes,
    /// What the file set is read with: [`PORTAL_OVERRIDES`], but for tests of the list.
    portal_overrides: Vec<PortalOverride>,
    /// Built from the map list, once there is one.
    matcher: Option<Matcher>,
    tracker: Tracker,
    shown_map: ShownMap,
    /// The wall clock (ms since the Unix epoch), as the shell last told it.
    wall_clock: Option<u64>,
    /// OSMS's pictures the panel asked for: the Map Preview's picture, and the map's mobs'
    /// animations for their Mob Cards.
    pictures: Pictures,
    /// What the player typed, highlighted and picked in Search.
    search: Search,
    /// The settings view, and the hotkey being rebound in it.
    hotkey_settings: HotkeySettings,
    /// Reads captures fed to [`Minerva::on_item_capture`]; loaded on the first one.
    item_reader: Option<ItemTooltipReader>,
    /// The latest Item Lookup.
    item_lookups: ItemLookups,
    /// OSMS's item files, for the Item Card, and when to ask for them next.
    items: Loader<ItemSet>,
    /// The Item Card showing.
    item_cards: ItemCards,
    /// A capture for an Item Card was asked for and its reading hasn't come back.
    card_capturing: bool,
    /// The market prices the player keeps, which the Item Card shows.
    market_prices: MarketPrices,
    /// The Deal Finder, and the Player Shop it read last.
    deal_finder: DealFinder,
    /// Reads captures fed to [`Minerva::on_player_shop_capture`]; loaded on the first one.
    shop_reader: Option<PlayerShopReader>,
    /// Reads captures fed to [`Minerva::on_status_bar_capture`]; loaded on the first one.
    status_bar_reader: Option<StatusBarReader>,
    /// Where the Status Bar's level, job and name were last found, and the game's client
    /// area then: our windows keep clear of them while the client area stays where it was.
    status_bar_area: Option<(ScreenRect, ScreenRect)>,
    /// Where the Minimap Title was last found, and the game's client area then: our windows
    /// keep clear of it while the client area stays where it was (ADR 0021)...
    title_found: Option<(ScreenRect, ScreenRect)>,
    /// ...and it is captured there while it is there: until a capture there finds none, or
    /// reads it as no map for [`TITLE_UNREAD_SEARCH_MS`].
    title_locked: bool,
    /// When a capture first found no Minimap Title, since the last that found one.
    title_missing_since: Option<u64>,
    /// When a capture where the title was found first read it as no map, since the last that
    /// read it as one.
    title_unread_since: Option<u64>,
    /// The Characters shown on the Map Leaderboards, and the uploads for them.
    leaderboards: Leaderboards,
    /// The checks for a newer version, and its download and install when the player chooses.
    updater: Updater,
    /// What the shell must do that came of time passing (a page MeowDB's answer settled),
    /// for [`Minerva::take_request`].
    requests: VecDeque<Request>,
    panel: Panel,
    setup: Setup,
    /// The last settings restored; the panel's own fields live in `panel`.
    settings: Settings,
    state: PanelState,
    /// The run so far, for a Bug Report (ADR 0009).
    recorder: Recorder,
}

impl Minerva {
    /// A core using the given data source and default settings. OSMS's file set is
    /// asked for on the first tick or frame.
    pub fn new(source: impl MapDataSource + Send + 'static) -> Self {
        Self::with_source(RecordedSource::live(Box::new(source)))
    }

    /// A core asking `source`: a live one, or a replay's.
    pub(crate) fn with_source(source: RecordedSource) -> Self {
        let mut minerva = Self {
            reader: None,
            exp_bar_reader: None,
            exp_bar: None,
            exp_figures_at: None,
            session: Session::default(),
            grind_tracker: GrindTracker::default(),
            bests_window: BestsWindow::default(),
            source,
            maps: Loader::default(),
            refreshes: Refreshes::default(),
            portal_overrides: PORTAL_OVERRIDES.to_vec(),
            matcher: None,
            tracker: Tracker::default(),
            shown_map: ShownMap::default(),
            wall_clock: None,
            pictures: Pictures::default(),
            search: Search::default(),
            hotkey_settings: HotkeySettings::default(),
            item_reader: None,
            item_lookups: ItemLookups::default(),
            items: Loader::default(),
            item_cards: ItemCards::default(),
            card_capturing: false,
            market_prices: MarketPrices::default(),
            deal_finder: DealFinder::default(),
            shop_reader: None,
            status_bar_reader: None,
            status_bar_area: None,
            title_found: None,
            title_locked: false,
            title_missing_since: None,
            title_unread_since: None,
            leaderboards: Leaderboards::default(),
            updater: Updater::default(),
            requests: VecDeque::new(),
            panel: Panel::default(),
            setup: Setup::default(),
            settings: Settings::default(),
            state: PanelState::default(),
            recorder: Recorder::default(),
        };
        minerva.sync_panel();
        minerva.recorder = Recorder::new(&minerva.state);
        minerva
    }

    /// The same core, reading OSMS's file set with these portal overrides in place of
    /// [`PORTAL_OVERRIDES`]. Only for tests of the override list (integration tests can't
    /// see test-only items); the app keeps the list it ships.
    #[doc(hidden)]
    pub fn with_portal_overrides(mut self, overrides: Vec<PortalOverride>) -> Self {
        self.portal_overrides = overrides;
        self
    }

    /// Read a title capture's frame, captured at the screen's top-left, and update the panel
    /// state: as [`Minerva::on_title_capture`] does.
    pub fn on_frame(&mut self, frame: Frame) -> &PanelState {
        self.on_title_capture(TitleCapture { frame, origin: ScreenPoint { x: 0, y: 0 }, ours: Vec::new() })
    }

    /// Read a capture of [`PanelState::title_capture`] and update the panel state: the same as
    /// [`Minerva::on_title_read`] with what a [`TitleReader`] reads in it, here.
    pub fn on_title_capture(&mut self, capture: TitleCapture) -> &PanelState {
        let reading = self.reader.get_or_insert_with(TitleReader::bundled).read(&capture);
        self.on_title_read(capture, reading)
    }

    /// Take what a title capture read as (read by a [`TitleReader`], possibly on another
    /// thread) and update the panel state.
    pub fn on_reading(&mut self, reading: TitleReading) -> &PanelState {
        let event = Event::Title { at_ms: reading.at_ms, lines: reading.lines.clone(), found: reading.found, same: false };
        self.record_input(event, Some(reading.at_ms), None);
        self.take_title_reading(reading)
    }

    /// As [`Minerva::on_reading`], with the capture read, whose crop of the title a Bug
    /// Report may keep.
    pub fn on_title_read(&mut self, capture: TitleCapture, reading: TitleReading) -> &PanelState {
        let crop = bug_report::title_crop(&capture, reading.found);
        let event = Event::Title { at_ms: reading.at_ms, lines: reading.lines.clone(), found: reading.found, same: false };
        self.record_input(event, Some(reading.at_ms), crop);
        self.take_title_reading(reading)
    }

    /// Where the Minimap Title is to be captured next (ADR 0021): where `reading` found it,
    /// while it is there; the whole client area once it isn't. And since when none has been
    /// found.
    fn follow_title(&mut self, reading: &TitleReading) {
        let client = self.panel.game.as_ref().map(|game| game.client);
        match (reading.found, client) {
            (Some(found), Some(client)) => {
                self.title_found = Some((client, found));
                self.title_locked = true;
                self.title_missing_since = None;
            }
            (Some(_), None) => self.title_missing_since = None,
            (None, _) => {
                self.title_locked = false;
                self.title_unread_since = None;
                self.title_missing_since.get_or_insert(reading.at_ms);
            }
        }
        self.state.title_lost =
            self.title_missing_since.is_some_and(|since| reading.at_ms.saturating_sub(since) >= TITLE_LOST_AFTER_MS);
    }

    fn take_title_reading(&mut self, reading: TitleReading) -> &PanelState {
        self.follow_title(&reading);
        self.load_osms(reading.at_ms);
        let (Some(matcher), Some(set)) = (&self.matcher, self.maps.held()) else {
            // Without the map list nothing can be recognised.
            return self.sync_panel();
        };
        let snap = reading.lines.and_then(|lines| match matcher.snap(&lines.street, &lines.map)? {
            Match::Sure(snap) => Some(snap),
            // Names differing only in a numeral the reading doesn't make out: the
            // one within the player's reach, if just one is; otherwise not sure.
            Match::OneOf { maps } => {
                let reach = self.shown_map.within_reach(set);
                let mut reachable = maps.into_iter().filter(|&i| {
                    matcher.same_title_as(i).into_iter().any(|j| reach.contains(&matcher.map(j).id))
                });
                match (reachable.next(), reachable.next()) {
                    (Some(index), None) => Some(Snap { index }),
                    _ => None,
                }
            }
        });
        let current_map = |i: usize| {
            let entry = matcher.map(i);
            CurrentMap {
                id: entry.id.clone(),
                street_name: entry.street_name.clone(),
                map_name: entry.map_name.clone(),
                url: meowdb::map_url(&entry.id),
                mark: entry.mark.clone(),
            }
        };
        // Found where it was, but read as no map for a while: the box may cut a moved minimap's
        // title. It is looked for afresh.
        if reading.found.is_some() && snap.is_none() {
            let since = *self.title_unread_since.get_or_insert(reading.at_ms);
            if reading.at_ms.saturating_sub(since) >= TITLE_UNREAD_SEARCH_MS {
                self.title_locked = false;
                self.title_unread_since = None;
            }
        } else {
            self.title_unread_since = None;
        }
        let tracked = self.tracker.update(snap);
        let titled = tracked.current.map_or_else(Vec::new, |i| matcher.same_title_as(i));
        let maps = titled.into_iter().map(current_map).collect();
        // Another title confirmed is another map: a new Visit. Losing the title, or choosing
        // among the maps sharing it, is not.
        if self.shown_map.show(maps, set) {
            self.session.arrive();
        }
        self.sync_shown_map();
        self.state.not_sure = tracked.not_sure;
        self.sync_panel()
    }

    /// Read a frame of the EXP figures alone, as if they had been found there, and update
    /// the panel state, as [`Minerva::on_exp_bar_reading`] does with what an
    /// [`ExpBarReader`] reads in it. The figures the shell reads are in its Status Bar
    /// captures ([`Minerva::on_status_bar_read`]).
    pub fn on_exp_bar_frame(&mut self, frame: Frame) -> &PanelState {
        let reading = self.exp_bar_reader.get_or_insert_with(ExpBarReader::bundled).read(&frame);
        let event = Event::ExpBar { at_ms: reading.at_ms, bar: reading.bar, place: reading.place };
        self.record_input(event, Some(reading.at_ms), Some((Region::ExpBar, frame)));
        self.take_exp_bar_reading(reading)
    }

    /// Take what the EXP figures read as (read by an [`ExpBarReader`], possibly on another
    /// thread) and update the panel state. A reading whose figures don't read whole keeps
    /// the last one; time still passes for the Session, which says so once the bar has gone
    /// unread for a while.
    pub fn on_exp_bar_reading(&mut self, reading: ExpBarReading) -> &PanelState {
        let event = Event::ExpBar { at_ms: reading.at_ms, bar: reading.bar, place: reading.place };
        self.record_input(event, Some(reading.at_ms), None);
        self.take_exp_bar_reading(reading)
    }

    fn take_exp_bar_reading(&mut self, reading: ExpBarReading) -> &PanelState {
        if let Some(game) = &self.panel.game {
            let client = game.client;
            if let Some(place) = reading.place {
                let a = place.area;
                let at = Rect { x: (a.x - client.x).max(0) as u32, y: (a.y - client.y).max(0) as u32, width: a.width, height: a.height };
                self.exp_figures_at = Some((Size { width: client.width, height: client.height }, at));
            }
        }
        match reading.bar {
            Some(bar) if self.tracking_on() => self.session.on_bar(bar, reading.at_ms),
            None if self.tracking_on() => self.session.on_unread(reading.at_ms),
            _ => self.session.tick(reading.at_ms),
        }
        if reading.bar.is_some() {
            self.exp_bar = reading.bar;
        }
        self.sync_panel()
    }

    fn sync_shown_map(&mut self) {
        let map = self.shown_map.map().map(|map| CurrentMap { mark: self.shown_map.mark(), ..map.clone() });
        let saved_copy_from = self.maps.saved_copy_from(self.wall_clock);
        let data = self.maps.held().map_or(MapDataStatus::NoMap, |maps| self.shown_map.data(maps, saved_copy_from));
        // Distances are measured from the map shown (with Candidates, the one shown).
        let moved = map.as_ref().map(|m| &m.id) != self.state.map.as_ref().map(|m| &m.id);
        let paged = data.data() != self.state.data.data();
        // Arriving on the Hunting Target's map (with Candidates, showing it), or leaving it
        // after.
        if let (true, Some(map)) = (moved, &map) {
            self.search.arrived(&map.id);
        }
        self.state.map = map;
        self.state.data = data;
        if moved || paged {
            self.sync_search();
        }
        self.state.candidates = self.maps.held().and_then(|maps| self.shown_map.candidates(maps));
        self.sync_pictured();
        self.state.credit = self.state.map.as_ref().map(|_| OSMS_AND_MEOWDB_CREDIT);
    }

    /// What shows OSMS's pictures, with those in so far: the Map Marks, the Map Preview (its
    /// portals on the route to the Hunting Target or Destination marked), the map's mobs'
    /// Mob Cards, Search's matches and the Hunting Target's mob.
    fn sync_pictured(&mut self) {
        let set = self.maps.held();
        self.state.map_marks = set.map(|maps| map_mark::table(maps, &self.pictures)).unwrap_or_default();
        let next_map = self.state.search.next_map_by_portal();
        self.state.map_preview =
            set.and_then(|maps| self.shown_map.preview(maps, &self.pictures)).map(|preview| preview.on_route_to(next_map));
        let mobs = self.state.data.data().map_or(&[][..], |data| &data.mobs);
        self.state.mob_cards = match set {
            Some(maps) => mob_card::cards(mobs, maps, &self.pictures),
            None => Vec::new(),
        };
        if let Some(maps) = set {
            let pictures = &self.pictures;
            self.state.search.fill_pictures(maps, |path| pictures.url(path).map(String::from));
        }
    }

    /// Ask for the pictures the panel wants, one at a time: those of Search's matches
    /// first, then the Item Card's (its item's, or those of the items it lists to pick
    /// from), as the player is looking at them; then every Map Mark, few and small and shown
    /// beside every map's name; then the Map Preview's, then the map's mobs' animations,
    /// then the Hunting Target mob's.
    fn poll_pictures(&mut self, at_ms: u64) {
        let mut wanted = self.state.search.pictures(self.maps.held());
        wanted.extend(self.item_cards.pictures());
        if let Some(maps) = self.maps.held() {
            let preview = self.state.map_preview.as_ref().map(|preview| map_preview::picture_path(&preview.picture));
            let mobs = self.state.data.data().map_or(&[][..], |data| &data.mobs);
            let hunted = self.state.search.hunting.as_ref().and_then(|hunting| maps.animation(&hunting.mob.id));
            wanted.extend(
                map_mark::pictures(maps)
                    .chain(preview)
                    .chain(mob_card::animations(mobs, maps))
                    .chain(hunted.map(String::from)),
            );
        }
        self.pictures.poll(&wanted, &mut self.source, at_ms);
        if self.maps.held().is_some() {
            self.sync_pictured();
        }
    }

    /// The game window moved, resized, appeared or went away (`None`: not found, or
    /// minimised).
    pub fn on_game_window(&mut self, game: Option<GameWindow>) -> &PanelState {
        self.record(Event::GameWindow { game: game.clone() });
        if let Some(game) = &game {
            self.panel.screens = game.screens.clone();
        }
        self.panel.game = game;
        self.sync_panel()
    }

    /// The overlay's expanded and collapsed sizes, in physical px for the current display.
    pub fn set_panel_sizes(&mut self, sizes: PanelSizes) -> &PanelState {
        self.record(Event::PanelSizes { sizes });
        self.panel.sizes = sizes;
        self.sync_panel()
    }

    /// The Item Lookup alert's size, in physical px for the current display.
    pub fn set_item_lookup_size(&mut self, size: Size) -> &PanelState {
        self.record(Event::ItemLookupSize { size });
        self.item_lookups.size = size;
        self.sync_panel()
    }

    /// The Item Card's size, in physical px for the current display.
    pub fn set_item_card_size(&mut self, size: Size) -> &PanelState {
        self.record(Event::ItemCardSize { size });
        self.item_cards.size = size;
        self.sync_panel()
    }

    /// The Deal Badges' window's width, in physical px for the current display (its height
    /// is the shop's list's).
    pub fn set_deal_badges_size(&mut self, size: Size) -> &PanelState {
        self.record(Event::DealBadgesSize { size });
        self.deal_finder.size = size;
        self.sync_panel()
    }

    /// The Grind Tracker window's size, in physical px for the current display.
    pub fn set_grind_tracker_size(&mut self, size: Size) -> &PanelState {
        self.record(Event::GrindTrackerSize { size });
        self.grind_tracker.size = size;
        self.sync_panel()
    }

    /// The size the Personal Bests window's page lays it out at, in physical px for the
    /// current display.
    pub fn set_personal_bests_size(&mut self, size: Size) -> &PanelState {
        self.record(Event::PersonalBestsSize { size });
        self.bests_window.size = size;
        self.sync_panel()
    }

    /// Apply something the player did.
    pub fn act(&mut self, action: Action) -> &PanelState {
        self.record(Event::Act { action });
        self.apply(action)
    }

    fn apply(&mut self, action: Action) -> &PanelState {
        match action {
            Action::ToggleExpanded => {
                self.panel.expanded = !self.panel.expanded;
                // The query box goes with the panel. So does the settings view.
                if !self.panel.expanded {
                    self.search.close();
                    self.sync_search();
                    self.hotkey_settings.close();
                }
            }
            Action::PanelMoved { x, y } => self.panel.moved_to(x, y),
            Action::PanelResized { width } => self.panel.resized_to(width),
            Action::CyclePlacement => self.panel.cycle_placement(),
            Action::ChooseCandidate => {
                self.shown_map.choose_next();
                self.sync_shown_map();
            }
            Action::FinishSetup => self.setup.finish(),
            Action::RequestSetup => self.setup.request(),
            Action::LaunchedAgain => {
                if self.state.setup.window_open {
                    // Setup shows already.
                } else if self.panel.game.is_some() {
                    // It shows with the game, even if the app the player started it from
                    // is in front for now.
                    self.panel.expanded = true;
                } else {
                    self.setup.request();
                }
            }
            Action::CancelSetup => {
                self.state.quit |= self.setup.needed() == Some(SetupReason::FirstRun);
                self.setup.cancel();
            }
            Action::ResetSession => self.session.reset_by_player(),
            Action::UndoReset => self.session.undo_reset(),
            Action::RemoveBest => {
                if let Some(map) = bests_map(&self.state) {
                    self.session.remove_best(map);
                }
                // With no best left, there is nothing for the Personal Bests window to show.
                self.bests_window.open &= self.session.bests().any();
            }
            Action::ToggleGrindTracker => self.grind_tracker.open = !self.grind_tracker.open,
            Action::CloseGrindTracker => self.grind_tracker.open = false,
            Action::GrindTrackerMoved { x, y } => {
                if let (Some(game), Some(panel)) = (&self.panel.game, self.panel.placement()) {
                    self.grind_tracker.moved_to(x, y, game, panel, self.panel.mode, &self.panel.avoid);
                }
            }
            Action::TogglePersonalBests => {
                self.bests_window.open = !self.bests_window.open && self.session.bests().any();
            }
            Action::ClosePersonalBests => self.bests_window.open = false,
            Action::ItemCardMoved { x, y } => {
                if let Some(game) = &self.panel.game {
                    self.item_cards.moved_to(x, y, game);
                }
            }
            Action::EditCardMarket => {
                if let Some(first) = self.market_prices.lines().first() {
                    self.item_cards.edit(Editing::Price { line: first.id.clone(), not_a_price: None });
                }
            }
            Action::CancelCardMarket => self.item_cards.close_market(),
            Action::AddCardMarketLine => self.item_cards.edit(Editing::Name { line: None, problem: None }),
            Action::AskRemoveCardMarketLine => {
                if let Some(Editing::Name { line: Some(line), .. }) = self.item_cards.editing() {
                    self.item_cards.edit(Editing::Remove { line: line.clone() });
                }
            }
            Action::RemoveCardMarketLine => {
                if let Some(Editing::Remove { line }) = self.item_cards.editing() {
                    self.market_prices.remove(line);
                    self.item_cards.close_market();
                }
            }
            Action::PersonalBestsMoved { x, y } => {
                if let Some(game) = &self.panel.game {
                    self.bests_window.moved_to(x, y, game, &self.panel.avoid);
                }
            }
            Action::OpenSearch => {
                // The search view is on the expanded panel, in the settings view's place.
                self.panel.expanded = true;
                self.hotkey_settings.close();
                self.search.open();
                self.sync_search();
            }
            Action::ReopenMapList => {
                self.panel.expanded = true;
                self.hotkey_settings.close();
                self.search.reopen();
                self.sync_search();
            }
            Action::SearchPrevious | Action::SearchNext => {
                let by = if action == Action::SearchNext { 1 } else { -1 };
                self.search.move_highlight(by, &searched(&self.maps, &self.items, &self.state, &self.settings));
                self.sync_search();
            }
            Action::SearchEnter => {
                let picked = self.search.enter(&searched(&self.maps, &self.items, &self.state, &self.settings));
                self.sync_search();
                if let Some(ids) = picked {
                    self.show_card(ids);
                }
            }
            Action::SearchEscape => {
                self.search.escape();
                self.sync_search();
            }
            Action::ClearHuntingTarget => {
                self.search.clear();
                self.sync_search();
            }
            // Settings, so they count whether or not the search view is open.
            Action::ToggleTaxis => {
                self.settings.travel.taxis = !self.settings.travel.taxis;
                self.sync_search();
            }
            Action::ToggleTeleportScrolls => {
                self.settings.travel.teleport_scrolls = !self.settings.travel.teleport_scrolls;
                self.sync_search();
            }
            Action::ToggleSection(section) => {
                self.settings.collapsed_sections = self.settings.collapsed_sections.toggled(section);
            }
            Action::OpenSettings => {
                // On the expanded panel, in the search view's place.
                self.panel.expanded = true;
                self.search.close();
                self.sync_search();
                self.hotkey_settings.open();
            }
            Action::CloseSettings => {
                self.hotkey_settings.close();
                self.refreshes.forget();
            }
            Action::CheckForUpdates => self.updater.ask(),
            Action::SetCheckForUpdates { on } => self.settings.check_for_updates = on,
            Action::UpdateNow => self.updater.update_now(),
            Action::UpdateWhenIQuit => self.updater.when_i_quit(),
            Action::UpdateLater => self.updater.later(),
            Action::Quit => self.state.quit |= self.updater.quit(),
            Action::RefreshGameData => {
                if self.refreshes.start() {
                    // Asked for at the next tick, unless a fetch is under way: it counts.
                    self.maps.ask_now();
                    self.items.ask_now();
                }
            }
            Action::BeginRebind(hotkey) => self.hotkey_settings.begin(hotkey),
            Action::CancelRebind => self.hotkey_settings.cancel(),
            Action::TurnOffHotkey => {
                if self.hotkey_settings.turn_off(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
            Action::SwapHotkeys => {
                if self.hotkey_settings.swap(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
            Action::ResetHotkeys => {
                if self.hotkey_settings.restore(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
            Action::UndoResetHotkeys => {
                if self.hotkey_settings.undo_restore(&mut self.settings.hotkeys) {
                    self.forget_unbound_hotkeys();
                }
            }
            Action::ToggleDealFinder => self.deal_finder.toggle(),
            Action::SetDealMargin { percent } => self.settings.deal_margin_percent = percent.min(MAX_DEAL_MARGIN),
            Action::SetGrindTracker { on } => self.settings.grind_tracker_on = on,
            Action::OpenDealItem { row } => self.open_deal_item(row),
        }
        self.sync_panel()
    }

    /// The player pressed a key on the settings view: while a hotkey waits for its new
    /// combination, that may be it, or Backspace or Delete to turn it off (see
    /// [`SettingsView`]). Saved with the settings, and registered in place of the old one
    /// once the hotkeys are live again.
    pub fn rebind_key(&mut self, press: KeyPress) -> &PanelState {
        self.record(Event::RebindKey { press: press.clone() });
        if self.hotkey_settings.press(&press, &mut self.settings.hotkeys) {
            self.forget_unbound_hotkeys();
        }
        self.sync_panel()
    }

    /// The combinations no hotkey has any more aren't unavailable: the shell reports on
    /// the new ones when it registers them.
    fn forget_unbound_hotkeys(&mut self) {
        let hotkeys = &self.settings.hotkeys;
        let bound = |combination: &String| hotkeys.all().iter().any(|hotkey| same_hotkey(hotkey, combination));
        self.state.unavailable_hotkeys.retain(bound);
        self.state.invalid_hotkeys.retain(bound);
    }

    /// The player typed in the search view: `query` is everything in the query box.
    pub fn search_query(&mut self, query: &str) -> &PanelState {
        self.record(Event::SearchQuery { query: query.to_string() });
        self.search.type_query(query);
        self.sync_search();
        self.sync_panel()
    }

    /// The player clicked the mob with this ID in the search view's matches, or the map
    /// with this ID in the mob's map list. Like Enter on it.
    pub fn search_pick(&mut self, id: &str) -> &PanelState {
        self.record(Event::SearchPick { id: id.to_string() });
        self.search.pick(id, &searched(&self.maps, &self.items, &self.state, &self.settings));
        self.sync_search();
        self.sync_panel()
    }

    /// The player clicked the map with this ID in the search view's matches: it becomes the
    /// Destination. Like Enter on it.
    pub fn search_pick_destination(&mut self, id: &str) -> &PanelState {
        self.record(Event::SearchPickDestination { id: id.to_string() });
        self.search.pick_destination(id, &searched(&self.maps, &self.items, &self.state, &self.settings));
        self.sync_search();
        self.sync_panel()
    }

    /// The player clicked the item with this ID (8 digits) in the search view's matches:
    /// its card shows, beside the panel. Like Enter on it.
    pub fn search_pick_item(&mut self, id: &str) -> &PanelState {
        self.record(Event::SearchPickItem { id: id.to_string() });
        let picked = self.search.pick_item(id, &searched(&self.maps, &self.items, &self.state, &self.settings));
        self.sync_search();
        if let Some(ids) = picked {
            self.show_card(ids);
        }
        self.sync_panel()
    }

    /// The card of the item listing `row`'s Deal Badge is about shows beside the badge; the
    /// alert has nothing more to say. Nothing changes for a badge not showing, or one about
    /// no item ("?").
    fn open_deal_item(&mut self, row: u32) {
        let Some(DealBadges { placement, badges }) = &self.state.deal_badges else { return };
        let (Some(badge), Some(ids), Some(shop)) =
            (badges.iter().find(|badge| badge.row == row), self.deal_finder.row_items(row), self.deal_finder.shop_window())
        else {
            return;
        };
        let at = ScreenRect { y: placement.y + badge.top as i32, height: badge.height, ..*placement };
        self.item_cards.show_beside_badge(ids, at, shop);
        self.item_lookups.clear();
    }

    /// The card of items `ids`, picked from Search, shows beside the panel; the alert has
    /// nothing more to say.
    fn show_card(&mut self, ids: Vec<u32>) {
        self.item_cards.show(ids);
        self.item_lookups.clear();
    }

    /// Bring the panel's Search up to date with the player's inputs, OSMS's file set and
    /// item files, the map shown and the travel toggles; and with it what follows from the
    /// Hunting Target or Destination: the pictures, and the Map Preview's portals on the
    /// route.
    fn sync_search(&mut self) {
        let search = self.search.state(&searched(&self.maps, &self.items, &self.state, &self.settings));
        self.state.search = search;
        self.sync_pictured();
    }

    /// The player pressed one of the global hotkeys (an accelerator string, as listed
    /// in the panel state's `hotkeys`). Returns what the shell must do at once, if
    /// anything: open a page, or bring the overlay with the search view open back in
    /// front.
    pub fn on_hotkey(&mut self, pressed: &str) -> Option<Request> {
        self.record(Event::Hotkey { pressed: pressed.to_string() });
        let hotkeys = &self.settings.hotkeys;
        // The Item Card's and Item Lookup's first: they capture rather than press a control.
        let captures = [Hotkey::ItemCard, Hotkey::ItemLookup];
        let hotkey = captures.into_iter().chain(Hotkey::ALL).find(|&hotkey| same_hotkey(hotkeys.get(hotkey), pressed))?;
        // Only over a game window, and not while the last capture is being read.
        let reading = self.card_capturing || self.item_lookups.capturing();
        let control = match hotkey {
            Hotkey::ItemCard => {
                let capture = self.panel.game.is_some() && !reading;
                self.card_capturing |= capture;
                // The card showing hides for the capture, which it would otherwise be in.
                self.sync_panel();
                return capture.then_some(Request::CaptureItemTooltip);
            }
            Hotkey::ItemLookup => {
                let capture = self.panel.game.is_some() && !reading && self.item_lookups.start();
                return capture.then_some(Request::CaptureItemTooltip);
            }
            Hotkey::TogglePanel => PanelControl::TogglePanel,
            Hotkey::CyclePlacement => PanelControl::CyclePlacement,
            Hotkey::OpenMeowdbPage => PanelControl::OpenMeowdbPage,
            Hotkey::Setup => PanelControl::Setup,
            Hotkey::ChooseCandidate => PanelControl::ChooseCandidate,
            Hotkey::GrindTracker => PanelControl::ToggleGrindTracker,
            Hotkey::Search => PanelControl::Search,
            Hotkey::DealFinder => PanelControl::ToggleDealFinder,
        };
        self.apply_press(control)
    }

    /// The player used a control: its hotkey, its icon at the top of the panel, or its
    /// button or link in another of our windows. Returns what the shell must do at once,
    /// if anything.
    pub fn press(&mut self, control: PanelControl) -> Option<Request> {
        self.record(Event::Press { control });
        self.apply_press(control)
    }

    fn apply_press(&mut self, control: PanelControl) -> Option<Request> {
        let action = match control {
            PanelControl::TogglePanel => Action::ToggleExpanded,
            PanelControl::CyclePlacement => Action::CyclePlacement,
            PanelControl::Setup => Action::RequestSetup,
            PanelControl::ChooseCandidate => Action::ChooseCandidate,
            PanelControl::ToggleGrindTracker => Action::ToggleGrindTracker,
            PanelControl::ResetSession => Action::ResetSession,
            PanelControl::UndoReset => Action::UndoReset,
            PanelControl::RemoveBest => Action::RemoveBest,
            PanelControl::CloseGrindTracker => Action::CloseGrindTracker,
            PanelControl::TogglePersonalBests => Action::TogglePersonalBests,
            PanelControl::ClosePersonalBests => Action::ClosePersonalBests,
            PanelControl::ToggleDealFinder => Action::ToggleDealFinder,
            // An open search view stays as it is; the shell brings the overlay in front on
            // opening it.
            PanelControl::Search if self.search.is_open() => return Some(Request::RaiseOverlay),
            PanelControl::Search => Action::OpenSearch,
            PanelControl::Settings => Action::OpenSettings,
            PanelControl::OpenMeowdbPage => {
                return self.state.map.as_ref().map(|map| Request::OpenInBrowser { url: map.url.clone() });
            }
            PanelControl::OpenSite => return Some(Request::OpenInBrowser { url: SITE_URL.into() }),
            PanelControl::OpenTitleHelp => return Some(Request::OpenInBrowser { url: TITLE_HELP_URL.into() }),
            PanelControl::OpenGithub => return Some(Request::OpenInBrowser { url: GITHUB_URL.into() }),
            PanelControl::OpenDiscord => return Some(Request::OpenInBrowser { url: DISCORD_URL.into() }),
            PanelControl::OpenRelease => return self.open_release(),
            PanelControl::Update => {
                match self.updater.choose(self.session_running()) {
                    Chosen::Shown => {
                        self.setup.request();
                        self.sync_panel();
                    }
                    Chosen::ReleasePage => return self.open_release(),
                    Chosen::Nothing => {}
                }
                return None;
            }
        };
        self.apply(action);
        None
    }

    /// The player removed one of `map`'s Visits in the Personal Bests window, one that came
    /// from a bug, named by whose it is (`character`, `None` for the Unassigned bests) and
    /// by its best as the window lists it (any map's, not only the one they are on). The
    /// Visits under it move up, the Best row follows if it is of `map`, and with no best
    /// left anywhere the window closes. There is no undo. The Visit under way's best can be
    /// named too: only the play since counts towards it again. A Visit no longer there
    /// changes nothing.
    pub fn remove_visit(&mut self, character: Option<&str>, map: &str, visit: &VisitBest) -> &PanelState {
        let event = Event::RemoveVisit { map: map.to_string(), visit: visit.clone(), character: character.map(String::from) };
        self.record(event);
        self.session.remove_visit(character, map, visit);
        // With no best left, there is nothing for the Personal Bests window to show.
        self.bests_window.open &= self.session.bests().any();
        self.sync_panel()
    }

    /// The player moved a Visit on `map` of the Character named `from` (`None`: an Unassigned
    /// one), named by its best as the window lists it, to the Character named `to`, after
    /// confirming: it joins their Visits on the map, as any Visit does, and a shown
    /// Character's set goes up again. The Visit under way's best, a Visit no longer there, or
    /// one moved to the Character it is with, changes nothing.
    pub fn move_visit(&mut self, from: Option<&str>, map: &str, visit: &VisitBest, to: &str) -> &PanelState {
        let event = Event::MoveVisit { map: map.to_string(), visit: visit.clone(), to: to.to_string(), from: from.map(String::from) };
        self.record(event);
        self.session.move_visit(from, map, visit, to);
        self.sync_panel()
    }

    /// The player gave the Character named `to` every Unassigned Visit in the Personal Bests
    /// window, after confirming: each joins their Visits on its map, as any Visit does, as if
    /// moved one by one.
    pub fn assign_unassigned(&mut self, to: &str) -> &PanelState {
        self.record(Event::AssignUnassigned { to: to.to_string() });
        self.session.assign_unassigned(to);
        self.sync_panel()
    }

    /// The player removed the Character named `name` in the Personal Bests window, after
    /// confirming: every best in their history goes, with no undo, and with no best left
    /// anywhere the window closes. The Visit under way's best stays: it joins the Visit
    /// History when the Visit ends. A Character playing stays one of the window's choices.
    pub fn remove_character(&mut self, name: &str) -> &PanelState {
        self.record(Event::RemoveCharacter { name: name.to_string() });
        self.session.remove_character(name);
        // Nor does the site keep theirs.
        self.leaderboards.show(name, false);
        self.bests_window.open &= self.session.bests().any();
        self.sync_panel()
    }

    /// The player merged the Character named `from` into the one named `into` in the
    /// Personal Bests window, after confirming: a name the game changed shows as another
    /// Character. Each of `from`'s Visits joins `into`'s on its map as any Visit does, so
    /// the best in each Level Band stays, and `from` is gone.
    pub fn merge_characters(&mut self, from: &str, into: &str) -> &PanelState {
        self.record(Event::MergeCharacters { from: from.to_string(), into: into.to_string() });
        self.session.merge_characters(from, into);
        // `from` has no bests left: the site keeps none of theirs either.
        if from != into {
            self.leaderboards.show(from, false);
        }
        self.sync_panel()
    }

    /// The player turned "Share" on (`on`) for the Character named `name` in the Personal Bests
    /// window, or off. On, their entries go up at the next tick, and again whenever they change;
    /// off, they are deleted from the site, and stay off. A Character met for the first time with
    /// an entry starts on without this.
    pub fn show_on_leaderboards(&mut self, name: &str, on: bool) -> &PanelState {
        self.record(Event::ShowOnLeaderboards { name: name.to_string(), on });
        self.leaderboards.show(name, on);
        self.sync_panel()
    }

    /// The player sent the Character named `name` to the Map Leaderboards again from the
    /// Personal Bests window, where they stood with a problem: an upload tried again, a name
    /// another key held, or uploads held since the bests file didn't read (having confirmed,
    /// since what is here then replaces what the site holds). It goes up at the next tick.
    pub fn send_to_leaderboards_again(&mut self, name: &str) -> &PanelState {
        self.record(Event::SendToLeaderboardsAgain { name: name.to_string() });
        self.leaderboards.send_again(name);
        self.sync_panel()
    }

    /// The shell made upload `id` of [`PanelState::leaderboard_uploads`], and the Map
    /// Leaderboards answered `answer` (or it couldn't be made: [`LeaderboardAnswer::Failed`]).
    /// An answer to an upload no longer under way changes nothing.
    pub fn on_leaderboard_answer(&mut self, id: u64, answer: LeaderboardAnswer) -> &PanelState {
        let name = self.leaderboards.uploads().into_iter().find(|upload| upload.id == id).map(|upload| upload.name);
        self.record(Event::LeaderboardAnswer { id, name: name.unwrap_or_default(), answer: answer.clone() });
        self.leaderboards.answer(id, answer, self.wall_clock);
        self.sync_panel()
    }

    /// The shell ran check `id` of [`Updates::check`], and it came to `answer`. An answer to a
    /// check no longer under way changes nothing.
    pub fn on_update_answer(&mut self, id: u64, answer: UpdateAnswer) -> &PanelState {
        self.record(Event::UpdateAnswer { id, answer: answer.clone() });
        self.updater.answer(id, answer, self.wall_clock);
        self.sync_panel()
    }

    /// The shell ran download `id` of [`Updates::job`], and it came to `answer`. One no longer
    /// under way changes nothing.
    pub fn on_update_download(&mut self, id: u64, answer: DownloadAnswer) -> &PanelState {
        self.record(Event::UpdateDownload { id, answer: answer.clone() });
        self.state.quit |= self.updater.downloaded(id, answer);
        self.sync_panel()
    }

    /// The installer download `id` of [`Updates::job`] fetched couldn't be started, for
    /// `why`: this version stays. (Once it starts, Minerva ends, and the installer shows any
    /// error of its own.) One no longer under way changes nothing.
    pub fn on_update_install_failed(&mut self, id: u64, why: &str) -> &PanelState {
        self.record(Event::UpdateInstallFailed { id, why: why.to_string() });
        self.state.quit |= self.updater.install_failed(id);
        self.sync_panel()
    }

    /// What this build can do with a newer version: the shell tells it at launch, from how
    /// the build was installed. Until it does, it is [`Build::NotifyOnly`].
    pub fn on_build(&mut self, build: Build) -> &PanelState {
        self.record(Event::Build { build });
        self.updater.set_build(build);
        self.sync_panel()
    }

    /// The player clicked a map's name in the Personal Bests window: open its MeowDB page.
    /// `None` for a map the window doesn't list.
    pub fn open_bests_map_page(&self, id: &str) -> Option<Request> {
        let characters = &self.state.personal_bests.characters;
        let map = characters.iter().flat_map(|c| &c.maps).find(|map| map.id == id)?;
        Some(Request::OpenInBrowser { url: map.url.clone() })
    }

    /// The player clicked a map's Map Leaderboard link among the bests of the Character named
    /// `character` (`None`: the Unassigned ones) in the Personal Bests window: open it. `None`
    /// for a map the window doesn't list for them, or lists with no Map Leaderboard.
    pub fn open_bests_leaderboard(&self, character: Option<&str>, id: &str) -> Option<Request> {
        let characters = &self.state.personal_bests.characters;
        let maps = &characters.iter().find(|c| c.name.as_deref() == character)?.maps;
        let map = maps.iter().find(|map| map.id == id)?;
        Some(Request::OpenInBrowser { url: map.leaderboard_url.clone()? })
    }

    /// The player clicked a listed mob's name in the panel: open its MeowDB page. `None`
    /// for a mob the panel doesn't list.
    pub fn open_mob_page(&self, id: &str) -> Option<Request> {
        let card = self.state.mob_cards.iter().find(|card| card.id == id)?;
        Some(Request::OpenInBrowser { url: card.url.clone() })
    }

    /// Read an Item Lookup or Item Card capture ([`Request::CaptureItemTooltip`]) and update
    /// the panel state: the same as [`Minerva::on_item_tooltip_reading`] with what an
    /// [`ItemTooltipReader`] reads in it, here.
    pub fn on_item_capture(&mut self, capture: ItemCapture) -> &PanelState {
        let reading = self.item_reader.get_or_insert_with(ItemTooltipReader::bundled).read(&capture);
        self.on_item_capture_read(capture, reading)
    }

    /// As [`Minerva::on_item_tooltip_reading`], with the capture it was read from (by an
    /// [`ItemTooltipReader`], possibly on another thread), whose part around the pointer a
    /// Bug Report keeps (ADR 0009).
    pub fn on_item_capture_read(&mut self, capture: ItemCapture, reading: ItemTooltipReading) -> &PanelState {
        let crop = bug_report::item_tooltip_crop(&capture);
        self.record_input(Event::ItemTooltip { reading: Some(reading.clone()) }, None, crop);
        self.take_item_tooltip_reading(Some(reading))
    }

    /// Take what an Item Lookup or Item Card capture read as (read by an
    /// [`ItemTooltipReader`], possibly on another thread; `None` if the capture failed).
    /// For Item Lookup, look its name up on MeowDB: a page to open comes from
    /// [`Minerva::take_request`], at once if MeowDB's answer was at hand, else once it
    /// arrives. For the Item Card, show the card of the item OSMS's item files have by
    /// that name, at the pointer.
    pub fn on_item_tooltip_reading(&mut self, reading: Option<ItemTooltipReading>) -> &PanelState {
        self.record(Event::ItemTooltip { reading: reading.clone() });
        self.take_item_tooltip_reading(reading)
    }

    fn take_item_tooltip_reading(&mut self, reading: Option<ItemTooltipReading>) -> &PanelState {
        // Our windows show in the capture: one over the name would be read instead.
        let ours: Vec<(OurWindow, ScreenRect)> = [
            self.state.panel_showing().map(|at| (OurWindow::Panel, at)),
            self.state.grind_tracker.placement.map(|at| (OurWindow::GrindTracker, at)),
            self.state.personal_bests.placement.map(|at| (OurWindow::PersonalBests, at)),
            self.state.item_lookup_placement.map(|at| (OurWindow::Alert, at)),
        ]
        .into_iter()
        .flatten()
        .collect();
        if std::mem::take(&mut self.card_capturing) {
            let at_ms = reading.as_ref().map_or(0, |r| r.at_ms);
            match self.item_cards.read(reading, &ours, self.items.items()) {
                Pressed::Shown => self.item_lookups.clear(),
                Pressed::Closed => {}
                Pressed::Alert(outcome) => self.item_lookups.alert(outcome, at_ms),
            }
            return self.sync_panel();
        }
        let open = self.item_lookups.read(reading, &ours, &mut self.source);
        self.requests.extend(open.map(|url| Request::OpenInBrowser { url }));
        self.sync_panel()
    }

    /// Read a Deal Finder capture of [`PanelState::deal_finder_capture`] and update the panel
    /// state: the same as [`Minerva::on_player_shop_read`] with what a [`PlayerShopReader`]
    /// reads in it, here.
    pub fn on_player_shop_capture(&mut self, capture: ShopCapture) -> &PanelState {
        let reading = self.shop_reader.get_or_insert_with(PlayerShopReader::bundled).read(&capture);
        self.on_player_shop_read(capture, reading)
    }

    /// As [`Minerva::on_player_shop_reading`], with the capture it was read from (by a
    /// [`PlayerShopReader`], possibly on another thread), whose shop list a Bug Report keeps
    /// (ADR 0009).
    pub fn on_player_shop_read(&mut self, capture: ShopCapture, reading: ShopReading) -> &PanelState {
        // The list alone, and the Item Tooltip by the pointer, as their samples keep them.
        let crops = [
            bug_report::player_shop_crop(&capture, reading.shop.as_ref()),
            bug_report::shop_tooltip_crop(&capture, reading.hover.as_ref()),
        ];
        let event = Event::PlayerShop { at_ms: reading.at_ms, shop: reading.shop.clone(), same: false, hover: reading.hover.clone() };
        self.record_inputs(event, Some(reading.at_ms), crops.into_iter().flatten().collect());
        self.deal_finder.read(reading);
        self.sync_panel()
    }

    /// Take what a Deal Finder capture read as (read by a [`PlayerShopReader`], possibly on
    /// another thread): the Player Shop's listings, whose Deal Badges show beside them.
    pub fn on_player_shop_reading(&mut self, reading: ShopReading) -> &PanelState {
        let event = Event::PlayerShop { at_ms: reading.at_ms, shop: reading.shop.clone(), same: false, hover: reading.hover.clone() };
        self.record_input(event, Some(reading.at_ms), None);
        self.deal_finder.read(reading);
        self.sync_panel()
    }

    /// Read a capture of [`PanelState::status_bar_capture`] and update the panel state: the
    /// same as [`Minerva::on_status_bar_read`] with what a [`StatusBarReader`] reads in it,
    /// and an [`ExpBarReader`] reads of its EXP figures while [`PanelState::exp_figures`]
    /// asks for them, here.
    pub fn on_status_bar_capture(&mut self, capture: StatusBarCapture) -> &PanelState {
        let reading = self.status_bar_reader.get_or_insert_with(StatusBarReader::bundled).read(&capture);
        let exp = self.state.exp_figures.map(|figures| {
            let reader = self.exp_bar_reader.get_or_insert_with(ExpBarReader::bundled);
            reader.read_status_bar(&capture, reading.bar.as_ref(), figures)
        });
        self.on_status_bar_read(capture, reading, exp)
    }

    /// As [`Minerva::on_status_bar_reading`], with the capture it was read from (by a
    /// [`StatusBarReader`], possibly on another thread), whose level, job and name a Bug
    /// Report keeps (ADR 0009); then as [`Minerva::on_exp_bar_reading`] with what its EXP
    /// figures read as (by [`ExpBarReader::read_status_bar`] with
    /// [`PanelState::exp_figures`]), whose crop of them it keeps too. `exp` is `None` while
    /// the figures weren't asked for.
    pub fn on_status_bar_read(&mut self, capture: StatusBarCapture, reading: StatusBarReading, exp: Option<ExpBarReading>) -> &PanelState {
        let crop = bug_report::status_bar_crop(&capture, reading.bar.as_ref());
        let event = Event::StatusBar { at_ms: reading.at_ms, bar: reading.bar.clone(), same: false };
        self.record_input(event, Some(reading.at_ms), crop);
        self.take_status_bar_reading(reading);
        let Some(exp) = exp else { return self.sync_panel() };
        let crop = exp.place.and_then(|place| exp_bar::figures_frame(&capture, place)).map(|frame| (Region::ExpBar, frame));
        let event = Event::ExpBar { at_ms: exp.at_ms, bar: exp.bar, place: exp.place };
        self.record_input(event, Some(exp.at_ms), crop);
        self.take_exp_bar_reading(exp)
    }

    /// Take what a Status Bar capture read as (read by a [`StatusBarReader`], possibly on
    /// another thread): the Character playing, their level and job. Another Character's
    /// name starts a new Session, and another level is the one the Session learns.
    pub fn on_status_bar_reading(&mut self, reading: StatusBarReading) -> &PanelState {
        let event = Event::StatusBar { at_ms: reading.at_ms, bar: reading.bar.clone(), same: false };
        self.record_input(event, Some(reading.at_ms), None);
        self.take_status_bar_reading(reading)
    }

    fn take_status_bar_reading(&mut self, reading: StatusBarReading) -> &PanelState {
        self.session.on_status_bar(reading.bar.as_ref(), reading.at_ms);
        if let (Some(bar), Some(game)) = (&reading.bar, &self.panel.game) {
            self.status_bar_area = Some((game.client, bar.area));
        }
        self.state.status_bar = reading.bar;
        self.sync_panel()
    }

    /// The player chose the Item Card's Market Line `line` (its ID) as the Deal Line, which
    /// the Deal Finder compares listings with. Saved with the market prices.
    pub fn choose_deal_line(&mut self, line: &str) -> &PanelState {
        self.record(Event::ChooseDealLine { line: line.to_string() });
        self.market_prices.choose_deal_line(line);
        self.sync_panel()
    }

    /// The player picked item `id` from those the Item Card lists as sharing the name read:
    /// its card shows.
    pub fn pick_card_item(&mut self, id: &str) -> &PanelState {
        self.record(Event::PickCardItem { id: id.to_string() });
        self.item_cards.pick(id);
        self.sync_panel()
    }

    /// The player clicked the Item Card's name, at `at_ms` (on the clock of frames and
    /// ticks): open the item's MeowDB page, through MeowDB's search as Item Lookup does.
    /// The page comes from [`Minerva::take_request`], and the Item Lookup alert says what
    /// came of it.
    pub fn open_card_meowdb_page(&mut self, at_ms: u64) -> &PanelState {
        self.record(Event::OpenCardMeowdbPage { at_ms });
        let Some(item) = self.items.items().ok().and_then(|items| self.item_cards.item(items)) else {
            return self.sync_panel();
        };
        let (name, kind) = (item.name.clone(), item.kind.clone());
        let open = self.item_lookups.look_up(name, kind, &mut self.source, at_ms);
        self.requests.extend(open.map(|url| Request::OpenInBrowser { url }));
        self.sync_panel()
    }

    /// The player clicked the MesoWatch button on the Item Card's market heading: open
    /// MesoWatch, where they look the item up (it has no page per item). `None` while no
    /// card shows.
    pub fn open_card_mesowatch(&self) -> Option<Request> {
        let Some(ShownItemCard { view: ItemCardView::Card(_), .. }) = &self.state.item_card else { return None };
        Some(Request::OpenInBrowser { url: MESOWATCH_URL.to_string() })
    }

    /// The player clicked the price on the Item Card's Market Line `line` (its ID), or its
    /// "Not set": its price field opens, in place of what the market section had open.
    pub fn edit_card_market_price(&mut self, line: &str) -> &PanelState {
        self.record(Event::EditCardMarketPrice { line: line.to_string() });
        if self.market_prices.line(line).is_some() {
            self.item_cards.edit(Editing::Price { line: line.to_string(), not_a_price: None });
        }
        self.sync_panel()
    }

    /// The player pressed Enter in the Item Card's open price field, holding `typed`: the
    /// price they keep for the item on that line, written as players write prices
    /// ("450,000", "450k", "1.2m"), dated by the wall clock. Nothing typed forgets the
    /// price. Either way the field closes; what isn't a price keeps it open, saying so.
    pub fn enter_card_market(&mut self, typed: &str) -> &PanelState {
        self.record(Event::EnterCardMarket { typed: typed.to_string() });
        if let (Some(Editing::Price { line, .. }), Some(id)) = (self.item_cards.editing().cloned(), self.item_cards.card_item()) {
            match market_prices::parse(typed) {
                Ok(mesos) => {
                    let price = mesos.map(|mesos| MarketPrice { mesos, set_at_unix_ms: self.wall_clock });
                    self.market_prices.set(&line, &id, price);
                    self.item_cards.close_market();
                }
                Err(()) => self.item_cards.edit(Editing::Price { line, not_a_price: Some(typed.to_string()) }),
            }
        }
        self.sync_panel()
    }

    /// The player clicked the name of the Item Card's Market Line `line` (its ID): its name
    /// field opens, to rename the line or remove it, in place of what the market section
    /// had open.
    pub fn edit_card_market_line(&mut self, line: &str) -> &PanelState {
        self.record(Event::EditCardMarketLine { line: line.to_string() });
        if self.market_prices.line(line).is_some() {
            self.item_cards.edit(Editing::Name { line: Some(line.to_string()), problem: None });
        }
        self.sync_panel()
    }

    /// The player pressed Enter in the Item Card's open line name field, holding `typed`:
    /// the line is renamed, or the line being added is added at the end, for every item.
    /// A name must have more than spaces, and no other line may have it in any case: the
    /// field stays open saying so.
    pub fn enter_card_market_line(&mut self, typed: &str) -> &PanelState {
        self.record(Event::EnterCardMarketLine { typed: typed.to_string() });
        if let Some(Editing::Name { line, .. }) = self.item_cards.editing().cloned() {
            let name = typed.trim();
            let problem = if name.is_empty() {
                Some(NameProblem::Empty)
            } else if self.market_prices.name_taken(name, line.as_deref()) {
                Some(NameProblem::Taken { name: name.to_string() })
            } else {
                None
            };
            match (problem, line) {
                (Some(problem), line) => self.item_cards.edit(Editing::Name { line, problem: Some(problem) }),
                (None, Some(line)) => {
                    self.market_prices.rename(&line, name);
                    self.item_cards.close_market();
                }
                (None, None) => {
                    self.market_prices.add(name);
                    self.item_cards.close_market();
                }
            }
        }
        self.sync_panel()
    }

    /// Pick up the market prices the player kept at earlier launches
    /// ([`SavedMarketPrices::default`] when none were). The shell restores them at startup.
    pub fn restore_market_prices(&mut self, prices: SavedMarketPrices) -> &PanelState {
        self.record(Event::RestoreMarketPrices { prices: prices.clone() });
        self.market_prices.restore(prices);
        self.sync_panel()
    }

    /// Take in the prices a tool of the player's own left for their Market Lines (ADR 0013),
    /// the newest winning over the prices held. The shell hands each over as it finds it,
    /// whether a card shows or not.
    pub fn import_market_prices(&mut self, import: MarketImport) -> &PanelState {
        self.record(Event::ImportMarketPrices { import: import.clone() });
        self.market_prices.import(import);
        self.sync_panel()
    }

    /// The market prices the player keeps, for the shell to save.
    pub fn market_prices(&self) -> SavedMarketPrices {
        self.market_prices.saved().clone()
    }

    /// Changes whenever what [`Minerva::market_prices`] holds does, so the shell saves them
    /// only then.
    pub fn market_prices_revision(&self) -> u64 {
        self.market_prices.revision()
    }

    /// The player clicked the Item Card's ×: close it.
    pub fn close_item_card(&mut self) -> &PanelState {
        self.record(Event::CloseItemCard {});
        self.item_cards.close();
        self.sync_panel()
    }

    /// The player picked one of the items an Item Lookup found sharing the name read:
    /// open its MeowDB page. `None` for an item the panel doesn't list.
    pub fn pick_item(&mut self, id: &str) -> Option<Request> {
        self.record(Event::PickItem { id: id.to_string() });
        let url = self.item_lookups.pick(id)?;
        self.sync_panel();
        Some(Request::OpenInBrowser { url })
    }

    /// What the shell must do that came of time passing or of a reading, rather than of
    /// the player's press: an Item Lookup's page, once MeowDB has answered. The shell
    /// takes them after each call, until there are none.
    pub fn take_request(&mut self) -> Option<Request> {
        self.requests.pop_front()
    }

    /// Another window came to the front. The hotkeys are live, and the overlay, the Grind
    /// Tracker's window and the Item Lookup alert show (with the game), while the game or
    /// one of our windows is in front, and nowhere else (see [`Layer`]).
    pub fn on_foreground(&mut self, foreground: Foreground) -> &PanelState {
        self.record(Event::Foreground { foreground });
        self.panel.foreground = Some(foreground);
        self.sync_panel()
    }

    /// These hotkeys couldn't be registered: `taken` because another app has them,
    /// `invalid` because they don't name a key combination.
    pub fn hotkeys_unavailable(&mut self, taken: Vec<String>, invalid: Vec<String>) -> &PanelState {
        self.record(Event::HotkeysUnavailable { taken: taken.clone(), invalid: invalid.clone() });
        self.state.unavailable_hotkeys = taken.into_iter().chain(invalid.iter().cloned()).collect();
        self.state.invalid_hotkeys = invalid;
        self.sync_panel()
    }

    /// Pick up settings saved by an earlier session. Hotkeys from before version 4
    /// (version 1's Ctrl+Shift letters, versions 2 and 3's Win+Alt) are replaced by the
    /// defaults, version 4's Item Lookup key goes to the Item Card, and the Deal Finder's
    /// default is off in a file from before it that gives it to another hotkey. The EXP Bar
    /// Region older files hold is ignored (ADR 0017).
    pub fn restore(&mut self, mut settings: Settings) -> &PanelState {
        self.record(Event::Restore { settings: Box::new(settings.clone()) });
        // Version 4 brought the Ctrl+Shift defaults, and version 5 the Item Card; a later
        // version keeps the hotkeys.
        match settings.version {
            ..4 => settings.hotkeys = Hotkeys::default(),
            4 => {
                settings.hotkeys.give_item_lookup_to_card();
                settings.hotkeys.free_deal_finder();
            }
            5 => settings.hotkeys.free_deal_finder(),
            _ => {}
        }
        settings.deal_margin_percent = settings.deal_margin_percent.min(MAX_DEAL_MARGIN);
        settings.version = settings::VERSION;
        self.panel.expanded = settings.expanded;
        self.panel.inside_offset = settings.inside_offset;
        self.panel.mode = settings.placement_mode;
        self.panel.detached_at = settings.detached_at;
        self.panel.resized_to(settings.panel_width.unwrap_or(DEFAULT_PANEL_WIDTH));
        self.setup.done = settings.setup_done;
        self.grind_tracker.open = settings.grind_tracker_open;
        self.grind_tracker.at = settings.grind_tracker_at;
        self.bests_window.at = settings.personal_bests_at;
        self.leaderboards.restore(settings.map_leaderboards.clone(), settings.map_leaderboards_met.clone());
        self.settings = settings;
        // The travel toggles change Distances.
        self.sync_search();
        self.sync_panel()
    }

    /// The player picked the game window by hand: look for windows with this title
    /// from now on.
    pub fn pick_game_window(&mut self, title: String) -> &PanelState {
        self.record(Event::PickGameWindow { title: title.clone() });
        self.settings.game_window_title = Some(title);
        self.sync_panel()
    }

    /// The settings to save: whatever the player has changed since [`Minerva::restore`].
    pub fn settings(&self) -> Settings {
        Settings {
            expanded: self.panel.expanded,
            inside_offset: self.panel.inside_offset,
            placement_mode: self.panel.mode,
            detached_at: self.panel.detached_at,
            panel_width: (self.panel.width != DEFAULT_PANEL_WIDTH).then_some(self.panel.width),
            setup_done: self.setup.done,
            grind_tracker_open: self.grind_tracker.open,
            grind_tracker_at: self.grind_tracker.at,
            personal_bests_at: self.bests_window.at,
            map_leaderboards: self.leaderboards.saved(),
            map_leaderboards_met: self.leaderboards.met(),
            ..self.settings.clone()
        }
    }

    /// The wall clock now, in ms since the Unix epoch: what tells how old a saved copy of
    /// OSMS's file set is, and dates Personal Bests. The shell tells it as time passes.
    pub fn on_wall_clock(&mut self, now_unix_ms: u64) {
        if self.recorder.wall_clock(now_unix_ms) {
            self.record(Event::WallClock { unix_ms: now_unix_ms });
        }
        self.wall_clock = Some(now_unix_ms);
    }

    /// Pick up the Personal Bests an earlier launch saved ([`SavedBests::default`] when
    /// none were). The shell restores them at startup, before any reading, after
    /// [`Minerva::restore`]: with settings from before version 7, every Character in them
    /// counts as met, so none of them starts shared on the Map Leaderboards.
    pub fn restore_bests(&mut self, bests: SavedBests) -> &PanelState {
        self.record(Event::RestoreBests { bests: bests.clone() });
        self.session.restore_bests(bests);
        self.leaderboards.bests_restored(self.session.bests().history());
        self.sync_panel()
    }

    /// The Personal Bests earlier launches saved were there but didn't read, so the shell
    /// restored none ([`Minerva::restore_bests`] with [`SavedBests::default`]). After
    /// [`Minerva::restore`]: every Character shown on the Map Leaderboards then has their uploads
    /// held, at this launch and the ones after, since sending what is here would take what the
    /// site holds of theirs off it. Turning one off and on in the Personal Bests window sends
    /// what is here then.
    pub fn bests_lost(&mut self) -> &PanelState {
        self.record(Event::BestsLost {});
        self.leaderboards.bests_lost();
        self.sync_panel()
    }

    /// Every map's Personal Bests, for the shell to save: the Visit under way's included,
    /// since closing Minerva ends it.
    pub fn bests(&self) -> SavedBests {
        self.session.bests().saved()
    }

    /// Changes whenever what [`Minerva::bests`] holds does, so the shell saves them only then.
    pub fn bests_revision(&self) -> u64 {
        self.session.bests().revision()
    }

    /// Time passed: check on fetches in flight, and let the Session pause. The shell calls
    /// this many times a second (`at_ms` on the same clock as frames), so nothing waits for
    /// the network.
    pub fn tick(&mut self, at_ms: u64) -> &PanelState {
        self.record_input(Event::Tick {}, Some(at_ms), None);
        self.session.tick(at_ms);
        // Uploads only on a tick: at launch the bests are back by the first.
        self.leaderboards.tick(at_ms, self.session.bests().history(), self.wall_clock);
        self.updater.tick(at_ms, self.settings.check_for_updates);
        self.load_osms(at_ms);
        if self.maps.held().is_some() {
            self.sync_shown_map();
        }
        self.poll_pictures(at_ms);
        // The Item Card's item files, on the map set's schedule ([`Minerva::load_osms`]).
        let Loaded { refresh, load } = self.items.load(&mut self.source, at_ms, self.wall_clock, osms_items::read);
        match load {
            Load::Hold(held) => {
                self.items.hold(held);
                // Search matches items once they are in.
                self.sync_search();
            }
            // Search says why they are missing.
            Load::Failed => self.sync_search(),
            Load::Nothing => {}
        }
        if let Some(refresh) = refresh {
            self.refreshes.items_fetched(refresh);
        }
        // An Item Lookup's search, which the player waits on.
        let open = self.item_lookups.poll(&mut self.source, at_ms);
        self.requests.extend(open.map(|url| Request::OpenInBrowser { url }));
        self.sync_panel()
    }

    /// OSMS's file set, as [`Loader::load`] keeps it: at startup the set this version saved
    /// at an earlier launch if it is under a day old, else one fetched afresh, and every day
    /// after, or when the player presses Refresh. A set that comes replaces the one held
    /// ([`Minerva::take_set`]); nothing held and none to be had shows in
    /// [`PanelState::recognition`].
    fn load_osms(&mut self, at_ms: u64) {
        let overrides = &self.portal_overrides;
        let read = |files: &osms::OsmsFiles| osms::read_with(files, overrides);
        let Loaded { refresh, load } = self.maps.load(&mut self.source, at_ms, self.wall_clock, read);
        if let Some(refresh) = refresh {
            self.refreshes.set_fetched(refresh);
        }
        if let Load::Hold(set) = load {
            self.take_set(set);
        }
    }

    /// Hold `set`, a file set just read, in place of the one held: search and Distance
    /// work over it at once. A new map list is matched against from then on, and the map
    /// the player is on stays theirs.
    fn take_set(&mut self, set: Held<MapSet>) {
        let entries = set.data.entries();
        if self.maps.held().is_none_or(|held| held.entries() != entries) {
            let matcher = Matcher::new(entries);
            if let Some(old) = &self.matcher {
                self.tracker.remap(|at| matcher.index_of(&old.map(at).id));
            }
            self.matcher = Some(matcher);
        }
        self.maps.hold(set);
        self.sync_search();
    }

    pub fn panel_state(&self) -> &PanelState {
        &self.state
    }

    /// The shell, or one of our windows (`from`), ran into an error that would otherwise
    /// go nowhere: a Bug Report keeps it.
    pub fn note_error(&mut self, from: &str, message: &str) {
        self.record(Event::Error { from: from.to_string(), message: message.to_string() });
    }

    /// A Bug Report of the run so far (ADR 0009), with what the shell knows of it.
    pub fn bug_report(&mut self, about: &ReportAbout) -> BugReport {
        self.mark_bug_report();
        let maps = self
            .recorder
            .maps_shown()
            .filter_map(|id| {
                let maps = self.maps.held()?;
                let entry = maps.entry(id)?;
                Some(MapFacts {
                    id: id.to_string(),
                    street_name: entry.street_name.clone(),
                    map_name: entry.map_name.clone(),
                    region: entry.region.clone(),
                    summary: maps.data(id).and_then(|data| data.summary),
                })
            })
            .collect();
        let game_client = self.panel.game.as_ref().map(|game| game.client);
        let press = Press { state: &self.state, game_client, maps, wall_clock: self.wall_clock };
        self.recorder.report(about, press)
    }

    /// Write down that a Bug Report was saved here.
    fn mark_bug_report(&mut self) {
        self.record(Event::BugReport {});
        self.close_input();
    }

    /// Write `event` down for a Bug Report, after what came of the input before it: an
    /// input at `at_ms` if it has a time, read from `frame` if it has one.
    fn record_input(&mut self, event: Event, at_ms: Option<u64>, frame: Option<(Region, Frame)>) {
        self.record_inputs(event, at_ms, frame.into_iter().collect());
    }

    /// As [`Minerva::record_input`], read from several frames.
    fn record_inputs(&mut self, event: Event, at_ms: Option<u64>, frames: Vec<(Region, Frame)>) {
        if self.recorder.takes(&event) {
            self.close_input();
            self.recorder.open(event, at_ms, frames);
        }
    }

    fn record(&mut self, event: Event) {
        self.record_input(event, None, None);
    }

    /// Write down what came of the input being taken.
    fn close_input(&mut self) {
        let answers = self.source.take_answers();
        let notes = self.session.take_notes();
        let bests_revision = self.session.bests().revision();
        self.recorder.close(Outcome { answers, notes, state: &self.state, bests_revision });
    }

    fn sync_panel(&mut self) -> &PanelState {
        // Whether titles can be snapped to real maps: by the file set held, or why none is.
        self.state.recognition = match (self.maps.held(), self.maps.failed()) {
            (Some(_), _) => match self.maps.saved_copy_from(self.wall_clock) {
                Some(saved_at_unix_ms) => Recognition::SavedCopy { saved_at_unix_ms },
                None => Recognition::Ready,
            },
            (None, None) => Recognition::Starting,
            (None, Some(SourceError::Unreachable(_))) => Recognition::Unavailable,
            (None, Some(SourceError::FormatChanged(_))) => Recognition::FormatChanged,
        };
        self.sync_setup();
        self.sync_status_bar();
        // Never over a captured region, nor the Status Bar's Character or its EXP figures: the
        // capture would read the panel.
        let client = self.panel.game.as_ref().map(|game| game.client);
        let status_bar = self.status_bar_area.filter(|&(at, _)| Some(at) == client).map(|(_, area)| area);
        let exp_figures = self.state.exp_figures.and_then(|figures| figures.last_found);
        self.panel.avoid = [self.title_area(), status_bar, exp_figures].into_iter().flatten().collect();
        self.state.expanded = self.panel.expanded;
        self.state.collapsed_sections = self.settings.collapsed_sections;
        self.state.game_found = self.panel.game.is_some();
        self.state.placement_mode = self.panel.mode;
        self.state.panel_width = self.panel.width;
        self.state.placement = self.panel.placement();
        self.state.layer = self.panel.layer();
        self.state.draggable = self.panel.draggable();
        self.sync_grind_tracker();
        self.sync_personal_bests();
        self.state.leaderboard_uploads = self.leaderboards.uploads();
        self.sync_item_lookup();
        self.sync_item_card();
        self.sync_deal_finder();
        self.state.our_windows = [
            self.state.panel_showing(),
            self.state.grind_tracker.placement,
            self.state.personal_bests.placement,
            self.state.item_lookup_placement,
            self.state.item_card_placement,
            self.state.deal_badges.as_ref().map(|badges| badges.placement),
        ]
        .into_iter()
        .flatten()
        .collect();
        self.state.hotkeys = self.settings.hotkeys.clone();
        let in_front = self.panel.foreground.is_some_and(|foreground| foreground != Foreground::OtherApp);
        self.state.hotkeys_live = in_front && !self.hotkey_settings.is_rebinding();
        self.state.settings_view = self.hotkey_settings.state(&self.settings.hotkeys);
        self.state.game_data = GameData {
            fetched_at_unix_ms: self.maps.saved_at_unix_ms(),
            refresh: self.refreshes.state(),
        };
        self.state.updates = self.updater.state(self.settings.check_for_updates);
        self.state.game_window_title =
            self.settings.game_window_title.clone().unwrap_or_else(|| GAME_WINDOW_TITLE.into());
        &self.state
    }

    /// The Item Lookup's alert, at the top centre of the game window while it has one,
    /// clear of the captured regions; only while the overlay can show (see [`Layer`]).
    fn sync_item_lookup(&mut self) {
        let line = self.item_lookups.line();
        let shows = self.panel.layer() != Layer::Hidden;
        self.state.item_lookup_placement = match (&line, &self.panel.game) {
            (Some(_), Some(game)) if shows => {
                Some(item_lookup::alert_placement(game, self.item_lookups.size, &self.panel.avoid))
            }
            _ => None,
        };
        self.state.item_lookup = line;
    }

    /// The Item Card, at the pointer while there is one; only while the overlay can show
    /// (see [`Layer`]), and not while the next press's capture is taken, which it would
    /// cover.
    fn sync_item_card(&mut self) {
        let pictures = &self.pictures;
        let saved_copy_from = self.items.saved_copy_from(self.wall_clock);
        self.state.item_card = self.item_cards.view(
            self.items.items().ok(),
            saved_copy_from,
            &self.market_prices,
            |path| pictures.url(path).map(String::from),
        );
        let shows = self.state.item_card.is_some() && self.panel.layer() != Layer::Hidden && !self.card_capturing;
        self.state.item_card_placement = match &self.panel.game {
            Some(game) if shows => {
                // Beside the panel, a card from Search keeps off our other windows there.
                let others: Vec<ScreenRect> =
                    [self.state.grind_tracker.placement, self.state.personal_bests.placement].into_iter().flatten().collect();
                self.item_cards.placement(game, &self.panel.avoid, self.state.panel_showing(), &others)
            }
            _ => None,
        };
        // Opened from a Deal Badge, it unfolds out of it, from the edge facing it.
        let unfold_from = self.item_cards.badge().zip(self.state.item_card_placement).map(|((badge, _), card)| {
            if card.x + (card.width as i32) / 2 < badge.x + (badge.width as i32) / 2 { Edge::Right } else { Edge::Left }
        });
        if let Some(card) = &mut self.state.item_card {
            card.unfold_from = unfold_from;
        }
    }

    /// The Deal Finder: the client area to capture while it is on, and the badges beside the
    /// Player Shop it last read, judged afresh against the Deal Line; only while the overlay
    /// can show (see [`Layer`]), and the badges not while an Item Card or Item Lookup press's
    /// capture is taken, which they would be in.
    fn sync_deal_finder(&mut self) {
        let shows = self.panel.layer() != Layer::Hidden;
        let game = self.panel.game.as_ref().filter(|_| shows);
        self.state.deal_finder_capture = game.filter(|_| self.deal_finder.on).map(|game| game.client);
        let capturing = self.card_capturing || self.item_lookups.capturing();
        let placement = game
            .filter(|_| !capturing)
            .and_then(|game| self.deal_finder.placement(game, &self.panel.avoid));
        let top = placement.map_or(0, |at| at.y);
        let items = self.items.items().ok();
        let opened = self.item_cards.badge().filter(|_| self.state.item_card.is_some());
        let badges = self.deal_finder.badges(items, &self.market_prices, self.settings.deal_margin_percent, top, opened);
        self.state.deal_badges = placement.map(|placement| DealBadges { placement, badges });
        self.state.deal_finder = DealFinderView {
            on: self.deal_finder.on,
            deal_line: self.market_prices.deal_line().map(|line| line.name.clone()),
            margin_percent: self.settings.deal_margin_percent,
            shop_found: self.deal_finder.shop().is_some(),
        };
    }

    /// The Status Bar's strip to capture, always while the overlay can show (see [`Layer`]):
    /// another app's pixels are never read. And the Character the Session is for.
    fn sync_status_bar(&mut self) {
        let shows = self.panel.layer() != Layer::Hidden;
        self.state.status_bar_capture = self.panel.game.as_ref().filter(|_| shows).map(|game| {
            let client = game.client;
            let height = STATUS_BAR_STRIP.min(client.height);
            ScreenRect { x: client.x, y: client.y + (client.height - height) as i32, width: client.width, height }
        });
        self.state.character = self.session.character().cloned();
    }

    /// Where the Minimap Title was last found, inside the game's client area, while the client
    /// area is where it was then.
    fn title_area(&self) -> Option<ScreenRect> {
        let client = self.panel.game.as_ref()?.client;
        let (at, found) = self.title_found.filter(|&(at, _)| at == client)?;
        let (x0, y0) = (found.x.max(at.x), found.y.max(at.y));
        let x1 = (found.x + found.width as i32).min(at.x + at.width as i32);
        let y1 = (found.y + found.height as i32).min(at.y + at.height as i32);
        (x1 > x0 && y1 > y0).then(|| ScreenRect { x: x0, y: y0, width: (x1 - x0) as u32, height: (y1 - y0) as u32 })
    }

    /// Where to capture for the Minimap Title (ADR 0021), while the overlay can show (see
    /// [`Layer`]): another app's pixels are never read. Where it was last found, or the whole
    /// client area while it is looked for.
    fn sync_title(&mut self) {
        let shows = self.panel.layer() != Layer::Hidden;
        let client = self.panel.game.as_ref().filter(|_| shows).map(|game| game.client);
        let found = self.title_area().filter(|_| shows && self.title_locked);
        if !shows {
            // No time behind another app counts as time the title wasn't found: none was
            // looked for.
            self.title_missing_since = None;
            self.title_unread_since = None;
            self.state.title_lost = false;
        }
        self.state.title_capture = found.or(client);
        self.state.title_searching = found.is_none() && client.is_some();
    }

    fn sync_setup(&mut self) {
        self.setup.follow_needed();
        self.state.setup = self.setup.state(self.state.map.clone());
        self.sync_title();
        let last_found = self.exp_figures_last_found();
        self.state.exp_figures = self.tracking_on().then_some(ExpFigures { last_found });
        if !self.tracking_on() {
            self.exp_bar = None;
        }
        self.state.exp_bar = self.exp_bar;
        // Off while the Grind Tracker is; the Session itself carries on.
        self.state.exp_tracking = if self.tracking_on() {
            let ceiling = self.state.data.solo_exp_ceiling();
            let tracking = self.session.tracking(ceiling, bests_map(&self.state), self.wall_clock);
            // Nowhere to read the figures: no Status Bar in the last capture, nor found before
            // at this size.
            let lost = self.state.status_bar.is_none() && last_found.is_none();
            let no_status_bar = tracking.state == TrackingState::Unreadable && lost;
            ExpTracking { no_status_bar, ..tracking }
        } else {
            ExpTracking::default()
        };
    }

    /// Where on screen the EXP figures were last read, while the game's client area is the
    /// size it was then.
    fn exp_figures_last_found(&self) -> Option<ScreenRect> {
        let client = self.panel.game.as_ref()?.client;
        let (size, at) = self.exp_figures_at?;
        let fits = (size.width, size.height) == (client.width, client.height);
        fits.then(|| ScreenRect { x: client.x + at.x as i32, y: client.y + at.y as i32, width: at.width, height: at.height })
    }

    /// The Grind Tracker's window shows while the player wants it and the Grind Tracker is
    /// on, but not while setup is open, and only while the overlay can show (see [`Layer`]).
    fn sync_grind_tracker(&mut self) {
        let shows = self.grind_tracker.open
            && self.tracking_on()
            && !self.state.setup.window_open
            && self.panel.layer() != Layer::Hidden;
        let placement = match (&self.panel.game, self.panel.placement()) {
            (Some(game), Some(panel)) if shows => {
                Some(self.grind_tracker.placement(game, panel, self.panel.mode, &self.panel.avoid))
            }
            _ => None,
        };
        self.state.grind_tracker = GrindTrackerWindow {
            on: self.settings.grind_tracker_on,
            open: self.grind_tracker.open,
            placement,
            personal_bests: self.session.bests().any(),
        };
    }

    /// The Personal Bests window shows while the player wants it, but not while setup is
    /// open, and only while the overlay can show (see [`Layer`]); EXP tracking needn't be
    /// on. Its rows are the bests as saved now, with what OSMS's file set held says of
    /// each map, and only while it is open.
    fn sync_personal_bests(&mut self) {
        let window = self.bests_window;
        let shows = window.open && !self.state.setup.window_open && self.panel.layer() != Layer::Hidden;
        let placement = match &self.panel.game {
            Some(game) if shows => Some(window.placement(game, &self.panel.avoid)),
            _ => None,
        };
        let characters = if window.open {
            let set = self.maps.held();
            let map = |id: &str| {
                let maps = set?;
                Some((maps.entry(id)?, maps.data(id)?))
            };
            let (saved, this_visit) = self.session.bests().standing();
            let this_visit = this_visit.as_ref().map(|(whose, map, at)| (whose.as_deref(), map.as_str(), *at));
            let playing = self.session.character();
            let current = (playing.map(|c| c.name.as_str()), playing.and_then(|c| c.level));
            let history = self.session.bests().history();
            let toggle = |name: &str| self.leaderboards.toggle(name, history, self.wall_clock);
            bests_window::characters(&saved, current, this_visit, map, certain_map(&self.state), toggle)
        } else {
            Vec::new()
        };
        self.state.personal_bests = PersonalBestsWindow { open: window.open, placement, characters };
    }

    /// Whether EXP tracking is on: the Grind Tracker is (the settings view's switch), and
    /// there is a game window to read the EXP figures in.
    fn tracking_on(&self) -> bool {
        self.settings.grind_tracker_on && self.panel.game.is_some()
    }

    /// A Session is running: the Grind Tracker measures one, which updating would end.
    fn session_running(&self) -> bool {
        self.tracking_on() && self.session.running()
    }

    /// Open the newer version's release page in the browser: nothing while none is known.
    fn open_release(&self) -> Option<Request> {
        self.updater.release_url().map(|url| Request::OpenInBrowser { url })
    }
}

/// What mob search works over: OSMS's file set, the map the panel shows, and the travel
/// the player has turned on.
fn searched<'a>(
    maps: &'a Loader<MapSet>,
    item_files: &'a Loader<ItemSet>,
    state: &'a PanelState,
    settings: &Settings,
) -> Searched<'a> {
    let items = item_files.items().map_err(|missing| match missing {
        ItemLookup::ItemsUnavailable => ItemsMissing::Unavailable,
        ItemLookup::ItemsFormatChanged => ItemsMissing::FormatChanged,
        _ => ItemsMissing::Loading,
    });
    Searched {
        set: maps.held(),
        items,
        here: state.map.as_ref().map(|map| map.id.as_str()),
        travel: settings.travel,
    }
}

/// The shown map, by ID, if Personal Bests are kept there: only where the map is
/// certain, and has a Map Summary (in a town, quest turn-ins would set one).
fn bests_map(state: &PanelState) -> Option<&str> {
    certain_map(state).filter(|_| state.data.solo_exp_ceiling().is_some())
}

/// The shown map, by ID, if the panel is certain the player is on it: no
/// Candidates waiting for a choice. One map has the title, the map the player came from
/// settled it, or the player chose it. Maps that share a title but would show the same
/// are certain too, as they show the same: the panel always shows the first of them, and
/// their bests are kept under its ID.
fn certain_map(state: &PanelState) -> Option<&str> {
    let certain = state.candidates.as_ref().is_none_or(|c| c.chosen);
    state.map.as_ref().filter(|_| certain).map(|m| m.id.as_str())
}

/// Whether two accelerator strings name the same keys ("Ctrl+Shift+C", "shift + ctrl + c").
/// An empty one, a hotkey turned off, names none.
pub(crate) fn same_hotkey(a: &str, b: &str) -> bool {
    let keys = |s: &str| {
        let mut keys: Vec<String> = s.split('+').map(|k| k.trim().to_lowercase()).collect();
        keys.sort();
        keys
    };
    !a.trim().is_empty() && keys(a) == keys(b)
}
