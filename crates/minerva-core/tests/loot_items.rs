//! Loot, the Items (GLOSSARY, ADR 0026, #171): the Items the Pickup Log says the player
//! picked up in the Session, named from OSMS's item files ([`common::items`], made up in their
//! shape) and tallied by name. Readings are built by hand and fed in as the reader's public
//! input; the assertions are on the Grind Tracker's `PanelState`.
//!
//! The Session is the EXP Session of `exp_session.rs`: 160 [3.81%] at 0 s, then 266 [6.33%] at
//! 10 s starts it.

mod common;

use common::items::{ItemWorld, equip, etc, item_world, scroll, setup, use_item, weapon};
use common::{FakeSource, S, tracking_with};
use minerva_core::{Action, Minerva, PanelControl, LootItem, PickupLine, PickupLogSeen};

use common::{LOG, picked_up, read};

/// Made-up item data: a few items of each tab.
fn world() -> ItemWorld {
    item_world(vec![
        etc(4_000_000, "Orange Mushroom Cap").sells_for(5),
        etc(4_000_001, "Blue Snail Shell").sells_for(7),
        etc(4_000_002, "Squishy Liquid").sells_for(1),
        etc(4_000_003, "Slime Bubble").sells_for(1),
        // Two names a misread can't tell apart.
        etc(4_000_004, "Red Slime Orb").sells_for(1),
        etc(4_000_005, "Red Slime Orc").sells_for(1),
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
    common::start_session(&mut minerva, 0, 10 * S);
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

fn tally(name: &str, count: u64) -> LootItem {
    LootItem { name: name.into(), count }
}

#[test]
fn an_item_named_exactly_is_tallied_by_name_and_its_quantities_add_up() {
    let mut minerva = started(world());

    picked_up(&mut minerva, 11 * S, vec![item("Orange Mushroom Cap", 1, "Etc")]);
    let state = picked_up(&mut minerva, 12 * S, vec![item("Orange Mushroom Cap", 26, "Etc"), item("Red Potion", 3, "Use")]);

    let loot = state.exp_tracking.loot;
    assert_eq!(loot.items, vec![tally("Orange Mushroom Cap", 27), tally("Red Potion", 3)]);
    assert_eq!(loot.items_total, 30);
}

#[test]
fn a_misread_name_close_enough_to_an_items_is_tallied_as_that_item() {
    let mut minerva = started(world());

    // A zero for the O, a capital I for the l: the letters the recogniser confuses.
    let state = picked_up(&mut minerva, 11 * S, vec![item("Orange Mushr0om Cap", 1, "Etc"), item("Red PotIon", 2, "Use")]);

    let items = state.exp_tracking.loot.items;
    assert_eq!(items, vec![tally("Red Potion", 2), tally("Orange Mushroom Cap", 1)]);
}

#[test]
fn a_name_read_several_glyphs_wrong_is_the_one_item_of_its_tab_it_is_clearly_closest_to() {
    let mut minerva = started(world());

    // Bug Report 0.15.0, 2026-10-26 09:03, over bright green: the S read as an E, a glyph lost,
    // the b as a k. Too far from the name for the whole set to say, but of the Etc tab's names
    // only Squishy Liquid and Slime Bubble are near.
    let state = picked_up(&mut minerva, 11 * S, vec![item("Equisny Liq?id", 1, "Etc"), item("Slime Bubkile", 2, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Slime Bubble", 2), tally("Squishy Liquid", 1)]);
}

#[test]
fn a_misread_name_is_never_a_guess_between_two_items_of_its_tab() {
    let mut minerva = started(world());

    // One glyph from Red Slime Orb and from Red Slime Orc: tallied as read, once read twice.
    let state = picked_up(&mut minerva, 11 * S, vec![item("Red Slime Orr", 1, "Etc")]);
    assert!(state.exp_tracking.loot.items.is_empty());
    let state = picked_up(&mut minerva, 12 * S, vec![item("Red Slime Orr", 1, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Red Slime Orr", 2)]);
}

#[test]
fn a_misread_name_is_only_matched_among_the_items_of_the_tab_its_line_says() {
    let mut minerva = started(world());

    // Squishy Liquid is an Etc item: filed under Use, this is a misread or something newer.
    let state = picked_up(&mut minerva, 11 * S, vec![item("Equisny Liq?id", 1, "Use")]);

    assert!(state.exp_tracking.loot.items.is_empty());
}

#[test]
fn a_misread_name_close_to_an_item_of_another_tab_is_not_that_item() {
    let mut minerva = started(world());

    // A glyph from Blue Snail Shell, close enough for the whole set to name it, but filed under
    // Use: Blue Snail Shell is an Etc item, so it isn't what was picked up. An exact name is still
    // the item whatever the tab read (the next line), as the tab may be the misread.
    let state = picked_up(&mut minerva, 11 * S, vec![item("Blue Snail Shel", 1, "Use"), item("Red Potion", 1, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Red Potion", 1)]);
}

#[test]
fn a_name_cut_short_that_starts_one_items_name_is_that_item() {
    let mut minerva = started(world());

    let state = picked_up(&mut minerva, 11 * S, vec![cut("Arrows for Cros", 21, "Use")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Arrows for Crossbow", 21)]);
}

#[test]
fn a_name_cut_short_that_starts_several_items_names_is_tallied_as_the_game_shows_it() {
    let mut minerva = started(world());

    picked_up(&mut minerva, 11 * S, vec![cut("Arrows for", 21, "Use")]);
    let state = picked_up(&mut minerva, 12 * S, vec![cut("Arrows for", 4, "Use")]);

    // Never a guess between Arrows for Bow and Arrows for Crossbow, never dropped.
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Arrows for...", 25)]);
}

#[test]
fn a_tab_decides_between_two_items_a_name_cut_short_starts() {
    let mut minerva = started(world());

    // Pet Food Bar is a Use item, Pet Food Bar Wrapper an Etc one.
    let state = picked_up(&mut minerva, 11 * S, vec![cut("Pet Food Bar", 2, "Use"), cut("Pet Food Bar", 5, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Pet Food Bar Wrapper", 5), tally("Pet Food Bar", 2)]);
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

    let names: Vec<_> = state.exp_tracking.loot.items.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, ["Maple Lonely Chair", "Maple Lonely Hat", "Maple Lonely Sword", "Scroll for Maple Lonely Sword for ATT"]);
}

#[test]
fn a_name_cut_short_in_a_tab_nothing_starting_so_is_filed_under_matches_no_item() {
    let mut minerva = started(world());

    // Arrows are Use items: the tab read says Etc, so it is a misread or something newer.
    let state = picked_up(&mut minerva, 11 * S, vec![cut("Arrows for Cros", 21, "Etc")]);
    assert!(state.exp_tracking.loot.items.is_empty());

    let state = picked_up(&mut minerva, 12 * S, vec![cut("Arrows for Cros", 21, "Etc")]);
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Arrows for Cros...", 42)]);
}

#[test]
fn a_name_matching_no_item_is_tallied_as_read_once_it_has_been_read_on_two_lines() {
    let mut minerva = started(world());

    let state = picked_up(&mut minerva, 11 * S, vec![item("Zorg Dung", 3, "Etc")]);
    assert!(state.exp_tracking.loot.items.is_empty(), "a single misread makes no Item of its own");
    let state = picked_up(&mut minerva, 12 * S, vec![item("Zorb Dung", 1, "Etc")]);
    assert!(state.exp_tracking.loot.items.is_empty(), "another text is another misread");

    // The second line of the same text tallies both, the quantity of the first not lost.
    let state = picked_up(&mut minerva, 13 * S, vec![item("Zorg Dung", 2, "Etc")]);
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Zorg Dung", 5)]);

    let state = picked_up(&mut minerva, 14 * S, vec![item("Zorg Dung", 1, "Etc")]);
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Zorg Dung", 6)]);
}

#[test]
fn two_lines_of_one_unmatched_name_in_one_reading_are_two_lines() {
    let mut minerva = started(world());

    let state = picked_up(&mut minerva, 11 * S, vec![item("Zorg Dung", 3, "Etc"), item("Zorg Dung", 1, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Zorg Dung", 4)]);
}

#[test]
fn without_item_files_a_name_is_tallied_once_read_on_two_lines() {
    let mut minerva = tracking_with(FakeSource::osms_offline());
    common::start_session(&mut minerva, 0, 10 * S);

    picked_up(&mut minerva, 11 * S, vec![item("Red Potion", 1, "Use")]);
    let state = picked_up(&mut minerva, 12 * S, vec![item("Red Potion", 1, "Use")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Red Potion", 2)]);
}

#[test]
fn only_the_lines_of_a_clean_capture_in_a_started_session_count() {
    let mut minerva = tracking_with(FakeSource::osms().with_items(world()));
    minerva.tick(0);
    read(&mut minerva, 0, 160, 381);
    // Before the first EXP change the Session hasn't started.
    picked_up(&mut minerva, 5 * S, vec![item("Red Potion", 9, "Use")]);
    read(&mut minerva, 10 * S, 266, 633);

    let covered = common::pickup_reading(11 * S, PickupLogSeen::Covered { area: LOG }, vec![item("Red Potion", 9, "Use")]);
    minerva.on_pickup_log_reading(covered);
    let state = picked_up(&mut minerva, 12 * S, vec![item("Red Potion", 1, "Use")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Red Potion", 1)]);
    assert_eq!(state.exp_tracking.loot.items_total, 1);
}

#[test]
fn a_reset_clears_the_items_with_the_mesos_and_its_undo_brings_them_back() {
    let mut minerva = started(world());
    picked_up(&mut minerva, 11 * S, vec![item("Red Potion", 4, "Use"), item("Zorg Dung", 1, "Etc")]);

    let state = minerva.act(Action::ResetSession).clone();
    assert!(state.exp_tracking.loot.items.is_empty());
    assert_eq!(state.exp_tracking.loot.items_total, 0);

    let state = minerva.act(Action::UndoReset).clone();
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Red Potion", 4)]);

    // The misread seen once came back with the Session: a second line tallies it.
    let state = picked_up(&mut minerva, 12 * S, vec![item("Zorg Dung", 1, "Etc")]);
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Red Potion", 4), tally("Zorg Dung", 2)]);
}

#[test]
fn the_loot_header_starts_collapsed_and_a_click_opens_and_closes_it_across_launches() {
    let mut minerva = started(world());
    assert!(!minerva.panel_state().grind_tracker.loot_open);
    assert!(!minerva.settings().loot_open);

    minerva.press(PanelControl::ToggleLoot);
    assert!(minerva.panel_state().grind_tracker.loot_open);

    // The next launch finds it as the player left it.
    let saved = minerva.settings();
    assert!(saved.loot_open);
    let mut next = tracking_with(FakeSource::osms());
    assert!(next.restore(saved).grind_tracker.loot_open);

    minerva.press(PanelControl::ToggleLoot);
    assert!(!minerva.panel_state().grind_tracker.loot_open);
    assert!(!minerva.settings().loot_open);
}

/// The Etc tab of a map where a quest hands out Very Dry Branches and Tree Branches drop, with
/// a pair of names one letter apart.
fn branches() -> ItemWorld {
    item_world(vec![
        etc(4_000_100, "Very Dry Branch").sells_for(1),
        etc(4_000_101, "Tree Branch").sells_for(1),
        etc(4_000_102, "Pine Cone").sells_for(1),
        etc(4_000_103, "Fine Cone").sells_for(1),
        use_item(2_000_100, "Red Potion").sells_for(25),
    ])
}

#[test]
fn a_name_misread_the_games_usual_ways_is_the_item_it_is_closest_to_on_its_first_line() {
    // Bug Report 0.15.0, 2026-10-26 23:05: 100 Very Dry Branches picked up, 57 lines read as one,
    // 10 of them as names no Item has. The D read as Ci, Li or [i, the B as Bl or El, the V
    // lost or read as ( * " \ M W, the y as v, a glyph lost as a "?".
    let misreads = [
        "Very Cir Aranch",
        "(ery [Ciry Elranch",
        "Very Ciry Elrarich",
        "Mery Ciry Blranch",
        "Very Liry Blraneh",
        "Very Cir: Blranch",
        "verv [\"? Branch.",
        "veryDr? Brarch",
    ];
    for misread in misreads {
        let mut minerva = started(branches());
        let state = picked_up(&mut minerva, 11 * S, vec![item(misread, 1, "Etc")]);
        assert_eq!(state.exp_tracking.loot.items, vec![tally("Very Dry Branch", 1)], "{misread:?}");
    }
}

#[test]
fn a_misread_nearer_the_other_of_two_alike_items_is_that_one() {
    let mut minerva = started(branches());

    let state = picked_up(&mut minerva, 11 * S, vec![item("Tie? Branih", 1, "Etc"), item("Tree Eranch", 2, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Tree Branch", 3)]);
}

#[test]
fn an_exact_name_is_its_item_whatever_the_slips_would_make_of_it() {
    let mut minerva = started(branches());

    let state = picked_up(&mut minerva, 11 * S, vec![item("Tree Branch", 1, "Etc"), item("Very Dry Branch", 2, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("Very Dry Branch", 2), tally("Tree Branch", 1)]);
}

#[test]
fn a_read_equally_close_to_two_items_stays_as_read() {
    let mut minerva = started(branches());

    // A "?" is any one letter: P or F.
    let state = picked_up(&mut minerva, 11 * S, vec![item("?ine Cone", 1, "Etc")]);
    assert!(state.exp_tracking.loot.items.is_empty());
    let state = picked_up(&mut minerva, 12 * S, vec![item("?ine Cone", 1, "Etc")]);

    assert_eq!(state.exp_tracking.loot.items, vec![tally("?ine Cone", 2)]);
}

#[test]
fn a_misread_is_never_the_name_of_an_item_of_another_tab() {
    let mut minerva = started(branches());

    // Very Dry Branch is an Etc item; this line says Use.
    let state = picked_up(&mut minerva, 11 * S, vec![item("Very Ciry Elrarich", 1, "Use")]);

    assert!(state.exp_tracking.loot.items.is_empty());
}

#[test]
fn a_read_too_far_from_every_item_for_its_slips_to_explain_stays_as_read() {
    let mut minerva = started(branches());

    let state = picked_up(&mut minerva, 11 * S, vec![item("Cran Brach", 1, "Etc")]);

    assert!(state.exp_tracking.loot.items.is_empty());
}

/// What one Etc line `read` is tallied as on its first line, in a Session with `names` in the Etc
/// tab: an Item's name, or nothing yet (a name that matches no Item waits for a second line).
fn tallied_first(names: &[&str], read: &str) -> Vec<LootItem> {
    let items = names.iter().enumerate().map(|(i, name)| etc(4_100_000 + i as u64, name).sells_for(1)).collect();
    let mut minerva = started(item_world(items));
    picked_up(&mut minerva, 11 * S, vec![item(read, 1, "Etc")]).exp_tracking.loot.items
}

#[test]
fn a_misread_a_whole_edit_nearer_an_item_than_the_next_is_that_item() {
    // The D read as "Ci" is a slip (0.4 of an edit); the other name is a letter further from it than
    // that, by one whole edit and some. Plain edits alone see them 2 and 3 apart, not 2 clear.
    let names = ["Very Dry Branch", "Very Dry Brunch"];

    assert_eq!(tallied_first(&names, "Very Ciry Branch"), vec![tally("Very Dry Branch", 1)]);
}

#[test]
fn a_misread_less_than_a_whole_edit_nearer_an_item_than_the_next_stays_as_read() {
    // The B read as "A" is a slip; the other name is one plain edit from the read (0.6 further).
    let names = ["Very Dry Branch", "Very Dry Oranch"];

    assert_eq!(tallied_first(&names, "Very Dry Aranch"), vec![]);
}

#[test]
fn a_misread_needs_the_next_item_to_be_half_as_far_again_as_the_closest() {
    // The read is two plain edits and a "?" (2.4) from the first name. A name a whole edit
    // further (3.4) is not half as far again; one 2 further (4.4) is.
    assert_eq!(tallied_first(&["Wooden Shield Frame", "Wooden Shield Flame"], "Woodan Shi?ld Framo"), vec![]);
    assert_eq!(tallied_first(&["Wooden Shield Frame", "Wooden Shield Flaze"], "Woodan Shi?ld Framo"), vec![tally("Wooden Shield Frame", 1)]);
}

#[test]
fn a_slip_seen_in_one_read_only_is_not_one() {
    // Each is a letter read as another in one distinct read of the Bug Report of 0.15.0, 2026-10-26
    // 23:05, and in no second one; on names this short a plain edit is too far to name.
    for (name, read) in [("Orb", "Crb"), ("Orb", "Drb"), ("Orb", "Prb"), ("Sap", "Eap"), ("Cab", "Eab"), ("Cab", "Iab"), ("Bat", "Iat"), ("Nag", "Rag")] {
        assert_eq!(tallied_first(&[name], read), vec![], "{read:?} for {name:?}");
    }
}

#[test]
fn plain_edits_name_a_misread_only_as_the_item_the_slips_put_closest() {
    // Three D's read as "Ci" are three slips (1.2) from the first name, which plain edits see as 6
    // away; the second is two plain edits (2.0) from the read, so the slips don't lead by a whole
    // edit, and plain edits (within 75%, but not the 85% of the whole set) name the second.
    let names = ["Dad Dug", "Ciaci Cinx"];

    assert_eq!(tallied_first(&names, "Ciaci Ciug"), vec![]);
    // With no such name beside it, the same read is the first name: nothing else is as near.
    assert_eq!(tallied_first(&names[..1], "Ciaci Ciug"), vec![tally("Dad Dug", 1)]);
}

/// Made-up Items whose names the game cuts short, a few of them alike at the start.
fn candies() -> ItemWorld {
    item_world(vec![
        etc(4_000_200, "Halloween Candies").sells_for(1),
        etc(4_000_201, "Halloween Teddy Bear Ear").sells_for(1),
        setup(3_010_200, "Halloween Broomstick Chair"),
        etc(4_000_202, "Red Slime Orb").sells_for(1),
        etc(4_000_203, "Red Slime Orc").sells_for(1),
    ])
}

#[test]
fn a_name_cut_short_and_misread_is_the_item_whose_start_it_is_clearly_closest_to() {
    // Bug Report 0.15.0, 2026-10-27 18:29: "Halloween Candies" is cut to "Halloween Can...", and
    // 25 of its 53 lines read the cut's last letters wrong ("Halloween Car", "Hailoween Car").
    // Plain prefixes start no Item, so they were tallied as read, under three names.
    let mut minerva = started(candies());

    let state = picked_up(
        &mut minerva,
        11 * S,
        vec![cut("Halloween Can", 1, "Etc"), cut("Halloween Car", 1, "Etc"), cut("Hailoween Car", 1, "Etc")],
    );
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Halloween Candies", 3)]);

    // The game's "..." read as ".." or ".:" is still a name cut short.
    let state = picked_up(&mut minerva, 12 * S, vec![item("Halloween Car..", 1, "Etc"), item("Halloween Can.:", 1, "Etc")]);
    assert_eq!(state.exp_tracking.loot.items, vec![tally("Halloween Candies", 5)]);
}

#[test]
fn a_name_cut_short_and_misread_as_near_two_items_starts_stays_as_read() {
    let mut minerva = started(candies());

    // As near the start of Red Slime Orb as of Red Slime Orc: no Item until read twice.
    let state = picked_up(&mut minerva, 11 * S, vec![cut("Red Sxime Or", 1, "Etc")]);

    assert!(state.exp_tracking.loot.items.is_empty());
}
