//! The Grind Tracker's window: borderless like the overlay, and like it on top of
//! everything while it shows (only while the game or one of our windows is in front),
//! clickable but never activated by a click (`focusable: false` in `tauri.conf.json`,
//! ADR 0001). Like the overlay it is declared in
//! `tauri.conf.json` (creating windows inside commands can deadlock on Windows) and is
//! never hidden: while the core says it doesn't show, the page draws nothing and every
//! click goes through. (Showing a hidden window again either activates it, taking the
//! keyboard from the game, or, done with `SW_SHOWNOACTIVATE` behind tao's back, leaves
//! it in the system's title bar and frame.) Every decision is the core's; this only
//! shows, hides and places the window, sends it what it renders, and relays the
//! player's drags.

use crate::errors::error;
use crate::sizemove::{self, Gesture, SizeMove};
use crate::view::{LastView, ViewSender};
use crate::{Command, LOOP_START_GRACE, keep_on_top, physical, place};
use minerva_core::{Character, ExpTracking, PanelState, Size};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Instant;
use tauri::{AppHandle, Manager, PhysicalPosition, State, WebviewWindow};

pub const GRIND_TRACKER: &str = "grind-tracker";

/// The window's width in CSS px (logical); `src/grind-tracker.css` lays out to fit. Its
/// height follows its content, as the page reports it, within these bounds.
const WIDTH: f64 = 280.0;
const DEFAULT_HEIGHT: f64 = 140.0;
const MIN_HEIGHT: f64 = 40.0;
const MAX_HEIGHT: f64 = 400.0;

/// The view last sent to the Grind Tracker's page, for when it loads after it was sent.
#[derive(Default)]
pub struct GrindTrackerView(pub LastView);

/// What the Grind Tracker's page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackerView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    tracking: &'a ExpTracking,
    /// The Character playing, as the Status Bar names them, beside the EXP bar's reading.
    character: Option<&'a Character>,
    /// Some map has a Personal Best: the window bar's Personal Bests button shows.
    personal_bests: bool,
    /// The Personal Bests window is open: that button shows as on.
    personal_bests_open: bool,
}

/// The player's drag of the window, from the mouse press until Windows' move loop ends.
#[derive(Debug, Clone, Copy)]
struct Drag {
    /// When the player pressed the mouse, until Windows' loop has started too.
    starting_since: Option<Instant>,
}

/// The Grind Tracker's window, kept in line with the panel state.
pub struct GrindTrackerWindow {
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

impl GrindTrackerWindow {
    pub fn new(app: &AppHandle, commands: Sender<Command>) -> Self {
        let window = app.get_webview_window(GRIND_TRACKER).expect("grind-tracker window in tauri.conf.json");
        // Without it the player's drags don't stick, but the window works.
        if let Err(e) = sizemove::watch(&window, commands.clone(), Command::GrindTrackerSizeMove, None) {
            error!("{e}");
        }
        let shows_on_top = keep_on_top(&window, "on-top-grind-tracker", commands, &[]);
        // Clicks pass through until it shows.
        let _ = window.set_ignore_cursor_events(true);
        Self {
            app: app.clone(),
            window,
            shown: false,
            shows_on_top,
            height: DEFAULT_HEIGHT,
            drag: None,
            view: ViewSender::new(GRIND_TRACKER, "grind-tracker-state"),
        }
    }

    /// The page's content is `height` CSS px tall.
    pub fn set_height(&mut self, height: f64) {
        self.height = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    }

    /// The window's size in physical px, for the display it is on.
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

    /// Show the window where the core says, or hide it; and send its page the Grind
    /// Tracker's figures.
    pub fn sync(&mut self, state: &PanelState) {
        let placement = state.grind_tracker.placement;
        let view = TrackerView {
            shown: placement.is_some(),
            tracking: &state.exp_tracking,
            character: state.character.as_ref(),
            personal_bests: state.grind_tracker.personal_bests,
            personal_bests_open: state.personal_bests.open,
        };
        self.view.send(&self.app, &view, &self.app.state::<GrindTrackerView>().0);
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
                error!("grind tracker click-through toggle failed: {e}");
            }
        }
    }
}

/// The page reports how tall its content is, so the window fits it.
/// The settings view's Grind Tracker switch: on, the EXP figures are read from the Status
/// Bar; off, they aren't, and the Grind Tracker doesn't show.
#[tauri::command]
pub fn grind_tracker_set(on: bool, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::Act(minerva_core::Action::SetGrindTracker { on }));
}

#[tauri::command]
pub fn grind_tracker_height(height: f64, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::GrindTrackerHeight(height));
}

/// What the Grind Tracker's page renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn grind_tracker_view(view: State<'_, GrindTrackerView>) -> Result<Option<serde_json::Value>, String> {
    Ok(view.0.get())
}
