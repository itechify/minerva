//! Every Item Tooltip sample, fed through the core's public inputs as the capture of an Item
//! Card press, shows the card of the item its name says, or says it couldn't be read, never
//! another item's card (ADR 0007, ADR 0010).
//!
//! - Samples are named as the probe's item mode and a Bug Report's crops name them:
//!   `<item-name>@<x>,<y>[--<tag>].png`, the item's name in lower case with anything but
//!   letters and digits as `-`, and (x, y) the pointer in the frame. `no-tooltip@<x>,<y>`
//!   frames have no Item Tooltip by the pointer.
//! - Each frame is fed as the game's client area at the screen's origin.

mod common;

use common::FakeSource;
use common::items::{ItemWorld, etc, item_world};
use minerva_core::{
    Minerva, Frame, GameWindow, ItemCapture, ItemCardView, ItemLookup, PanelState, Request, ScreenPoint, ScreenRect,
    Settings, Size,
};
use std::path::{Path, PathBuf};

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/item-tooltip")
}

/// A sample: the item its name says (`None` for `no-tooltip`), and where the pointer is.
struct Sample {
    file: String,
    item: Option<String>,
    pointer: ScreenPoint,
}

fn samples() -> Vec<Sample> {
    let mut files: Vec<String> = std::fs::read_dir(samples_dir())
        .expect("samples/item-tooltip")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let stem = file.trim_end_matches(".png");
            let stem = stem.split_once("--").map_or(stem, |(stem, _)| stem);
            let (item, at) = stem.split_once('@').unwrap_or_else(|| panic!("{file}: <item>@<x>,<y>"));
            let (x, y) = at.split_once(',').unwrap_or_else(|| panic!("{file}: <item>@<x>,<y>"));
            let pointer = ScreenPoint { x: x.parse().expect("x"), y: y.parse().expect("y") };
            Sample { item: (item != "no-tooltip").then(|| item.to_string()), file, pointer }
        })
        .collect()
}

/// A name as samples write it: lower case, anything but letters and digits as `-`.
fn slug(name: &str) -> String {
    let words: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    words.join("-")
}

/// Made-up item data with an item of each sample's name, its words capitalised ("stirge-
/// wing": "Stirge Wing"), as the game names items.
fn sample_items(samples: &[Sample]) -> ItemWorld {
    let mut items = item_world(Vec::new());
    for (n, item) in samples.iter().filter_map(|sample| sample.item.as_deref()).enumerate() {
        let name: Vec<String> = item
            .split('-')
            .map(|word| {
                let mut letters = word.chars();
                letters.next().map_or_else(String::new, |first| first.to_ascii_uppercase().to_string() + letters.as_str())
            })
            .collect();
        items = items.with(etc(4_100_000 + n as u64, &name.join(" ")).sells_for(10));
    }
    items
}

/// What the core shows for a sample pressed with Ctrl+Shift+I: the card of the item it read,
/// or the alert.
fn press(minerva: &mut Minerva, sample: &Sample) -> PanelState {
    let img = image::open(samples_dir().join(&sample.file)).unwrap_or_else(|e| panic!("{}: {e}", sample.file)).to_rgba8();
    let frame = Frame { width: img.width(), height: img.height(), rgba: img.into_raw(), at_ms: 1_000 };
    assert_eq!(minerva.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
    minerva.on_item_capture(ItemCapture { frame, origin: ScreenPoint { x: 0, y: 0 }, pointer: sample.pointer }).clone()
}

/// A core over a 1920x1200 game window at the screen's origin, with the samples' items in.
fn started(samples: &[Sample]) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline().with_items(sample_items(samples)));
    minerva.restore(Settings::default());
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    minerva.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    minerva.set_item_card_size(Size { width: 280, height: 160 });
    minerva.tick(0);
    minerva
}

/// The card's item, by its name as samples write it; or what the alert says.
fn shown(state: &PanelState) -> Result<String, Option<ItemLookup>> {
    match state.item_card.as_ref().map(|shown| &shown.view) {
        Some(ItemCardView::Card(card)) => Ok(slug(&card.name)),
        _ => Err(state.item_lookup.as_ref().map(|line| line.outcome.clone())),
    }
}

#[test]
fn every_sample_shows_its_items_card_or_says_it_couldnt_read_it_never_another_items() {
    let samples = samples();
    assert!(samples.len() >= 3, "expected the Item Tooltip samples, found {}", samples.len());
    let mut minerva = started(&samples);

    let mut wrong = Vec::new();
    for sample in &samples {
        let state = press(&mut minerva, sample);
        let fine = match (&sample.item, shown(&state)) {
            (Some(item), Ok(read)) => *item == read,
            (Some(_), Err(Some(ItemLookup::Unreadable | ItemLookup::NoSuchItem { .. }))) => true,
            (None, Err(Some(ItemLookup::NoTooltip))) => true,
            _ => false,
        };
        if !fine {
            wrong.push(format!("{}: {:?}", sample.file, shown(&state)));
        }
        minerva.close_item_card();
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
}

/// The Item Tooltip's fill is see-through: where the tooltip lies over the Item Inventory
/// window, the fill left of the name's bullet is light, not the dark slate it is over the
/// game world.
#[test]
fn a_tooltip_over_the_item_inventory_window_is_read() {
    let samples = samples();
    let mut minerva = started(&samples);

    for file in ["stirge-wing@60,20.png", "horny-mushroom-cap@60,20.png", "screw@60,20.png"] {
        let sample = samples.iter().find(|sample| sample.file == file).unwrap_or_else(|| panic!("no {file}"));
        let state = press(&mut minerva, sample);
        assert_eq!(shown(&state), Ok(sample.item.clone().unwrap()), "{file}");
        minerva.close_item_card();
    }
}

/// The name ends at the tooltip's edge, not past it: where the tooltip lies over the Player
/// Shop's chat, the light chat window past the edge isn't more of the name, though it is as
/// light as the name's letters, and the chat's text shows through the edge. Item Lookup
/// says the name as read.
#[test]
fn a_tooltip_over_a_light_window_is_read_to_its_edge() {
    let samples = samples();
    let mut minerva = started(&samples);

    let file = "one-handed-sword-attack-scroll-greater@10,15.png";
    let sample = samples.iter().find(|sample| sample.file == file).unwrap_or_else(|| panic!("no {file}"));
    let img = image::open(samples_dir().join(file)).expect(file).to_rgba8();
    let frame = Frame { width: img.width(), height: img.height(), rgba: img.into_raw(), at_ms: 1_000 };
    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), Some(Request::CaptureItemTooltip));
    let state = minerva.on_item_capture(ItemCapture { frame, origin: ScreenPoint { x: 0, y: 0 }, pointer: sample.pointer });
    let read = match state.item_lookup.as_ref().map(|line| &line.outcome) {
        Some(ItemLookup::LookingUp { read } | ItemLookup::NotFound { read }) => read.clone(),
        other => panic!("{file}: {other:?}"),
    };
    assert_eq!(read, "One-Handed Sword Attack Scroll: Greater");
}
