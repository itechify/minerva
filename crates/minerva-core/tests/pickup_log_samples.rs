mod common;

use minerva_core::{PickupLine, PickupLogCapture, PickupLogReader, PickupLogSeen, ScreenPoint, ScreenRect};

fn clip(name: &str, size: f32) -> Vec<PickupLine> {
    let path = common::samples_in("pickup-log").join(name);
    let frames = common::pngs_in(&path);
    let mut reader = PickupLogReader::bundled();
    let items = common::items::item_world(vec![
        common::items::etc(4_000_000, "Ligator Skin"),
        common::items::use_item(2_060_000, "Arrows for Crossbow"),
        common::items::use_item(2_061_000, "Arrows for Bows"),
    ]);
    let mut minerva = common::tracking_with(common::FakeSource::osms().with_items(items));
    minerva.tick(0);
    for at_ms in [0, 100, 200] {
        minerva.on_status_bar_reading(common::status_bar_reading(Some("Egbert"), at_ms));
    }
    common::start_session(&mut minerva, 300, 1300);
    let expected_mesos = mesos(&truth(name));
    let mut arrived = Vec::new();
    for path in frames {
        let at_ms = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
        let img = image::open(path).unwrap().to_rgba8();
        let img = if size == 1.0 { img } else { common::scaled(&img, size) };
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: size };
        let reading = reader.read(&capture);
        assert!(reading.shift <= 6);
        assert!(reading.rows.iter().all(|&slot| slot < 6));
        let state = minerva.on_pickup_log_reading(reading.clone());
        assert!(state.exp_tracking.loot.mesos <= expected_mesos, "{name} ×{size}: mesos exceeded truth");
        arrived.extend(reading.arrived);
    }
    if size == 1.0 {
        let counted = match name { "empty-to-loot" => 26, "covered" => 0, _ => expected_mesos };
        assert_eq!(minerva.panel_state().exp_tracking.loot.mesos, counted, "{name}");
        let expected_items = match name {
            "cut-short" => vec![
                minerva_core::LootItem { name: "Arrows for Crossbow".into(), count: 21 },
                minerva_core::LootItem { name: "Ligator Skin".into(), count: 1 },
            ],
            "colour-breaks-tie" => vec![minerva_core::LootItem { name: "Ligator Skin".into(), count: 1 }],
            _ => vec![],
        };
        let loot = &minerva.panel_state().exp_tracking.loot;
        assert_eq!(loot.items, expected_items, "{name}");
        assert_eq!(loot.items_total, match name { "cut-short" => 22, "colour-breaks-tie" => 1, _ => 0 }, "{name}");
    }
    arrived
}

fn mesos(lines: &[PickupLine]) -> u64 {
    lines
        .iter()
        .filter_map(|line| match line {
            PickupLine::Mesos { amount } => Some(amount),
            _ => None,
        })
        .sum()
}

#[test]
fn a_mob_covering_the_first_capture_after_reset_cannot_recount_loot() {
    let mut minerva = common::tracking();
    common::start_session(&mut minerva, 0, 1000);
    let mut reader = PickupLogReader::bundled();
    let capture = |sample: u64, at_ms| {
        let path = common::samples_in("pickup-log/empty-to-loot").join(format!("{sample:08}.png"));
        let img = image::open(path).unwrap().to_rgba8();
        PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: 1.0 }
    };
    minerva.on_pickup_log_reading(reader.read(&capture(3000, 3000)));
    minerva.on_pickup_log_reading(reader.read(&capture(3300, 3300)));
    assert_eq!(minerva.panel_state().exp_tracking.loot.mesos, 26);

    reader.reset();
    // An opaque Mob hides every line on the first capture of the new stream.
    let mob = image::RgbaImage::from_pixel(325, 139, image::Rgba([80, 60, 40, 255]));
    minerva.on_pickup_log_reading(reader.read(&PickupLogCapture {
        frame: common::frame(&mob, 3400), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: 1.0,
    }));
    minerva.on_pickup_log_reading(reader.read(&capture(3300, 3500)));
    assert_eq!(minerva.panel_state().exp_tracking.loot.mesos, 26);
}

fn truth(name: &str) -> Vec<PickupLine> {
    let path = common::samples_in("pickup-log").join(name).join("truth.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let dir = common::samples_in("pickup-log").join(name);
    assert!(dir.join(value["baseline"].as_str().unwrap()).is_file());
    assert!(value["arrivals"].as_array().unwrap().iter().all(|v| dir.join(v["frame"].as_str().unwrap()).is_file()));
    value["arrivals"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|v| serde_json::from_value::<Vec<PickupLine>>(v["lines"].clone()).unwrap())
        .collect()
}

#[test]
fn a_first_sighting_of_six_repeated_lines_establishes_the_baseline() {
    assert!(clip("repeated-pairs", 1.0).is_empty());
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
        0
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
fn every_clip_counts_native_truth_and_at_other_ui_sizes_the_same_or_fewer_never_more() {
    for dir in std::fs::read_dir(common::samples_in("pickup-log")).unwrap() {
        let name = dir.unwrap().file_name().to_str().unwrap().to_owned();
        let expected = truth(&name);
        for size in std::iter::once(1.0).chain(common::OTHER_UI_SIZES) {
            assert_conservative(&clip(&name, size), &expected, &format!("{name} ×{size}"));
        }
    }
}

#[test]
fn our_window_over_the_log_is_skipped_and_the_next_clean_stack_is_matched() {
    let mut reader = PickupLogReader::bundled();
    let mut arrivals = Vec::new();
    for at_ms in [2000, 2800, 2900, 3000, 3100, 3200] {
        let path = common::samples_in("pickup-log/empty-to-loot").join(format!("{at_ms:08}.png"));
        let img = image::open(path).unwrap().to_rgba8();
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: 1.0 };
        let ours = if at_ms == 2900 { vec![ScreenRect { x: 1590, y: 830, width: 325, height: 139 }] } else { vec![] };
        let reading = reader.read_avoiding(&capture, &ours);
        if at_ms == 2900 {
            // Nothing has located the log yet: a window over the search band alone
            // cannot establish where the log is, or claim its rows were found.
            assert_eq!(reading.seen, PickupLogSeen::Nothing);
            assert!(reading.arrived.is_empty());
        }
        arrivals.extend(reading.arrived);
    }
    assert_eq!(arrivals, vec![PickupLine::Mesos { amount: 26 }]);

    // With geometry established, the same window is a known cover. A reset also
    // establishes a baseline, so the four lines already present are not recounted.
    reader.reset();
    let mut arrivals = Vec::new();
    for at_ms in [3000, 3200, 3300] {
        let path = common::samples_in("pickup-log/empty-to-loot").join(format!("{at_ms:08}.png"));
        let img = image::open(path).unwrap().to_rgba8();
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: 1.0 };
        let ours = if at_ms == 3200 { vec![ScreenRect { x: 1590, y: 830, width: 325, height: 139 }] } else { vec![] };
        let reading = reader.read_avoiding(&capture, &ours);
        if at_ms == 3000 {
            assert!(reading.arrived.is_empty());
        }
        if at_ms == 3200 {
            assert!(matches!(reading.seen, PickupLogSeen::Covered { .. }));
        }
        arrivals.extend(reading.arrived);
    }
    assert_eq!(arrivals, vec![PickupLine::Mesos { amount: 26 }]);
}

#[test]
fn first_sighting_is_a_baseline_even_after_an_apparently_empty_log() {
    let path = common::samples_in("pickup-log/empty-to-loot");
    let frames = common::pngs_in(&path);
    let mut reader = PickupLogReader::bundled();
    let mut arrived = Vec::new();
    for path in frames {
        let at_ms = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
        let img = image::open(path).unwrap().to_rgba8();
        let capture = PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: 1.0 };
        arrived.extend(reader.read(&capture).arrived);
    }
    // The first four lines cannot be distinguished from a previously covered log.
    assert_eq!(arrived, vec![PickupLine::Mesos { amount: 26 }]);
}

#[test]
fn log_band_captures_count_mesos_on_the_panel_and_replay_without_pixels() {
    let mut minerva = common::tracking();
    for at_ms in [0, 100, 200] {
        minerva.on_status_bar_reading(common::status_bar_reading(Some("Egbert"), at_ms));
    }
    common::start_session(&mut minerva, 300, 1300);
    let band = minerva.panel_state().pickup_log_capture.unwrap();
    let files = common::pngs_in(common::samples_in("pickup-log/empty-to-loot"));
    for path in files {
        let at_ms = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
        let img = image::open(path).unwrap().to_rgba8();
        let mut pixels = image::RgbaImage::from_pixel(band.width, band.height, image::Rgba([0, 0, 0, 255]));
        image::imageops::replace(&mut pixels, &img, 1590 - band.x as i64, 830 - band.y as i64);
        let state = minerva
            .on_pickup_log_capture(PickupLogCapture { frame: common::frame(&pixels, at_ms), origin: ScreenPoint { x: band.x, y: band.y }, ui_size: 1.0 });
        assert!(state.exp_tracking.loot.mesos <= 26);
    }
    let recording = minerva_core::bug_report::read(&minerva.bug_report(&minerva_core::ReportAbout::default()).zip).unwrap();
    assert_eq!(minerva.panel_state().exp_tracking.loot.mesos, 26);
    assert_eq!(minerva_core::bug_report::replay(&recording, Some(common::osms::game().files())).divergence, None);
}

/// A whole 400×480 band at the bottom-right of a 1920×1200 client area (Bug Report 0.15.0,
/// 2026-10-26 07:02), over rock, brick and cloud backgrounds, with the game's quickslots and Cash
/// Shop button in its bottom-left. Its lines end at band x≈390 and the newest is at band y≈240.
fn band_capture(path: &std::path::Path, at_ms: u64) -> PickupLogCapture {
    let img = image::open(path).unwrap().to_rgba8();
    assert_eq!(img.dimensions(), (400, 480));
    PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 2198, y: 846 }, ui_size: 1.0 }
}

fn band_frames(kind: &str) -> Vec<std::path::PathBuf> {
    common::pngs_in(common::samples_in("pickup-log-band"))
        .into_iter()
        .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with(kind))
        .collect()
}

#[test]
fn a_log_in_a_whole_band_is_never_found_anywhere_but_where_its_lines_are() {
    let frames = band_frames("log--");
    assert!(frames.len() >= 10);
    for path in frames {
        let reading = PickupLogReader::bundled().read(&band_capture(&path, 0));
        // Faded lines on a bright background may not be found at all, but never elsewhere.
        let (PickupLogSeen::Clear { area } | PickupLogSeen::Covered { area }) = reading.seen else { continue };
        let (right, bottom) = (area.x + area.width as i32 - 2198, area.y + area.height as i32 - 846);
        assert!((380..=396).contains(&right), "{path:?}: the log's right edge is at band x {right}");
        assert!((238..=252).contains(&bottom), "{path:?}: the log's bottom is at band y {bottom}");
    }
}

#[test]
fn a_band_with_no_log_in_it_finds_none() {
    let frames = band_frames("no-log--");
    assert!(frames.len() >= 4);
    for path in frames {
        let reading = PickupLogReader::bundled().read(&band_capture(&path, 0));
        assert_eq!(reading.seen, PickupLogSeen::Nothing, "{path:?}");
    }
}

/// A `empty-to-loot` frame (325×139, client (1590,830) in a 1920×1200 client area) placed
/// where it was in a whole band, `up` px higher, over a band with no log in it.
fn log_in_band(frame_ms: u64, background: &str, up: i64, at_ms: u64) -> PickupLogCapture {
    let mut band = image::open(common::samples_in("pickup-log-band").join(background)).unwrap().to_rgba8();
    let path = common::samples_in("pickup-log/empty-to-loot").join(format!("{frame_ms:08}.png"));
    image::imageops::replace(&mut band, &image::open(path).unwrap().to_rgba8(), 70, 110 - up);
    PickupLogCapture { frame: common::frame(&band, at_ms), origin: ScreenPoint { x: 2198, y: 846 }, ui_size: 1.0 }
}

fn arrivals(reader: &mut PickupLogReader, captures: impl IntoIterator<Item = PickupLogCapture>) -> Vec<PickupLine> {
    captures.into_iter().flat_map(|capture| reader.read(&capture).arrived).collect()
}

const CLIP_MS: [u64; 14] = [2000, 2100, 2200, 2300, 2400, 2500, 2600, 2700, 2800, 2900, 3000, 3100, 3200, 3300];

#[test]
fn a_log_over_a_band_of_rock_is_found_and_counted() {
    for background in ["no-log--222954.png", "no-log--224804.png", "no-log--244695.png"] {
        let mut reader = PickupLogReader::bundled();
        let captures = CLIP_MS.map(|ms| log_in_band(ms, background, 0, ms));
        assert_eq!(arrivals(&mut reader, captures), vec![PickupLine::Mesos { amount: 26 }], "{background}");
    }
}

#[test]
fn a_band_that_looked_like_a_log_does_not_keep_the_real_one_from_being_found() {
    // Bug Report 0.15.0, 2026-10-26: a frame with fading lines in it was read as a log at the
    // rock's own right edge, and the real log, with fresh lines, was never looked for after.
    let mut reader = PickupLogReader::bundled();
    assert!(reader.read(&band_capture(&common::samples_in("pickup-log-band").join("log--229262.png"), 0)).arrived.is_empty());
    let captures = CLIP_MS.map(|ms| log_in_band(ms, "no-log--244695.png", 0, 100 + ms));
    assert_eq!(arrivals(&mut reader, captures), vec![PickupLine::Mesos { amount: 26 }]);
}

/// `capture` with its lines' text wiped but for their last glyphs: closing parentheses in a
/// stack at the log's pitch, a lookalike with no line to read.
fn wiped(mut capture: PickupLogCapture) -> PickupLogCapture {
    let mut img = image::RgbaImage::from_raw(capture.frame.width, capture.frame.height, capture.frame.rgba).unwrap();
    for y in 40..260 {
        for x in 0..340 {
            img.put_pixel(x, y, image::Rgba([150, 110, 60, 255]));
        }
    }
    capture.frame = common::frame(&img, capture.frame.at_ms);
    capture
}

#[test]
fn a_stack_of_closing_parentheses_counts_nothing_until_a_line_in_it_reads() {
    let mut reader = PickupLogReader::bundled();
    for ms in CLIP_MS {
        let reading = reader.read(&wiped(log_in_band(ms, "no-log--244695.png", 0, ms)));
        assert_eq!(reading.seen, PickupLogSeen::Nothing, "{ms}");
        assert!(reading.arrived.is_empty(), "{ms}");
    }
}

#[test]
fn a_lookalike_that_never_reads_is_dropped_and_the_real_log_is_found() {
    let mut reader = PickupLogReader::bundled();
    // A lookalike two lines higher than the real log, for four seconds.
    for i in 0..40 {
        let reading = reader.read(&wiped(log_in_band(CLIP_MS[i % CLIP_MS.len()], "no-log--244695.png", 2 * 22, i as u64 * 100)));
        assert_eq!(reading.seen, PickupLogSeen::Nothing);
    }
    // The real log, still for four seconds, then the clip: its lines, all of them, are counted,
    // the first four too, because the real log was followed from before they arrived.
    let still = (0..40).map(|i| log_in_band(2000, "no-log--244695.png", 0, 4000 + i * 100));
    let clip = CLIP_MS.map(|ms| log_in_band(ms, "no-log--244695.png", 0, 8000 + ms));
    assert_eq!(arrivals(&mut reader, still.chain(clip)), truth("empty-to-loot"));
}

/// Bug Report 0.15.0, 2026-10-26 09:03 (Ellinia: bright greens, a white sky). Whole bands, in
/// order, whose lines are plainly legible, white over green: the log fading (182792, 183333);
/// mesos (+13) and `Squishy Liquid x1 earned. (Etc)` arriving (183631); the same log 0.9 s
/// later; another `Squishy Liquid x1` (187992; an EXP (+2) line arrived between, in a crop the
/// report left out); and `Slime Bubble x1 earned. (Etc)` (188167).
const LINES: [u64; 6] = [182792, 183333, 183631, 184483, 187992, 188167];

fn lines_clip() -> Vec<PickupLogCapture> {
    LINES.iter().map(|ms| band_capture(&common::samples_in("pickup-log-band").join(format!("lines--{ms}.png")), *ms)).collect()
}

#[test]
fn a_log_is_drawn_at_the_ui_size_not_at_the_height_of_one_parenthesis() {
    // The Status Bar says the UI Size is 1: the located log is 313x130 at it, however many
    // pixels tall the bright background leaves its closing parentheses.
    let mut reader = PickupLogReader::bundled();
    let readings = lines_clip().iter().map(|capture| reader.read(capture)).collect::<Vec<_>>();
    let Some(PickupLogSeen::Clear { area }) = readings.get(2).map(|reading| reading.seen) else { panic!("{readings:?}") };
    assert_eq!((area.width, area.height), (313, 130));
}

#[test]
fn legible_white_lines_over_bright_green_read_as_their_kinds() {
    let mut reader = PickupLogReader::bundled();
    let arrived = arrivals(&mut reader, lines_clip());
    let is_item = |line: &PickupLine| matches!(line, PickupLine::Item { quantity: 1, tab, .. } if tab == "Etc");
    // Every line that arrived reads as its kind, none as an Unreadable or twice (an EXP line
    // that arrived unseen between two crops is missed, never made up).
    assert!(matches!(arrived.as_slice(), [PickupLine::Mesos { amount: 13 }, a, b, c] if is_item(a) && is_item(b) && is_item(c)), "{arrived:?}");
}

#[test]
fn the_items_those_lines_name_are_tallied_by_name_and_counted_once() {
    let items = common::items::item_world(vec![
        common::items::etc(4_000_100, "Squishy Liquid"),
        common::items::etc(4_000_101, "Slime Bubble"),
        common::items::etc(4_000_102, "Orange Mushroom Cap"),
    ]);
    let mut minerva = common::tracking_with(common::FakeSource::osms().with_items(items));
    minerva.tick(0);
    for at_ms in [0, 100, 200] {
        minerva.on_status_bar_reading(common::status_bar_reading(Some("Egbert"), at_ms));
    }
    common::start_session(&mut minerva, 300, 1300);
    let mut reader = PickupLogReader::bundled();
    for capture in lines_clip() {
        minerva.on_pickup_log_reading(reader.read(&capture));
    }
    let loot = &minerva.panel_state().exp_tracking.loot;
    assert_eq!(loot.mesos, 13);
    assert_eq!(
        loot.items,
        vec![
            minerva_core::LootItem { name: "Squishy Liquid".into(), count: 2 },
            minerva_core::LootItem { name: "Slime Bubble".into(), count: 1 },
        ]
    );
}

#[test]
fn a_shadow_that_brightens_around_an_unchanged_line_is_not_a_new_arrival() {
    let original = image::open(common::samples_in("pickup-log/repeated-mesos").join("00144700.png")).unwrap().to_rgba8();
    let mut lifted = original.clone();
    for p in lifted.pixels_mut().filter(|p| p.0[..3].iter().all(|&c| c < 100)) {
        for c in &mut p.0[..3] {
            *c += 20;
        }
    }
    let capture = |img: &image::RgbaImage, at_ms| PickupLogCapture { frame: common::frame(img, at_ms), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: 1.0 };
    let mut reader = PickupLogReader::bundled();
    let captures = [capture(&original, 0), capture(&lifted, 100), capture(&original, 200)];
    assert_eq!(arrivals(&mut reader, captures), vec![]);
}

/// The readings of a clip, read as the band is, at the samples' size.
fn readings(name: &str) -> Vec<minerva_core::PickupLogReading> {
    let mut reader = PickupLogReader::bundled();
    common::pngs_in(&common::samples_in("pickup-log").join(name))
        .into_iter()
        .map(|path| {
            let at_ms = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
            let img = image::open(path).unwrap().to_rgba8();
            reader.read(&PickupLogCapture { frame: common::frame(&img, at_ms), origin: ScreenPoint { x: 1590, y: 830 }, ui_size: 1.0 })
        })
        .collect()
}

#[test]
fn a_reading_guessed_only_where_colour_and_brightness_left_more_than_one_shift() {
    for name in ["colour-breaks-tie", "repeated-mesos", "repeated-pairs", "solo-repeated", "cut-short", "empty-to-loot", "covered"] {
        let readings = readings(name);
        assert!(readings.iter().all(|r| !r.guessed || r.chose), "{name}: a guess is a choice between shapes' shifts");
    }
    // Colour settles 1 of the 10 times that shapes alone fitted several shifts here: not a guess.
    let colour = readings("colour-breaks-tie");
    assert_eq!(colour.iter().filter(|r| r.chose).count(), 10);
    assert_eq!(colour.iter().filter(|r| r.guessed).count(), 9);
    assert!(readings("repeated-mesos").iter().any(|r| r.guessed), "identical lines leave the smallest shift to be taken");
}
