//! Every EXP bar sample, fed through the core's public inputs, reads as what it shows, or as
//! unreadable; never as a wrong number. Two layers (ADR 0017):
//!
//! - The samples here are the EXP figures alone, fed as a frame of them
//!   ([`Minerva::on_exp_bar_frame`]): they test reading the figures.
//! - The `--window` samples in `samples/minimap` are whole game windows: the bottom of their
//!   client area, captured as the shell captures it for the Status Bar, tests finding the
//!   figures there too.
//!
//! - Single samples are named for their reading: `<exp>__<percent>[--variant].png`
//!   ("160__3.81--kerning-city-window"), or `unreadable[--variant].png`. The
//!   `--window` ones are client (1005, h-56) 240x32 cropped from the window samples in
//!   `samples/minimap`. The unreadable ones are frames ocrs once misread, named for
//!   what they show and how it went wrong ("unreadable--68012-71.07-seven-as-two"):
//!   a Bug Report's EXP Bar Region crops (2026-10-11, client (1010, 1143) 396x33 of a
//!   1920x1200 client), each of a misread that ended the Session as a character
//!   switch, or could have. Those are crops of the EXP Bar Region players once marked
//!   (ADR 0005), from before the figures were found in the Status Bar.
//! - `recording-*` folders are the probe's record mode (one crop a second, named by
//!   capture time in ms) with their readings in `readings.json` (`null`: unreadable).
//!   2026-10-07: 150 s of hunting at Lv 15, client (1005,1144) 240x32 of a 1920x1200
//!   client. Its readings all fit one level total of ~16,682 EXP (the game rounds the
//!   percentage), which is how the frames ocrs misread were caught.

mod common;

use common::FakeSource;
use minerva_core::{
    ExpBar, Frame, GameWindow, Minerva, PanelState, Rect, ScreenPoint, ScreenRect, Settings, Size, StatusBarCapture,
    TitleRegion,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/exp-bar")
}

fn frame(path: &Path) -> Frame {
    let img = image::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())).to_rgba8();
    common::frame(&img, 0)
}

/// A fresh core (so no earlier reading lingers), set up as a player's is, over a game window
/// with this client area at the screen's origin: the Grind Tracker is on.
fn started(width: u32, height: u32) -> Minerva {
    let client = Size { width, height };
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.restore(Settings {
        title_region: Some(TitleRegion { marked: Rect { x: 70, y: 40, width: 130, height: 48 }, client }),
        ..Settings::default()
    });
    let screen = ScreenRect { x: 0, y: 0, width, height };
    minerva.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    minerva
}

/// What the panel shows for a frame of the EXP figures alone.
fn read(frame: Frame) -> Option<ExpBar> {
    started(1920, 1200).on_exp_bar_frame(frame).exp_bar
}

/// The capture the core asks for of a client area for the Status Bar, at the screen's origin.
fn status_bar_capture(state: &PanelState, client: &image::RgbaImage, at_ms: u64) -> StatusBarCapture {
    let strip = state.status_bar_capture.expect("the Status Bar is captured with the game in front");
    let crop = image::imageops::crop_imm(client, strip.x as u32, strip.y as u32, strip.width, strip.height).to_image();
    StatusBarCapture { frame: common::frame(&crop, at_ms), origin: ScreenPoint { x: strip.x, y: strip.y } }
}

/// The minimap samples that are whole game windows, and what their EXP bars show.
const WINDOWS: [(&str, ExpBar); 4] = [
    ("hidden-street__northern-top-of-construction-site--window", ExpBar { exp: 335, percent_hundredths: 798 }),
    ("victoria-road__construction-site-north-of-kerning-city--window", ExpBar { exp: 266, percent_hundredths: 633 }),
    ("victoria-road__kerning-city--window", ExpBar { exp: 160, percent_hundredths: 381 }),
    ("victoria-road__sunset-sky--window", ExpBar { exp: 160, percent_hundredths: 381 }),
];

#[test]
fn every_window_samples_figures_are_found_in_its_status_bar_and_read() {
    let mut wrong = Vec::new();
    for (window, shows) in WINDOWS {
        let client = common::client_of(&common::load(window));
        let mut minerva = started(client.width(), client.height());
        let capture = status_bar_capture(minerva.panel_state(), &client, 0);
        let read = minerva.on_status_bar_capture(capture).exp_bar;
        if read != Some(shows) {
            wrong.push(format!("{window}: panel shows {read:?}"));
        }
    }
    assert!(wrong.is_empty(), "wrong or unread:\n{}", wrong.join("\n"));
}

/// "160__3.81" -> 160 EXP at 381 hundredths; "unreadable" -> `None`.
fn named_reading(name: &str) -> Option<ExpBar> {
    let answer = name.split_once("--").map_or(name, |(a, _)| a);
    if answer == "unreadable" {
        return None;
    }
    let (exp, percent) = answer.split_once("__").expect("<exp>__<percent> or unreadable");
    let (whole, hundredths) = percent.split_once('.').expect("a percentage with two decimals");
    Some(ExpBar {
        exp: exp.parse().expect("exp"),
        percent_hundredths: whole.parse::<u32>().expect("percent") * 100 + hundredths.parse::<u32>().expect("percent"),
    })
}

#[test]
fn every_single_sample_reads_as_its_filename_names() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 3, "expected the EXP bar samples, found {}", paths.len());

    let mut wrong = Vec::new();
    for path in &paths {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let shown = read(frame(path));
        if shown != named_reading(&name) {
            wrong.push(format!("{name}: panel shows {shown:?}"));
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expected {
    exp: u64,
    percent_hundredths: u32,
}

#[test]
fn every_recorded_frame_reads_as_its_readings_file_says() {
    let recordings: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_dir())
        .collect();
    assert!(!recordings.is_empty(), "expected a recording");

    let mut wrong = Vec::new();
    for dir in &recordings {
        let json = std::fs::read_to_string(dir.join("readings.json")).expect("readings.json");
        let expected: BTreeMap<String, Option<Expected>> = serde_json::from_str(&json).expect("readings json");
        let pngs = std::fs::read_dir(dir).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "png"));
        assert_eq!(pngs.count(), expected.len(), "{}: a reading for every crop", dir.display());
        for (file, want) in expected {
            let want = want.map(|e| ExpBar { exp: e.exp, percent_hundredths: e.percent_hundredths });
            let shown = read(frame(&dir.join(&file)));
            if shown != want {
                wrong.push(format!("{}/{file}: panel shows {shown:?}, expected {want:?}", dir.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}
