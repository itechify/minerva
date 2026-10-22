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
//!   Ideal Ratio) at 1366x768 to 2560x1440: drawn at x0.64 to x1.28 of the rest.
//! - Each frame is fed as it is and as the darker colours of live captures (`prep.rs`), to a
//!   core of its own, at the screen's origin; and scaled to sizes no sample is at, where it
//!   may read as unreadable, and is found where it is at that size (ADR 0020).
//! - The `--window` samples in `samples/minimap` are whole game windows: the bottom of their
//!   client area, as the shell captures it, reads too.

mod common;

use common::{FakeSource, client_of};
use minerva_core::{Frame, GameWindow, Minerva, PanelState, ScreenPoint, ScreenRect, Settings, StatusBar, StatusBarCapture};
use std::path::{Path, PathBuf};

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/status-bar")
}

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
    let mut files: Vec<String> = std::fs::read_dir(samples_dir())
        .expect("samples/status-bar")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    files
}

/// `img`'s pixels, their colours times `brightness`.
fn frame(img: &image::RgbaImage, brightness: f32) -> Frame {
    let mut rgba = img.as_raw().clone();
    for (i, channel) in rgba.iter_mut().enumerate() {
        if i % 4 != 3 {
            *channel = (*channel as f32 * brightness).round() as u8;
        }
    }
    Frame { width: img.width(), height: img.height(), rgba, at_ms: 1_000 }
}

/// A core over a 1920x1200 game window at the screen's origin.
fn started() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.restore(Settings::default());
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    minerva.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    minerva.tick(0);
    minerva
}

/// What `state` says the Status Bar read: its level, job and name, `None` where unreadable.
fn read(state: &PanelState) -> Option<(Option<u32>, Option<String>, Option<String>)> {
    let bar: &StatusBar = state.status_bar.as_ref()?;
    let job = bar.job.as_ref().map(|job| job.to_lowercase().replace(' ', "-"));
    Some((bar.level, job, bar.name.clone()))
}

/// Whether `read` is what `expected` says, but for what couldn't be read; and whether all of
/// it was read.
fn judge(expected: &Expected, read: &Option<(Option<u32>, Option<String>, Option<String>)>) -> (bool, bool) {
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
        let img = image::open(samples_dir().join(file)).unwrap_or_else(|e| panic!("{file}: {e}")).to_rgba8();
        for brightness in [1.0, 0.935] {
            let mut minerva = started();
            let capture = StatusBarCapture { frame: frame(&img, brightness), origin: ScreenPoint { x: 0, y: 0 } };
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

/// Sizes no sample is at, to scale each sample to (ADR 0020): from about the smallest the game
/// draws its UI at (×0.5, Default Ratio at 600 rows) to past the largest (×1.8, at 2160).
const OTHER_UI_SIZES: [f32; 4] = [0.55, 0.8, 1.45, 1.9];

#[test]
fn every_status_bar_sample_at_other_ui_sizes_reads_as_its_name_says_or_as_unreadable_never_wrong() {
    let mut wrong = Vec::new();
    for file in sample_files() {
        let expected = expected(&file);
        let img = image::open(samples_dir().join(&file)).unwrap_or_else(|e| panic!("{file}: {e}")).to_rgba8();
        let at = |minerva: &mut Minerva, img: &image::RgbaImage| {
            let capture = StatusBarCapture { frame: frame(img, 1.0), origin: ScreenPoint { x: 0, y: 0 } };
            minerva.on_status_bar_capture(capture).status_bar.clone()
        };
        let area = at(&mut started(), &img).expect("every sample reads").area;
        for size in OTHER_UI_SIZES {
            let (w, h) = ((img.width() as f32 * size).round() as u32, (img.height() as f32 * size).round() as u32);
            let scaled = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
            let mut minerva = started();
            let bar = at(&mut minerva, &scaled);
            let read = read(minerva.panel_state());
            if !judge(&expected, &read).0 {
                wrong.push(format!("{file} ×{size}: {read:?}"));
            }
            // What was found is put back where it is at this size: give or take a few pixels at
            // the samples' size (the bar's area is 331 wide there), and 2% for the measuring.
            let ui_size = area.width as f32 * size / 331.0;
            let off = |found: i32, sample: i32| {
                let at = sample as f32 * size;
                (found as f32 - at).abs() > 4.0 * ui_size + 0.02 * at
            };
|found: i32, at_samples: i32| (found as f32 - at_samples as f32 * size).abs() > 3.0 * size.max(1.0);
            let placed = |a: ScreenRect| off(a.x, area.x) || off(a.y, area.y) || off(a.width as i32, area.width as i32);
            if let Some(found) = bar.map(|bar| bar.area).filter(|&a| placed(a)) {
                wrong.push(format!("{file} ×{size}: found at {found:?}, at the samples' size {area:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}

/// The minimap samples that are whole game windows: all show the Status Bar of the
/// maintainer's own Character, Egbert, a level 13 Magician.
const WINDOWS: [&str; 4] = [
    "hidden-street__northern-top-of-construction-site--window",
    "victoria-road__construction-site-north-of-kerning-city--window",
    "victoria-road__kerning-city--window",
    "victoria-road__sunset-sky--window",
];

/// A window sample's client area, on screen at the origin: the shell's capture for the
/// Status Bar is what the core asks for of it.
fn captured(window: &str, brightness: f32) -> (Minerva, StatusBarCapture) {
    let client = client_of(&common::load(window));
    let mut minerva = started();
    let screen = ScreenRect { x: 0, y: 0, width: client.width(), height: client.height() };
    minerva.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    let strip = minerva.panel_state().status_bar_capture.expect("the Status Bar is captured with the game in front");
    assert_eq!((strip.x, strip.width), (0, client.width()), "{window}: the whole width");
    assert_eq!(strip.y as u32 + strip.height, client.height(), "{window}: down to the bottom");
    let crop = image::imageops::crop_imm(&client, 0, strip.y as u32, strip.width, strip.height).to_image();
    let capture = StatusBarCapture { frame: frame(&crop, brightness), origin: ScreenPoint { x: strip.x, y: strip.y } };
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
    let mut img = image::open(samples_dir().join(file)).expect(file).to_rgba8();
    // The game's pointer, white with a black outline, over the "b" of "Egbert".
    let (x, y) = (165, img.height() - 18);
    for dy in 0..12 {
        for dx in 0..=dy / 2 {
            let edge = dx == 0 || dx == dy / 2 || dy == 11;
            img.put_pixel(x + dx, y + dy, image::Rgba(if edge { [0, 0, 0, 255] } else { [255, 255, 255, 255] }));
        }
    }
    let mut minerva = started();
    let capture = StatusBarCapture { frame: frame(&img, 1.0), origin: ScreenPoint { x: 0, y: 0 } };

    let read = read(minerva.on_status_bar_capture(capture));

    assert_eq!(read, Some((Some(13), Some("magician".to_string()), None)));
}

#[test]
fn the_top_of_a_client_area_has_no_status_bar() {
    for window in WINDOWS {
        let client = client_of(&common::load(window));
        let top = image::imageops::crop_imm(&client, 0, 0, client.width(), 96).to_image();
        let mut minerva = started();
        let capture = StatusBarCapture { frame: frame(&top, 1.0), origin: ScreenPoint { x: 0, y: 0 } };
        assert_eq!(minerva.on_status_bar_capture(capture).status_bar, None, "{window}");
    }
}
