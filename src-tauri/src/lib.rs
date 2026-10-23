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
mod hotkey_settings;
mod hotkeys;
mod http;
mod item_card;
mod item_lookup;
mod leaderboards;
mod market_prices;
mod meowdb;
mod old_caches;
pub mod osms;
pub mod osms_items;
mod pictures;
mod search;
mod settings;
mod setup;
mod sizemove;
pub mod source;
mod tray;
pub mod update_manifest;
mod updates;
mod view;

use minerva_core::{
    Action, DEAL_FINDER_CAPTURE_MS, Minerva, ExpBarReader, ExpBarReading, ExpFigures, Frame, GameWindow, ItemCapture, ItemTooltipReader,
    ItemTooltipReading, KeyPress, Layer, LeaderboardAnswer, MIN_PANEL_WIDTH, PanelControl, PanelSizes, PlacementMode, PanelState, PlayerShopReader,
    Request, ScreenPoint, ScreenRect, ShopCapture, ShopReading, Size, StatusBarCapture, StatusBarReader, StatusBarReading,
    TITLE_READ_MS, TITLE_SEARCH_MS, TitleCapture, TitleReader, TitleReading, UpdateAnswer, VisitBest,
};
use source::WebSource;
use bests_window::{BestsWindow, PersonalBestsView};
use deal_badges::{DealBadgesView, DealBadgesWindow};
use errors::error;
use grind_tracker::{GrindTrackerView, GrindTrackerWindow};
use item_card::{ItemCardView, ItemCardWindow, OFF_SCREEN_SETTLE};
use item_lookup::{ItemLookupView, ItemLookupWindow};
use setup::{Bridge, SetupPageView, SetupWindows};
use sizemove::{Gesture, SizeMove};
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
const WINDOW_TICK: Duration = Duration::from_millis(50);
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
/// A drag or resize the player started whose move or size loop hasn't started within this
/// long never will (the mouse was let go first): the shell places the panel again.
pub(crate) const LOOP_START_GRACE: Duration = Duration::from_secs(1);

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
    /// The player pressed this key on the settings view, for the core.
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
    /// The core's check `id` for a newer version came to this.
    UpdateAnswer { id: u64, answer: UpdateAnswer },
    /// The core's download `id` of a newer version came to this: its installer, checked
    /// against the public key, or why not.
    UpdateDownload { id: u64, fetched: Result<updates::Fetched, String> },
    /// The player picked the item with this MeowDB ID from the ones an Item Lookup found
    /// sharing the name it read.
    PickItem(String),
    /// The expanded panel's content is this tall (CSS px).
    PanelHeight(f64),
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
    /// The Grind Tracker's content is this tall (CSS px).
    GrindTrackerHeight(f64),
    /// Windows told of a drag of the Grind Tracker's window.
    GrindTrackerSizeMove(SizeMove),
    /// The Personal Bests window's content is this tall (CSS px).
    PersonalBestsHeight(f64),
    /// Windows told of a drag of the Personal Bests window.
    PersonalBestsSizeMove(SizeMove),
    /// The Item Lookup alert is this tall (CSS px).
    ItemLookupHeight(f64),
    /// The Item Card is this tall (CSS px).
    ItemCardHeight(f64),
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
    /// Windows told of a drag of the Item Card's window.
    ItemCardSizeMove(SizeMove),
    /// The player chose Quit in the tray icon's menu: save the settings and exit.
    Quit,
    /// The player chose Bug Report in the tray icon's menu: save one, and open the bug
    /// form for it.
    BugReport,
    /// The shell or one of our windows (`from`) ran into an error, for the Bug Report.
    Error { from: String, message: String },
    /// The player started Minerva again; that one exited, and this one shows itself.
    LaunchedAgain,
    /// A watcher took one of our windows out of sight, with another app in front, while
    /// the core still showed it: check the game window and the foreground now.
    CheckForeground,
    /// The reader is done with a capture of `region` for the Minimap Title: the capture and
    /// what it read, or `None` if the capture failed.
    Read { region: ScreenRect, reading: Option<(Box<TitleCapture>, TitleReading)> },

    /// The reader is done with an Item Card's or Item Lookup's capture: the capture, and
    /// what it read.
    ReadItemTooltip { capture: Box<ItemCapture>, reading: ItemTooltipReading },
    /// The reader is done with a Deal Finder capture of `region` (the game's client area):
    /// the capture and what it read, or `None` if the capture failed.
    ReadShop { region: ScreenRect, reading: Option<(Box<ShopCapture>, ShopReading)> },
    /// The reader is done with a capture of `region` (the bottom of the game's client area)
    /// for the Status Bar: the capture, what it read, and what its EXP figures read as (if
    /// they were asked for), or `None` if the capture failed.
    ReadStatusBar {
        region: ScreenRect,
        reading: Option<(Box<StatusBarCapture>, StatusBarReading, Option<ExpBarReading>)>,
    },
}

/// The overlay's drags and resizes, as the core loop hears them.
fn overlay_size_move(what: SizeMove) -> Command {
    match what {
        SizeMove::Pressed(gesture) => Command::GestureStarted(gesture),
        SizeMove::LoopStarted => Command::SizeMoveStarted,
        SizeMove::LoopEnded => Command::SizeMoveEnded,
    }
}

/// The overlay reports how tall its expanded content is, so the window fits it.
#[tauri::command]
fn panel_height(height: f64, commands: tauri::State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PanelHeight(height));
}

/// Where OSMS's file set and pictures, and MeowDB's search answers, are cached. WebView2
/// shares the app cache folder, hence the subfolder.
fn cache_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_cache_dir()?.join("cache"))
}

/// Capture the game's whole client area for the Minimap Title, as the app does while it looks
/// for it (ADR 0021): for the probe.
pub fn capture_title(client: ScreenRect, at_ms: u64) -> Result<TitleCapture, String> {
    let rgba = capture::capture_rgba(client.x, client.y, client.width as i32, client.height as i32)?;
    let frame = Frame { width: client.width, height: client.height, rgba, at_ms };
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
        .manage(SetupPageView::default())
        .manage(GrindTrackerView::default())
        .manage(PersonalBestsView::default())
        .manage(ItemLookupView::default())
        .manage(ItemCardView::default())
        .manage(DealBadgesView::default())
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
            let (picks, picks_rx) = mpsc::channel();
            app.manage(Bridge { commands: commands.clone(), picks });
            let setup_windows = SetupWindows::new(app.handle())?;
            // Without it the player's drags and resizes don't stick, but the panel works.
            if let Err(e) =
                sizemove::watch(&overlay, commands.clone(), overlay_size_move, sizemove::overlay_resizable())
            {
                error!("{e}");
            }
            let grind_tracker = GrindTrackerWindow::new(app.handle(), commands.clone());
            // After the overlay and the Grind Tracker's window, which it stays above.
            let personal_bests = BestsWindow::new(app.handle(), commands.clone());
            // The page earlier versions wrote for the browser, which this window replaces.
            let _ = std::fs::remove_file(app.path().app_config_dir()?.join("personal-bests.html"));
            let item_lookup = ItemLookupWindow::new(app.handle(), commands.clone());
            let item_card = ItemCardWindow::new(app.handle(), commands.clone());
            let deal_badges = DealBadgesWindow::new(app.handle(), commands.clone());
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
                // or the settings view open is Esc: it has the keyboard only while it's in
                // front. Not when focus only moved within our windows, as into the
                // overlay's own webview.
                WindowEvent::Focused(false) if !gamewin::foreground_is_ours() => {
                    let _ = commands.send(Command::Act(Action::SearchEscape));
                    let _ = commands.send(Command::Act(Action::CloseSettings));
                }
                _ => {}
            });

            let handle = app.handle().clone();
            // Both off the UI thread. Spawned from setup, so the process is already
            // Per-Monitor-V2 DPI aware.
            let reader = ReaderThread::spawn(app.state::<Sender<Command>>().inner().clone())?;
            let channels = Channels { commands: commands_rx, picks: picks_rx };
            let windows = Windows {
                overlay: overlay_window,
                setup: setup_windows,
                grind_tracker,
                personal_bests,
                item_lookup,
                item_card,
                deal_badges,
                hotkeys,
                tray,
            };
            // The uploads to the Map Leaderboards, with the Leaderboard Key, which the core never has.
            let key = leaderboards::KeyFile::new(leaderboards::key_path(app.handle())?);
            let uploader = leaderboards::Uploader::new(key, app.state::<Sender<Command>>().inner().clone());
            let updater = updates::Updater::new(app.handle().clone(), build, app.state::<Sender<Command>>().inner().clone());
            std::thread::Builder::new().name("core-loop".into()).spawn(move || {
                let files =
                    Files { settings: settings_path, bests: bests_file, market_prices: prices_file, market_imports };
                core_loop(handle, windows, minerva, reader, channels, files, Online { uploader, updater })
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            panel_height,
            errors::window_error,
            setup::setup_view,
            setup::pick_game_window,
            setup::finish_setup,
            search::panel_press,
            search::open_mob_page,
            search::pick_item,
            search::search_query,
            search::search_move,
            search::search_enter,
            search::search_pick,
            search::search_pick_destination,
            search::search_pick_item,
            search::search_escape,
            search::hunting_reopen,
            search::hunting_clear,
            search::search_toggle_taxis,
            search::search_toggle_teleport_scrolls,
            search::panel_toggle_section,
            hotkey_settings::settings_close,
            hotkey_settings::rebind_begin,
            hotkey_settings::rebind_key,
            hotkey_settings::rebind_cancel,
            hotkey_settings::rebind_turn_off,
            hotkey_settings::rebind_swap,
            hotkey_settings::hotkeys_reset,
            hotkey_settings::hotkeys_undo_reset,
            hotkey_settings::game_data_refresh,
            hotkey_settings::check_for_updates_set,
            setup::open_hotkey_settings,
            setup::open_site,
            setup::open_github,
            setup::open_discord,
            setup::check_for_updates,
            setup::open_release,
            setup::update,
            setup::update_choice,
            grind_tracker::grind_tracker_set,
            grind_tracker::grind_tracker_height,
            grind_tracker::grind_tracker_view,
            bests_window::personal_bests_height,
            bests_window::personal_bests_view,
            bests_window::personal_bests_open_map,
            bests_window::personal_bests_remove_visit,
            bests_window::personal_bests_move_visit,
            bests_window::personal_bests_assign_unassigned,
            bests_window::personal_bests_send_again,
            bests_window::personal_bests_remove_character,
            bests_window::personal_bests_merge_characters,
            bests_window::personal_bests_show_on_leaderboards,
            bests_window::personal_bests_open_leaderboard,
            item_lookup::item_lookup_height,
            item_lookup::item_lookup_view,
            item_card::item_card_height,
            item_card::item_card_view,
            item_card::item_card_close,
            item_card::item_card_pick,
            item_card::item_card_open_meowdb,
            item_card::item_card_open_mesowatch,
            item_card::item_card_edit_market,
            item_card::item_card_enter_market,
            item_card::item_card_cancel_market,
            item_card::item_card_edit_market_line,
            item_card::item_card_enter_market_line,
            item_card::item_card_add_market_line,
            item_card::item_card_ask_remove_market_line,
            item_card::item_card_remove_market_line,
            item_card::item_card_choose_deal_line,
            deal_badges::deal_badges_view,
            deal_badges::deal_badge_open,
            deal_badges::deal_margin_set,
        ])
        .run(context)
        .expect("error while running tauri application");
}

/// Our windows: the overlay (with Search in it), setup, the
/// Grind Tracker's, the Personal Bests window, the Item Lookup alert's, the Item Card's and
/// the Deal Badges'.
struct Windows {
    overlay: OverlayWindow,
    setup: SetupWindows,
    grind_tracker: GrindTrackerWindow,
    personal_bests: BestsWindow,
    item_lookup: ItemLookupWindow,
    item_card: ItemCardWindow,
    deal_badges: DealBadgesWindow,
    /// Not a window, but registered and released as the foreground changes.
    hotkeys: hotkeys::GlobalHotkeys,
    /// Not a window either: the owl in the tray, which shows a newer version.
    tray: tray::Tray,
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
    save_bests(minerva, &files.bests, &mut saved.bests);
    save_market_prices(minerva, &files.market_prices, &mut saved.prices);
    let current = minerva.settings();
    if current != saved.settings {
        if let Err(e) = settings::save(&files.settings, &current) {
            error!("saving settings failed: {e}");
        }
        saved.settings = current;
    }
}

/// Save the Personal Bests if the core's bests revision moved on from `saved` (the one
/// last saved, or restored at startup). At once, not once they settle: the Visit under
/// way's best is kept as it changes, so however Minerva closes, it keeps it. Never
/// over a file that didn't read at startup and couldn't be set aside.
fn save_bests(minerva: &Minerva, file: &bests::BestsFile, saved: &mut u64) {
    let revision = minerva.bests_revision();
    if *saved == revision {
        return;
    }
    *saved = revision;
    if let Err(e) = file.save(&minerva.bests()) {
        error!("saving personal bests failed: {e}");
    }
}

/// Save the market prices the player keeps if the core's revision of them moved on from
/// `saved`, at once, as the Personal Bests are. Never over a file that didn't read at
/// startup and couldn't be set aside.
fn save_market_prices(minerva: &Minerva, file: &market_prices::MarketPricesFile, saved: &mut u64) {
    let revision = minerva.market_prices_revision();
    if *saved == revision {
        return;
    }
    *saved = revision;
    if let Err(e) = file.save(&minerva.market_prices()) {
        error!("saving market prices failed: {e}");
    }
}

/// The wall clock, in ms since the Unix epoch.
fn unix_now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// What the core loop hears from hotkeys, the overlay, the setup window and the
/// reader thread.
struct Channels {
    commands: Receiver<Command>,
    /// Titles of game windows the player picked by hand.
    picks: Receiver<String>,
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
            let grab = |r: ScreenRect| match capture::capture_rgba(r.x, r.y, r.width as i32, r.height as i32) {
                Ok(rgba) => {
                    let at_ms = start.elapsed().as_millis() as u64;
                    Some(Frame { width: r.width, height: r.height, rgba, at_ms })
                }
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
                        Command::Read { region, reading }
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
                        Command::ReadShop { region, reading }
                    }
                    Capture::StatusBar(region, figures) => {
                        let reading = grab(region).map(|frame| {
                            let capture = StatusBarCapture { frame, origin: ScreenPoint { x: region.x, y: region.y } };
                            let read = status_bar_reader.read(&capture);
                            let exp = figures.map(|figures| exp_bar_reader.read_status_bar(&capture, read.bar.as_ref(), figures));
                            (Box::new(capture), read, exp)
                        });
                        Command::ReadStatusBar { region, reading }
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
        let rgba = capture::capture_rgba(client.x, client.y, client.width as i32, client.height as i32)?;
        let frame = Frame { width: client.width, height: client.height, rgba, at_ms: self.now_ms() };
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
    channels: Channels,
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
        mut grind_tracker,
        mut personal_bests,
        mut item_lookup,
        mut item_card,
        mut deal_badges,
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
        match channels.commands.recv_timeout(due.saturating_duration_since(Instant::now())) {
            Ok(Command::Act(action)) => {
                minerva.act(action);
            }
            Ok(Command::Hotkey(pressed)) => {
                let request = minerva.on_hotkey(&pressed);
                carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, request);
            }
            Ok(Command::Press(control)) => {
                let request = minerva.press(control);
                carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, request);
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
                carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, request);
            }
            Ok(Command::OpenBestsMapPage(id)) => {
                let request = minerva.open_bests_map_page(&id);
                carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, request);
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
                carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, request);
            }
            Ok(Command::LeaderboardAnswer { id, answer }) => {
                minerva.on_leaderboard_answer(id, answer);
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
                carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, request);
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
                carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, request);
            }
            Ok(Command::PanelHeight(height)) => {
                expanded_height = height.clamp(MIN_EXPANDED_HEIGHT, MAX_EXPANDED_HEIGHT);
                set_panel_sizes(&mut minerva, &window.overlay, expanded_height);
            }
            Ok(Command::LaunchedAgain) => {
                minerva.act(Action::LaunchedAgain);
                setup_windows.bring_to_front();
            }
            Ok(Command::CheckForeground) => next_window_check = Instant::now(),
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
            Ok(Command::GrindTrackerHeight(height)) => {
                grind_tracker.set_height(height);
                if let Some(size) = grind_tracker.size() {
                    minerva.set_grind_tracker_size(size);
                }
            }
            Ok(Command::ItemLookupHeight(height)) => {
                item_lookup.set_height(height);
                if let Some(size) = item_lookup.size() {
                    minerva.set_item_lookup_size(size);
                }
            }
            Ok(Command::ItemCardHeight(height)) => {
                item_card.set_height(height);
                if let Some(size) = item_card.size() {
                    minerva.set_item_card_size(size);
                }
            }
            Ok(Command::GrindTrackerSizeMove(what)) => {
                if let Some(at) = grind_tracker.size_move(what) {
                    minerva.act(Action::GrindTrackerMoved { x: at.x, y: at.y });
                }
            }
            Ok(Command::PersonalBestsHeight(height)) => {
                personal_bests.set_height(height);
                if let Some(size) = personal_bests.size() {
                    minerva.set_personal_bests_size(size);
                }
            }
            Ok(Command::ItemCardSizeMove(what)) => {
                if let Some(at) = item_card.size_move(what) {
                    minerva.act(Action::ItemCardMoved { x: at.x, y: at.y });
                }
            }
            Ok(Command::PersonalBestsSizeMove(what)) => {
                if let Some(at) = personal_bests.size_move(what) {
                    minerva.act(Action::PersonalBestsMoved { x: at.x, y: at.y });
                }
            }
            Ok(Command::Read { region, reading }) => {
                reader.title_busy = false;
                // A reading of a region no longer captured (the game window moved, say) would
                // put the title where it isn't.
                if let (Some((capture, reading)), true) = (reading, minerva.panel_state().title_capture == Some(region)) {
                    minerva.on_title_read(*capture, reading);
                }
            }

            Ok(Command::ReadItemTooltip { capture, reading }) => {
                // With the capture, whose part around the pointer a Bug Report keeps.
                minerva.on_item_capture_read(*capture, reading);
            }
            Ok(Command::ReadShop { region, reading }) => {
                reader.shop_busy = false;
                // Not once the Deal Finder is off, or the game window moved.
                if let (Some((capture, reading)), true) = (reading, minerva.panel_state().deal_finder_capture == Some(region)) {
                    minerva.on_player_shop_read(*capture, reading);
                }
            }
            Ok(Command::ReadStatusBar { region, reading }) => {
                reader.status_bar_busy = false;
                // Not once the game window moved, or another app came in front.
                if let (Some((capture, reading, exp)), true) = (reading, minerva.panel_state().status_bar_capture == Some(region)) {
                    // Nor the EXP figures once the Grind Tracker is off.
                    let exp = exp.filter(|_| minerva.panel_state().exp_figures.is_some());
                    minerva.on_status_bar_read(*capture, reading, exp);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(title) = channels.picks.try_recv() {
            minerva.pick_game_window(title);
            next_window_check = Instant::now(); // look for it at once
        }
        gamewin::look_for(&minerva.panel_state().game_window_title);
        // Checked after every wake-up, so a stream of drag moves can't starve them.
        if Instant::now() >= next_window_check {
            next_window_check = Instant::now() + WINDOW_TICK;
            game = find_game(&mut minerva, &window.overlay, expanded_height);
            // For the display they are on now.
            if let Some(size) = grind_tracker.size() {
                minerva.set_grind_tracker_size(size);
            }
            if let Some(size) = personal_bests.size() {
                minerva.set_personal_bests_size(size);
            }
            if let Some(size) = item_lookup.size() {
                minerva.set_item_lookup_size(size);
            }
            if let Some(size) = item_card.size() {
                minerva.set_item_card_size(size);
            }
            if let Some(size) = deal_badges.size() {
                minerva.set_deal_badges_size(size);
            }
            // The hotkeys are live, and our windows show, only while the game or one of our
            // windows is in front.
            minerva.on_foreground(gamewin::foreground(game.map(|g| g.handle)));
        }
        // After every wake-up too: the hotkeys are released as soon as the player starts
        // rebinding one (a registered combination would never reach the overlay), and the
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
            carry_out(&app, &window.overlay, &mut item_card, &mut deal_badges, &reader, &mut minerva, Some(request));
        }
        // The core asks for uploads to the Map Leaderboards on a tick: each is made once.
        uploader.sync(&minerva.panel_state().leaderboard_uploads);
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
        window.show(state);
        grind_tracker.sync(state);
        personal_bests.sync(state);
        item_lookup.sync(state);
        item_card.sync(state);
        deal_badges.sync(state);

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
        save_bests(&minerva, &files.bests, &mut saved.bests);
        save_market_prices(&minerva, &files.market_prices, &mut saved.prices);
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
    item_card: &mut ItemCardWindow,
    deal_badges: &mut DealBadgesWindow,
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
            let card_moved = item_card.out_of_the_capture();
            if deal_badges.out_of_the_capture() || card_moved {
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

/// CSS px to physical px, rounded up so the panel's content is never clipped.
pub(crate) fn physical((width, height): (f64, f64), scale: f64) -> Size {
    Size { width: (width * scale).ceil() as u32, height: (height * scale).ceil() as u32 }
}

/// The player's drag or resize: it runs in Windows' move or size loop until the mouse is
/// let go.
#[derive(Debug, Clone, Copy)]
struct PlayerGesture {
    gesture: Gesture,
    /// When the player pressed the mouse to start it, until Windows' loop has started too.
    starting_since: Option<Instant>,
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
    /// The search view or the settings view is open: the overlay has the keyboard.
    takes_keyboard: bool,
    /// Tells its watcher the core shows it, so it keeps it on top while the game or one
    /// of our windows is in front.
    shows_on_top: Arc<AtomicBool>,
    /// The player's drag or resize, from the mouse press that starts it until Windows'
    /// move or size loop ends.
    gesture: Option<PlayerGesture>,
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
            gesture: None,
            resized_to: None,
            min_size_expanded: None,
        }
    }

    /// The player pressed the mouse to drag or resize the overlay.
    fn gesture_started(&mut self, gesture: Gesture) {
        self.gesture = Some(PlayerGesture { gesture, starting_since: Some(Instant::now()) });
        self.resized_to = None;
    }

    /// Windows' move or size loop started: the drag or resize the player started is
    /// running. Any other loop isn't the player's.
    fn size_move_started(&mut self) {
        self.under_way();
        if let Some(player) = &mut self.gesture {
            player.starting_since = None;
        }
    }

    /// Windows' move or size loop ended: the player let go of the mouse. The size their
    /// resize left the overlay, once (when it changed anything).
    fn size_move_ended(&mut self) -> Option<PhysicalSize<u32>> {
        let ended = self.gesture.take_if(|player| player.starting_since.is_none())?;
        if ended.gesture == Gesture::Resize { self.resized_to.take() } else { None }
    }

    /// The player's drag or resize, from the mouse press until they let go. One
    /// whose loop hasn't started within `LOOP_START_GRACE` never will.
    fn under_way(&mut self) -> Option<Gesture> {
        let starting_since = self.gesture.and_then(|player| player.starting_since);
        if starting_since.is_some_and(|at| at.elapsed() >= LOOP_START_GRACE) {
            self.gesture = None;
        }
        self.gesture.map(|player| player.gesture)
    }

    /// The player's drag or resize while Windows' loop runs it.
    fn running(&self) -> Option<Gesture> {
        self.gesture.filter(|player| player.starting_since.is_none()).map(|player| player.gesture)
    }

    /// The overlay window is now `size`: the player's, while they resize it. Any other
    /// size is the shell's own.
    fn player_resized(&mut self, size: PhysicalSize<u32>) {
        if self.running() == Some(Gesture::Resize) {
            self.resized_to = Some(size);
        }
    }

    /// The overlay window moved: whether that was the player's drag. Any other move is
    /// the shell's own or the system's, and the core never hears of it.
    fn player_moved(&self) -> bool {
        self.running() == Some(Gesture::Drag)
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
        // Only the open search view takes the typing, and the open settings view the
        // combinations the player presses. Otherwise the overlay can't be activated
        // (WS_EX_NOACTIVATE), so a click on it leaves the keyboard with the game.
        let takes_keyboard = state.search.view.is_some() || state.settings_view.is_some();
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
        // The search view or the settings view opens right after our hotkey or a click on
        // the overlay, so Windows lets us take the foreground; when it closes, Windows
        // gives focus back to the window below.
        match (self.takes_keyboard, takes_keyboard) {
            (false, true) => search::take_focus(&self.overlay),
            (true, false) => search::give_focus_back(&self.overlay),
            _ => {}
        }
        self.takes_keyboard = takes_keyboard;
        // Expanded, the player can't resize the panel narrower than the core allows;
        // collapsed, the window is the icon's size, and they can't resize it.
        sizemove::set_resizable(state.expanded);
        if self.min_size_expanded != Some(state.expanded) {
            self.min_size_expanded = Some(state.expanded);
            let min = LogicalSize::new(f64::from(MIN_PANEL_WIDTH), MIN_EXPANDED_HEIGHT);
            let _ = self.overlay.set_min_size(state.expanded.then_some(min));
        }
        // While the player resizes it the size (and position) is theirs too.
        let gesture = self.under_way();
        if let Some(placement) = state.placement
            && gesture != Some(Gesture::Resize)
        {
            // While the player drags it the position is theirs; once they let go it goes
            // where the core says (clear of the Minimap Title, inside the game).
            place(&self.overlay, placement, gesture == Some(Gesture::Drag) && !layer_changed);
        }
    }
}

/// Keep our `window` on top of every other window while the core shows it (the flag
/// returned, which the core loop sets) and the game or one of our windows is in front,
/// and out of sight otherwise: on a thread of its own (named `name`), at its own steady
/// pace. On top, it stays above our windows `beneath` too. The core decides what shows, but the core loop can stall for a while (setting
/// window sizes and positions), and a window on top of everything must not sit over
/// another app meanwhile: this watcher checks what is in front itself, takes the window
/// out of sight within a tick of another app coming to the front, and tells the minerva
/// loop (`commands`) to check the foreground at once. It gets the window's handle once,
/// here: asking Tauri for it from another thread waits on the main thread's event loop,
/// which can leave the watcher idle for seconds while another app is in front.
pub(crate) fn keep_on_top(
    window: &WebviewWindow,
    name: &str,
    commands: Sender<Command>,
    beneath: &[WebviewWindow],
) -> Arc<AtomicBool> {
    let shows = Arc::new(AtomicBool::new(false));
    #[cfg(windows)]
    match window.hwnd() {
        Ok(hwnd) => {
            let (handle, watcher_flag) = (hwnd.0 as isize, shows.clone());
            let beneath: Vec<isize> = beneath.iter().filter_map(|w| w.hwnd().ok()).map(|h| h.0 as isize).collect();
            let spawned = std::thread::Builder::new()
                .name(name.into())
                .spawn(move || keep_on_top_while_in_front(handle, beneath, watcher_flag, commands));
            if let Err(e) = spawned {
                error!("{name} watcher not started: {e}");
            }
        }
        Err(e) => error!("{name} watcher not started: no {} window handle: {e}", window.label()),
    }
    #[cfg(not(windows))]
    let _ = (window, name, commands, beneath);
    shows
}

/// Every `WINDOW_TICK`, put our window (its handle) on top of everything if `shows` is
/// set and the game or one of our windows is in front, and at the bottom, out of sight,
/// if not; on top, back above our windows `beneath` should one have come over it. Nothing
/// here waits on the main thread.
///
/// The core alone decides what shows (`Panel::layer` in minerva-core's panel.rs: a game
/// window, and the game or one of ours in front), and `shows` is its last word. The
/// watcher restates that rule against what is in front now, as a fast path: the minerva
/// loop reports the foreground to the core every tick but can stall (setting window
/// sizes and positions), and a window on top of everything must not sit over another
/// app while it does. So the rule here must agree with the core's: never show what the
/// core wouldn't. Should they part, change both.
#[cfg(windows)]
fn keep_on_top_while_in_front(window: isize, beneath: Vec<isize>, shows: Arc<AtomicBool>, commands: Sender<Command>) {
    use minerva_core::Foreground;
    use gamewin::Stacking;
    use windows::Win32::Foundation::HWND;
    let window = HWND(window as *mut _);
    let beneath: Vec<HWND> = beneath.into_iter().map(|handle| HWND(handle as *mut _)).collect();
    let mut last_error = None;
    // On top or at the bottom, as this watcher last put it: the window's style alone can't
    // be trusted to say (see `gamewin::restack`).
    let mut set_before = None;
    loop {
        std::thread::sleep(WINDOW_TICK);
        let shows = shows.load(Ordering::Relaxed);
        // The core's rule (`Panel::layer`), from what is in front now rather than at the
        // core loop's last report, which a stall can leave behind.
        let in_front = || {
            gamewin::find_game().is_some_and(|game| gamewin::foreground(Some(game.handle)) != Foreground::OtherApp)
        };
        let to = if shows && in_front() { Stacking::OnTop } else { Stacking::Bottom };
        // One of `beneath` came over it (on top again after another app, say): on top
        // afresh.
        let covered = to == Stacking::OnTop && gamewin::below_any(window, &beneath);
        let moved = gamewin::restack(window, to, if covered { None } else { set_before });
        if moved.is_ok() {
            set_before = Some(to);
        }
        // The core still shows it, over another app: the core loop is behind.
        let behind = shows && to == Stacking::Bottom && matches!(moved, Ok(true));
        if behind && commands.send(Command::CheckForeground).is_err() {
            return;
        }
        // Polled 20 times a second: report a failure once, not every time.
        let error = moved.err().map(|e| e.to_string());
        if error.is_some() && error != last_error {
            error!("putting our window on top or out of sight failed: {}", error.as_deref().unwrap_or_default());
        }
        last_error = error;
    }
}

/// Size our window to the core's placement, and move it there unless the player is
/// dragging it.
pub(crate) fn place(overlay: &WebviewWindow, placement: ScreenRect, player_drags_it: bool) {
    let size = PhysicalSize::new(placement.width, placement.height);
    if overlay.inner_size().ok() != Some(size) {
        let _ = overlay.set_size(size);
    }
    let to = PhysicalPosition::new(placement.x, placement.y);
    if !player_drags_it && overlay.outer_position().ok() != Some(to) {
        let _ = overlay.set_position(to);
    }
}
