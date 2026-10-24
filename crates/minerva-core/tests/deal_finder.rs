//! The Deal Finder (ADR 0014): while the player has it on (Ctrl+Shift+D), the shell captures
//! the game's client area a few times a second, the Player Shop's listings are read, and a
//! Deal Badge beside each says whether its price is a deal against the Deal Line, the Market
//! Line the player chose on the Item Card.
//!
//! The readings here stand for what the Player Shop reader made of a capture. OSMS's item
//! files are made up in their shapes ([`common::items`]).
//!
//! Worked example: the game's client area at (812,110), 1920x1200 physical px, on one
//! 3440x1440 monitor; a shop's item list (its picture column) at (1373,600), 311 px wide, so
//! its window from x 1365 to 2182; Deal Badges' window 160 px wide, so at x 1365 - 6 - 160 =
//! 1199. Its listings' bands are 54 px tall, 62 or 63 px apart.

mod common;

use common::{FakeSource, GAME, SCREEN};
use common::items::{ItemWorld, equip, etc, item_world, scroll, use_item};
use minerva_core::{
    Action, DealBadge, DealBadges, DealFinderView, DealGrade, DealItem, Edge, Foreground, GameWindow, Hotkey, Hotkeys, ItemCardView, ListedPrice,
    MarketPrice, Minerva, PanelControl, PanelSizes, PanelState, Request, SavedMarketLine, SavedMarketPrices, ScreenRect, ShopHover, ShopList, ShopReading, ShopRow, Size, Verdict, ItemTooltip, ScreenPoint,
};
use std::task::Poll;

/// The shop's item list, from its first listing to its sixth.
const LIST: ScreenRect = ScreenRect { x: 1373, y: 600, width: 311, height: 367 };
const BADGES: Size = Size { width: 160, height: 0 };
const CARD: Size = Size { width: 300, height: 160 };

const GARNET: &str = "04021000";
const GARNET_ORE: &str = "04020000";
const BRONZE_ORE: &str = "04010000";
const EMERALD_STAFF: &str = "01382004";
const WOODEN_TOP: &str = "02070000";

/// Made-up item data: ores and gems, a staff, two Blue Moons that don't look alike, and two
/// Pet Equip Speed Scrolls.
fn items() -> ItemWorld {
    item_world(vec![
        etc(4_021_000, "Garnet").sells_for(500),
        etc(4_020_000, "Garnet Ore").sells_for(250),
        etc(4_010_000, "Bronze Ore").sells_for(100),
        equip(1_382_004, "Emerald Staff", "Weapon").sells_for(2_000),
        equip(1_041_078, "Blue Moon", "Top").sells_for(11_000),
        equip(1_032_011, "Blue Moon", "Earring").sells_for(10_500),
        use_item(2_070_000, "Wooden Top").recharged_at(1.0),
    ])
    .with_scroll(scroll(2_048_000, "Pet Equip Speed Scroll: Legs for Made Up"))
    .with_scroll(scroll(2_048_001, "Pet Equip Speed Scroll: Lungs for Made Up"))
    .with_scroll(scroll(2_048_010, "Pet Equip Jump Scroll: Head for Made Up"))
}

/// The market prices: "Your price" and "Lowest ask", with a few prices on each.
fn market() -> SavedMarketPrices {
    let line = |id: &str, name: &str, prices: &[(&str, u64)]| SavedMarketLine {
        id: id.into(),
        name: name.into(),
        prices: prices.iter().map(|&(item, mesos)| (item.to_string(), MarketPrice { mesos, set_at_unix_ms: None })).collect(),
    };
    SavedMarketPrices {
        lines: vec![
            line(
                "1",
                "Your price",
                &[(GARNET, 1_470), (GARNET_ORE, 900), (EMERALD_STAFF, 3_800), ("02048000", 15_000), (WOODEN_TOP, 188_888)],
            ),
            line("2", "Lowest ask", &[(GARNET, 900), (GARNET_ORE, 700)]),
        ],
        ..SavedMarketPrices::default()
    }
}

/// A core over the game window, its windows' sizes known, the item data in and the market
/// prices restored.
fn started() -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms_offline().with_items(items()));
    minerva.restore(common::set_up());
    minerva.restore_market_prices(market());
    minerva.set_panel_sizes(PanelSizes { expanded: Size { width: 340, height: 72 }, collapsed: Size { width: 44, height: 44 } });
    minerva.set_item_card_size(CARD);
    minerva.set_deal_badges_size(BADGES);
    minerva.on_game_window(Some(common::window_on_wide_screen(GAME)));
    minerva.on_foreground(Foreground::Game);
    minerva.tick(0);
    minerva
}

/// The core with the Deal Finder turned on.
fn on() -> Minerva {
    let mut minerva = started();
    assert_eq!(minerva.on_hotkey("ctrl+shift+d"), None);
    minerva
}

/// A listing in slot `slot` (0 at the top) read as `name` and `price`.
fn row(slot: u32, name: Option<&str>, price: Option<(u64, u64)>) -> ShopRow {
    let y = LIST.y + (slot as f32 * 62.6).round() as i32;
    ShopRow {
        area: ScreenRect { x: LIST.x + 58, y, width: 253, height: 54 },
        name: name.map(String::from),
        price: price.map(|(quantity, mesos)| ListedPrice { quantity, mesos }),
        covered: false,
    }
}

/// The listing in slot `slot` covered: by the pointer, or the game's tooltip by it.
fn covered(slot: u32) -> ShopRow {
    ShopRow { covered: true, ..row(slot, None, None) }
}

fn shop(rows: Vec<ShopRow>, at_ms: u64) -> ShopReading {
    ShopReading { shop: Some(ShopList { area: LIST, rows }), hover: None, at_ms, near_miss: None }
}

fn listed(quantity: u64, mesos: u64) -> ListedPrice {
    ListedPrice { quantity, mesos }
}

fn item(id: &str, name: &str) -> DealItem {
    DealItem { id: id.into(), name: name.into(), rechargeable: false }
}

fn verdicts(state: &PanelState) -> Vec<Verdict> {
    state.deal_badges.as_ref().map(|badges| badges.badges.iter().map(|b| b.verdict.clone()).collect()).unwrap_or_default()
}

#[test]
fn it_is_off_at_launch_and_its_hotkey_turns_it_on_and_off() {
    let mut minerva = started();
    assert_eq!(minerva.panel_state().deal_finder, DealFinderView { on: false, deal_line: Some("Your price".into()), margin_percent: 0, shop_found: false });
    assert_eq!(minerva.panel_state().deal_finder_capture, None, "nothing captured while it is off");
    assert_eq!(minerva.panel_state().hotkeys.deal_finder, "ctrl+shift+d");

    assert_eq!(minerva.on_hotkey("ctrl+shift+d"), None);
    assert!(minerva.panel_state().deal_finder.on);
    assert_eq!(minerva.panel_state().deal_finder_capture, Some(GAME), "the whole client area, where the shop may be");
    assert_eq!(minerva.panel_state().deal_badges, None, "no shop read yet");

    // It stays on with no shop open, waiting for one.
    minerva.on_player_shop_reading(ShopReading { shop: None, hover: None, at_ms: 400, near_miss: None });
    assert!(minerva.panel_state().deal_finder.on);
    assert_eq!(minerva.panel_state().deal_finder_capture, Some(GAME));

    assert_eq!(minerva.on_hotkey("ctrl+shift+d"), None);
    assert!(!minerva.panel_state().deal_finder.on);
    assert_eq!(minerva.panel_state().deal_finder_capture, None);
}

#[test]
fn the_panels_deal_finder_line_turns_it_on_and_off_too() {
    let mut minerva = started();
    assert_eq!(minerva.press(PanelControl::ToggleDealFinder), None);
    assert!(minerva.panel_state().deal_finder.on);
    minerva.press(PanelControl::ToggleDealFinder);
    assert!(!minerva.panel_state().deal_finder.on);
}

#[test]
fn it_isnt_saved_with_the_settings() {
    let minerva = on();
    let saved = serde_json::to_string(&minerva.settings()).unwrap();
    let mut again = Minerva::new(FakeSource::osms_offline());
    again.restore(serde_json::from_str(&saved).unwrap());
    assert!(!again.panel_state().deal_finder.on, "off at every launch");
}

#[test]
fn each_listing_gets_a_badge_saying_whether_it_is_a_deal_against_the_deal_line() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(
        vec![
            row(0, Some("Garnet"), Some((1, 999))),
            row(1, Some("Garnet Ore"), Some((1, 1_008))),
            row(2, Some("Bronze Ore"), Some((1, 177))),
            row(3, Some("Garnet Ore"), Some((1, 900))),
        ],
        400,
    ));

    assert_eq!(
        verdicts(state),
        [
            Verdict::Deal { item: item(GARNET, "Garnet"), listed: listed(1, 999), percent: -32, line_mesos: 1_470, grade: DealGrade::Great },
            Verdict::NoDeal { item: item(GARNET_ORE, "Garnet Ore"), listed: listed(1, 1_008), percent: 12, line_mesos: 900 },
            Verdict::NoPrice { item: item(BRONZE_ORE, "Bronze Ore"), listed: listed(1, 177) },
            Verdict::NoDeal { item: item(GARNET_ORE, "Garnet Ore"), listed: listed(1, 900), percent: 0, line_mesos: 900 },
        ]
    );
    assert!(state.deal_finder.shop_found);
}

#[test]
fn a_listing_of_several_is_judged_by_the_price_of_one() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet Ore"), Some((10, 6_120)))], 400));
    assert_eq!(
        verdicts(state),
        [Verdict::Deal { item: item(GARNET_ORE, "Garnet Ore"), listed: listed(10, 6_120), percent: -32, line_mesos: 900, grade: DealGrade::Great }]
    );
}

#[test]
fn equipment_is_judged_by_its_name_alone() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(
        vec![row(0, Some("Emerald Staff"), Some((1, 3_999))), row(1, Some("Emerald Staff"), Some((1, 3_499)))],
        400,
    ));
    assert_eq!(
        verdicts(state),
        [
            Verdict::NoDeal { item: item(EMERALD_STAFF, "Emerald Staff"), listed: listed(1, 3_999), percent: 5, line_mesos: 3_800 },
            Verdict::Deal { item: item(EMERALD_STAFF, "Emerald Staff"), listed: listed(1, 3_499), percent: -8, line_mesos: 3_800, grade: DealGrade::Good },
        ]
    );
}

#[test]
fn a_name_read_with_the_letters_the_reader_confuses_is_still_the_item() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet 0re"), Some((1, 800)))], 400));
    assert_eq!(
        verdicts(state),
        [Verdict::Deal { item: item(GARNET_ORE, "Garnet Ore"), listed: listed(1, 800), percent: -11, line_mesos: 900, grade: DealGrade::Good }]
    );
}

#[test]
fn what_isnt_sure_gets_a_question_mark_never_another_items_verdict() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(
        vec![
            // Unreadable, or covered.
            row(0, None, Some((1, 999))),
            row(1, Some("Garnet"), None),
            // No item of that name.
            row(2, Some("Zakum Helmet Shard"), Some((1, 999))),
            // Items sharing the name that don't look alike.
            row(3, Some("Blue Moon"), Some((1, 9_000))),
            // A name cut short that several items' names start as.
            row(4, Some("Pet Equip Speed Scroll: L..."), Some((1, 12_999))),
        ],
        400,
    ));
    assert_eq!(verdicts(state), vec![Verdict::Unknown; 5]);
}

#[test]
fn a_name_cut_short_is_the_one_item_whose_name_starts_so() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(
        vec![row(0, Some("Pet Equip Speed Scroll: Le..."), Some((1, 12_999))), row(1, Some("Pet Equip Jump Scroll: H…"), Some((1, 500)))],
        400,
    ));
    assert_eq!(
        verdicts(state),
        [
            Verdict::Deal {
                item: item("02048000", "Pet Equip Speed Scroll: Legs for Made Up"),
                listed: listed(1, 12_999),
                percent: -13,
                line_mesos: 15_000,
                grade: DealGrade::Good,
            },
            Verdict::NoPrice { item: item("02048010", "Pet Equip Jump Scroll: Head for Made Up"), listed: listed(1, 500) },
        ]
    );
}

#[test]
fn before_the_item_files_are_in_every_badge_is_a_question_mark() {
    let mut minerva = Minerva::new(FakeSource::osms_offline().answering_item_set(vec![Poll::Pending]));
    minerva.restore(common::set_up());
    minerva.set_deal_badges_size(BADGES);
    minerva.on_game_window(Some(GameWindow { client: GAME, frame: GAME, screens: vec![SCREEN] }));
    minerva.tick(0);
    minerva.on_hotkey("ctrl+shift+d");
    let state = minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 400));
    assert_eq!(verdicts(state), [Verdict::Unknown]);
}

#[test]
fn the_deal_margin_is_how_far_below_the_line_a_deal_must_be() {
    let mut minerva = on();
    let rows = || vec![row(0, Some("Garnet Ore"), Some((1, 855))), row(1, Some("Garnet Ore"), Some((1, 810)))];

    minerva.act(Action::SetDealMargin { percent: 10 });
    let state = minerva.on_player_shop_reading(shop(rows(), 400));
    assert_eq!(state.deal_finder.margin_percent, 10);
    assert_eq!(
        verdicts(state),
        [
            Verdict::NoDeal { item: item(GARNET_ORE, "Garnet Ore"), listed: listed(1, 855), percent: -5, line_mesos: 900 },
            Verdict::Deal { item: item(GARNET_ORE, "Garnet Ore"), listed: listed(1, 810), percent: -10, line_mesos: 900, grade: DealGrade::Good },
        ]
    );
    assert_eq!(minerva.settings().deal_margin_percent, 10, "saved with the settings");

    minerva.act(Action::SetDealMargin { percent: 250 });
    assert_eq!(minerva.panel_state().deal_finder.margin_percent, 99, "at most 99%");
}

#[test]
fn the_deal_line_is_chosen_on_the_item_card_and_saved_with_the_market_prices() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 400));
    minerva.act(Action::OpenDealItem { row: 0 });
    let marked = |state: &PanelState| match &state.item_card.as_ref().unwrap().view {
        ItemCardView::Card(card) => card.market.iter().filter(|line| line.deal_line).map(|line| line.id.clone()).collect::<Vec<_>>(),
        _ => panic!("a card"),
    };
    assert_eq!(marked(minerva.panel_state()), ["1"], "the first line until the player chooses");

    let state = minerva.choose_deal_line("2");
    assert_eq!(marked(state), ["2"]);
    assert_eq!(state.deal_finder.deal_line.as_deref(), Some("Lowest ask"));
    assert_eq!(
        verdicts(state),
        [Verdict::NoDeal { item: item(GARNET, "Garnet"), listed: listed(1, 999), percent: 11, line_mesos: 900 }],
        "judged afresh against the line chosen"
    );
    assert_eq!(minerva.market_prices().deal_line.as_deref(), Some("2"));

    // Removing it, the first line is the Deal Line again.
    minerva.edit_card_market_line("2");
    minerva.act(Action::AskRemoveCardMarketLine);
    let state = minerva.act(Action::RemoveCardMarketLine);
    assert_eq!(marked(state), ["1"]);
    assert_eq!(state.deal_finder.deal_line.as_deref(), Some("Your price"));
    assert_eq!(minerva.market_prices().deal_line, None);
}

#[test]
fn choosing_a_line_that_isnt_there_changes_nothing() {
    let mut minerva = started();
    let revision = minerva.market_prices_revision();
    minerva.choose_deal_line("7");
    assert_eq!(minerva.market_prices_revision(), revision);
    assert_eq!(minerva.panel_state().deal_finder.deal_line.as_deref(), Some("Your price"));
}

#[test]
fn the_badges_sit_beside_the_shops_window_level_with_each_listing() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999))), row(2, None, None)], 400));
    let DealBadges { placement, badges } = state.deal_badges.clone().expect("badges");
    assert_eq!(placement, ScreenRect { x: 1199, y: 600, width: 160, height: 367 });
    let tops: Vec<(u32, u32, u32)> = badges.iter().map(|b: &DealBadge| (b.row, b.top, b.height)).collect();
    assert_eq!(tops, [(0, 0, 54), (1, 125, 54)]);
}

#[test]
fn with_no_room_left_of_the_shop_the_badges_go_right_of_it() {
    let mut minerva = on();
    let list = ScreenRect { x: GAME.x + 100, ..LIST };
    let reading = ShopReading { shop: Some(ShopList { area: list, rows: vec![] }), hover: None, at_ms: 400, near_miss: None };
    let state = minerva.on_player_shop_reading(reading);
    assert_eq!(state.deal_badges.as_ref().map(|b| b.placement.x), Some(list.x + 809 + 6));
}

/// #130's UI Size (ADR 0020): a listing's band 162 px wide, not 253, and the shop's window as
/// much smaller.
#[test]
fn at_another_ui_size_the_shops_window_is_as_much_smaller() {
    let mut minerva = on();
    let list = ScreenRect { x: GAME.x + 100, y: 600, width: 199, height: 235 };
    let band = ShopRow { area: ScreenRect { x: list.x + 37, y: 600, width: 162, height: 35 }, ..row(0, None, None) };
    let reading = ShopReading { shop: Some(ShopList { area: list, rows: vec![band] }), hover: None, at_ms: 400, near_miss: None };
    let state = minerva.on_player_shop_reading(reading);
    let DealBadges { placement, badges } = state.deal_badges.clone().expect("badges");
    assert_eq!(placement.x, list.x + 518 + 6, "right of a window 809 × 162/253 px right of the list");
    assert_eq!(badges.iter().map(|b| (b.top, b.height)).collect::<Vec<_>>(), [(0, 35)]);
}

#[test]
fn the_badges_go_when_the_shop_closes_or_the_deal_finder_is_turned_off() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 400));
    let state = minerva.on_player_shop_reading(ShopReading { shop: None, hover: None, at_ms: 800, near_miss: None });
    assert_eq!(state.deal_badges, None);
    assert!(!state.deal_finder.shop_found);

    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 1_200));
    let state = minerva.act(Action::ToggleDealFinder);
    assert_eq!(state.deal_badges, None);
    // A capture taken before it was turned off brings nothing back.
    let state = minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 1_600));
    assert_eq!(state.deal_badges, None);
}

#[test]
fn nothing_shows_or_is_captured_while_another_app_is_in_front() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 400));
    let state = minerva.on_foreground(Foreground::OtherApp);
    assert_eq!(state.deal_badges, None);
    assert_eq!(state.deal_finder_capture, None);
    let state = minerva.on_foreground(Foreground::Game);
    assert!(state.deal_badges.is_some());
}

#[test]
fn the_badges_hide_while_an_item_card_press_is_captured() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 400));
    assert_eq!(minerva.on_hotkey("ctrl+shift+i"), Some(Request::CaptureItemTooltip));
    assert_eq!(minerva.panel_state().deal_badges, None, "it would be in the capture");
    minerva.on_item_tooltip_reading(None);
    assert!(minerva.panel_state().deal_badges.is_some());
}

#[test]
fn a_click_on_a_badge_shows_its_items_card_beside_it_off_the_shop() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Bronze Ore"), Some((1, 177))), row(1, Some("Garnet"), Some((1, 999)))], 400));

    let state = minerva.act(Action::OpenDealItem { row: 1 });

    match &state.item_card.as_ref().expect("a card").view {
        ItemCardView::Card(card) => assert_eq!(card.id, GARNET),
        other => panic!("{other:?}"),
    }
    // Left of the badge, the tops level: the badge at 600 + 63.
    assert_eq!(state.item_card_placement, Some(ScreenRect { x: 1199 - 8 - 300, y: 663, width: 300, height: 160 }));
}

#[test]
fn a_click_on_a_question_mark_badge_shows_nothing() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Blue Moon"), Some((1, 9_000)))], 400));
    let state = minerva.act(Action::OpenDealItem { row: 0 });
    assert_eq!(state.item_card, None);
}

#[test]
fn a_file_from_before_the_deal_finder_gets_its_hotkey_unless_another_has_it() {
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    let file = r#"{"version":5,"hotkeys":{"itemCard":"ctrl+shift+i"}}"#;
    minerva.restore(serde_json::from_str(file).unwrap());
    assert_eq!(minerva.panel_state().hotkeys.deal_finder, "ctrl+shift+d");

    let mut minerva = Minerva::new(FakeSource::osms_offline());
    let file = r#"{"version":5,"hotkeys":{"search":"ctrl+shift+d"}}"#;
    minerva.restore(serde_json::from_str(file).unwrap());
    assert_eq!(minerva.panel_state().hotkeys.deal_finder, "", "off, as another hotkey has its default");
    assert_eq!(minerva.panel_state().hotkeys.search, "ctrl+shift+d");
    assert_eq!(minerva.settings().version, 8);
}

#[test]
fn its_hotkey_is_rebound_like_the_others() {
    assert_eq!(Hotkeys::default().get(Hotkey::DealFinder), "ctrl+shift+d");
    assert!(Hotkey::ALL.contains(&Hotkey::DealFinder));
}

#[test]
fn market_prices_saved_before_the_deal_line_read_with_the_first_line_as_it() {
    let saved: SavedMarketPrices = serde_json::from_str(r#"{"version":2,"lines":[{"id":"1","name":"Your price"}]}"#).unwrap();
    assert_eq!(saved.deal_line, None);
    let written = serde_json::to_string(&SavedMarketPrices { deal_line: Some("1".into()), ..saved }).unwrap();
    assert!(written.contains(r#""dealLine":"1""#), "{written}");
}

fn badge(state: &PanelState, row: u32) -> (bool, Verdict) {
    let badge = state.deal_badges.as_ref().expect("badges").badges.iter().find(|b| b.row == row).expect("that badge").clone();
    (badge.covered, badge.verdict)
}

fn garnet_deal() -> Verdict {
    Verdict::Deal { item: item(GARNET, "Garnet"), listed: listed(1, 999), percent: -32, line_mesos: 1_470, grade: DealGrade::Great }
}

#[test]
fn a_covered_listing_keeps_the_verdict_read_there_before_marked_covered() {
    let mut minerva = on();
    let ore = || row(1, Some("Garnet Ore"), Some((1, 1_008)));
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999))), ore()], 400));

    let state = minerva.on_player_shop_reading(shop(vec![covered(0), ore()], 800));
    assert_eq!(badge(state, 0), (true, garnet_deal()));
    assert!(!badge(state, 1).0);

    // A click on it shows its item's card, as before it was covered.
    let state = minerva.act(Action::OpenDealItem { row: 0 });
    match &state.item_card.as_ref().expect("a card").view {
        ItemCardView::Card(card) => assert_eq!(card.id, GARNET),
        other => panic!("{other:?}"),
    }

    // Uncovered, it is read afresh.
    let state = minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999))), ore()], 1_200));
    assert_eq!(badge(state, 0), (false, garnet_deal()));
}

#[test]
fn a_covered_listing_never_read_before_is_a_question_mark() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(vec![covered(0), row(1, Some("Garnet"), Some((1, 999)))], 400));
    assert_eq!(badge(state, 0), (true, Verdict::Unknown));
}

#[test]
fn once_the_list_has_changed_a_covered_listing_keeps_nothing_read_before() {
    // Scrolled: another listing reads otherwise where it was.
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999))), row(1, Some("Garnet Ore"), Some((1, 900)))], 400));
    let state = minerva.on_player_shop_reading(shop(vec![covered(0), row(1, Some("Bronze Ore"), Some((1, 177)))], 800));
    assert_eq!(badge(state, 0), (true, Verdict::Unknown));

    // A listing read before is gone (sold out, or bought), and nothing covers its slot.
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999))), row(1, Some("Garnet Ore"), Some((1, 900)))], 400));
    let state = minerva.on_player_shop_reading(shop(vec![covered(0)], 800));
    assert_eq!(badge(state, 0), (true, Verdict::Unknown));

    // The shop moved.
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 400));
    let moved = ShopRow { area: ScreenRect { x: LIST.x + 100, ..covered(0).area }, ..covered(0) };
    let reading = ShopReading { shop: Some(ShopList { area: ScreenRect { x: LIST.x + 42, ..LIST }, rows: vec![moved] }), hover: None, at_ms: 800, near_miss: None };
    let state = minerva.on_player_shop_reading(reading);
    assert_eq!(badge(state, 0), (true, Verdict::Unknown));

    // Turned off and on again.
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Garnet"), Some((1, 999)))], 400));
    minerva.act(Action::ToggleDealFinder);
    minerva.act(Action::ToggleDealFinder);
    let state = minerva.on_player_shop_reading(shop(vec![covered(0)], 800));
    assert_eq!(badge(state, 0), (true, Verdict::Unknown));
}

/// The name the game cuts both Pet Equip Speed Scrolls short to.
const SPEED_SCROLL_L: &str = "Pet Equip Speed Scroll: L...";
const LEGS: &str = "Pet Equip Speed Scroll: Legs for Made Up";

/// The pointer on slot `slot`'s picture, and the game's Item Tooltip below it, its name read
/// as `name` (`None`: unreadable).
fn hovering(slot: u32, name: Option<&str>) -> ShopHover {
    let band = row(slot, None, None).area;
    let tooltip = ScreenRect { x: LIST.x + 30, y: band.y + 52, width: 420, height: 18 };
    ShopHover { pointer: ScreenPoint { x: LIST.x + 20, y: band.y + 20 }, tooltip: ItemTooltip::Found { area: tooltip, name: name.map(String::from) } }
}

/// `rows` read with the pointer on the list, as `hover` says.
fn hovered(rows: Vec<ShopRow>, hover: ShopHover, at_ms: u64) -> ShopReading {
    ShopReading { hover: Some(hover), ..shop(rows, at_ms) }
}

/// A Pet Equip Speed Scroll listed for 12,999 mesos, its name cut short, in slot 0; and a
/// Garnet in slot 1.
fn scrolls_listed() -> Vec<ShopRow> {
    vec![row(0, Some(SPEED_SCROLL_L), Some((1, 12_999))), row(1, Some("Garnet"), Some((1, 999)))]
}

/// The same, the game's tooltip by the pointer on the scroll's listing hiding its price.
fn scroll_hovered() -> Vec<ShopRow> {
    vec![row(0, Some(SPEED_SCROLL_L), None), row(1, Some("Garnet"), Some((1, 999)))]
}

fn legs_deal() -> Verdict {
    Verdict::Deal { item: item("02048000", LEGS), listed: listed(1, 12_999), percent: -13, line_mesos: 15_000, grade: DealGrade::Good }
}

#[test]
fn a_name_cut_short_that_starts_several_items_names_is_named_by_the_tooltip_hovered_over_it() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(scrolls_listed(), 400));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown), "both scrolls' names start so");

    // Hovered: the tooltip names it, and hides its price, which stands as read before.
    let state = minerva.on_player_shop_reading(hovered(scroll_hovered(), hovering(0, Some(LEGS)), 800));
    assert_eq!(badge(state, 0), (true, legs_deal()));
    assert_eq!(badge(state, 1), (false, garnet_deal()));

    // The pointer gone, the name stands while the list does.
    let state = minerva.on_player_shop_reading(shop(scrolls_listed(), 1_200));
    assert_eq!(badge(state, 0), (false, legs_deal()));

    // A click on its badge shows that scroll's card.
    let state = minerva.act(Action::OpenDealItem { row: 0 });
    match &state.item_card.as_ref().expect("a card").view {
        ItemCardView::Card(card) => assert_eq!(card.id, "02048000"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_tooltip_whose_name_doesnt_start_as_the_listings_names_nothing() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(scrolls_listed(), 400));

    // The last listing's tooltip, lingering; or one read wrong.
    let state = minerva.on_player_shop_reading(hovered(
        scroll_hovered(),
        hovering(0, Some("Pet Equip Jump Scroll: Head for Made Up")),
        800,
    ));
    assert_eq!(badge(state, 0), (true, Verdict::Unknown));
    let state = minerva.on_player_shop_reading(shop(scrolls_listed(), 1_200));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));
}

#[test]
fn a_tooltip_names_only_the_listing_the_pointer_is_on() {
    // Between the listings' bands.
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(scrolls_listed(), 400));
    let mut between = hovering(0, Some(LEGS));
    between.pointer.y = row(0, None, None).area.y + 58;
    let state = minerva.on_player_shop_reading(hovered(scrolls_listed(), between, 800));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));

    // On the listing below, whose name is whole.
    let state = minerva.on_player_shop_reading(hovered(scrolls_listed(), hovering(1, Some(LEGS)), 1_200));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));
    assert_eq!(badge(state, 1), (false, garnet_deal()));
}

#[test]
fn a_tooltip_naming_items_that_dont_look_alike_names_nothing() {
    let mut minerva = on();
    let moon = || vec![row(0, Some("Blue Mo..."), Some((1, 9_000)))];
    minerva.on_player_shop_reading(shop(moon(), 400));
    let state = minerva.on_player_shop_reading(hovered(vec![row(0, Some("Blue Mo..."), None)], hovering(0, Some("Blue Moon")), 800));
    assert_eq!(badge(state, 0), (true, Verdict::Unknown), "a top and an earring");
}

#[test]
fn a_name_a_tooltip_gave_is_forgotten_once_the_list_changes() {
    let named = || {
        let mut minerva = on();
        minerva.on_player_shop_reading(shop(scrolls_listed(), 400));
        minerva.on_player_shop_reading(hovered(scroll_hovered(), hovering(0, Some(LEGS)), 800));
        minerva
    };

    // Scrolled: another listing reads otherwise where it was.
    let mut minerva = named();
    let scrolled = vec![row(0, Some(SPEED_SCROLL_L), Some((1, 12_999))), row(1, Some("Garnet Ore"), Some((1, 900)))];
    let state = minerva.on_player_shop_reading(shop(scrolled, 1_200));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));

    // Its listing reads at another price: another seller's scroll, or the other scroll.
    let mut minerva = named();
    let state = minerva.on_player_shop_reading(shop(vec![row(0, Some(SPEED_SCROLL_L), Some((1, 14_000))), row(1, Some("Garnet"), Some((1, 999)))], 1_200));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));

    // Bought, or sold out: gone, and nothing covers its slot.
    let mut minerva = named();
    minerva.on_player_shop_reading(shop(vec![row(1, Some("Garnet"), Some((1, 999)))], 1_200));
    let state = minerva.on_player_shop_reading(shop(scrolls_listed(), 1_600));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));

    // The shop moved: another shop's, or this one dragged.
    let mut minerva = named();
    let moved = |row: ShopRow| ShopRow { area: ScreenRect { x: row.area.x + 42, ..row.area }, ..row };
    let list = ShopList { area: ScreenRect { x: LIST.x + 42, ..LIST }, rows: scrolls_listed().into_iter().map(moved).collect() };
    let state = minerva.on_player_shop_reading(ShopReading { shop: Some(list), hover: None, at_ms: 1_200, near_miss: None });
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));

    // Turned off and on again.
    let mut minerva = named();
    minerva.act(Action::ToggleDealFinder);
    minerva.act(Action::ToggleDealFinder);
    let state = minerva.on_player_shop_reading(shop(scrolls_listed(), 1_200));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));
}

#[test]
fn a_listing_never_read_clean_isnt_named_until_it_is() {
    // The pointer was on it from the first capture, covering its name: nothing to compare
    // the tooltip's with.
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(hovered(vec![covered(0), row(1, Some("Garnet"), Some((1, 999)))], hovering(0, Some(LEGS)), 400));
    assert_eq!(badge(state, 0), (true, Verdict::Unknown));
    let state = minerva.on_player_shop_reading(shop(scrolls_listed(), 800));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown), "not named by a tooltip it wasn't compared with");

    // Its name read, its price hidden from the first: no price yet.
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(hovered(scroll_hovered(), hovering(0, Some(LEGS)), 400));
    assert_eq!(badge(state, 0), (false, Verdict::Unknown));
    let state = minerva.on_player_shop_reading(shop(scrolls_listed(), 800));
    assert_eq!(badge(state, 0), (false, legs_deal()), "named while its name read, priced once it read whole");
}

#[test]
fn a_throwing_star_is_judged_by_its_listings_whole_price_however_many_it_holds() {
    let mut minerva = on();
    let state = minerva.on_player_shop_reading(shop(
        vec![row(0, Some("Wooden Top"), Some((800, 999_999))), row(1, Some("Wooden Top"), Some((800, 150_000)))],
        400,
    ));
    let star = DealItem { rechargeable: true, ..item(WOODEN_TOP, "Wooden Top") };
    assert_eq!(
        verdicts(state),
        [
            Verdict::NoDeal { item: star.clone(), listed: listed(800, 999_999), percent: 429, line_mesos: 188_888 },
            Verdict::Deal { item: star, listed: listed(800, 150_000), percent: -21, line_mesos: 188_888, grade: DealGrade::Great },
        ]
    );
}

#[test]
fn a_card_opened_from_a_badge_unfolds_from_the_edge_facing_it_and_the_badge_stays_pressed() {
    let mut minerva = on();
    minerva.on_player_shop_reading(shop(vec![row(0, Some("Bronze Ore"), Some((1, 177))), row(1, Some("Garnet"), Some((1, 999)))], 400));

    let state = minerva.act(Action::OpenDealItem { row: 1 }).clone();
    assert_eq!(state.item_card.as_ref().and_then(|card| card.unfold_from), Some(Edge::Right), "the card is left of the badge");
    let open: Vec<bool> = state.deal_badges.as_ref().unwrap().badges.iter().map(|b| b.open).collect();
    assert_eq!(open, [false, true]);
    assert!(state.deal_badges_clickable);

    // Dragged, it no longer belongs beside the badge.
    let card = state.item_card_placement.unwrap();
    let state = minerva.act(Action::ItemCardMoved { x: card.x - 100, y: card.y });
    assert_eq!(state.item_card.as_ref().and_then(|card| card.unfold_from), None);
    assert!(state.deal_badges.as_ref().unwrap().badges.iter().all(|b| !b.open));

    // Closed, no badge is pressed.
    minerva.act(Action::OpenDealItem { row: 1 });
    let state = minerva.close_item_card();
    assert!(state.deal_badges.as_ref().unwrap().badges.iter().all(|b| !b.open));
}

#[test]
fn a_deal_is_graded_good_great_or_a_steal_by_how_far_below_the_line_it_is() {
    let mut minerva = on();
    let grades = |state: &PanelState| -> Vec<DealGrade> {
        verdicts(state)
            .into_iter()
            .map(|verdict| match verdict {
                Verdict::Deal { grade, .. } => grade,
                other => panic!("{other:?}"),
            })
            .collect()
    };
    let state = minerva.on_player_shop_reading(shop(
        vec![
            row(0, Some("Garnet Ore"), Some((1, 899))),
            row(1, Some("Garnet Ore"), Some((1, 725))),
            row(2, Some("Garnet Ore"), Some((1, 720))),
            row(3, Some("Garnet Ore"), Some((1, 545))),
            row(4, Some("Garnet Ore"), Some((1, 540))),
        ],
        400,
    ));
    assert_eq!(
        grades(state),
        [DealGrade::Good, DealGrade::Good, DealGrade::Great, DealGrade::Great, DealGrade::Steal],
        "−0%, −19%, −20%, −39%, −40% of 900"
    );
}
