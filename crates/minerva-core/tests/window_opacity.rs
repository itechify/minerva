//! Window opacity: Settings' Opacity slider says how solid the panel and the Grind Tracker are
//! drawn over the game, from 30% to 100%. Every file without one, and a new player, has them
//! fully solid, as they always were. The pages fade the whole window and show it solid again
//! while the pointer is over it; the core only keeps the figure.

mod common;

use common::FakeSource;
use minerva_core::{Action, Minerva, Settings};

fn launched() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.tick(0);
    minerva
}

#[test]
fn the_windows_start_fully_solid() {
    let minerva = launched();

    assert_eq!(minerva.panel_state().window_opacity_percent, 100);
}

#[test]
fn the_slider_sets_how_solid_the_windows_are() {
    let mut minerva = launched();

    let state = minerva.act(Action::SetWindowOpacity { percent: 60 });

    assert_eq!(state.window_opacity_percent, 60);
}

#[test]
fn the_figure_stays_between_30_and_100_percent() {
    let mut minerva = launched();

    assert_eq!(minerva.act(Action::SetWindowOpacity { percent: 5 }).window_opacity_percent, 30, "at least 30%");
    assert_eq!(minerva.act(Action::SetWindowOpacity { percent: 250 }).window_opacity_percent, 100, "at most 100%");
}

#[test]
fn the_figure_is_kept_between_launches() {
    let mut minerva = launched();
    minerva.act(Action::SetWindowOpacity { percent: 45 });
    let saved = minerva.settings();

    let mut again = Minerva::new(FakeSource::osms());
    let state = again.restore(saved);

    assert_eq!(state.window_opacity_percent, 45);
}

#[test]
fn a_file_without_it_has_the_windows_fully_solid() {
    let settings: Settings = serde_json::from_str(r#"{"version": 8}"#).expect("an older file");

    let mut minerva = Minerva::new(FakeSource::osms());
    let state = minerva.restore(settings);

    assert_eq!(state.window_opacity_percent, 100);
}

#[test]
fn a_file_out_of_range_is_brought_into_it() {
    let settings: Settings = serde_json::from_str(r#"{"version": 8, "windowOpacityPercent": 0}"#).expect("a file");

    let mut minerva = Minerva::new(FakeSource::osms());
    let state = minerva.restore(settings);

    assert_eq!(state.window_opacity_percent, 30);
}
