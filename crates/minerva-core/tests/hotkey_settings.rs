//! The hotkeys in the Settings window: the gear at the top of the panel opens it (see
//! `settings_window.rs`), and there the player rebinds a hotkey by clicking its Change button
//! and pressing the new combination. It
//! needs Ctrl, Alt or Win, and can't be another hotkey's (Swap trades the two); Esc
//! cancels, and Backspace, Delete or Turn off turns the hotkey off. While the player
//! rebinds, the hotkeys aren't live, so the combination reaches the window rather than
//! acting. Every change is saved at once and acts at once; Restore defaults can be undone
//! until the player changes another.

mod common;

use common::FakeSource;
use minerva_core::{
    Action, Minerva, Foreground, Hotkey, Hotkeys, KeyPress, PanelControl, RebindProblem, Settings, SettingsNotice,
    SettingsWindow,
};

/// The key `code` (a `KeyboardEvent.code`) pressed with the modifiers in `held`, such as
/// `"ctrl+shift"`.
fn press(held: &str, code: &str) -> KeyPress {
    let has = |modifier: &str| held.split('+').any(|m| m == modifier);
    KeyPress { code: code.into(), ctrl: has("ctrl"), alt: has("alt"), shift: has("shift"), win: has("win") }
}

/// A core with the game window found and in front: the Settings window shows over it.
fn over_the_game() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(Some(common::window_on_wide_screen(common::GAME)));
    minerva.on_foreground(Foreground::Game);
    minerva
}

/// A core with the game in front and the Settings window open.
fn settings_open() -> Minerva {
    let mut minerva = over_the_game();
    minerva.press(PanelControl::Settings);
    minerva
}

fn view(minerva: &Minerva) -> SettingsWindow {
    minerva.panel_state().settings_window.clone().expect("the Settings window is open")
}

/// The hotkey waiting for its new combination, and why the last one pressed wasn't taken.
fn waiting(minerva: &Minerva) -> (Option<Hotkey>, Option<RebindProblem>) {
    let view = view(minerva);
    (view.rebinding, view.problem)
}

fn saved(hotkey: Hotkey) -> Option<SettingsNotice> {
    Some(SettingsNotice::Saved { hotkey, swapped_with: None })
}

#[test]
fn the_gear_opens_the_settings_window_with_no_hotkey_waiting() {
    let mut minerva = over_the_game();
    assert_eq!(minerva.panel_state().settings_window, None);

    assert_eq!(minerva.press(PanelControl::Settings), None);

    let view = view(&minerva);
    assert_eq!((view.rebinding, view.problem, view.notice, view.at_defaults), (None, None, None, true));
}

#[test]
fn its_close_button_or_esc_closes_the_settings_window() {
    let mut minerva = settings_open();
    minerva.act(Action::CloseSettings);
    assert_eq!(minerva.panel_state().settings_window, None);
}

#[test]
fn the_new_combination_acts_at_once_and_the_old_one_does_nothing() {
    let mut minerva = settings_open();

    minerva.act(Action::BeginRebind(Hotkey::TogglePanel));
    assert_eq!(waiting(&minerva), (Some(Hotkey::TogglePanel), None));
    let state = minerva.rebind_key(press("ctrl+shift", "KeyK")).clone();

    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+k");
    let view = state.settings_window.expect("still open");
    assert_eq!((view.rebinding, view.notice), (None, saved(Hotkey::TogglePanel)), "and says it is saved");
    assert_eq!(minerva.on_hotkey("ctrl+shift+c"), None);
    assert!(minerva.panel_state().expanded);
    minerva.on_hotkey("ctrl+shift+k");
    assert!(!minerva.panel_state().expanded);
}

#[test]
fn a_rebound_hotkey_is_saved_and_the_others_are_kept() {
    let mut minerva = settings_open();

    minerva.act(Action::BeginRebind(Hotkey::ItemLookup));
    minerva.rebind_key(press("alt", "F5"));

    let saved = minerva.settings().hotkeys;
    assert_eq!(saved, Hotkeys { item_lookup: "alt+f5".into(), ..Hotkeys::default() });
    let mut later = Minerva::new(FakeSource::osms());
    later.restore(common::settings_after_restart(&minerva));
    assert_eq!(later.panel_state().hotkeys.item_lookup, "alt+f5", "and kept at the next launch");
}

#[test]
fn the_hotkeys_are_not_live_while_the_player_rebinds_one() {
    let mut minerva = settings_open();
    assert!(minerva.panel_state().hotkeys_live);

    minerva.act(Action::BeginRebind(Hotkey::Search));
    assert!(!minerva.panel_state().hotkeys_live, "the combination pressed must reach the window");
    minerva.rebind_key(press("ctrl+alt", "KeyF"));
    assert!(minerva.panel_state().hotkeys_live, "live again, the new one with them");

    minerva.act(Action::BeginRebind(Hotkey::Search));
    minerva.act(Action::CancelRebind);
    assert!(minerva.panel_state().hotkeys_live);

    minerva.act(Action::BeginRebind(Hotkey::Search));
    minerva.on_foreground(Foreground::Minerva);
    assert!(!minerva.panel_state().hotkeys_live, "whatever is in front");
}

#[test]
fn a_modifier_pressed_on_its_own_waits_for_the_key() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Setup));

    minerva.rebind_key(press("ctrl", "ControlLeft"));
    minerva.rebind_key(press("ctrl+shift", "ShiftLeft"));

    assert_eq!(waiting(&minerva), (Some(Hotkey::Setup), None));
    assert_eq!(minerva.panel_state().hotkeys.setup, "ctrl+shift+s");
}

#[test]
fn a_combination_without_ctrl_alt_or_win_is_refused_and_another_can_be_pressed() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Setup));

    // The game would never get Shift+K, or K, while the hotkeys are live: not even in chat.
    minerva.rebind_key(press("shift", "KeyK"));
    assert_eq!(
        waiting(&minerva),
        (Some(Hotkey::Setup), Some(RebindProblem::NoModifier { pressed: "shift+k".into() }))
    );
    minerva.rebind_key(press("", "KeyK"));
    assert_eq!(view(&minerva).problem, Some(RebindProblem::NoModifier { pressed: "k".into() }));
    assert_eq!(minerva.panel_state().hotkeys.setup, "ctrl+shift+s");

    minerva.rebind_key(press("win+shift", "KeyK"));
    assert_eq!(waiting(&minerva), (None, None));
    assert_eq!(minerva.panel_state().hotkeys.setup, "super+shift+k");
}

#[test]
fn another_hotkeys_combination_is_refused_naming_that_hotkey() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::TogglePanel));

    minerva.rebind_key(press("ctrl+shift", "KeyF"));

    assert_eq!(
        waiting(&minerva),
        (
            Some(Hotkey::TogglePanel),
            Some(RebindProblem::InUse { pressed: "ctrl+shift+f".into(), by: Hotkey::Search })
        )
    );
    assert_eq!(minerva.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn swap_gives_the_combination_to_the_hotkey_being_rebound_and_its_old_one_to_the_other() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::TogglePanel));
    minerva.rebind_key(press("ctrl+shift", "KeyF"));

    let state = minerva.act(Action::SwapHotkeys).clone();

    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+f");
    assert_eq!(state.hotkeys.search, "ctrl+shift+c");
    let view = state.settings_window.expect("still open");
    assert_eq!((view.rebinding, view.problem), (None, None));
    assert_eq!(view.notice, Some(SettingsNotice::Saved { hotkey: Hotkey::TogglePanel, swapped_with: Some(Hotkey::Search) }));
    minerva.on_hotkey("ctrl+shift+f");
    assert!(!minerva.panel_state().expanded, "Ctrl+Shift+F collapses the panel now");
}

#[test]
fn swap_does_nothing_unless_the_combination_pressed_is_another_hotkeys() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::TogglePanel));
    minerva.rebind_key(press("shift", "KeyF"));

    minerva.act(Action::SwapHotkeys);

    assert_eq!(minerva.panel_state().hotkeys, Hotkeys::default());
    assert_eq!(view(&minerva).rebinding, Some(Hotkey::TogglePanel), "still waiting");
}

#[test]
fn pressing_a_hotkeys_own_combination_keeps_it() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::GrindTracker));

    minerva.rebind_key(press("ctrl+shift", "KeyG"));

    assert_eq!(waiting(&minerva), (None, None));
    assert_eq!(minerva.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn a_key_no_hotkey_can_use_is_refused() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Search));

    for code in ["IntlBackslash", "CapsLock", "ContextMenu", "Key", "KeyAB", "F25", "Digit10"] {
        minerva.rebind_key(press("ctrl", code));
        assert_eq!(view(&minerva).problem, Some(RebindProblem::UnsupportedKey { code: code.into() }), "{code}");
    }
    assert_eq!(view(&minerva).rebinding, Some(Hotkey::Search));
    assert_eq!(minerva.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn keys_beyond_letters_are_named_as_hotkey_registration_names_them() {
    let cases = [
        ("ctrl", "Digit1", "ctrl+1"),
        ("ctrl+alt", "F12", "ctrl+alt+f12"),
        ("alt", "Numpad3", "alt+numpad3"),
        ("ctrl", "NumpadAdd", "ctrl+numpadadd"),
        ("ctrl+shift", "BracketLeft", "ctrl+shift+bracketleft"),
        ("win+ctrl+alt+shift", "Space", "super+ctrl+alt+shift+space"),
        ("ctrl", "ArrowUp", "ctrl+arrowup"),
        ("ctrl", "Backspace", "ctrl+backspace"),
    ];
    for (held, code, combination) in cases {
        let mut minerva = settings_open();
        minerva.act(Action::BeginRebind(Hotkey::ChooseCandidate));

        minerva.rebind_key(press(held, code));

        assert_eq!(minerva.panel_state().hotkeys.choose_candidate, combination, "{held} {code}");
    }
}

#[test]
fn esc_stops_waiting_and_changes_nothing() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Search));
    minerva.rebind_key(press("shift", "KeyQ"));

    minerva.rebind_key(press("", "Escape"));

    assert_eq!(waiting(&minerva), (None, None), "the window stays, its problem gone");
    assert_eq!(view(&minerva).notice, None, "nothing was saved");
    assert_eq!(minerva.panel_state().hotkeys, Hotkeys::default());
}

#[test]
fn closing_the_settings_window_mid_rebind_changes_nothing_and_the_hotkeys_are_live_again() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Search));

    minerva.act(Action::CloseSettings);
    minerva.rebind_key(press("ctrl", "KeyQ"));

    let state = minerva.panel_state();
    assert_eq!(state.settings_window, None);
    assert_eq!(state.hotkeys, Hotkeys::default());
    assert!(state.hotkeys_live);
}

#[test]
fn keys_pressed_while_no_hotkey_waits_change_nothing() {
    let mut minerva = settings_open();
    let before = minerva.panel_state().clone();

    minerva.rebind_key(press("ctrl", "KeyQ"));
    minerva.rebind_key(press("", "Backspace"));

    assert_eq!(*minerva.panel_state(), before);
}

#[test]
fn backspace_delete_or_turn_off_turns_a_hotkey_off_and_its_old_combination_does_nothing() {
    for turn_off in ["Backspace", "Delete", "button"] {
        let mut minerva = settings_open();
        minerva.act(Action::BeginRebind(Hotkey::TogglePanel));

        let state = if turn_off == "button" {
            minerva.act(Action::TurnOffHotkey).clone()
        } else {
            minerva.rebind_key(press("", turn_off)).clone()
        };

        assert_eq!(state.hotkeys.toggle_panel, "", "{turn_off}");
        assert!(!state.hotkeys.all().contains(&"ctrl+shift+c"), "{turn_off}: not registered");
        assert_eq!(state.hotkeys.all().len(), 9, "the other nine");
        let view = state.settings_window.expect("still open");
        assert_eq!((view.rebinding, view.notice), (None, saved(Hotkey::TogglePanel)));
        assert_eq!(minerva.on_hotkey("ctrl+shift+c"), None);
        assert!(minerva.panel_state().expanded, "{turn_off}: the old combination does nothing");
        minerva.press(PanelControl::TogglePanel);
        assert!(!minerva.panel_state().expanded, "{turn_off}: its button still works");
    }
}

#[test]
fn shift_with_backspace_is_a_combination_to_refuse_not_a_way_to_turn_off() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Search));

    minerva.rebind_key(press("shift", "Backspace"));

    assert_eq!(view(&minerva).problem, Some(RebindProblem::NoModifier { pressed: "shift+backspace".into() }));
    assert_eq!(minerva.panel_state().hotkeys.search, "ctrl+shift+f");
}

#[test]
fn a_hotkey_turned_off_stays_off_at_the_next_launch_and_can_be_turned_on_again() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::ItemLookup));
    minerva.rebind_key(press("", "Delete"));

    let mut later = over_the_game();
    later.restore(common::settings_after_restart(&minerva));
    assert_eq!(later.panel_state().hotkeys.item_lookup, "", "still off");
    assert!(later.panel_state().hotkeys.all().iter().all(|hotkey| !hotkey.is_empty()));

    later.press(PanelControl::Settings);
    later.act(Action::BeginRebind(Hotkey::ItemLookup));
    later.rebind_key(press("ctrl+shift", "KeyL"));
    assert_eq!(later.panel_state().hotkeys.item_lookup, "ctrl+shift+l");
}

#[test]
fn hotkeys_turned_off_are_never_in_the_way_of_another_combination() {
    let mut minerva = settings_open();
    for hotkey in [Hotkey::Search, Hotkey::Setup] {
        minerva.act(Action::BeginRebind(hotkey));
        minerva.rebind_key(press("", "Backspace"));
    }

    minerva.act(Action::BeginRebind(Hotkey::TogglePanel));
    minerva.rebind_key(press("ctrl+shift", "KeyF"));

    assert_eq!(waiting(&minerva), (None, None), "Search's old combination is free");
    assert_eq!(minerva.panel_state().hotkeys.toggle_panel, "ctrl+shift+f");
}

#[test]
fn swapping_with_a_hotkey_that_was_off_turns_the_other_off() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Setup));
    minerva.rebind_key(press("", "Backspace"));

    minerva.act(Action::BeginRebind(Hotkey::Setup));
    minerva.rebind_key(press("ctrl+shift", "KeyF"));
    minerva.act(Action::SwapHotkeys);

    let hotkeys = &minerva.panel_state().hotkeys;
    assert_eq!((hotkeys.setup.as_str(), hotkeys.search.as_str()), ("ctrl+shift+f", ""));
}

#[test]
fn starting_another_change_clears_the_last_ones_notice() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::Search));
    minerva.rebind_key(press("ctrl+alt", "KeyF"));
    assert_eq!(view(&minerva).notice, saved(Hotkey::Search));

    minerva.act(Action::BeginRebind(Hotkey::Setup));

    assert_eq!(view(&minerva).notice, None);
}

#[test]
fn restore_defaults_puts_every_hotkey_back_and_on() {
    let mut minerva = over_the_game();
    let hotkeys = Hotkeys { toggle_panel: "ctrl+shift+x".into(), search: "".into(), ..Hotkeys::default() };
    minerva.restore(Settings { hotkeys, ..Settings::default() });
    minerva.press(PanelControl::Settings);
    assert!(!view(&minerva).at_defaults);
    minerva.act(Action::BeginRebind(Hotkey::Setup));

    let state = minerva.act(Action::ResetHotkeys).clone();

    assert_eq!(state.hotkeys, Hotkeys::default());
    let window = state.settings_window.expect("still open");
    assert_eq!(
        (window.rebinding, window.problem, window.notice, window.at_defaults),
        (None, None, Some(SettingsNotice::Restored), true)
    );
    assert_eq!(minerva.settings().hotkeys, Hotkeys::default(), "and saved");
}

#[test]
fn undo_after_restore_defaults_brings_the_hotkeys_from_before_back() {
    let mut minerva = over_the_game();
    let mine = Hotkeys { toggle_panel: "ctrl+shift+x".into(), search: "".into(), ..Hotkeys::default() };
    minerva.restore(Settings { hotkeys: mine.clone(), ..Settings::default() });
    minerva.press(PanelControl::Settings);
    minerva.act(Action::ResetHotkeys);

    let state = minerva.act(Action::UndoResetHotkeys).clone();

    assert_eq!(state.hotkeys, mine);
    let view = state.settings_window.expect("still open");
    assert_eq!((view.notice, view.at_defaults), (Some(SettingsNotice::RestoreUndone), false));
    assert_eq!(minerva.settings().hotkeys, mine, "and saved");
    minerva.act(Action::UndoResetHotkeys);
    assert_eq!(minerva.panel_state().hotkeys, mine, "once");
}

#[test]
fn restore_defaults_can_no_longer_be_undone_once_another_change_starts_or_the_window_closes() {
    for after in [Action::BeginRebind(Hotkey::Search), Action::CloseSettings] {
        let mut minerva = over_the_game();
        let mine = Hotkeys { toggle_panel: "ctrl+shift+x".into(), ..Hotkeys::default() };
        minerva.restore(Settings { hotkeys: mine, ..Settings::default() });
        minerva.press(PanelControl::Settings);
        minerva.act(Action::ResetHotkeys);

        minerva.act(after);
        minerva.press(PanelControl::Settings);
        minerva.act(Action::UndoResetHotkeys);

        assert_eq!(minerva.panel_state().hotkeys, Hotkeys::default(), "{after:?}");
    }
}

#[test]
fn restore_defaults_with_every_hotkey_at_its_default_changes_and_says_nothing() {
    let mut minerva = settings_open();
    assert!(view(&minerva).at_defaults);

    minerva.act(Action::ResetHotkeys);

    assert_eq!(view(&minerva).notice, None);
}

#[test]
fn the_page_gets_the_settings_window_with_the_names_it_reads() {
    let mut minerva = settings_open();
    minerva.act(Action::BeginRebind(Hotkey::TogglePanel));
    minerva.rebind_key(press("ctrl+shift", "KeyF"));
    let refused = serde_json::to_string(&minerva.panel_state().settings_window).unwrap();
    minerva.act(Action::SwapHotkeys);
    let swapped = serde_json::to_string(&minerva.panel_state().settings_window).unwrap();
    minerva.act(Action::ResetHotkeys);
    let restored = serde_json::to_string(&minerva.panel_state().settings_window).unwrap();

    assert!(refused.contains(r#""problem":{"kind":"inUse","pressed":"ctrl+shift+f","by":"search"}"#), "{refused}");
    assert!(refused.contains(r#""rebinding":"togglePanel""#), "{refused}");
    assert!(
        swapped.contains(r#""notice":{"kind":"saved","hotkey":"togglePanel","swappedWith":"search"}"#),
        "{swapped}"
    );
    assert!(swapped.contains(r#""atDefaults":false"#), "{swapped}");
    assert!(restored.contains(r#""notice":{"kind":"restored"}"#), "{restored}");
    assert!(restored.contains(r#""atDefaults":true"#), "{restored}");
}

#[test]
fn a_combination_rebound_away_from_is_no_longer_reported_unavailable() {
    let mut minerva = over_the_game();
    let hotkeys = Hotkeys { setup: "ctrl+shift+banana".into(), ..Hotkeys::default() };
    minerva.restore(Settings { hotkeys, ..Settings::default() });
    minerva.hotkeys_unavailable(vec!["ctrl+shift+n".into()], vec!["ctrl+shift+banana".into()]);
    minerva.press(PanelControl::Settings);

    minerva.act(Action::BeginRebind(Hotkey::ChooseCandidate));
    minerva.rebind_key(press("ctrl+alt", "KeyN"));
    let state = minerva.panel_state();
    assert_eq!(state.unavailable_hotkeys, ["ctrl+shift+banana"], "the shell reports on Ctrl+Alt+N once it registers it");
    assert_eq!(state.invalid_hotkeys, ["ctrl+shift+banana"]);

    let state = minerva.act(Action::ResetHotkeys);
    assert!(state.unavailable_hotkeys.is_empty());
    assert!(state.invalid_hotkeys.is_empty());
}
