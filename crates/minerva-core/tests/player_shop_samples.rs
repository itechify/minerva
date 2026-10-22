//! Every Player Shop sample, fed through the core's public inputs as a Deal Finder capture,
//! shows a Deal Badge for each listing not sold out, about the item and price the sample's
//! readings say, or a "?" badge: never another item, never another price (ADR 0014).
//!
//! - Each sample is a frame cropped to a shop's item list, as a Bug Report's
//!   `crops/player-shop-<ms>.png` is, so no seller's name, shop title or chat is committed;
//!   beside it, `<name>.json` lists its listings not sold out, top to bottom, each with the
//!   item's name as the game shows it ("…" and all, for a name it cuts short), the quantity
//!   and the mesos; or `{ "covered": true }` for one the pointer or the game's tooltip
//!   covers. Read on its own, a covered listing's badge says it is covered, or it has none:
//!   with no capture before, a slot the tooltip hides wholly isn't known to hold a listing.
//! - Each frame is fed as the game's client area at the screen's origin, as it is and as
//!   the darker colours of live captures (`prep.rs`), to a core of its own.
//! - Each is fed again at the other UI Sizes in [`OTHER_UI_SIZES`], scaled smoothly as the game
//!   draws its UI larger or smaller (ADR 0020): there a listing may read as unreadable, never as
//!   another item or price, and its badge sits level with it.

mod common;

use common::FakeSource;
use common::items::{ItemWorld, etc, item_world};
use minerva_core::{
    DealBadge, Frame, GameWindow, ListedPrice, Minerva, PanelState, ScreenPoint, ScreenRect, Settings, ShopCapture, Size,
    Verdict,
};
use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A listing's band is this wide at the UI Size most samples were taken at.
const SAMPLES_BAND_W: f32 = 253.0;
/// The UI Sizes every sample is also read at, next to the samples': #130's (a band 162 px
/// wide), and two larger.
const OTHER_UI_SIZES: [f32; 3] = [0.64, 1.5, 1.8];

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/player-shop")
}

/// One listing as a sample's readings write it: read, or covered.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Listing {
    Read { name: String, quantity: u64, mesos: u64 },
    Covered { covered: bool },
}

struct Sample {
    file: String,
    listings: Vec<Listing>,
}

fn samples() -> Vec<Sample> {
    let mut files: Vec<String> = std::fs::read_dir(samples_dir())
        .expect("samples/player-shop")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let readings = samples_dir().join(file.replace(".png", ".json"));
            let text = std::fs::read_to_string(&readings).unwrap_or_else(|e| panic!("{}: {e}", readings.display()));
            let listings: Vec<Listing> = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", readings.display()));
            let written = listings.iter().all(|l| !matches!(l, Listing::Covered { covered: false }));
            assert!(written, "{}: a listing is read, or `\"covered\": true`", readings.display());
            Sample { file, listings }
        })
        .collect()
}

/// The item a listing's name stands for: the name, or for one the game cut short, a
/// made-up name that starts as it does.
fn item_name(listing: &str) -> String {
    match listing.strip_suffix("...").or_else(|| listing.strip_suffix('…')) {
        Some(start) => format!("{start}egs for Made Up"),
        None => listing.to_string(),
    }
}

/// Made-up item data with an item of each listing's name.
fn sample_items(samples: &[Sample]) -> ItemWorld {
    let mut names: Vec<String> = samples
        .iter()
        .flat_map(|s| &s.listings)
        .filter_map(|l| match l {
            Listing::Read { name, .. } => Some(item_name(name)),
            Listing::Covered { .. } => None,
        })
        .collect();
    names.sort();
    names.dedup();
    let mut items = item_world(Vec::new());
    for (n, name) in names.iter().enumerate() {
        items = items.with(etc(4_100_000 + n as u64, name).sells_for(10));
    }
    items
}

/// A core over a 1920x1200 game window at the screen's origin, with the samples' items in
/// and the Deal Finder on.
fn started(samples: &[Sample]) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline().with_items(sample_items(samples)));
    minerva.restore(Settings::default());
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    minerva.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    minerva.set_deal_badges_size(Size { width: 160, height: 0 });
    minerva.tick(0);
    assert!(minerva.on_hotkey("ctrl+shift+d").is_none());
    minerva
}

fn image_of(file: &str) -> RgbaImage {
    image::open(samples_dir().join(file)).unwrap_or_else(|e| panic!("{file}: {e}")).to_rgba8()
}

/// `img` at `factor` times its size, scaled smoothly.
fn scaled(img: &RgbaImage, factor: f32) -> RgbaImage {
    let size = |px: u32| (px as f32 * factor).round() as u32;
    imageops::resize(img, size(img.width()), size(img.height()), FilterType::Triangle)
}

/// The UI Size `img` shows, next to the samples': the widest its listings' bands run on a
/// listing's worth of rows, of the band's colour at any brightness (narrower where something
/// covers them), over the samples'.
fn ui_size(img: &RgbaImage) -> f32 {
    let band = |p: &Rgba<u8>| {
        let k = p[2] as f32 / 223.0;
        (0.8..=1.1).contains(&k) && (p[0] as f32 - 202.0 * k).abs() <= 6.0 && (p[1] as f32 - 213.0 * k).abs() <= 6.0
    };
    let mut runs: HashMap<u32, usize> = HashMap::new();
    for y in 0..img.height() {
        let mut x = 0;
        while x < img.width() {
            let start = x;
            while x < img.width() && band(img.get_pixel(x, y)) {
                x += 1;
            }
            if x - start >= 100 {
                *runs.entry(x - start).or_default() += 1;
            }
            x = x.max(start + 1);
        }
    }
    let width = runs.into_iter().filter(|&(_, rows)| rows >= 10).map(|(width, _)| width).max().expect("a listing's band");
    width as f32 / SAMPLES_BAND_W
}

/// The sample's frame, its colours times `brightness`.
fn frame(sample: &Sample, brightness: f32) -> Frame {
    frame_of(&image_of(&sample.file), brightness)
}

/// `img` as a frame, its colours times `brightness`.
fn frame_of(img: &RgbaImage, brightness: f32) -> Frame {
    let mut rgba = img.as_raw().clone();
    for (i, channel) in rgba.iter_mut().enumerate() {
        if i % 4 != 3 {
            *channel = (*channel as f32 * brightness).round() as u8;
        }
    }
    Frame { width: img.width(), height: img.height(), rgba, at_ms: 1_000 }
}

fn badges(state: &PanelState) -> Vec<DealBadge> {
    state.deal_badges.as_ref().map(|shown| shown.badges.clone()).unwrap_or_default()
}

/// What a badge says the listing is: its item's name and price; `None` for "?".
fn read(badge: &DealBadge) -> Option<(String, ListedPrice)> {
    match &badge.verdict {
        Verdict::Deal { item, listed, .. } | Verdict::NoDeal { item, listed, .. } | Verdict::NoPrice { item, listed } => {
            Some((item.name.clone(), *listed))
        }
        Verdict::Unknown => None,
    }
}

/// What a frame of a sample read as, against its readings.
struct Checked {
    /// Its listings read as unreadable.
    unreadable: usize,
    /// Where each badge of a listing not covered is on screen: its top and its height.
    places: Vec<(i32, u32)>,
}

/// Feed `frame` of `sample` to a core of its own, and check its badges against the sample's
/// readings: what reads wrong goes in `wrong`, labelled `label`; `None` when the badges
/// aren't the readings' listings one for one.
fn check(samples: &[Sample], sample: &Sample, frame: Frame, label: &str, wrong: &mut Vec<String>) -> Option<Checked> {
    let mut minerva = started(samples);
    let state = minerva.on_player_shop_capture(ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 } });
    let window_top = state.deal_badges.as_ref().map_or(0, |shown| shown.placement.y);
    let badges = badges(state);
    let (covered, uncovered): (Vec<&DealBadge>, Vec<&DealBadge>) = badges.iter().partition(|b| b.covered);
    let covers = sample.listings.iter().filter(|l| matches!(l, Listing::Covered { .. })).count();
    let reads: Vec<(&String, u64, u64)> = sample
        .listings
        .iter()
        .filter_map(|l| match l {
            Listing::Read { name, quantity, mesos } => Some((name, *quantity, *mesos)),
            Listing::Covered { .. } => None,
        })
        .collect();
    if uncovered.len() != reads.len() || covered.len() > covers {
        let found = format!("{} listings found and {} covered", uncovered.len(), covered.len());
        wrong.push(format!("{label}: {found}, not {} and up to {covers}", reads.len()));
        return None;
    }
    let mut checked = Checked { unreadable: 0, places: Vec::new() };
    for (badge, (name, quantity, mesos)) in uncovered.into_iter().zip(reads) {
        checked.places.push((window_top + badge.top as i32, badge.height));
        let expected = (item_name(name), ListedPrice { quantity, mesos });
        match read(badge) {
            Some(read) if read == expected => {}
            None => checked.unreadable += 1,
            Some(read) => wrong.push(format!("{label}: {read:?}, not {expected:?}")),
        }
    }
    Some(checked)
}

#[test]
fn every_samples_listings_read_as_its_readings_say_or_as_unreadable_never_wrong() {
    let samples = samples();
    assert!(samples.len() >= 3, "expected the Player Shop samples, found {}", samples.len());
    let mut wrong = Vec::new();
    let mut unreadable = 0;
    for sample in &samples {
        for brightness in [1.0, 0.935] {
            let label = format!("{} ×{brightness}", sample.file);
            if let Some(checked) = check(&samples, sample, frame(sample, brightness), &label, &mut wrong) {
                unreadable += checked.unreadable;
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
    // The samples were all read whole when they were added: a reader that reads less of them
    // has got worse, though it says nothing wrong.
    assert_eq!(unreadable, 0, "listings now unreadable");
}

#[test]
fn every_samples_listings_read_at_other_ui_sizes_as_its_readings_say_or_as_unreadable_never_wrong() {
    let samples = samples();
    let mut wrong = Vec::new();
    let mut misplaced = Vec::new();
    for sample in &samples {
        let img = image_of(&sample.file);
        let own = ui_size(&img);
        let at_own = check(&samples, sample, frame_of(&img, 1.0), &sample.file, &mut wrong);
        for size in OTHER_UI_SIZES {
            let factor = size / own;
            if (factor - 1.0).abs() < 0.05 {
                continue;
            }
            let resized = scaled(&img, factor);
            for brightness in [1.0, 0.935] {
                let label = format!("{} at ×{size} ×{brightness}", sample.file);
                let Some(checked) = check(&samples, sample, frame_of(&resized, brightness), &label, &mut wrong) else {
                    continue;
                };
                let Some(at_own) = &at_own else { continue };
                for (&(top, height), &(own_top, own_height)) in checked.places.iter().zip(&at_own.places) {
                    let expected = ((own_top as f32 * factor).round() as i32, (own_height as f32 * factor).round() as u32);
                    if top.abs_diff(expected.0) > 2 || height.abs_diff(expected.1) > 2 {
                        misplaced.push(format!("{label}: a badge at {top}, {height} px tall, not {}, {}", expected.0, expected.1));
                    }
                }
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
    assert!(misplaced.is_empty(), "badges not level with their listings:\n{}", misplaced.join("\n"));
}

/// Bands narrower or wider than at any UI Size the reader takes (ADR 0020), and a stack of the
/// band's colour with no listing on it, are no shop.
#[test]
fn bands_too_narrow_or_too_wide_for_any_ui_size_or_with_no_listing_show_no_badges() {
    let samples = samples();
    for sample in &samples {
        let img = image_of(&sample.file);
        let own = ui_size(&img);
        for size in [0.5, 2.0] {
            let mut minerva = started(&samples);
            let frame = frame_of(&scaled(&img, size / own), 1.0);
            let state = minerva.on_player_shop_capture(ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 } });
            assert!(badges(state).is_empty(), "{} at ×{size}", sample.file);
        }
    }
    let mut client = RgbaImage::from_pixel(1920, 1200, Rgba([96, 150, 210, 255]));
    imageops::replace(&mut client, &RgbaImage::from_pixel(300, 400, Rgba([202, 213, 223, 255])), 800, 400);
    let mut minerva = started(&samples);
    let state = minerva.on_player_shop_capture(ShopCapture { frame: frame_of(&client, 1.0), origin: ScreenPoint { x: 0, y: 0 } });
    assert!(badges(state).is_empty(), "a plain stack of the band's colour");
}

#[test]
fn listings_read_before_the_pointer_and_its_tooltip_came_over_them_keep_their_badges_covered() {
    for size in [1.0, 1.5] {
        listings_read_before_the_pointer_and_its_tooltip_came_over_them_keep_their_badges_covered_at(size);
    }
}

fn listings_read_before_the_pointer_and_its_tooltip_came_over_them_keep_their_badges_covered_at(size: f32) {
    let samples = samples();
    let mut minerva = started(&samples);
    let capture =
        |file: &str| ShopCapture { frame: frame_of(&scaled(&image_of(file), size), 1.0), origin: ScreenPoint { x: 0, y: 0 } };
    let shown = |state: &PanelState| -> Vec<(bool, Option<String>)> {
        badges(state).iter().map(|b| (b.covered, read(b).map(|(name, _)| name))).collect()
    };
    let name = |name: &str| Some(name.to_string());

    minerva.on_player_shop_capture(capture("equips-two-sold-out.png"));
    let state = minerva.on_player_shop_capture(capture("equips-under-the-pointer-and-its-tooltip.png"));

    assert_eq!(
        shown(state),
        [
            (false, name("Red Guise")),
            (true, name("Green Knucklevest")),
            (true, name("Black Knucklevest")),
            (true, name("Black Amoria Skirt")),
        ],
        "none for the sold-out listings, and none between the listings, at ×{size}"
    );
}

/// A live capture of the game's 1920x1200 client area holding `file`, a Bug Report's crop of a
/// shop's list, back where it was captured: its top-left corner at `at`, the rest the game's
/// sky; all of it `size` times as large.
fn placed(file: &str, at: (u32, u32), at_ms: u64, size: f32) -> ShopCapture {
    let mut client = RgbaImage::from_pixel(1920, 1200, Rgba([96, 150, 210, 255]));
    imageops::replace(&mut client, &image_of(file), at.0 as i64, at.1 as i64);
    let client = if size == 1.0 { client } else { scaled(&client, size) };
    let frame = Frame { width: client.width(), height: client.height(), rgba: client.into_raw(), at_ms };
    ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 } }
}

/// A Bug Report's run, as the player saw it: one shop, then another opened a few px higher
/// where it was, then that one scrolled down to five sold-out listings and Brown Guise. Live
/// captures show a sold-out listing near white, its rules white: none of it covers anything.
#[test]
fn sold_out_listings_in_live_captures_get_no_badge_from_one_shop_to_the_next() {
    for size in [1.0, 1.5] {
        sold_out_listings_in_live_captures_get_no_badge_from_one_shop_to_the_next_at(size);
    }
}

fn sold_out_listings_in_live_captures_get_no_badge_from_one_shop_to_the_next_at(size: f32) {
    let samples = samples();
    let mut minerva = started(&samples);
    let shown = |state: &PanelState| -> Vec<(bool, Option<String>)> {
        badges(state).iter().map(|b| (b.covered, read(b).map(|(name, _)| name))).collect()
    };
    let name = |name: &str| (false, Some(name.to_string()));

    let state = minerva.on_player_shop_capture(placed("live-ores-four-sold-out.png", (1343, 680), 18_605, size));
    assert_eq!(shown(state), [name("Silver Ore"), name("Emerald Ore")]);

    let state = minerva.on_player_shop_capture(placed("live-hats-two-sold-out.png", (1343, 643), 22_011, size));
    let sailor = item_name("White Ribboned Sailor Dr...");
    assert_eq!(
        shown(state),
        [name("Blue Cap"), name("Bunny Earmuffs"), name("Bunny Earmuffs"), (false, Some(sailor))],
        "none on the last shop's slots, which lie between this one's"
    );

    let state = minerva.on_player_shop_capture(placed("live-hats-scrolled-five-sold-out.png", (1343, 696), 25_481, size));
    assert_eq!(shown(state), [name("Brown Guise")], "none on the sold-out listings, nor what was there before");
}
