mod common;

use minerva_core::bug_report::{self, Event};
use minerva_core::{Action, Foreground, Minerva, ScreenRect, StatusBarReading};
use minerva_core::{PickupLine, PickupLogCapture, PickupLogReading, PickupLogSeen, ReportAbout, ScreenPoint};

fn character(minerva: &mut Minerva, size: f32) {
    let mut reading = common::status_bar_reading(Some("Egbert"), 100);
    let bar = reading.bar.as_mut().unwrap();
    bar.area = ScreenRect { x: 0, y: 1140, width: (300.0 * size) as u32, height: (55.0 * size) as u32 };
    bar.level = Some(30);
    minerva.on_status_bar_reading(reading);
}

#[test]
fn pickup_capture_needs_the_tracker_a_character_and_a_visible_overlay() {
    let mut minerva = common::tracking();
    assert_eq!(minerva.panel_state().pickup_log_capture, None);
    character(&mut minerva, 1.0);
    let band = minerva.panel_state().pickup_log_capture.expect("a band for the log");
    assert!(band.x <= 1600 && band.x + band.width as i32 == 1920);
    assert!(band.y <= 835 && band.y + band.height as i32 >= 970);
    minerva.act(Action::SetGrindTracker { on: false });
    assert_eq!(minerva.panel_state().pickup_log_capture, None);
    minerva.act(Action::SetGrindTracker { on: true });
    minerva.on_foreground(Foreground::OtherApp);
    assert_eq!(minerva.panel_state().pickup_log_capture, None);
}

#[test]
fn the_core_starts_a_new_pickup_stream_only_at_capture_or_character_boundaries() {
    let mut minerva = common::tracking();
    character(&mut minerva, 1.0);
    let first = minerva.panel_state().pickup_log_generation;
    character(&mut minerva, 1.0);
    minerva.tick(200);
    assert_eq!(minerva.panel_state().pickup_log_generation, first);
    minerva.on_status_bar_reading(common::status_bar_reading(Some("Bob"), 300));
    let bob = minerva.panel_state().pickup_log_generation;
    assert!(bob > first);
    minerva.act(Action::SetGrindTracker { on: false });
    minerva.act(Action::SetGrindTracker { on: true });
    let resumed = minerva.panel_state().pickup_log_generation;
    assert!(resumed > bob);
    minerva.on_foreground(Foreground::OtherApp);
    minerva.on_foreground(Foreground::Game);
    assert!(minerva.panel_state().pickup_log_generation > resumed);
}

#[test]
fn a_reading_and_a_stale_capture_are_recorded_and_replay_without_pixels() {
    let mut minerva = common::tracking();
    character(&mut minerva, 1.0);
    common::start_session(&mut minerva, 110, 150);
    let region = minerva.panel_state().pickup_log_capture.unwrap();
    let reading = PickupLogReading { rows: vec![5], shift: 1,
        ..common::pickup_reading(200, PickupLogSeen::Clear { area: region }, vec![PickupLine::Mesos { amount: 1_234 }]) };
    minerva.on_pickup_log_reading(reading.clone());
    assert_eq!(minerva.panel_state().exp_tracking.loot.mesos, 1_234);
    minerva.on_foreground(Foreground::OtherApp);
    let img = image::RgbaImage::new(region.width, region.height);
    let capture = PickupLogCapture { frame: common::frame(&img, 300), origin: ScreenPoint { x: region.x, y: region.y }, ui_size: 1.0 };
    minerva.on_pickup_log_read(capture, PickupLogReading { at_ms: 300, ..reading });
    assert_eq!(minerva.panel_state().pickup_log_capture, None);
    assert_eq!(minerva.panel_state().exp_tracking.loot.mesos, 1_234);
    let recording = bug_report::read(&minerva.bug_report(&ReportAbout::default()).zip).unwrap();
    let readings: Vec<_> = recording
        .lines
        .iter()
        .filter_map(|l| match &l.event {
            Event::PickupLog { reading, stale_region } => Some((reading.at_ms, *stale_region)),
            _ => None,
        })
        .collect();
    assert_eq!(readings, vec![(200, None), (300, Some(region))]);
    assert_eq!(bug_report::replay(&recording, Some(common::osms::game().files())).divergence, None);
}

#[test]
fn the_status_bar_sizes_the_band_and_no_character_stops_capture() {
    let mut minerva = common::tracking();
    character(&mut minerva, 2.0);
    assert_eq!(minerva.panel_state().pickup_log_capture, Some(ScreenRect { x: 1120, y: 240, width: 800, height: 960 }));
    minerva.on_status_bar_reading(StatusBarReading { at_ms: 200, bar: None, near_miss: None });
    assert_eq!(minerva.panel_state().pickup_log_capture, None);
}

#[test]
fn reports_keep_only_ambiguous_missing_and_unreadable_log_crops() {
    let mut minerva = common::tracking();
    character(&mut minerva, 1.0);
    let region = minerva.panel_state().pickup_log_capture.unwrap();
    let img = image::RgbaImage::new(region.width, region.height);
    for at_ms in 200..=700 {
        if at_ms % 100 != 0 {
            continue;
        }
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: region.x, y: region.y }, ui_size: 1.0 };
        let seen = if [400, 500].contains(&at_ms) { PickupLogSeen::Nothing } else { PickupLogSeen::Clear { area: region } };
        let arrived = if at_ms == 600 { vec![PickupLine::Unreadable] } else { vec![] };
        minerva.on_pickup_log_read(
            capture,
            PickupLogReading { chose: at_ms == 300, ..common::pickup_reading(at_ms, seen, arrived) },
        );
    }
    assert!(minerva.panel_state().pickup_log_capture.is_some());
    let recording = bug_report::read(&minerva.bug_report(&ReportAbout::default()).zip).unwrap();
    let crops: Vec<_> = recording.crops.iter().filter(|p| p.contains("pickup-log")).cloned().collect();
    assert_eq!(crops, vec!["crops/pickup-log-0000000300.png", "crops/pickup-log-0000000400.png", "crops/pickup-log-0000000600.png"]);
}

#[test]
fn a_window_before_the_log_is_found_does_not_make_an_empty_capture_interesting() {
    let mut minerva = common::tracking();
    character(&mut minerva, 1.0);
    let region = minerva.panel_state().pickup_log_capture.unwrap();
    let img = image::RgbaImage::new(region.width, region.height);
    for (at_ms, seen) in [(200, PickupLogSeen::Covered { area: region }), (300, PickupLogSeen::Nothing)] {
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: region.x, y: region.y }, ui_size: 1.0 };
        minerva.on_pickup_log_read(
            capture,
            common::pickup_reading(at_ms, seen, vec![]),
        );
    }
    assert!(minerva.panel_state().pickup_log_capture.is_some());
    let recording = bug_report::read(&minerva.bug_report(&ReportAbout::default()).zip).unwrap();
    assert!(recording.crops.iter().all(|p| !p.contains("pickup-log")));
}
