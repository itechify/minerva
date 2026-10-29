//! Game window locator, foreground check, and the switch that puts our windows on top of
//! everything or out of sight (Windows). Reads window-manager state only: it never sends
//! the game a message, opens its process or hooks it, and the only windows it ever
//! changes are our own (ADR 0001).

use minerva_core::{Foreground, ScreenRect};

/// The game window as found on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Game {
    /// The window handle as a number, only used to tell whether it is in front.
    pub handle: isize,
    /// The client area (what the game draws in) in physical screen pixels.
    pub client: ScreenRect,
    /// The visible frame (caption and borders, not the invisible resize borders).
    pub frame: ScreenRect,
}

/// The title [`find_game`] looks for, as the core says (the panel state's
/// `game_window_title`). Nothing is found until it is set.
static LOOK_FOR: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// Look for windows titled exactly `title` from now on.
pub fn look_for(title: &str) {
    let mut current = LOOK_FOR.write().unwrap_or_else(|e| e.into_inner());
    if *current != title {
        *current = title.to_string();
    }
}

/// The title [`find_game`] looks for.
fn looking_for() -> String {
    LOOK_FOR.read().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(windows)]
pub use win::{
    Stacking, below_any, cursor_position, find_game, foreground, foreground_is_ours, monitor_work_areas, restack,
    window_under_cursor,
};

#[cfg(not(windows))]
pub fn cursor_position() -> Option<minerva_core::ScreenPoint> {
    None
}

#[cfg(not(windows))]
pub fn foreground_is_ours() -> bool {
    false
}

/// Elsewhere the foreground can't be told apart yet, so the hotkeys stay live as before.
#[cfg(not(windows))]
pub fn foreground(_game: Option<isize>) -> Foreground {
    Foreground::Game
}

/// The game's client area, or `None` when it isn't running or is minimised.
pub fn find_game_client() -> Option<ScreenRect> {
    find_game().map(|game| game.client)
}

#[cfg(not(windows))]
pub fn find_game() -> Option<Game> {
    None
}

#[cfg(not(windows))]
pub fn window_under_cursor() -> Result<String, String> {
    Err("picking a window is only implemented on Windows so far".into())
}

#[cfg(not(windows))]
pub fn monitor_work_areas() -> Vec<ScreenRect> {
    Vec::new()
}

#[cfg(windows)]
mod win {
    use super::{Foreground, Game, ScreenRect};
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{
        DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
    };
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GA_ROOT, GW_HWNDPREV, GWL_EXSTYLE, GetAncestor, GetClientRect, GetCursorPos, GetForegroundWindow,
        GetWindow, GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, HWND_BOTTOM, HWND_NOTOPMOST,
        HWND_TOPMOST, IsIconic, IsWindowVisible, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, WS_EX_TOPMOST,
        WindowFromPoint,
    };
    use windows::core::BOOL;

    /// The first visible, uncloaked, not minimised top-level window titled exactly
    /// "MapleStory" (or as picked by hand). `None` when the game isn't running or is
    /// minimised.
    pub fn find_game() -> Option<Game> {
        let title = super::looking_for();
        let mut search: (&str, Vec<HWND>) = (&title, Vec::new());
        unsafe {
            let _ = EnumWindows(Some(collect), LPARAM(&mut search as *mut (&str, Vec<HWND>) as isize));
        }
        search.1.into_iter().find_map(game)
    }

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let (title, found) = unsafe { &mut *(lparam.0 as *mut (&str, Vec<HWND>)) };
        if is_game_window(hwnd, title) {
            found.push(hwnd);
        }
        BOOL(1) // keep enumerating
    }

    /// Where the mouse pointer is on screen (physical px), for Item Lookup and the Deal
    /// Finder. Only reads window-manager state (ADR 0001).
    pub fn cursor_position() -> Option<minerva_core::ScreenPoint> {
        let mut at = POINT::default();
        unsafe { GetCursorPos(&mut at) }.ok()?;
        Some(minerva_core::ScreenPoint { x: at.x, y: at.y })
    }

    /// The title of the top-level window under the mouse pointer, for picking the game
    /// window by hand. Only reads window-manager state: the pointer position, which
    /// window is there, its root and its stored caption (ADR 0001).
    pub fn window_under_cursor() -> Result<String, String> {
        let mut at = POINT::default();
        unsafe { GetCursorPos(&mut at) }.map_err(|e| format!("the pointer's position couldn't be read ({e})"))?;
        let root = unsafe { GetAncestor(WindowFromPoint(at), GA_ROOT) };
        if root.is_invalid() {
            return Err("there's no window under the pointer".into());
        }
        if is_ours(root) {
            return Err("that's Minerva's own window".into());
        }
        match caption(root) {
            Some(title) if !title.trim().is_empty() => Ok(title),
            _ => Err("the window under the pointer has no title".into()),
        }
    }

    /// The caption stored by the window manager; no WM_GETTEXT is sent to another
    /// process's window.
    fn caption(hwnd: HWND) -> Option<String> {
        let mut buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
        (n > 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }

    fn is_game_window(hwnd: HWND, title: &str) -> bool {
        unsafe {
            // Ours first: reading the caption of one of our own windows sends it a message
            // and waits for our main thread, which the on-top watchers must never do.
            if !IsWindowVisible(hwnd).as_bool() || is_ours(hwnd) {
                return false;
            }
            if caption(hwnd).as_deref() != Some(title) {
                return false;
            }
            let mut cloaked: u32 = 0;
            let cloaked_ok = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut u32 as *mut _,
                size_of::<u32>() as u32,
            );
            cloaked_ok.is_err() || cloaked == 0
        }
    }

    fn game(hwnd: HWND) -> Option<Game> {
        unsafe {
            if IsIconic(hwnd).as_bool() {
                return None;
            }
            let mut client_rect = RECT::default();
            GetClientRect(hwnd, &mut client_rect).ok()?;
            let mut origin = POINT { x: 0, y: 0 };
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return None;
            }
            let width = (client_rect.right - client_rect.left) as u32;
            let height = (client_rect.bottom - client_rect.top) as u32;
            if width == 0 || height == 0 {
                return None;
            }
            let client = ScreenRect { x: origin.x, y: origin.y, width, height };
            // GetWindowRect would include the invisible resize borders (~7 px).
            let mut frame = RECT::default();
            let frame = match DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut frame as *mut RECT as *mut _,
                size_of::<RECT>() as u32,
            ) {
                Ok(()) => rect(frame),
                Err(_) => ScreenRect { x: client.x, y: client.y, width, height },
            };
            Some(Game { handle: hwnd.0 as isize, client, frame })
        }
    }

    fn rect(r: RECT) -> ScreenRect {
        ScreenRect { x: r.left, y: r.top, width: (r.right - r.left) as u32, height: (r.bottom - r.top) as u32 }
    }

    /// The work area (screen minus taskbar) of every monitor.
    pub fn monitor_work_areas() -> Vec<ScreenRect> {
        let mut areas: Vec<ScreenRect> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(work_area), LPARAM(&mut areas as *mut Vec<ScreenRect> as isize));
        }
        areas
    }

    unsafe extern "system" fn work_area(monitor: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        let areas = unsafe { &mut *(lparam.0 as *mut Vec<ScreenRect>) };
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            areas.push(rect(info.rcWork));
        }
        BOOL(1)
    }

    fn is_topmost(hwnd: HWND) -> bool {
        unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
    }

    fn is_ours(hwnd: HWND) -> bool {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid == unsafe { GetCurrentProcessId() }
    }

    /// Which window is in front: the `game` (its handle, if it was found), one of ours,
    /// or another app's. Only reads the window order.
    pub fn foreground(game: Option<isize>) -> Foreground {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground.is_invalid() {
            Foreground::OtherApp
        } else if game == Some(foreground.0 as isize) {
            Foreground::Game
        } else if is_ours(foreground) {
            Foreground::Minerva
        } else {
            Foreground::OtherApp
        }
    }

    /// Whether the foreground window is one of ours (e.g. focus only moved inside our
    /// window, into its webview), rather than the game or another app.
    pub fn foreground_is_ours() -> bool {
        let foreground = unsafe { GetForegroundWindow() };
        !foreground.is_invalid() && is_ours(foreground)
    }

    /// Where [`restack`] puts one of our windows in the z-order.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Stacking {
        /// On top of every other window, in the topmost band. Any process may make its
        /// own window topmost, so this works while the game has the foreground (Windows
        /// silently ignores a background process raising a normal window above the
        /// foreground one).
        OnTop,
        /// Out of sight: the bottom of the z-order, under every other window, which also
        /// takes it out of the topmost band. (`HWND_NOTOPMOST` alone would put it at the
        /// top of the normal windows, over the app in front.)
        Bottom,
    }

    /// Whether any of `others` (our own windows) is above our `window` in the z-order.
    /// Reads the window order only, walking up from `window`.
    pub fn below_any(window: HWND, others: &[HWND]) -> bool {
        if others.is_empty() {
            return false;
        }
        let mut above = unsafe { GetWindow(window, GW_HWNDPREV) };
        // However many windows there are, never walk for ever.
        for _ in 0..10_000 {
            match above {
                Ok(hwnd) if !hwnd.is_invalid() => {
                    if others.contains(&hwnd) {
                        return true;
                    }
                    above = unsafe { GetWindow(hwnd, GW_HWNDPREV) };
                }
                _ => return false,
            }
        }
        false
    }

    /// Put our `window` on top of every other window, or send it to the bottom, out of
    /// sight (`to`). Only our own window is moved, and only when it may not be where it
    /// should be: its style says otherwise, or the caller last put it elsewhere
    /// (`set_before`). (Tao rewrites our windows' extended style from its own flags
    /// whenever one of them changes, without `WS_EX_TOPMOST`, so the style alone can say
    /// a window on top isn't.) Returns whether it moved it.
    ///
    /// Put on top, the window first leaves the topmost band, then joins it. Coming back
    /// from the bottom as it shows again (another app in front, then the game), Windows
    /// can ignore a plain `HWND_TOPMOST` for it, returning success and leaving it under
    /// the game, and goes on ignoring it however often it is asked, from the window's own
    /// thread too; going by `HWND_NOTOPMOST` first puts it on top every time. Seen on
    /// whichever of our windows the player clicked last.
    ///
    /// No owner/parent link to the game (that would attach input queues across the
    /// processes, ADR 0001) and no event hook: the caller polls this.
    pub fn restack(window: HWND, to: Stacking, set_before: Option<Stacking>) -> windows::core::Result<bool> {
        if is_topmost(window) == (to == Stacking::OnTop) && set_before == Some(to) {
            return Ok(false);
        }
        let flags = SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE;
        match to {
            Stacking::OnTop => unsafe {
                SetWindowPos(window, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags)?;
                SetWindowPos(window, Some(HWND_TOPMOST), 0, 0, 0, 0, flags)?;
            },
            Stacking::Bottom => unsafe { SetWindowPos(window, Some(HWND_BOTTOM), 0, 0, 0, 0, flags)? },
        }
        Ok(true)
    }
}
