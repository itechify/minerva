//! The Personal Bests window: every map's Personal Best, over the game where the core puts
//! it. Borderless like the overlay, and like it on top of everything while it shows (only
//! while the game or one of our windows is in front), above the panel and the Grind
//! Tracker's window, clickable but never activated by a click (`focusable: false` in
//! `tauri.conf.json`, ADR 0001). Like the Grind Tracker's window it is declared in
//! `tauri.conf.json` and never hidden: while the core says it doesn't show, the page draws
//! nothing and every click goes through. Every decision is the core's; this only places
//! the window, sends its page what it renders, and relays the player's drags.

use crate::errors::error;
use crate::grind_tracker::GRIND_TRACKER;
use crate::sizemove::{self, Gesture, SizeMove};
use crate::view::{LastView, ViewSender};
use crate::{Command, LOOP_START_GRACE, keep_on_top, physical, place};
use minerva_core::{MapMark, PanelState, PersonalBestsCharacter, Size, VisitBest};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Instant;
use tauri::{AppHandle, Manager, PhysicalPosition, State, WebviewWindow};

pub const PERSONAL_BESTS: &str = "personal-bests";

/// The width its page lays out at, in CSS px (logical); the core narrows the window to a
/// narrower game, and `src/personal-bests.css` lays out to whatever it gets. Its height
/// follows its content, as the page reports it, within these bounds: past the most, the
/// table scrolls.
const WIDTH: f64 = 920.0;
const DEFAULT_HEIGHT: f64 = 300.0;
const MIN_HEIGHT: f64 = 80.0;
const MAX_HEIGHT: f64 = 600.0;

/// The view last sent to the Personal Bests page, for when it loads after it was sent.
#[derive(Default)]
pub struct PersonalBestsView(pub LastView);

/// What the Personal Bests page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BestsView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    /// Whose bests it can show, one at a time, as its picker offers them.
    characters: &'a [PersonalBestsCharacter],
    /// Every Map Mark, which each map names its own by.
    map_marks: &'a BTreeMap<String, MapMark>,
}

/// The player's drag of the window, from the mouse press until Windows' move loop ends.
#[derive(Debug, Clone, Copy)]
struct Drag {
    /// When the player pressed the mouse, until Windows' loop has started too.
    starting_since: Option<Instant>,
}

/// The Personal Bests window, kept in line with the panel state.
pub struct BestsWindow {
    app: AppHandle,
    window: WebviewWindow,
    shown: bool,
    /// Tells its watcher the core shows it, so it keeps it on top while the game or one
    /// of our windows is in front.
    shows_on_top: Arc<AtomicBool>,
    /// Its content's height in CSS px, as the page last reported it.
    height: f64,
    drag: Option<Drag>,
    view: ViewSender,
}

impl BestsWindow {
    pub fn new(app: &AppHandle, commands: Sender<Command>) -> Self {
        let window = app.get_webview_window(PERSONAL_BESTS).expect("personal-bests window in tauri.conf.json");
        // Without it the player's drags don't stick, but the window works.
        if let Err(e) = sizemove::watch(&window, commands.clone(), Command::PersonalBestsSizeMove, None) {
            error!("{e}");
        }
        // Over the panel and the Grind Tracker where it covers them: the player opened it
        // to read it.
        let beneath: Vec<WebviewWindow> =
            ["overlay", GRIND_TRACKER].into_iter().filter_map(|label| app.get_webview_window(label)).collect();
        let shows_on_top = keep_on_top(&window, "on-top-personal-bests", commands, &beneath);
        // Clicks pass through until it shows.
        let _ = window.set_ignore_cursor_events(true);
        Self {
            app: app.clone(),
            window,
            shown: false,
            shows_on_top,
            height: DEFAULT_HEIGHT,
            drag: None,
            view: ViewSender::new(PERSONAL_BESTS, "personal-bests-state"),
        }
    }

    /// The page's content is `height` CSS px tall.
    pub fn set_height(&mut self, height: f64) {
        self.height = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    }

    /// The size its page lays it out at, in physical px, for the display it is on.
    pub fn size(&self) -> Option<Size> {
        let scale = self.window.scale_factor().ok()?;
        Some(physical((WIDTH, self.height), scale))
    }

    /// Windows told of a drag of the window. Once the player lets go, where they left
    /// it (its top-left corner), for the core.
    pub fn size_move(&mut self, what: SizeMove) -> Option<PhysicalPosition<i32>> {
        match what {
            SizeMove::Pressed(Gesture::Drag) => self.drag = Some(Drag { starting_since: Some(Instant::now()) }),
            // It isn't resizable.
            SizeMove::Pressed(Gesture::Resize) => {}
            SizeMove::LoopStarted => {
                self.under_way();
                if let Some(drag) = &mut self.drag {
                    drag.starting_since = None;
                }
            }
            SizeMove::LoopEnded => {
                self.drag.take_if(|drag| drag.starting_since.is_none())?;
                return self.window.outer_position().ok();
            }
        }
        None
    }

    /// Whether the player is dragging the window. A press whose loop hasn't started
    /// within `LOOP_START_GRACE` never will (the mouse was let go first).
    fn under_way(&mut self) -> bool {
        if self.drag.and_then(|drag| drag.starting_since).is_some_and(|at| at.elapsed() >= LOOP_START_GRACE) {
            self.drag = None;
        }
        self.drag.is_some()
    }

    /// Show the window where the core says, or hide it; and send its page the rows.
    pub fn sync(&mut self, state: &PanelState) {
        let bests = &state.personal_bests;
        let placement = bests.placement;
        let view = BestsView {
            shown: placement.is_some(),
            characters: &bests.characters,
            map_marks: &state.map_marks,
        };
        self.view.send(&self.app, &view, &self.app.state::<PersonalBestsView>().0);
        self.shows_on_top.store(placement.is_some(), Ordering::Relaxed);
        let dragging = self.under_way();
        if let Some(placement) = placement {
            // While the player drags it the position is theirs; once they let go it goes
            // where the core says (on a monitor, clear of the captured regions).
            place(&self.window, placement, dragging);
        }
        // Hidden, the page draws nothing and clicks pass through.
        if self.shown != placement.is_some() {
            self.shown = placement.is_some();
            if let Err(e) = self.window.set_ignore_cursor_events(!self.shown) {
                error!("personal bests click-through toggle failed: {e}");
            }
        }
    }
}

/// The page reports how tall its content is, so the window fits it.
#[tauri::command]
pub fn personal_bests_height(height: f64, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PersonalBestsHeight(height));
}

/// A click on a map's name in the window (its ID): open its MeowDB page.
#[tauri::command]
pub fn personal_bests_open_map(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenBestsMapPage(id));
}

/// A second click on a Visit's × in the window: remove it from the Visit History of the map
/// (its ID) of the Character named `character` (`None`: the Unassigned one), by its best as
/// the window lists it.
#[tauri::command]
pub fn personal_bests_remove_visit(
    character: Option<String>,
    map: String,
    visit: VisitBest,
    commands: State<'_, Sender<Command>>,
) {
    let _ = commands.send(Command::RemoveVisit { character, map, visit });
}

/// The player confirmed moving a Visit on the map (its ID) of the Character named `from`
/// (`None`: an Unassigned one), by its best as the window lists it, to the Character named
/// `to`.
#[tauri::command]
pub fn personal_bests_move_visit(
    from: Option<String>,
    map: String,
    visit: VisitBest,
    to: String,
    commands: State<'_, Sender<Command>>,
) {
    let _ = commands.send(Command::MoveVisit { from, map, visit, to });
}

/// The player confirmed giving the Character named `to` every Unassigned Visit.
#[tauri::command]
pub fn personal_bests_assign_unassigned(to: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::AssignUnassigned(to));
}

/// The player sent the Character named `name` to the Map Leaderboards again (having confirmed,
/// for uploads held).
#[tauri::command]
pub fn personal_bests_send_again(name: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SendToLeaderboardsAgain(name));
}

/// The player confirmed removing the Character named `name`, with their bests.
#[tauri::command]
pub fn personal_bests_remove_character(name: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::RemoveCharacter(name));
}

/// The player confirmed merging the Character named `from` into the one named `into`.
#[tauri::command]
pub fn personal_bests_merge_characters(from: String, into: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::MergeCharacters { from, into });
}

/// The player turned "Share" on (`on`) or off for the Character named `name`.
#[tauri::command]
pub fn personal_bests_show_on_leaderboards(name: String, on: bool, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::ShowOnLeaderboards { name, on });
}

/// A click on a map's Map Leaderboard link (its ID), among the bests of the Character named
/// `character` (`None`: the Unassigned ones): open it.
#[tauri::command]
pub fn personal_bests_open_leaderboard(character: Option<String>, id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenBestsLeaderboard { character, id });
}

/// What the Personal Bests page renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn personal_bests_view(view: State<'_, PersonalBestsView>) -> Result<Option<serde_json::Value>, String> {
    Ok(view.0.get())
}
