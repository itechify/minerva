//! Minerva shell: a thin adapter around `minerva-core`. It finds the game window,
//! captures the Minimap Title (the whole client area while it looks for it, ADR 0021) and the
//! bottom of the client area for the Status Bar (and,
//! for Item Lookup, the client area with the pointer), feeds frames, hotkeys and drags to
//! the core and renders the panel state in an overlay on top of everything (detached, with a taskbar entry), the
//! Grind Tracker in a window of its own beside it, the Personal Bests window over the game
//! while the player has it open, and the Item Lookup alert at the top centre of the game
//! window: all four only while the game or one of our windows is in front. All logic
//! lives in the core.

pub mod capture;
pub mod gamewin;
mod bests;
mod bests_window;
mod bug_reports;
mod deal_badges;
mod errors;
mod files;
mod grind_tracker;
mod hotkeys;
mod http;
mod item_card;
mod item_lookup;
mod leaderboards;
mod market_prices;
mod meowdb;
mod old_caches;
pub mod osms;
mod panel_window;
mod pictures;
mod search;
mod settings;
mod settings_window;
mod setup;
mod sizemove;
pub mod source;
mod tray;
mod updates;
mod view;

use minerva_core::{
    Action, DEAL_FINDER_CAPTURE_MS, Minerva, ExpBarReader, ExpBarReading, ExpFigures, GameWindow, ItemCapture, ItemTooltipReader,
    ItemTooltipReading, KeyPress, Layer, LeaderboardAnswer, DiscordLinkAnswer, MIN_PANEL_WIDTH, PanelControl, PanelSizes, PlacementMode, PanelState, PlayerShopReader,
    Request, ScreenPoint, ScreenRect, ShopCapture, ShopReading, StatusBarCapture, StatusBarReader, StatusBarReading,
    TITLE_READ_MS, TITLE_SEARCH_MS, TitleCapture, TitleReader, TitleReading, UpdateAnswer, VisitBest,
};
use source::WebSource;
use errors::error;
use item_card::OFF_SCREEN_SETTLE;
use panel_window::{PanelWindow, keep_on_top, physical, place};
use setup::SetupWindows;
use sizemove::{Gesture, GestureTracker, SizeMove};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};
use tauri_plugin_opener::OpenerExt;

/// Minimap Title capture rate where it was found: ~3 Hz keeps a map change within about a
/// second...
const TICK: Duration = Duration::from_millis(TITLE_READ_MS);
/// ...and while it is looked for in the whole client area (the core's pace).
const TITLE_SEARCH_TICK: Duration = Duration::from_millis(TITLE_SEARCH_MS);
/// Status Bar capture rate: EXP tracking needs its figures about once a second.
const EXP_BAR_TICK: Duration = Duration::from_millis(1000);
/// Deal Finder capture rate, while it is on (the core's pace).
const DEAL_FINDER_TICK: Duration = Duration::from_millis(DEAL_FINDER_CAPTURE_MS);
/// Game window and foreground check rate: a docked panel follows a moving game window,
/// and our windows show within ~50 ms of the game coming to the front, and are out of
/// sight within ~50 ms of another app doing so (their watchers check at this rate too).
pub(crate) const WINDOW_TICK: Duration = Duration::from_millis(50);
/// How often the `market-imports` folder is looked in (ADR 0013): a tool's prices show
/// within a couple of seconds.
const IMPORT_TICK: Duration = Duration::from_secs(2);
/// Settings are saved once they have stopped changing for this long (after a drag).
const SAVE_DELAY: Duration = Duration::from_millis(500);
/// The overlay's size in CSS px (logical); `src/styles.css` lays out to fit. The
/// expanded panel is as wide as the core says (the player sets it by a bottom corner),
/// and its height follows its content, as the overlay reports it; the core keeps it
/// within the space where the panel is placed. It is never taller than
/// [`MAX_EXPANDED_HEIGHT`]: past that the route or the map list scrolls.
const EXPANDED_HEIGHT: f64 = 72.0;
const MIN_EXPANDED_HEIGHT: f64 = 44.0;
const MAX_EXPANDED_HEIGHT: f64 = 600.0;
const COLLAPSED: (f64, f64) = (44.0, 44.0);

/// What the core loop is asked to do.
#[derive(Debug, Clone)]
pub(crate) enum Command {
    /// Something the player did in the overlay or the setup window, for the core.
    Act(Action),
    /// The player pressed this hotkey (its accelerator string), for the core.
    Hotkey(String),
    /// The player clicked one of the icons at the top of the panel, for the core.
    Press(PanelControl),
    /// The overlay's query box now holds this, for the core.
    SearchQuery(String),
    /// The player pressed this key on the Settings window, for the core.
    RebindKey(KeyPress),
    /// The player clicked the search view's mob (in the matches) or map (in the mob's
    /// map list) with this ID.
    SearchPick(String),
    /// The player clicked the map with this ID in the search view's matches.
    SearchPickDestination(String),
    /// The player clicked the item with this ID in the search view's matches.
    SearchPickItem(String),
    /// The player clicked the name of the listed mob with this ID in the panel.
    OpenMobPage(String),
    /// The player clicked the name of the map with this ID in the Personal Bests window.
    OpenBestsMapPage(String),
    /// The player removed the Visit whose best was `visit` from `map`'s Visit History, the
    /// Character's named `character` (`None`: the Unassigned one), in the Personal Bests window.
    RemoveVisit { character: Option<String>, map: String, visit: VisitBest },
    /// The player moved the Visit whose best was `visit` on `map`, the Character's named `from`
    /// (`None`: an Unassigned one), to the Character named `to`, in the Personal Bests window.
    MoveVisit { from: Option<String>, map: String, visit: VisitBest, to: String },
    /// The player gave the Character with this name every Unassigned Visit, in the Personal
    /// Bests window.
    AssignUnassigned(String),
    /// The player removed the Character with this name, in the Personal Bests window.
    RemoveCharacter(String),
    /// The player merged the Character named `from` into the one named `into`, in the
    /// Personal Bests window.
    MergeCharacters { from: String, into: String },
    /// The player turned "Share" on the Map Leaderboards on (`on`) or off for the Character
    /// named `name`, in the Personal Bests window.
    ShowOnLeaderboards { name: String, on: bool },
    /// The player sent the Character with this name to the Map Leaderboards again, in the
    /// Personal Bests window.
    SendToLeaderboardsAgain(String),
    /// The player clicked the Map Leaderboard link of the map with ID `id` among the bests of
    /// the Character named `character` (`None`: the Unassigned ones), in the Personal Bests
    /// window.
    OpenBestsLeaderboard { character: Option<String>, id: String },
    /// The Map Leaderboards answered the core's upload `id` (or it couldn't be made).
    LeaderboardAnswer { id: u64, answer: LeaderboardAnswer },
    /// The player pressed Link Discord in the Personal Bests window.
    LinkDiscord,
    /// The player said not now to the Personal Bests window's prompt to link Discord.
    DismissLinkPrompt,
    /// The core's request `id` about linking came to `answer`; a link started, with the link
    /// page to open first, its link token in it.
    DiscordLinkAnswer { id: u64, answer: DiscordLinkAnswer, open: Option<String> },
    /// The core's check `id` for a newer version came to this.
    UpdateAnswer { id: u64, answer: UpdateAnswer },
    /// The core's download `id` of a newer version came to this: its installer, checked
    /// against the public key, or why not.
    UpdateDownload { id: u64, fetched: Result<updates::Fetched, String> },
    /// The player picked the item with this MeowDB ID from the ones an Item Lookup found
    /// sharing the name it read.
    PickItem(String),
    /// The page in our window with this label (the overlay's, the Grind Tracker's, the
    /// Personal Bests', the Item Lookup alert's or the Item Card's) is this tall (CSS px).
    Height(String, f64),
    /// The overlay window moved here (its outer top-left corner). The player's move only
    /// during a drag; otherwise the shell's own, or the system's (a DPI or monitor
    /// change).
    Moved(PhysicalPosition<i32>),
    /// The player pressed the mouse to drag the overlay (by its header or collapsed icon)
    /// or to resize it (by a corner grip or an edge), so the moves or sizes that follow
    /// are theirs.
    GestureStarted(Gesture),
    /// The overlay window is now this size. The player's resize only during a resize;
    /// otherwise the shell's own.
    Resized(PhysicalSize<u32>),
    /// Windows' move or size loop on the overlay started: a drag or resize is running.
    SizeMoveStarted,
    /// Windows' move or size loop on the overlay ended: the player let go of the mouse.
    SizeMoveEnded,
    /// Windows told of a drag of our window with this label (the Grind Tracker's, the
    /// Personal Bests' or the Item Card's).
    SizeMove(&'static str, SizeMove),
    /// The player clicked the Item Card's ×.
    CloseItemCard,
    /// The player picked the item with this game ID from those the Item Card lists.
    PickCardItem(String),
    /// The player clicked the Item Card's name: open the item's MeowDB page.
    OpenCardMeowdbPage,
    /// The player clicked the Item Card's MesoWatch button: open MesoWatch.
    OpenCardMesowatch,
    /// The player clicked the price on the Item Card's Market Line with this ID.
    EditCardMarketPrice(String),
    /// The player pressed Enter in the Item Card's price field, holding this.
    EnterCardMarket(String),
    /// The player clicked the name of the Item Card's Market Line with this ID.
    EditCardMarketLine(String),
    /// The player pressed Enter in the Item Card's line name field, holding this.
    EnterCardMarketLine(String),
    /// The player chose the Item Card's Market Line with this ID as the Deal Line.
    ChooseDealLine(String),
    /// The player chose Quit in the tray icon's menu: save the settings and exit.
    Quit,
    /// The player chose Bug Report in the tray icon's menu: save one, and open the bug
    /// form for it.
    BugReport,
    /// The shell or one of our windows (`from`) ran into an error, for the Bug Report.
    Error { from: String, message: String },
    /// The player started Minerva again; that one exited, and this one shows itself.
    LaunchedAgain,
    /// The player picked the game window with this title by hand, in setup: it is looked for
    /// at once.
    PickGameWindow(String),
    /// A watcher took one of our windows out of sight, with another app in front, while
    /// the core still showed it: check the game window and the foreground now.
    CheckForeground,
    /// The reader is done with a capture for the Minimap Title: the capture (which says
    /// where it was of) and what it read, or `None` if the capture failed.
    Read { reading: Option<(Box<TitleCapture>, TitleReading)> },

    /// The reader is done with an Item Card's or Item Lookup's capture: the capture, and
    /// what it read.
    ReadItemTooltip { capture: Box<ItemCapture>, reading: ItemTooltipReading },
    /// The reader is done with a Deal Finder capture (of the game's client area): the
    /// capture and what it read, or `None` if the capture failed.
    ReadShop { reading: Option<(Box<ShopCapture>, ShopReading)> },
    /// The reader is done with a capture of the bottom of the game's client area for the
    /// Status Bar: the capture, what it read, and what its EXP figures read as (if they were
    /// asked for), or `None` if the capture failed.
    ReadStatusBar { reading: Option<(Box<StatusBarCapture>, StatusBarReading, Option<ExpBarReading>)> },
}

/// The overlay's drags and resizes, as the core loop hears them.
fn overlay_size_move(what: SizeMove) -> Command {
    match what {
        SizeMove::Pressed(gesture) => Command::GestureStarted(gesture),
        SizeMove::LoopStarted => Command::SizeMoveStarted,
        SizeMove::LoopEnded => Command::SizeMoveEnded,
    }
}

/// Something the player did in one of our pages, for the core: any [`Action`], as JSON in
/// its own shape (`"searchEnter"`, `{ "setGrindTracker": { "on": true } }`). Plain (not
/// async), so commands run in the order they come: a quick type-then-Enter reaches the core
/// as typed.
#[tauri::command]
fn act(action: Action, commands: tauri::State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(action));
}

/// A page reports how tall its content is, so its window fits it: the overlay's expanded
/// panel, the Grind Tracker, the Personal Bests, the Settings, the Item Lookup alert and the
/// Item Card.
#[tauri::command]
fn page_height(window: tauri::Window, height: f64, commands: tauri::State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Height(window.label().to_string(), height));
}

/// Where OSMS's file set and pictures, and MeowDB's search answers, are cached. WebView2
/// shares the app cache folder, hence the subfolder.
fn cache_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_cache_dir()?.join("cache"))
}

/// Capture the game's whole client area for the Minimap Title, as the app does while it looks
/// for it (ADR 0021): for the probe.
pub fn capture_title(client: ScreenRect, at_ms: u64) -> Result<TitleCapture, String> {
    let frame = capture::frame(client, || at_ms)?;
    Ok(TitleCapture { frame, origin: ScreenPoint { x: client.x, y: client.y }, ours: Vec::new() })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut context = tauri::generate_context!();
    // An http feed only on this computer, for the update smoke test (ADR 0019).
    updates::allow_local_feed(context.config_mut());
    tauri::Builder::default()
        // First, so a second launch exits before it sets anything up: only one
        // Minerva runs, and the running one shows itself instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(commands) = app.try_state::<Sender<Command>>() {
                let _ = commands.send(Command::LaunchedAgain);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        // Checks for a newer version (ADR 0019), driven from Rust only: no capability.
        .plugin(tauri_plugin_updater::Builder::new().build())
        // The views last sent to our pages, managed before the windows in tauri.conf.json
        // are made: a page loads and asks for its view while the rest are still being made,
        // before `setup` below runs.
        .manage(view::PageViews::default())
        // OSMS's pictures, such as the Mob Cards' animations, from our saved copies.
        .register_uri_scheme_protocol(pictures::SCHEME, |ctx, request| match cache_dir(ctx.app_handle()) {
            Ok(dir) => pictures::serve(&dir, &request),
            Err(_) => tauri::http::Response::builder().status(500).body(Vec::new()).unwrap_or_default(),
        })
        .setup(|app| {
            let overlay = app.get_webview_window("overlay").expect("overlay window in tauri.conf.json");
            // Not content-protected: the panel shows in the player's screenshots and
            // streams. The core keeps it off the Minimap Title instead, so our own capture
            // never reads it.
            // Clicks pass through until there is a panel to show; then it is always
            // clickable, but never activated by a click (`focusable: false` in
            // tauri.conf.json is WS_EX_NOACTIVATE), so the game keeps the keyboard.
            overlay.set_ignore_cursor_events(true)?;

            // OSMS's file set and pictures are kept on disk (ADR 0008), and so are MeowDB's
            // search answers (ADR 0007).
            let cache = cache_dir(app.handle())?;
            // Off the core loop: what earlier versions cached and nothing reads now, and
            // the pictures unused for 30 days.
            let swept = cache.clone();
            std::thread::Builder::new().name("cache-sweeper".into()).spawn(move || {
                old_caches::delete(&swept);
                pictures::sweep(&swept);
            })?;
            let source = WebSource::new(cache)?;
            let client = source.client();
            let mut minerva = Minerva::new(source);
            let settings_path = settings::path(app.handle())?;
            // The core brings older settings up to date (hotkeys included).
            minerva.restore(settings::load(&settings_path));
            // The Personal Bests earlier launches set, beside the settings: a small file,
            // read before anything is captured, so no best is set before they are back.
            // After the settings: one there but unreadable holds the Map Leaderboards' uploads
            // of the Characters they show there.
            let bests_file = bests::restore(&mut minerva, bests::BestsFile::load(bests::path(app.handle())?), &settings_path);
            // The market prices the player keeps on Item Cards, beside them.
            let (prices_file, saved_prices) =
                market_prices::MarketPricesFile::load(market_prices::path(app.handle())?);
            minerva.restore_market_prices(saved_prices);
            // Whether this build can update in place, or only say a newer version is out.
            let build = updates::build();
            minerva.on_build(build);
            // Where a tool of the player's own leaves prices, taken in as the core loop runs.
            let market_imports = market_prices::imports_path(app.handle())?;
            let (commands, commands_rx) = mpsc::channel();
            // From now on errors go to the core too, for the Bug Report; those from before
            // (the settings or bests not reading) are held for it till now.
            errors::send_to(commands.clone());
            // Registered only while the game or one of our windows is in front.
            let hotkeys = hotkeys::install(app, &minerva.panel_state().hotkeys, commands.clone())?;
            // The one way to quit at any time, the game open or not.
            let tray = tray::install(app, commands.clone())?;
            app.manage(commands.clone());
            let setup_windows = SetupWindows::new(app.handle());
            // Without it the player's drags and resizes don't stick, but the panel works.
            if let Err(e) =
                sizemove::watch(&overlay, commands.clone(), overlay_size_move, Some(&sizemove::OVERLAY_RESIZABLE))
            {
                error!("{e}");
            }
            let panels = Panels::new(app.handle(), &commands);
            settings_window::watch_focus(app.handle(), commands.clone());
            // The page earlier versions wrote for the browser, which the Personal Bests
            // window replaces.
            let _ = std::fs::remove_file(app.path().app_config_dir()?.join("personal-bests.html"));
            let overlay_window = OverlayWindow::new(overlay.clone(), commands.clone());
            overlay.on_window_event(move |event| match event {
                // Only moves during a drag the overlay started are the player's.
                WindowEvent::Moved(to) => {
                    let _ = commands.send(Command::Moved(*to));
                }
                // Only sizes during a resize the overlay started are the player's.
                WindowEvent::Resized(size) => {
                    let _ = commands.send(Command::Resized(*size));
                }
                // Leaving the overlay for another window (the game) with the search view
                // open is Esc: it has the keyboard only while it's in front. Not when focus
                // only moved within our windows, as into the overlay's own webview or the
                // Settings window.
                WindowEvent::Focused(false) if !gamewin::foreground_is_ours() => {
                    let _ = commands.send(Command::Act(Action::SearchEscape));
                }
                _ => {}
            });

            let handle = app.handle().clone();
            // Both off the UI thread. Spawned from setup, so the process is already
            // Per-Monitor-V2 DPI aware.
            let reader = ReaderThread::spawn(app.state::<Sender<Command>>().inner().clone())?;
            let windows = Windows {
                overlay: overlay_window,
                setup: setup_windows,
                panels,
                hotkeys,
                tray,
            };
            // The uploads to the Map Leaderboards, with the Leaderboard Key, which the core never has.
            let key = leaderboards::KeyFile::new(leaderboards::key_path(app.handle())?);
            let uploader = leaderboards::Uploader::new(key, client, app.state::<Sender<Command>>().inner().clone());
            let updater = updates::Updater::new(app.handle().clone(), build, app.state::<Sender<Command>>().inner().clone());
            std::thread::Builder::new().name("core-loop".into()).spawn(move || {
                let files =
                    Files { settings: settings_path, bests: bests_file, market_prices: prices_file, market_imports };
                core_loop(handle, windows, minerva, reader, commands_rx, files, Online { uploader, updater })
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            act,
            page_height,
            view::page_view,
            errors::window_error,
            setup::pick_game_window,
            search::panel_press,
            search::open_mob_page,
            search::pick_item,
            search::search_query,
            search::search_pick,
            search::search_pick_destination,
            search::search_pick_item,
            settings_window::rebind_key,
            bests_window::personal_bests_open_map,
            bests_window::personal_bests_remove_visit,
            bests_window::personal_bests_move_visit,
            bests_window::personal_bests_assign_unassigned,
            bests_window::personal_bests_send_again,
            bests_window::personal_bests_remove_character,
            bests_window::personal_bests_merge_characters,
            bests_window::personal_bests_show_on_leaderboards,
            bests_window::personal_bests_open_leaderboard,
            bests_window::personal_bests_link_discord,
            bests_window::personal_bests_dismiss_link_prompt,
            item_card::item_card_close,
            item_card::item_card_pick,
            item_card::item_card_open_meowdb,
            item_card::item_card_open_mesowatch,
            item_card::item_card_edit_market,
            item_card::item_card_enter_market,
            item_card::item_card_edit_market_line,
            item_card::item_card_enter_market_line,
            item_card::item_card_choose_deal_line,
        ])
        .run(context)
        .expect("error while running tauri application");
}

/// Our windows: the overlay (with Search in it), setup, and the windows over the game.
struct Windows {
    overlay: OverlayWindow,
    setup: SetupWindows,
    panels: Panels,
    /// Not a window, but registered and released as the foreground changes.
    hotkeys: hotkeys::GlobalHotkeys,
    /// Not a window either: the owl in the tray, which shows a newer version.
    tray: tray::Tray,
}

/// Our windows over the game but the overlay, each kept by a [`PanelWindow`] as its spec says.
struct Panels {
    grind_tracker: PanelWindow,
    personal_bests: PanelWindow,
    settings: PanelWindow,
    item_lookup: PanelWindow,
    item_card: PanelWindow,
    deal_badges: PanelWindow,
}

impl Panels {
    /// The Personal Bests window after the overlay and the Grind Tracker's, which it stays
    /// above; the Settings window after them all, which it stays above.
    fn new(app: &AppHandle, commands: &Sender<Command>) -> Self {
        Self {
            grind_tracker: PanelWindow::new(app, &grind_tracker::SPEC, commands.clone()),
            personal_bests: PanelWindow::new(app, &bests_window::SPEC, commands.clone()),
            settings: PanelWindow::new(app, &settings_window::SPEC, commands.clone()),
            item_lookup: PanelWindow::new(app, &item_lookup::SPEC, commands.clone()),
            item_card: PanelWindow::new(app, &item_card::SPEC, commands.clone()),
            deal_badges: PanelWindow::new(app, &deal_badges::SPEC, commands.clone()),
        }
    }

    fn all_mut(&mut self) -> [&mut PanelWindow; 6] {
        [
            &mut self.grind_tracker,
            &mut self.personal_bests,
            &mut self.settings,
            &mut self.item_lookup,
            &mut self.item_card,
            &mut self.deal_badges,
        ]
    }

    /// The window with this label.
    fn by_label(&mut self, label: &str) -> Option<&mut PanelWindow> {
        self.all_mut().into_iter().find(|panel| panel.label() == label)
    }

    /// Show each window as the panel state says, and send its page its view.
    fn sync(&mut self, state: &PanelState) {
        grind_tracker::sync(&mut self.grind_tracker, state);
        bests_window::sync(&mut self.personal_bests, state);
        settings_window::sync(&mut self.settings, state);
        item_lookup::sync(&mut self.item_lookup, state);
        item_card::sync(&mut self.item_card, state);
        deal_badges::sync(&mut self.deal_badges, state);
    }
}

/// Where the core loop saves the settings, the Personal Bests and the market prices, and
/// the folder it takes Market Imports from.
struct Files {
    settings: PathBuf,
    bests: bests::BestsFile,
    market_prices: market_prices::MarketPricesFile,
    market_imports: PathBuf,
}

/// What of the settings, the Personal Bests and the market prices the core loop saved last
/// (or restored at startup): the settings themselves, and the core's revisions of the others.
struct Saved {
    settings: minerva_core::Settings,
    bests: u64,
    prices: u64,
}

/// What the core loop asks over the network for the core: the uploads to the Map
/// Leaderboards (with the Leaderboard Key's file), and the checks for a newer version, with
/// its downloads and installs.
struct Online {
    uploader: leaderboards::Uploader,
    updater: updates::Updater,
}

/// Save whatever Quit Minerva saves to `files`: the Personal Bests, the market prices, and
/// the settings if they changed since `saved`, which then holds them.
fn save_for_quit(minerva: &Minerva, files: &Files, saved: &mut Saved) {
    save_bests_and_prices(minerva, files, saved);
    let current = minerva.settings();
    if current != saved.settings {
        if let Err(e) = settings::save(&files.settings, &current) {
            error!("saving settings failed: {e}");
        }
        saved.settings = current;
    }
}

/// Save the Personal Bests or the market prices the player keeps (what `saving` gives) to
/// `file` if the core's `revision` of them moved on from `saved` (the one last saved, or
/// restored at startup). At once, not once they settle: the Visit under way's best is kept
/// as it changes, so however Minerva closes, it keeps it. Never over a file that didn't read
/// at startup and couldn't be set aside.
fn save_revised<T: files::JsonShape>(file: &files::GuardedJsonFile<T>, revision: u64, saved: &mut u64, saving: impl FnOnce() -> T) {
    if *saved == revision {
        return;
    }
    *saved = revision;
    if let Err(e) = file.save(&saving()) {
        error!("saving {} failed: {e}", T::WHAT);
    }
}

/// Save the Personal Bests and the market prices, each if it changed since `saved`.
fn save_bests_and_prices(minerva: &Minerva, files: &Files, saved: &mut Saved) {
    save_revised(&files.bests, minerva.bests_revision(), &mut saved.bests, || minerva.bests());
    save_revised(&files.market_prices, minerva.market_prices_revision(), &mut saved.prices, || minerva.market_prices());
}

/// The wall clock, in ms since the Unix epoch.
fn unix_now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// A region for the reader thread to capture and read, or an Item Lookup's capture,
/// taken already, to read.
#[derive(Debug, Clone)]
enum Capture {
    /// Where the Minimap Title is to be looked for, and where our windows are (which the
    /// capture holds, and the reader leaves out).
    Title(ScreenRect, Vec<ScreenRect>),
    Item(Box<ItemCapture>),
    /// The game's client area, for the Deal Finder.
    Shop(ScreenRect),
    /// The bottom of the game's client area, for the Status Bar, and how to read its EXP
    /// figures (`None`: they aren't asked for).
    StatusBar(ScreenRect, Option<ExpFigures>),
}

/// Captures and reads the Minimap Title, the Status Bar's strip (its EXP figures too) and the Deal
/// Finder's client area on its own thread, and reads Item Lookup's captures (taken at the
/// press, while the Item Tooltip shows). Reading takes tens of ms (far more in a debug
/// build), so the core loop never waits for it: it keeps following the game window at its
/// own pace and takes each reading when it arrives.
struct ReaderThread {
    captures: Sender<Capture>,
    /// The clock frames and ticks are timed by.
    start: Instant,
    /// A capture for the Minimap Title was asked for and its reading hasn't come back yet.
    title_busy: bool,
    /// Likewise for the Deal Finder's capture.
    shop_busy: bool,
    /// Likewise for the Status Bar's.
    status_bar_busy: bool,
}

impl ReaderThread {
    fn spawn(commands: Sender<Command>) -> std::io::Result<Self> {
        let start = Instant::now();
        let (captures, asked) = mpsc::channel::<Capture>();
        std::thread::Builder::new().name("reader".into()).spawn(move || {
            // The readers share one recogniser.
            // It remembers the titles it read: standing in one map costs no recognition.
            let mut title_reader = TitleReader::bundled();
            let exp_bar_reader = ExpBarReader::bundled();
            let item_reader = ItemTooltipReader::bundled();
            // It remembers the lines it read, so a shop left open costs no recognition.
            let mut shop_reader = PlayerShopReader::bundled();
            // So does this one: an unchanged Status Bar costs no recognition.
            let mut status_bar_reader = StatusBarReader::bundled();
            let grab = |r: ScreenRect| match capture::frame(r, || start.elapsed().as_millis() as u64) {
                Ok(frame) => Some(frame),
                Err(e) => {
                    error!("capture failed: {e}");
                    None
                }
            };
            for capture in asked {
                let command = match capture {
                    Capture::Title(region, ours) => {
                        let reading = grab(region).map(|frame| {
                            let capture = TitleCapture { frame, origin: ScreenPoint { x: region.x, y: region.y }, ours };
                            let read = title_reader.read(&capture);
                            (Box::new(capture), read)
                        });
                        Command::Read { reading }
                    }

                    Capture::Item(capture) => {
                        let reading = item_reader.read(&capture);
                        Command::ReadItemTooltip { capture, reading }
                    }
                    Capture::Shop(region) => {
                        // The pointer first, as for Item Lookup: the tooltip follows it.
                        let pointer = gamewin::cursor_position();
                        let reading = grab(region).map(|frame| {
                            let capture = ShopCapture { frame, origin: ScreenPoint { x: region.x, y: region.y }, pointer };
                            let read = shop_reader.read(&capture);
                            (Box::new(capture), read)
                        });
                        Command::ReadShop { reading }
                    }
                    Capture::StatusBar(region, figures) => {
                        let reading = grab(region).map(|frame| {
                            let capture = StatusBarCapture { frame, origin: ScreenPoint { x: region.x, y: region.y } };
                            let read = status_bar_reader.read(&capture);
                            let exp = figures.map(|figures| exp_bar_reader.read_status_bar(&capture, read.bar.as_ref(), figures));
                            (Box::new(capture), read, exp)
                        });
                        Command::ReadStatusBar { reading }
                    }
                };
                if commands.send(command).is_err() {
                    return;
                }
            }
        })?;
        Ok(Self { captures, start, title_busy: false, shop_busy: false, status_bar_busy: false })
    }

    /// Capture `region` and read the Minimap Title in it, but not in `ours` (our windows), unless
    /// the last capture for it is still being read.
    fn read_title(&mut self, region: ScreenRect, ours: Vec<ScreenRect>) {
        if !self.title_busy {
            self.title_busy = self.captures.send(Capture::Title(region, ours)).is_ok();
        }
    }


    /// Likewise for the Deal Finder's capture of the game's client area `region`.
    fn read_shop(&mut self, region: ScreenRect) {
        if !self.shop_busy {
            self.shop_busy = self.captures.send(Capture::Shop(region)).is_ok();
        }
    }

    /// Likewise for the Status Bar's capture of the bottom of the game's client area `region`,
    /// reading its EXP figures as `figures` says, if they are asked for.
    fn read_status_bar(&mut self, region: ScreenRect, figures: Option<ExpFigures>) {
        if !self.status_bar_busy {
            self.status_bar_busy = self.captures.send(Capture::StatusBar(region, figures)).is_ok();
        }
    }

    /// Item Lookup: take the pointer's position and capture the game's client area now,
    /// while the Item Tooltip shows, and have it read. Only reads the pointer position
    /// and copies screen pixels (ADR 0001).
    fn read_item_tooltip(&self) -> Result<(), String> {
        let pointer = gamewin::cursor_position().ok_or("no pointer position")?;
        let client = gamewin::find_game_client().ok_or("no game window")?;
        let frame = capture::frame(client, || self.now_ms())?;
        let capture = ItemCapture { frame, origin: ScreenPoint { x: client.x, y: client.y }, pointer };
        self.captures.send(Capture::Item(Box::new(capture))).map_err(|_| "the reader has stopped".to_string())
    }

    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

fn core_loop(
    app: AppHandle,
    windows: Windows,
    mut minerva: Minerva,
    mut reader: ReaderThread,
    commands: Receiver<Command>,
    files: Files,
    online: Online,
) {
    let Online { mut uploader, mut updater } = online;
    // Restored at startup: saved already.
    let mut saved =
        Saved { settings: minerva.settings(), bests: minerva.bests_revision(), prices: minerva.market_prices_revision() };
    let Windows {
        overlay: mut window,
        setup: mut setup_windows,
        mut panels,
        mut hotkeys,
        mut tray,
    } = windows;
    let mut pending = (saved.settings.clone(), Instant::now());
    let mut emitted: Option<PanelState> = None;
    let mut game = None;
    let mut expanded_height = EXPANDED_HEIGHT;
    let mut next_capture = Instant::now();
    let mut next_exp_bar_capture = Instant::now();
    let mut next_shop_capture = Instant::now();
    let mut next_window_check = Instant::now();
    let mut next_import_check = Instant::now();
    loop {
        // Wait for the player until the next check is due, so hotkeys act at once.
        let due = next_capture.min(next_exp_bar_capture).min(next_window_check);
        match commands.recv_timeout(due.saturating_duration_since(Instant::now())) {
            Ok(Command::Act(action)) => {
                minerva.act(action);
                // Opened again while it has the keyboard: from setup, which is in front.
                if action == Action::OpenSettings {
                    panels.settings.take_focus_again();
                }
            }
            Ok(Command::Hotkey(pressed)) => {
                let request = minerva.on_hotkey(&pressed);
                carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, request);
            }
            Ok(Command::Press(control)) => {
                let request = minerva.press(control);
                carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, request);
            }
            Ok(Command::SearchQuery(query)) => {
                minerva.search_query(&query);
            }
            Ok(Command::RebindKey(press)) => {
                minerva.rebind_key(press);
            }
            Ok(Command::SearchPick(id)) => {
                minerva.search_pick(&id);
            }
            Ok(Command::SearchPickDestination(id)) => {
                minerva.search_pick_destination(&id);
            }
            Ok(Command::SearchPickItem(id)) => {
                minerva.search_pick_item(&id);
            }
            Ok(Command::OpenMobPage(id)) => {
                let request = minerva.open_mob_page(&id);
                carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, request);
            }
            Ok(Command::OpenBestsMapPage(id)) => {
                let request = minerva.open_bests_map_page(&id);
                carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, request);
            }
            Ok(Command::RemoveVisit { character, map, visit }) => {
                minerva.remove_visit(character.as_deref(), &map, &visit);
            }
            Ok(Command::MoveVisit { from, map, visit, to }) => {
                minerva.move_visit(from.as_deref(), &map, &visit, &to);
            }
            Ok(Command::AssignUnassigned(to)) => {
                minerva.assign_unassigned(&to);
            }
            Ok(Command::RemoveCharacter(name)) => {
                minerva.remove_character(&name);
            }
            Ok(Command::MergeCharacters { from, into }) => {
                minerva.merge_characters(&from, &into);
            }
            Ok(Command::ShowOnLeaderboards { name, on }) => {
                minerva.show_on_leaderboards(&name, on);
            }
            Ok(Command::SendToLeaderboardsAgain(name)) => {
                minerva.send_to_leaderboards_again(&name);
            }
            Ok(Command::OpenBestsLeaderboard { character, id }) => {
                let request = minerva.open_bests_leaderboard(character.as_deref(), &id);
                carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, request);
            }
            Ok(Command::LeaderboardAnswer { id, answer }) => {
                minerva.on_leaderboard_answer(id, answer);
            }
            Ok(Command::LinkDiscord) => {
                minerva.link_discord();
            }
            Ok(Command::DismissLinkPrompt) => {
                minerva.dismiss_link_prompt();
            }
            Ok(Command::DiscordLinkAnswer { id, answer, open }) => {
                // The error isn't kept, nor logged: it may quote the address, with its link token.
                let answer = match open.map(|url| app.opener().open_url(url, None::<&str>)) {
                    Some(Err(_)) => DiscordLinkAnswer::Failed { why: "the browser didn't open".to_string() },
                    _ => answer,
                };
                minerva.on_discord_link_answer(id, answer);
            }
            Ok(Command::UpdateAnswer { id, answer }) => {
                minerva.on_update_answer(id, answer);
            }
            Ok(Command::UpdateDownload { id, fetched }) => {
                let answer = updater.downloaded(id, fetched);
                minerva.on_update_download(id, answer);
            }
            Ok(Command::PickItem(id)) => {
                let request = minerva.pick_item(&id);
                carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, request);
            }
            Ok(Command::CloseItemCard) => {
                minerva.close_item_card();
            }
            Ok(Command::PickCardItem(id)) => {
                minerva.pick_card_item(&id);
            }
            Ok(Command::OpenCardMeowdbPage) => {
                // Its page comes from `take_request`, once MeowDB's search has answered.
                minerva.open_card_meowdb_page(reader.now_ms());
            }
            Ok(Command::EditCardMarketPrice(line)) => {
                minerva.edit_card_market_price(&line);
            }
            Ok(Command::EnterCardMarket(typed)) => {
                minerva.enter_card_market(&typed);
            }
            Ok(Command::EditCardMarketLine(line)) => {
                minerva.edit_card_market_line(&line);
            }
            Ok(Command::EnterCardMarketLine(typed)) => {
                minerva.enter_card_market_line(&typed);
            }
            Ok(Command::ChooseDealLine(line)) => {
                minerva.choose_deal_line(&line);
            }
            Ok(Command::OpenCardMesowatch) => {
                let request = minerva.open_card_mesowatch();
                carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, request);
            }
            Ok(Command::Height(label, height)) => match label.as_str() {
                "overlay" => {
                    expanded_height = height.clamp(MIN_EXPANDED_HEIGHT, MAX_EXPANDED_HEIGHT);
                    set_panel_sizes(&mut minerva, &window.overlay, expanded_height);
                }
                label => {
                    if let Some(panel) = panels.by_label(label) {
                        panel.set_height(height);
                        panel.tell_size(&mut minerva);
                    }
                }
            },
            Ok(Command::LaunchedAgain) => {
                minerva.act(Action::LaunchedAgain);
                setup_windows.bring_to_front();
            }
            Ok(Command::CheckForeground) => next_window_check = Instant::now(),
            Ok(Command::PickGameWindow(title)) => {
                minerva.pick_game_window(title);
                next_window_check = Instant::now(); // look for it at once
            }
            Ok(Command::BugReport) => bug_reports::save(&app, &mut minerva, &window.overlay),
            Ok(Command::Error { from, message }) => minerva.note_error(&from, &message),
            Ok(Command::Quit) => {
                // The core says whether to quit now (below), or to install first the newer
                // version the player chose to install when they quit; chosen again while
                // that waits, it quits at once.
                minerva.act(Action::Quit);
            }
            Ok(Command::GestureStarted(gesture)) => window.gesture_started(gesture),
            Ok(Command::Resized(size)) => window.player_resized(size),
            Ok(Command::SizeMoveStarted) => window.size_move_started(),
            Ok(Command::SizeMoveEnded) => {
                // Once the player lets go of a resize, its width is the panel's (the core
                // keeps it over the minimum), and where the window ended up is where the
                // panel is: a bottom-left corner moves its left edge.
                if let Some(size) = window.size_move_ended()
                    && let Ok(scale) = window.overlay.scale_factor()
                {
                    minerva.act(Action::PanelResized { width: (f64::from(size.width) / scale).round() as u32 });
                    set_panel_sizes(&mut minerva, &window.overlay, expanded_height);
                    if let Ok(at) = window.overlay.outer_position() {
                        minerva.act(Action::PanelMoved { x: at.x, y: at.y });
                    }
                }
            }
            Ok(Command::Moved(to)) => {
                if window.player_moved() {
                    minerva.act(Action::PanelMoved { x: to.x, y: to.y });
                }
            }
            Ok(Command::SizeMove(label, what)) => {
                // Once the player lets go, where they left it.
                if let Some(moved) = panels.by_label(label).and_then(|panel| panel.size_move(what)) {
                    minerva.act(moved);
                }
            }
            // A reading of a region no longer captured (the game window moved, say) the core
            // leaves, and writes down that it did; as it does a Deal Finder reading once it is
            // off, and EXP figures once the Grind Tracker is.
            Ok(Command::Read { reading }) => {
                reader.title_busy = false;
                if let Some((capture, reading)) = reading {
                    minerva.on_title_read(*capture, reading);
                }
            }

            Ok(Command::ReadItemTooltip { capture, reading }) => {
                // With the capture, whose part around the pointer a Bug Report keeps.
                minerva.on_item_capture_read(*capture, reading);
            }
            Ok(Command::ReadShop { reading }) => {
                reader.shop_busy = false;
                if let Some((capture, reading)) = reading {
                    minerva.on_player_shop_read(*capture, reading);
                }
            }
            Ok(Command::ReadStatusBar { reading }) => {
                reader.status_bar_busy = false;
                if let Some((capture, reading, exp)) = reading {
                    minerva.on_status_bar_read(*capture, reading, exp);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        gamewin::look_for(&minerva.panel_state().game_window_title);
        // Checked after every wake-up, so a stream of drag moves can't starve them.
        if Instant::now() >= next_window_check {
            next_window_check = Instant::now() + WINDOW_TICK;
            game = find_game(&mut minerva, &window.overlay, expanded_height);
            // For the display they are on now.
            for panel in panels.all_mut() {
                panel.tell_size(&mut minerva);
            }
            // The hotkeys are live, and our windows show, only while the game or one of our
            // windows is in front.
            minerva.on_foreground(gamewin::foreground(game.map(|g| g.handle)));
        }
        // After every wake-up too: the hotkeys are released as soon as the player starts
        // rebinding one (a registered combination would never reach the Settings window), and the
        // new one is registered as soon as they have pressed it.
        let state = minerva.panel_state();
        if let Some((taken, invalid)) = hotkeys.sync(&app, &state.hotkeys, state.hotkeys_live) {
            minerva.hotkeys_unavailable(taken, invalid);
        }
        // Fetches in flight (OSMS's file set and pictures, MeowDB's search) are checked on,
        // never waited for.
        minerva.on_wall_clock(unix_now_ms());
        minerva.tick(reader.now_ms());
        // An Item Lookup's page, once MeowDB has answered.
        while let Some(request) = minerva.take_request() {
            carry_out(&app, &window.overlay, &mut panels, &reader, &mut minerva, Some(request));
        }
        // The core asks for uploads to the Map Leaderboards on a tick: each is made once.
        uploader.sync(&minerva.panel_state().leaderboard_uploads);
        uploader.sync_discord(&minerva.panel_state().discord_link_requests);
        // And for a check for a newer version, and its download: each is run once.
        updater.sync(&minerva.panel_state().updates);
        // Its install, on this thread: whatever Quit Minerva saves is saved first, as the
        // installer ends Minerva with `process::exit(0)`, and nothing after runs.
        if let Some(install) = updater.install(&minerva.panel_state().updates) {
            let (id, why) = match install {
                Ok(install) => {
                    let id = install.id;
                    save_for_quit(&minerva, &files, &mut saved);
                    // Saved, as on the way to quitting: nothing waits to be saved again.
                    pending = (saved.settings.clone(), Instant::now());
                    // Nor would the owl be taken out of the tray after: it would stay there,
                    // a ghost, until the pointer passed over it. Back if it didn't start.
                    tray.set_visible(false);
                    let why = install.run();
                    tray.set_visible(true);
                    (id, why)
                }
                Err(id) => (id, "its download is gone".to_string()),
            };
            error!("starting the installer failed: {why}");
            minerva.on_update_install_failed(id, &why);
        }
        // Prices a tool of the player's own left (ADR 0013), saved below like those typed.
        // Left waiting while the prices can't be saved, rather than taken in and lost.
        if Instant::now() >= next_import_check && files.market_prices.saves() {
            next_import_check = Instant::now() + IMPORT_TICK;
            for import in market_prices::take_imports(&files.market_imports) {
                minerva.import_market_prices(import);
            }
        }
        if Instant::now() >= next_capture {
            // Where the title was found, or the whole client area while it is looked for, less
            // often (ADR 0021). None while another app is in front.
            let searching = minerva.panel_state().title_searching;
            next_capture = Instant::now() + if searching { TITLE_SEARCH_TICK } else { TICK };
            if let Some(region) = minerva.panel_state().title_capture {
                reader.read_title(region, minerva.panel_state().our_windows.clone());
            }
        }
        if Instant::now() >= next_exp_bar_capture {
            next_exp_bar_capture = Instant::now() + EXP_BAR_TICK;
            // The Status Bar and its EXP figures (ADR 0017), read from one capture: the name
            // reaches the core just before the EXP does. None while another app is in front;
            // no figures while the Grind Tracker is off.
            if let Some(region) = minerva.panel_state().status_bar_capture {
                reader.read_status_bar(region, minerva.panel_state().exp_figures);
            }
        }
        if Instant::now() >= next_shop_capture {
            next_shop_capture = Instant::now() + DEAL_FINDER_TICK;
            // None while the Deal Finder is off.
            if let Some(region) = minerva.panel_state().deal_finder_capture {
                reader.read_shop(region);
            }
        }
        let state = minerva.panel_state();
        if state.quit {
            // Whatever the player changed last (a drag, say) is saved first.
            save_for_quit(&minerva, &files, &mut saved);
            app.exit(0);
            return;
        }
        // Live even while Quit Minerva waits for the update: it says so, and Quit Minerva
        // chosen again quits at once.
        tray.sync(&state.updates);
        if state.updates.quitting {
            // Waiting for the download to install: no window shows again.
            for webview in app.webview_windows().values() {
                let _ = webview.hide();
            }
            continue;
        }
        setup_windows.sync(state, game.map(|g| g.client));
        if emitted.as_ref() != Some(state) {
            if let Err(e) = app.emit_to("overlay", "overlay-state", state) {
                error!("emit failed: {e}");
            }
            emitted = Some(state.clone());
        }
        // The window that takes the keyboard takes it before the other gives it back: given
        // back first, Windows would hand it to the game in between, and might then refuse it
        // to ours. The search view takes it from the Settings window, and the Settings window
        // from the search view.
        if state.settings_window.as_ref().is_some_and(|settings| settings.takes_keyboard) {
            panels.sync(state);
            window.show(state);
        } else {
            window.show(state);
            panels.sync(state);
        }

        // Save once the settings settle, not on every move of a drag.
        let current = minerva.settings();
        if current != pending.0 {
            pending = (current, Instant::now());
        }
        if pending.0 != saved.settings && pending.1.elapsed() >= SAVE_DELAY {
            if let Err(e) = settings::save(&files.settings, &pending.0) {
                error!("saving settings failed: {e}");
            }
            saved.settings = pending.0.clone();
        }
        save_bests_and_prices(&minerva, &files, &mut saved);
    }
}

/// Find the game window and the monitors, and feed them to the core.
fn find_game(minerva: &mut Minerva, overlay: &WebviewWindow, expanded_height: f64) -> Option<gamewin::Game> {
    set_panel_sizes(minerva, overlay, expanded_height);
    let game = gamewin::find_game();
    minerva.on_game_window(game.map(|g| GameWindow {
        client: g.client,
        frame: g.frame,
        screens: gamewin::monitor_work_areas(),
    }));
    game
}

/// Tell the core the panel's sizes in physical px, for the overlay's current display:
/// expanded, as wide as the core says and as tall as its content.
fn set_panel_sizes(minerva: &mut Minerva, overlay: &WebviewWindow, expanded_height: f64) {
    if let Ok(scale) = overlay.scale_factor() {
        let width = f64::from(minerva.panel_state().panel_width);
        minerva.set_panel_sizes(PanelSizes {
            expanded: physical((width, expanded_height), scale),
            collapsed: physical(COLLAPSED, scale),
        });
    }
}

/// Do what a hotkey, a panel icon, a link or an Item Lookup asked of the shell. Right after
/// the player's own key press or click, so Windows lets us take the foreground.
fn carry_out(
    app: &AppHandle,
    overlay: &WebviewWindow,
    panels: &mut Panels,
    reader: &ReaderThread,
    minerva: &mut Minerva,
    request: Option<Request>,
) {
    match request {
        Some(Request::OpenInBrowser { url }) => open_in_browser(app, &url),
        Some(Request::RaiseOverlay) => search::take_focus(overlay),
        Some(Request::CaptureItemTooltip) => {
            // The Item Card and the Deal Badges show in our captures: they would be read in
            // place of the Item Tooltip they cover. The capture waits for the screen to be
            // drawn without them.
            let card_moved = panels.item_card.out_of_the_capture();
            if panels.deal_badges.out_of_the_capture() || card_moved {
                std::thread::sleep(OFF_SCREEN_SETTLE);
            }
            if let Err(e) = reader.read_item_tooltip() {
                error!("Item Lookup: {e}");
                minerva.on_item_tooltip_reading(None);
            }
        }
        None => {}
    }
}

/// Open a page (a MeowDB page, or the Minerva site) in the player's browser.
fn open_in_browser(app: &AppHandle, url: &str) {
    if let Err(e) = app.opener().open_url(url, None::<&str>) {
        error!("couldn't open {url}: {e}");
    }
}

/// The overlay window, kept in line with the panel state. Window properties are only
/// set when they change.
struct OverlayWindow {
    overlay: WebviewWindow,
    layer: Option<Layer>,
    click_through: bool,
    focusable: Option<bool>,
    /// Whether the overlay is listed in the taskbar (only while detached).
    in_taskbar: Option<bool>,
    /// The search view is open: the overlay has the keyboard.
    takes_keyboard: bool,
    /// Tells its watcher the core shows it, so it keeps it on top while the game or one
    /// of our windows is in front.
    shows_on_top: Arc<AtomicBool>,
    /// The player's drag or resize, from the mouse press that starts it until Windows'
    /// move or size loop ends.
    gesture: GestureTracker,
    /// The size the player's resize last gave the overlay.
    resized_to: Option<PhysicalSize<u32>>,
    /// Whether the window's minimum size is the expanded panel's (or none, collapsed).
    min_size_expanded: Option<bool>,
}

impl OverlayWindow {
    fn new(overlay: WebviewWindow, commands: Sender<Command>) -> Self {
        let shows_on_top = keep_on_top(&overlay, "on-top", commands, &[]);
        Self {
            overlay,
            layer: None,
            click_through: true,
            focusable: None,
            in_taskbar: None,
            takes_keyboard: false,
            shows_on_top,
            gesture: GestureTracker::default(),
            resized_to: None,
            min_size_expanded: None,
        }
    }

    /// The player pressed the mouse to drag or resize the overlay.
    fn gesture_started(&mut self, gesture: Gesture) {
        self.gesture.pressed(gesture, Instant::now());
        self.resized_to = None;
    }

    /// Windows' move or size loop started: the drag or resize the player started is
    /// running. Any other loop isn't the player's.
    fn size_move_started(&mut self) {
        self.gesture.loop_started(Instant::now());
    }

    /// Windows' move or size loop ended: the player let go of the mouse. The size their
    /// resize left the overlay, once (when it changed anything).
    fn size_move_ended(&mut self) -> Option<PhysicalSize<u32>> {
        let ended = self.gesture.loop_ended()?;
        if ended == Gesture::Resize { self.resized_to.take() } else { None }
    }

    /// The overlay window is now `size`: the player's, while they resize it. Any other
    /// size is the shell's own.
    fn player_resized(&mut self, size: PhysicalSize<u32>) {
        if self.gesture.running() == Some(Gesture::Resize) {
            self.resized_to = Some(size);
        }
    }

    /// The overlay window moved: whether that was the player's drag. Any other move is
    /// the shell's own or the system's, and the core never hears of it.
    fn player_moved(&self) -> bool {
        self.gesture.running() == Some(Gesture::Drag)
    }

    fn show(&mut self, state: &PanelState) {
        // On top of everything while it shows: its watcher alone changes that, so a
        // state the core has already moved on from never puts it back over another app.
        self.shows_on_top.store(state.layer == Layer::OnTop, Ordering::Relaxed);
        let layer_changed = self.layer != Some(state.layer);
        self.layer = Some(state.layer);
        // Detached, it can be found in the taskbar.
        let detached = state.placement_mode == PlacementMode::Detached;
        if self.in_taskbar != Some(detached) {
            self.in_taskbar = Some(detached);
            let _ = self.overlay.set_skip_taskbar(!detached);
        }
        // Only the open search view takes the typing. Otherwise the overlay can't be
        // activated (WS_EX_NOACTIVATE), so a click on it leaves the keyboard with the game.
        let takes_keyboard = state.overlay_takes_keyboard;
        if self.focusable != Some(takes_keyboard) {
            self.focusable = Some(takes_keyboard);
            let _ = self.overlay.set_focusable(takes_keyboard);
        }
        // Always clickable while it shows. Hidden (the game is gone, or another app is in
        // front) the webview draws nothing, so clicks pass through and nothing is in the
        // way.
        let click_through = state.layer == Layer::Hidden;
        if self.click_through != click_through {
            self.click_through = click_through;
            if let Err(e) = self.overlay.set_ignore_cursor_events(click_through) {
                error!("click-through toggle failed: {e}");
            }
        }
        // The search view opens right after our hotkey or a click on the overlay, so
        // Windows lets us take the foreground; when it closes, Windows gives focus back to
        // the window below.
        match (self.takes_keyboard, takes_keyboard) {
            (false, true) => search::take_focus(&self.overlay),
            (true, false) => search::give_focus_back(&self.overlay),
            _ => {}
        }
        self.takes_keyboard = takes_keyboard;
        // Expanded, the player can't resize the panel narrower than the core allows;
        // collapsed, the window is the icon's size, and they can't resize it.
        sizemove::set_overlay_resizable(state.expanded);
        if self.min_size_expanded != Some(state.expanded) {
            self.min_size_expanded = Some(state.expanded);
            let min = LogicalSize::new(f64::from(MIN_PANEL_WIDTH), MIN_EXPANDED_HEIGHT);
            let _ = self.overlay.set_min_size(state.expanded.then_some(min));
        }
        // While the player resizes it the size (and position) is theirs too.
        let gesture = self.gesture.under_way(Instant::now());
        if let Some(placement) = state.placement
            && gesture != Some(Gesture::Resize)
        {
            // While the player drags it the position is theirs; once they let go it goes
            // where the core says (clear of the Minimap Title, inside the game).
            place(&self.overlay, placement, gesture == Some(Gesture::Drag) && !layer_changed);
        }
    }
}

#[cfg(test)]
mod tests {
    use minerva_core::{Action, Hotkey, PanelControl, Section};

    /// What our pages send `act` and `panel_press`, written as they write it (`src/*.ts`):
    /// a typo there reaches no type check, so each must read as the core's.
    #[test]
    fn what_the_pages_send_reads_as_the_cores_actions_and_controls() {
        let actions = [
            (r#""searchNext""#, Action::SearchNext),
            (r#""searchPrevious""#, Action::SearchPrevious),
            (r#""searchEnter""#, Action::SearchEnter),
            (r#""searchEscape""#, Action::SearchEscape),
            (r#""reopenMapList""#, Action::ReopenMapList),
            (r#""clearHuntingTarget""#, Action::ClearHuntingTarget),
            (r#""toggleTaxis""#, Action::ToggleTaxis),
            (r#""toggleTeleportScrolls""#, Action::ToggleTeleportScrolls),
            (r#"{"toggleSection":"hiddenPortals"}"#, Action::ToggleSection(Section::HiddenPortals)),
            (r#""openSettings""#, Action::OpenSettings),
            (r#""closeSettings""#, Action::CloseSettings),
            (r#""focusSettings""#, Action::FocusSettings),
            (r#"{"beginRebind":"itemCard"}"#, Action::BeginRebind(Hotkey::ItemCard)),
            (r#""cancelRebind""#, Action::CancelRebind),
            (r#""turnOffHotkey""#, Action::TurnOffHotkey),
            (r#""swapHotkeys""#, Action::SwapHotkeys),
            (r#""resetHotkeys""#, Action::ResetHotkeys),
            (r#""undoResetHotkeys""#, Action::UndoResetHotkeys),
            (r#""refreshGameData""#, Action::RefreshGameData),
            (r#"{"setCheckForUpdates":{"on":true}}"#, Action::SetCheckForUpdates { on: true }),
            (r#""cancelCardMarket""#, Action::CancelCardMarket),
            (r#""addCardMarketLine""#, Action::AddCardMarketLine),
            (r#""askRemoveCardMarketLine""#, Action::AskRemoveCardMarketLine),
            (r#""removeCardMarketLine""#, Action::RemoveCardMarketLine),
            (r#"{"openDealItem":{"row":2}}"#, Action::OpenDealItem { row: 2 }),
            (r#"{"setDealMargin":{"percent":15}}"#, Action::SetDealMargin { percent: 15 }),
            (r#"{"setGrindTracker":{"on":false}}"#, Action::SetGrindTracker { on: false }),
            (r#""checkForUpdates""#, Action::CheckForUpdates),
            (r#""updateNow""#, Action::UpdateNow),
            (r#""updateWhenIQuit""#, Action::UpdateWhenIQuit),
            (r#""updateLater""#, Action::UpdateLater),
            (r#""finishSetup""#, Action::FinishSetup),
        ];
        for (sent, action) in actions {
            assert_eq!(serde_json::from_str::<Action>(sent).ok(), Some(action), "{sent}");
        }
        let controls = [
            (r#""settings""#, PanelControl::Settings),
            (r#""openSite""#, PanelControl::OpenSite),
            (r#""openGithub""#, PanelControl::OpenGithub),
            (r#""openDiscord""#, PanelControl::OpenDiscord),
            (r#""openRelease""#, PanelControl::OpenRelease),
            (r#""update""#, PanelControl::Update),
        ];
        for (sent, control) in controls {
            assert_eq!(serde_json::from_str::<PanelControl>(sent).ok(), Some(control), "{sent}");
        }
    }
}
