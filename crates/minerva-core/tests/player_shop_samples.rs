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
//!   A listing whose name the game cut short so that it starts several items' names also
//!   says `"named"`: the item it is, as the Item Tooltip hovered over it names it. Its badge
//!   is "?" until it is hovered, unless its picture tells it.
//! - A scroll's listing whose name the game cut short says its `"tier"` too, as its picture
//!   shows it (#140), and only such a listing (a whole name names itself). In the made-up
//!   world every scroll whose name the game cut short starts four scrolls' names, one of each
//!   tier: only its picture tells which it is, so its badge names it unhovered.
//! - A hovered sample, `<name>@<x>,<y>.png`, is a frame of `<name>.png`'s shop with the
//!   pointer at (x, y) on a listing and the game's Item Tooltip by it, cut as far right as
//!   the tooltip's name line and past its edge, the shop's chat window painted out but for
//!   that line, and the chat showing through the tooltip painted its fill. It is fed after
//!   `<name>.png`, as the player sees a shop and then hovers a listing: its `"named"`
//!   listing's badge is that item's, or "?", whether the tooltip covers it or not.
//! - Each frame is fed as the game's client area at the screen's origin, as it is and as
//!   the darker colours of live captures (`prep.rs`), to a core of its own.
//! - Each is fed again at the other UI Sizes in [`OTHER_UI_SIZES`], scaled smoothly as the game
//!   draws its UI larger or smaller (ADR 0020): there a listing may read as unreadable, never as
//!   another item or price, and its badge sits level with it.

mod common;

use common::FakeSource;
use common::items::{ItemWorld, etc, item_world, scroll};
use minerva_core::{
    DealBadge, Frame, ListedPrice, Minerva, PanelState, ScreenPoint, Settings, ShopCapture, Size,
    Verdict,
};
use image::imageops::{self};
use image::{Rgba, RgbaImage};
use serde::Deserialize;
use std::collections::HashMap;

/// A listing's band is this wide at the UI Size most samples were taken at.
const SAMPLES_BAND_W: f32 = 253.0;
/// The UI Sizes every sample is also read at, next to the samples': #130's (a band 162 px
/// wide), and two larger.
const OTHER_UI_SIZES: [f32; 3] = [0.64, 1.5, 1.8];

/// One listing as a sample's readings write it: read, or covered; and the item an Item
/// Tooltip names it as, for one whose name the game cut short so that it could be several.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Listing {
    Read {
        name: String,
        quantity: u64,
        mesos: u64,
        #[serde(default)]
        tier: Option<String>,
        #[serde(default)]
        named: Option<String>,
    },
    Covered {
        covered: bool,
        #[serde(default)]
        named: Option<String>,
    },
}

impl Listing {
    fn named(&self) -> Option<&str> {
        match self {
            Listing::Read { named, .. } | Listing::Covered { named, .. } => named.as_deref(),
        }
    }
}

struct Sample {
    file: String,
    listings: Vec<Listing>,
    /// A hovered sample's pointer, in the frame.
    pointer: Option<ScreenPoint>,
    /// The sample of the same shop a hovered sample is fed after.
    partner: Option<String>,
}

fn samples() -> Vec<Sample> {
    let mut files: Vec<String> = std::fs::read_dir(common::samples_in("player-shop"))
        .expect("samples/player-shop")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".png"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let readings = common::samples_in("player-shop").join(file.replace(".png", ".json"));
            let text = std::fs::read_to_string(&readings).unwrap_or_else(|e| panic!("{}: {e}", readings.display()));
            let listings: Vec<Listing> = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", readings.display()));
            let written = listings.iter().all(|l| !matches!(l, Listing::Covered { covered: false, .. }));
            assert!(written, "{}: a listing is read, or `\"covered\": true`", readings.display());
            let tiers_cut = listings.iter().all(|l| !matches!(l, Listing::Read { name, tier: Some(_), .. } if cut_start(name).is_none()));
            assert!(tiers_cut, "{}: a `\"tier\"` only beside a name the game cut short", readings.display());
            let (pointer, partner) = match file.trim_end_matches(".png").split_once('@') {
                Some((name, at)) => {
                    let (x, y) = at.split_once(',').unwrap_or_else(|| panic!("{file}: <name>@<x>,<y>"));
                    let pointer = ScreenPoint { x: x.parse().expect("x"), y: y.parse().expect("y") };
                    (Some(pointer), Some(format!("{name}.png")))
                }
                None => (None, None),
            };
            Sample { file, listings, pointer, partner }
        })
        .collect()
}

/// The scroll tiers, as the readings write them.
const TIERS: [&str; 4] = ["Lesser", "Intermediate", "Greater", "Chaos"];

/// Where a listing's name the game cut short starts; `None` for a whole one.
fn cut_start(listing: &str) -> Option<&str> {
    listing.strip_suffix("...").or_else(|| listing.strip_suffix('…'))
}

/// The item a listing's name stands for: the name, or for one the game cut short, a
/// made-up name that starts as it does.
fn item_name(listing: &str) -> String {
    match cut_start(listing) {
        Some(start) => format!("{start}egs for Made Up"),
        None => listing.to_string(),
    }
}

/// The made-up scrolls each scroll listing whose name the game cut short could be, by where
/// its name starts: one of each tier (in [`TIERS`]' order), the item a listing of that tier
/// is `"named"` where one is.
fn families(samples: &[Sample]) -> HashMap<String, [String; 4]> {
    let mut named: HashMap<String, [Option<String>; 4]> = HashMap::new();
    for listing in samples.iter().flat_map(|s| &s.listings) {
        if let Listing::Read { name, tier: Some(tier), named: of, .. } = listing
            && let Some(start) = cut_start(name)
        {
            let family = named.entry(start.to_string()).or_default();
            if let Some(of) = of {
                family[tier_at(tier)] = Some(of.clone());
            }
        }
    }
    named
        .into_iter()
        .map(|(start, family)| {
            let scroll = |at: usize| family[at].clone().unwrap_or_else(|| format!("{start}egs for Made Up, {}", TIERS[at]));
            let family = std::array::from_fn(scroll);
            (start, family)
        })
        .collect()
}

/// Where `tier` is in [`TIERS`].
fn tier_at(tier: &str) -> usize {
    TIERS.iter().position(|&t| t == tier).unwrap_or_else(|| panic!("a tier: {tier:?}"))
}

/// The item a listing read as `name` names, unhovered: a scroll whose name the game cut short,
/// the one of its picture's tier; any other, its own, unless it could be several (`named`).
fn expected(families: &HashMap<String, [String; 4]>, name: &str, tier: Option<&str>, named: Option<&str>) -> Option<String> {
    match cut_start(name).and_then(|start| families.get(start)).zip(tier) {
        Some((family, tier)) => Some(family[tier_at(tier)].clone()),
        None => named.is_none().then(|| item_name(name)),
    }
}

/// The made-up world the samples are read in: its items, and the scrolls each scroll listing
/// whose name the game cut short could be ([`families`]; none in a world where no item is a
/// scroll).
struct World {
    items: ItemWorld,
    families: HashMap<String, [String; 4]>,
}

/// Made-up item data with an item of each listing's name, and the item each is named as: a
/// listing named so starts both items' names. With `scrolls`, a scroll's listing whose name the
/// game cut short starts the names of the four scrolls of its family instead ([`families`]), so
/// its picture names it; without, only its tooltip can.
fn world(samples: &[Sample], scrolls: bool) -> World {
    let families = if scrolls { families(samples) } else { HashMap::new() };
    let in_family: Vec<&String> = families.values().flatten().collect();
    let mut names: Vec<String> = samples
        .iter()
        .flat_map(|s| &s.listings)
        .flat_map(|l| {
            let read = match l {
                Listing::Read { name, .. } if cut_start(name).is_some_and(|start| families.contains_key(start)) => None,
                Listing::Read { name, .. } => Some(item_name(name)),
                Listing::Covered { .. } => None,
            };
            read.into_iter().chain(l.named().map(String::from))
        })
        .filter(|name| !in_family.contains(&name))
        .collect();
    names.sort();
    names.dedup();
    let mut items = item_world(Vec::new());
    for (n, name) in names.iter().enumerate() {
        items = items.with(etc(4_100_000 + n as u64, name).sells_for(10));
    }
    let mut starts: Vec<&String> = families.keys().collect();
    starts.sort();
    for (n, start) in starts.into_iter().enumerate() {
        for (at, name) in families[start].iter().enumerate() {
            items = items.with_scroll(scroll(2_040_000 + 4 * n as u64 + at as u64, name).of_tier(TIERS[at]));
        }
    }
    World { items, families }
}

/// A core over a 1920x1200 game window at the screen's origin, with `world`'s items in and the
/// Deal Finder on.
fn started(world: &World) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline().with_items(world.items.clone()));
    minerva.restore(Settings::default());
    minerva.on_game_window(Some(common::window_at_origin(1920, 1200)));
    minerva.set_deal_badges_size(Size { width: 160, height: 0 });
    minerva.tick(0);
    assert!(minerva.on_hotkey("ctrl+shift+d").is_none());
    minerva
}

fn image_of(file: &str) -> RgbaImage {
    image::open(common::samples_in("player-shop").join(file)).unwrap_or_else(|e| panic!("{file}: {e}")).to_rgba8()
}

/// The UI Size `img` shows, next to the samples': the widest its listings' bands run on a
/// listing's worth of rows, of the band's colour at any brightness (narrower where something
/// covers them), over the samples'.
fn sample_ui_size(img: &RgbaImage) -> f32 {
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

/// The captures `sample` is fed as, each image made a frame by `frame` at `factor` times its
/// size: a hovered sample's partner's first, then its own with the pointer on it.
fn captures(sample: &Sample, factor: f32, frame: impl Fn(&RgbaImage) -> Frame) -> Vec<ShopCapture> {
    let at = |v: i32| (v as f32 * factor).round() as i32;
    let pointer = sample.pointer.map(|p| ScreenPoint { x: at(p.x), y: at(p.y) });
    let capture = |file: &str, pointer| ShopCapture { frame: frame(&image_of(file)), origin: ScreenPoint { x: 0, y: 0 }, pointer };
    sample.partner.iter().map(|partner| capture(partner, None)).chain([capture(&sample.file, pointer)]).collect()
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
    /// Of them, the hovered one.
    hovered_unreadable: usize,
    /// Where each badge of a listing not covered is on screen: its top and its height.
    places: Vec<(i32, u32)>,
}

/// Feed `captures` of `sample` to a core of its own, and check its badges after against the
/// sample's readings: what reads wrong goes in `wrong`, labelled `label`; `None` when the
/// badges aren't the readings' listings one for one.
fn check(world: &World, sample: &Sample, captures: Vec<ShopCapture>, label: &str, wrong: &mut Vec<String>) -> Option<Checked> {
    let mut minerva = started(world);
    let pointer = captures.last().and_then(|capture| capture.pointer);
    let mut state = None;
    for capture in captures {
        state = Some(minerva.on_player_shop_capture(capture).clone());
    }
    let state = state.expect("a capture");
    let window_top = state.deal_badges.as_ref().map_or(0, |shown| shown.placement.y);
    let badges = badges(&state);
    let mut checked = Checked { unreadable: 0, hovered_unreadable: 0, places: Vec::new() };
    // A hovered listing reads covered or not, as far as the tooltip hides it: it is set
    // aside, and its badge is the one level with the pointer.
    let hovered = sample.listings.iter().position(|l| l.named().is_some()).zip(pointer);
    let (listings, badges): (Vec<&Listing>, Vec<&DealBadge>) = match hovered {
        Some((at, pointer)) => {
            let named = sample.listings[at].named().expect("named");
            let level = |b: &&DealBadge| (0..b.height as i32).contains(&(pointer.y - window_top - b.top as i32));
            match badges.iter().find(level).and_then(read) {
                Some((read, _)) if read == named => {}
                None => {
                    checked.unreadable += 1;
                    checked.hovered_unreadable += 1;
                }
                Some(read) => wrong.push(format!("{label}: {read:?}, not {named:?}")),
            }
            let listings = sample.listings.iter().enumerate().filter(|&(i, _)| i != at).map(|(_, l)| l).collect();
            (listings, badges.iter().filter(|b| !level(b)).collect())
        }
        None => (sample.listings.iter().collect(), badges.iter().collect()),
    };
    let (covered, uncovered): (Vec<&DealBadge>, Vec<&DealBadge>) = badges.into_iter().partition(|b| b.covered);
    let covers = listings.iter().filter(|l| matches!(l, Listing::Covered { .. })).count();
    let reads: Vec<(Option<String>, ListedPrice)> = listings
        .iter()
        .filter_map(|l| match l {
            Listing::Read { name, quantity, mesos, tier, .. } => {
                let price = ListedPrice { quantity: *quantity, mesos: *mesos };
                Some((expected(&world.families, name, tier.as_deref(), l.named()), price))
            }
            Listing::Covered { .. } => None,
        })
        .collect();
    if uncovered.len() != reads.len() || covered.len() > covers {
        let found = format!("{} listings found and {} covered", uncovered.len(), covered.len());
        wrong.push(format!("{label}: {found}, not {} and up to {covers}", reads.len()));
        return None;
    }
    for (badge, (named, price)) in uncovered.into_iter().zip(reads) {
        checked.places.push((window_top + badge.top as i32, badge.height));
        // Not hovered, a name that starts several items' is "?", unless its picture tells it.
        let expected = named.map(|named| (named, price));
        match (read(badge), expected) {
            (Some(read), Some(expected)) if read == expected => {}
            (None, Some(_)) => checked.unreadable += 1,
            (None, None) => {}
            (Some(read), expected) => wrong.push(format!("{label}: {read:?}, not {expected:?}")),
        }
    }
    Some(checked)
}

#[test]
fn every_samples_listings_read_as_its_readings_say_or_as_unreadable_never_wrong() {
    let samples = samples();
    assert!(samples.len() >= 3, "expected the Player Shop samples, found {}", samples.len());
    let world = world(&samples, true);
    let mut wrong = Vec::new();
    let mut unreadable = 0;
    for sample in &samples {
        for brightness in [1.0, 0.935] {
            let label = format!("{} ×{brightness}", sample.file);
            let captures = captures(sample, 1.0, |img| common::dimmed(img, brightness));
            if let Some(checked) = check(&world, sample, captures, &label, &mut wrong) {
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
    let world = world(&samples, true);
    let mut wrong = Vec::new();
    let mut misplaced = Vec::new();
    for sample in &samples {
        let img = image_of(&sample.file);
        let own = sample_ui_size(&img);
        let at_own = check(&world, sample, captures(sample, 1.0, |img| common::dimmed(img, 1.0)), &sample.file, &mut wrong);
        for size in OTHER_UI_SIZES {
            let factor = size / own;
            if (factor - 1.0).abs() < 0.05 {
                continue;
            }
            for brightness in [1.0, 0.935] {
                let label = format!("{} at ×{size} ×{brightness}", sample.file);
                let captures = captures(sample, factor, |img| common::dimmed(&common::scaled(img, factor), brightness));
                let Some(checked) = check(&world, sample, captures, &label, &mut wrong) else {
                    continue;
                };
                let Some(at_own) = &at_own else { continue };
                // Within 2 px, and a px at the sample's own size: read at another UI Size than
                // the samples' itself, its own badges are placed to a px there.
                let slack = 2 + if (own - 1.0).abs() < 0.02 { 0 } else { factor.ceil() as u32 };
                for (&(top, height), &(own_top, own_height)) in checked.places.iter().zip(&at_own.places) {
                    let expected = ((own_top as f32 * factor).round() as i32, (own_height as f32 * factor).round() as u32);
                    if top.abs_diff(expected.0) > slack || height.abs_diff(expected.1) > slack {
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
    let world = world(&samples, true);
    for sample in &samples {
        let img = image_of(&sample.file);
        let own = sample_ui_size(&img);
        for size in [0.5, 2.0] {
            let mut minerva = started(&world);
            let frame = common::dimmed(&common::scaled(&img, size / own), 1.0);
            let state = minerva.on_player_shop_capture(ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 }, pointer: None });
            assert!(badges(state).is_empty(), "{} at ×{size}", sample.file);
        }
    }
    let mut client = RgbaImage::from_pixel(1920, 1200, Rgba([96, 150, 210, 255]));
    imageops::replace(&mut client, &RgbaImage::from_pixel(300, 400, Rgba([202, 213, 223, 255])), 800, 400);
    let mut minerva = started(&world);
    let state = minerva.on_player_shop_capture(ShopCapture { frame: common::dimmed(&client, 1.0), origin: ScreenPoint { x: 0, y: 0 }, pointer: None });
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
    let mut minerva = started(&world(&samples, true));
    let capture =
        |file: &str| ShopCapture { frame: common::dimmed(&common::scaled(&image_of(file), size), 1.0), origin: ScreenPoint { x: 0, y: 0 }, pointer: None };
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
    let client = if size == 1.0 { client } else { common::scaled(&client, size) };
    let frame = Frame { width: client.width(), height: client.height(), rgba: client.into_raw(), at_ms };
    ShopCapture { frame, origin: ScreenPoint { x: 0, y: 0 }, pointer: None }
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
    let mut minerva = started(&world(&samples, true));
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

/// Another of the game's windows drawn in the band's colour, with more rows of it than the
/// shop's list, doesn't hide the shop.
#[test]
fn a_larger_stack_of_the_bands_colour_elsewhere_doesnt_hide_the_shop() {
    let samples = samples();
    let mut minerva = started(&world(&samples, true));
    let mut client = RgbaImage::from_pixel(1920, 1200, Rgba([96, 150, 210, 255]));
    imageops::replace(&mut client, &RgbaImage::from_pixel(300, 900, Rgba([202, 213, 223, 255])), 100, 100);
    imageops::replace(&mut client, &image_of("gems-and-ores.png"), 1343, 643);

    let state = minerva.on_player_shop_capture(ShopCapture { frame: common::dimmed(&client, 1.0), origin: ScreenPoint { x: 0, y: 0 }, pointer: None });

    assert_eq!(badges(state).iter().filter_map(read).count(), 5, "the shop's five listings");
}

/// Hovered, a listing whose name the game cut short is named by the Item Tooltip by the pointer
/// alone where its picture can't name it (#135): in a world where no item is a scroll.
#[test]
fn hovered_listings_are_named_by_their_tooltip_where_their_picture_cant() {
    let samples = samples();
    let world = world(&samples, false);
    let mut wrong = Vec::new();
    let mut unnamed = Vec::new();
    for sample in samples.iter().filter(|sample| sample.pointer.is_some()) {
        for brightness in [1.0, 0.935] {
            let label = format!("{} ×{brightness}", sample.file);
            let captures = captures(sample, 1.0, |img| common::dimmed(img, brightness));
            match check(&world, sample, captures, &label, &mut wrong) {
                Some(checked) if checked.hovered_unreadable == 0 => {}
                _ => unnamed.push(label),
            }
        }
    }
    assert!(wrong.is_empty(), "wrong readings:\n{}", wrong.join("\n"));
    assert!(unnamed.is_empty(), "hovered listings unnamed:\n{}", unnamed.join("\n"));
}
