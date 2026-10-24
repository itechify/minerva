//! Readings that come back from the shell's reader thread after the region they were captured
//! for is no longer captured (the game window moved, the Deal Finder was turned off), and EXP
//! figures read after the Grind Tracker was turned off: the core leaves them, so nothing they
//! read is put where it isn't.

mod common;

use common::{FakeSource, S, set_up, window_at_origin, window_on_wide_screen};
use minerva_core::{
    Action, ExpBar, ExpBarReading, Foreground, Frame, ListedPrice, Minerva, PanelState, STATUS_BAR_STRIP, ScreenPoint, ScreenRect,
    ShopCapture, ShopList, ShopReading, ShopRow, StatusBar, StatusBarCapture, StatusBarReading, TitleCapture, TitleLines,
    TitleReading,
};

const SUNSET_SKY: (&str, &str) = ("Victoria Road", "Sunset Sky");
const KERNING_CITY: (&str, &str) = ("Victoria Road", "Kerning City");

/// The game window at the screen's origin, in front, with setup done.
fn playing() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(set_up());
    minerva.on_game_window(Some(window_at_origin(1920, 1200)));
    minerva.on_foreground(Foreground::Game);
    minerva
}

/// The game window moved to (812,110): what was captured before is no longer.
fn move_the_game(minerva: &mut Minerva) {
    minerva.on_game_window(Some(window_on_wide_screen(common::GAME)));
}

/// A blank frame of `region`, captured at `at_ms`.
fn blank(region: ScreenRect, at_ms: u64) -> Frame {
    Frame { width: region.width, height: region.height, rgba: vec![0; (region.width * region.height * 4) as usize], at_ms }
}

fn title_capture(region: ScreenRect, at_ms: u64) -> TitleCapture {
    TitleCapture { frame: blank(region, at_ms), origin: ScreenPoint { x: region.x, y: region.y }, ours: Vec::new() }
}

/// A capture of `region` the title was read in as `(street, map)`, found where these tests
/// find it on the client area `client`.
fn read_title(minerva: &mut Minerva, client: ScreenRect, region: ScreenRect, (street, map): (&str, &str), at_ms: u64) {
    let lines = Some(TitleLines { street: street.into(), map: map.into() });
    minerva.on_title_read(title_capture(region, at_ms), TitleReading { lines, found: Some(common::title_on(client)), at_ms });
}

fn shown_map(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|map| map.map_name.as_str())
}

#[test]
fn a_title_read_in_a_region_no_longer_captured_leaves_the_map_shown() {
    let mut minerva = playing();
    let client = window_at_origin(1920, 1200).client;
    for at_ms in [S, 2 * S] {
        let region = minerva.panel_state().title_capture.expect("a title capture with the game in front");
        read_title(&mut minerva, client, region, SUNSET_SKY, at_ms);
    }
    assert_eq!(shown_map(minerva.panel_state()), Some("Sunset Sky"));

    // Kerning City, read in captures taken before the game window moved.
    let before = minerva.panel_state().title_capture.expect("where the title was found");
    move_the_game(&mut minerva);
    for at_ms in [3 * S, 4 * S] {
        read_title(&mut minerva, client, before, KERNING_CITY, at_ms);
    }
    assert_eq!(shown_map(minerva.panel_state()), Some("Sunset Sky"));

    // Read where the core asks now, it is shown.
    for at_ms in [5 * S, 6 * S] {
        let region = minerva.panel_state().title_capture.expect("a title capture in the moved client area");
        read_title(&mut minerva, common::GAME, region, KERNING_CITY, at_ms);
    }
    assert_eq!(shown_map(minerva.panel_state()), Some("Kerning City"));
}

/// A Player Shop listing a Bronze Ore, read in a capture of `region`.
fn read_shop(minerva: &mut Minerva, region: ScreenRect, at_ms: u64) -> PanelState {
    let row = ShopRow {
        area: ScreenRect { x: region.x + 619, y: region.y + 513, width: 253, height: 54 },
        name: Some("Bronze Ore".into()),
        price: Some(ListedPrice { quantity: 1, mesos: 999 }),
        covered: false,
        tier: None,
    };
    let shop = ShopList { area: ScreenRect { x: region.x + 600, y: region.y + 500, width: 300, height: 300 }, rows: vec![row] };
    let capture = ShopCapture { frame: blank(region, at_ms), origin: ScreenPoint { x: region.x, y: region.y }, pointer: None };
    minerva.on_player_shop_read(capture, ShopReading { shop: Some(shop), hover: None, at_ms, near_miss: None }).clone()
}

#[test]
fn a_shop_read_in_a_region_no_longer_captured_finds_no_shop() {
    let mut minerva = playing();
    minerva.act(Action::ToggleDealFinder);
    let before = minerva.panel_state().deal_finder_capture.expect("the client area, with the Deal Finder on");

    // The game window moved while the capture was read.
    move_the_game(&mut minerva);
    let state = read_shop(&mut minerva, before, S);
    assert!(!state.deal_finder.shop_found);
    assert_eq!(state.deal_badges, None);

    // Nor once the Deal Finder is off.
    let now = state.deal_finder_capture.expect("the moved client area");
    minerva.act(Action::ToggleDealFinder);
    assert!(!read_shop(&mut minerva, now, 2 * S).deal_finder.shop_found);

    // Read where the core asks, it is found.
    minerva.act(Action::ToggleDealFinder);
    assert!(read_shop(&mut minerva, now, 3 * S).deal_finder.shop_found);
}

/// The bottom of the client area `client`, where the core has the Status Bar captured.
fn strip_of(client: ScreenRect) -> ScreenRect {
    ScreenRect { y: client.y + (client.height - STATUS_BAR_STRIP) as i32, height: STATUS_BAR_STRIP, ..client }
}

/// A capture of `region` the Status Bar was read in as a level 13 Magician named `name`, with
/// its EXP figures read as `exp`, if they were.
fn read_status_bar(minerva: &mut Minerva, region: ScreenRect, name: &str, exp: Option<ExpBar>, at_ms: u64) -> PanelState {
    let area = ScreenRect { x: region.x + 334, y: region.y + 73, width: 331, height: 55 };
    let bar = StatusBar { area, level: Some(13), job: Some("Magician".into()), name: Some(name.into()), exp_figures: None };
    let capture = StatusBarCapture { frame: blank(region, at_ms), origin: ScreenPoint { x: region.x, y: region.y } };
    let exp = exp.map(|bar| ExpBarReading { bar: Some(bar), at_ms, place: None });
    minerva.on_status_bar_read(capture, StatusBarReading { bar: Some(bar), at_ms, near_miss: None }, exp).clone()
}

fn character(state: &PanelState) -> Option<&str> {
    state.character.as_ref().map(|character| character.name.as_str())
}

#[test]
fn a_status_bar_read_in_a_region_no_longer_captured_names_no_character() {
    let mut minerva = playing();
    let before = minerva.panel_state().status_bar_capture.expect("the bottom of the client area");
    assert_eq!(before, strip_of(window_at_origin(1920, 1200).client));

    // Egbert, read in a capture taken before the game window moved.
    move_the_game(&mut minerva);
    let state = read_status_bar(&mut minerva, before, "Egbert", None, S);
    assert_eq!(state.status_bar, None);
    assert_eq!(character(&state), None);

    // Read where the core asks now, Egbert is playing.
    let now = state.status_bar_capture.expect("the bottom of the moved client area");
    assert_eq!(now, strip_of(common::GAME));
    assert_eq!(character(&read_status_bar(&mut minerva, now, "Egbert", None, 2 * S)), Some("Egbert"));
}

#[test]
fn exp_figures_read_after_the_grind_tracker_was_turned_off_leave_the_exp_tracking() {
    let mut minerva = playing();
    let strip = minerva.panel_state().status_bar_capture.expect("the bottom of the client area");
    let exp = |exp: u64| Some(ExpBar { exp, percent_hundredths: common::percent(exp, 20_000) });
    let mut state = PanelState::default();
    for second in 1..=5 {
        state = read_status_bar(&mut minerva, strip, "Egbert", exp(1000 + 25 * second), second * S);
    }
    assert!(state.exp_figures.is_some(), "asked for while the Grind Tracker is on");
    let before = state.exp_tracking.clone();
    assert_ne!(read_status_bar(&mut minerva, strip, "Egbert", exp(1300), 6 * S).exp_tracking, before, "taken in while on");

    minerva.act(Action::SetGrindTracker { on: false });
    let before = minerva.panel_state().exp_tracking.clone();
    assert_eq!(minerva.panel_state().exp_figures, None);
    let state = read_status_bar(&mut minerva, strip, "Egbert", exp(1500), 7 * S);
    assert_eq!(state.exp_tracking, before);
}
