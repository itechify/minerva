//! The EXP figures found in the Status Bar (ADR 0017): the Grind Tracker needs no mark. The
//! figures ("EXP. 160 [3.81%]") are read from the Status Bar's own capture, where it is found,
//! or where they were last found while it can't be, until the client area changes size. The
//! settings view's switch turns the Grind Tracker off.
//!
//! Worked example used throughout: the Kerning City window sample's client area (1916x1197)
//! shown at (812,110). Its Status Bar's LV box ends at client x 456, and the EXP figures are
//! read from client (1005,1144) 224x32: from just left of the "EXP." label (x 1019) to just
//! short of the Cash Shop button.

mod common;

use common::FakeSource;
use minerva_core::{
    Action, ExpBar, GameWindow, Minerva, PanelState, Rect, ScreenPoint, ScreenRect, Settings, Size, StatusBarCapture,
    TitleRegion, TrackingState,
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

/// Settings with the Title Region kept on a client area of the sample's size.
fn set_up(sample: &str) -> Settings {
    let game = game(sample);
    let client = Size { width: game.width, height: game.height };
    Settings { title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }), ..Settings::default() }
}

/// A core restored from `settings`, over the sample's game window.
fn started(settings: Settings, sample: &str) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(settings);
    minerva.on_game_window(window(game(sample)));
    minerva
}

/// The capture the core asks for of a sample for the Status Bar, at `at_ms`, after `paint`
/// has had its way with the strip (its coordinates the strip's).
fn capture(minerva: &Minerva, sample: &str, at_ms: u64, paint: impl Fn(&mut image::RgbaImage)) -> StatusBarCapture {
    let game = game(sample);
    let strip = minerva.panel_state().status_bar_capture.expect("the Status Bar is captured with the game in front");
    let client = common::client_of(&common::load(sample));
    let (x, y) = ((strip.x - game.x) as u32, (strip.y - game.y) as u32);
    let mut crop = image::imageops::crop_imm(&client, x, y, strip.width, strip.height).to_image();
    paint(&mut crop);
    StatusBarCapture { frame: common::frame(&crop, at_ms), origin: ScreenPoint { x: strip.x, y: strip.y } }
}

/// The sample's Status Bar captured whole at `at_ms`.
fn read(minerva: &mut Minerva, sample: &str, at_ms: u64) -> PanelState {
    let capture = capture(minerva, sample, at_ms, |_| {});
    minerva.on_status_bar_capture(capture).clone()
}

/// Something black over columns `x0..x1` of the strip, and the rows `up.0` to `up.1` rows up
/// from its bottom (the client area's).
fn cover(img: &mut image::RgbaImage, (x0, x1): (u32, u32), up: (u32, u32)) {
    for y in img.height() - up.0..img.height() - up.1 {
        for x in x0..x1 {
            img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
}

/// The LV box and its tiles, covered: the Status Bar can't be found. (The client area is 1197
/// rows; the tiles are at client (403,1163).)
fn lv_box_covered(img: &mut image::RgbaImage) {
    cover(img, (380, 470), (46, 6));
}

/// The EXP figures covered, the rest of the bar not.
fn figures_covered(img: &mut image::RgbaImage) {
    cover(img, (1005, 1229), (53, 21));
}

/// The sample's Status Bar captured every second from `from` to `until`, painted.
fn read_painted(minerva: &mut Minerva, sample: &str, (from, until): (u64, u64), paint: fn(&mut image::RgbaImage)) -> PanelState {
    let mut state = minerva.panel_state().clone();
    for at_ms in (from..=until).step_by(1_000) {
        let capture = capture(minerva, sample, at_ms, paint);
        state = minerva.on_status_bar_capture(capture).clone();
    }
    state
}

#[test]
fn setup_marks_only_the_title_and_the_grind_tracker_is_on_after_it() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_game_window(window(game(KERNING)));
    let region = minerva.act(Action::MarkTitleRegion(Rect { x: 70, y: 40, width: 130, height: 48 })).title_region.unwrap();
    let client = common::client_of(&common::load(KERNING));
    minerva.on_frame(common::capture(&client, game(KERNING), region, 0));

    let state = minerva.act(Action::FinishSetup).clone();

    assert_eq!(state.setup.needed, None);
    assert!(state.grind_tracker.on);
    assert!(state.exp_figures.is_some(), "the figures are read in each Status Bar capture");
    assert_eq!(state.exp_tracking.state, TrackingState::Waiting);
}

#[test]
fn the_figures_are_read_where_the_status_bar_has_them() {
    let mut minerva = started(set_up(KERNING), KERNING);

    let state = read(&mut minerva, KERNING, 0);

    assert_eq!(state.exp_bar, Some(ExpBar { exp: 160, percent_hundredths: 381 }));
    let found = state.exp_figures.and_then(|figures| figures.last_found);
    assert_eq!(found, Some(ScreenRect { x: 812 + 1005, y: 110 + 1144, width: 224, height: 32 }));
}

#[test]
fn a_dimmer_capture_still_reads() {
    let mut minerva = started(set_up(KERNING), KERNING);
    let capture = capture(&minerva, KERNING, 0, |img| {
        for p in img.pixels_mut() {
            for c in &mut p.0[..3] {
                *c = (*c as f32 * 0.935).round() as u8;
            }
        }
    });

    assert_eq!(minerva.on_status_bar_capture(capture).exp_bar, Some(ExpBar { exp: 160, percent_hundredths: 381 }));
}

#[test]
fn the_figures_move_with_the_game_window() {
    let mut minerva = started(set_up(KERNING), KERNING);
    read(&mut minerva, KERNING, 0);

    let moved = ScreenRect { x: 100, y: 50, ..game(KERNING) };
    let state = minerva.on_game_window(window(moved));

    let found = state.exp_figures.and_then(|figures| figures.last_found);
    assert_eq!(found, Some(ScreenRect { x: 100 + 1005, y: 50 + 1144, width: 224, height: 32 }));
}

#[test]
fn while_the_lv_box_is_covered_the_figures_are_read_where_they_were_last_found() {
    let mut minerva = started(set_up(KERNING), KERNING);
    read(&mut minerva, KERNING, 0);

    let state = read_painted(&mut minerva, KERNING, (1_000, 30_000), lv_box_covered);

    assert_eq!(state.status_bar, None, "the Status Bar isn't found");
    assert_ne!(state.exp_tracking.state, TrackingState::Unreadable, "the figures still read");
    assert!(state.exp_figures.and_then(|figures| figures.last_found).is_some());
}

#[test]
fn after_the_client_area_changes_size_the_figures_are_only_read_where_the_status_bar_is_found() {
    let mut minerva = started(set_up(KERNING), KERNING);
    read(&mut minerva, KERNING, 0);

    let state = minerva.on_game_window(window(ScreenRect { width: 1280, height: 720, ..game(KERNING) })).clone();

    assert_eq!(state.exp_figures.map(|figures| figures.last_found), Some(None));
}

#[test]
fn a_status_bar_never_found_is_what_the_grind_tracker_says_cant_be_read() {
    let mut minerva = started(set_up(KERNING), KERNING);

    let state = read_painted(&mut minerva, KERNING, (0, 15_000), lv_box_covered);

    assert_eq!(state.exp_tracking.state, TrackingState::Unreadable);
    assert!(state.exp_tracking.no_status_bar);
}

#[test]
fn figures_covered_on_a_status_bar_found_are_unreadable_with_the_status_bar_there() {
    let mut minerva = started(set_up(KERNING), KERNING);
    read(&mut minerva, KERNING, 0);

    let state = read_painted(&mut minerva, KERNING, (1_000, 15_000), figures_covered);

    assert!(state.status_bar.is_some());
    assert_eq!(state.exp_tracking.state, TrackingState::Unreadable);
    assert!(!state.exp_tracking.no_status_bar);
}

#[test]
fn turning_the_grind_tracker_off_stops_reading_the_figures_but_not_the_status_bar() {
    let mut minerva = started(set_up(KERNING), KERNING);

    minerva.act(Action::SetGrindTracker { on: false });
    let state = read(&mut minerva, KERNING, 0);

    assert!(!state.grind_tracker.on);
    assert_eq!(state.grind_tracker.placement, None);
    assert_eq!(state.exp_figures, None);
    assert_eq!(state.exp_bar, None);
    assert_eq!(state.exp_tracking.state, TrackingState::Off);
    assert_eq!(state.status_bar.and_then(|bar| bar.name).as_deref(), Some("Egbert"), "Characters are still read");
}

#[test]
fn the_switch_survives_a_restart() {
    let mut minerva = started(set_up(KERNING), KERNING);
    minerva.act(Action::SetGrindTracker { on: false });

    let json = serde_json::to_string(&minerva.settings()).unwrap();
    let restarted = started(serde_json::from_str(&json).unwrap(), KERNING);

    assert!(!restarted.panel_state().grind_tracker.on);
    assert_eq!(restarted.panel_state().exp_figures, None);
}

/// Files from before ADR 0017 hold the EXP Bar Region the player marked: it is ignored, and
/// the Grind Tracker is on, as it is for a file without one.
#[test]
fn settings_with_an_exp_bar_region_load_with_the_grind_tracker_on_and_the_region_forgotten() {
    for exp_bar_region in [r#""expBarRegion": { "marked": { "x": 1005, "y": 1141, "width": 240, "height": 32 }, "client": { "width": 1916, "height": 1197 } },"#, ""] {
        let json = format!(
            r#"{{
                "version": 6,
                {exp_bar_region}
                "titleRegion": {{
                    "marked": {{ "x": 70, "y": 40, "width": 130, "height": 48 }},
                    "client": {{ "width": 1916, "height": 1197 }}
                }}
            }}"#
        );
        let minerva = started(serde_json::from_str(&json).unwrap(), KERNING);

        let state = minerva.panel_state();
        assert_eq!(state.setup.needed, None, "{exp_bar_region}");
        assert!(state.grind_tracker.on, "{exp_bar_region}");
        let saved = serde_json::to_string(&minerva.settings()).unwrap();
        assert!(!saved.contains("expBarRegion"), "{saved}");
        assert_eq!(minerva.settings().version, 7, "saved as the current version");
    }
}

/// A client area of another size asks for the title to be marked again, but never for the
/// EXP bar: the figures are found wherever the Status Bar is.
#[test]
fn another_client_size_with_the_title_marked_on_it_needs_no_setup() {
    let mut settings = set_up(KERNING);
    let resized = ScreenRect { width: 1914, height: 1197, ..game(KERNING) };
    settings.title_region = Some(TitleRegion {
        marked: Rect { x: 70, y: 40, width: 130, height: 48 },
        client: Size { width: resized.width, height: resized.height },
    });
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(settings);

    let state = minerva.on_game_window(window(resized));

    assert_eq!(state.setup.needed, None);
    assert!(state.exp_figures.is_some());
}
