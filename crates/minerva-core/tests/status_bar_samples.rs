//! Every Status Bar sample, fed through the core's public inputs as a capture of the bottom
//! of the game's client area, reads as its name says, or as unreadable: never another
//! level, job or name.
//!
//! - A sample is named `<level>__<job>__<Name>--<what>.png`: the level after "LV.", the job
//!   in lower case with `-` for its spaces, and the name as the game writes it. A crop cut
//!   down to the level and job, for a player who didn't agree to their name being kept, is
//!   `<level>__<job>--<what>.png`: no name may be read in it. One with no Status Bar in it is
//!   `no-status-bar--<what>.png`.
//! - The `<ui-size>-<w>x<h>` samples (2026-10-22, #132) are the bar cut by hand from the probe's
//!   record mode at each of the game's UI Sizes (Default Ratio, with its filter or not, and
//!   Ideal Ratio) at 1366x768 to 2560x1440: drawn at ×0.64 to ×1.28 of the rest.
//! - Each frame is fed as it is and as the darker colours of live captures (`prep.rs`), to a
//!   core of its own, at the screen's origin; and scaled to sizes no sample is at, where it
//!   may read as unreadable, and is found where it is at that size (ADR 0020).
//! - The `--window` samples in `samples/minimap` are whole game windows: the bottom of their
//!   client area, as the shell captures it, reads too.

mod common;

use common::{FakeSource, client_of, SUNSET_SAMPLE, KERNING_SAMPLE, CONSTRUCTION_SAMPLE};
use minerva_core::{Minerva, PanelState, ScreenPoint, ScreenRect, Settings, StatusBar, StatusBarCapture};

/// What a sample's name says it shows: `None` for no Status Bar, else its level, its job
/// (as the file names it) and its name, if the crop keeps it.
type Expected = Option<(u32, String, Option<String>)>;

fn expected(file: &str) -> Expected {
    let (read, _) = file.trim_end_matches(".png").split_once("--").unwrap_or_else(|| panic!("{file}: no `--`"));
    if read == "no-status-bar" {
        return None;
    }
    let parts: Vec<&str> = read.split("__").collect();
    let level = parts[0].parse().unwrap_or_else(|_| panic!("{file}: no level"));
    match parts[..] {
        [_, job] => Some((level, job.to_string(), None)),
        [_, job, name] => Some((level, job.to_string(), Some(name.to_string()))),
        _ => panic!("{file}: not <level>__<job>[__<Name>]--<what>.png"),
    }
}

fn sample_files() -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(common::samples_in("status-bar"))
        .expect("samples/status-bar")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    files
}

/// A core over a 1920x1200 game window at the screen's origin.
fn started() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.restore(Settings::default());
    minerva.on_game_window(Some(common::window_at_origin(1920, 1200)));
    minerva.tick(0);
    minerva
}

/// What a Status Bar read as: its level, job and name, `None` where unreadable.
type Read = Option<(Option<u32>, Option<String>, Option<String>)>;

/// What `state` says the Status Bar read.
fn read(state: &PanelState) -> Read {
    let bar: &StatusBar = state.status_bar.as_ref()?;
    let job = bar.job.as_ref().map(|job| job.to_lowercase().replace(' ', "-"));
    Some((bar.level, job, bar.name.clone()))
}

/// Whether `read` is what `expected` says, but for what couldn't be read; and whether all of
/// it was read.
fn judge(expected: &Expected, read: &Read) -> (bool, bool) {
    match (expected, read) {
        (None, None) => (true, true),
        (None, Some(_)) => (false, false),
        (Some(_), None) => (true, false),
        (Some((level, job, name)), Some((read_level, read_job, read_name))) => {
            let fits = |read: &Option<String>, want: Option<&String>| read.is_none() || read.as_ref() == want;
            let right = read_level.is_none_or(|l| l == *level) && fits(read_job, Some(job)) && fits(read_name, name.as_ref());
            let whole = read_level.is_some() && read_job.is_some() && (read_name.is_some() == name.is_some());
            (right, whole)
        }
    }
}

#[test]
fn every_status_bar_sample_reads_as_its_name_says_or_as_unreadable_never_wrong() {
    let files = sample_files();
    assert!(files.len() >= 4, "expected the Status Bar samples, found {}", files.len());
    let mut wrong = Vec::new();
    let mut unreadable = Vec::new();
    for file in &files {
        let expected = expected(file);
        let img = image::open(common::samples_in("status-bar").join(file)).unwrap_or_else(|e| panic!("{file}: {e}")).to_rgba8();
        for brightness in [1.0, 0.935] {
            let mut minerva = started();
            let capture = StatusBarCapture { frame: common::dimmed(&img, brightness), origin: ScreenPoint { x: 0, y: 0 } };
            let read = read(minerva.on_status_bar_capture(capture));
            match judge(&expected, &read) {
                (false, _) => wrong.push(format!("{file} ×{brightness}: {read:?}")),
                (true, false) => unreadable.push(format!("{file} ×{brightness}: {read:?}")),
                (true, true) => {}
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
    // The samples were all read whole when they were added: a reader that reads less of them
    // has got worse, though it says nothing wrong.
    assert!(unreadable.is_empty(), "now unreadable:\n{}", unreadable.join("\n"));
}

/// The Status Bar `img` reads as in a core of its own, at the screen's origin.
fn bar_of(img: &image::RgbaImage) -> (Option<StatusBar>, Read) {
    let mut minerva = started();
    let capture = StatusBarCapture { frame: common::dimmed(img, 1.0), origin: ScreenPoint { x: 0, y: 0 } };
    let state = minerva.on_status_bar_capture(capture);
    (state.status_bar.clone(), read(state))
}

#[test]
fn every_status_bar_sample_at_other_ui_sizes_reads_as_its_name_says_or_as_unreadable_never_wrong() {
    let mut wrong = Vec::new();
    for file in sample_files() {
        let expected = expected(&file);
        let img = image::open(common::samples_in("status-bar").join(&file)).unwrap_or_else(|e| panic!("{file}: {e}")).to_rgba8();
        let area = bar_of(&img).0.expect("every sample reads").area;
        for size in common::OTHER_UI_SIZES {
            let (bar, read) = bar_of(&common::scaled(&img, size));
            if !judge(&expected, &read).0 {
                wrong.push(format!("{file} ×{size}: {read:?}"));
            }
            // What was found is put back where it is at this size, give or take 3% for the
            // measuring and a few pixels for the finding.
            let width = area.width as f32 * size;
            let off = |found: i32, sample: i32| (found as f32 - sample as f32 * size).abs() > 0.03 * width;
            let misplaced = |a: ScreenRect| off(a.x, area.x) || off(a.y, area.y) || off(a.width as i32, area.width as i32);
            if let Some(found) = bar.map(|bar| bar.area).filter(|&a| misplaced(a)) {
                wrong.push(format!("{file} ×{size}: found at {found:?}, at the samples' size {area:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}

/// A sample at each UI Size the game was seen to draw the bar at, filtered or not.
const AT_EACH_UI_SIZE: [&str; 7] = [
    "32__wizard-(fire,-poison)__Egbert--perion-live.png",
    "34__wizard-(fire,-poison)__Egbert--default-ratio-filter-1920x1200.png",
    "34__wizard-(fire,-poison)__Egbert--ideal-ratio-1366x768.png",
    "34__wizard-(fire,-poison)__Egbert--default-ratio-1920x1080.png",
    "34__wizard-(fire,-poison)__Egbert--default-ratio-filter-1920x1080.png",
    "34__wizard-(fire,-poison)__Egbert--default-ratio-2560x1440.png",
    "34__wizard-(fire,-poison)__Egbert--ideal-ratio-2560x1440.png",
];

#[test]
fn the_pointer_anywhere_over_the_bar_at_any_ui_size_never_reads_as_another() {
    let mut wrong = Vec::new();
    for file in AT_EACH_UI_SIZE {
        let expected = expected(file);
        let img = image::open(common::samples_in("status-bar").join(file)).unwrap_or_else(|e| panic!("{file}: {e}")).to_rgba8();
        let area = bar_of(&img).0.expect("every sample reads").area;
        // The pointer is the game's, the same size at any UI Size.
        let step = (area.height / 5).max(2);
        for y in (area.y as u32..(area.y as u32 + area.height)).step_by(step as usize) {
            for x in (area.x as u32..area.x as u32 + area.width).step_by(step as usize) {
                let mut covered = img.clone();
                common::pointer(&mut covered, (x, y), 14);
                let read = bar_of(&covered).1;
                if !judge(&expected, &read).0 {
                    wrong.push(format!("{file}, pointer at ({x}, {y}): {read:?}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}

/// The minimap samples that are whole game windows: all show the Status Bar of the
/// maintainer's own Character, Egbert, a level 13 Magician.
const WINDOWS: [&str; 4] = [
    "hidden-street__northern-top-of-construction-site--window",
    CONSTRUCTION_SAMPLE,
    KERNING_SAMPLE,
    SUNSET_SAMPLE,
];

/// A window sample's client area, on screen at the origin: the shell's capture for the
/// Status Bar is what the core asks for of it.
fn captured(window: &str, brightness: f32) -> (Minerva, StatusBarCapture) {
    let client = client_of(&common::load(window));
    let mut minerva = started();
    minerva.on_game_window(Some(common::window_at_origin(client.width(), client.height())));
    let strip = minerva.panel_state().status_bar_capture.expect("the Status Bar is captured with the game in front");
    assert_eq!((strip.x, strip.width), (0, client.width()), "{window}: the whole width");
    assert_eq!(strip.y as u32 + strip.height, client.height(), "{window}: down to the bottom");
    let crop = image::imageops::crop_imm(&client, 0, strip.y as u32, strip.width, strip.height).to_image();
    let capture = StatusBarCapture { frame: common::dimmed(&crop, brightness), origin: ScreenPoint { x: strip.x, y: strip.y } };
    (minerva, capture)
}

#[test]
fn the_status_bar_is_found_at_the_bottom_of_a_whole_client_area_wherever_it_sits() {
    let want = (Some(13), Some("magician".to_string()), Some("Egbert".to_string()));
    for window in WINDOWS {
        for brightness in [1.0, 0.935] {
            let (mut minerva, capture) = captured(window, brightness);
            let state = minerva.on_status_bar_capture(capture);
            assert_eq!(read(state), Some(want.clone()), "{window} ×{brightness}");
        }
    }
}

#[test]
fn a_name_under_the_pointer_is_unreadable_and_the_rest_still_reads() {
    let file = "13__magician__Egbert--kerning-city-window.png";
    let mut img = image::open(common::samples_in("status-bar").join(file)).expect(file).to_rgba8();
    // The game's pointer over the "b" of "Egbert".
    let tip = (165, img.height() - 18);
    common::pointer(&mut img, tip, 12);
    let mut minerva = started();
    let capture = StatusBarCapture { frame: common::dimmed(&img, 1.0), origin: ScreenPoint { x: 0, y: 0 } };

    let read = read(minerva.on_status_bar_capture(capture));

    assert_eq!(read, Some((Some(13), Some("magician".to_string()), None)));
}

#[test]
fn the_top_of_a_client_area_has_no_status_bar() {
    for window in WINDOWS {
        let client = client_of(&common::load(window));
        let top = image::imageops::crop_imm(&client, 0, 0, client.width(), 96).to_image();
        let mut minerva = started();
        let capture = StatusBarCapture { frame: common::dimmed(&top, 1.0), origin: ScreenPoint { x: 0, y: 0 } };
        assert_eq!(minerva.on_status_bar_capture(capture).status_bar, None, "{window}");
    }
}
