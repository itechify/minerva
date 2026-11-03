//! Map Loot (GLOSSARY, ADR 0033, #225): when a Visit ends, a Character shown on the Map
//! Leaderboards sends its Loot, for the map's page to work out mesos and Items an hour from. Only
//! a Visit worth counting goes: on a map with a Map Summary, with 10 minutes of active play, some
//! mesos, and the Pickup Log readable for nine tenths of it. The shell sends it and tells the core
//! each answer; what waits to go is kept between launches.
//!
//! The player hunts as in `personal_best.rs` ([`Player`]), the EXP bar read every 10 s, with a
//! Pickup Log reading every second and the lines a test gives picked up at the end of each 10 s.

mod common;

use common::items::{ItemWorld, etc, item_world, use_item};
use common::player::Player;
use common::{CONSTRUCTION_SAMPLE, CONSTRUCTION_SITE_ID, FakeSource, LOG, MIN, S, SUNSET_SAMPLE, SUNSET_SKY_ID, pickup_reading, tracking_with};
use minerva_core::{Action, MapLootAnswer, MapLootItem, MapLootUpload, PickupLine, PickupLogSeen};

/// Made-up item files: two Items that share a name, as some of the game's do, and two whose
/// names start alike.
fn world() -> ItemWorld {
    item_world(vec![
        etc(4_000_000, "Orange Mushroom Cap").sells_for(5),
        etc(4_000_001, "Blue Snail Shell").sells_for(7),
        etc(4_000_002, "Blue Snail Stamp").sells_for(7),
        use_item(2_000_000, "Red Potion").sells_for(25),
        use_item(2_000_007, "Red Potion").sells_for(25),
    ])
}

/// Egbert, a Magician shown on the Map Leaderboards, hunting with the wall clock told and the
/// item files in.
fn egbert() -> Player {
    let mut minerva = tracking_with(FakeSource::osms().with_items(world()));
    // The item files come in on a tick.
    minerva.tick(0);
    let mut player = Player::clocked(minerva);
    player.status(Some("Egbert"), 23);
    player.minerva.show_on_leaderboards("Egbert", true);
    player.hunt(30 * S, 10);
    player
}

fn mesos(amount: u64) -> PickupLine {
    PickupLine::Mesos { amount }
}

fn item(name: &str, quantity: u32, tab: &str) -> PickupLine {
    PickupLine::Item { name: name.into(), quantity, tab: tab.into(), cut_short: false }
}

/// Hunt for `ms`, the Pickup Log read as `seen` every second, and `lines` picked up at the end of
/// each 10 s.
fn hunt_seen(player: &mut Player, ms: u64, seen: PickupLogSeen, lines: &[PickupLine]) {
    for _ in 0..ms / (10 * S) {
        for second in 1..10 {
            let reading = pickup_reading(player.at + second * S, seen, Vec::new());
            player.minerva.on_pickup_log_reading(reading);
        }
        let reading = pickup_reading(player.at + 9 * S + S / 2, seen, lines.to_vec());
        player.minerva.on_pickup_log_reading(reading);
        player.hunt(10 * S, 10);
    }
}

/// Hunt for `ms` with the Pickup Log read clean throughout.
fn hunt_looting(player: &mut Player, ms: u64, lines: &[PickupLine]) {
    hunt_seen(player, ms, PickupLogSeen::Clear { area: LOG }, lines);
}

#[test]
fn a_visit_of_ten_minutes_with_mesos_picked_up_sends_its_map_loot_when_it_ends() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 10 * MIN, &[mesos(50), item("Orange Mushroom Cap", 2, "Etc")]);
    assert_eq!(player.minerva.panel_state().map_loot_uploads, [], "nothing while the Visit is under way");

    let state = player.arrive(CONSTRUCTION_SAMPLE);

    let upload = MapLootUpload {
        id: 1,
        name: "Egbert".into(),
        map: SUNSET_SKY_ID.into(),
        party: false,
        active_ms: 10 * MIN,
        mesos: 60 * 50,
        items: vec![MapLootItem { id: 4_000_000, count: 60 * 2 }],
    };
    assert_eq!(state.map_loot_uploads, [upload]);
}

/// The Map Loot upload under way, if one is.
fn upload(player: &Player) -> Option<MapLootUpload> {
    player.minerva.panel_state().map_loot_uploads.first().cloned()
}

#[test]
fn a_visit_short_of_ten_minutes_or_with_no_mesos_sends_nothing() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 9 * MIN + 50 * S, &[mesos(50)]);
    player.arrive(CONSTRUCTION_SAMPLE);
    assert_eq!(upload(&player), None, "short of 10 minutes");

    hunt_looting(&mut player, 20 * MIN, &[item("Orange Mushroom Cap", 2, "Etc")]);
    player.arrive(SUNSET_SAMPLE);
    assert_eq!(upload(&player), None, "no mesos: the drops went unpicked");
}

#[test]
fn a_visit_whose_pickup_log_was_covered_for_more_than_a_tenth_of_it_sends_nothing() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 9 * MIN, &[mesos(50)]);
    hunt_seen(&mut player, 2 * MIN, PickupLogSeen::Covered { area: LOG }, &[]);
    player.arrive(CONSTRUCTION_SAMPLE);
    assert_eq!(upload(&player), None, "covered for 2 of 11 minutes");

    // Covered for a minute of 11 is under a tenth; a log with no lines in it, between pickups,
    // is readable.
    hunt_looting(&mut player, 9 * MIN, &[mesos(50)]);
    hunt_seen(&mut player, MIN, PickupLogSeen::Covered { area: LOG }, &[]);
    hunt_seen(&mut player, MIN, PickupLogSeen::Nothing, &[]);
    player.arrive(SUNSET_SAMPLE);
    assert_eq!(upload(&player).map(|upload| upload.mesos), Some(54 * 50));
}

#[test]
fn time_the_pickup_log_went_unread_is_not_readable_time() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 8 * MIN, &[mesos(50)]);
    // The Grind Tracker hidden, say: the log isn't read at all.
    player.hunt(2 * MIN, 10);
    player.arrive(CONSTRUCTION_SAMPLE);

    assert_eq!(upload(&player), None);
}

#[test]
fn a_visit_with_the_party_bonus_in_it_is_a_party_one() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 5 * MIN, &[mesos(50)]);
    hunt_looting(&mut player, 5 * MIN, &[mesos(50), PickupLine::PartyBonus { amount: 3 }]);
    player.arrive(CONSTRUCTION_SAMPLE);

    assert_eq!(upload(&player).map(|upload| upload.party), Some(true));
}

#[test]
fn only_real_items_are_sent_each_by_the_lowest_id_its_name_has() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    let lines = [
        mesos(50),
        item("Red Potion", 1, "Use"),
        // Matches no Item: never tallied.
        item("Zorg Dung", 4, "Etc"),
        // Cut short and starting two Items' names: the Grind Tracker tallies it as shown, but
        // it can't be named.
        PickupLine::Item { name: "Blue Snail".into(), quantity: 1, tab: "Etc".into(), cut_short: true },
    ];
    hunt_looting(&mut player, 10 * MIN, &lines);
    let tallied: Vec<_> = player.minerva.panel_state().exp_tracking.loot.items.iter().map(|item| item.name.clone()).collect();
    player.arrive(CONSTRUCTION_SAMPLE);

    assert_eq!(tallied, ["Blue Snail...", "Red Potion"]);
    assert_eq!(upload(&player).map(|upload| upload.items), Some(vec![MapLootItem { id: 2_000_000, count: 60 }]));
}

#[test]
fn a_reset_on_the_visit_keeps_its_loot_and_undoing_it_counts_the_time_once() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 5 * MIN, &[mesos(50)]);
    player.minerva.act(Action::ResetSession);
    player.minerva.act(Action::UndoReset);
    hunt_looting(&mut player, 3 * MIN, &[mesos(50)]);
    player.minerva.act(Action::ResetSession);
    hunt_looting(&mut player, 3 * MIN, &[mesos(50)]);
    player.arrive(CONSTRUCTION_SAMPLE);

    let sent = upload(&player).expect("sent");
    // The first EXP change after the second reset starts the new Session: its play counts from
    // then, 10 s in, and the mesos picked up before it don't count.
    assert_eq!((sent.active_ms, sent.mesos), (11 * MIN - 10 * S, 65 * 50));
}

#[test]
fn another_character_named_ends_the_loot_of_the_one_before() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 10 * MIN, &[mesos(50)]);
    player.status(Some("Bob"), 40);
    hunt_looting(&mut player, MIN, &[mesos(50)]);

    assert_eq!(upload(&player).map(|upload| (upload.name, upload.map)), Some(("Egbert".into(), SUNSET_SKY_ID.into())));
}

#[test]
fn a_character_not_shown_on_the_map_leaderboards_sends_nothing() {
    let mut player = egbert();
    player.minerva.show_on_leaderboards("Egbert", false);
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 10 * MIN, &[mesos(50)]);
    player.arrive(CONSTRUCTION_SAMPLE);

    assert_eq!(upload(&player), None);
}

#[test]
fn taken_it_is_done_refused_it_is_dropped_and_failed_it_is_tried_again_later() {
    let mut player = egbert();
    for map in [SUNSET_SAMPLE, CONSTRUCTION_SAMPLE, SUNSET_SAMPLE] {
        player.arrive(map);
        hunt_looting(&mut player, 10 * MIN, &[mesos(50)]);
    }
    player.arrive(CONSTRUCTION_SAMPLE);

    // One at a time, oldest first.
    let first = upload(&player).expect("the first");
    assert_eq!(player.minerva.panel_state().map_loot_uploads.len(), 1);
    player.minerva.on_map_loot_answer(first.id, MapLootAnswer::Failed { why: "offline".into() });
    assert_eq!(upload(&player), None, "it waits");
    player.minerva.tick(player.at + 31 * S);
    let again = upload(&player).expect("tried again");
    assert_eq!((&again.map, again.active_ms), (&first.map, first.active_ms));

    player.minerva.on_map_loot_answer(again.id, MapLootAnswer::Taken {});
    let second = upload(&player).expect("the next");
    assert_eq!(second.map, CONSTRUCTION_SITE_ID);
    player.minerva.on_map_loot_answer(second.id, MapLootAnswer::Refused { why: "HTTP 400".into() });
    let third = upload(&player).expect("the last");
    assert_eq!(third.map, SUNSET_SKY_ID);
    player.minerva.on_map_loot_answer(third.id, MapLootAnswer::Taken {});

    assert_eq!(upload(&player), None);
    assert_eq!(player.minerva.map_loot().visits, []);
}

#[test]
fn what_waits_and_the_visit_under_way_are_saved_and_sent_at_the_next_launch() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    let revision = player.minerva.map_loot_revision();
    hunt_looting(&mut player, 10 * MIN, &[mesos(50)]);
    assert_ne!(player.minerva.map_loot_revision(), revision, "worth sending, it's saved");
    let saved = player.minerva.map_loot();
    assert_eq!(saved.visits.len(), 1);

    // Closed now, which ends the Visit; at the next launch it goes.
    let mut minerva = tracking_with(FakeSource::osms().with_items(world()));
    minerva.restore(player.minerva.settings());
    minerva.restore_map_loot(saved);
    let state = minerva.tick(0).clone();

    let sent = state.map_loot_uploads.first().expect("sent");
    assert_eq!((sent.name.as_str(), sent.map.as_str(), sent.mesos), ("Egbert", SUNSET_SKY_ID, 60 * 50));
}

#[test]
fn the_visit_under_way_is_saved_afresh_once_a_minute_not_with_every_line() {
    let mut player = egbert();
    player.arrive(SUNSET_SAMPLE);
    hunt_looting(&mut player, 10 * MIN, &[mesos(50)]);
    let revision = player.minerva.map_loot_revision();
    hunt_looting(&mut player, 30 * S, &[mesos(50)]);
    assert_eq!(player.minerva.map_loot_revision(), revision);
    hunt_looting(&mut player, 30 * S, &[mesos(50)]);
    assert_ne!(player.minerva.map_loot_revision(), revision);
}
