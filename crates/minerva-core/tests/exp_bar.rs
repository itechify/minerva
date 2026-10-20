//! EXP tracking setup (ADR 0005): after the Title Region, the player may mark the EXP
//! Bar Region around the figures on the in-game EXP bar ("EXP. 160 [3.81%]"). A test
//! read only accepts a mark whose figures read whole; skipping leaves tracking off.
//! Once kept, the panel state carries the region to capture and the latest reading.
//!
//! Worked example used throughout: the window samples' client areas shown at (812,110),
//! as in `setup.rs`. The EXP figures sit at the bottom centre of the client: on the
//! Kerning City sample (client 1916x1197) the "EXP." label spans client x 1018..1056,
//! y 1150..1167, and the figures run to x ~1154. The player marks (1012, h-52) 150x26.

mod common;

use common::FakeSource;
use minerva_core::{
    Action, Minerva, ExpBar, ExpBarTestRead, Frame, GameWindow, MarkTarget, PanelControl, Rect, ScreenRect, SetupReason,
};

const KERNING: &str = "victoria-road__kerning-city--window";

/// The client area of a window sample shown at (812,110).
fn game(sample: &str) -> ScreenRect {
    let client = common::client_of(&common::load(sample));
    ScreenRect { x: 812, y: 110, width: client.width(), height: client.height() }
}

/// The game window as the shell reports it, with this client area on a 3440x1440 screen.
fn window(client: ScreenRect) -> Option<GameWindow> {
    let frame = ScreenRect { x: client.x - 1, y: client.y - 31, width: client.width + 2, height: client.height + 32 };
    Some(GameWindow { client, frame, screens: vec![ScreenRect { x: 0, y: 0, width: 3440, height: 1440 }] })
}

/// The player's mark around the minimap title (as in `setup.rs`).
const TITLE_MARK: Rect = Rect { x: 70, y: 40, width: 130, height: 48 };

/// The player's mark around the EXP figures of a sample: the status bar is anchored to
/// the bottom of the client area.
fn exp_mark(sample: &str) -> Rect {
    Rect { x: 1012, y: game(sample).height - 52, width: 150, height: 26 }
}

/// Mark and test-read the minimap title of a sample, as in `setup.rs`.
fn mark_title(minerva: &mut Minerva, sample: &str) {
    let game = game(sample);
    minerva.on_game_window(window(game));
    let region = minerva.act(Action::MarkTitleRegion(TITLE_MARK)).title_region.expect("a title region");
    let client = common::client_of(&common::load(sample));
    minerva.on_frame(common::capture(&client, game, region, 0));
}

/// Capture the EXP Bar Region the core asks for from a sample.
fn exp_frame(minerva: &Minerva, sample: &str, at_ms: u64) -> Frame {
    let region = minerva.panel_state().exp_bar_region.expect("an EXP bar region to capture");
    let client = common::client_of(&common::load(sample));
    common::capture(&client, game(sample), region, at_ms)
}

/// Mark the EXP figures of a sample and test-read the first frame captured there.
fn mark_and_test_read_exp(minerva: &mut Minerva, sample: &str) -> Option<ExpBarTestRead> {
    minerva.act(Action::MarkExpBarRegion(exp_mark(sample)));
    let frame = exp_frame(minerva, sample, 0);
    minerva.on_exp_bar_frame(frame).setup.exp_bar_test_read.clone()
}

fn read(exp: u64, percent_hundredths: u32) -> Option<ExpBarTestRead> {
    Some(ExpBarTestRead::Read { bar: ExpBar { exp, percent_hundredths } })
}

/// A core set up on the Kerning City sample with the title only (EXP skipped).
fn set_up_without_exp() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);
    minerva.act(Action::FinishSetup);
    minerva
}

/// A core set up on the Kerning City sample with the title and the EXP bar.
fn set_up_with_exp() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);
    mark_and_test_read_exp(&mut minerva, KERNING);
    minerva.act(Action::FinishSetup);
    minerva
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(minerva: &Minerva) -> Minerva {
    let json = serde_json::to_string(&minerva.settings()).unwrap();
    let mut restarted = Minerva::new(FakeSource::osms());
    restarted.restore(serde_json::from_str(&json).unwrap());
    restarted
}

#[test]
fn setup_finished_without_marking_the_exp_bar_leaves_tracking_off() {
    let minerva = set_up_without_exp();
    let state = minerva.panel_state();

    assert_eq!(state.setup.needed, None);
    assert!(!state.setup.window_open);
    assert!(!state.setup.exp_bar_kept);
    assert_eq!(state.exp_bar_region, None);
    assert_eq!(state.exp_bar, None);
}

#[test]
fn marking_the_exp_bar_starts_a_test_read_there_keeping_the_marked_corner_and_height() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);

    let state = minerva.act(Action::MarkExpBarRegion(exp_mark(KERNING)));

    assert_eq!(state.setup.exp_bar_test_read, Some(ExpBarTestRead::Reading));
    let region = state.exp_bar_region.expect("a region to capture");
    // The client is at (812,110) on screen; Kerning City's client is 1197 tall.
    assert_eq!((region.x, region.y, region.height), (1824, 110 + 1145, 26));
    // Wide enough for 7-digit EXP: the figures grow to the right as the EXP does.
    assert!(region.width >= 12 * 26, "{region:?}");
}

#[test]
fn the_test_read_shows_the_exp_and_percentage_on_every_window_sample() {
    for (sample, exp, hundredths) in [
        ("victoria-road__kerning-city--window", 160, 381),
        ("victoria-road__sunset-sky--window", 160, 381),
        ("victoria-road__construction-site-north-of-kerning-city--window", 266, 633),
        ("hidden-street__northern-top-of-construction-site--window", 335, 798),
    ] {
        let mut minerva = Minerva::new(FakeSource::osms());
        mark_title(&mut minerva, sample);

        assert_eq!(mark_and_test_read_exp(&mut minerva, sample), read(exp, hundredths), "{sample}");
    }
}

/// A tight mark around the digits alone, and a loose one taking in the bar below and
/// the panel's edge above, read the same.
#[test]
fn tight_and_loose_marks_read_the_same_figures() {
    let h = game(KERNING).height;
    for mark in [
        Rect { x: 1056, y: h - 48, width: 100, height: 20 },
        Rect { x: 1000, y: h - 60, width: 230, height: 45 },
    ] {
        let mut minerva = Minerva::new(FakeSource::osms());
        mark_title(&mut minerva, KERNING);
        minerva.act(Action::MarkExpBarRegion(mark));
        let frame = exp_frame(&minerva, KERNING, 0);

        let state = minerva.on_exp_bar_frame(frame);

        assert_eq!(state.setup.exp_bar_test_read, read(160, 381), "{mark:?}");
    }
}

/// Live captures come out uniformly darker than the samples (`prep.rs`: ~0.935x).
#[test]
fn a_dimmer_capture_still_reads() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);
    minerva.act(Action::MarkExpBarRegion(exp_mark(KERNING)));
    let mut frame = exp_frame(&minerva, KERNING, 0);
    for px in frame.rgba.chunks_exact_mut(4) {
        for c in &mut px[..3] {
            *c = (*c as f32 * 0.9).round() as u8;
        }
    }

    let state = minerva.on_exp_bar_frame(frame);

    assert_eq!(state.setup.exp_bar_test_read, read(160, 381));
}

#[test]
fn a_mark_that_misses_the_figures_fails_its_test_read_and_setup_cannot_finish_with_it() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);
    // Around the minimap's map name instead.
    minerva.act(Action::MarkExpBarRegion(Rect { x: 70, y: 60, width: 150, height: 26 }));
    let frame = exp_frame(&minerva, KERNING, 0);
    assert_eq!(minerva.on_exp_bar_frame(frame).setup.exp_bar_test_read, Some(ExpBarTestRead::Failed));

    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, Some(SetupReason::FirstRun));
    assert_eq!(state.setup.exp_bar_test_read, Some(ExpBarTestRead::Failed));
}

#[test]
fn discarding_a_failed_mark_lets_setup_finish_with_tracking_off() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);
    minerva.act(Action::MarkExpBarRegion(exp_mark(KERNING)));
    minerva.on_exp_bar_frame(common::black_frame(0));

    minerva.act(Action::DiscardExpBarMark);
    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.setup.exp_bar_test_read, None);
    assert_eq!(state.exp_bar_region, None);
}

#[test]
fn finishing_setup_keeps_the_exp_bar_and_the_panel_shows_its_live_reading() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);
    mark_and_test_read_exp(&mut minerva, KERNING);
    let marked = minerva.panel_state().exp_bar_region;

    let state = minerva.act(Action::FinishSetup);
    assert_eq!(state.setup.needed, None);
    assert_eq!(state.setup.exp_bar_test_read, None);
    assert!(state.setup.exp_bar_kept);
    assert_eq!(state.exp_bar_region, marked);

    let frame = exp_frame(&minerva, KERNING, 1000);
    let state = minerva.on_exp_bar_frame(frame);
    assert_eq!(state.exp_bar, Some(ExpBar { exp: 160, percent_hundredths: 381 }));

    // A frame where the figures can't be read (covered) keeps the last reading.
    let state = minerva.on_exp_bar_frame(common::black_frame(2000));
    assert_eq!(state.exp_bar, Some(ExpBar { exp: 160, percent_hundredths: 381 }));
}

#[test]
fn the_exp_bar_region_survives_a_restart() {
    let minerva = set_up_with_exp();

    let mut restarted = restart(&minerva);
    let state = restarted.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, None);
    assert!(state.setup.exp_bar_kept);
    assert_eq!(state.exp_bar_region, minerva.panel_state().exp_bar_region);
}

#[test]
fn the_exp_bar_region_moves_with_the_game_window() {
    let mut minerva = set_up_with_exp();
    let before = minerva.panel_state().exp_bar_region.unwrap();

    let state = minerva.on_game_window(window(ScreenRect { x: 100, y: 50, ..game(KERNING) }));

    assert_eq!(state.exp_bar_region, Some(ScreenRect { x: 100 + 1012, y: 50 + 1145, ..before }));
}

#[test]
fn a_new_game_resolution_stops_reading_the_exp_bar_and_asks_for_setup_again() {
    let mut minerva = set_up_with_exp();

    let resized = ScreenRect { width: 1280, height: 720, ..game(KERNING) };
    let state = minerva.on_game_window(window(resized));

    assert_eq!(state.setup.needed, Some(SetupReason::ResolutionChanged));
    assert_eq!(state.exp_bar_region, None);
    assert_eq!(state.exp_bar, None);
}

/// An EXP Bar Region marked at another resolution can't be kept: setup redone there
/// without marking the EXP bar again finishes with tracking off.
#[test]
fn setup_redone_at_a_new_resolution_without_the_exp_bar_turns_tracking_off() {
    let mut minerva = set_up_with_exp();
    let narrower = ScreenRect { width: game(KERNING).width - 200, ..game(KERNING) };
    minerva.on_game_window(window(narrower));
    let region = minerva.act(Action::MarkTitleRegion(TITLE_MARK)).title_region.unwrap();
    let client = common::client_of(&common::load(KERNING));
    minerva.on_frame(common::capture(&client, narrower, region, 1000));
    assert!(!minerva.panel_state().setup.exp_bar_kept);

    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.exp_bar_region, None);
}

/// Tracking can be turned on later: setup again, keep the title, mark the EXP bar.
#[test]
fn a_requested_setup_can_keep_the_title_and_add_the_exp_bar() {
    let mut minerva = set_up_without_exp();
    let title = minerva.panel_state().title_region;
    minerva.act(Action::RequestSetup);

    let read_now = mark_and_test_read_exp(&mut minerva, KERNING);
    let state = minerva.act(Action::FinishSetup);

    assert_eq!(read_now, read(160, 381));
    assert_eq!(state.setup.needed, None);
    assert_eq!(state.title_region, title);
    assert!(state.setup.exp_bar_kept);
    assert!(state.exp_bar_region.is_some());
}

/// ...and a requested setup that changes nothing keeps the EXP bar too.
#[test]
fn a_requested_setup_finished_without_marks_keeps_both_regions() {
    let mut minerva = set_up_with_exp();
    let before = minerva.panel_state().clone();
    minerva.act(Action::RequestSetup);

    let state = minerva.act(Action::FinishSetup);

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.title_region, before.title_region);
    assert_eq!(state.exp_bar_region, before.exp_bar_region);
}

#[test]
fn turning_tracking_off_in_a_requested_setup_forgets_the_exp_bar() {
    let mut minerva = set_up_with_exp();
    minerva.act(Action::RequestSetup);

    minerva.act(Action::ClearExpBarRegion);
    let state = minerva.act(Action::FinishSetup);

    assert!(!state.setup.exp_bar_kept);
    assert_eq!(state.exp_bar_region, None);
    assert_eq!(restart(&minerva).settings().exp_bar_region, None);
}

#[test]
fn cancelling_setup_drops_an_exp_bar_mark_and_keeps_the_saved_one() {
    let mut minerva = set_up_with_exp();
    let saved = minerva.panel_state().exp_bar_region;
    minerva.act(Action::RequestSetup);
    minerva.act(Action::MarkExpBarRegion(Rect { x: 70, ..exp_mark(KERNING) }));

    let state = minerva.act(Action::CancelSetup);

    assert_eq!(state.setup.exp_bar_test_read, None);
    assert_eq!(state.exp_bar_region, saved);
}

/// The marking window opens on the EXP bar with a keyboard-adjustable selection (where
/// the figures usually are, until one has been marked), and Enter test-reads it.
#[test]
fn the_exp_bar_can_be_marked_with_the_keyboard_from_where_the_figures_usually_are() {
    let mut minerva = Minerva::new(FakeSource::osms());
    mark_title(&mut minerva, KERNING);

    let state = minerva.act(Action::BeginSelection(MarkTarget::ExpBar)).clone();
    assert_eq!(state.setup.selection_target, Some(MarkTarget::ExpBar));
    // Kerning City's client is 1916x1197: (1916 * 1005 / 1920, 1197 - 56).
    assert_eq!(state.setup.selection, Some(Rect { x: 1002, y: 1141, width: 240, height: 32 }));
    assert_eq!(state.exp_bar_region, None, "nothing is read until it is confirmed");

    let state = minerva.act(Action::ConfirmSelection).clone();
    assert_eq!(state.setup.selection, None);
    assert_eq!(state.setup.exp_bar_test_read, Some(ExpBarTestRead::Reading));
    let frame = exp_frame(&minerva, KERNING, 0);
    assert_eq!(minerva.on_exp_bar_frame(frame).setup.exp_bar_test_read, read(160, 381));

    // Opened again, it starts from the box just marked.
    let state = minerva.act(Action::BeginSelection(MarkTarget::ExpBar));
    assert_eq!(state.setup.selection, Some(Rect { x: 1002, y: 1141, width: 240, height: 32 }));
}

/// Version 2 files (before EXP tracking) load with tracking off; their Win+Alt hotkeys
/// give way to the Ctrl+Shift defaults (version 4).
#[test]
fn version_2_settings_load_with_tracking_off() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(
        serde_json::from_str(
            r#"{
                "version": 2,
                "hotkeys": { "togglePanel": "super+alt+j" },
                "titleRegion": {
                    "marked": { "x": 70, "y": 40, "width": 130, "height": 48 },
                    "client": { "width": 1916, "height": 1197 }
                }
            }"#,
        )
        .unwrap(),
    );

    let state = minerva.on_game_window(window(game(KERNING)));

    assert_eq!(state.setup.needed, None);
    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+c");
    assert_eq!(state.exp_bar_region, None);
    assert_eq!(minerva.settings().version, 7, "saved as the current version");
}

#[test]
fn marking_the_exp_bar_anew_at_setup_opens_a_closed_grind_tracker_window() {
    let mut minerva = set_up_with_exp();
    minerva.press(PanelControl::CloseGrindTracker);

    // Setup again, keeping the EXP bar as it was: the window stays closed.
    minerva.press(PanelControl::Setup);
    minerva.act(Action::FinishSetup);
    assert!(!minerva.panel_state().grind_tracker.open);

    // Setup again, marking the EXP bar anew: the player wants to see it.
    minerva.press(PanelControl::Setup);
    mark_and_test_read_exp(&mut minerva, KERNING);
    minerva.act(Action::FinishSetup);
    assert!(minerva.panel_state().grind_tracker.open);
}
