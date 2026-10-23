//! Every minimap sample, fed through the core's public inputs, shows the map its
//! filename names: `street-name__map-name[--variant].png`. The title is found in it
//! wherever it is (ADR 0021), open on two lines or on one, "Street : Map".
//!
//! `--window` samples are whole game windows: their client area is fed whole, as the shell
//! captures it while it looks for the title. The two with one-line titles, Perion
//! Department Store and Subway Ticketing Booth, are maps without a minimap. `--title-region`
//! samples are the 560x96 rectangle at client (0, 16) that the probe once captured, the
//! WORLD button cut off at their top. The Tree That Grew II's is a live capture. I's and
//! III's are that same rectangle cropped from the player's screenshots of the game's client
//! area (no Windows title bar, so the minimap starts at y = 0); those screenshots are
//! hand-cropped to within a few px of the client area, so the crops can sit a pixel or three
//! off the live capture. In I's, the mouse pointer covers the map icon. `--collapsed` samples
//! are a Bug Report's crops of where a collapsed minimap's one-line title was found, the WORLD
//! button 23 rows down; Free Market <1>'s are cut to the bar, past which other players' shops
//! stand, and `--collapsed-dim`'s is taken as the map fades in, the bar and its text darker.

mod common;

use common::{FakeSource, sample_frame, samples_dir};
use minerva_core::{Minerva, CurrentMap};
use std::path::PathBuf;

/// Map names that differ only in a trailing Roman numeral: ocrs reads the game font's
/// "II" and "III" as an unknown glyph, which once snapped both to "The Tree That Grew I".
#[test]
fn the_tree_that_grew_i_ii_and_iii_each_show_their_own_map() {
    for (sample, id, map_name) in [
        ("victoria-road__the-tree-that-grew-i--title-region", "010002060", "The Tree That Grew I"),
        ("victoria-road__the-tree-that-grew-ii--title-region", "010002061", "The Tree That Grew II"),
        ("victoria-road__the-tree-that-grew-iii--title-region", "010002062", "The Tree That Grew III"),
    ] {
        let mut minerva = Minerva::new(FakeSource::osms());
        minerva.on_frame(sample_frame(sample, 0));
        let state = minerva.on_frame(sample_frame(sample, 333));

        let shown = state.map.as_ref().map(|m| (m.id.as_str(), m.street_name.as_str(), m.map_name.as_str()));
        assert_eq!(shown, Some((id, "Victoria Road", map_name)), "{sample}");
        assert!(!state.not_sure, "{sample}");
    }
}

/// ocrs runs this title's last two words together ("Rocky RoadI"); the panel still
/// shows the real map.
#[test]
fn a_slightly_misread_title_snaps_to_the_right_map() {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.on_frame(sample_frame("victoria-road__rocky-road-i--title-region", 0));
    let state = minerva.on_frame(sample_frame("victoria-road__rocky-road-i--title-region", 333));

    assert_eq!(
        state.map,
        Some(CurrentMap {
            id: "010004060".into(),
            street_name: "Victoria Road".into(),
            map_name: "Rocky Road I".into(),
            url: "https://meowdb.com/msclassic/maps/010004060".into(),
            mark: None,
        })
    );
    assert!(!state.not_sure);
}

/// A name as the sample filenames spell it: lower-case words joined by `-`.
/// `"Rocky Road I"` gives `"rocky-road-i"`; a reading such as `"Rocky RoadI"` would not.
fn slug(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Every sample, in one session, each read on two consecutive frames (the panel only
/// switches maps after repeated confident reads).
#[test]
fn every_sample_shows_the_street_name_and_map_name_its_filename_names() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 6, "expected the minimap samples, found {}", paths.len());

    let mut minerva = Minerva::new(FakeSource::osms());
    let mut wrong = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let answer = name.split_once("--").map_or(name.as_str(), |(a, _)| a);
        let (street, map) = answer.split_once("__").expect("street__map filename");
        let at_ms = 1000 * i as u64;
        minerva.on_frame(sample_frame(&name, at_ms));
        let state = minerva.on_frame(sample_frame(&name, at_ms + 333));

        let shown = state.map.as_ref().map(|m| (slug(&m.street_name), slug(&m.map_name)));
        if shown != Some((street.to_string(), map.to_string())) || state.not_sure {
            wrong.push(format!("{name}: panel shows {:?} (not sure: {})", state.map, state.not_sure));
        }
    }
    assert!(wrong.is_empty(), "wrong maps:\n{}", wrong.join("\n"));
}

/// A map without a minimap shows its title on one line, "Street : Map", on the minimap
/// window's bar, as a collapsed minimap does (ADR 0021).
#[test]
fn a_map_without_a_minimap_shows_the_map_its_one_line_title_names() {
    for (sample, id, map_name) in [
        ("victoria-road__perion-department-store--window", "010004002", "Perion Department Store"),
        ("victoria-road__subway-ticketing-booth--window", "010003060", "Subway Ticketing Booth"),
    ] {
        let mut minerva = Minerva::new(FakeSource::osms());
        let state = common::arrive(&mut minerva, sample, 0);

        let shown = state.map.as_ref().map(|m| (m.id.as_str(), m.street_name.as_str(), m.map_name.as_str()));
        assert_eq!(shown, Some((id, "Victoria Road", map_name)), "{sample}");
        assert!(!state.not_sure, "{sample}");
    }
}

/// Every sample at UI Sizes no sample is at (ADR 0020): the minimap found and read at the
/// samples' size, as its map, or as unreadable, never as another map.
#[test]
fn every_sample_at_another_ui_size_shows_its_map_or_none() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(samples_dir())
        .expect("samples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    paths.sort();
    let (mut wrong, mut unread) = (Vec::new(), Vec::new());
    for path in &paths {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let answer = name.split_once("--").map_or(name.as_str(), |(a, _)| a);
        let (street, map) = answer.split_once("__").expect("street__map filename");
        let img = common::load(&name);
        // A window's client area, cut to its top, where the minimaps are: scaling the rest is slow.
        let img = if name.ends_with("--window") {
            let client = common::client_of(&img);
            image::imageops::crop_imm(&client, 0, 0, client.width(), 300).to_image()
        } else {
            img
        };
        for size in common::OTHER_UI_SIZES {
            let scaled = common::scaled(&img, size);
            let mut minerva = Minerva::new(FakeSource::osms());
            minerva.on_frame(common::frame(&scaled, 0));
            let state = minerva.on_frame(common::frame(&scaled, 333));
            match state.map.as_ref().map(|m| (slug(&m.street_name), slug(&m.map_name))) {
                Some(shown) if shown == (street.to_string(), map.to_string()) => {}
                Some(shown) => wrong.push(format!("{name} ×{size}: panel shows {shown:?}")),
                None => unread.push(format!("{name} ×{size}")),
            }
        }
    }
    eprintln!("unreadable:\n{}", unread.join("\n"));
    assert!(wrong.is_empty(), "wrong maps:\n{}", wrong.join("\n"));
}
