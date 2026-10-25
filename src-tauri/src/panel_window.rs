//! Our windows over the game but the overlay: the Grind Tracker's, the Personal Bests', the
//! Owl Bubble's, the Item Card's and the Deal Badges'. Each is borderless like the
//! overlay, and like it on top of everything while it shows (only while the game or one of
//! our windows is in front), and never activated by a click (`focusable: false` in
//! `tauri.conf.json`, ADR 0001). Each is declared in `tauri.conf.json` (creating windows
//! inside commands can deadlock on Windows) and is never hidden: while the core says it
//! doesn't show, its page draws nothing and every click goes through. (Showing a hidden
//! window again either activates it, taking the keyboard from the game, or, done with
//! `SW_SHOWNOACTIVATE` behind tao's back, leaves it in the system's title bar and frame.)
//!
//! A [`PanelWindow`] keeps one of them in line with the panel state, as its [`WindowSpec`]
//! says: it sends its page what it renders, keeps it on top or out of sight, places it where
//! the core says (unless the player is dragging it), lets clicks through or not, gives it
//! the keyboard while the core says it takes it, relays the player's drags, and tells the
//! core its size. Every decision is the core's; each window's module says only what its page
//! renders and what of the panel state it shows.

use crate::errors::error;
use crate::item_card::OUT_OF_SIGHT;
use crate::search;
use crate::sizemove::{self, Gesture, GestureTracker, SizeMove};
use crate::view::ViewSender;
use crate::{Command, WINDOW_TICK};
use minerva_core::{Action, Minerva, PanelState, ScreenRect, Size};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Instant;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

/// What sets one of our windows apart.
pub struct WindowSpec {
    /// Its label in `tauri.conf.json`.
    pub label: &'static str,
    /// The event its page hears its view by.
    pub event: &'static str,
    /// The name of its watcher's thread ([`keep_on_top`]).
    pub watcher: &'static str,
    /// Its width in CSS px (logical), which its page lays out to; for a page that fits its
    /// width to its content (the Owl Bubble's), the most it can be.
    pub width: f64,
    /// Its height in CSS px until its page reports its content's, and the bounds of that:
    /// past the most, the page scrolls.
    pub height: Heights,
    /// Tells the core the window's size in physical px, for the display it is on.
    pub sized: fn(&mut Minerva, Size) -> &PanelState,
    /// What a drag of it by the player comes to, once they let go, where they left its
    /// top-left corner; `None` for a window the player doesn't drag.
    pub moved: Option<fn(i32, i32) -> Action>,
    /// Our windows (by label) it stays above while it shows.
    pub beneath: &'static [&'static str],
}

/// A window's height in CSS px: until its page reports one, and the least and most.
pub struct Heights {
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

/// What of one of our windows shows, as the panel state says.
pub struct Shows {
    /// Where the core places it; `None` while it doesn't show.
    pub placement: Option<ScreenRect>,
    /// It takes the clicks on itself: otherwise every click goes through.
    pub clickable: bool,
    /// It takes the keyboard (a field on it is open).
    pub takes_keyboard: bool,
}

/// One of our windows, kept in line with the panel state. Window properties are only set
/// when they change.
pub struct PanelWindow {
    spec: &'static WindowSpec,
    app: AppHandle,
    window: WebviewWindow,
    view: ViewSender,
    /// Tells its watcher the core shows it, so it keeps it on top while the game or one
    /// of our windows is in front.
    shows_on_top: Arc<AtomicBool>,
    /// It takes clicks.
    clickable: bool,
    /// It has the keyboard.
    takes_keyboard: bool,
    /// Its content's height in CSS px, as the page last reported it.
    height: f64,
    /// Its width in CSS px: the spec's, or its content's, as a page that fits it last
    /// reported it.
    width: f64,
    /// The player's drag of it.
    gesture: GestureTracker,
}

impl PanelWindow {
    pub fn new(app: &AppHandle, spec: &'static WindowSpec, commands: Sender<Command>) -> Self {
        let label = spec.label;
        let window =
            app.get_webview_window(label).unwrap_or_else(|| panic!("{label} window in tauri.conf.json"));
        // Without it the player's drags don't stick, but the window works.
        if spec.moved.is_some()
            && let Err(e) = sizemove::watch(&window, commands.clone(), move |what| Command::SizeMove(label, what), None)
        {
            error!("{e}");
        }
        let beneath: Vec<WebviewWindow> = spec.beneath.iter().filter_map(|label| app.get_webview_window(label)).collect();
        let shows_on_top = keep_on_top(&window, spec.watcher, commands, &beneath);
        // Clicks pass through until it shows.
        let _ = window.set_ignore_cursor_events(true);
        Self {
            spec,
            app: app.clone(),
            window,
            view: ViewSender::new(label, spec.event),
            shows_on_top,
            clickable: false,
            takes_keyboard: false,
            height: spec.height.default,
            width: spec.width,
            gesture: GestureTracker::default(),
        }
    }

    pub fn label(&self) -> &'static str {
        self.spec.label
    }

    /// The page's content is `height` CSS px tall.
    pub fn set_height(&mut self, height: f64) {
        let Heights { min, max, .. } = self.spec.height;
        self.height = height.clamp(min, max);
    }

    /// The page's content is `width` CSS px wide: its window fits it, never wider than the
    /// spec's.
    pub fn set_width(&mut self, width: f64) {
        self.width = width.clamp(1.0, self.spec.width);
    }

    /// The window's size in physical px, for the display it is on.
    pub fn size(&self) -> Option<Size> {
        let scale = self.window.scale_factor().ok()?;
        Some(physical((self.width, self.height), scale))
    }

    /// Tell the core the window's size, for the display it is on now.
    pub fn tell_size(&self, minerva: &mut Minerva) {
        if let Some(size) = self.size() {
            (self.spec.sized)(minerva, size);
        }
    }

    /// Windows told of a drag of the window. Once the player lets go, where they left it
    /// (its top-left corner), for the core. It isn't resizable.
    pub fn size_move(&mut self, what: SizeMove) -> Option<Action> {
        let moved = self.spec.moved?;
        match what {
            SizeMove::Pressed(Gesture::Drag) => self.gesture.pressed(Gesture::Drag, Instant::now()),
            SizeMove::Pressed(Gesture::Resize) => {}
            SizeMove::LoopStarted => self.gesture.loop_started(Instant::now()),
            SizeMove::LoopEnded => {
                self.gesture.loop_ended()?;
                let at = self.window.outer_position().ok()?;
                return Some(moved(at.x, at.y));
            }
        }
        None
    }

    /// Send the page `view`, and show the window as `shows` says.
    pub fn sync(&mut self, view: &impl Serialize, shows: Shows) {
        let Shows { placement, clickable, takes_keyboard } = shows;
        self.view.send(&self.app, view);
        self.shows_on_top.store(placement.is_some(), Ordering::Relaxed);
        let dragging = self.gesture.under_way(Instant::now()) == Some(Gesture::Drag);
        if let Some(placement) = placement {
            // While the player drags it the position is theirs; once they let go it goes
            // where the core says (on a monitor, clear of the captured regions).
            place(&self.window, placement, dragging);
        }
        // Not taking clicks, the page draws nothing there and they pass through.
        if self.clickable != clickable {
            self.clickable = clickable;
            if let Err(e) = self.window.set_ignore_cursor_events(!clickable) {
                error!("{} click-through toggle failed: {e}", self.spec.label);
            }
        }
        // Only an open field takes the typing. Otherwise the window can't be activated
        // (WS_EX_NOACTIVATE), so a click on it leaves the keyboard with the game.
        if self.takes_keyboard != takes_keyboard {
            let _ = self.window.set_focusable(takes_keyboard);
            // The field opens right after the player's click on the window, so Windows lets
            // us take the foreground; when it closes, Windows gives focus back to the
            // window below, the game.
            if takes_keyboard {
                search::take_focus(&self.window);
            } else {
                search::give_focus_back(&self.window);
            }
            self.takes_keyboard = takes_keyboard;
        }
    }

    /// The window has the keyboard already, but another of ours took the foreground (setup,
    /// whose Change hotkeys opens the Settings window again): it comes back in front, right
    /// after the player's click, as it did when it took the keyboard.
    pub fn take_focus_again(&self) {
        if self.takes_keyboard {
            search::take_focus(&self.window);
        }
    }

    /// Before a capture of the game while the window takes clicks (it shows a card, or
    /// badges): move it off the screen now, so it is never read in place of an Item Tooltip
    /// it covers; whether it moved, so the capture waits for the screen to be drawn without
    /// it ([`crate::item_card::OFF_SCREEN_SETTLE`]). The next sync puts it back where the core
    /// says.
    pub fn out_of_the_capture(&mut self) -> bool {
        self.clickable && self.window.set_position(OUT_OF_SIGHT).is_ok()
    }
}

/// CSS px to physical px, rounded up so a page's content is never clipped.
pub(crate) fn physical((width, height): (f64, f64), scale: f64) -> Size {
    Size { width: (width * scale).ceil() as u32, height: (height * scale).ceil() as u32 }
}

/// Size our window to the core's placement, and move it there unless the player is
/// dragging it.
pub(crate) fn place(window: &WebviewWindow, placement: ScreenRect, player_drags_it: bool) {
    let size = PhysicalSize::new(placement.width, placement.height);
    if window.inner_size().ok() != Some(size) {
        let _ = window.set_size(size);
    }
    let to = PhysicalPosition::new(placement.x, placement.y);
    if !player_drags_it && window.outer_position().ok() != Some(to) {
        let _ = window.set_position(to);
    }
}

/// Keep our `window` on top of every other window while the core shows it (the flag
/// returned, which the core loop sets) and the game or one of our windows is in front,
/// and out of sight otherwise: on a thread of its own (named `name`), at its own steady
/// pace. On top, it stays above our windows `beneath` too. The core decides what shows, but
/// the core loop can stall for a while (setting window sizes and positions), and a window on
/// top of everything must not sit over another app meanwhile: this watcher checks what is in
/// front itself, takes the window out of sight within a tick of another app coming to the
/// front, and tells the core loop (`commands`) to check the foreground at once. It gets the
/// window's handle once, here: asking Tauri for it from another thread waits on the main
/// thread's event loop, which can leave the watcher idle for seconds while another app is in
/// front.
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
/// The core alone decides what shows, and `shows` is its last word. The watcher takes the
/// core's own rule ([`minerva_core::shows_on_top`], which the panel's layer follows) to what
/// is in front now, as a fast path: the core loop reports the foreground to the core every
/// tick but can stall (setting window sizes and positions), and a window on top of
/// everything must not sit over another app while it does.
#[cfg(windows)]
fn keep_on_top_while_in_front(window: isize, beneath: Vec<isize>, shows: Arc<AtomicBool>, commands: Sender<Command>) {
    use crate::gamewin::{self, Stacking};
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
        // The core's rule, from what is in front now rather than at the core loop's last
        // report, which a stall can leave behind.
        let in_front = || {
            let game = gamewin::find_game();
            minerva_core::shows_on_top(game.is_some(), game.map(|game| gamewin::foreground(Some(game.handle))))
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
