//! The settings view's Game Data: when OSMS's map set was fetched, and Refresh, which has
//! OSMS's file set and item files fetched afresh at once, whatever their daily schedule,
//! for the new maps, mobs and items a patch brings. It says what it came to until the
//! settings view closes: up to date, OSMS unreachable, or Minerva needing an update for
//! what OSMS serves now. The data held stays whenever a fresh one can't be had.

mod common;

use common::items::{etc, item_world};
use common::osms::{self, map};
use common::{FakeSource, ItemSetAnswer, OsmsAnswer, fresh, osms_unreachable, osms_changed_format};
use minerva_core::osms_items::ItemFiles;
use minerva_core::{Action, Minerva, PanelState, Refresh};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::Poll;

/// When the launch was: 2026-10-21 16:00 UTC, in ms since the Unix epoch.
const NOW: u64 = 1_792_598_400_000;

fn items() -> ItemSetAnswer {
    Poll::Ready(Ok(item_world(vec![etc(4_000_000, "Blue Snail Shell")]).files()))
}

fn items_changed_format() -> ItemSetAnswer {
    Poll::Ready(Ok(ItemFiles { items: "<html>".into(), ..ItemFiles::default() }))
}

/// The game world after a patch: a new map on Victoria Road.
fn patched() -> osms::World {
    osms::game().with(map(100_000_005, "Victoria Road", "A New Field"))
}

/// How many times the source was asked for each, polls included.
struct Asked {
    sets: Arc<AtomicU32>,
    item_sets: Arc<AtomicU32>,
}

impl Asked {
    fn counts(&self) -> (u32, u32) {
        (self.sets.load(Ordering::SeqCst), self.item_sets.load(Ordering::SeqCst))
    }
}

/// Started with the wall clock at [`NOW`], both sets fetched at launch, and the settings
/// view open.
fn started(sets: Vec<OsmsAnswer>, item_sets: Vec<ItemSetAnswer>) -> (Minerva, Asked) {
    let source = FakeSource::answering_osms(sets).answering_item_set(item_sets);
    let asked = Asked { sets: source.osms_requests.clone(), item_sets: source.item_set_requests.clone() };
    let mut minerva = Minerva::new(source);
    minerva.on_wall_clock(NOW);
    minerva.tick(0);
    minerva.act(Action::OpenSettings);
    (minerva, asked)
}

fn refresh(state: &PanelState) -> Option<Refresh> {
    state.game_data.refresh
}

fn found(minerva: &mut Minerva, query: &str) -> Vec<String> {
    minerva.act(Action::CloseSettings);
    minerva.on_hotkey(common::world::SEARCH);
    let state = minerva.search_query(query).clone();
    state.search.maps.shown.iter().map(|m| m.map_name.clone()).collect()
}

#[test]
fn the_settings_view_says_when_the_map_set_was_fetched() {
    let (minerva, _) = started(vec![fresh(osms::game())], vec![items()]);

    let state = minerva.panel_state();

    assert_eq!(state.game_data.fetched_at_unix_ms, Some(NOW));
    assert_eq!(refresh(state), None);
}

/// A patch landed after launch. Refresh has both fetched afresh at the next tick, rather
/// than a day later, and the new map is in Search.
#[test]
fn refresh_fetches_the_set_and_item_files_afresh_at_once() {
    let (mut minerva, asked) = started(vec![fresh(osms::game()), fresh(patched())], vec![items()]);
    assert_eq!(asked.counts(), (1, 1));

    assert_eq!(refresh(minerva.act(Action::RefreshGameData)), Some(Refresh::Refreshing));
    minerva.on_wall_clock(NOW + 60_000);
    let state = minerva.tick(60_000).clone();

    assert_eq!(asked.counts(), (2, 2));
    assert_eq!(refresh(&state), Some(Refresh::UpToDate));
    assert_eq!(state.game_data.fetched_at_unix_ms, Some(NOW + 60_000));
    assert_eq!(found(&mut minerva, "A New Field"), vec!["A New Field"]);
}

/// The set takes a while to arrive: Refresh says so meanwhile, and pressing it again
/// starts nothing new. It is up to date once the set is in.
#[test]
fn refresh_is_under_way_until_both_arrive_and_pressing_it_again_starts_nothing_new() {
    let (mut minerva, asked) = started(vec![fresh(osms::game()), Poll::Pending, fresh(patched())], vec![items()]);

    minerva.act(Action::RefreshGameData);
    assert_eq!(refresh(minerva.tick(1_000)), Some(Refresh::Refreshing));
    assert_eq!(asked.counts(), (2, 2), "the item files are in; the set is under way");
    assert_eq!(refresh(minerva.act(Action::RefreshGameData)), Some(Refresh::Refreshing));
    minerva.tick(2_000);

    assert_eq!(asked.counts(), (3, 2), "the set's fetch polled once more, and the item files weren't asked for again");
    assert_eq!(refresh(minerva.panel_state()), Some(Refresh::UpToDate));
}

/// What OSMS serves can't be had or doesn't read: the data held stays, and Refresh says
/// why, the worse of the two sets' outcomes.
#[test]
fn refresh_says_what_kept_the_data_held() {
    let cases = [
        ("set unreachable", osms_unreachable(), items(), Refresh::Unreachable),
        ("item files unreachable", fresh(osms::game()), common::items_unreachable(), Refresh::Unreachable),
        ("set changed format", osms_changed_format(), items(), Refresh::NeedsUpdate),
        ("item files changed format", fresh(osms::game()), items_changed_format(), Refresh::NeedsUpdate),
        ("both", osms_changed_format(), common::items_unreachable(), Refresh::NeedsUpdate),
    ];
    for (how, set, item_set, said) in cases {
        let (mut minerva, _) = started(vec![fresh(osms::game()), set], vec![items(), item_set]);

        minerva.act(Action::RefreshGameData);
        minerva.on_wall_clock(NOW + 60_000);
        let state = minerva.tick(60_000).clone();

        assert_eq!(refresh(&state), Some(said), "{how}");
        if how.starts_with("set") || how == "both" {
            assert_eq!(state.game_data.fetched_at_unix_ms, Some(NOW), "{how}: the set held stays");
        }
        assert!(found(&mut minerva, "Sunset Sky").contains(&"Sunset Sky".to_string()), "{how}");
    }
}

#[test]
fn what_refresh_came_to_goes_when_the_settings_view_closes() {
    let (mut minerva, _) = started(vec![fresh(osms::game())], vec![items()]);
    minerva.act(Action::RefreshGameData);
    minerva.tick(1_000);

    let state = minerva.act(Action::CloseSettings);
    assert_eq!(refresh(state), None);
    let state = minerva.act(Action::OpenSettings);
    assert_eq!(refresh(state), None);
}
