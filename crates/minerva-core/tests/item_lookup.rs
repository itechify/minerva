//! Item Lookup (ADR 0007): the player hovers an item in the game and presses Ctrl+Shift+U.
//! The shell captures the game's client area with the pointer, the Item Tooltip's name is
//! read, and MeowDB's search turns the name into the item's page, which opens in the
//! browser. A name MeowDB doesn't clearly have opens nothing, and the panel says why.
//!
//! The readings here stand for what the Item Tooltip reader made of a capture (its own
//! samples are in `item_tooltip_samples.rs`); MeowDB's answers are made up in its search's
//! shape ([`made_up_search`]).
//!
//! Worked example, as in `grind_tracker_window.rs`: the game's client area at (812,110),
//! 1920x1200 physical px, on one 3440x1440 monitor; an expanded panel of 340x72, so at
//! (2376,510), and an Owl Bubble of 420x56, which says what a lookup came to. The Item
//! Tooltip's name line is read at (1000,600), clear of both.

mod common;

use common::{FakeSource, GAME, NAME, OWL, POINTER, found, owl_at, owl_clickable, owl_line, owl_mood, requests};
use minerva_core::{
    Action, BESTS_VERSION, Minerva, Foreground, ItemChoice, ItemHit, ItemLookup, ItemTooltip,
    ItemTooltipReading, MapBests, OurWindow, OwlLine, OwlMood, PanelControl, PanelSizes, PanelState, Request, SavedBests, ScreenRect, Settings, Size, SourceError, VisitBest, meowdb,
};

/// Where the expanded panel is.
const PANEL: ScreenRect = ScreenRect { x: 2376, y: 510, width: 340, height: 72 };

/// A core over the game window, its panel's sizes known, MeowDB's search answered by
/// `source`.
fn started_with(source: FakeSource, settings: Settings) -> Minerva {
    let mut minerva = Minerva::new(source);
    minerva.restore(settings);
    minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    minerva.set_grind_tracker_size(Size { width: 300, height: 150 });
    minerva.set_owl_bubble_size(OWL);
    minerva.on_game_window(Some(common::window_on_wide_screen(GAME)));
    common::found_title(&mut minerva, common::title_on(GAME), 0);
    minerva
}

/// MeowDB's search, made up: for each name these tests look up, an answer in its search's
/// shape (up to six close items, closest first, each with MeowDB's own ID and its category
/// as `sub`), read with the core's MeowDB reader. The IDs are invented, and so are most
/// of the other items. A name without an answer here can't be reached.
fn made_up_search(query: &str) -> Result<Vec<ItemHit>, SourceError> {
    let answer = match query {
        // Other ores, but no Bronze Ore.
        "Orihalcon Ore" => {
            r#"{"items":[
                {"id":101,"name":"Orihalcon Ore","sub":"Ore"},
                {"id":102,"name":"Silver Ore","sub":"Ore"},
                {"id":103,"name":"Steel Ore","sub":"Ore"},
                {"id":104,"name":"Adamantium Ore","sub":"Ore"},
                {"id":105,"name":"Garnet Ore","sub":"Gem"},
                {"id":106,"name":"Lidium Ore","sub":null}
            ]}"#
        }
        "Orihalcon Oro" => {
            r#"{"items":[
                {"id":101,"name":"Orihalcon Ore","sub":"Ore"},
                {"id":111,"name":"Orihalcon Plate","sub":"Ore"},
                {"id":112,"name":"Orihalcon Helmet","sub":"Hat"},
                {"id":113,"name":"Orihalcon Shield","sub":"Shield"}
            ]}"#
        }
        // A chair: no category.
        "The Green Relaxer" => {
            r#"{"items":[
                {"id":201,"name":"The Green Relaxer","sub":null},
                {"id":202,"name":"Green Cushion","sub":null},
                {"id":203,"name":"Relaxing Bench","sub":null}
            ]}"#
        }
        "Soap Bubbles" => {
            r#"{"items":[
                {"id":301,"name":"Soap Bubbles","sub":"Weather Effect"},
                {"id":302,"name":"Bubble Wand","sub":"Toy"},
                {"id":303,"name":"Soap Dish","sub":null}
            ]}"#
        }
        "Ribboned Pig Headband" => {
            r#"{"items":[
                {"id":401,"name":"Ribboned Pig Headband","sub":"Hat"},
                {"id":402,"name":"Pink Ribbon","sub":"Hat"},
                {"id":403,"name":"Pig Doll","sub":"Toy"}
            ]}"#
        }
        "Mithril Guards" => {
            r#"{"items":[
                {"id":501,"name":"Mithril Guards","sub":"Claw"},
                {"id":502,"name":"Mithril Plate","sub":"Ore"},
                {"id":503,"name":"Mithril Shield","sub":"Shield"}
            ]}"#
        }
        // Two items share the name; a quest of that name isn't an item.
        "Hero's Gladius" => {
            r#"{"items":[
                {"id":601,"name":"Hero's Gladius","sub":"Quest Item"},
                {"id":602,"name":"Hero's Gladius","sub":"One-Handed Sword"},
                {"id":603,"name":"Gladius","sub":"One-Handed Sword"},
                {"id":604,"name":"Hero's Cape","sub":"Cape"}
            ],"quests":[
                {"id":"9001","name":"Hero's Gladius","sub":"A quest"}
            ]}"#
        }
        // Six items share the name, as many as an answer holds.
        "Omok Table" => {
            r#"{"items":[
                {"id":701,"name":"Omok Table","sub":"Quest Item"},
                {"id":702,"name":"Omok Table","sub":"Quest Item"},
                {"id":703,"name":"Omok Table","sub":"Quest Item"},
                {"id":704,"name":"Omok Table","sub":"Quest Item"},
                {"id":705,"name":"Omok Table","sub":"Quest Item"},
                {"id":706,"name":"Omok Table","sub":"Quest Item"}
            ]}"#
        }
        "Zqxv Nothing" => r#"{"items":[]}"#,
        _ => return Err(SourceError::Unreachable(format!("no made-up answer for {query:?}"))),
    };
    meowdb::read_item_search(answer)
}

/// MeowDB's search answers with its made-up answers, as soon as asked (from the cache).
fn made_up_meowdb() -> FakeSource {
    FakeSource::osms_offline().with_item_searches(made_up_search)
}

/// MeowDB's made-up answers, as soon as asked.
fn started() -> Minerva {
    started_with(made_up_meowdb(), common::set_up())
}

/// MeowDB's search answers whatever is asked with its made-up answer for `query`.
fn answering(query: &'static str) -> FakeSource {
    FakeSource::osms_offline().with_item_searches(move |_| made_up_search(query))
}

/// MeowDB's search answers whatever is asked with these items.
fn answering_items(items: &[(&str, &str)]) -> FakeSource {
    let hits: Vec<ItemHit> = items
        .iter()
        .map(|(id, name)| ItemHit { id: id.to_string(), name: name.to_string(), category: None })
        .collect();
    FakeSource::osms_offline().with_item_searches(move |_| Ok(hits.clone()))
}

/// The Item Tooltip near the pointer read as `name`, after the player pressed the hotkey.
fn look_up(minerva: &mut Minerva, name: &str, at_ms: u64) -> PanelState {
    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), Some(Request::CaptureItemTooltip));
    minerva.on_item_tooltip_reading(found(name, at_ms)).clone()
}

fn line(state: &PanelState) -> Option<ItemLookup> {
    state.item_lookup.as_ref().map(|line| line.outcome.clone())
}

fn opened(name: &str, id: &str) -> Option<ItemLookup> {
    Some(ItemLookup::Opened { name: name.into(), url: meowdb::item_url(id) })
}

fn not_found(read: &str) -> Option<ItemLookup> {
    Some(ItemLookup::NotFound { read: read.into() })
}

fn open(id: &str) -> Request {
    Request::OpenInBrowser { url: meowdb::item_url(id) }
}

#[test]
fn ctrl_shift_u_over_the_game_asks_the_shell_to_capture_it_with_the_pointer() {
    let mut minerva = started();

    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), Some(Request::CaptureItemTooltip));
}

#[test]
fn without_a_game_window_ctrl_shift_u_does_nothing() {
    let mut minerva = Minerva::new(FakeSource::osms_offline());

    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), None);
    assert_eq!(minerva.panel_state().item_lookup, None);
}

#[test]
fn a_press_while_the_last_capture_is_still_being_read_is_ignored() {
    let mut minerva = started();
    minerva.on_hotkey("ctrl+shift+u");

    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), None, "the first capture is still being read");

    minerva.on_item_tooltip_reading(found("Orihalcon Ore", 1_000));
    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), Some(Request::CaptureItemTooltip));
}

#[test]
fn a_capture_that_failed_lets_the_next_press_capture_again() {
    let mut minerva = started();
    minerva.on_hotkey("ctrl+shift+u");

    let state = minerva.on_item_tooltip_reading(None).clone();

    assert_eq!(line(&state), Some(ItemLookup::Unreadable));
    assert_eq!(minerva.on_hotkey("ctrl+shift+u"), Some(Request::CaptureItemTooltip));
}

#[test]
fn a_name_meowdb_has_opens_its_page() {
    let mut minerva = started();

    let state = look_up(&mut minerva, "Orihalcon Ore", 1_000);

    assert_eq!(line(&state), opened("Orihalcon Ore", "101"));
    assert_eq!(requests(&mut minerva), [open("101")]);
}

#[test]
fn a_saved_copy_of_meowdbs_answer_opens_the_page_as_a_fresh_one_does() {
    // MeowDB can't be reached, and every search is answered with a copy saved earlier.
    let mut minerva = started_with(made_up_meowdb().serving_saved_copies(common::SAVED_AT), common::set_up());

    let state = look_up(&mut minerva, "Orihalcon Ore", 1_000);

    assert_eq!(line(&state), opened("Orihalcon Ore", "101"));
    assert_eq!(requests(&mut minerva), [open("101")]);
}

#[test]
fn every_seed_item_opens_its_own_page() {
    for (name, id) in [
        ("Orihalcon Ore", "101"),
        ("The Green Relaxer", "201"),
        ("Soap Bubbles", "301"),
        ("Ribboned Pig Headband", "401"),
        ("Mithril Guards", "501"),
    ] {
        let mut minerva = started();
        let state = look_up(&mut minerva, name, 1_000);
        assert_eq!(line(&state), opened(name, id), "{name}");
        assert_eq!(requests(&mut minerva), [open(id)], "{name}");
    }
}

#[test]
fn while_meowdb_is_asked_the_panel_shows_the_name_read_and_the_page_opens_when_it_answers() {
    let mut minerva = started_with(made_up_meowdb().items_taking(2), common::set_up());

    let state = look_up(&mut minerva, "Orihalcon Ore", 1_000);
    assert_eq!(line(&state), Some(ItemLookup::LookingUp { read: "Orihalcon Ore".into() }));
    assert!(requests(&mut minerva).is_empty());

    minerva.tick(1_100);
    let state = minerva.tick(1_200).clone();

    assert_eq!(line(&state), opened("Orihalcon Ore", "101"));
    assert_eq!(requests(&mut minerva), [open("101")]);
}

#[test]
fn meowdb_is_asked_once_per_lookup() {
    let source = made_up_meowdb().items_taking(2);
    let asked = source.item_requests.clone();
    let mut minerva = started_with(source, common::set_up());

    look_up(&mut minerva, "Orihalcon Ore", 1_000);
    for at_ms in [1_100, 1_200, 1_300, 1_400] {
        minerva.tick(at_ms);
    }

    assert_eq!(*asked.lock().unwrap(), ["Orihalcon Ore"]);
}

#[test]
fn meowdb_is_asked_no_more_than_once_every_2_seconds() {
    // Each search goes over the network, answering on the poll after it started.
    let source = made_up_meowdb().items_taking(1);
    let asked = source.item_requests.clone();
    let mut minerva = started_with(source, common::set_up());
    look_up(&mut minerva, "Orihalcon Ore", 1_000);
    minerva.tick(1_100);

    // A lookup a second later waits out MeowDB's pace, the name read showing meanwhile.
    let state = look_up(&mut minerva, "Soap Bubbles", 2_000);
    assert_eq!(line(&state), Some(ItemLookup::LookingUp { read: "Soap Bubbles".into() }));
    minerva.tick(2_900);
    assert_eq!(*asked.lock().unwrap(), ["Orihalcon Ore"]);

    minerva.tick(3_000);
    let state = minerva.tick(3_100).clone();

    assert_eq!(*asked.lock().unwrap(), ["Orihalcon Ore", "Soap Bubbles"]);
    assert_eq!(line(&state), opened("Soap Bubbles", "301"));
}

#[test]
fn letters_the_recogniser_confuses_and_case_still_find_the_item() {
    for read in ["0rihalcon 0re", "Orihalc0n Ore", "Oriha1con Ore", "orihalcon ore", "Orihalcon  Ore"] {
        let mut minerva = started_with(answering("Orihalcon Ore"), common::set_up());
        let state = look_up(&mut minerva, read, 1_000);
        assert_eq!(line(&state), opened("Orihalcon Ore", "101"), "{read:?}");
    }
    let mut minerva = started_with(answering("Hero's Gladius"), common::set_up());
    let state = look_up(&mut minerva, "Heros GIadius", 1_000);
    assert!(matches!(line(&state), Some(ItemLookup::Choose { .. })), "punctuation and I for l: {state:?}");
    assert!(owl_clickable(&state), "its items are picked with a click");
}

#[test]
fn a_name_one_letter_off_opens_the_item_far_closer_than_any_other() {
    // Orihalcon Ore, then other Orihalcon items.
    let mut minerva = started();

    let state = look_up(&mut minerva, "Orihalcon Oro", 1_000);

    assert_eq!(line(&state), opened("Orihalcon Ore", "101"));
}

#[test]
fn a_name_as_close_to_two_items_opens_neither() {
    let items = [("126", "Hat MP Scroll: Greater"), ("125", "Hat HP Scroll: Greater")];
    let mut minerva = started_with(answering_items(&items), common::set_up());

    let state = look_up(&mut minerva, "Hat NP Scroll: Greater", 1_000);

    assert_eq!(line(&state), not_found("Hat NP Scroll: Greater"));
    assert!(requests(&mut minerva).is_empty());
}

#[test]
fn a_name_one_letter_closer_to_one_of_two_items_still_opens_neither() {
    let items = [("126", "Hat MP Scroll: Greater"), ("125", "Hat HP Scroll: Greater")];
    let mut minerva = started_with(answering_items(&items), common::set_up());

    let state = look_up(&mut minerva, "Hat MP Scroll: Greatr", 1_000);

    assert_eq!(line(&state), not_found("Hat MP Scroll: Greatr"));
}

#[test]
fn the_exact_name_wins_over_a_neighbour_one_letter_away() {
    let items = [("125", "Hat HP Scroll: Greater"), ("126", "Hat MP Scroll: Greater")];
    let mut minerva = started_with(answering_items(&items), common::set_up());

    let state = look_up(&mut minerva, "Hat MP Scroll: Greater", 1_000);

    assert_eq!(line(&state), opened("Hat MP Scroll: Greater", "126"));
}

#[test]
fn a_name_meowdb_answers_only_with_other_items_opens_none() {
    // Silver Ore, Steel Ore and other ores, but no Bronze Ore.
    let mut minerva = started_with(answering("Orihalcon Ore"), common::set_up());

    let state = look_up(&mut minerva, "Bronze Ore", 1_000);

    assert_eq!(line(&state), not_found("Bronze Ore"));
    assert!(requests(&mut minerva).is_empty());
}

#[test]
fn a_name_no_meowdb_item_has_says_so() {
    let mut minerva = started();

    let state = look_up(&mut minerva, "Zqxv Nothing", 1_000);

    assert_eq!(line(&state), not_found("Zqxv Nothing"));
    assert!(requests(&mut minerva).is_empty());
}

#[test]
fn a_name_several_items_share_lists_them_with_their_categories_to_pick_from() {
    let mut minerva = started();

    let state = look_up(&mut minerva, "Hero's Gladius", 1_000);

    let choice = |id: &str, category: &str| ItemChoice {
        id: id.into(),
        name: "Hero's Gladius".into(),
        category: Some(category.into()),
        url: meowdb::item_url(id),
    };
    assert_eq!(
        line(&state),
        Some(ItemLookup::Choose {
            read: "Hero's Gladius".into(),
            items: vec![choice("601", "Quest Item"), choice("602", "One-Handed Sword")],
        }),
        "not Gladius, Hero's Cape or the quest"
    );
    assert!(requests(&mut minerva).is_empty(), "nothing opens until the player picks");

    assert_eq!(minerva.pick_item("602"), Some(open("602")));
    assert_eq!(line(minerva.panel_state()), opened("Hero's Gladius", "602"));
}

#[test]
fn only_an_item_on_the_list_can_be_picked() {
    let mut minerva = started();
    look_up(&mut minerva, "Hero's Gladius", 1_000);

    assert_eq!(minerva.pick_item("603"), None, "Gladius isn't on the list");
}

#[test]
fn six_items_sharing_a_name_are_all_listed() {
    let mut minerva = started();

    let state = look_up(&mut minerva, "Omok Table", 1_000);

    let Some(ItemLookup::Choose { items, .. }) = line(&state) else { panic!("a list: {state:?}") };
    let ids: Vec<&str> = items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, ["701", "702", "703", "704", "705", "706"]);
}

#[test]
fn the_list_goes_after_half_a_minute() {
    let mut minerva = started();
    look_up(&mut minerva, "Hero's Gladius", 1_000);

    assert!(line(minerva.tick(30_999)).is_some());
    assert_eq!(line(minerva.tick(31_000)), None);
}

#[test]
fn the_next_lookup_replaces_the_list() {
    let mut minerva = started();
    look_up(&mut minerva, "Hero's Gladius", 1_000);

    let state = look_up(&mut minerva, "Orihalcon Ore", 2_000);

    assert_eq!(line(&state), opened("Orihalcon Ore", "101"));
    assert_eq!(minerva.pick_item("602"), None, "the list has gone");
}

#[test]
fn meowdb_unreachable_says_so() {
    let source = FakeSource::osms_offline().with_item_searches(|_| Err(common::unreachable()));
    let mut minerva = started_with(source, common::set_up());

    let state = look_up(&mut minerva, "Orihalcon Ore", 1_000);

    assert_eq!(line(&state), Some(ItemLookup::Unreachable));
    assert!(requests(&mut minerva).is_empty());
}

#[test]
fn a_search_answer_in_another_shape_says_lookup_is_unavailable() {
    for answer in ["<!DOCTYPE html><html></html>", r#"{"results":[]}"#, r#"{"items":[{"id":101}]}"#] {
        let source = FakeSource::osms_offline().with_item_searches(move |_| meowdb::read_item_search(answer));
        let mut minerva = started_with(source, common::set_up());
        let state = look_up(&mut minerva, "Orihalcon Ore", 1_000);
        assert_eq!(line(&state), Some(ItemLookup::FormatChanged), "{answer}");
        assert!(requests(&mut minerva).is_empty(), "{answer}");
    }
}

#[test]
fn no_item_tooltip_near_the_pointer_says_so_and_nothing_is_looked_up() {
    let source = FakeSource::osms_offline();
    let asked = source.item_requests.clone();
    let mut minerva = started_with(source, common::set_up());
    minerva.on_hotkey("ctrl+shift+u");

    let state = minerva.on_item_tooltip_reading(Some(ItemTooltipReading { pointer: POINTER, tooltip: ItemTooltip::None, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::NoTooltip));
    assert!(asked.lock().unwrap().is_empty());
}

#[test]
fn an_item_tooltip_whose_name_cant_be_read_says_so() {
    let mut minerva = started();
    minerva.on_hotkey("ctrl+shift+u");

    let unreadable = ItemTooltip::Found { area: NAME, name: None };
    let state = minerva.on_item_tooltip_reading(Some(ItemTooltipReading { pointer: POINTER, tooltip: unreadable, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::Unreadable));
}

#[test]
fn a_name_read_as_too_little_to_be_one_is_unreadable() {
    let mut minerva = started();

    let state = look_up(&mut minerva, " . ", 1_000);

    assert_eq!(line(&state), Some(ItemLookup::Unreadable));
}

#[test]
fn our_panel_over_the_name_says_so_and_nothing_is_looked_up() {
    let source = FakeSource::osms_offline();
    let asked = source.item_requests.clone();
    let mut minerva = started_with(source, common::set_up());
    assert_eq!(minerva.panel_state().placement, Some(PANEL));
    minerva.on_hotkey("ctrl+shift+u");

    // The name runs on under the panel's left edge.
    let under_panel = ScreenRect { x: PANEL.x - 150, y: PANEL.y + 20, width: 170, height: 19 };
    let tooltip = ItemTooltip::Found { area: under_panel, name: Some("Orihalcon".into()) };
    let state = minerva.on_item_tooltip_reading(Some(ItemTooltipReading { pointer: POINTER, tooltip, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::Covered { by: OurWindow::Panel }));
    assert!(asked.lock().unwrap().is_empty(), "a name cut short isn't looked up");
}

#[test]
fn the_grind_tracker_over_the_name_says_so() {
    let tracking = Settings {
        ..common::set_up()
    };
    let mut minerva = started_with(FakeSource::osms_offline(), tracking);
    let tracker = minerva.panel_state().grind_tracker.placement.expect("the Grind Tracker shows");
    minerva.on_hotkey("ctrl+shift+u");

    let under_tracker = ScreenRect { x: tracker.x + 10, y: tracker.y + 10, width: 170, height: 19 };
    let tooltip = ItemTooltip::Found { area: under_tracker, name: Some("Orihalcon Ore".into()) };
    let state = minerva.on_item_tooltip_reading(Some(ItemTooltipReading { pointer: POINTER, tooltip, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::Covered { by: OurWindow::GrindTracker }));
}

#[test]
fn the_personal_bests_window_over_the_name_says_so() {
    let mut minerva = started_with(FakeSource::osms_offline(), common::set_up());
    let visits = vec![VisitBest { exp_per_hour: 7200, ..VisitBest::default() }];
    let unassigned = vec![MapBests { id: "010003070".into(), visits }];
    minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned, characters: Vec::new() });
    minerva.set_personal_bests_size(Size { width: 920, height: 400 });
    minerva.press(PanelControl::TogglePersonalBests);
    let window = minerva.panel_state().personal_bests.placement.expect("the Personal Bests window shows");
    minerva.on_hotkey("ctrl+shift+u");

    let under_window = ScreenRect { x: window.x + 10, y: window.y + 10, width: 170, height: 19 };
    let tooltip = ItemTooltip::Found { area: under_window, name: Some("Orihalcon Ore".into()) };
    let state = minerva.on_item_tooltip_reading(Some(ItemTooltipReading { pointer: POINTER, tooltip, at_ms: 1_000 }));

    assert_eq!(line(state), Some(ItemLookup::Covered { by: OurWindow::PersonalBests }));
}

#[test]
fn a_scrolled_items_upgrade_count_isnt_looked_up_with_its_name() {
    let source = made_up_meowdb();
    let asked = source.item_requests.clone();
    let mut minerva = started_with(source, common::set_up());

    let state = look_up(&mut minerva, "Mithril Guards (+3)", 1_000);

    assert_eq!(*asked.lock().unwrap(), ["Mithril Guards"]);
    assert_eq!(line(&state), opened("Mithril Guards", "501"));
}

#[test]
fn the_line_goes_after_six_seconds() {
    let mut minerva = started();
    look_up(&mut minerva, "Zqxv Nothing", 1_000);

    assert!(line(minerva.tick(6_999)).is_some());
    assert_eq!(line(minerva.tick(7_000)), None);
}

#[test]
fn each_lookup_is_numbered_so_the_same_line_twice_shows_afresh() {
    let mut minerva = started();

    let first = look_up(&mut minerva, "Zqxv Nothing", 1_000).item_lookup.unwrap();
    let second = look_up(&mut minerva, "Zqxv Nothing", 2_000).item_lookup.unwrap();

    assert_eq!(first.outcome, second.outcome);
    assert_ne!(first.lookup, second.lookup);
}

#[test]
fn the_owl_says_it_at_the_top_centre_of_the_game_window() {
    let mut minerva = started();

    let state = look_up(&mut minerva, "Zqxv Nothing", 1_000);

    // Centred on the 1920 px client at x 812, a twentieth of its 1200 px down from y 110.
    assert_eq!(owl_at(&state), Some(ScreenRect { x: 1562, y: 170, width: 420, height: 56 }));
}

#[test]
fn the_owl_says_nothing_before_a_lookup_nor_once_its_line_goes() {
    let mut minerva = started();
    assert_eq!(owl_at(minerva.panel_state()), None);

    look_up(&mut minerva, "Zqxv Nothing", 1_000);

    assert!(owl_at(minerva.tick(6_999)).is_some());
    assert_eq!(owl_at(minerva.tick(7_000)), None);
}

#[test]
fn the_owl_hides_while_another_app_is_in_front_of_the_game_and_shows_again_until_it_goes() {
    let mut minerva = started();
    let shown = owl_at(&look_up(&mut minerva, "Zqxv Nothing", 1_000));
    assert!(shown.is_some());

    // The browser, say, with the page an Item Lookup opened.
    assert_eq!(owl_at(minerva.on_foreground(Foreground::OtherApp)), None);

    assert_eq!(owl_at(minerva.on_foreground(Foreground::Game)), shown);
    minerva.on_foreground(Foreground::OtherApp);
    assert_eq!(owl_at(minerva.on_foreground(Foreground::Minerva)), shown);
    assert_eq!(owl_at(minerva.tick(7_000)), None, "it goes when it would have");
}

#[test]
fn the_owl_goes_below_a_minimap_title_found_where_it_would_be() {
    let mut minerva = started_with(FakeSource::osms_offline(), common::set_up());
    // A minimap title found at the top centre of the client area.
    let region = ScreenRect { x: GAME.x + 800, y: GAME.y + 40, width: 320, height: 48 };
    common::found_title(&mut minerva, region, 0);

    let owl = owl_at(&look_up(&mut minerva, "Zqxv Nothing", 1_000)).expect("the owl");

    assert!(owl.y >= region.y + region.height as i32, "{owl:?} clear of {region:?}");
    assert_eq!(owl.x, 1562, "still centred");
}

#[test]
fn the_owl_is_never_wider_than_the_game_window() {
    let mut minerva = started();
    minerva.set_owl_bubble_size(Size { width: 2400, height: 56 });

    let owl = owl_at(&look_up(&mut minerva, "Zqxv Nothing", 1_000)).expect("the owl");

    assert_eq!((owl.x, owl.width), (GAME.x, GAME.width));
}

#[test]
fn a_collapsed_panel_stays_collapsed_whatever_a_lookup_comes_to() {
    let mut minerva = started();
    minerva.act(Action::ToggleExpanded);

    assert!(!look_up(&mut minerva, "Zqxv Nothing", 1_000).expanded, "the owl says a failure");
    assert!(!look_up(&mut minerva, "Hero's Gladius", 2_000).expanded, "and so does a list to pick from");
    assert!(!look_up(&mut minerva, "Orihalcon Ore", 3_000).expanded);
    assert!(!minerva.settings().expanded);
}

#[test]
fn the_owl_over_the_name_says_so() {
    let mut minerva = started();
    let owl = owl_at(&look_up(&mut minerva, "Zqxv Nothing", 1_000)).expect("the owl");
    minerva.on_hotkey("ctrl+shift+u");

    let under_owl = ScreenRect { x: owl.x + 20, y: owl.y + 10, width: 170, height: 19 };
    let tooltip = ItemTooltip::Found { area: under_owl, name: Some("Orihalcon Ore".into()) };
    let state = minerva.on_item_tooltip_reading(Some(ItemTooltipReading { pointer: POINTER, tooltip, at_ms: 2_000 }));

    assert_eq!(line(state), Some(ItemLookup::Covered { by: OurWindow::OwlBubble }));
}

#[test]
fn the_owl_says_the_lookups_line_waiting_on_meowdb_then_with_news_or_a_problem() {
    let mut minerva = started_with(made_up_meowdb().items_taking(2), common::set_up());

    let state = look_up(&mut minerva, "Orihalcon Ore", 1_000);
    assert_eq!(owl_line(&state), state.item_lookup.clone().map(OwlLine::ItemLookup));
    assert_eq!(owl_mood(&state), Some(OwlMood::Waiting));
    assert!(!owl_clickable(&state), "clicks go through to the game");

    minerva.tick(1_100);
    let state = minerva.tick(1_200).clone();
    assert_eq!(line(&state), opened("Orihalcon Ore", "101"));
    assert_eq!(owl_line(&state), state.item_lookup.clone().map(OwlLine::ItemLookup));
    assert_eq!(owl_mood(&state), Some(OwlMood::News));

    look_up(&mut minerva, "Zqxv Nothing", 5_000);
    minerva.tick(5_100);
    let state = minerva.tick(5_200).clone();
    assert_eq!(line(&state), not_found("Zqxv Nothing"));
    assert_eq!(owl_mood(&state), Some(OwlMood::Problem));
}
