//! The Pickup Log at a 1280x720 client area, where the game draws its UI at about 0.64 of the
//! samples' size (a Status Bar of 35 px against 55): lines 14 px apart, text 10 px tall
//! (ADR 0026; Bug Report 0.15.0, 2026-10-26 14:41, whose reader found the log in none of 6,652
//! captures). The clips in `samples/pickup-log-720p` are fed as the real band is: the band the
//! core asks for at that size, with the clip pasted in where it was on screen.

mod common;

use minerva_core::{
    LootItem, Minerva, PickupLine, PickupLogCapture, PickupLogReader, PickupLogSeen, ScreenPoint, ScreenRect, StatusBar,
    StatusBarReading,
};

/// Where a clip's top-left pixel is in the 1280x720 client area (the clips are cut from the log's
/// corner, the lines ending 7 px inside the right edge).
const CLIP_AT: (i64, i64) = (1072, 480);

/// A 1280x720 game with the Grind Tracker on and a Session started, and the Item files of the
/// made-up game world the clips' Items are named from.
fn at_720p() -> Minerva {
    let items = common::items::item_world(vec![
        common::items::etc(4_000_000, "Ligator Skin"),
        common::items::etc(4_000_001, "Jr. Necki Skin"),
        common::items::use_item(2_000_000, "Orange Potion"),
        common::items::use_item(2_060_000, "Arrows for Crossbow"),
        common::items::use_item(2_061_000, "Arrows for Bows"),
    ]);
    let mut minerva = Minerva::new(common::FakeSource::osms().with_items(items));
    minerva.restore(common::set_up());
    minerva.on_game_window(Some(common::window_at_origin(1280, 720)));
    minerva.tick(0);
    for at_ms in [0, 100, 200] {
        let area = ScreenRect { x: 242, y: 683, width: 211, height: 35 };
        let bar = StatusBar { area, level: Some(13), job: Some("Magician".into()), name: Some("Egbert".into()), exp_figures: None };
        minerva.on_status_bar_reading(StatusBarReading { bar: Some(bar), at_ms, near_miss: None });
    }
    common::start_session(&mut minerva, 300, 1300);
    minerva
}

fn truth(name: &str) -> Vec<(String, Vec<PickupLine>)> {
    let dir = common::samples_in("pickup-log-720p").join(name);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("truth.json")).unwrap()).unwrap();
    assert!(dir.join(value["baseline"].as_str().unwrap()).is_file());
    value["arrivals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let frame = v["frame"].as_str().unwrap().to_owned();
            assert!(dir.join(&frame).is_file(), "{name}: truth names a frame that is not there");
            (frame, serde_json::from_value(v["lines"].clone()).unwrap())
        })
        .collect()
}

/// What a clip read as: the lines it counted, in order, and the Session's Loot.
struct Read {
    lines: Vec<PickupLine>,
    /// The same lines by the frame (its ms) that brought them.
    at: Vec<(u64, PickupLine)>,
    seen_clear: usize,
    mesos: u64,
    items: Vec<LootItem>,
}

/// Read a clip from a cold reader, as the core's own shell would: each frame pasted into the
/// band at the Status Bar's UI Size, and its reading taken in.
fn clip(name: &str) -> Read {
    let mut minerva = at_720p();
    let band = minerva.panel_state().pickup_log_capture.expect("the band is asked for");
    assert_eq!((band.x, band.y, band.width, band.height), (1025, 414, 255, 306));
    let ui_size = minerva.panel_state().pickup_log_ui_size_permille as f32 / 1000.0;
    assert_eq!(ui_size, 0.636);
    let mut reader = PickupLogReader::bundled();
    let mut read = Read { lines: Vec::new(), at: Vec::new(), seen_clear: 0, mesos: 0, items: Vec::new() };
    for path in common::pngs_in(common::samples_in("pickup-log-720p").join(name)) {
        let at_ms = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
        let clip = image::open(path).unwrap().to_rgba8();
        let mut pixels = image::RgbaImage::from_pixel(band.width, band.height, image::Rgba([0, 0, 0, 255]));
        image::imageops::replace(&mut pixels, &clip, CLIP_AT.0 - band.x as i64, CLIP_AT.1 - band.y as i64);
        let capture = PickupLogCapture { frame: common::frame(&pixels, at_ms), origin: ScreenPoint { x: band.x, y: band.y }, ui_size };
        let reading = reader.read(&capture);
        read.seen_clear += usize::from(matches!(reading.seen, PickupLogSeen::Clear { .. }));
        read.lines.extend(reading.arrived.iter().cloned());
        read.at.extend(reading.arrived.iter().map(|line| (at_ms, line.clone())));
        minerva.on_pickup_log_reading(reading);
    }
    let loot = &minerva.panel_state().exp_tracking.loot;
    read.mesos = loot.mesos;
    read.items = loot.items.clone();
    read
}

/// Whether a line read is the line that arrived: for an Item, its quantity and tab (its name is
/// matched to the item files, and checked as tallied).
fn is(read: &PickupLine, arrived: &PickupLine) -> bool {
    match (read, arrived) {
        (PickupLine::Item { quantity: q, tab: t, .. }, PickupLine::Item { quantity, tab, .. }) => q == quantity && t == tab,
        _ => read == arrived,
    }
}

/// How many lines of a clip arrived, and how many a reading got right: each line read is a line
/// that arrived, in order (an `Unreadable` stands for any), never another or one more.
fn right(name: &str, read: &[PickupLine]) -> (usize, usize) {
    let arrived: Vec<PickupLine> = truth(name).into_iter().flat_map(|(_, lines)| lines).collect();
    let mut remaining = &arrived[..];
    let mut right = 0;
    for line in read {
        let at = remaining
            .iter()
            .position(|want| *line == PickupLine::Unreadable || is(line, want))
            .unwrap_or_else(|| panic!("{name}: {line:?} is not a line that arrived (or arrived once already); read {read:?}; arrived {arrived:?}"));
        right += usize::from(*line != PickupLine::Unreadable);
        remaining = &remaining[at + 1..];
    }
    (right, arrived.len())
}

/// What a clip reads, at the least, of the lines that arrived: the log is found in each from a
/// cold start (the Bug Report's reader found it in none), and what is left unread is what the
/// clip's cover, or the blur of a drawing at this size, leaves.
fn found_and_read(name: &str, at_least: usize) {
    let read = clip(name);
    assert!(read.seen_clear > 0, "{name}: the log was never found");
    let (right, arrived) = right(name, &read.lines);
    assert!(right >= at_least, "{name}: read {right} of the {arrived} lines that arrived, {at_least} at least; read {:?}", read.lines);
}

#[test]
fn a_few_mesos_and_exp_lines_are_read_at_a_1280_by_720_ui_size() {
    found_and_read("exp-and-mesos", 4);
}

#[test]
fn etc_and_use_items_and_mesos_in_a_busy_log_are_read_at_a_1280_by_720_ui_size() {
    found_and_read("etc-and-use-items", 4);
}

#[test]
fn lines_arriving_under_and_beside_a_skill_effect_are_read_at_a_1280_by_720_ui_size() {
    found_and_read("skill-effect", 3);
}
#[test]
fn a_burst_of_lines_is_read_at_a_1280_by_720_ui_size() {
    found_and_read("burst", 3);
}

const CLIPS: [&str; 4] = ["exp-and-mesos", "etc-and-use-items", "skill-effect", "burst"];
#[test]
fn a_clip_never_counts_a_line_that_did_not_arrive_or_one_it_already_did() {
    for name in CLIPS {
        let read = clip(name);
        let arrived = truth(name).into_iter().flat_map(|(_, lines)| lines).collect::<Vec<_>>();
        right(name, &read.lines);
        let mesos: u64 = arrived.iter().filter_map(|line| if let PickupLine::Mesos { amount } = line { Some(*amount) } else { None }).sum();
        assert!(read.mesos <= mesos, "{name}: counted {} mesos of the {mesos} that arrived", read.mesos);
        for item in &read.items {
            let arrived: u64 = arrived
                .iter()
                .filter_map(|line| match line {
                    PickupLine::Item { name, quantity, .. } if item.name.starts_with(name.as_str()) => Some(u64::from(*quantity)),
                    _ => None,
                })
                .sum();
            assert!(item.count <= arrived, "{name}: counted {} {} of the {arrived} that arrived", item.count, item.name);
        }
    }
}

#[test]
fn a_line_is_counted_in_the_frame_that_brought_it_or_not_at_all() {
    for name in CLIPS {
        let read = clip(name);
        let truth = truth(name);
        for (at_ms, _) in &read.at {
            let counted = read.at.iter().filter(|(ms, _)| ms == at_ms).count();
            let arrived = truth
                .iter()
                .find(|(frame, _)| frame.starts_with(&format!("{at_ms:08}")))
                .map_or(0, |(_, lines)| lines.len());
            assert!(counted <= arrived, "{name}: {counted} lines counted at {at_ms}, {arrived} arrived in it");
        }
    }
}

#[test]
fn a_stack_that_stays_as_it_was_counts_nothing_new() {
    // Nothing arrives after 46800 in the busy clip, though every line of the stack is at full
    // brightness and a Mob's sprite passes over its top row: a reader that took that row for a line
    // that was gone counted a second `+73` and a second Ligator Skin at 47400.
    let read = clip("etc-and-use-items");
    let later = read.at.iter().filter(|(at_ms, _)| *at_ms > 46_800).collect::<Vec<_>>();
    assert!(later.is_empty(), "lines counted after the last that arrived: {later:?}");
}