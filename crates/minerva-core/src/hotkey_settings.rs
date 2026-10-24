//! The hotkeys in the Settings window ([`crate::settings_window`]): there the player rebinds
//! a hotkey by pressing its new key combination on the window. While they do, the hotkeys
//! are released (`PanelState::hotkeys_live`): a registered combination reaches no window,
//! the Settings window's included. A hotkey can also be turned off
//! (Backspace or Delete, or the waiting row's Turn off): it then has no combination, and its
//! action stays on its button.

use crate::panel::ScreenRect;
use crate::same_hotkey;
use crate::settings::{Hotkey, Hotkeys};
use crate::settings_window::SettingsWindow;
use serde::{Deserialize, Serialize};

/// A key the player pressed on the Settings window, as its page saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPress {
    /// The key itself, wherever the keyboard layout puts its letter: the web's
    /// `KeyboardEvent.code`, such as `"KeyK"`, `"Digit1"` or `"F5"`.
    pub code: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// The Windows key.
    pub win: bool,
}

/// Why a combination the player pressed can't be the hotkey they are rebinding. They can
/// press another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RebindProblem {
    /// Neither Ctrl, Alt nor Win was held: while the hotkeys are live the game would never
    /// get the key, not even to type it in its chat.
    NoModifier { pressed: String },
    /// A key no hotkey can use, by its `KeyboardEvent.code`.
    UnsupportedKey { code: String },
    /// Another of Minerva's hotkeys has the combination. Swap gives it to the hotkey
    /// being rebound, and that one's old keys to `by`.
    InUse { pressed: String, by: Hotkey },
}

/// What the player's last change in the Settings window came to. Every change is saved at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SettingsNotice {
    /// `hotkey` has new keys, or is off; with a swap, `swapped_with` took its old ones.
    Saved {
        hotkey: Hotkey,
        #[serde(rename = "swappedWith")]
        swapped_with: Option<Hotkey>,
    },
    /// Every hotkey is back to its default. Undo brings back the ones from before, until the
    /// player starts changing another or closes the window.
    Restored,
    /// Undo brought back the hotkeys from before Restore defaults.
    RestoreUndone,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HotkeySettings {
    /// `None` while the Settings window is closed.
    view: Option<OpenView>,
    /// The open window has the keyboard (see [`SettingsWindow::takes_keyboard`]).
    keyboard: bool,
    /// How many times the player opened the window (see [`SettingsWindow::opened`]).
    opened: u32,
}

#[derive(Debug, Clone, Default)]
struct OpenView {
    rebinding: Option<Hotkey>,
    problem: Option<RebindProblem>,
    notice: Option<SettingsNotice>,
    /// The hotkeys from before Restore defaults, while Undo can bring them back.
    before_restore: Option<Hotkeys>,
}

impl HotkeySettings {
    /// Open the Settings window, with no hotkey being rebound, or keep it as it is if it is
    /// open; either way it takes the keyboard.
    pub fn open(&mut self) {
        self.view.get_or_insert_with(OpenView::default);
        self.keyboard = true;
        self.opened = self.opened.wrapping_add(1);
    }

    /// Close the window; a rebinding under way is cancelled.
    pub fn close(&mut self) {
        self.view = None;
        self.keyboard = false;
    }

    pub fn is_open(&self) -> bool {
        self.view.is_some()
    }

    /// The open window takes the keyboard (the player clicked a field that needs it).
    pub fn take_keyboard(&mut self) {
        self.keyboard = self.view.is_some();
    }

    /// The keyboard went elsewhere: the game, or the search view. A rebinding under way is
    /// cancelled, as the keys pressed for it would no longer reach the window; it stays open.
    pub fn leave(&mut self) {
        self.keyboard = false;
        self.cancel();
    }

    /// Wait for the player to press `hotkey`'s new combination, with the keyboard. The last
    /// change's notice goes, and with it any Undo of Restore defaults.
    pub fn begin(&mut self, hotkey: Hotkey) {
        if let Some(view) = &mut self.view {
            *view = OpenView { rebinding: Some(hotkey), ..OpenView::default() };
            self.keyboard = true;
        }
    }

    /// Stop waiting, changing nothing.
    pub fn cancel(&mut self) {
        if let Some(view) = &mut self.view {
            view.rebinding = None;
            view.problem = None;
        }
    }

    pub fn is_rebinding(&self) -> bool {
        self.view.as_ref().is_some_and(|view| view.rebinding.is_some())
    }

    /// The window while it is open, at `placement` (`None`: hidden). It has the keyboard
    /// only while it shows.
    pub fn state(&self, hotkeys: &Hotkeys, placement: Option<ScreenRect>) -> Option<SettingsWindow> {
        self.view.as_ref().map(|view| SettingsWindow {
            placement,
            takes_keyboard: self.keyboard && placement.is_some(),
            opened: self.opened,
            rebinding: view.rebinding,
            problem: view.problem.clone(),
            notice: view.notice,
            at_defaults: *hotkeys == Hotkeys::default(),
        })
    }

    /// The player pressed `press` while rebinding: bind it in `hotkeys` if it can be, or
    /// say why not. Backspace or Delete on its own turns the hotkey off; a modifier on its
    /// own waits for the key; Esc cancels. Whether `hotkeys` changed.
    pub fn press(&mut self, press: &KeyPress, hotkeys: &mut Hotkeys) -> bool {
        let Some(view) = self.view.as_mut() else { return false };
        let Some(hotkey) = view.rebinding else { return false };
        if MODIFIER_CODES.contains(&press.code.as_str()) {
            return false;
        }
        if press.code == "Escape" {
            view.rebinding = None;
            view.problem = None;
            return false;
        }
        let alone = !(press.ctrl || press.alt || press.shift || press.win);
        if alone && matches!(press.code.as_str(), "Backspace" | "Delete") {
            return bind(view, hotkeys, hotkey, String::new(), None);
        }
        let Some(key) = key_name(&press.code) else {
            view.problem = Some(RebindProblem::UnsupportedKey { code: press.code.clone() });
            return false;
        };
        let pressed = [(press.win, "super"), (press.ctrl, "ctrl"), (press.alt, "alt"), (press.shift, "shift")]
            .into_iter()
            .filter_map(|(held, name)| held.then_some(name))
            .chain([key.as_str()])
            .collect::<Vec<_>>()
            .join("+");
        let in_use = Hotkey::ALL.into_iter().find(|&other| other != hotkey && same_hotkey(hotkeys.get(other), &pressed));
        let problem = if !(press.ctrl || press.alt || press.win) {
            RebindProblem::NoModifier { pressed }
        } else if let Some(by) = in_use {
            RebindProblem::InUse { pressed, by }
        } else {
            return bind(view, hotkeys, hotkey, pressed, None);
        };
        view.problem = Some(problem);
        false
    }

    /// The waiting row's Turn off: the hotkey has no combination any more. Whether
    /// `hotkeys` changed.
    pub fn turn_off(&mut self, hotkeys: &mut Hotkeys) -> bool {
        let Some(view) = self.view.as_mut() else { return false };
        let Some(hotkey) = view.rebinding else { return false };
        bind(view, hotkeys, hotkey, String::new(), None)
    }

    /// Swap, after the player pressed another hotkey's combination: the hotkey being
    /// rebound takes it, and the other takes its old keys (none, if it was off). Whether
    /// `hotkeys` changed.
    pub fn swap(&mut self, hotkeys: &mut Hotkeys) -> bool {
        let Some(view) = self.view.as_mut() else { return false };
        let (Some(hotkey), Some(RebindProblem::InUse { pressed, by })) = (view.rebinding, view.problem.clone()) else {
            return false;
        };
        hotkeys.set(by, hotkeys.get(hotkey).to_string());
        bind(view, hotkeys, hotkey, pressed, Some(by))
    }

    /// Restore defaults: every hotkey back to its default, and on again. Until the player
    /// starts changing another or closes the window, Undo brings back the ones from before.
    /// Whether `hotkeys` changed.
    pub fn restore(&mut self, hotkeys: &mut Hotkeys) -> bool {
        let before = std::mem::take(hotkeys);
        let changed = before != *hotkeys;
        if let Some(view) = &mut self.view {
            *view = OpenView {
                notice: changed.then_some(SettingsNotice::Restored),
                before_restore: changed.then_some(before),
                ..OpenView::default()
            };
        }
        changed
    }

    /// Undo, after Restore defaults: the hotkeys from before it are back. Whether `hotkeys`
    /// changed.
    pub fn undo_restore(&mut self, hotkeys: &mut Hotkeys) -> bool {
        let Some(view) = self.view.as_mut() else { return false };
        let Some(before) = view.before_restore.take() else { return false };
        *hotkeys = before;
        *view = OpenView { notice: Some(SettingsNotice::RestoreUndone), ..OpenView::default() };
        true
    }
}

/// Bind `hotkey` to `combination` (empty: off), and stop waiting: it is saved.
fn bind(view: &mut OpenView, hotkeys: &mut Hotkeys, hotkey: Hotkey, combination: String, swapped_with: Option<Hotkey>) -> bool {
    hotkeys.set(hotkey, combination);
    *view = OpenView { notice: Some(SettingsNotice::Saved { hotkey, swapped_with }), ..OpenView::default() };
    true
}

/// The modifier keys, which a combination holds rather than ends with.
const MODIFIER_CODES: [&str; 10] = [
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "ShiftLeft",
    "ShiftRight",
    "MetaLeft",
    "MetaRight",
    "OSLeft",
    "OSRight",
];

/// The keys besides letters, digits, F-keys and the number pad's digits that a hotkey can
/// end with, by `KeyboardEvent.code`: those the shell's hotkey registration knows, without
/// the lock and media keys.
const OTHER_KEYS: [&str; 34] = [
    "Backquote",
    "Backslash",
    "BracketLeft",
    "BracketRight",
    "Comma",
    "Equal",
    "Minus",
    "Period",
    "Quote",
    "Semicolon",
    "Slash",
    "Backspace",
    "Enter",
    "Space",
    "Tab",
    "Delete",
    "End",
    "Home",
    "Insert",
    "PageDown",
    "PageUp",
    "Pause",
    "PrintScreen",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "NumpadAdd",
    "NumpadDecimal",
    "NumpadDivide",
    "NumpadEnter",
    "NumpadEqual",
    "NumpadMultiply",
    "NumpadSubtract",
];

/// The key a hotkey's combination names for `code`: `"KeyK"` is `"k"`, `"Digit1"` is
/// `"1"`, `"F5"` is `"f5"`, `"BracketLeft"` is `"bracketleft"`. `None` for a key no
/// hotkey can use.
fn key_name(code: &str) -> Option<String> {
    let number = |prefix: &str| {
        let digits = code.strip_prefix(prefix).filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))?;
        digits.parse::<u32>().ok()
    };
    let letter = code.strip_prefix("Key").filter(|rest| rest.len() == 1 && rest.bytes().all(|b| b.is_ascii_uppercase()));
    if let Some(letter) = letter {
        return Some(letter.to_ascii_lowercase());
    }
    if let Some(digit) = number("Digit").filter(|&n| n <= 9) {
        return Some(digit.to_string());
    }
    let usable = number("F").is_some_and(|n| (1..=24).contains(&n))
        || number("Numpad").is_some_and(|n| n <= 9)
        || OTHER_KEYS.contains(&code);
    usable.then(|| code.to_ascii_lowercase())
}
