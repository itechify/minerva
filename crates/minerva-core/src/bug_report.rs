//! Bug Reports (ADR 0009): what the core took in over the run and what came of it, saved
//! as a `.zip` when the player asks, to go with a bug's issue.
//!
//! - The trace has a line for every input the core takes in (title and EXP bar readings,
//!   ticks, the player's actions, the game window...), in order, with what came of it:
//!   the source's answers, what the core made of it ([`Note`]s), and what the Grind
//!   Tracker and the panel's title showed after, where that changed. The core holds no
//!   clock, so feeding the inputs to a new core ([`replay`]) takes it through the same
//!   states, and says where it doesn't.
//! - To keep it small: a tick that changes nothing is left out, but for the last of a run
//!   of them, which the next line says came first (`ticked`); an input that only sets a
//!   value (the game window, a window's size, what is in front) is written only when the
//!   value changes; a title reading the same as the one before says so, not what it read;
//!   and the wall clock is written only when it strays from the capture clock.
//! - The trace reaches back to launch, but no further than [`TRACE_MS`]; the lines that
//!   set the run up (the settings and Personal Bests restored) are kept even then. It is
//!   held compressed.
//! - Crops, from the last [`RECENT_CROPS_MS`]: the EXP figures' where they changed; the
//!   Minimap Title's, the Status Bar's and the Player Shop's where their reading changed;
//!   the Item Tooltip's at each press, and by the pointer on a Player Shop's list where its
//!   name changed. For the whole run, those behind a reading the core found unusual
//!   ([`Note::is_unusual`]), that changed the map shown, or that found a part but couldn't
//!   read it (it may have been misread).
//! - Where a reader that looks for its place in a capture found nothing, one such capture from
//!   the last [`RECENT_CROPS_MS`], whole (#133): the Deal Finder's client area, the latest whose
//!   reader saw a near miss, else the latest; and the Status Bar's strip, the latest. It is held
//!   as captured, and made a PNG only when a report is saved.
//! - The text and the ordinary crops are at most [`MAX_TEXT_AND_CROPS_BYTES`]: crops are left
//!   out to keep it so, the oldest ordinary ones first, then the oldest kept for the run. The
//!   text never is. The whole captures come on top, while the report stays under
//!   [`MAX_REPORT_BYTES`]; one that won't fit is left out, and `summary.txt` says so.
//! - OSMS's file set isn't in a report (ADR 0008), only its fingerprint: a replay is given
//!   the set to replay with, and a set that has changed since replays differently. The
//!   same goes for the Item Card's OSMS item files (ADR 0010).

use crate::exp_bar::{ExpBar, ExpBarPlace, ExpBarReading};
use crate::hotkey_settings::KeyPress;
use crate::item_lookup::ItemHit;
use crate::item_lookup::ItemLookup;
use crate::item_tooltip::{ItemCapture, ItemTooltip, ItemTooltipReading};
use crate::discord_link::{DiscordLinkAnswer, DiscordLinkAsk, DiscordLinkRequest};
use crate::leaderboards::{LeaderboardAnswer, RejectedReason};
use crate::map_data::MapSummary;
use crate::map_index::{Fetched, MapDataSource, SourceError};
use crate::market_prices::{MarketImport, SavedMarketPrices};
use crate::osms::{OsmsFiles, SavedFiles};
use crate::osms_items::{ItemFiles, SavedItemFiles};
use crate::panel::{Action, Foreground, GameWindow, PanelSizes, ScreenPoint, ScreenRect, Size};
use crate::personal_best::{SavedBests, VisitBest};
use crate::player_shop::{ShopCapture, ShopHover, ShopList, ShopNearMiss, ShopReading};
use crate::recogniser::{TitleCapture, TitleLines, TitleReading};
use crate::session::{ExpTracking, TrackingState};
use crate::settings::Settings;
use crate::status_bar::{StatusBar, StatusBarCapture, StatusBarNearMiss, StatusBarReading};
use crate::updates::{Build, DownloadAnswer, UpdateAnswer, UpdateJob, UpdateStep};
use crate::{MINERVA_VERSION, Minerva, Frame, PanelControl, PanelState};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, VecDeque};
use std::io::{Cursor, Read, Write};
use std::task::Poll;

/// How far back the trace reaches: a day.
const TRACE_MS: u64 = 24 * 60 * 60 * 1000;
/// How long a crop that isn't kept for the run is kept.
pub const RECENT_CROPS_MS: u64 = 10 * 60_000;
/// The most the crops kept for the run take; beyond it the oldest go.
const KEPT_CROPS_BYTES: usize = 8 * 1024 * 1024;
/// The most a Bug Report's text and its ordinary crops take...
pub const MAX_TEXT_AND_CROPS_BYTES: usize = 5 * 1024 * 1024;
/// ...and the biggest it gets, with the captures where a reader found nothing: under 10 MB, in
/// MB of a million bytes, for a Discord DM from a free account.
pub const MAX_REPORT_BYTES: usize = 10_000_000;
/// Room left under each for what the crops left out change in the text.
const MARGIN_BYTES: usize = 4096;
/// The trace's text is compressed in chunks of about this much.
const CHUNK_BYTES: usize = 256 * 1024;
/// The wall clock is written down again when it strays this far from the capture clock.
const WALL_CLOCK_DRIFT_MS: u64 = 1000;
/// How many notes and errors `summary.txt` lists, the latest.
const NOTABLE_LINES: usize = 200;

/// The level's total EXP as the Session learned it from the bar: `lo..=hi`, `hi`
/// unbounded (`None`) while only 0.00% has been read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub lo: u64,
    pub hi: Option<u64>,
}

/// What the core made of an input, beyond what it showed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Note {
    /// The EXP changed for the first time: the Session started.
    SessionStarted,
    /// A level-up from a level learned as `level`: `gain` EXP counted, what was left of
    /// it and what the bar shows in the new one.
    LevelUp { gain: i64, level: Level },
    /// Less EXP in the same level: a death, costing `exp_lost`.
    Death { exp_lost: u64 },
    /// No level's total allows the reading: a misread, set aside.
    NoLevelAllows,
    /// The reading doesn't fit the level learned (`level`): set aside as a misread, unless
    /// it is the third in a row (`count` so far) to fit another level.
    Misfit { count: u32, level: Level },
    /// Readings in a row fitted another level, carrying on from the last: the level was
    /// learned wrong (as `from`), and is learned anew (as `to`).
    Relearned { from: Level, to: Level },
    /// Readings in a row fitted another level (`level`): another character, and a new
    /// Session.
    CharacterSwitch { level: Level },
    /// The Status Bar named another Character, `name`, after it was out of sight: a new
    /// Session.
    NameSwitch { name: String },
    /// The Status Bar's name read as `name`, not the Character's, with none named or after it
    /// was out of sight: `count` readings in a row so far, short of the three that name it.
    NameUnconfirmed { name: String, count: u32 },
    /// An EXP bar reading while the Status Bar's name read as another's, not yet three times
    /// in a row: it may be theirs, and is set aside.
    NameUnconfirmedBar,
    /// The Status Bar, with no name read, showed another level (`level`), not the next, after
    /// it was out of sight: another Character, and a new Session.
    LevelSwitch { level: u32 },
    /// The Status Bar's name read as `name`, not the Character's, while it stayed in sight:
    /// a misread, set aside.
    NameMisread { name: String },
    /// The Status Bar showed the next level, `level`: the next EXP bar reading is in it.
    LevelShown { level: u32 },
    /// The level-up the Status Bar showed couldn't be counted (the level before's total wasn't
    /// known, or the reading had more EXP than the last): the new level is learned afresh.
    LevelUpUncounted,
    /// The Status Bar showed level `to`, not `from` or the next: the level's total is learned
    /// afresh from the next EXP bar reading.
    LevelChanged { from: u32, to: u32 },
    /// The first whole reading after the bar went unread for over 2 minutes: taken as
    /// where the Session stands, without counting the change since.
    Unseen,
    /// The panel confirmed another map: a new Visit.
    Visit,
    /// An Item line's name, cut short by the game, starts more than one Item's name (within
    /// its tab): tallied as the game shows it, `name`.
    ItemCutShort { name: String },
    /// An Item line's name, `name`, matches no Item in OSMS's files, and has been read on
    /// `lines` lines so far: not tallied before the second.
    ItemUnmatched { name: String, lines: u32 },
}

/// An input the core took in, or something written down beside them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Event {
    /// Time passed, to the line's `t`.
    Tick {},
    /// A title capture taken at `at_ms` read as `lines` (`None`: unreadable), the Minimap
    /// Title found on screen at `found` (`None`: not found), or the `same` as the reading
    /// before, found where it was.
    Title {
        at_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lines: Option<TitleLines>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        found: Option<ScreenRect>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        same: bool,
        /// The region the capture was of, when it was no longer captured by the time it was
        /// read (the game window moved, say): the reading was left. `None` for one taken in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stale_region: Option<ScreenRect>,
    },
    /// The EXP figures captured at `at_ms` read as `bar` (`None`: not whole), at `place` in
    /// a Status Bar capture (`None`: a frame of the figures alone, or nowhere to read them).
    ExpBar {
        at_ms: u64,
        bar: Option<ExpBar>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<ExpBarPlace>,
    },
    GameWindow { game: Option<GameWindow> },
    PanelSizes { sizes: PanelSizes },
    /// Once `itemLookupSize`, the Item Lookup alert's, which the Owl Bubble's window was.
    #[serde(alias = "itemLookupSize")]
    OwlBubbleSize { size: Size },
    ItemCardSize { size: Size },
    DealBadgesSize { size: Size },
    GrindTrackerSize { size: Size },
    PersonalBestsSize { size: Size },
    SettingsSize { size: Size },
    Act { action: Action },
    RebindKey { press: KeyPress },
    SearchQuery { query: String },
    SearchPick { id: String },
    SearchPickDestination { id: String },
    SearchPickItem { id: String },
    Hotkey { pressed: String },
    Press { control: PanelControl },
    ItemTooltip { reading: Option<ItemTooltipReading> },
    /// A Deal Finder capture taken at `at_ms` read as `shop` (`None`: no Player Shop, with
    /// the `near_miss` the reader came closest with, if any), or the `same` as the reading
    /// before, its near miss too; with the pointer on the list and the Item Tooltip by it, if
    /// it was on the list.
    PlayerShop {
        at_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shop: Option<ShopList>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        near_miss: Option<ShopNearMiss>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        same: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hover: Option<ShopHover>,
        /// The region the capture was of, when it was no longer captured by the time it was
        /// read (the game window moved, say): the reading was left. `None` for one taken in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stale_region: Option<ScreenRect>,
    },
    /// A Status Bar capture taken at `at_ms` read as `bar` (`None`: no Status Bar, with the
    /// `near_miss` the reader came closest with, if any), or the `same` as the reading
    /// before, its near miss too.
    StatusBar {
        at_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bar: Option<StatusBar>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        near_miss: Option<StatusBarNearMiss>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        same: bool,
        /// The region the capture was of, when it was no longer captured by the time it was
        /// read (the game window moved, say): the reading was left. `None` for one taken in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stale_region: Option<ScreenRect>,
    },
    /// The Pickup Log's observation and arrivals; a stale band is recorded and left.
    PickupLog {
        reading: crate::PickupLogReading,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stale_region: Option<ScreenRect>,
    },
    /// The player chose the Item Card's Market Line `line` as the Deal Line.
    ChooseDealLine { line: String },
    PickItem { id: String },
    PickCardItem { id: String },
    /// The player clicked the Item Card's name at `at_ms`.
    OpenCardMeowdbPage { at_ms: u64 },
    CloseItemCard {},
    /// The player clicked the price on the Item Card's Market Line `line`.
    EditCardMarketPrice { line: String },
    /// The player pressed Enter in the Item Card's price field, holding `typed`.
    EnterCardMarket { typed: String },
    /// The player clicked the name of the Item Card's Market Line `line`.
    EditCardMarketLine { line: String },
    /// The player pressed Enter in the Item Card's line name field, holding `typed`.
    EnterCardMarketLine { typed: String },
    Foreground { foreground: Foreground },
    HotkeysUnavailable { taken: Vec<String>, invalid: Vec<String> },
    Restore { settings: Box<Settings> },
    PickGameWindow { title: String },
    WallClock { unix_ms: u64 },
    RestoreBests { bests: SavedBests },
    /// The Personal Bests file was there but didn't read: the shell restored none.
    BestsLost {},
    RestoreMarketPrices { prices: SavedMarketPrices },
    /// A tool of the player's own left `import` for their Market Lines (ADR 0013).
    ImportMarketPrices { import: MarketImport },
    /// The player removed the Visit whose best was `visit` from `map`'s Visit History, the
    /// Character's named `character` (the Unassigned one for `None`, as every Visit was
    /// before Characters).
    RemoveVisit {
        map: String,
        visit: VisitBest,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        character: Option<String>,
    },
    /// The player moved the Visit whose best was `visit` on `map`, the Character's named
    /// `from` (an Unassigned one for `None`, as every Visit moved was before Move was on every
    /// Visit), to the Character named `to`.
    MoveVisit {
        map: String,
        visit: VisitBest,
        to: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
    },
    /// The player moved the Visit whose best was `visit` on `map`, the Character's named
    /// `character` (the Unassigned one for `None`), to the Party Bests (`party`) or to the Solo
    /// Bests.
    SetVisitParty {
        map: String,
        visit: VisitBest,
        party: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        character: Option<String>,
    },
    /// The player gave the Character named `to` every Unassigned Visit.
    AssignUnassigned { to: String },
    /// The player removed the Character named `name`, with their bests.
    RemoveCharacter { name: String },
    /// The player merged the Character named `from` into the one named `into`.
    MergeCharacters { from: String, into: String },
    /// The player turned "Share to the Map Leaderboards" on (`on`) or off for the Character
    /// named `name`.
    ShowOnLeaderboards { name: String, on: bool },
    /// The player sent the Character named `name` to the Map Leaderboards again.
    SendToLeaderboardsAgain { name: String },
    /// The Map Leaderboards answered upload `id`, for the Character named `name` (empty for an
    /// upload no longer under way), with `answer`.
    LeaderboardAnswer { id: u64, name: String, answer: LeaderboardAnswer },
    /// The player pressed Link Discord in the Personal Bests window.
    LinkDiscord {},
    /// The player said not now to the prompt to link Discord.
    DismissLinkPrompt {},
    /// Request `id` about linking to Discord came to `answer`. A link page opened says so,
    /// never with its token.
    DiscordLinkAnswer { id: u64, answer: DiscordLinkAnswer },
    /// Check `id` for a newer version came to `answer`.
    UpdateAnswer { id: u64, answer: UpdateAnswer },
    /// What this build can do with a newer version: install it, or only say it is out.
    Build { build: Build },
    /// Download `id` of a newer version came to `answer`.
    UpdateDownload { id: u64, answer: DownloadAnswer },
    /// The installer download `id` fetched couldn't be started, for `why`.
    UpdateInstallFailed { id: u64, why: String },
    /// Not an input: an error the shell or one of our windows ran into (`from`), which
    /// would otherwise go nowhere.
    Error { from: String, message: String },
    /// Not an input: the player saved a Bug Report here.
    BugReport {},
}

/// What the source answered while the core took an input in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Answer {
    /// The OSMS file set an earlier launch saved; `None` if none was.
    SavedSet { saved: Option<SavedSet> },
    /// OSMS's file set fetched afresh: its [`fingerprint`], or why it couldn't be had.
    Set { answer: Result<String, SourceError> },
    /// Where one of OSMS's pictures loads from, or why it couldn't be had.
    Picture { path: String, answer: Result<String, SourceError> },
    /// What MeowDB's search gave for an item's name.
    ItemSearch { name: String, answer: Result<ItemSearchAnswer, SourceError> },
    /// The OSMS item files an earlier launch saved; `None` if none were.
    SavedItems { saved: Option<SavedSet> },
    /// OSMS's item files fetched afresh: their [`item_fingerprint`], or why they couldn't
    /// be had.
    Items { answer: Result<String, SourceError> },
}

/// An OSMS file set (or its item files) saved at an earlier launch: its fingerprint, and
/// when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSet {
    pub fingerprint: String,
    pub saved_at_unix_ms: u64,
    /// The version that wrote the report down saved it. Only that counts, not the version,
    /// so the report replays the same in a later one. A report from before saved sets said
    /// which version saved them has every set as its own, as that version took them.
    #[serde(default = "yes")]
    pub this_version: bool,
}

impl SavedSet {
    fn of(fingerprint: String, saved_at_unix_ms: u64, saved_by: Option<&str>) -> Self {
        Self { fingerprint, saved_at_unix_ms, this_version: saved_by == Some(MINERVA_VERSION) }
    }

    /// Who saved it, as the replaying version takes it.
    fn saved_by(&self) -> Option<String> {
        self.this_version.then(|| MINERVA_VERSION.to_string())
    }
}

fn yes() -> bool {
    true
}

/// MeowDB's items for a name, and when they were saved if it couldn't be reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSearchAnswer {
    pub hits: Vec<ItemHit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_at_unix_ms: Option<u64>,
}

/// The map the panel shows, as a line writes it down.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShownMap {
    /// Its 9-digit ID; `None` while no map is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub street_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub map_name: String,
    /// How many maps share its title while it isn't settled which the player is on; 0
    /// once it is.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub candidates: usize,
}

impl ShownMap {
    fn of(state: &PanelState) -> Self {
        let Some(map) = &state.map else { return Self::default() };
        Self {
            id: Some(map.id.clone()),
            street_name: map.street_name.clone(),
            map_name: map.map_name.clone(),
            candidates: state.candidates.as_ref().filter(|c| !c.chosen).map_or(0, |c| c.maps.len()),
        }
    }
}

/// One line of the trace: an input, and what came of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// Capture time, in ms since launch: the latest the core had seen. On a `tick`, the
    /// time it ticked to.
    pub t: u64,
    /// The core ticked to `t` just before, and nothing came of that.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ticked: bool,
    #[serde(flatten)]
    pub event: Event,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<Answer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
    /// What the Grind Tracker showed after, where it changed: the fields of
    /// [`ExpTracking`] that did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grind_tracker: Option<Map<String, Value>>,
    /// The map the panel showed after, where it changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<ShownMap>,
    /// The uploads to the Map Leaderboards the core asked the shell for after.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uploads: Vec<AskedUpload>,
    /// The request about linking to Discord the core asked the shell for after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discord_link: Option<DiscordLinkRequest>,
    /// The check for a newer version the core asked the shell for after, by its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_check: Option<u64>,
    /// The step of updating the core asked the shell for after: a download, or an install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_job: Option<UpdateJob>,
    /// The crop of the frame read, kept in the report under this name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<String>,
    /// The crop of the Item Tooltip by the pointer in the frame read (a Deal Finder
    /// capture's), kept in the report under this name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip_crop: Option<String>,
}

impl Line {
    fn new(t: u64, ticked: bool, event: Event) -> Self {
        let (answers, notes, uploads) = (Vec::new(), Vec::new(), Vec::new());
        Self {
            t,
            ticked,
            event,
            answers,
            notes,
            grind_tracker: None,
            map: None,
            uploads,
            discord_link: None,
            update_check: None,
            update_job: None,
            crop: None,
            tooltip_crop: None,
        }
    }
}

/// An upload to the Map Leaderboards the core asked for, as a line writes it down: its id,
/// whose, and how many entries (`None`: deleting them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskedUpload {
    pub id: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries: Option<usize>,
}

/// What the shell knows of the run that the core doesn't, for a Bug Report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReportAbout {
    /// Minerva's version.
    pub version: String,
    /// The operating system, as the shell can tell it ("Windows 11, build 26100").
    pub os: String,
    /// The display scaling of the panel's monitor: 1.5 for 150%.
    pub display_scale: f64,
    /// The local date and time now, as `yyyy-mm-dd hh:mm:ss`.
    pub local_time: String,
}

/// A map the panel showed over the run, with what OSMS's set held of it then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapFacts {
    pub id: String,
    pub street_name: String,
    pub map_name: String,
    pub region: String,
    pub summary: Option<MapSummary>,
}

/// `report.json`: the run's facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportFile {
    pub version: String,
    pub os: String,
    pub display_scale: f64,
    /// The local date and time it was saved.
    pub saved_at: String,
    /// The capture time it was saved at, in ms since launch.
    pub saved_at_ms: u64,
    pub saved_at_unix_ms: Option<u64>,
    /// The capture time the trace starts at: 0, unless the run outlasted [`TRACE_MS`].
    pub trace_from_ms: u64,
    /// The run outlasted [`TRACE_MS`]: the trace starts partway, after the lines that
    /// set the run up.
    pub truncated: bool,
    /// The game window's client area, physical screen px.
    pub game_client: Option<ScreenRect>,
    /// Where the Minimap Title was being captured, physical screen px: where it was found, or
    /// the whole client area while it was looked for (ADR 0021).
    pub title_capture: Option<ScreenRect>,
    /// The fingerprints of the OSMS file sets the run took, in order.
    pub osms_sets: Vec<String>,
    /// Every map the panel showed.
    pub maps: Vec<MapFacts>,
    /// The crops in the report, and those left out to keep it small.
    pub crops: usize,
    pub crops_left_out: usize,
    /// The captures where a reader found nothing (#133) left out, too big for the report: their
    /// names, and their size as PNGs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub captures_left_out: Vec<LeftOut>,
}

/// A capture where a reader found nothing, left out of a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftOut {
    pub name: String,
    pub bytes: usize,
}

/// A Bug Report, ready to save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BugReport {
    /// `minerva-bug-report-<version>-<yyyy-mm-dd>-<hhmmss>.zip`.
    pub file_name: String,
    pub zip: Vec<u8>,
}

/// A fingerprint of an OSMS file set (FNV-1a, 64 bits, in hex): two sets with the same
/// one are the same set.
pub fn fingerprint(files: &OsmsFiles) -> String {
    texts_fingerprint([&*files.maps, &files.portals, &files.monsters, &files.map_manifest, &files.lookups])
}

/// OSMS's item files' fingerprint, as [`fingerprint`] the map set's.
pub fn item_fingerprint(files: &ItemFiles) -> String {
    texts_fingerprint([&*files.items, &files.cash_shop])
}

fn texts_fingerprint<'a>(texts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for text in texts {
        for byte in text.bytes().chain([0]) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Which region a frame was captured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Region {
    Title,
    ExpBar,
    /// The game's client area around the pointer, at an Item Card or Item Lookup press,
    /// with the pointer at (x, y) in it.
    ItemTooltip { pointer: (u32, u32) },
    /// A Player Shop's item list, at a Deal Finder capture.
    PlayerShop,
    /// The Status Bar's level, job and name, at a Status Bar capture.
    StatusBar,
    /// The game's whole client area, at a Deal Finder capture with no shop found (#133):
    /// `near_miss`, its reader saw one.
    ClientArea { near_miss: bool },
    /// The whole strip, at a Status Bar capture with no bar found (#133).
    StatusBarStrip,
    PickupLog,
}

impl Region {
    /// What its crop of a frame captured at `at_ms` is named in a report.
    fn crop_name(self, at_ms: u64) -> String {
        match self {
            Region::Title => format!("crops/minimap-title-{at_ms:010}.png"),
            Region::ExpBar => format!("crops/exp-bar-{at_ms:010}.png"),
            // Named as `samples/item-tooltip` names its frames: the pointer's spot in it.
            Region::ItemTooltip { pointer: (x, y) } => format!("crops/item-tooltip-{at_ms:010}@{x},{y}.png"),
            Region::PlayerShop => format!("crops/player-shop-{at_ms:010}.png"),
            Region::StatusBar => format!("crops/status-bar-{at_ms:010}.png"),
            Region::ClientArea { .. } => format!("crops/client-area-{at_ms:010}.png"),
            Region::StatusBarStrip => format!("crops/status-bar-strip-{at_ms:010}.png"),
            Region::PickupLog => format!("crops/pickup-log-{at_ms:010}.png"),
        }
    }
}

/// How far the crop of an Item Card or Item Lookup capture reaches from the pointer (px,
/// each way): far enough for the tooltip wherever the game puts it beside the pointer.
const ITEM_TOOLTIP_REACH: u32 = 400;

/// The part of an Item Card or Item Lookup capture around the pointer, cut to the game's
/// client area, for a Bug Report: the Item Tooltip the reader read, or should have.
pub(crate) fn item_tooltip_crop(capture: &ItemCapture) -> Option<(Region, Frame)> {
    around_pointer(&capture.frame, capture.origin, capture.pointer)
}

/// Likewise for the Item Tooltip by the pointer on a Player Shop's list, in a Deal Finder
/// capture: `None` while the pointer was off the list.
pub(crate) fn shop_tooltip_crop(capture: &ShopCapture, hover: Option<&ShopHover>) -> Option<(Region, Frame)> {
    around_pointer(&capture.frame, capture.origin, hover?.pointer)
}

/// The part of `frame`, its top-left pixel at `origin` on screen, around `pointer`.
fn around_pointer(frame: &Frame, origin: ScreenPoint, pointer: ScreenPoint) -> Option<(Region, Frame)> {
    if frame.width == 0 || frame.height == 0 || frame.rgba.len() < (frame.width * frame.height * 4) as usize {
        return None;
    }
    let at = |pointer: i32, origin: i32, size: u32| (pointer - origin).clamp(0, size as i32 - 1) as u32;
    let (px, py) = (at(pointer.x, origin.x, frame.width), at(pointer.y, origin.y, frame.height));
    let (x0, y0) = (px.saturating_sub(ITEM_TOOLTIP_REACH), py.saturating_sub(ITEM_TOOLTIP_REACH));
    let (x1, y1) = ((px + ITEM_TOOLTIP_REACH).min(frame.width), (py + ITEM_TOOLTIP_REACH).min(frame.height));
    Some((Region::ItemTooltip { pointer: (px - x0, py - y0) }, cut(frame, (x0, y0), (x1, y1))))
}

/// Columns `x0..x1` and rows `y0..y1` of `frame`, which holds them.
fn cut(frame: &Frame, (x0, y0): (u32, u32), (x1, y1): (u32, u32)) -> Frame {
    let mut rgba = Vec::with_capacity(((x1 - x0) * (y1 - y0) * 4) as usize);
    for y in y0..y1 {
        let row = ((y * frame.width + x0) * 4) as usize;
        rgba.extend_from_slice(&frame.rgba[row..row + ((x1 - x0) * 4) as usize]);
    }
    Frame { width: x1 - x0, height: y1 - y0, rgba, at_ms: frame.at_ms }
}

/// What of `frame`, its top-left pixel at `origin` on screen, lies in `rect` on screen, grown
/// by `margin` px each way; `None` if nothing does, or the frame is short of pixels.
pub(crate) fn rect_crop(frame: &Frame, origin: ScreenPoint, rect: ScreenRect, margin: u32) -> Option<Frame> {
    let margin = margin as i32;
    let top_left = (rect.x - margin, rect.y - margin);
    let bottom_right = (rect.x + rect.width as i32 + margin, rect.y + rect.height as i32 + margin);
    cut_on_screen(frame, origin, top_left, bottom_right)
}

/// What of `frame`, its top-left pixel at `origin` on screen, lies between the screen points
/// `top_left` and `bottom_right`; `None` if nothing does, or the frame is short of pixels.
fn cut_on_screen(frame: &Frame, origin: ScreenPoint, top_left: (i32, i32), bottom_right: (i32, i32)) -> Option<Frame> {
    if frame.rgba.len() < (frame.width * frame.height * 4) as usize {
        return None;
    }
    let at = |screen: i32, origin: i32, size: u32| (screen - origin).clamp(0, size as i32) as u32;
    let (x0, y0) = (at(top_left.0, origin.x, frame.width), at(top_left.1, origin.y, frame.height));
    let (x1, y1) = (at(bottom_right.0, origin.x, frame.width), at(bottom_right.1, origin.y, frame.height));
    (x1 > x0 && y1 > y0).then(|| cut(frame, (x0, y0), (x1, y1)))
}

/// How far the crop of a Deal Finder capture reaches around the shop's list (px, at the
/// samples' UI Size; as much farther at a larger one, ADR 0020): enough for the reader to
/// find the list in it again, with the scroll bar, and none of the shop's title, its seller's
/// name or its chat.
const PLAYER_SHOP_MARGIN: f32 = 10.0;
/// ...and how far right of the list's picture column: past the scroll bar.
const PLAYER_SHOP_WIDTH: f32 = 352.0;

/// The Player Shop's item list in a Deal Finder capture, for a Bug Report: the listings the
/// reader read, or should have, as `samples/player-shop` keeps them. With no shop, the whole
/// capture, as it was captured (#133).
pub(crate) fn player_shop_crop(capture: ShopCapture, reading: &ShopReading) -> Option<(Region, Frame)> {
    let Some(shop) = &reading.shop else {
        return Some((Region::ClientArea { near_miss: reading.near_miss.is_some() }, capture.frame));
    };
    // Down to every listing, the covered ones past the bands found included.
    let (list, size) = (shop.span(), shop.ui_size());
    let (margin, width) = ((PLAYER_SHOP_MARGIN * size).round() as i32, (PLAYER_SHOP_WIDTH * size).round() as i32);
    let top_left = (list.x - margin, list.y - margin);
    let bottom_right = (list.x + width, list.y + list.height as i32 + margin);
    Some((Region::PlayerShop, cut_on_screen(&capture.frame, capture.origin, top_left, bottom_right)?))
}

/// The Minimap Title in a title capture, for a Bug Report: where it was found (ADR 0021), as
/// a sample of it in `samples/minimap` keeps it, and none of the rest of the client area.
/// `None` with no title found.
pub(crate) fn title_crop(capture: &TitleCapture, found: Option<ScreenRect>) -> Option<(Region, Frame)> {
    Some((Region::Title, rect_crop(&capture.frame, capture.origin, found?, 0)?))
}

/// How far the crop of a Status Bar capture reaches around the bar's level, job and name
/// (px): enough for the reader to find them in it again.
const STATUS_BAR_MARGIN: u32 = 10;

/// The Status Bar's level, job and name in a Status Bar capture, for a Bug Report, as
/// `samples/status-bar` keeps them: none of the rest of the bar, nor the game above it. With
/// no Status Bar, the whole strip, as it was captured (#133).
pub(crate) fn status_bar_crop(capture: StatusBarCapture, bar: Option<&StatusBar>) -> Option<(Region, Frame)> {
    let Some(bar) = bar else { return Some((Region::StatusBarStrip, capture.frame)) };
    Some((Region::StatusBar, rect_crop(&capture.frame, capture.origin, bar.area, STATUS_BAR_MARGIN)?))
}

/// A crop of a captured region, as a PNG.
struct Crop {
    /// Its name in the report: `crops/exp-bar-<capture ms>.png`.
    name: String,
    /// The capture time.
    at_ms: u64,
    /// Kept for the run: behind a reading the core found unusual, or one that changed
    /// the map shown.
    kept: bool,
    png: Vec<u8>,
}

/// The latest capture a reader found nothing in (#133), held as captured until a report is
/// saved, when it is made a PNG: the capture loop costs no more for it.
#[derive(Default)]
struct Unfound {
    latest: Option<Frame>,
    /// The latest whose reader saw a near miss, when it isn't the latest.
    near_miss: Option<Frame>,
}

impl Unfound {
    /// Hold `frame`, the latest; with a `near_miss`, over the one held for one before.
    fn hold(&mut self, frame: Frame, near_miss: bool) {
        if near_miss {
            self.near_miss = Some(frame);
            self.latest = None;
        } else {
            self.latest = Some(frame);
        }
    }

    /// Let go of any captured before `since`.
    fn expire(&mut self, since: u64) {
        for held in [&mut self.latest, &mut self.near_miss] {
            if held.as_ref().is_some_and(|frame| frame.at_ms < since) {
                *held = None;
            }
        }
    }

    /// The one a report keeps, of those captured since `since`: the latest with a near miss,
    /// else the latest.
    fn kept<'a>(&'a self, since: u64) -> Option<&'a Frame> {
        let recent = |held: Option<&'a Frame>| held.filter(|frame| frame.at_ms >= since);
        recent(self.near_miss.as_ref()).or(recent(self.latest.as_ref()))
    }
}

/// The trace's text for a stretch of it, compressed.
struct Chunk {
    /// The `t` of its last line.
    to_t: u64,
    deflated: Vec<u8>,
}

/// The input being taken, written down once what came of it is known.
struct Open {
    line: Line,
    frames: Vec<(Region, Frame)>,
}

/// What came of an input.
pub(crate) struct Outcome<'a> {
    pub answers: Vec<Answer>,
    pub notes: Vec<Note>,
    pub state: &'a PanelState,
    pub bests_revision: u64,
}

/// The last of everything a line is written against.
#[derive(Default)]
struct Last {
    state: PanelState,
    grind_tracker: Map<String, Value>,
    map: ShownMap,
    bests_revision: u64,
    game: Option<Option<GameWindow>>,
    panel_sizes: Option<PanelSizes>,
    owl_bubble_size: Option<Size>,
    item_card_size: Option<Size>,
    deal_badges_size: Option<Size>,
    grind_tracker_size: Option<Size>,
    personal_bests_size: Option<Size>,
    settings_size: Option<Size>,
    foreground: Option<Foreground>,
    unavailable_hotkeys: Option<(Vec<String>, Vec<String>)>,
    /// What the last title capture read as, and where the title was found.
    title: Option<(Option<TitleLines>, Option<ScreenRect>)>,
    /// What the last Deal Finder capture read as, and the near miss with no shop.
    shop: Option<(Option<ShopList>, Option<ShopNearMiss>)>,
    /// What the Item Tooltip by the pointer on the shop's list said in the last Deal Finder
    /// capture with one: its name as read (`None`: unreadable).
    shop_tooltip: Option<Option<String>>,
    /// What the last Status Bar capture read as, and the near miss with no bar.
    status_bar: Option<(Option<StatusBar>, Option<StatusBarNearMiss>)>,
    exp_bar_rgba: Vec<u8>,
    /// The id of the last upload to the Map Leaderboards written down as asked for.
    upload_id: u64,
    /// The id of the last request about linking to Discord written down as asked for.
    discord_link_id: u64,
    /// The id of the last check for a newer version written down as asked for.
    update_check: u64,
    /// The last step of updating written down as asked for.
    update_job: Option<UpdateJob>,
}

/// Writes the trace down as the core takes its inputs in, and keeps the crops.
#[derive(Default)]
pub(crate) struct Recorder {
    chunks: VecDeque<Chunk>,
    /// The trace's text since the last chunk.
    text: String,
    /// The lines that set the run up, kept should their chunk go.
    launch: String,
    /// A chunk has gone.
    truncated: bool,
    open: Option<Open>,
    /// A tick that changed nothing, since the last line written.
    idle_tick: Option<u64>,
    /// The latest capture time seen.
    now: u64,
    /// The wall clock as last written down, and the capture time then.
    wall_clock: Option<(u64, u64)>,
    last: Last,
    crops: VecDeque<Crop>,
    /// The latest Deal Finder capture with no shop found, and Status Bar capture with no bar.
    client_area: Unfound,
    status_bar_strip: Unfound,
    /// Every map shown over the run, by ID.
    maps: BTreeSet<String>,
    /// The fingerprints of the OSMS sets taken, in order.
    sets: Vec<String>,
    /// The latest notes and errors, as `summary.txt` lists them.
    notable: VecDeque<String>,
}

/// Whether `value` isn't what was `last` written down, which it is from now on.
fn changes<T: PartialEq + Clone>(last: &mut Option<T>, value: &T) -> bool {
    if last.as_ref() == Some(value) {
        return false;
    }
    *last = Some(value.clone());
    true
}

impl Recorder {
    /// A recorder for a run starting from `state`: lines are written against it.
    pub fn new(state: &PanelState) -> Self {
        let last = Last {
            state: state.clone(),
            grind_tracker: fields(&state.exp_tracking),
            map: ShownMap::of(state),
            ..Last::default()
        };
        Self { last, ..Self::default() }
    }

    /// Whether `event` is to be written down: an input that only sets a value is, only
    /// when the value changes.
    pub fn takes(&mut self, event: &Event) -> bool {
        let last = &mut self.last;
        match event {
            Event::GameWindow { game } => changes(&mut last.game, game),
            Event::PanelSizes { sizes } => changes(&mut last.panel_sizes, sizes),
            Event::OwlBubbleSize { size } => changes(&mut last.owl_bubble_size, size),
            Event::ItemCardSize { size } => changes(&mut last.item_card_size, size),
            Event::DealBadgesSize { size } => changes(&mut last.deal_badges_size, size),
            Event::GrindTrackerSize { size } => changes(&mut last.grind_tracker_size, size),
            Event::PersonalBestsSize { size } => changes(&mut last.personal_bests_size, size),
            Event::SettingsSize { size } => changes(&mut last.settings_size, size),
            Event::Foreground { foreground } => changes(&mut last.foreground, foreground),
            Event::HotkeysUnavailable { taken, invalid } => {
                changes(&mut last.unavailable_hotkeys, &(taken.clone(), invalid.clone()))
            }
            _ => true,
        }
    }

    /// Start writing down `event`, an input captured or ticked at `at_ms` if it has a
    /// time, read from `frames` if it has any. What came of it is written once the next
    /// input comes. A reading the same as the one before says so, not what it read; a
    /// reading left, as for a region no longer captured, is neither the same as the one
    /// before nor one the next can be.
    pub fn open(&mut self, mut event: Event, at_ms: Option<u64>, frames: Vec<(Region, Frame)>) {
        if let Event::Title { lines, found, same, stale_region: None, .. } = &mut event {
            *same = !changes(&mut self.last.title, &(lines.clone(), *found));
            if *same {
                *lines = None;
                *found = None;
            }
        }
        // A near miss is part of what was read: a shop opened that isn't found is no longer
        // the same as no shop open.
        if let Event::PlayerShop { shop, near_miss, same, hover, stale_region: None, .. } = &mut event {
            // Off the list, no tooltip: the next one by it is new.
            if hover.is_none() {
                self.last.shop_tooltip = None;
            }
            *same = !changes(&mut self.last.shop, &(shop.clone(), *near_miss));
            if *same {
                *shop = None;
                *near_miss = None;
            }
        }
        if let Event::StatusBar { bar, near_miss, same, stale_region: None, .. } = &mut event {
            *same = !changes(&mut self.last.status_bar, &(bar.clone(), *near_miss));
            if *same {
                *bar = None;
                *near_miss = None;
            }
        }
        let line = match (&event, at_ms) {
            (Event::Tick {}, Some(at)) => Line::new(at, false, event),
            _ => match self.idle_tick.take() {
                Some(at) => Line::new(at, true, event),
                None => Line::new(self.now, false, event),
            },
        };
        if let Some(at) = at_ms {
            self.now = self.now.max(at);
        }
        self.open = Some(Open { line, frames });
    }

    /// Write down the input being taken, with what came of it. A tick that changed
    /// nothing isn't written: the next line says it came first.
    pub fn close(&mut self, outcome: Outcome<'_>) {
        let Some(Open { mut line, frames }) = self.open.take() else { return };
        let changed = *outcome.state != self.last.state;
        let bests_changed = outcome.bests_revision != self.last.bests_revision;
        if line.event == (Event::Tick {}) {
            if !changed && !bests_changed && outcome.answers.is_empty() && outcome.notes.is_empty() {
                self.idle_tick = Some(line.t);
                return;
            }
            // It comes later than the idle one, which it makes no odds to.
            self.idle_tick = None;
        }
        line.answers = outcome.answers;
        line.notes = outcome.notes;
        self.last.bests_revision = outcome.bests_revision;
        if changed {
            let grind_tracker = fields(&outcome.state.exp_tracking);
            let diff: Map<String, Value> = grind_tracker
                .iter()
                .filter(|(key, value)| self.last.grind_tracker.get(*key) != Some(value))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            if !diff.is_empty() {
                line.grind_tracker = Some(diff);
                self.last.grind_tracker = grind_tracker;
            }
            let map = ShownMap::of(outcome.state);
            if map != self.last.map {
                line.map = Some(map.clone());
                self.last.map = map;
            }
            let last_upload = self.last.upload_id;
            for upload in outcome.state.leaderboard_uploads.iter().filter(|u| u.id > last_upload) {
                let entries = upload.entries.as_ref().map(Vec::len);
                line.uploads.push(AskedUpload { id: upload.id, name: upload.name.clone(), entries });
                self.last.upload_id = self.last.upload_id.max(upload.id);
            }
            if let Some(request) = outcome.state.discord_link_requests.iter().find(|r| r.id > self.last.discord_link_id) {
                line.discord_link = Some(request.clone());
                self.last.discord_link_id = request.id;
            }
            if let Some(id) = outcome.state.updates.check.filter(|&id| id > self.last.update_check) {
                line.update_check = Some(id);
                self.last.update_check = id;
            }
            let job = outcome.state.updates.job;
            if job != self.last.update_job {
                line.update_job = job;
                self.last.update_job = job;
            }
            self.last.state = outcome.state.clone();
        }
        for (region, frame) in frames {
            self.keep_crop(&mut line, region, frame, outcome.state);
        }
        self.write(line);
    }

    /// The wall clock now: whether it strays from the capture clock since it was last
    /// written down, so it is to be written again.
    pub fn wall_clock(&mut self, unix_ms: u64) -> bool {
        let expected = self.wall_clock.map(|(wall, at)| wall + self.now.saturating_sub(at));
        let strays = expected.is_none_or(|expected| expected.abs_diff(unix_ms) > WALL_CLOCK_DRIFT_MS);
        if strays {
            self.wall_clock = Some((unix_ms, self.now));
        }
        strays
    }

    /// Every map shown over the run, by ID.
    pub fn maps_shown(&self) -> impl Iterator<Item = &str> {
        self.maps.iter().map(String::as_str)
    }

    /// Keep the crop of `frame`, read for `line`: one that changed, or that the core made
    /// something of (`state` after it).
    fn keep_crop(&mut self, line: &mut Line, region: Region, frame: Frame, state: &PanelState) {
        let since = self.now.saturating_sub(RECENT_CROPS_MS);
        let (changed, kept) = match region {
            // Held whole, apart from the crops (#133).
            Region::ClientArea { near_miss } => {
                self.client_area.hold(frame, near_miss);
                self.client_area.expire(since);
                return;
            }
            // The latest, near miss or not: a bar not found while playing is the bug itself.
            Region::StatusBarStrip => {
                self.status_bar_strip.hold(frame, false);
                self.status_bar_strip.expire(since);
                return;
            }
            Region::ExpBar => {
                let changed = frame.rgba != self.last.exp_bar_rgba;
                (changed, line.notes.iter().any(Note::is_unusual))
            }
            // The caller supplies only exceptional Pickup Log crops, never every tick.
            Region::PickupLog => (true, true),
            Region::Title => {
                let same = matches!(line.event, Event::Title { same: true, .. });
                (!same, line.map.is_some())
            }
            // A Deal Finder capture's, when its tooltip says something new, as the pointer
            // lingers on a listing and the tooltip follows it; for the run, one whose name
            // couldn't be read.
            Region::ItemTooltip { .. } if matches!(line.event, Event::PlayerShop { .. }) => {
                let says = match &line.event {
                    Event::PlayerShop { hover: Some(ShopHover { tooltip: ItemTooltip::Found { name, .. }, .. }), .. } => {
                        Some(name.clone())
                    }
                    _ => None,
                };
                let changed = says.is_some() && says != self.last.shop_tooltip;
                let unread = says == Some(None);
                self.last.shop_tooltip = says;
                (changed, changed && unread)
            }
            // Each press's, as presses are few; for the run, one whose name couldn't be read
            // or isn't an item's, as it may have been misread.
            Region::ItemTooltip { .. } => {
                let unread = matches!(
                    &line.event,
                    Event::ItemTooltip { reading: Some(ItemTooltipReading { tooltip: ItemTooltip::Found { name: None, .. }, .. }) }
                );
                let no_such = state.item_lookup.as_ref().is_some_and(|said| {
                    matches!(said.outcome, ItemLookup::NoSuchItem { .. } | ItemLookup::NotFound { .. })
                });
                (true, unread || no_such)
            }
            // One that changed; for the run, one with a listing that couldn't be read whole,
            // and nothing covers, as it may have been misread.
            Region::PlayerShop => {
                let same = matches!(line.event, Event::PlayerShop { same: true, .. });
                let unread = match &line.event {
                    Event::PlayerShop { shop: Some(shop), .. } => {
                        shop.rows.iter().any(|row| !row.covered && (row.name.is_none() || row.price.is_none()))
                    }
                    _ => false,
                };
                (!same, !same && unread)
            }
            // One that changed; for the run, one the core made something of (another
            // Character, a misread name, a level), or with a part found but not read, as it
            // may have been misread.
            Region::StatusBar => {
                let same = matches!(line.event, Event::StatusBar { same: true, .. });
                let unread = matches!(
                    &line.event,
                    Event::StatusBar { bar: Some(bar), .. } if bar.level.is_none() || bar.job.is_none() || bar.name.is_none()
                );
                (!same, !same && (unread || line.notes.iter().any(Note::is_unusual)))
            }
        };
        if changed || kept {
            let name = region.crop_name(frame.at_ms);
            if let Some(png) = png(&frame) {
                let shop_tooltip = matches!(region, Region::ItemTooltip { .. }) && matches!(line.event, Event::PlayerShop { .. });
                *(if shop_tooltip { &mut line.tooltip_crop } else { &mut line.crop }) = Some(name.clone());
                self.crops.push_back(Crop { name, at_ms: frame.at_ms, kept, png });
            }
        }
        if region == Region::ExpBar {
            self.last.exp_bar_rgba = frame.rgba;
        }
        self.client_area.expire(since);
        self.status_bar_strip.expire(since);
        self.crops.retain(|crop| crop.kept || crop.at_ms >= since);
        let mut kept_bytes: usize = self.crops.iter().filter(|crop| crop.kept).map(|crop| crop.png.len()).sum();
        while kept_bytes > KEPT_CROPS_BYTES {
            let Some(oldest) = self.crops.iter().position(|crop| crop.kept) else { break };
            kept_bytes -= self.crops.remove(oldest).map_or(0, |crop| crop.png.len());
        }
    }

    fn write(&mut self, line: Line) {
        for answer in &line.answers {
            let fingerprint = match answer {
                Answer::SavedSet { saved: Some(saved) } => Some(&saved.fingerprint),
                Answer::Set { answer: Ok(fingerprint) } => Some(fingerprint),
                _ => None,
            };
            if let Some(fingerprint) = fingerprint.filter(|f| !self.sets.contains(f)) {
                self.sets.push(fingerprint.clone());
            }
        }
        if let Some(id) = line.map.as_ref().and_then(|map| map.id.as_ref()) {
            self.maps.insert(id.clone());
        }
        let title = self.last.title.as_ref().and_then(|(lines, _)| lines.as_ref());
        let notable = match &line.event {
            Event::Error { .. } | Event::BugReport {} => Some(line.event.describe(title)),
            event if !line.notes.is_empty() => {
                let notes: Vec<String> = line.notes.iter().map(Note::describe).collect();
                Some(format!("{}: {}", event.describe(title), notes.join("; ")))
            }
            _ => None,
        };
        if let Some(notable) = notable {
            self.notable.push_back(format!("{}  {notable}", clock(line.event.at_ms().unwrap_or(line.t))));
            if self.notable.len() > NOTABLE_LINES {
                self.notable.pop_front();
            }
        }
        let Ok(json) = serde_json::to_string(&line) else { return };
        let launch = matches!(
            line.event,
            Event::Restore { .. }
                | Event::RestoreBests { .. }
                | Event::BestsLost {}
                | Event::RestoreMarketPrices { .. }
                | Event::Build { .. }
        );
        if launch {
            self.launch.push_str(&json);
            self.launch.push('\n');
        }
        self.text.push_str(&json);
        self.text.push('\n');
        if self.text.len() >= CHUNK_BYTES {
            let mut deflater = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            if deflater.write_all(self.text.as_bytes()).is_ok()
                && let Ok(deflated) = deflater.finish()
            {
                self.chunks.push_back(Chunk { to_t: line.t, deflated });
                self.text.clear();
            }
        }
        let since = self.now.saturating_sub(TRACE_MS);
        while self.chunks.front().is_some_and(|chunk| chunk.to_t < since) {
            self.chunks.pop_front();
            self.truncated = true;
        }
    }

    /// The trace as written down so far.
    pub fn trace(&self) -> String {
        let mut trace = if self.truncated { self.launch.clone() } else { String::new() };
        for chunk in &self.chunks {
            let _ = flate2::read::DeflateDecoder::new(&chunk.deflated[..]).read_to_string(&mut trace);
        }
        trace.push_str(&self.text);
        trace
    }

    /// The Bug Report: the trace so far, with `summary.txt`, `report.json` and the crops, in a
    /// `.zip` whose text and ordinary crops take at most [`MAX_TEXT_AND_CROPS_BYTES`], and
    /// which is at most [`MAX_REPORT_BYTES`] with the captures where a reader found nothing.
    pub fn report(&self, about: &ReportAbout, press: Press<'_>) -> BugReport {
        let trace = self.trace();
        let trace_from_ms = match (self.truncated, self.chunks.front()) {
            (true, Some(_)) => {
                let first = trace.lines().nth(self.launch.lines().count());
                first.and_then(|line| serde_json::from_str::<Line>(line).ok()).map_or(0, |line| line.t)
            }
            _ => 0,
        };
        let file = |crops: usize, crops_left_out: usize, captures_left_out: Vec<LeftOut>| ReportFile {
            version: about.version.clone(),
            os: about.os.clone(),
            display_scale: about.display_scale,
            saved_at: about.local_time.clone(),
            saved_at_ms: self.now,
            saved_at_unix_ms: press.wall_clock,
            trace_from_ms,
            truncated: self.truncated,
            game_client: press.game_client,
            title_capture: press.state.title_capture,
            osms_sets: self.sets.clone(),
            maps: press.maps.clone(),
            crops,
            crops_left_out,
            captures_left_out,
        };
        let texts = |file: &ReportFile| {
            let report = serde_json::to_string_pretty(file).unwrap_or_default();
            vec![
                ("summary.txt".to_string(), self.summary(file, press.state).into_bytes()),
                ("report.json".to_string(), report.into_bytes()),
                ("trace.jsonl".to_string(), trace.clone().into_bytes()),
            ]
        };
        // The crops kept for the run, and the others from the last 10 minutes: one taken
        // longer ago than that goes even when no crop has come since.
        let since = self.now.saturating_sub(RECENT_CROPS_MS);
        let mut crops: Vec<&Crop> = self.crops.iter().filter(|crop| crop.kept || crop.at_ms >= since).collect();
        // The text alone, then as many crops as fit beside it: the oldest ordinary ones
        // go first, then the oldest kept for the run.
        let text_bytes = zip(&texts(&file(crops.len(), 0, Vec::new())), &[]).len();
        let size = |name: &str, png: &[u8]| png.len() + 2 * name.len() + 100;
        crops.sort_by_key(|crop| crop.at_ms);
        let mut crop_bytes: usize = crops.iter().map(|crop| size(&crop.name, &crop.png)).sum();
        let mut left_out = 0;
        while text_bytes + crop_bytes > MAX_TEXT_AND_CROPS_BYTES - MARGIN_BYTES && !crops.is_empty() {
            let oldest = crops.iter().position(|crop| !crop.kept).unwrap_or(0);
            let crop = crops.remove(oldest);
            crop_bytes -= size(&crop.name, &crop.png);
            left_out += 1;
        }
        let mut pictures: Vec<(String, Vec<u8>)> = crops.iter().map(|crop| (crop.name.clone(), crop.png.clone())).collect();
        // On top, the captures where a reader found nothing, made PNGs now: the strip first, as
        // it is small, then the client area, each if it fits.
        let unfound = [(&self.status_bar_strip, Region::StatusBarStrip), (&self.client_area, Region::ClientArea { near_miss: false })];
        let mut captures_left_out = Vec::new();
        for (frame, region) in unfound.into_iter().filter_map(|(unfound, region)| Some((unfound.kept(since)?, region))) {
            let (name, Some(png)) = (region.crop_name(frame.at_ms), png(frame)) else { continue };
            let bytes = size(&name, &png);
            if text_bytes + crop_bytes + bytes > MAX_REPORT_BYTES - MARGIN_BYTES {
                captures_left_out.push(LeftOut { name, bytes: png.len() });
                continue;
            }
            crop_bytes += bytes;
            pictures.push((name, png));
        }
        let zip = zip(&texts(&file(pictures.len(), left_out, captures_left_out)), &pictures);
        let stamp: String = about.local_time.chars().filter(|c| c.is_ascii_digit()).collect();
        let stamp = match (stamp.get(..4), stamp.get(4..6), stamp.get(6..8), stamp.get(8..14)) {
            (Some(year), Some(month), Some(day), Some(time)) => format!("{year}-{month}-{day}-{time}"),
            _ => stamp,
        };
        BugReport { file_name: format!("minerva-bug-report-{}-{stamp}.zip", about.version), zip }
    }

    /// `summary.txt`: what to look at first.
    fn summary(&self, file: &ReportFile, state: &PanelState) -> String {
        let mut out = String::from("Minerva Bug Report\n\n");
        out += &format!("Version: {}\n", file.version);
        out += &format!("System: {}, display scale {}%\n", file.os, (file.display_scale * 100.0).round());
        out += &format!("Saved: {}, {} after Minerva started\n", file.saved_at, clock(file.saved_at_ms));
        out += &if file.truncated {
            format!(
                "Trace: from {} on (and the settings and Personal Bests restored at launch): \
                 the run outlasted a day\n",
                clock(file.trace_from_ms)
            )
        } else {
            "Trace: every input since launch\n".to_string()
        };
        if file.crops_left_out > 0 {
            out += &format!("Crops: {} kept, {} left out to keep the report small\n", file.crops, file.crops_left_out);
        }
        for left_out in &file.captures_left_out {
            out += &format!(
                "Left out: {}, a whole capture where nothing was found, {:.1} MB as a PNG: too big for the report\n",
                left_out.name,
                left_out.bytes as f64 / 1e6
            );
        }
        out += "\nAt the press\n";
        out += &format!(
            "  Map: {}\n",
            state.map.as_ref().map_or("none".to_string(), |map| format!(
                "{} · {} ({}){}",
                map.street_name,
                map.map_name,
                map.id,
                if state.not_sure { ", not sure" } else { "" }
            ))
        );
        out += &format!("  Grind Tracker: {}\n", grind_tracker(&state.exp_tracking));
        out += &format!("\nWhat the core made of things, and errors (the latest {NOTABLE_LINES})\n");
        if self.notable.is_empty() {
            out += "  none\n";
        }
        for line in &self.notable {
            out += &format!("  {line}\n");
        }
        out += "\nFiles\n";
        out += "  report.json  the run's facts: version, system, regions, the maps shown and their Map Summaries\n";
        out += "  trace.jsonl  every input the core took in, a JSON object a line (t: ms since launch),\n";
        out += "               with what came of it: answers, notes, what the Grind Tracker and map showed\n";
        out += "  crops/       the EXP figures', the Minimap Title's, the Status Bar's, the Item Tooltip's and the Player Shop's crops,\n";
        out += "               named by capture time (ms since launch); and, where nothing was found, a whole capture:\n";
        out += "               client-area-<ms>.png, the Deal Finder's with no shop found (chat and names included),\n";
        out += "               and status-bar-strip-<ms>.png, the bottom of the game window with no Status Bar found\n";
        out
    }
}

/// What the core knows at a press that a report states.
pub(crate) struct Press<'a> {
    pub state: &'a PanelState,
    pub game_client: Option<ScreenRect>,
    pub maps: Vec<MapFacts>,
    pub wall_clock: Option<u64>,
}

/// A `.zip` of `texts` (compressed) and `pictures` (already compressed: stored).
fn zip(texts: &[(String, Vec<u8>)], pictures: &[(String, Vec<u8>)]) -> Vec<u8> {
    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let files = texts.iter().map(|file| (file, deflated)).chain(pictures.iter().map(|file| (file, stored)));
    for ((name, bytes), options) in files {
        if writer.start_file(name.as_str(), options).is_err() || writer.write_all(bytes).is_err() {
            break;
        }
    }
    writer.finish().map(Cursor::into_inner).unwrap_or_default()
}

/// A frame as a PNG.
fn png(frame: &Frame) -> Option<Vec<u8>> {
    use image::ImageEncoder;
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&frame.rgba, frame.width, frame.height, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(out)
}

/// What the Grind Tracker shows, field by field, as a line compares it.
fn fields(tracking: &ExpTracking) -> Map<String, Value> {
    match serde_json::to_value(tracking) {
        Ok(Value::Object(fields)) => fields,
        _ => Map::new(),
    }
}

/// The source the core asks for OSMS's file set and pictures and MeowDB's search,
/// writing down what it answers; or, in a replay, answering what was written down.
pub(crate) struct RecordedSource {
    from: From,
    /// What it answered since they were last taken.
    answers: Vec<Answer>,
}

enum From {
    Live(Box<dyn MapDataSource + Send>),
    /// A replay's: what to replay with, and the answers due for the line replayed.
    Replay { with: Box<ReplayData>, due: VecDeque<Answer> },
}

impl RecordedSource {
    pub fn live(source: Box<dyn MapDataSource + Send>) -> Self {
        Self { from: From::Live(source), answers: Vec::new() }
    }

    fn replaying(with: ReplayData) -> Self {
        Self { from: From::Replay { with: Box::new(with), due: VecDeque::new() }, answers: Vec::new() }
    }

    /// What it answered since this was last asked.
    pub fn take_answers(&mut self) -> Vec<Answer> {
        std::mem::take(&mut self.answers)
    }

    /// In a replay, the answers the line replayed had.
    fn answer_with(&mut self, answers: &[Answer]) {
        if let From::Replay { due, .. } = &mut self.from {
            *due = answers.iter().cloned().collect();
        }
    }

    /// In a replay, the next answer due if `matches` it.
    fn due(&mut self, matches: impl Fn(&Answer) -> bool) -> Option<Answer> {
        match &mut self.from {
            From::Replay { due, .. } if due.front().is_some_and(matches) => due.pop_front(),
            _ => None,
        }
    }

    /// In a replay, what it replays with.
    fn replayed(&self) -> Option<&ReplayData> {
        match &self.from {
            From::Replay { with, .. } => Some(with),
            From::Live(_) => None,
        }
    }

    /// In a replay, OSMS's file set replayed with, if one was given.
    fn replayed_set(&self) -> Option<OsmsFiles> {
        self.replayed()?.osms.clone()
    }

    /// In a replay, OSMS's item files replayed with, if they were given.
    fn replayed_items(&self) -> Option<ItemFiles> {
        self.replayed()?.items.clone()
    }
}

impl MapDataSource for RecordedSource {
    fn saved_osms_files(&mut self) -> Option<SavedFiles> {
        let saved = match &mut self.from {
            From::Live(source) => source.saved_osms_files(),
            From::Replay { .. } => match self.due(|a| matches!(a, Answer::SavedSet { .. })) {
                Some(Answer::SavedSet { saved: Some(saved) }) => self.replayed_set().map(|files| SavedFiles {
                    files,
                    saved_at_unix_ms: saved.saved_at_unix_ms,
                    saved_by: saved.saved_by(),
                }),
                _ => None,
            },
        };
        let shape =
            saved.as_ref().map(|s| SavedSet::of(fingerprint(&s.files), s.saved_at_unix_ms, s.saved_by.as_deref()));
        self.answers.push(Answer::SavedSet { saved: shape });
        saved
    }

    fn fetch_osms_files(&mut self) -> Poll<Result<OsmsFiles, SourceError>> {
        let fetched = match &mut self.from {
            From::Live(source) => source.fetch_osms_files(),
            From::Replay { .. } => match self.due(|a| matches!(a, Answer::Set { .. })) {
                Some(Answer::Set { answer: Ok(_) }) => Poll::Ready(
                    self.replayed_set().ok_or_else(|| SourceError::Unreachable("not in the Bug Report".into())),
                ),
                Some(Answer::Set { answer: Err(e) }) => Poll::Ready(Err(e)),
                _ => Poll::Pending,
            },
        };
        if let Poll::Ready(answer) = &fetched {
            self.answers.push(Answer::Set { answer: answer.as_ref().map(fingerprint).map_err(Clone::clone) });
        }
        fetched
    }

    fn picture(&mut self, path: &str) -> Poll<Result<String, SourceError>> {
        let answer = match &mut self.from {
            From::Live(source) => source.picture(path),
            From::Replay { .. } => match self.due(|a| matches!(a, Answer::Picture { path: p, .. } if p == path)) {
                Some(Answer::Picture { answer, .. }) => Poll::Ready(answer),
                _ => Poll::Pending,
            },
        };
        if let Poll::Ready(answer) = &answer {
            self.answers.push(Answer::Picture { path: path.to_string(), answer: answer.clone() });
        }
        answer
    }

    fn item_search(&mut self, name: &str) -> Poll<Result<Fetched<Vec<ItemHit>>, SourceError>> {
        let answer = match &mut self.from {
            From::Live(source) => source.item_search(name),
            From::Replay { .. } => match self.due(|a| matches!(a, Answer::ItemSearch { name: n, .. } if n == name)) {
                Some(Answer::ItemSearch { answer, .. }) => Poll::Ready(answer.map(|answer| match answer.saved_at_unix_ms {
                    Some(saved_at_unix_ms) => Fetched::SavedCopy { data: answer.hits, saved_at_unix_ms },
                    None => Fetched::Fresh(answer.hits),
                })),
                _ => Poll::Pending,
            },
        };
        if let Poll::Ready(answer) = &answer {
            let shape = answer.as_ref().map_err(Clone::clone).map(|fetched| match fetched {
                Fetched::Fresh(hits) => ItemSearchAnswer { hits: hits.clone(), saved_at_unix_ms: None },
                Fetched::SavedCopy { data, saved_at_unix_ms } => {
                    ItemSearchAnswer { hits: data.clone(), saved_at_unix_ms: Some(*saved_at_unix_ms) }
                }
            });
            self.answers.push(Answer::ItemSearch { name: name.to_string(), answer: shape });
        }
        answer
    }

    fn saved_item_files(&mut self) -> Option<SavedItemFiles> {
        let saved = match &mut self.from {
            From::Live(source) => source.saved_item_files(),
            From::Replay { .. } => match self.due(|a| matches!(a, Answer::SavedItems { .. })) {
                Some(Answer::SavedItems { saved: Some(saved) }) => self.replayed_items().map(|files| SavedItemFiles {
                    files,
                    saved_at_unix_ms: saved.saved_at_unix_ms,
                    saved_by: saved.saved_by(),
                }),
                _ => None,
            },
        };
        let shape =
            saved.as_ref().map(|s| SavedSet::of(item_fingerprint(&s.files), s.saved_at_unix_ms, s.saved_by.as_deref()));
        self.answers.push(Answer::SavedItems { saved: shape });
        saved
    }

    fn fetch_item_files(&mut self) -> Poll<Result<ItemFiles, SourceError>> {
        let fetched = match &mut self.from {
            From::Live(source) => source.fetch_item_files(),
            From::Replay { .. } => match self.due(|a| matches!(a, Answer::Items { .. })) {
                Some(Answer::Items { answer: Ok(_) }) => Poll::Ready(
                    self.replayed_items().ok_or_else(|| SourceError::Unreachable("not in the Bug Report".into())),
                ),
                Some(Answer::Items { answer: Err(e) }) => Poll::Ready(Err(e)),
                _ => Poll::Pending,
            },
        };
        if let Poll::Ready(answer) = &fetched {
            self.answers.push(Answer::Items { answer: answer.as_ref().map(item_fingerprint).map_err(Clone::clone) });
        }
        fetched
    }
}

/// A Bug Report as read back.
#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    pub report: ReportFile,
    pub lines: Vec<Line>,
    /// The names of the crops in it.
    pub crops: Vec<String>,
}

/// Read a Bug Report's `.zip`.
pub fn read(zip: &[u8]) -> Result<Recording, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip)).map_err(|e| format!("not a .zip: {e}"))?;
    let mut text = |name: &str| -> Result<String, String> {
        let mut file = archive.by_name(name).map_err(|e| format!("no {name}: {e}"))?;
        let mut text = String::new();
        file.read_to_string(&mut text).map_err(|e| format!("{name} doesn't read: {e}"))?;
        Ok(text)
    };
    let report: ReportFile =
        serde_json::from_str(&text("report.json")?).map_err(|e| format!("report.json doesn't read: {e}"))?;
    let lines = text("trace.jsonl")?
        .lines()
        .enumerate()
        .map(|(n, line)| serde_json::from_str(line).map_err(|e| format!("trace.jsonl line {}: {e}", n + 1)))
        .collect::<Result<Vec<Line>, String>>()?;
    let crops = archive.file_names().filter(|name| name.starts_with("crops/")).map(String::from).collect();
    Ok(Recording { report, lines, crops })
}

/// What came of replaying a Bug Report.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    /// How many lines were replayed.
    pub lines: usize,
    /// The first line the replay didn't write down as the report did; `None` when it wrote
    /// every one the same, so it took the core through the same states.
    pub divergence: Option<Divergence>,
}

/// A line a replay wrote down otherwise than the report.
#[derive(Debug, Clone, PartialEq)]
pub struct Divergence {
    /// Its index in the trace, from 0.
    pub index: usize,
    /// As the report has it, and as the replay wrote it (JSON, without its crop);
    /// `None` past the end of either.
    pub recorded: Option<String>,
    pub replayed: Option<String>,
}

/// What a replay gives the core where the report holds only fingerprints: OSMS's file set
/// and its item files. `None` answers as if it couldn't be had.
#[derive(Debug, Clone, Default)]
pub struct ReplayData {
    pub osms: Option<OsmsFiles>,
    pub items: Option<ItemFiles>,
}

/// [`replay_with`] OSMS's file set `osms`, and no item files.
pub fn replay(recording: &Recording, osms: Option<OsmsFiles>) -> Replay {
    replay_with(recording, ReplayData { osms, ..ReplayData::default() })
}

/// Feed a Bug Report's inputs to a new core, with what `with` holds where the report had
/// it, and compare what it writes down with the report's trace. A set's fingerprint isn't compared, but what the core makes of it is:
/// a set other than the run's makes it diverge where that shows, as do a trace that
/// starts partway and a change to the core since.
pub fn replay_with(recording: &Recording, with: ReplayData) -> Replay {
    let mut minerva = Minerva::with_source(RecordedSource::replaying(with));
    // The wall clock as last written down, and the capture time then: the shell tells the
    // core it with every tick.
    let mut wall_clock: Option<(u64, u64)> = None;
    // The last readings, which a line the same as the one before stands for.
    let mut title: (Option<TitleLines>, Option<ScreenRect>) = (None, None);
    let mut shop: (Option<ShopList>, Option<ShopNearMiss>) = (None, None);
    let mut status_bar: (Option<StatusBar>, Option<StatusBarNearMiss>) = (None, None);
    for line in &recording.lines {
        if let Some((unix_ms, at)) = wall_clock {
            minerva.wall_clock = Some(unix_ms + line.t.saturating_sub(at));
        }
        if line.ticked {
            minerva.tick(line.t);
        }
        minerva.source.answer_with(&line.answers);
        match line.event.clone() {
            Event::Tick {} => {
                minerva.tick(line.t);
            }
            Event::Title { at_ms, lines, found, same, stale_region } => {
                let (lines, found) = if stale_region.is_some() { (lines, found) } else { as_before(&mut title, (lines, found), same) };
                minerva.title_reading_for(TitleReading { lines, found, at_ms }, stale_region);
            }
            Event::ExpBar { at_ms, bar, place } => {
                minerva.on_exp_bar_reading(ExpBarReading { bar, at_ms, place });
            }
            Event::GameWindow { game } => {
                minerva.on_game_window(game);
            }
            Event::PanelSizes { sizes } => {
                minerva.set_panel_sizes(sizes);
            }
            Event::OwlBubbleSize { size } => {
                minerva.set_owl_bubble_size(size);
            }
            Event::ItemCardSize { size } => {
                minerva.set_item_card_size(size);
            }
            Event::GrindTrackerSize { size } => {
                minerva.set_grind_tracker_size(size);
            }
            Event::PersonalBestsSize { size } => {
                minerva.set_personal_bests_size(size);
            }
            Event::SettingsSize { size } => {
                minerva.set_settings_size(size);
            }
            Event::Act { action } => {
                minerva.act(action);
            }
            Event::RebindKey { press } => {
                minerva.rebind_key(press);
            }
            Event::SearchQuery { query } => {
                minerva.search_query(&query);
            }
            Event::SearchPick { id } => {
                minerva.search_pick(&id);
            }
            Event::SearchPickDestination { id } => {
                minerva.search_pick_destination(&id);
            }
            Event::SearchPickItem { id } => {
                minerva.search_pick_item(&id);
            }
            Event::Hotkey { pressed } => {
                minerva.on_hotkey(&pressed);
            }
            Event::Press { control } => {
                minerva.press(control);
            }
            Event::ItemTooltip { reading } => {
                minerva.on_item_tooltip_reading(reading);
            }
            Event::PlayerShop { at_ms, shop: read, near_miss, same, hover, stale_region } => {
                let read = (read, near_miss);
                let (shop, near_miss) = if stale_region.is_some() { read } else { as_before(&mut shop, read, same) };
                minerva.player_shop_reading_for(ShopReading { shop, hover, at_ms, near_miss }, stale_region);
            }
            Event::StatusBar { at_ms, bar, near_miss, same, stale_region } => {
                let read = (bar, near_miss);
                let (bar, near_miss) = if stale_region.is_some() { read } else { as_before(&mut status_bar, read, same) };
                minerva.status_bar_reading_for(StatusBarReading { bar, at_ms, near_miss }, stale_region);
            }
            Event::PickupLog { reading, stale_region } => {
                minerva.pickup_log_reading_for(reading, stale_region);
            }
            Event::ChooseDealLine { line } => {
                minerva.choose_deal_line(&line);
            }
            Event::DealBadgesSize { size } => {
                minerva.set_deal_badges_size(size);
            }
            Event::PickItem { id } => {
                minerva.pick_item(&id);
            }
            Event::PickCardItem { id } => {
                minerva.pick_card_item(&id);
            }
            Event::OpenCardMeowdbPage { at_ms } => {
                minerva.open_card_meowdb_page(at_ms);
            }
            Event::CloseItemCard {} => {
                minerva.close_item_card();
            }
            Event::EditCardMarketPrice { line } => {
                minerva.edit_card_market_price(&line);
            }
            Event::EnterCardMarket { typed } => {
                minerva.enter_card_market(&typed);
            }
            Event::EditCardMarketLine { line } => {
                minerva.edit_card_market_line(&line);
            }
            Event::EnterCardMarketLine { typed } => {
                minerva.enter_card_market_line(&typed);
            }
            Event::RestoreMarketPrices { prices } => {
                minerva.restore_market_prices(prices);
            }
            Event::ImportMarketPrices { import } => {
                minerva.import_market_prices(import);
            }
            Event::Foreground { foreground } => {
                minerva.on_foreground(foreground);
            }
            Event::HotkeysUnavailable { taken, invalid } => {
                minerva.hotkeys_unavailable(taken, invalid);
            }
            Event::Restore { settings } => {
                minerva.restore(*settings);
            }
            Event::PickGameWindow { title } => {
                minerva.pick_game_window(title);
            }
            Event::WallClock { unix_ms } => {
                minerva.on_wall_clock(unix_ms);
                wall_clock = Some((unix_ms, line.t));
            }
            Event::RestoreBests { bests } => {
                minerva.restore_bests(bests);
            }
            Event::BestsLost {} => {
                minerva.bests_lost();
            }
            Event::RemoveVisit { map, visit, character } => {
                minerva.remove_visit(character.as_deref(), &map, &visit);
            }
            Event::MoveVisit { map, visit, to, from } => {
                minerva.move_visit(from.as_deref(), &map, &visit, &to);
            }
            Event::SetVisitParty { map, visit, party, character } => {
                minerva.set_visit_party(character.as_deref(), &map, &visit, party);
            }
            Event::AssignUnassigned { to } => {
                minerva.assign_unassigned(&to);
            }
            Event::RemoveCharacter { name } => {
                minerva.remove_character(&name);
            }
            Event::MergeCharacters { from, into } => {
                minerva.merge_characters(&from, &into);
            }
            Event::ShowOnLeaderboards { name, on } => {
                minerva.show_on_leaderboards(&name, on);
            }
            Event::SendToLeaderboardsAgain { name } => {
                minerva.send_to_leaderboards_again(&name);
            }
            Event::LeaderboardAnswer { id, answer, .. } => {
                minerva.on_leaderboard_answer(id, answer);
            }
            Event::LinkDiscord {} => {
                minerva.link_discord();
            }
            Event::DismissLinkPrompt {} => {
                minerva.dismiss_link_prompt();
            }
            Event::DiscordLinkAnswer { id, answer } => {
                minerva.on_discord_link_answer(id, answer);
            }
            Event::UpdateAnswer { id, answer } => {
                minerva.on_update_answer(id, answer);
            }
            Event::Build { build } => {
                minerva.on_build(build);
            }
            Event::UpdateDownload { id, answer } => {
                minerva.on_update_download(id, answer);
            }
            Event::UpdateInstallFailed { id, why } => {
                minerva.on_update_install_failed(id, &why);
            }
            Event::Error { from, message } => minerva.note_error(&from, &message),
            Event::BugReport {} => minerva.mark_bug_report(),
        }
    }
    minerva.close_input();
    let replayed: Vec<Option<Line>> = minerva.recorder.trace().lines().map(|line| serde_json::from_str(line).ok()).collect();
    let as_compared = |line: &Line| {
        let answers = line.answers.iter().cloned().map(|answer| match answer {
            Answer::SavedSet { saved: Some(saved) } => {
                Answer::SavedSet { saved: Some(SavedSet { fingerprint: String::new(), ..saved }) }
            }
            Answer::Set { answer: Ok(_) } => Answer::Set { answer: Ok(String::new()) },
            Answer::SavedItems { saved: Some(saved) } => {
                Answer::SavedItems { saved: Some(SavedSet { fingerprint: String::new(), ..saved }) }
            }
            Answer::Items { answer: Ok(_) } => Answer::Items { answer: Ok(String::new()) },
            other => other,
        });
        // What came of a reading, not what its reader came closest with: a near miss is
        // never what a replay diverges over.
        let mut event = line.event.clone();
        if let Event::PlayerShop { near_miss, .. } = &mut event {
            *near_miss = None;
        }
        if let Event::StatusBar { near_miss, .. } = &mut event {
            *near_miss = None;
        }
        let line = Line { event, crop: None, tooltip_crop: None, answers: answers.collect(), ..line.clone() };
        serde_json::to_string(&line).unwrap_or_default()
    };
    let length = recording.lines.len().max(replayed.len());
    let divergence = (0..length).find_map(|index| {
        let recorded = recording.lines.get(index).map(as_compared);
        let replayed = replayed.get(index).map(|line| line.as_ref().map(as_compared).unwrap_or_default());
        (recorded != replayed).then_some(Divergence { index, recorded, replayed })
    });
    Replay { lines: recording.lines.len(), divergence }
}

/// A reading as a replayed line has it: `read`, or what was read `last` if the line says it
/// read the `same`; the last from then on.
fn as_before<T: Clone>(last: &mut T, read: T, same: bool) -> T {
    if !same {
        *last = read;
    }
    last.clone()
}

impl Recording {
    /// The trace as lines to read: what came in and what came of it. Before `from_ms`,
    /// only the lines the core made something of, changed the map, or the player acted
    /// on; from it on, every line but ticks and title readings that changed nothing.
    pub fn timeline(&self, from_ms: u64) -> Vec<String> {
        let mut out = Vec::new();
        let mut title: Option<&TitleLines> = None;
        for line in &self.lines {
            if let Event::Title { lines, same: false, stale_region: None, .. } = &line.event {
                title = lines.as_ref();
            }
            let consequences = line.consequences();
            // The whole capture kept where a reader found nothing, if it is this line's.
            let whole = match line.event {
                Event::PlayerShop { at_ms, stale_region: None, .. } => Some(Region::ClientArea { near_miss: false }.crop_name(at_ms)),
                Event::StatusBar { at_ms, stale_region: None, .. } => Some(Region::StatusBarStrip.crop_name(at_ms)),
                _ => None,
            }
            .filter(|name| self.crops.contains(name));
            let acted = matches!(
                line.event,
                Event::Act { .. }
                    | Event::Press { .. }
                    | Event::Hotkey { .. }
                    | Event::PickCardItem { .. }
                    | Event::OpenCardMeowdbPage { .. }
                    | Event::CloseItemCard {}
                    | Event::EditCardMarketPrice { .. }
                    | Event::EnterCardMarket { .. }
                    | Event::EditCardMarketLine { .. }
                    | Event::EnterCardMarketLine { .. }
                    | Event::ChooseDealLine { .. }
                    | Event::Restore { .. }
                    | Event::RestoreBests { .. }
                    | Event::BestsLost {}
                    | Event::RestoreMarketPrices { .. }
                    | Event::ImportMarketPrices { .. }
                    | Event::RemoveVisit { .. }
                    | Event::MoveVisit { .. }
                    | Event::SetVisitParty { .. }
                    | Event::AssignUnassigned { .. }
                    | Event::RemoveCharacter { .. }
                    | Event::MergeCharacters { .. }
                    | Event::ShowOnLeaderboards { .. }
                    | Event::SendToLeaderboardsAgain { .. }
                    | Event::LeaderboardAnswer { .. }
                    | Event::LinkDiscord {}
                    | Event::DismissLinkPrompt {}
                    | Event::DiscordLinkAnswer { .. }
                    | Event::UpdateAnswer { .. }
                    | Event::Build { .. }
                    | Event::UpdateDownload { .. }
                    | Event::UpdateInstallFailed { .. }
                    | Event::Error { .. }
                    | Event::BugReport {}
            );
            let notable = acted
                || whole.is_some()
                || !line.notes.is_empty()
                || line.map.is_some()
                || !line.uploads.is_empty()
                || line.discord_link.is_some()
                || line.update_check.is_some()
                || line.update_job.is_some()
                || line.answers.iter().any(Answer::is_set);
            let quiet = consequences.is_empty()
                && matches!(
                    line.event,
                    Event::Tick {}
                        | Event::Title { same: true, .. }
                        | Event::PlayerShop { same: true, hover: None, .. }
                        | Event::StatusBar { same: true, .. }
                );
            if notable || (line.t >= from_ms && !quiet) {
                let mut text = format!("{}  {}", clock(line.event.at_ms().unwrap_or(line.t)), line.event.describe(title));
                if !consequences.is_empty() {
                    text += &format!("  →  {}", consequences.join("; "));
                }
                for crop in line.crop.iter().chain(&line.tooltip_crop).chain(&whole) {
                    text += &format!("  [{crop}]");
                }
                out.push(text);
            }
        }
        out
    }
}

impl Line {
    /// What came of the line's input, in words.
    fn consequences(&self) -> Vec<String> {
        let mut out: Vec<String> = self.answers.iter().filter_map(Answer::describe).collect();
        out.extend(self.notes.iter().map(Note::describe));
        if let Some(map) = &self.map {
            out.push(match &map.id {
                Some(id) => {
                    let candidates = if map.candidates > 0 { format!(", one of {} sharing its title", map.candidates) } else { String::new() };
                    format!("map {} · {} ({id}){candidates}", map.street_name, map.map_name)
                }
                None => "no map".to_string(),
            });
        }
        if let Some(changes) = self.grind_tracker.as_ref().and_then(grind_tracker_changes) {
            out.push(changes);
        }
        out.extend(self.uploads.iter().map(|upload| match upload.entries {
            Some(entries) => format!("Map Leaderboards upload {} asked: {:?}'s {entries} entries", upload.id, upload.name),
            None => format!("Map Leaderboards upload {} asked: deleting {:?}'s entries", upload.id, upload.name),
        }));
        out.extend(self.discord_link.as_ref().map(|request| match request.ask {
            DiscordLinkAsk::Start => format!("Discord link request {} asked: open the link page", request.id),
            DiscordLinkAsk::State => format!("Discord link request {} asked: read the link state", request.id),
        }));
        out.extend(self.update_check.map(|id| format!("update check {id} asked")));
        out.extend(self.update_job.map(|job| match job.step {
            UpdateStep::Download => format!("update download {} asked", job.id),
            UpdateStep::Install { relaunch: true } => format!("update download {}'s install asked, relaunching", job.id),
            UpdateStep::Install { relaunch: false } => format!("update download {}'s install asked, not relaunching", job.id),
        }));
        out
    }
}

impl Answer {
    fn is_set(&self) -> bool {
        matches!(self, Answer::SavedSet { .. } | Answer::Set { .. } | Answer::SavedItems { .. } | Answer::Items { .. })
    }

    fn describe(&self) -> Option<String> {
        Some(match self {
            Answer::SavedSet { saved: Some(saved) } => format!("OSMS's set saved earlier ({}){}", saved.fingerprint, by_another(saved)),
            Answer::SavedSet { saved: None } => "no OSMS set saved earlier".to_string(),
            Answer::Set { answer: Ok(fingerprint) } => format!("OSMS's set fetched ({fingerprint})"),
            Answer::Set { answer: Err(e) } => format!("OSMS's set: {e}"),
            Answer::Picture { answer: Ok(_), .. } => return None,
            Answer::Picture { path, answer: Err(e) } => format!("picture {path}: {e}"),
            Answer::ItemSearch { name, answer: Ok(answer) } => format!("MeowDB's search for {name:?}: {} items", answer.hits.len()),
            Answer::ItemSearch { name, answer: Err(e) } => format!("MeowDB's search for {name:?}: {e}"),
            Answer::SavedItems { saved: Some(saved) } => format!("OSMS's item files saved earlier ({}){}", saved.fingerprint, by_another(saved)),
            Answer::SavedItems { saved: None } => "no OSMS item files saved earlier".to_string(),
            Answer::Items { answer: Ok(fingerprint) } => format!("OSMS's item files fetched ({fingerprint})"),
            Answer::Items { answer: Err(e) } => format!("OSMS's item files: {e}"),
        })
    }
}

impl Note {
    /// Whether the reading it came of is one to look at again: a misread, or one taken as
    /// a level-up, a death, another character or a level learned wrong. Not a Session
    /// starting or a Visit, which every run has.
    fn is_unusual(&self) -> bool {
        !matches!(self, Note::SessionStarted | Note::Visit)
    }

    /// In words.
    fn describe(&self) -> String {
        match self {
            Note::SessionStarted => "the Session started".to_string(),
            Note::LevelUp { gain, level: l } => format!("level-up: {} EXP counted, from a level of {}", signed(*gain), level(*l)),
            Note::Death { exp_lost } => format!("death: {} EXP lost", thousands(*exp_lost)),
            Note::NoLevelAllows => "misread: no level's total allows it; set aside".to_string(),
            Note::Misfit { count, level: l } => {
                format!("doesn't fit the level learned ({}): set aside, {count} in a row", level(*l))
            }
            Note::Relearned { from, to } => format!("the level was learned wrong: {} now, not {}", level(*to), level(*from)),
            Note::CharacterSwitch { level: l } => format!("another character, a level of {}: a new Session", level(*l)),
            Note::NameSwitch { name } => format!("the Status Bar names another character, {name:?}: a new Session"),
            Note::LevelSwitch { level } => {
                format!("the Status Bar shows another character's level, {level}, and no name: a new Session")
            }
            Note::NameUnconfirmed { name, count } => {
                format!("the Status Bar's name read as {name:?}, {count} in a row: not yet the character")
            }
            Note::NameUnconfirmedBar => "the Status Bar names another, not yet for sure: set aside".to_string(),
            Note::NameMisread { name } => format!("the Status Bar's name misread as {name:?} while in sight: set aside"),
            Note::LevelShown { level } => format!("the Status Bar shows level {level}: the next reading is in it"),
            Note::LevelUpUncounted => "the Status Bar's level-up couldn't be counted: the new level is learned afresh".to_string(),
            Note::LevelChanged { from, to } => {
                format!("the Status Bar shows level {to}, not {from}: the level is learned afresh")
            }
            Note::Unseen => "the bar went unread over 2 minutes: taken without counting the change".to_string(),
            Note::Visit => "a new Visit".to_string(),
            Note::ItemCutShort { name } => format!("Pickup Log item {name:?} starts more than one item's name: tallied as shown"),
            Note::ItemUnmatched { name, lines } => {
                format!("Pickup Log item {name:?} matches no item, read on {lines} line(s): tallied from the second")
            }
        }
    }
}

impl Event {
    /// A reading left as for a region no longer captured: the reading as if it had been
    /// taken in, and the region its capture was of.
    fn left_as_stale(&self) -> Option<(Event, ScreenRect)> {
        match self.clone() {
            Event::Title { at_ms, lines, found, same, stale_region: Some(region) } => {
                Some((Event::Title { at_ms, lines, found, same, stale_region: None }, region))
            }
            Event::PlayerShop { at_ms, shop, near_miss, same, hover, stale_region: Some(region) } => {
                Some((Event::PlayerShop { at_ms, shop, near_miss, same, hover, stale_region: None }, region))
            }
            Event::StatusBar { at_ms, bar, near_miss, same, stale_region: Some(region) } => {
                Some((Event::StatusBar { at_ms, bar, near_miss, same, stale_region: None }, region))
            }
            Event::PickupLog { reading, stale_region: Some(region) } => {
                Some((Event::PickupLog { reading, stale_region: None }, region))
            }
            _ => None,
        }
    }

    /// When a reading's frame was captured.
    fn at_ms(&self) -> Option<u64> {
        match self {
            Event::Title { at_ms, .. }
            | Event::ExpBar { at_ms, .. }
            | Event::PlayerShop { at_ms, .. }
            | Event::StatusBar { at_ms, .. }
            | Event::OpenCardMeowdbPage { at_ms } => Some(*at_ms),
            Event::PickupLog { reading, .. } => Some(reading.at_ms),
            _ => None,
        }
    }

    /// In words; a title read the same as the one before as what that one read, `title`.
    fn describe(&self, title: Option<&TitleLines>) -> String {
        if let Some((read, region)) = self.left_as_stale() {
            let ScreenRect { x, y, width, height } = region;
            return format!(
                "{}, for a region no longer captured ({width}x{height} at ({x}, {y})): left",
                read.describe(title)
            );
        }
        match self {
            Event::Tick {} => "tick".to_string(),
            Event::Title { lines: Some(lines), .. } => format!("title read {:?} / {:?}", lines.street, lines.map),
            Event::Title { same: true, .. } => match title {
                Some(lines) => format!("title read {:?} / {:?} again", lines.street, lines.map),
                None => "title unreadable again".to_string(),
            },
            Event::Title { found: None, .. } => "no title found".to_string(),
            Event::Title { .. } => "title unreadable".to_string(),
            Event::ExpBar { bar: Some(b), place, .. } => format!("EXP bar read {}{}", bar(b), remembered(place)),
            Event::ExpBar { bar: None, place: None, .. } => "EXP bar not whole".to_string(),
            Event::ExpBar { bar: None, place, .. } => format!("EXP bar not whole{}", remembered(place)),
            Event::GameWindow { game: Some(game) } => {
                let c = game.client;
                format!("game window: client {}x{} at ({}, {})", c.width, c.height, c.x, c.y)
            }
            Event::GameWindow { game: None } => "no game window".to_string(),
            Event::Foreground { foreground } => format!("in front: {foreground:?}"),
            Event::PanelSizes { sizes } => {
                let (e, c) = (sizes.expanded, sizes.collapsed);
                format!("panel sizes {}x{}, collapsed {}x{}", e.width, e.height, c.width, c.height)
            }
            Event::OwlBubbleSize { size } => format!("Owl Bubble size {}x{}", size.width, size.height),
            Event::ItemCardSize { size } => format!("Item Card size {}x{}", size.width, size.height),
            Event::DealBadgesSize { size } => format!("Deal Badges size {}x{}", size.width, size.height),
            Event::GrindTrackerSize { size } => format!("Grind Tracker size {}x{}", size.width, size.height),
            Event::PersonalBestsSize { size } => format!("Personal Bests window size {}x{}", size.width, size.height),
            Event::SettingsSize { size } => format!("Settings window size {}x{}", size.width, size.height),
            Event::RebindKey { press } => format!("key pressed in the Settings window {}", json(press)),
            Event::SearchPick { id } => format!("search picked {id}"),
            Event::SearchPickDestination { id } => format!("search picked map {id} as the Destination"),
            Event::SearchPickItem { id } => format!("search picked item {id}"),
            Event::PickItem { id } => format!("item picked {id}"),
            Event::PickCardItem { id } => format!("Item Card's item picked {id}"),
            Event::OpenCardMeowdbPage { .. } => "Item Card's name clicked".to_string(),
            Event::CloseItemCard {} => "Item Card closed".to_string(),
            Event::EditCardMarketPrice { line } => format!("Item Card's price on market line {line} clicked"),
            Event::EnterCardMarket { typed } => format!("Item Card's market price entered {typed:?}"),
            Event::EditCardMarketLine { line } => format!("Item Card's market line {line} name clicked"),
            Event::EnterCardMarketLine { typed } => format!("Item Card's market line name entered {typed:?}"),
            Event::RestoreMarketPrices { prices } => {
                let names: Vec<String> =
                    prices.lines.iter().map(|line| format!("{:?} ({} prices)", line.name, line.prices.len())).collect();
                format!("market prices restored: {}", if names.is_empty() { "no lines".into() } else { names.join(", ") })
            }
            Event::ImportMarketPrices { import } => {
                let names: Vec<String> =
                    import.lines.iter().map(|line| format!("{:?} ({} prices)", line.id, line.prices.len())).collect();
                format!("market prices imported: {}", if names.is_empty() { "no lines".into() } else { names.join(", ") })
            }
            Event::HotkeysUnavailable { taken, invalid } => {
                format!("hotkeys unavailable: taken {taken:?}, invalid {invalid:?}")
            }
            Event::PickGameWindow { title } => format!("game window picked by hand: {title:?}"),
            Event::Act { action } => format!("action {}", json(action)),
            Event::Press { control } => format!("pressed {}", json(control)),
            Event::Hotkey { pressed } => format!("hotkey {pressed}"),
            Event::SearchQuery { query } => format!("search typed {query:?}"),
            Event::ItemTooltip { reading } => format!("Item Tooltip read {}", json(reading)),
            Event::PlayerShop { hover: Some(hover), same, shop, near_miss, at_ms, .. } => {
                let read = Event::PlayerShop {
                    at_ms: *at_ms,
                    shop: shop.clone(),
                    near_miss: *near_miss,
                    same: *same,
                    hover: None,
                    stale_region: None,
                }
                .describe(title);
                let tooltip = match &hover.tooltip {
                    ItemTooltip::Found { name: Some(name), .. } => format!("its tooltip {name:?}"),
                    ItemTooltip::Found { name: None, .. } => "its tooltip unreadable".to_string(),
                    ItemTooltip::None => "no tooltip".to_string(),
                };
                format!("{read}; the pointer on it at {}, {}, {tooltip}", hover.pointer.x, hover.pointer.y)
            }
            Event::PlayerShop { same: true, .. } => "Player Shop read as before".to_string(),
            Event::PlayerShop { shop: None, near_miss: None, .. } => "no Player Shop".to_string(),
            Event::PlayerShop { shop: None, near_miss: Some(near), .. } => {
                let ScreenRect { x, y, width, height } = near.area;
                format!(
                    "no Player Shop; nearest, {} band rows {width} px wide at ({x}, {y}), {width}x{height}, at {}% brightness",
                    near.rows, near.brightness_hundredths
                )
            }
            Event::PlayerShop { shop: Some(shop), .. } => {
                let rows: Vec<String> = shop
                    .rows
                    .iter()
                    .map(|row| {
                        if row.covered {
                            return "covered".to_string();
                        }
                        let name = row.name.as_deref().map_or("?".to_string(), |name| format!("{name:?}"));
                        let tier = row.tier.map_or(String::new(), |tier| format!(" (a {tier:?} scroll's picture)"));
                        let price = row.price.map_or("?".to_string(), |p| format!("{} for {}", p.quantity, thousands(p.mesos)));
                        format!("{name}{tier} {price}")
                    })
                    .collect();
                format!("Player Shop read: {}", if rows.is_empty() { "no listings".into() } else { rows.join(", ") })
            }
            Event::StatusBar { same: true, .. } => "Status Bar read as before".to_string(),
            Event::PickupLog { reading, .. } => format!("Pickup Log: {:?}; rows {:?}, shift {}; arrivals {:?}{}", reading.seen, reading.rows, reading.shift, reading.arrived, if reading.chose { "; chose between shifts" } else { "" }),
            Event::StatusBar { bar: None, near_miss: None, .. } => "no Status Bar".to_string(),
            Event::StatusBar { bar: None, near_miss: Some(near), .. } => {
                let ScreenRect { x, y, width, height } = near.area;
                let boxed = near.box_height.map_or("no LV box round them".to_string(), |h| format!("in an LV box {h} px tall"));
                format!("no Status Bar; nearest, {} level tiles at ({x}, {y}), {width}x{height}, {boxed}", near.tiles)
            }
            Event::StatusBar { bar: Some(bar), .. } => {
                let or_unread = |part: Option<String>| part.unwrap_or_else(|| "?".to_string());
                let level = or_unread(bar.level.map(|level| level.to_string()));
                let job = or_unread(bar.job.as_ref().map(|job| format!("{job:?}")));
                let name = or_unread(bar.name.as_ref().map(|name| format!("{name:?}")));
                format!("Status Bar read: level {level}, {job}, {name}")
            }
            Event::ChooseDealLine { line } => format!("Item Card's market line {line} chosen as the Deal Line"),
            Event::Restore { .. } => "settings restored".to_string(),
            Event::RestoreBests { bests } => {
                let maps = bests.unassigned.len() + bests.characters.iter().map(|c| c.maps.len()).sum::<usize>();
                format!("Personal Bests restored ({maps} maps, {} Characters)", bests.characters.len())
            }
            Event::BestsLost {} => "Personal Bests file unreadable: uploads to the Map Leaderboards held".to_string(),
            Event::RemoveVisit { map, visit, character } => {
                let whose = character.as_ref().map_or("the Unassigned".to_string(), |name| format!("{name:?}'s"));
                format!("Visit removed from {whose} Visit History of {map}: {}", visit_words(visit))
            }
            Event::MoveVisit { map, visit, to, from } => {
                let whose = from.as_ref().map_or("Unassigned".to_string(), |name| format!("{name:?}'s"));
                format!("{whose} Visit on {map} moved to {to:?}: {}", visit_words(visit))
            }
            Event::SetVisitParty { map, visit, party, character } => {
                let whose = character.as_ref().map_or("Unassigned".to_string(), |name| format!("{name:?}'s"));
                let to = if *party { "the Party Bests" } else { "the Solo Bests" };
                format!("{whose} Visit on {map} moved to {to}: {}", visit_words(visit))
            }
            Event::AssignUnassigned { to } => format!("Every Unassigned Visit given to {to:?}"),
            Event::RemoveCharacter { name } => format!("Character {name:?} removed, with their bests"),
            Event::MergeCharacters { from, into } => format!("Character {from:?} merged into {into:?}"),
            Event::ShowOnLeaderboards { name, on: true } => format!("{name:?} shown on the Map Leaderboards"),
            Event::ShowOnLeaderboards { name, on: false } => format!("{name:?} taken off the Map Leaderboards"),
            Event::SendToLeaderboardsAgain { name } => format!("{name:?} sent to the Map Leaderboards again"),
            Event::LeaderboardAnswer { id, name, answer } => {
                let said = match answer {
                    LeaderboardAnswer::Accepted { rejected } if rejected.is_empty() => "taken".to_string(),
                    LeaderboardAnswer::Accepted { rejected } => {
                        let rejected: Vec<String> = rejected
                            .iter()
                            .map(|r| match r.reason {
                                RejectedReason::OverCap { cap } => format!("{} band {} over the cap of {}", r.map, r.band, thousands(cap)),
                                RejectedReason::Removed {} => format!("{} band {} removed", r.map, r.band),
                            })
                            .collect();
                        format!("taken but for {}", rejected.join(", "))
                    }
                    LeaderboardAnswer::Deleted {} => "deleted".to_string(),
                    LeaderboardAnswer::Claimed {} => "the name is claimed by another key".to_string(),
                    LeaderboardAnswer::RateLimited { retry_after_secs } => format!("rate-limited for {retry_after_secs} s"),
                    LeaderboardAnswer::Failed { why } => format!("failed: {why}"),
                };
                format!("Map Leaderboards upload {id} for {name:?}: {said}")
            }
            Event::LinkDiscord {} => "Link Discord pressed".to_string(),
            Event::DismissLinkPrompt {} => "the prompt to link Discord dismissed".to_string(),
            Event::DiscordLinkAnswer { id, answer } => {
                let said = match answer {
                    DiscordLinkAnswer::Opened {} => "the link page opened".to_string(),
                    DiscordLinkAnswer::Linked { username } => format!("linked to {username:?}"),
                    DiscordLinkAnswer::NotLinked {} => "not linked".to_string(),
                    DiscordLinkAnswer::Failed { why } => format!("failed: {why}"),
                };
                format!("Discord link request {id}: {said}")
            }
            Event::UpdateAnswer { id, answer: UpdateAnswer::Latest { version } } => {
                format!("update check {id}: the latest version is {version}")
            }
            Event::UpdateAnswer { id, answer: UpdateAnswer::Failed { why } } => format!("update check {id} failed: {why}"),
            Event::Build { build: Build::Nsis } => "build installed with the setup .exe: updates in place".to_string(),
            Event::Build { build: Build::Msi } => "build installed with the .msi: updates in place".to_string(),
            Event::Build { build: Build::NotifyOnly } => "portable or dev build: updates are notify-only".to_string(),
            Event::UpdateDownload { id, answer: DownloadAnswer::Downloaded { version } } => {
                format!("update download {id}: {version} downloaded, its signature checked")
            }
            Event::UpdateDownload { id, answer: DownloadAnswer::Failed { why } } => {
                format!("update download {id} failed: {why}")
            }
            Event::UpdateInstallFailed { id, why } => format!("update download {id}'s installer didn't start: {why}"),
            Event::WallClock { unix_ms } => format!("wall clock {unix_ms} (ms since 1970)"),
            Event::Error { from, message } => format!("{from} error: {message}"),
            Event::BugReport {} => "Bug Report saved".to_string(),
        }
    }
}

/// The Grind Tracker's changed fields that say something, in words; `None` if none do
/// (the bar's reading, say, which its line has).
fn grind_tracker_changes(changed: &Map<String, Value>) -> Option<String> {
    let rate = |value: &Value| value["expPerHour"].as_i64().map(|rate| format!("{} EXP/h", signed(rate)));
    let words: Vec<String> = changed
        .iter()
        .filter_map(|(key, value)| match key.as_str() {
            "state" => value.as_str().map(|state| format!("state {state}")),
            "rolling" => Some(format!("Last 5 min {}", rate(value).unwrap_or("—".into()))),
            "sessionAverage" => Some(format!("Session {}", rate(value).unwrap_or("—".into()))),
            "timeToLevelBadge" => Some(format!("Time to Level {}", value.as_str().unwrap_or("—"))),
            "deaths" => Some(format!("deaths {}", value["count"])),
            "personalBest" => Some(format!("Best {}", value.as_u64().map_or("—".into(), |best| format!("{} EXP/h", thousands(best))))),
            "newBest" => Some(format!("new best {value}")),
            "ceilingShareHundredths" => Some(format!("{} of the ceiling", value.as_u64().map_or("—".into(), |h| percent(h as u32)))),
            "characterSwitched" if value.as_bool() == Some(true) => Some("character switched".to_string()),
            "resetUndoable" => Some(format!("reset undoable {value}")),
            _ => None,
        })
        .collect();
    (!words.is_empty()).then(|| format!("Grind Tracker: {}", words.join(", ")))
}

/// What the Grind Tracker shows, in words.
fn grind_tracker(tracking: &ExpTracking) -> String {
    if tracking.state == TrackingState::Off {
        return "off".to_string();
    }
    let rate = |rate: &Option<crate::ExpRate>| rate.map_or("—".to_string(), |r| format!("{} EXP/h", signed(r.exp_per_hour)));
    let mut out = format!(
        "{:?}; Last 5 min {}; Session {}; Time to Level {}; deaths {} ({} EXP)",
        tracking.state,
        rate(&tracking.rolling),
        rate(&tracking.session_average),
        tracking.time_to_level_badge.as_deref().unwrap_or("—"),
        tracking.deaths.count,
        thousands(tracking.deaths.exp_lost)
    );
    if let Some(b) = &tracking.bar {
        out += &format!("; EXP bar {}", bar(b));
    }
    if let Some(best) = tracking.personal_best {
        out += &format!("; Best {} EXP/h", thousands(best));
    }
    out
}

fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Capture time as `h:mm:ss` since launch.
fn clock(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// A Visit's best in words: "7,200 EXP/h, level 23 \"Magician\", set at 1791547200000".
fn visit_words(visit: &VisitBest) -> String {
    let set = visit.set_at_unix_ms.map_or("no date".to_string(), |at| format!("set at {at}"));
    let level = visit.level.map(|level| format!(", level {level}")).unwrap_or_default();
    let job = visit.job.as_ref().map(|job| format!(" {job:?}")).unwrap_or_default();
    let kind = if visit.kind.is_party() { ", a Party Best" } else { "" };
    format!("{} EXP/h{level}{job}{kind}, {set}", thousands(visit.exp_per_hour))
}

fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn signed(n: i64) -> String {
    if n < 0 { format!("−{}", thousands(n.unsigned_abs())) } else { thousands(n as u64) }
}

fn percent(hundredths: u32) -> String {
    format!("{}.{:02}%", hundredths / 100, hundredths % 100)
}

/// ", where it was last found" for EXP figures read where they were remembered.
fn remembered(place: &Option<ExpBarPlace>) -> &'static str {
    if place.is_some_and(|place| place.remembered) { ", where it was last found" } else { "" }
}

fn bar(bar: &ExpBar) -> String {
    format!("{} [{}]", thousands(bar.exp), percent(bar.percent_hundredths))
}

fn level(level: Level) -> String {
    match level.hi {
        Some(hi) => format!("{}–{} EXP", thousands(level.lo), thousands(hi)),
        None => format!("at least {} EXP", thousands(level.lo)),
    }
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// How a saved set's line ends: saying another version saved it, if one did.
fn by_another(saved: &SavedSet) -> &'static str {
    if saved.this_version { "" } else { ", saved by another version" }
}
