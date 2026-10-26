mod common;

use minerva_core::bug_report::{self, Event};
use minerva_core::{Action, Foreground, Minerva, ScreenRect, StatusBar, StatusBarReading};
use minerva_core::{PickupLine, PickupLogCapture, PickupLogReading, PickupLogSeen, ReportAbout, ScreenPoint};

fn character(minerva: &mut Minerva, size: f32) {
    minerva.on_status_bar_reading(StatusBarReading {
        at_ms: 100,
        bar: Some(StatusBar {
            area: ScreenRect { x: 0, y: 1140, width: (300.0 * size) as u32, height: (55.0 * size) as u32 },
            level: Some(30),
            job: Some("Magician".into()),
            name: Some("Egbert".into()),
            exp_figures: None,
        }),
        near_miss: None,
    });
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
fn a_reading_and_a_stale_capture_are_recorded_and_replay_without_pixels() {
    let mut minerva = common::tracking();
    character(&mut minerva, 1.0);
    common::read(&mut minerva, 110, 160, 381);
    common::read(&mut minerva, 150, 266, 633);
    let region = minerva.panel_state().pickup_log_capture.unwrap();
    let reading = PickupLogReading {
        at_ms: 200,
        seen: PickupLogSeen::Clear { area: region },
        rows: vec![5],
        shift: 1,
        arrived: vec![PickupLine::Mesos { amount: 1_234 }],
        chose: false,
        overflowed: false,
    };
    minerva.on_pickup_log_reading(reading.clone());
    assert_eq!(minerva.panel_state().exp_tracking.pickups.mesos, 1_234);
    minerva.on_foreground(Foreground::OtherApp);
    let img = image::RgbaImage::new(region.width, region.height);
    let capture = PickupLogCapture { frame: common::frame(&img, 300), origin: ScreenPoint { x: region.x, y: region.y } };
    minerva.on_pickup_log_read(capture, PickupLogReading { at_ms: 300, ..reading });
    assert_eq!(minerva.panel_state().pickup_log_capture, None);
    assert_eq!(minerva.panel_state().exp_tracking.pickups.mesos, 1_234);
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
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: region.x, y: region.y } };
        let seen = if [400, 500].contains(&at_ms) { PickupLogSeen::Nothing } else { PickupLogSeen::Clear { area: region } };
        let arrived = if at_ms == 600 { vec![PickupLine::Unreadable] } else { vec![] };
        minerva.on_pickup_log_read(
            capture,
            PickupLogReading { at_ms, seen, rows: vec![], shift: 0, arrived, chose: at_ms == 300, overflowed: false },
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
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: region.x, y: region.y } };
        minerva.on_pickup_log_read(
            capture,
            PickupLogReading { at_ms, seen, rows: vec![], shift: 0, arrived: vec![], chose: false, overflowed: false },
        );
    }
    assert!(minerva.panel_state().pickup_log_capture.is_some());
    let recording = bug_report::read(&minerva.bug_report(&ReportAbout::default()).zip).unwrap();
    assert!(recording.crops.iter().all(|p| !p.contains("pickup-log")));
}
