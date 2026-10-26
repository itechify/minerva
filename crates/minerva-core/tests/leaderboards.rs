//! Uploading (#110): a Character met for the first time is shown on the Map Leaderboards, and a
//! player turns one on or off from the Personal Bests window. The core says what the site should
//! hold of them: each shown Character's best Visit on each map in each Level Band, as the shell
//! is to upload it. The shell does the HTTP and
//! tells the core each answer, which changes what the toggle says.

mod common;

use common::player::Player;
use common::{CONSTRUCTION_SITE_ID, NORTHERN_TOP_ID, SUNSET_SKY_ID, tracking, SUNSET_SAMPLE, CONSTRUCTION_SAMPLE, S, DAY, MIN, WALL};
use minerva_core::{
    BESTS_VERSION, CharacterBests, LeaderboardAnswer, LeaderboardCharacter, LeaderboardEntry, LeaderboardStatus,
    LeaderboardToggle, LeaderboardUpload, MapBests, Minerva, PanelControl, PanelState, RejectedEntry, RejectedListing, RejectedReason, Request, SavedBests, ScreenRect,
    Settings, StatusBar, StatusBarReading, VisitBest,
};

/// A Visit's best: `exp` EXP/h, set `days_ago` before [`WALL`] (`None`: before Minerva knew the
/// date) by a Magician of `level`.
fn visit(exp: u64, days_ago: Option<u64>, level: u32) -> VisitBest {
    VisitBest {
        exp_per_hour: exp,
        set_at_unix_ms: days_ago.map(|days| WALL - days * DAY),
        level: Some(level),
        job: Some("Magician".to_string()),
        ..VisitBest::default()
    }
}

fn map(id: &str, visits: Vec<VisitBest>) -> MapBests {
    MapBests { id: id.to_string(), visits }
}

/// Egbert's bests: on Sunset Sky in two Level Bands (10–19 and 20–29, a lower Visit in each
/// too, and an undated one), and on the Construction Site in 30–39; on the Northern Top only a
/// best with no date, which can't be uploaded. Bob has a best on Sunset Sky, and so do the
/// Unassigned bests.
fn saved_bests() -> SavedBests {
    let egbert = CharacterBests {
        name: "Egbert".to_string(),
        maps: vec![
            map(
                SUNSET_SKY_ID,
                vec![visit(9000, Some(3), 23), visit(8000, Some(2), 25), visit(6000, None, 14), visit(5000, Some(9), 13)],
            ),
            map(CONSTRUCTION_SITE_ID, vec![visit(6000, Some(1), 31)]),
            map(NORTHERN_TOP_ID, vec![visit(7000, None, 30)]),
        ],
    };
    let bob = CharacterBests { name: "Bob".to_string(), maps: vec![map(SUNSET_SKY_ID, vec![visit(4000, Some(5), 40)])] };
    SavedBests {
        version: BESTS_VERSION,
        unassigned: vec![map(CONSTRUCTION_SITE_ID, vec![unassigned_visit()])],
        characters: vec![egbert, bob],
    }
}

/// The Unassigned best: set on the Construction Site by a level 50 Magician whose name
/// couldn't be read.
fn unassigned_visit() -> VisitBest {
    visit(3000, Some(4), 50)
}

/// A core at launch, the bests restored, the wall clock told and the Personal Bests window open.
/// Egbert and Bob were met at earlier launches, and neither is shown.
fn launched() -> Minerva {
    let mut minerva = tracking();
    minerva.restore(Settings { map_leaderboards_met: Some(vec!["Egbert".to_string(), "Bob".to_string()]), ..minerva.settings() });
    minerva.restore_bests(saved_bests());
    minerva.on_wall_clock(WALL);
    minerva.press(PanelControl::TogglePersonalBests);
    minerva.tick(0);
    minerva
}

/// The toggle the Personal Bests window shows for `name`.
fn toggle(state: &PanelState, name: &str) -> LeaderboardToggle {
    let character = state.personal_bests.characters.iter().find(|c| c.name.as_deref() == Some(name));
    character.and_then(|c| c.leaderboard.clone()).unwrap_or_else(|| panic!("no toggle for {name}"))
}

/// An entry on Map Leaderboard `map`, set `days_ago`.
fn entry(map: &str, level: u32, exp: u64, days_ago: u64) -> LeaderboardEntry {
    LeaderboardEntry { map: map.to_string(), level, job: "Magician".to_string(), exp_per_hour: exp, set_at: WALL - days_ago * DAY, party: false }
}

/// A Party Best's entry on Map Leaderboard `map`, set `days_ago`.
fn party_entry(map: &str, level: u32, exp: u64, days_ago: u64) -> LeaderboardEntry {
    LeaderboardEntry { party: true, ..entry(map, level, exp, days_ago) }
}

/// Egbert's entries, as his bests above make them: the best dated Visit in each Level Band.
fn egberts_entries() -> Vec<LeaderboardEntry> {
    vec![entry(SUNSET_SKY_ID, 13, 5000, 9), entry(SUNSET_SKY_ID, 23, 9000, 3), entry(CONSTRUCTION_SITE_ID, 31, 6000, 1)]
}

#[test]
fn turning_a_character_on_uploads_their_best_in_each_level_band() {
    let mut minerva = launched();
    let off = LeaderboardToggle {
        on: false,
        maps: 2,
        offered: true,
        problem: false,
        status: LeaderboardStatus::Off,
        uploaded_at_unix_ms: None,
        rejected: Vec::new(),
    };
    assert_eq!(toggle(minerva.panel_state(), "Egbert"), off);
    assert_eq!(minerva.panel_state().leaderboard_uploads, []);

    minerva.show_on_leaderboards("Egbert", true);
    let state = minerva.tick(50).clone();
    let upload = LeaderboardUpload { id: 1, name: "Egbert".to_string(), entries: Some(egberts_entries()) };
    assert_eq!(state.leaderboard_uploads, [upload]);
    assert_eq!(toggle(&state, "Egbert"), LeaderboardToggle { on: true, status: LeaderboardStatus::Sending, ..off });
}

#[test]
fn a_maps_solo_best_and_party_best_are_entries_of_their_own() {
    let party = |exp, days_ago, level| VisitBest { kind: minerva_core::BestKind::Party, ..visit(exp, days_ago, level) };
    let egbert = CharacterBests {
        name: "Egbert".to_string(),
        maps: vec![map(
            SUNSET_SKY_ID,
            // Levels 20-29: a Party Best over the Solo Best; levels 10-19: a Solo Best over a
            // Party Best. A band's two bests both go up.
            vec![party(9000, Some(3), 23), visit(8000, Some(2), 25), visit(5000, Some(9), 13), party(4000, Some(8), 13)],
        )],
    };
    let mut minerva = tracking();
    minerva.restore(Settings { map_leaderboards_met: Some(vec!["Egbert".to_string()]), ..minerva.settings() });
    minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned: Vec::new(), characters: vec![egbert] });
    minerva.on_wall_clock(WALL);
    minerva.show_on_leaderboards("Egbert", true);
    let state = minerva.tick(50).clone();
    // By Level Band, a band's Solo Best before its Party Best.
    let entries = vec![
        entry(SUNSET_SKY_ID, 13, 5000, 9),
        party_entry(SUNSET_SKY_ID, 13, 4000, 8),
        entry(SUNSET_SKY_ID, 25, 8000, 2),
        party_entry(SUNSET_SKY_ID, 23, 9000, 3),
    ];
    assert_eq!(state.leaderboard_uploads, [LeaderboardUpload { id: 1, name: "Egbert".to_string(), entries: Some(entries) }]);
}

/// The entries of the uploads under way, by whose they are: `None` to delete them.
fn sent(state: &PanelState) -> Vec<(String, Option<Vec<LeaderboardEntry>>)> {
    state.leaderboard_uploads.iter().map(|u| (u.name.clone(), u.entries.clone())).collect()
}

/// The shell makes every upload under way, and the site takes each: the core ticks after.
fn all_taken(minerva: &mut Minerva, at_ms: u64) {
    for upload in minerva.panel_state().leaderboard_uploads.clone() {
        let answer = if upload.entries.is_some() { taken() } else { LeaderboardAnswer::Deleted {} };
        minerva.on_leaderboard_answer(upload.id, answer);
    }
    minerva.tick(at_ms);
}

#[test]
fn a_new_best_goes_up_when_its_visit_ends_not_while_it_rises() {
    let mut player = Player::clocked(tracking());
    player.status(Some("Egbert"), 13);
    player.arrive(SUNSET_SAMPLE);
    player.minerva.show_on_leaderboards("Egbert", true);
    let at = player.at;
    // Nothing has ended yet: an empty set, which claims the name.
    assert_eq!(sent(player.minerva.tick(at)), [("Egbert".to_string(), Some(Vec::new()))]);
    all_taken(&mut player.minerva, at);

    player.hunt(6 * MIN, 20);
    // The Visit under way has its best, as the shell saves it.
    assert_eq!(player.minerva.bests().maps_of(Some("Egbert")).len(), 1);
    let at = player.at;
    assert_eq!(sent(player.minerva.tick(at)), []);

    // On to the Construction Site: the Visit to Sunset Sky ends, and its best goes up.
    player.arrive(CONSTRUCTION_SAMPLE);
    let at = player.at;
    let set_at = player.minerva.bests().maps_of(Some("Egbert"))[0].visits[0].set_at_unix_ms.unwrap();
    let best = LeaderboardEntry { map: SUNSET_SKY_ID.to_string(), level: 13, job: "Magician".to_string(), exp_per_hour: 7200, set_at, party: false };
    assert_eq!(sent(player.minerva.tick(at)), [("Egbert".to_string(), Some(vec![best]))]);
}

/// Egbert and Bob shown on the Map Leaderboards, their first uploads taken, at 1 s.
fn both_up() -> Minerva {
    let mut minerva = launched();
    minerva.show_on_leaderboards("Egbert", true);
    minerva.show_on_leaderboards("Bob", true);
    minerva.tick(50);
    all_taken(&mut minerva, S);
    assert_eq!(sent(minerva.panel_state()), []);
    minerva
}

fn egbert(entries: Vec<LeaderboardEntry>) -> (String, Option<Vec<LeaderboardEntry>>) {
    ("Egbert".to_string(), Some(entries))
}

#[test]
fn removing_a_visit_sends_the_set_again_with_the_next_best_in_its_band() {
    let mut minerva = both_up();
    minerva.remove_visit(Some("Egbert"), SUNSET_SKY_ID, &visit(9000, Some(3), 23));
    let state = minerva.tick(2 * S).clone();
    let entries = vec![entry(SUNSET_SKY_ID, 13, 5000, 9), entry(SUNSET_SKY_ID, 25, 8000, 2), entry(CONSTRUCTION_SITE_ID, 31, 6000, 1)];
    assert_eq!(sent(&state), [egbert(entries)]);
}

#[test]
fn moving_an_unassigned_visit_to_a_shown_character_sends_their_set_again() {
    let mut minerva = both_up();
    minerva.move_visit(None, CONSTRUCTION_SITE_ID, &unassigned_visit(), "Egbert");
    let state = minerva.tick(2 * S).clone();
    let mut entries = egberts_entries();
    entries.push(entry(CONSTRUCTION_SITE_ID, 50, 3000, 4));
    assert_eq!(sent(&state), [egbert(entries)]);
}

#[test]
fn moving_a_visit_between_shown_characters_sends_both_sets_again() {
    let mut minerva = both_up();
    minerva.move_visit(Some("Bob"), SUNSET_SKY_ID, &visit(4000, Some(5), 40), "Egbert");
    let state = minerva.tick(2 * S).clone();
    let mut entries = egberts_entries();
    entries.insert(2, entry(SUNSET_SKY_ID, 40, 4000, 5));
    assert_eq!(sent(&state), [egbert(entries), ("Bob".to_string(), Some(Vec::new()))]);
}

#[test]
fn giving_a_shown_character_every_unassigned_visit_sends_their_set_again() {
    let mut minerva = both_up();
    minerva.assign_unassigned("Egbert");
    let state = minerva.tick(2 * S).clone();
    let mut entries = egberts_entries();
    entries.push(entry(CONSTRUCTION_SITE_ID, 50, 3000, 4));
    assert_eq!(sent(&state), [egbert(entries)]);
}

#[test]
fn merging_takes_the_one_merged_off_and_sends_the_others_set_again() {
    let mut minerva = both_up();
    minerva.merge_characters("Bob", "Egbert");
    let state = minerva.tick(2 * S).clone();
    let mut entries = egberts_entries();
    entries.insert(2, entry(SUNSET_SKY_ID, 40, 4000, 5));
    assert_eq!(sent(&state), [egbert(entries), ("Bob".to_string(), None)]);
    // Once the site has deleted Bob's, he is off it, and kept no more.
    all_taken(&mut minerva, 3 * S);
    let names: Vec<String> = minerva.settings().map_leaderboards.into_iter().map(|c| c.name).collect();
    assert_eq!(names, ["Egbert"]);
}

#[test]
fn removing_a_character_takes_them_off_the_site() {
    let mut minerva = both_up();
    minerva.remove_character("Bob");
    assert_eq!(sent(minerva.tick(2 * S)), [("Bob".to_string(), None)]);
}

#[test]
fn turning_it_off_deletes_their_entries_and_the_toggle_says_so_until_the_site_has() {
    let mut minerva = both_up();
    minerva.show_on_leaderboards("Egbert", false);
    let state = minerva.tick(2 * S).clone();
    assert_eq!(sent(&state), [("Egbert".to_string(), None)]);
    let off = toggle(&state, "Egbert");
    assert_eq!((off.on, off.status), (false, LeaderboardStatus::Sending));
    all_taken(&mut minerva, 3 * S);
    let state = minerva.panel_state();
    assert_eq!((toggle(state, "Egbert").on, toggle(state, "Egbert").status), (false, LeaderboardStatus::Off));
    assert_eq!(sent(state), []);
}

#[test]
fn entries_the_site_kept_off_are_listed_but_the_upload_went_through() {
    let (mut minerva, id) = egbert_uploading();
    let rejected = vec![
        RejectedEntry { map: SUNSET_SKY_ID.to_string(), band: 20, party: true, reason: RejectedReason::OverCap { cap: 8000 } },
        RejectedEntry { map: CONSTRUCTION_SITE_ID.to_string(), band: 30, party: false, reason: RejectedReason::Removed {} },
        RejectedEntry { map: NORTHERN_TOP_ID.to_string(), band: 1, party: false, reason: RejectedReason::Removed {} },
    ];
    let state = minerva.on_leaderboard_answer(id, LeaderboardAnswer::Accepted { rejected }).clone();
    let up = toggle(&state, "Egbert");
    assert_eq!((up.status, up.uploaded_at_unix_ms), (LeaderboardStatus::UpToDate, Some(WALL)));
    // Each with its Level Band's levels, for the window to name.
    let listed = vec![
        RejectedListing { map: SUNSET_SKY_ID.to_string(), lowest_level: 20, highest_level: 29, party: true, reason: RejectedReason::OverCap { cap: 8000 } },
        RejectedListing { map: CONSTRUCTION_SITE_ID.to_string(), lowest_level: 30, highest_level: 39, party: false, reason: RejectedReason::Removed {} },
        RejectedListing { map: NORTHERN_TOP_ID.to_string(), lowest_level: 1, highest_level: 9, party: false, reason: RejectedReason::Removed {} },
    ];
    assert_eq!(up.rejected, listed);
    // The site says so at every upload: it isn't tried again for it.
    assert_eq!(sent(minerva.tick(60 * MIN)), []);
    // Taken off, nothing of theirs is on the site to keep off.
    minerva.show_on_leaderboards("Egbert", false);
    assert_eq!(toggle(minerva.tick(61 * MIN), "Egbert").rejected, []);
}

#[test]
fn a_character_with_nothing_that_can_be_uploaded_isnt_offered_the_switch() {
    // Carol's only best has no date: it can't be an entry.
    let mut bests = saved_bests();
    let undated = VisitBest { set_at_unix_ms: None, ..visit(7000, None, 30) };
    bests.characters.push(CharacterBests { name: "Carol".to_string(), maps: vec![map(NORTHERN_TOP_ID, vec![undated])] });
    let mut minerva = tracking();
    minerva.restore_bests(bests);
    minerva.on_wall_clock(WALL);
    minerva.press(PanelControl::TogglePersonalBests);
    let state = minerva.tick(0).clone();
    let carol = toggle(&state, "Carol");
    assert_eq!((carol.on, carol.maps, carol.offered), (false, 0, false));
    assert!(toggle(&state, "Egbert").offered);
}

/// Where the toggle for Egbert says he stands: (status, a problem).
fn standing(state: &PanelState) -> (LeaderboardStatus, bool) {
    let toggle = toggle(state, "Egbert");
    (toggle.status, toggle.problem)
}

#[test]
fn an_upload_that_failed_or_whose_name_another_key_holds_is_a_problem_and_sent_again_goes_at_once() {
    use LeaderboardStatus::*;
    let offline = LeaderboardAnswer::Failed { why: "owlofminerva.app couldn't be reached".to_string() };
    for (answer, status) in [(offline, Retrying), (LeaderboardAnswer::Claimed {}, Claimed)] {
        let (mut minerva, id) = egbert_uploading();
        assert_eq!(standing(minerva.panel_state()), (Sending, false));
        assert_eq!(standing(&minerva.on_leaderboard_answer(id, answer).clone()), (status, true));
        assert_eq!(sent(minerva.tick(S)), []);

        // Sent again, it goes at the next tick rather than waiting, and is being sent.
        minerva.send_to_leaderboards_again("Egbert");
        let state = minerva.tick(2 * S).clone();
        assert_eq!(sent(&state), [egbert(egberts_entries())]);
        assert_eq!(standing(&state), (Sending, false));
        let state = minerva.on_leaderboard_answer(state.leaderboard_uploads[0].id, taken()).clone();
        assert_eq!(standing(&state), (UpToDate, false));
    }
}

#[test]
fn a_name_another_key_holds_says_so_and_is_tried_again_now_and_then_and_at_each_change() {
    let (mut minerva, id) = egbert_uploading();
    let state = minerva.on_leaderboard_answer(id, LeaderboardAnswer::Claimed {}).clone();
    let claimed = toggle(&state, "Egbert");
    assert_eq!((claimed.on, claimed.status, claimed.uploaded_at_unix_ms), (true, LeaderboardStatus::Claimed, None));
    assert_eq!(sent(minerva.tick(29 * MIN)), []);
    // Half an hour on: the maintainer may have handed the name to this key, which it refused.
    let state = minerva.tick(30 * MIN + 50).clone();
    assert_eq!(sent(&state), [egbert(egberts_entries())]);
    assert_eq!(toggle(&state, "Egbert").status, LeaderboardStatus::Claimed);
    let id = state.leaderboard_uploads[0].id;
    minerva.on_leaderboard_answer(id, LeaderboardAnswer::Claimed {});

    // A change goes up at once.
    minerva.remove_visit(Some("Egbert"), CONSTRUCTION_SITE_ID, &visit(6000, Some(1), 31));
    let state = minerva.tick(31 * MIN).clone();
    assert_eq!(sent(&state), [egbert(egberts_entries()[..2].to_vec())]);
    let id = state.leaderboard_uploads[0].id;
    let state = minerva.on_leaderboard_answer(id, taken()).clone();
    assert_eq!(toggle(&state, "Egbert").status, LeaderboardStatus::UpToDate);
}

#[test]
fn an_upload_that_fails_is_tried_again_quietly_later_each_time() {
    let (mut minerva, id) = egbert_uploading();
    let offline = || LeaderboardAnswer::Failed { why: "owlofminerva.app couldn't be reached".to_string() };
    let state = minerva.on_leaderboard_answer(id, offline()).clone();
    let retrying = toggle(&state, "Egbert");
    assert_eq!((retrying.on, retrying.status), (true, LeaderboardStatus::Retrying));
    assert_eq!(sent(&state), []);
    assert_eq!(sent(minerva.tick(29 * S)), []);
    let state = minerva.tick(31 * S).clone();
    assert_eq!(sent(&state), [egbert(egberts_entries())]);
    // Still saying it is being tried again.
    assert_eq!(toggle(&state, "Egbert").status, LeaderboardStatus::Retrying);

    // Failing again, it waits twice as long.
    minerva.on_leaderboard_answer(state.leaderboard_uploads[0].id, offline());
    assert_eq!(sent(minerva.tick(90 * S)), []);
    let state = minerva.tick(92 * S).clone();
    assert_eq!(sent(&state), [egbert(egberts_entries())]);
    minerva.on_wall_clock(WALL + 92 * S);
    let state = minerva.on_leaderboard_answer(state.leaderboard_uploads[0].id, taken()).clone();
    let up = toggle(&state, "Egbert");
    assert_eq!((up.status, up.uploaded_at_unix_ms), (LeaderboardStatus::UpToDate, Some(WALL + 92 * S)));
}

#[test]
fn rate_limited_it_waits_as_long_as_the_site_says_even_through_a_change() {
    let (mut minerva, id) = egbert_uploading();
    minerva.on_leaderboard_answer(id, LeaderboardAnswer::RateLimited { retry_after_secs: 120 });
    minerva.remove_visit(Some("Egbert"), CONSTRUCTION_SITE_ID, &visit(6000, Some(1), 31));
    assert_eq!(sent(minerva.tick(110 * S)), []);
    assert_eq!(toggle(minerva.panel_state(), "Egbert").status, LeaderboardStatus::Retrying);
    assert_eq!(sent(minerva.tick(121 * S)), [egbert(egberts_entries()[..2].to_vec())]);
}

#[test]
fn an_answer_to_an_upload_no_longer_under_way_changes_nothing() {
    let (mut minerva, id) = egbert_uploading();
    let before = minerva.panel_state().clone();
    assert_eq!(minerva.on_leaderboard_answer(id + 1, taken()), &before);

    // Taken off while his set was going up: once it is taken, his entries are deleted.
    minerva.show_on_leaderboards("Egbert", false);
    minerva.on_leaderboard_answer(id, taken());
    assert_eq!(sent(minerva.tick(S)), [("Egbert".to_string(), None)]);
}

#[test]
fn characters_not_shown_and_the_unassigned_bests_never_leave_the_machine() {
    let mut player = Player::clocked(tracking());
    player.minerva.show_on_leaderboards("Egbert", true);
    // Bob was turned off before he had a best: he stays off.
    player.minerva.show_on_leaderboards("Bob", false);
    let mut uploads: Vec<LeaderboardUpload> = Vec::new();
    let mut play = |player: &mut Player, name: Option<&str>, level: u32, exp: u64| {
        player.status(name, level);
        player.arrive(SUNSET_SAMPLE);
        player.hunt(6 * MIN, exp);
        player.arrive(CONSTRUCTION_SAMPLE);
        player.hunt(MIN, 10);
        // The shell's loop: a tick, and each upload under way made and taken.
        let at = player.at;
        player.minerva.tick(at);
        uploads.extend(player.minerva.panel_state().leaderboard_uploads.iter().cloned());
        all_taken(&mut player.minerva, at);
    };
    play(&mut player, None, 31, 30);
    play(&mut player, Some("Bob"), 42, 40);
    play(&mut player, Some("Egbert"), 13, 20);
    play(&mut player, Some("Bob"), 42, 50);

    let bests = player.minerva.bests();
    assert_eq!((bests.unassigned.len(), bests.maps_of(Some("Bob")).len()), (1, 1));
    assert!(uploads.iter().all(|u| u.name == "Egbert"), "{uploads:?}");
    let entries: Vec<(u32, u64)> = uploads.iter().flat_map(|u| u.entries.iter().flatten()).map(|e| (e.level, e.exp_per_hour)).collect();
    assert_eq!(entries, [(13, 7200)]);
}

/// Egbert plays a Visit to Sunset Sky at level 13 and moves on to the Construction Site, which
/// ends it, and the shell's loop ticks: the uploads asked for then.
fn a_visit_ends(player: &mut Player) -> Vec<(String, Option<Vec<LeaderboardEntry>>)> {
    player.status(Some("Egbert"), 13);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    let at = player.at;
    assert_eq!(sent(player.minerva.tick(at)), [], "nothing while the Visit is under way");
    player.arrive(CONSTRUCTION_SAMPLE);
    let at = player.at;
    sent(player.minerva.tick(at))
}

#[test]
fn a_character_met_for_the_first_time_is_shared_once_they_have_a_best_that_can_go_up() {
    let mut player = Player::clocked(tracking());
    let sent = a_visit_ends(&mut player);
    player.press(PanelControl::TogglePersonalBests);
    assert_eq!(sent, [egbert(vec![egberts_new_entry(&player)])]);
    let on = toggle(player.minerva.panel_state(), "Egbert");
    assert_eq!((on.on, on.offered, on.status), (true, true, LeaderboardStatus::Sending));
    assert_eq!(player.minerva.settings().map_leaderboards_met, Some(vec!["Egbert".to_string()]));
}

#[test]
fn a_character_turned_off_stays_off_after_the_site_deleted_them_and_at_the_next_launch() {
    let mut player = Player::clocked(tracking());
    a_visit_ends(&mut player);
    player.press(PanelControl::TogglePersonalBests);
    let at = player.at;
    all_taken(&mut player.minerva, at);
    player.minerva.show_on_leaderboards("Egbert", false);
    player.minerva.tick(at + S);
    // The site deletes them, and they are kept no more but as met.
    all_taken(&mut player.minerva, at + 2 * S);
    assert_eq!(player.minerva.settings().map_leaderboards, []);

    assert_eq!(a_visit_ends(&mut player), []);
    assert!(!toggle(player.minerva.panel_state(), "Egbert").on);

    let mut again = tracking();
    again.restore(player.minerva.settings());
    again.restore_bests(player.minerva.bests());
    again.on_wall_clock(WALL + DAY);
    again.press(PanelControl::TogglePersonalBests);
    let state = again.tick(0).clone();
    assert_eq!(sent(&state), []);
    assert!(!toggle(&state, "Egbert").on);
}

#[test]
fn characters_in_the_bests_of_settings_from_before_the_default_stay_off_and_new_ones_are_shared() {
    let mut minerva = tracking();
    // A file from before version 7: no Characters met.
    let old: Settings = serde_json::from_str(r#"{"version":6}"#).unwrap();
    minerva.restore(Settings { version: 6, map_leaderboards_met: old.map_leaderboards_met, ..minerva.settings() });
    minerva.restore_bests(saved_bests());
    minerva.on_wall_clock(WALL);
    minerva.press(PanelControl::TogglePersonalBests);
    let state = minerva.tick(0).clone();
    assert_eq!(sent(&state), []);
    assert_eq!((toggle(&state, "Egbert").on, toggle(&state, "Bob").on), (false, false));
    assert_eq!(minerva.settings().map_leaderboards_met, Some(vec!["Egbert".to_string(), "Bob".to_string()]));

    // A Character met since is new.
    let mut player = Player::clocked(minerva);
    player.status(Some("Carol"), 13);
    player.arrive(SUNSET_SAMPLE);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    let at = player.at;
    let names: Vec<String> = player.minerva.tick(at).leaderboard_uploads.iter().map(|u| u.name.clone()).collect();
    assert_eq!(names, ["Carol"]);
}

#[test]
fn with_settings_from_before_the_default_and_bests_that_couldnt_be_read_no_one_is_shared_until_they_read() {
    let mut minerva = tracking();
    minerva.restore(Settings { version: 6, map_leaderboards_met: None, ..minerva.settings() });
    minerva.restore_bests(SavedBests::default());
    minerva.bests_lost();
    let mut player = Player::clocked(minerva);
    assert_eq!(a_visit_ends(&mut player), []);
    assert_eq!(player.minerva.settings().map_leaderboards_met, None);

    // The next launch's bests read: everyone in them was met before.
    let mut again = tracking();
    again.restore(player.minerva.settings());
    again.restore_bests(saved_bests());
    again.on_wall_clock(WALL + DAY);
    assert_eq!(sent(again.tick(0)), []);
}

/// The Map Leaderboard link of the row for `map` among `name`'s (`None`: the Unassigned
/// bests) in the Personal Bests window.
fn link(state: &PanelState, name: Option<&str>, map: &str) -> Option<String> {
    let character = state.personal_bests.characters.iter().find(|c| c.name.as_deref() == name).unwrap();
    character.maps.iter().find(|m| m.id == map).unwrap().leaderboard_url.clone()
}

#[test]
fn each_map_links_to_its_map_leaderboard_at_the_band_of_the_character_playing() {
    let mut minerva = launched();
    let area = ScreenRect { x: 334, y: 1145, width: 331, height: 55 };
    let bar = StatusBar { area, level: Some(27), job: Some("Magician".to_string()), name: Some("Egbert".to_string()), exp_figures: None };
    // Read three times in a row, as a name must be to name the Character.
    for at_ms in [S, 2 * S] {
        minerva.on_status_bar_reading(StatusBarReading { bar: Some(bar.clone()), at_ms, near_miss: None });
    }
    let state = minerva.on_status_bar_reading(StatusBarReading { bar: Some(bar), at_ms: 3 * S, near_miss: None }).clone();
    let site = "https://owlofminerva.app/leaderboards/map/";

    // Egbert, playing at level 27: his band, 20–29, on every map, and his name.
    let egbert = |map: &str| Some(format!("{site}?map={map}&band=20&name=Egbert"));
    assert_eq!(link(&state, Some("Egbert"), SUNSET_SKY_ID), egbert(SUNSET_SKY_ID));
    assert_eq!(link(&state, Some("Egbert"), CONSTRUCTION_SITE_ID), egbert(CONSTRUCTION_SITE_ID));
    // Bob, not playing: the band of his best there.
    assert_eq!(link(&state, Some("Bob"), SUNSET_SKY_ID), Some(format!("{site}?map={SUNSET_SKY_ID}&band=40&name=Bob")));
    // The Unassigned bests are no one's on the site: no link.
    assert_eq!(link(&state, None, CONSTRUCTION_SITE_ID), None);
    assert_eq!(minerva.open_bests_leaderboard(None, CONSTRUCTION_SITE_ID), None);
    let opened = minerva.open_bests_leaderboard(Some("Egbert"), SUNSET_SKY_ID);
    assert_eq!(opened, egbert(SUNSET_SKY_ID).map(|url| Request::OpenInBrowser { url }));
}

#[test]
fn an_upload_is_the_apis_put_of_the_whole_set_or_its_delete() {
    let put = LeaderboardUpload { id: 1, name: "Egbert".to_string(), entries: Some(vec![entry(SUNSET_SKY_ID, 23, 9000, 3)]) };
    assert_eq!(put.path(), "api/characters/Egbert/entries");
    let body: serde_json::Value = serde_json::from_str(&put.body().unwrap()).unwrap();
    let expected = serde_json::json!({ "entries": [
        { "map": SUNSET_SKY_ID, "level": 23, "job": "Magician", "expPerHour": 9000, "setAt": WALL - 3 * DAY }
    ]});
    assert_eq!(body, expected);
    // A party entry says so; a solo one says nothing (a site before this reads it as solo).
    let party = LeaderboardUpload { entries: Some(vec![party_entry(SUNSET_SKY_ID, 23, 9000, 3)]), ..put.clone() };
    let body: serde_json::Value = serde_json::from_str(&party.body().unwrap()).unwrap();
    let expected = serde_json::json!({ "entries": [
        { "map": SUNSET_SKY_ID, "level": 23, "job": "Magician", "expPerHour": 9000, "setAt": WALL - 3 * DAY, "party": true }
    ]});
    assert_eq!(body, expected);
    let delete = LeaderboardUpload { entries: None, ..put };
    assert_eq!(delete.body(), None);
}

#[test]
fn the_sites_answers_read_as_what_they_mean() {
    let read = minerva_core::read_leaderboard_answer;
    let body = r#"{ "accepted": 1, "rejected": [
        { "map": "010003070", "band": 20, "party": true, "reason": "over-cap", "cap": 766080 },
        { "map": "010003080", "band": 30, "reason": "removed" }
    ] }"#;
    let rejected = vec![
        RejectedEntry { map: SUNSET_SKY_ID.to_string(), band: 20, party: true, reason: RejectedReason::OverCap { cap: 766_080 } },
        RejectedEntry { map: CONSTRUCTION_SITE_ID.to_string(), band: 30, party: false, reason: RejectedReason::Removed {} },
    ];
    assert_eq!(read(200, None, body), LeaderboardAnswer::Accepted { rejected });
    assert_eq!(read(200, None, r#"{ "accepted": 3, "rejected": [] }"#), taken());
    assert_eq!(read(204, None, ""), LeaderboardAnswer::Deleted {});
    assert_eq!(read(409, None, r#"{ "error": "claimed" }"#), LeaderboardAnswer::Claimed {});
    assert_eq!(read(429, Some("37"), r#"{ "error": "rate-limited" }"#), LeaderboardAnswer::RateLimited { retry_after_secs: 37 });
    // With no time given, a minute.
    assert_eq!(read(429, None, ""), LeaderboardAnswer::RateLimited { retry_after_secs: 60 });
    let why = |answer| match answer {
        LeaderboardAnswer::Failed { why } => why,
        other => panic!("{other:?} is no failure"),
    };
    let bad = r#"{ "error": "bad-request", "detail": "an entry's level isn't 1 to 200" }"#;
    assert_eq!(why(read(400, None, bad)), "HTTP 400: bad-request (an entry's level isn't 1 to 200)");
    assert_eq!(why(read(502, None, "<html>")), "HTTP 502");
}

/// Egbert opted in at launch and his first upload asked for, at 50 ms: its id.
fn egbert_uploading() -> (Minerva, u64) {
    let mut minerva = launched();
    minerva.show_on_leaderboards("Egbert", true);
    let id = minerva.tick(50).leaderboard_uploads[0].id;
    (minerva, id)
}

/// The site took everything.
fn taken() -> LeaderboardAnswer {
    LeaderboardAnswer::Accepted { rejected: Vec::new() }
}

#[test]
fn an_upload_taken_says_when_and_nothing_more_goes_up_until_something_changes() {
    let (mut minerva, id) = egbert_uploading();
    minerva.on_wall_clock(WALL + 2 * S);
    let state = minerva.on_leaderboard_answer(id, taken()).clone();
    let up = toggle(&state, "Egbert");
    assert_eq!((up.on, up.status, up.uploaded_at_unix_ms), (true, LeaderboardStatus::UpToDate, Some(WALL + 2 * S)));
    assert_eq!(state.leaderboard_uploads, []);
    assert_eq!(minerva.tick(10 * 60 * S).leaderboard_uploads, []);

    // Kept with the settings, and at the next launch uploaded once again.
    let saved = minerva.settings().map_leaderboards;
    let kept = LeaderboardCharacter { name: "Egbert".to_string(), shown: true, uploaded_at_unix_ms: Some(WALL + 2 * S), held: false };
    assert_eq!(saved, [kept]);
    let mut again = tracking();
    again.restore(Settings { map_leaderboards: saved, ..minerva.settings() });
    again.restore_bests(saved_bests());
    again.press(PanelControl::TogglePersonalBests);
    // Not before the wall clock is known.
    assert_eq!(again.tick(0).leaderboard_uploads, []);
    again.on_wall_clock(WALL + DAY);
    let state = again.tick(50).clone();
    let sent: Vec<_> = state.leaderboard_uploads.iter().map(|u| u.entries.clone()).collect();
    assert_eq!(sent, [Some(egberts_entries())]);
    let up = toggle(&state, "Egbert");
    assert_eq!((up.status, up.uploaded_at_unix_ms), (LeaderboardStatus::Sending, Some(WALL + 2 * S)));
}

/// A launch whose bests file didn't read, with the Characters `shown` shown on the Map
/// Leaderboards at an earlier one: it starts with no bests, and the shell says so. Egbert plays,
/// at level 13.
fn lost_bests(shown: &[&str]) -> Player {
    let mut minerva = tracking();
    let shown = shown
        .iter()
        .map(|name| LeaderboardCharacter { name: name.to_string(), shown: true, uploaded_at_unix_ms: Some(WALL - DAY), held: false })
        .collect();
    minerva.restore(Settings { map_leaderboards: shown, ..minerva.settings() });
    minerva.restore_bests(SavedBests::default());
    minerva.bests_lost();
    let mut player = Player::clocked(minerva);
    player.status(Some("Egbert"), 13);
    player
}

/// As [`lost_bests`], then Egbert sets a best on Sunset Sky, and the Visit ends: his only best,
/// and the Personal Bests window open. The capture time then.
fn a_best_after_the_bests_were_lost(shown: &[&str]) -> (Player, u64) {
    let mut player = lost_bests(shown);
    player.arrive(SUNSET_SAMPLE);
    let at = player.at;
    // An empty set would delete every entry they have on the site.
    assert_eq!(sent(player.minerva.tick(at)), []);
    player.hunt(6 * MIN, 20);
    player.arrive(CONSTRUCTION_SAMPLE);
    player.press(PanelControl::TogglePersonalBests);
    let at = player.at;
    (player, at)
}

/// Egbert's one entry, as [`a_best_after_the_bests_were_lost`] sets it.
fn egberts_new_entry(player: &Player) -> LeaderboardEntry {
    let set_at = player.minerva.bests().maps_of(Some("Egbert"))[0].visits[0].set_at_unix_ms.unwrap();
    LeaderboardEntry { map: SUNSET_SKY_ID.to_string(), level: 13, job: "Magician".to_string(), exp_per_hour: 7200, set_at, party: false }
}

#[test]
fn bests_that_couldnt_be_read_hold_every_shown_characters_uploads_and_the_toggle_says_why() {
    let (mut player, at) = a_best_after_the_bests_were_lost(&["Egbert", "Bob"]);
    // His new best is all the bests there are: sent, the rest of his entries would go.
    let state = player.minerva.tick(at).clone();
    assert_eq!(sent(&state), []);
    let held = toggle(&state, "Egbert");
    assert_eq!((held.on, held.status), (true, LeaderboardStatus::Held));
    assert_eq!(sent(player.minerva.tick(at + 60 * MIN)), []);
}

#[test]
fn held_uploads_stay_held_at_the_next_launch() {
    let player = lost_bests(&["Egbert", "Bob"]);
    // The bests file the shell saves from now on reads at the next launch, but holds only
    // this launch's bests.
    let mut again = tracking();
    again.restore(Settings { map_leaderboards: player.minerva.settings().map_leaderboards, ..again.settings() });
    again.restore_bests(saved_bests());
    again.on_wall_clock(WALL + DAY);
    again.press(PanelControl::TogglePersonalBests);
    let state = again.tick(0).clone();
    assert_eq!(sent(&state), []);
    assert_eq!((toggle(&state, "Egbert").status, toggle(&state, "Bob").status), (LeaderboardStatus::Held, LeaderboardStatus::Held));
}

#[test]
fn turning_a_held_character_off_and_on_sends_what_is_here_now() {
    let (mut player, at) = a_best_after_the_bests_were_lost(&["Egbert", "Bob"]);
    player.minerva.show_on_leaderboards("Egbert", false);
    assert_eq!(sent(player.minerva.tick(at)), [("Egbert".to_string(), None)]);
    all_taken(&mut player.minerva, at);
    player.minerva.show_on_leaderboards("Egbert", true);
    let state = player.minerva.tick(at + S).clone();
    assert_eq!(sent(&state), [egbert(vec![egberts_new_entry(&player)])]);
    assert_eq!(toggle(&state, "Egbert").status, LeaderboardStatus::Sending);
    // Bob's are still held.
    let held: Vec<(String, bool)> = player.minerva.settings().map_leaderboards.into_iter().map(|c| (c.name, c.held)).collect();
    assert_eq!(held, [("Bob".to_string(), true), ("Egbert".to_string(), false)]);
}

#[test]
fn a_held_character_sent_again_has_what_is_here_replace_their_entries_without_deleting_them_first() {
    let (mut player, at) = a_best_after_the_bests_were_lost(&["Egbert", "Bob"]);
    assert_eq!(standing(player.minerva.tick(at)), (LeaderboardStatus::Held, true));

    // One upload of what is here, which replaces the set on the site: no deletion before it.
    player.minerva.send_to_leaderboards_again("Egbert");
    let state = player.minerva.tick(at + S).clone();
    assert_eq!(sent(&state), [egbert(vec![egberts_new_entry(&player)])]);
    assert!(toggle(&state, "Egbert").on);
    assert_eq!(standing(&state), (LeaderboardStatus::Sending, false));
    // Bob's are still held.
    let held: Vec<(String, bool)> = player.minerva.settings().map_leaderboards.into_iter().map(|c| (c.name, c.held)).collect();
    assert_eq!(held, [("Egbert".to_string(), false), ("Bob".to_string(), true)]);
}

#[test]
fn a_character_shown_after_the_bests_were_lost_is_not_held() {
    let (mut player, at) = a_best_after_the_bests_were_lost(&["Bob"]);
    player.minerva.show_on_leaderboards("Egbert", true);
    assert_eq!(sent(player.minerva.tick(at)), [egbert(vec![egberts_new_entry(&player)])]);
}

#[test]
fn removing_every_visit_of_a_shown_character_sends_their_empty_set() {
    let mut minerva = both_up();
    minerva.remove_visit(Some("Bob"), SUNSET_SKY_ID, &visit(4000, Some(5), 40));
    assert_eq!(sent(minerva.tick(2 * S)), [("Bob".to_string(), Some(Vec::new()))]);
}
