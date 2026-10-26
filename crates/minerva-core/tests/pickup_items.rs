//! Pickups, the Items (GLOSSARY, ADR 0026, #171): the Items the Pickup Log says the player
//! picked up in the Session, named from OSMS's item files ([`common::items`], made up in their
//! shape) and tallied by name. Readings are built by hand and fed in as the reader's public
//! input; the assertions are on the Grind Tracker's `PanelState`.
//!
//! The Session is the EXP Session of `exp_session.rs`: 160 [3.81%] at 0 s, then 266 [6.33%] at
//! 10 s starts it.

mod common;

use common::items::{ItemWorld, equip, etc, item_world, scroll, setup, use_item, weapon};
use common::{FakeSource, S, read, tracking_with};
use minerva_core::{Action, Minerva, PanelControl, PanelState, PickedItem, PickupLine, PickupLogReading, PickupLogSeen, ScreenRect};

/// Where the log was seen, in the bottom right of a 1920x1200 client area at the origin.
const LOG: ScreenRect = ScreenRect { x: 1500, y: 800, width: 408, height: 160 };

/// Made-up item data: a few items of each tab.
fn world() -> ItemWorld {
    item_world(vec![
        etc(4_000_000, "Orange Mushroom Cap").sells_for(5),
        etc(4_000_001, "Blue Snail Shell").sells_for(7),
        use_item(2_000_000, "Red Potion").sells_for(25),
        // Names the game cuts short: one start with one item, one with several, one that a
        // tab decides.
        use_item(2_060_000, "Arrows for Crossbow").sells_for(1),
        use_item(2_061_000, "Arrows for Bow").sells_for(1),
        use_item(2_070_000, "Pet Food Bar").sells_for(1),
        etc(4_031_000, "Pet Food Bar Wrapper").sells_for(1),
        weapon(1_302_000, "Maple Lonely Sword", "1H Sword").sells_for(100),
        equip(1_002_000, "Maple Lonely Hat", "Hat").sells_for(100),
        setup(3_010_000, "Maple Lonely Chair"),
    ])
    .with_scroll(scroll(2_040_000, "Scroll for Maple Lonely Sword for ATT"))
}

/// A Session a few seconds in, with `world`'s item files in.
fn started(world: ItemWorld) -> Minerva {
    let mut minerva = tracking_with(FakeSource::osms().with_items(world));
    minerva.tick(0);
    read(&mut minerva, 0, 160, 381);
    read(&mut minerva, 10 * S, 266, 633);
    minerva
}

/// An Item line as the reader gives it.
fn item(name: &str, quantity: u32, tab: &str) -> PickupLine {
    PickupLine::Item { name: name.into(), quantity, tab: tab.into(), cut_short: false }
}

/// An Item line whose name the game cut short: `name` is without the "...".
fn cut(name: &str, quantity: u32, tab: &str) -> PickupLine {
    PickupLine::Item { name: name.into(), quantity, tab: tab.into(), cut_short: true }
}

/// A clean reading of `lines` arriving at `at_ms`.
fn picked_up(minerva: &mut Minerva, at_ms: u64, lines: Vec<PickupLine>) -> PanelState {
    let reading =
        PickupLogReading { at_ms, seen: PickupLogSeen::Clear { area: LOG }, arrived: lines, chose: false, overflowed: false };
    minerva.on_pickup_log_reading(reading).clone()
}

fn tally(name: &str, count: u64) -> PickedItem {
    PickedItem { name: name.into(), count }
}

#[test]
fn an_item_named_exactly_is_tallied_by_name_and_its_quantities_add_up() {
    let mut minerva = started(world());

    picked_up(&mut minerva, 11 * S, vec![item("Orange Mushroom Cap", 1, "Etc")]);
    let state = picked_up(&mut minerva, 12 * S, vec![item("Orange Mushroom Cap", 26, "Etc"), item("Red Potion", 3, "Use")]);

    let pickups = state.exp_tracking.pickups;
    assert_eq!(pickups.items, vec![tally("Orange Mushroom Cap", 27), tally("Red Potion", 3)]);
    assert_eq!(pickups.items_total, 30);
}

#[test]
fn a_misread_name_close_enough_to_an_items_is_tallied_as_that_item() {
    let mut minerva = started(world());

    // A zero for the O, a capital I for the l: the letters the recogniser confuses.
    let state = picked_up(&mut minerva, 11 * S, vec![item("Orange Mushr0om Cap", 1, "Etc"), item("Red PotIon", 2, "Use")]);

    let items = state.exp_tracking.pickups.items;
    assert_eq!(items, vec![tally("Red Potion", 2), tally("Orange Mushroom Cap", 1)]);
}

#[test]
fn a_name_cut_short_that_starts_one_items_name_is_that_item() {
    let mut minerva = started(world());

    let state = picked_up(&mut minerva, 11 * S, vec![cut("Arrows for Cros", 21, "Use")]);

    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Arrows for Crossbow", 21)]);
}

#[test]
fn a_name_cut_short_that_starts_several_items_names_is_tallied_as_the_game_shows_it() {
    let mut minerva = started(world());

    picked_up(&mut minerva, 11 * S, vec![cut("Arrows for", 21, "Use")]);
    let state = picked_up(&mut minerva, 12 * S, vec![cut("Arrows for", 4, "Use")]);

    // Never a guess between Arrows for Bow and Arrows for Crossbow, never dropped.
    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Arrows for...", 25)]);
}

#[test]
fn a_tab_decides_between_two_items_a_name_cut_short_starts() {
    let mut minerva = started(world());

    // Pet Food Bar is a Use item, Pet Food Bar Wrapper an Etc one.
    let state = picked_up(&mut minerva, 11 * S, vec![cut("Pet Food Bar", 2, "Use"), cut("Pet Food Bar", 5, "Etc")]);

    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Pet Food Bar Wrapper", 5), tally("Pet Food Bar", 2)]);
}

#[test]
fn equipment_and_scrolls_are_filed_under_the_tabs_the_game_files_them_under() {
    let mut minerva = started(world());

    let state = picked_up(
        &mut minerva,
        11 * S,
        vec![
            cut("Maple Lonely S", 1, "Equip"),
            cut("Maple Lonely H", 1, "Equip"),
            cut("Maple Lonely C", 1, "Setup"),
            cut("Scroll for Maple Lonely S", 1, "Use"),
        ],
    );

    let names: Vec<_> = state.exp_tracking.pickups.items.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, ["Maple Lonely Chair", "Maple Lonely Hat", "Maple Lonely Sword", "Scroll for Maple Lonely Sword for ATT"]);
}

#[test]
fn a_name_cut_short_in_a_tab_nothing_starting_so_is_filed_under_matches_no_item() {
    let mut minerva = started(world());

    // Arrows are Use items: the tab read says Etc, so it is a misread or something newer.
    let state = picked_up(&mut minerva, 11 * S, vec![cut("Arrows for Cros", 21, "Etc")]);
    assert!(state.exp_tracking.pickups.items.is_empty());

    let state = picked_up(&mut minerva, 12 * S, vec![cut("Arrows for Cros", 21, "Etc")]);
    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Arrows for Cros...", 42)]);
}

#[test]
fn a_name_matching_no_item_is_tallied_as_read_once_it_has_been_read_on_two_lines() {
    let mut minerva = started(world());

    let state = picked_up(&mut minerva, 11 * S, vec![item("Zorg Dung", 3, "Etc")]);
    assert!(state.exp_tracking.pickups.items.is_empty(), "a single misread makes no Item of its own");
    let state = picked_up(&mut minerva, 12 * S, vec![item("Zorb Dung", 1, "Etc")]);
    assert!(state.exp_tracking.pickups.items.is_empty(), "another text is another misread");

    // The second line of the same text tallies both, the quantity of the first not lost.
    let state = picked_up(&mut minerva, 13 * S, vec![item("Zorg Dung", 2, "Etc")]);
    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Zorg Dung", 5)]);

    let state = picked_up(&mut minerva, 14 * S, vec![item("Zorg Dung", 1, "Etc")]);
    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Zorg Dung", 6)]);
}

#[test]
fn two_lines_of_one_unmatched_name_in_one_reading_are_two_lines() {
    let mut minerva = started(world());

    let state = picked_up(&mut minerva, 11 * S, vec![item("Zorg Dung", 3, "Etc"), item("Zorg Dung", 1, "Etc")]);

    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Zorg Dung", 4)]);
}

#[test]
fn without_item_files_a_name_is_tallied_once_read_on_two_lines() {
    let mut minerva = tracking_with(FakeSource::osms_offline());
    read(&mut minerva, 0, 160, 381);
    read(&mut minerva, 10 * S, 266, 633);

    picked_up(&mut minerva, 11 * S, vec![item("Red Potion", 1, "Use")]);
    let state = picked_up(&mut minerva, 12 * S, vec![item("Red Potion", 1, "Use")]);

    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Red Potion", 2)]);
}

#[test]
fn only_the_lines_of_a_clean_capture_in_a_started_session_count() {
    let mut minerva = tracking_with(FakeSource::osms().with_items(world()));
    minerva.tick(0);
    read(&mut minerva, 0, 160, 381);
    // Before the first EXP change the Session hasn't started.
    picked_up(&mut minerva, 5 * S, vec![item("Red Potion", 9, "Use")]);
    read(&mut minerva, 10 * S, 266, 633);

    let covered = PickupLogReading {
        at_ms: 11 * S,
        seen: PickupLogSeen::Covered { area: LOG },
        arrived: vec![item("Red Potion", 9, "Use")],
        chose: false,
        overflowed: false,
    };
    minerva.on_pickup_log_reading(covered);
    let state = picked_up(&mut minerva, 12 * S, vec![item("Red Potion", 1, "Use")]);

    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Red Potion", 1)]);
    assert_eq!(state.exp_tracking.pickups.items_total, 1);
}

#[test]
fn a_reset_clears_the_items_with_the_mesos_and_its_undo_brings_them_back() {
    let mut minerva = started(world());
    picked_up(&mut minerva, 11 * S, vec![item("Red Potion", 4, "Use"), item("Zorg Dung", 1, "Etc")]);

    let state = minerva.act(Action::ResetSession).clone();
    assert!(state.exp_tracking.pickups.items.is_empty());
    assert_eq!(state.exp_tracking.pickups.items_total, 0);

    let state = minerva.act(Action::UndoReset).clone();
    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Red Potion", 4)]);

    // The misread seen once came back with the Session: a second line tallies it.
    let state = picked_up(&mut minerva, 12 * S, vec![item("Zorg Dung", 1, "Etc")]);
    assert_eq!(state.exp_tracking.pickups.items, vec![tally("Red Potion", 4), tally("Zorg Dung", 2)]);
}

#[test]
fn the_pickups_header_starts_collapsed_and_a_click_opens_and_closes_it_across_launches() {
    let mut minerva = started(world());
    assert!(!minerva.panel_state().grind_tracker.pickups_open);
    assert!(!minerva.settings().pickups_open);

    minerva.press(PanelControl::TogglePickups);
    assert!(minerva.panel_state().grind_tracker.pickups_open);

    // The next launch finds it as the player left it.
    let saved = minerva.settings();
    assert!(saved.pickups_open);
    let mut next = tracking_with(FakeSource::osms());
    assert!(next.restore(saved).grind_tracker.pickups_open);

    minerva.press(PanelControl::TogglePickups);
    assert!(!minerva.panel_state().grind_tracker.pickups_open);
    assert!(!minerva.settings().pickups_open);
}
