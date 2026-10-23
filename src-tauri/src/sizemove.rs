//! The player's drags and resizes of our borderless windows, as Windows runs them: the
//! overlay's (dragged by its header or collapsed icon, resized sideways while expanded), and
//! those of the Grind Tracker's, the Personal Bests' and the Item Card's windows (dragged by
//! their bars, never resized). The page starts a drag from its header and a resize from a
//! corner grip, and Tauri's own borders around a resizable borderless window start a resize
//! from any edge; all of them by sending the window a mouse press on its frame
//! (`WM_NCLBUTTONDOWN`). Windows then runs its move or size loop (`WM_ENTERSIZEMOVE` to
//! `WM_EXITSIZEMOVE`), and the page hears nothing when the mouse is let go. Hearing these
//! means subclassing our own windows, nothing of the game's (ADR 0001). A
//! [`GestureTracker`] follows each window's drag or resize from the press to the loop's end.

use crate::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};
use tauri::WebviewWindow;

/// A drag or resize the player started whose move or size loop hasn't started within this
/// long never will (the mouse was let go first): the shell places the window again.
pub const LOOP_START_GRACE: Duration = Duration::from_secs(1);

/// A drag or resize of one of our windows by the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    Drag,
    Resize,
}

/// What Windows tells of a drag or resize of one of our windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeMove {
    /// The player pressed the mouse to drag or resize it.
    Pressed(Gesture),
    /// Windows' move or size loop on it started: a drag or resize is running.
    LoopStarted,
    /// Windows' move or size loop on it ended: the player let go of the mouse.
    LoopEnded,
}

/// The player's drag or resize of one window: from the mouse press that starts it until
/// Windows' move or size loop that runs it ends. A loop the player didn't start (the
/// system's own) isn't theirs.
#[derive(Debug, Default)]
pub struct GestureTracker {
    gesture: Option<PlayerGesture>,
}

#[derive(Debug, Clone, Copy)]
struct PlayerGesture {
    gesture: Gesture,
    /// When the player pressed the mouse to start it, until Windows' loop has started too.
    starting_since: Option<Instant>,
}

impl GestureTracker {
    /// The player pressed the mouse, at `now`, to start `gesture`.
    pub fn pressed(&mut self, gesture: Gesture, now: Instant) {
        self.gesture = Some(PlayerGesture { gesture, starting_since: Some(now) });
    }

    /// Windows' move or size loop started, at `now`: the drag or resize the player started
    /// is running.
    pub fn loop_started(&mut self, now: Instant) {
        self.under_way(now);
        if let Some(player) = &mut self.gesture {
            player.starting_since = None;
        }
    }

    /// Windows' move or size loop ended: the player let go of the mouse. The drag or resize
    /// that ended, if the loop was the player's.
    pub fn loop_ended(&mut self) -> Option<Gesture> {
        self.gesture.take_if(|player| player.starting_since.is_none()).map(|player| player.gesture)
    }

    /// The player's drag or resize at `now`, from the mouse press until they let go. One
    /// whose loop hasn't started within [`LOOP_START_GRACE`] never will.
    pub fn under_way(&mut self, now: Instant) -> Option<Gesture> {
        let starting_since = self.gesture.and_then(|player| player.starting_since);
        if starting_since.is_some_and(|at| now.saturating_duration_since(at) >= LOOP_START_GRACE) {
            self.gesture = None;
        }
        self.gesture.map(|player| player.gesture)
    }

    /// The player's drag or resize while Windows' loop runs it.
    pub fn running(&self) -> Option<Gesture> {
        self.gesture.filter(|player| player.starting_since.is_none()).map(|player| player.gesture)
    }
}

/// What a watched window's subclass needs: where to send what it hears, as which
/// command, and whether the player may resize the window now (`None`: never).
struct Watcher {
    commands: Sender<Command>,
    command: Box<dyn Fn(SizeMove) -> Command>,
    resizable: Option<&'static AtomicBool>,
}

/// Whether the player can resize the overlay now: only the expanded panel, never the
/// collapsed icon.
pub static OVERLAY_RESIZABLE: AtomicBool = AtomicBool::new(false);

pub fn set_overlay_resizable(resizable: bool) {
    OVERLAY_RESIZABLE.store(resizable, Ordering::Relaxed);
}

/// Send `command(SizeMove::Pressed(_))` as the player starts a drag or resize of
/// `window`, and `command(SizeMove::LoopStarted)` and `command(SizeMove::LoopEnded)` as
/// Windows' loop that runs it starts and ends. A window that isn't `resizable` (now) is
/// only ever dragged. Call it on the thread that owns the window (the main thread, as in
/// setup).
#[cfg(windows)]
pub fn watch(
    window: &WebviewWindow,
    commands: Sender<Command>,
    command: impl Fn(SizeMove) -> Command + 'static,
    resizable: Option<&'static AtomicBool>,
) -> Result<(), String> {
    use windows::Win32::UI::Shell::SetWindowSubclass;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    // Our windows last as long as the app, and so does what their subclass is given.
    let watcher = Box::into_raw(Box::new(Watcher { commands, command: Box::new(command), resizable }));
    if unsafe { SetWindowSubclass(hwnd, Some(on_message), 1, watcher as usize) }.as_bool() {
        Ok(())
    } else {
        // Not subclassed: nothing will use it.
        drop(unsafe { Box::from_raw(watcher) });
        Err(format!("couldn't watch the {} window's drags and resizes", window.label()))
    }
}

#[cfg(windows)]
unsafe extern "system" fn on_message(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    watcher: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::{LRESULT, WPARAM};
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{
        HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
        WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCLBUTTONDOWN,
    };
    // What `watch` leaked for this subclass.
    let watcher = unsafe { &*(watcher as *const Watcher) };
    let send = |what| {
        let _ = watcher.commands.send((watcher.command)(what));
    };
    let mut wparam = wparam;
    match msg {
        WM_NCLBUTTONDOWN => match wparam.0 as u32 {
            HTCAPTION => send(SizeMove::Pressed(Gesture::Drag)),
            hit @ (HTLEFT | HTTOPLEFT | HTBOTTOMLEFT | HTRIGHT | HTTOPRIGHT | HTBOTTOMRIGHT | HTTOP | HTBOTTOM) => {
                // The panel's height follows its content: the player sets its width only,
                // by its left or right edge (a corner counts as its side). Nor do they
                // resize the collapsed icon.
                let side = match hit {
                    HTLEFT | HTTOPLEFT | HTBOTTOMLEFT => HTLEFT,
                    HTRIGHT | HTTOPRIGHT | HTBOTTOMRIGHT => HTRIGHT,
                    _ => return LRESULT(0),
                };
                if !watcher.resizable.is_some_and(|resizable| resizable.load(Ordering::Relaxed)) {
                    return LRESULT(0);
                }
                send(SizeMove::Pressed(Gesture::Resize));
                wparam = WPARAM(side as usize);
            }
            _ => {}
        },
        WM_ENTERSIZEMOVE => send(SizeMove::LoopStarted),
        WM_EXITSIZEMOVE => send(SizeMove::LoopEnded),
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(not(windows))]
pub fn watch(
    _window: &WebviewWindow,
    _commands: Sender<Command>,
    _command: impl Fn(SizeMove) -> Command + 'static,
    _resizable: Option<&'static AtomicBool>,
) -> Result<(), String> {
    Err("watching drags and resizes is only implemented on Windows so far".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_runs_from_the_press_until_windows_loop_ends_and_ends_once() {
        let (mut tracker, at) = (GestureTracker::default(), Instant::now());
        tracker.pressed(Gesture::Drag, at);
        assert_eq!(tracker.under_way(at), Some(Gesture::Drag));
        assert_eq!(tracker.running(), None, "not till the loop starts");

        tracker.loop_started(at + Duration::from_millis(30));
        assert_eq!(tracker.running(), Some(Gesture::Drag));
        // However long the player holds it.
        assert_eq!(tracker.under_way(at + 10 * LOOP_START_GRACE), Some(Gesture::Drag));

        assert_eq!(tracker.loop_ended(), Some(Gesture::Drag));
        assert_eq!(tracker.loop_ended(), None);
        assert_eq!(tracker.under_way(at), None);
    }

    #[test]
    fn a_press_whose_loop_never_starts_is_given_up_after_the_grace() {
        let (mut tracker, at) = (GestureTracker::default(), Instant::now());
        tracker.pressed(Gesture::Resize, at);
        assert_eq!(tracker.under_way(at + LOOP_START_GRACE / 2), Some(Gesture::Resize));
        assert_eq!(tracker.under_way(at + LOOP_START_GRACE), None);

        // A loop starting after that isn't the player's.
        tracker.loop_started(at + 2 * LOOP_START_GRACE);
        assert_eq!(tracker.running(), None);
        assert_eq!(tracker.loop_ended(), None);
    }

    #[test]
    fn a_loop_the_player_did_not_start_is_not_theirs() {
        let (mut tracker, at) = (GestureTracker::default(), Instant::now());
        tracker.loop_started(at);
        assert_eq!((tracker.running(), tracker.loop_ended()), (None, None));

        // Nor does a loop's end before its start end the player's press.
        tracker.pressed(Gesture::Drag, at);
        assert_eq!(tracker.loop_ended(), None);
        assert_eq!(tracker.under_way(at), Some(Gesture::Drag));
    }
}
