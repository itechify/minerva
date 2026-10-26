mod common;

use minerva_core::{PickupLine, PickupLogCapture, PickupLogReader, PickupLogSeen, ScreenPoint, ScreenRect};

fn clip(name: &str, size: f32) -> Vec<PickupLine> {
    let path = common::samples_in("pickup-log").join(name);
    let mut frames: Vec<_> =
        std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "png")).collect();
    frames.sort();
    let mut reader = PickupLogReader::bundled();
    let mut arrived = Vec::new();
    for path in frames {
        let at_ms = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
        let img = image::open(path).unwrap().to_rgba8();
        let img = if size == 1.0 { img } else { common::scaled(&img, size) };
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 } };
        let reading = reader.read(&capture);
        assert!(reading.shift <= 6);
        assert!(reading.rows.iter().all(|&slot| slot < 6));
        arrived.extend(reading.arrived);
    }
    arrived
}

fn truth(name: &str) -> Vec<PickupLine> {
    let path = common::samples_in("pickup-log").join(name).join("truth.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    value["arrivals"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|v| serde_json::from_value::<Vec<PickupLine>>(v["lines"].clone()).unwrap())
        .collect()
}

#[test]
fn repeated_pairs_are_six_arrivals_and_fading_is_not_an_arrival() {
    assert_eq!(clip("repeated-pairs", 1.0), truth("repeated-pairs"));
}

#[test]
fn colour_and_digits_break_the_four_or_six_line_shift_tie() {
    assert_eq!(clip("colour-breaks-tie", 1.0), truth("colour-breaks-tie"));
}

#[test]
fn cut_short_items_keep_their_quantity_tab_and_visible_name() {
    assert_eq!(clip("cut-short", 1.0), truth("cut-short"));
}

#[test]
fn identical_mesos_and_solo_exp_are_not_deduplicated() {
    for name in ["repeated-mesos", "solo-repeated"] {
        assert_eq!(clip(name, 1.0), truth(name), "{name}");
    }
}

#[test]
fn covering_the_stack_does_not_fake_a_burst() {
    let read = clip("covered", 1.0);
    let expected = truth("covered");
    assert_conservative(&read, &expected, "covered");
    assert_eq!(
        read.iter()
            .filter_map(|l| match l {
                PickupLine::Mesos { amount } => Some(amount),
                _ => None,
            })
            .sum::<u64>(),
        37
    );
}

fn assert_conservative(read: &[PickupLine], expected: &[PickupLine], context: &str) {
    let mut remaining = expected;
    for line in read {
        let at = remaining
            .iter()
            .position(|want| line == want || *line == PickupLine::Unreadable)
            .unwrap_or_else(|| panic!("{context}: extra or wrong {line:?}; read {read:?}; truth {expected:?}"));
        remaining = &remaining[at + 1..];
    }
}

#[test]
fn every_clip_at_other_ui_sizes_counts_the_same_or_fewer_never_more() {
    for dir in std::fs::read_dir(common::samples_in("pickup-log")).unwrap() {
        let name = dir.unwrap().file_name().to_str().unwrap().to_owned();
        let expected = truth(&name);
        for size in common::OTHER_UI_SIZES {
            assert_conservative(&clip(&name, size), &expected, &format!("{name} ×{size}"));
        }
    }
}

#[test]
fn our_window_over_the_log_is_skipped_and_the_next_clean_stack_is_matched() {
    let mut reader = PickupLogReader::bundled();
    let mut arrivals = Vec::new();
    for at_ms in [2000, 2800, 2900, 3000, 3100, 3200] {
        let path = common::samples_in("pickup-log/empty-to-pickups").join(format!("{at_ms:08}.png"));
        let img = image::open(path).unwrap().to_rgba8();
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 } };
        let ours = if at_ms == 2900 { vec![ScreenRect { x: 1590, y: 830, width: 325, height: 139 }] } else { vec![] };
        let reading = reader.read_avoiding(&capture, &ours);
        if at_ms == 2900 {
            assert!(matches!(reading.seen, PickupLogSeen::Covered { .. }));
        }
        arrivals.extend(reading.arrived);
    }
    assert_eq!(arrivals, truth("empty-to-pickups"));
}

#[test]
fn first_pickups_in_an_empty_log_are_read_once() {
    let path = common::samples_in("pickup-log/empty-to-pickups");
    let mut frames: Vec<_> =
        std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "png")).collect();
    frames.sort();
    let mut reader = PickupLogReader::bundled();
    let mut arrived = Vec::new();
    for path in frames {
        let at_ms = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
        let img = image::open(path).unwrap().to_rgba8();
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 } };
        arrived.extend(reader.read(&capture).arrived);
    }
    assert_eq!(
        arrived,
        vec![
            PickupLine::Item { name: "Ligator Skin".into(), quantity: 1, tab: "Etc".into(), cut_short: false },
            PickupLine::Exp { amount: 21, party: true },
            PickupLine::PartyBonus { amount: 2 },
            PickupLine::Mesos { amount: 37 },
            PickupLine::Mesos { amount: 26 },
        ]
    );
}
