//! Bug Reports (ADR 0009): the core writes down every input it takes in and what came of
//! it, and saves it, with crops of what it read, as a `.zip` that replays: a new core fed
//! the same inputs goes through the same states. The core is driven here as the shell's
//! loop drives it, on the made-up game world.

mod common;

use common::items::{ItemWorld, equip, etc, item_world, scroll};
use common::osms;
use common::{CONSTRUCTION_SITE_ID, FakeSource, SUNSET_SKY_ID, frame, HOUR, S, MIN, percent};
use minerva_core::bug_report::{self, Answer, Event, Note, Recording, ReplayData, MAX_REPORT_BYTES, MAX_TEXT_AND_CROPS_BYTES};
use minerva_core::{
    ListedPrice, ScrollTier, ShopCapture, ShopHover, ShopList, ShopNearMiss, Verdict, ShopReading, ShopRow, StatusBar, StatusBarCapture, StatusBarNearMiss, StatusBarReading,
    Action, BESTS_VERSION, Minerva, PanelControl, ExpBar, ExpBarPlace, ExpBarReading, Foreground, Frame, GameWindow, ItemCapture, ItemTooltip,
    CharacterBests, DiscordLinkAnswer, ItemTooltipReading, LeaderboardAnswer, LeaderboardCharacter, MapBests, RejectedEntry, RejectedReason, ReportAbout, SavedBests, STATUS_BAR_STRIP, ScreenPoint, ScreenRect, Settings, Size, TitleLines,
    TitleReading, TrackingState, UpdateAnswer, UpdateStatus, VisitBest, Build, DownloadAnswer, UpdateStage,
};

/// The wall clock at capture time 0: 11 Oct 2026, 12:00 UTC.
const WALL_AT_START: u64 = 1_791_720_000_000;

const CONSTRUCTION_SITE: (&str, &str) = ("Victoria Road", "Construction Site North of Kerning City");
const SUNSET_SKY: (&str, &str) = ("Victoria Road", "Sunset Sky");

fn about() -> ReportAbout {
    ReportAbout {
        version: "0.7.0".into(),
        os: "Windows 11, build 26100".into(),
        display_scale: 1.5,
        local_time: "2026-10-11 14:03:27".into(),
    }
}

/// The game window, at the screen's origin.
fn game() -> GameWindow {
    common::window_at_origin(1920, 1200)
}

/// The bottom of the game's client area, where the core has the Status Bar captured.
fn status_bar_strip() -> ScreenRect {
    let client = game().client;
    ScreenRect { y: client.y + (client.height - STATUS_BAR_STRIP) as i32, height: STATUS_BAR_STRIP, ..client }
}

/// The core as the shell runs it: the settings and Personal Bests restored at launch, then
/// a loop that every 50 ms reports the game window, what is in front and the Grind
/// Tracker's size, tells the wall clock and ticks, and takes the readings as they come.
struct Shell {
    minerva: Minerva,
    at: u64,
}

impl Shell {
    fn new(source: FakeSource) -> Self {
        let mut minerva = Minerva::new(source);
        minerva.restore(common::set_up());
        minerva.restore_bests(SavedBests::default());
        let mut shell = Shell { minerva, at: 0 };
        shell.step();
        shell
    }

    /// One turn of the loop, at the time it is now.
    fn step(&mut self) {
        let minerva = &mut self.minerva;
        minerva.on_game_window(Some(game()));
        minerva.on_foreground(Foreground::Game);
        minerva.set_grind_tracker_size(Size { width: 300, height: 200 });
        minerva.on_wall_clock(WALL_AT_START + self.at);
        minerva.tick(self.at);
    }

    /// Play on until `until`: the title on `map` read every 350 ms, and the EXP bar every
    /// second as `exp` says (`None`: not whole) at that time.
    fn play(&mut self, until: u64, map: (&str, &str), exp: impl Fn(u64) -> Option<ExpBar>) {
        while self.at < until {
            self.at += 50;
            if self.at.is_multiple_of(350) {
                let lines = Some(TitleLines { street: map.0.into(), map: map.1.into() });
                self.minerva.on_reading(TitleReading { lines, found: Some(common::TITLE_FOUND), at_ms: self.at });
            }
            if self.at.is_multiple_of(S) {
                self.minerva.on_exp_bar_reading(ExpBarReading { bar: exp(self.at), at_ms: self.at, place: None });
            }
            self.step();
        }
    }

    fn report(&mut self) -> (minerva_core::BugReport, Recording) {
        let report = self.minerva.bug_report(&about());
        let recording = bug_report::read(&report.zip).expect("the report reads back");
        (report, recording)
    }
}

/// A grind: 18,000 EXP of a 20,000 EXP level at the start, 25 more a second, a death of
/// 400 EXP at 0:30, a misread at 0:40, and a level-up into a 25,000 EXP level at 1:37.
fn grind(at: u64) -> Option<ExpBar> {
    let s = at / S;
    let exp = 18_000 + 25 * s - if s >= 30 { 425 } else { 0 };
    if s == 40 {
        return Some(ExpBar { exp: 84_350, percent_hundredths: 4172 });
    }
    if exp >= 20_000 {
        let exp = exp - 20_000;
        return Some(ExpBar { exp, percent_hundredths: percent(exp, 25_000) });
    }
    Some(ExpBar { exp, percent_hundredths: percent(exp, 20_000) })
}

#[test]
fn a_report_replays_through_the_same_states_with_the_runs_osms_set() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);
    shell.play(3 * MIN, SUNSET_SKY, |at| grind(at).map(|bar| ExpBar { exp: bar.exp + 10, ..bar }));

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
}

#[test]
fn a_visit_removed_in_the_personal_bests_window_replays_and_reads_in_the_timeline() {
    // Sunset Sky's Visits at 10,800 and 7,200 EXP/h, the second set 11 Oct 2026, 11:00 UTC.
    let mut shell = Shell::new(FakeSource::osms());
    let removed = VisitBest { exp_per_hour: 7200, set_at_unix_ms: Some(WALL_AT_START - HOUR), ..VisitBest::default() };
    let visits = vec![VisitBest { exp_per_hour: 10_800, ..VisitBest::default() }, removed.clone()];
    let maps = vec![MapBests { id: SUNSET_SKY_ID.into(), visits }];
    shell.minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned: maps, characters: Vec::new() });
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    shell.minerva.remove_visit(None, SUNSET_SKY_ID, &removed);
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let rates: Vec<u64> = shell.minerva.bests().unassigned[0].visits.iter().map(|v| v.exp_per_hour).collect();
    assert_eq!(rates, [10_800]);
    let (_, recording) = shell.report();
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
    let said = format!(
        "Visit removed from the Unassigned Visit History of {SUNSET_SKY_ID}: 7,200 EXP/h, set at {}",
        WALL_AT_START - HOUR
    );
    assert!(recording.timeline(0).iter().any(|line| line.ends_with(&said)), "{:#?}", recording.timeline(0));
}

#[test]
fn a_visit_moved_to_the_party_bests_replays_and_reads_in_the_timeline() {
    let mut shell = Shell::new(FakeSource::osms());
    let moved = VisitBest { exp_per_hour: 7200, set_at_unix_ms: Some(WALL_AT_START - HOUR), ..VisitBest::default() };
    let maps = vec![MapBests { id: SUNSET_SKY_ID.into(), visits: vec![moved.clone()] }];
    shell.minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned: maps, characters: Vec::new() });
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    shell.minerva.set_visit_party(None, SUNSET_SKY_ID, &moved, true);
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    assert!(shell.minerva.bests().unassigned[0].visits[0].party);
    let (_, recording) = shell.report();
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
    let said = format!(
        "Unassigned Visit on {SUNSET_SKY_ID} moved to the Party Bests: 7,200 EXP/h, set at {}",
        WALL_AT_START - HOUR
    );
    assert!(recording.timeline(0).iter().any(|line| line.ends_with(&said)), "{:#?}", recording.timeline(0));
}

#[test]
fn this_visits_best_removed_from_the_best_row_replays_counting_only_the_play_since() {
    // 6 minutes on the Construction Site set this Visit's best, which the Best row shows.
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(6 * MIN, CONSTRUCTION_SITE, grind);
    assert!(shell.minerva.panel_state().exp_tracking.personal_best_this_visit);

    // Removed, then 4 minutes more: too few since to set it again.
    shell.minerva.press(PanelControl::RemoveBest);
    shell.play(10 * MIN, CONSTRUCTION_SITE, grind);
    assert_eq!(shell.minerva.panel_state().exp_tracking.personal_best, None);
    assert!(shell.minerva.bests().unassigned.is_empty());

    let (_, recording) = shell.report();
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
}

#[test]
fn a_visit_moved_and_characters_removed_and_merged_replay_and_read_in_the_timeline() {
    // An Unassigned Visit on Sunset Sky; Egbert's, Egbert2's and Bob's on the Construction
    // Site.
    let mut shell = Shell::new(FakeSource::osms());
    let unassigned = VisitBest { exp_per_hour: 7200, set_at_unix_ms: Some(WALL_AT_START - HOUR), ..VisitBest::default() };
    let at = |exp_per_hour, level: u32| VisitBest {
        exp_per_hour,
        set_at_unix_ms: None,
        level: Some(level),
        job: Some("Magician".to_string()),
        ..VisitBest::default()
    };
    let on_the_site = |visit: VisitBest| vec![MapBests { id: CONSTRUCTION_SITE_ID.into(), visits: vec![visit] }];
    let characters = vec![
        CharacterBests { name: "Egbert".into(), maps: on_the_site(at(5000, 13)) },
        CharacterBests { name: "Egbert2".into(), maps: on_the_site(at(6000, 14)) },
        CharacterBests { name: "Bob".into(), maps: on_the_site(at(3000, 40)) },
        CharacterBests { name: "Carol".into(), maps: on_the_site(at(4000, 15)) },
    ];
    // And one more Unassigned, on the Construction Site.
    let unassigned_maps = vec![
        MapBests { id: SUNSET_SKY_ID.into(), visits: vec![unassigned.clone()] },
        MapBests { id: CONSTRUCTION_SITE_ID.into(), visits: vec![at(2000, 12)] },
    ];
    shell.minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned: unassigned_maps, characters });
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    shell.minerva.move_visit(None, SUNSET_SKY_ID, &unassigned, "Egbert");
    shell.play(MIN + 5 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.move_visit(Some("Carol"), CONSTRUCTION_SITE_ID, &at(4000, 15), "Egbert");
    shell.play(MIN + 10 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.assign_unassigned("Egbert");
    shell.play(MIN + 20 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.remove_character("Bob");
    shell.play(MIN + 40 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.merge_characters("Egbert2", "Egbert");
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let bests = shell.minerva.bests();
    let names: Vec<&str> = bests.characters.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Egbert"]);
    assert!(bests.unassigned.is_empty());
    let (_, recording) = shell.report();
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
    let timeline = recording.timeline(0);
    for said in [
        format!("Unassigned Visit on {SUNSET_SKY_ID} moved to \"Egbert\": 7,200 EXP/h, set at {}", WALL_AT_START - HOUR),
        format!("\"Carol\"'s Visit on {CONSTRUCTION_SITE_ID} moved to \"Egbert\": 4,000 EXP/h"),
        "Every Unassigned Visit given to \"Egbert\"".to_string(),
        "Character \"Bob\" removed, with their bests".to_string(),
        "Character \"Egbert2\" merged into \"Egbert\"".to_string(),
    ] {
        assert!(timeline.iter().any(|line| line.contains(&said)), "{said} in {timeline:#?}");
    }
}

/// The upload under way for the Character named `name`: its id.
fn upload_for(shell: &Shell, name: &str) -> u64 {
    let uploads = &shell.minerva.panel_state().leaderboard_uploads;
    uploads.iter().find(|u| u.name == name).unwrap_or_else(|| panic!("no upload for {name} in {uploads:?}")).id
}

#[test]
fn showing_a_character_on_the_map_leaderboards_and_each_answer_replay_and_read_in_the_timeline() {
    // Egbert's best on the Construction Site, set at level 13 an hour before.
    let mut shell = Shell::new(FakeSource::osms());
    let best = VisitBest {
        exp_per_hour: 5000,
        set_at_unix_ms: Some(WALL_AT_START - HOUR),
        level: Some(13),
        job: Some("Magician".to_string()),
        ..VisitBest::default()
    };
    let maps = vec![MapBests { id: CONSTRUCTION_SITE_ID.into(), visits: vec![best] }];
    let characters = vec![CharacterBests { name: "Egbert".into(), maps }];
    shell.minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned: Vec::new(), characters });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);

    shell.minerva.show_on_leaderboards("Egbert", true);
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    let id = upload_for(&shell, "Egbert");
    shell.minerva.on_leaderboard_answer(id, LeaderboardAnswer::Failed { why: "offline".into() });
    // Sent again 10 s on, rather than waiting, and taken but for one the maintainer removed.
    shell.play(25 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.send_to_leaderboards_again("Egbert");
    shell.play(35 * S, CONSTRUCTION_SITE, grind);
    let id = upload_for(&shell, "Egbert");
    let rejected = vec![RejectedEntry { map: CONSTRUCTION_SITE_ID.into(), band: 10, party: false, reason: RejectedReason::Removed {} }];
    shell.minerva.on_leaderboard_answer(id, LeaderboardAnswer::Accepted { rejected });
    shell.play(MIN, CONSTRUCTION_SITE, grind);
    shell.minerva.show_on_leaderboards("Egbert", false);
    shell.play(MIN + 10 * S, CONSTRUCTION_SITE, grind);
    let id = upload_for(&shell, "Egbert");
    shell.minerva.on_leaderboard_answer(id, LeaderboardAnswer::Deleted {});
    shell.play(MIN + 20 * S, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));
    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let timeline = recording.timeline(0);
    for said in [
        "\"Egbert\" shown on the Map Leaderboards",
        "Map Leaderboards upload 1 asked: \"Egbert\"'s 1 entries",
        "Map Leaderboards upload 1 for \"Egbert\": failed: offline",
        "\"Egbert\" sent to the Map Leaderboards again",
        "Map Leaderboards upload 2 asked: \"Egbert\"'s 1 entries",
        "Map Leaderboards upload 2 for \"Egbert\": taken but for 010003080 band 10 removed",
        "\"Egbert\" taken off the Map Leaderboards",
        "Map Leaderboards upload 3 asked: deleting \"Egbert\"'s entries",
        "Map Leaderboards upload 3 for \"Egbert\": deleted",
    ] {
        assert!(timeline.iter().any(|line| line.contains(said)), "{said} in {timeline:#?}");
    }
}

#[test]
fn update_checks_their_answers_the_build_and_the_setting_replay_and_read_in_the_timeline() {
    // The launch check fails; with the setting turned off, the player checks by hand and a
    // newer version is out.
    let mut shell = Shell::new(FakeSource::osms());
    shell.minerva.on_build(Build::Nsis);
    shell.play(5 * S, CONSTRUCTION_SITE, grind);
    let id = shell.minerva.panel_state().updates.check.expect("the launch check");
    shell.minerva.on_update_answer(id, UpdateAnswer::Failed { why: "offline".into() });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.act(Action::SetCheckForUpdates { on: false });
    shell.minerva.act(Action::CheckForUpdates);
    shell.play(15 * S, CONSTRUCTION_SITE, grind);
    let id = shell.minerva.panel_state().updates.check.expect("the check pressed for");
    shell.minerva.on_update_answer(id, UpdateAnswer::Latest { version: "999.0.0".into() });
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    assert_eq!(shell.minerva.panel_state().updates.status, UpdateStatus::NewerVersion);

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));
    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let timeline = recording.timeline(0);
    for said in [
        "build installed with the setup .exe: updates in place",
        "update check 1 asked",
        "update check 1 failed: offline",
        "action {\"setCheckForUpdates\":{\"on\":false}}",
        "action \"checkForUpdates\"",
        "update check 2 asked",
        "update check 2: the latest version is 999.0.0",
    ] {
        assert!(timeline.iter().any(|line| line.contains(said)), "{said} in {timeline:#?}");
    }
}

#[test]
fn the_owls_update_line_dismissed_and_opened_replays_and_reads_in_the_timeline() {
    // The launch check finds a newer version, which the owl says; the player dismisses it,
    // then a later check finds a newer one still, and they click it.
    let mut shell = Shell::new(FakeSource::osms());
    shell.minerva.set_owl_bubble_size(Size { width: 420, height: 56 });
    shell.play(5 * S, CONSTRUCTION_SITE, grind);
    let id = shell.minerva.panel_state().updates.check.expect("the launch check");
    shell.minerva.on_update_answer(id, UpdateAnswer::Latest { version: "999.0.0".into() });
    assert!(shell.minerva.panel_state().owl_bubble.is_some(), "the owl says it");
    shell.minerva.act(Action::DismissUpdateLine);
    shell.minerva.act(Action::CheckForUpdates);
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    let id = shell.minerva.panel_state().updates.check.expect("the check pressed for");
    shell.minerva.on_update_answer(id, UpdateAnswer::Latest { version: "999.0.1".into() });
    shell.minerva.act(Action::OpenUpdateLine);
    shell.play(15 * S, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));
    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let timeline = recording.timeline(0);
    for said in ["Owl Bubble size 420x56", "action \"dismissUpdateLine\"", "action \"openUpdateLine\""] {
        assert!(timeline.iter().any(|line| line.contains(said)), "{said} in {timeline:#?}");
    }
}

#[test]
fn the_item_lookup_alerts_size_in_a_report_from_before_the_owl_reads_as_the_owl_bubbles() {
    let event: bug_report::Event = serde_json::from_str(r#"{"itemLookupSize": {"size": {"width": 440, "height": 40}}}"#).unwrap();
    assert_eq!(event, bug_report::Event::OwlBubbleSize { size: Size { width: 440, height: 40 } });
}

#[test]
fn updating_its_choices_downloads_and_installs_replay_and_read_in_the_timeline() {
    // A Session running, so choosing to update asks first: Later, then Update when I quit,
    // whose download fails; then Update now, whose installer doesn't start; then Update when
    // I quit again, downloaded, and Quit Minerva installs it.
    let mut shell = Shell::new(FakeSource::osms());
    shell.minerva.on_build(Build::Nsis);
    shell.play(5 * S, CONSTRUCTION_SITE, grind);
    let id = shell.minerva.panel_state().updates.check.expect("the launch check");
    shell.minerva.on_update_answer(id, UpdateAnswer::Latest { version: "999.0.0".into() });
    shell.play(40 * S, CONSTRUCTION_SITE, grind);

    shell.minerva.press(PanelControl::Update);
    assert_eq!(shell.minerva.panel_state().updates.notice.as_ref().map(|notice| notice.stage), Some(UpdateStage::Asking));
    shell.minerva.act(Action::UpdateLater);
    shell.minerva.press(PanelControl::Update);
    shell.minerva.act(Action::UpdateWhenIQuit);
    shell.play(42 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.on_update_download(1, DownloadAnswer::Failed { why: "the signature didn't check out".into() });
    shell.play(44 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.press(PanelControl::Update);
    shell.minerva.act(Action::UpdateNow);
    shell.play(46 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.on_update_download(2, DownloadAnswer::Downloaded { version: "999.0.0".into() });
    shell.minerva.on_update_install_failed(2, "the installer was blocked");
    shell.play(48 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.press(PanelControl::Update);
    shell.minerva.act(Action::UpdateWhenIQuit);
    shell.minerva.on_update_download(3, DownloadAnswer::Downloaded { version: "999.0.0".into() });
    shell.play(50 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.act(Action::Quit);
    assert_eq!(shell.minerva.panel_state().updates.notice.as_ref().map(|notice| notice.stage), Some(UpdateStage::Installing));

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));
    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let timeline = recording.timeline(0);
    for said in [
        "pressed \"update\"",
        "action \"updateLater\"",
        "action \"updateWhenIQuit\"",
        "update download 1 asked",
        "update download 1 failed: the signature didn't check out",
        "action \"updateNow\"",
        "update download 2: 999.0.0 downloaded, its signature checked",
        "update download 2's install asked, relaunching",
        "update download 2's installer didn't start: the installer was blocked",
        "update download 3 asked",
        "action \"quit\"",
        "update download 3's install asked, not relaunching",
    ] {
        assert!(timeline.iter().any(|line| line.contains(said)), "{said} in {timeline:#?}");
    }
}

/// The request about linking to Discord under way: its id.
fn discord_link_request(shell: &Shell) -> u64 {
    let requests = &shell.minerva.panel_state().discord_link_requests;
    requests.first().unwrap_or_else(|| panic!("no request about linking under way")).id
}

#[test]
fn linking_discord_its_answers_and_the_prompt_dismissed_replay_and_read_in_the_timeline() {
    // A best, so the Personal Bests window opens.
    let mut shell = Shell::new(FakeSource::osms());
    let best = VisitBest { exp_per_hour: 5000, ..VisitBest::default() };
    let unassigned = vec![MapBests { id: CONSTRUCTION_SITE_ID.into(), visits: vec![best] }];
    shell.minerva.restore_bests(SavedBests { version: BESTS_VERSION, unassigned, characters: Vec::new() });
    shell.minerva.press(PanelControl::TogglePersonalBests);
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    let id = discord_link_request(&shell);
    shell.minerva.on_discord_link_answer(id, DiscordLinkAnswer::NotLinked {});
    shell.minerva.dismiss_link_prompt();
    shell.play(15 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.link_discord();
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    let id = discord_link_request(&shell);
    shell.minerva.on_discord_link_answer(id, DiscordLinkAnswer::Opened {});
    shell.play(30 * S, CONSTRUCTION_SITE, grind);
    let id = discord_link_request(&shell);
    shell.minerva.on_discord_link_answer(id, DiscordLinkAnswer::Linked { username: "egberticus".into() });
    shell.play(40 * S, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));
    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let timeline = recording.timeline(0);
    for said in [
        "Discord link request 1 asked: read the link state",
        "Discord link request 1: not linked",
        "the prompt to link Discord dismissed",
        "Link Discord pressed",
        "Discord link request 2 asked: open the link page",
        "Discord link request 2: the link page opened",
        "Discord link request 3 asked: read the link state",
        "Discord link request 3: linked to \"egberticus\"",
    ] {
        assert!(timeline.iter().any(|line| line.contains(said)), "{said} in {timeline:#?}");
    }
}

#[test]
fn bests_lost_at_launch_replay_holding_the_uploads_and_read_in_the_timeline() {
    // Egbert shown on the Map Leaderboards at an earlier launch, and this one's bests file
    // unreadable.
    let mut shell = Shell::new(FakeSource::osms());
    let egbert = LeaderboardCharacter { name: "Egbert".into(), shown: true, uploaded_at_unix_ms: None, held: false };
    let settings = Settings { map_leaderboards: vec![egbert], ..shell.minerva.settings() };
    shell.minerva.restore(settings);
    shell.minerva.restore_bests(SavedBests::default());
    shell.minerva.bests_lost();
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    assert_eq!(shell.minerva.panel_state().leaderboard_uploads, []);

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::game().files()));
    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let said = "Personal Bests file unreadable: uploads to the Map Leaderboards held";
    let timeline = recording.timeline(0);
    assert!(timeline.iter().any(|line| line.contains(said)), "{said} in {timeline:#?}");
}

#[test]
fn a_visit_removed_in_a_report_from_before_characters_replays_as_an_unassigned_one() {
    // A line as a report from before Characters wrote it: no Character, no level or job.
    let line = format!(
        r#"{{"removeVisit": {{"map": "{SUNSET_SKY_ID}", "visit": {{"expPerHour": 7200, "setAtUnixMs": null}}}}}}"#
    );
    let event: bug_report::Event = serde_json::from_str(&line).unwrap();
    let visit = VisitBest { exp_per_hour: 7200, ..VisitBest::default() };
    assert_eq!(event, bug_report::Event::RemoveVisit { map: SUNSET_SKY_ID.into(), visit, character: None });
}

#[test]
fn a_replay_with_another_osms_set_diverges_where_that_shows() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let replay = bug_report::replay(&recording, Some(osms::made_up_world(3).files()));

    let divergence = replay.divergence.expect("another set takes the core elsewhere");
    let recorded = divergence.recorded.expect("a line the report has");
    assert!(recorded.contains("Construction Site North of Kerning City"), "where the map was recognised: {recorded}");
}

#[test]
fn a_replay_with_a_set_that_shows_the_same_goes_through_the_same_states() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    // A set fetched on another day, the same but for a map no one went to.
    let newer = osms::game().naming(osms::HENESYS_HUNTING_GROUND_I, "Henesys Hunting Ground One");
    let replay = bug_report::replay(&recording, Some(newer.files()));

    assert_eq!(replay.divergence, None);
}

#[test]
fn the_trace_says_what_the_session_made_of_the_readings() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let noted = |note: fn(&Note) -> bool| {
        recording.lines.iter().filter(|line| line.notes.iter().any(note)).map(|line| &line.event).collect::<Vec<_>>()
    };

    assert_eq!(noted(|n| *n == Note::SessionStarted).len(), 1);
    assert_eq!(noted(|n| *n == Note::Visit).len(), 1, "one map confirmed");
    let deaths = noted(|n| matches!(n, Note::Death { exp_lost: 400 }));
    assert!(matches!(deaths[..], [Event::ExpBar { bar: Some(ExpBar { exp: 18_325, .. }), .. }]), "{deaths:?}");
    let misreads = noted(|n| matches!(n, Note::Misfit { count: 1, .. }));
    assert!(matches!(misreads[..], [Event::ExpBar { bar: Some(ExpBar { exp: 84_350, .. }), .. }]), "{misreads:?}");
    let level_ups = noted(|n| matches!(n, Note::LevelUp { .. }));
    assert_eq!(level_ups.len(), 1, "{level_ups:?}");
}

#[test]
fn the_trace_says_what_the_grind_tracker_and_the_title_showed_where_it_changed() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let maps: Vec<_> = recording.lines.iter().filter_map(|line| line.map.as_ref()).collect();
    let states: Vec<_> =
        recording.lines.iter().filter_map(|line| line.grind_tracker.as_ref()?.get("state")?.as_str()).collect();

    assert_eq!(maps.len(), 1);
    assert_eq!(maps[0].id.as_deref(), Some("010003080"));
    assert_eq!(states, ["waiting", "warmingUp", "active"]);
    let last = shell.minerva.panel_state().exp_tracking.clone();
    assert_eq!(last.state, TrackingState::Active);
}

#[test]
fn ticks_that_change_nothing_are_left_out_but_for_the_last_before_a_line() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let ticks = recording.lines.iter().filter(|line| line.event == Event::Tick {}).count();
    let ticked = recording.lines.iter().filter(|line| line.ticked).count();

    // 1,200 turns of the loop, each with a tick.
    assert!(ticks < 60, "{ticks} ticks written");
    assert!(ticked > 200, "the readings come after a tick: {ticked}");
}

#[test]
fn inputs_that_only_set_a_value_are_written_when_it_changes() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);
    shell.minerva.on_foreground(Foreground::OtherApp);
    shell.minerva.on_foreground(Foreground::OtherApp);

    let (_, recording) = shell.report();
    let count = |is: fn(&Event) -> bool| recording.lines.iter().filter(|line| is(&line.event)).count();

    assert_eq!(count(|e| matches!(e, Event::GameWindow { .. })), 1);
    assert_eq!(count(|e| matches!(e, Event::Foreground { .. })), 2);
    assert_eq!(count(|e| matches!(e, Event::GrindTrackerSize { .. })), 1);
}

#[test]
fn the_wall_clock_is_written_down_again_only_when_it_strays() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);
    // The PC's clock set 10 s on.
    shell.minerva.on_wall_clock(WALL_AT_START + shell.at + 10 * S);
    shell.minerva.tick(shell.at + 50);

    let (_, recording) = shell.report();
    let wall_clocks: Vec<u64> = recording
        .lines
        .iter()
        .filter_map(|line| match line.event {
            Event::WallClock { unix_ms } => Some(unix_ms),
            _ => None,
        })
        .collect();

    assert_eq!(wall_clocks, [WALL_AT_START, WALL_AT_START + MIN + 10 * S]);
}

/// The EXP figures captured at `at_ms`, their pixels telling `exp` apart.
fn exp_bar_frame(exp: u64, at_ms: u64) -> Frame {
    let shade = (exp % 251) as u8;
    frame(&image::RgbaImage::from_pixel(224, 32, image::Rgba([shade, shade, shade, 255])), at_ms)
}

/// A Status Bar capture with `figures` at (1000,1100) on screen in the bar's plain slate, read
/// as Egbert's Status Bar with the EXP figures in it read as `bar`.
fn read_figures(shell: &mut Shell, figures: Frame, bar: ExpBar) {
    read_figures_at(shell, figures, bar, ScreenPoint { x: 1000, y: 1100 });
}

/// As [`read_figures`], with the figures at `at` on screen.
fn read_figures_at(shell: &mut Shell, figures: Frame, bar: ExpBar, at: ScreenPoint) {
    let at_ms = figures.at_ms;
    let area = ScreenRect { x: at.x, y: at.y, width: figures.width, height: figures.height };
    let strip = status_bar_strip();
    let img = image::RgbaImage::from_fn(strip.width, strip.height, |x, y| {
        let at = ScreenPoint { x: strip.x + x as i32, y: strip.y + y as i32 };
        if !area.holds(at) {
            return image::Rgba([58, 67, 72, 255]);
        }
        let i = 4 * ((at.y - area.y) as usize * area.width as usize + (at.x - area.x) as usize);
        image::Rgba(figures.rgba[i..i + 4].try_into().unwrap())
    });
    let capture = StatusBarCapture { frame: frame(&img, at_ms), origin: ScreenPoint { x: strip.x, y: strip.y } };
    let exp = ExpBarReading { bar: Some(bar), at_ms, place: Some(ExpBarPlace { area, remembered: false }) };
    shell.minerva.on_status_bar_read(capture, status_bar_reading(Some("Egbert"), at_ms), Some(exp));
}

/// Read the EXP bar from a frame every second from `from` to `until` as `exp` says.
fn read_frames(shell: &mut Shell, until: u64, exp: impl Fn(u64) -> ExpBar) {
    while shell.at < until {
        shell.at += S;
        let bar = exp(shell.at);
        read_figures(shell, exp_bar_frame(bar.exp, shell.at), bar);
        shell.step();
    }
}

#[test]
fn the_exp_bars_crops_are_kept_where_they_changed_for_10_minutes_and_for_the_run_where_they_misread() {
    let mut shell = Shell::new(FakeSource::osms());
    // 1,000 EXP of a 20,000 EXP level, 5 more every 4 s, misread once at 0:30.
    let exp = |at: u64| {
        let exp = 1000 + 5 * (at / (4 * S));
        if at == 30 * S {
            return ExpBar { exp: 84_350, percent_hundredths: 4172 };
        }
        ExpBar { exp, percent_hundredths: percent(exp, 20_000) }
    };
    read_frames(&mut shell, 20 * MIN, exp);

    let (_, recording) = shell.report();
    let crop = |at: u64| format!("crops/exp-bar-{at:010}.png");

    assert!(recording.crops.contains(&crop(30 * S)), "the misread's crop stays for the run");
    assert!(!recording.crops.contains(&crop(8 * S)), "an ordinary crop goes after 10 minutes");
    assert!(recording.crops.contains(&crop(20 * MIN)), "a crop that changed, in the last 10 minutes");
    assert!(!recording.crops.contains(&crop(20 * MIN - S)), "one that didn't change isn't kept");
    // A crop every 4 s for 10 minutes, the misread, and the one after it.
    assert!((150..=153).contains(&recording.crops.len()), "{} crops", recording.crops.len());
}

/// A frame of noise as big as the Status Bar strip, which no PNG compresses: about 980 KB.
fn noise_frame(seed: u64, at_ms: u64) -> Frame {
    let ScreenRect { width, height, .. } = status_bar_strip();
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    let rgba = (0..width * height * 4)
        .map(|_| {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (state >> 56) as u8
        })
        .collect();
    Frame { width, height, rgba, at_ms }
}

#[test]
fn the_exp_figures_read_in_a_status_bar_capture_are_written_with_where_they_were_read_and_replay() {
    let mut shell = Shell::new(FakeSource::osms());
    let exp = |at: u64| {
        let exp = 1000 + 5 * (at / (4 * S));
        ExpBar { exp, percent_hundredths: percent(exp, 20_000) }
    };
    read_frames(&mut shell, 30 * S, exp);

    let (report, recording) = shell.report();

    let area = ScreenRect { x: 1000, y: 1100, width: 224, height: 32 };
    let places: Vec<_> = recording.lines.iter().filter_map(|line| match line.event {
        Event::ExpBar { place, .. } => Some(place),
        _ => None,
    }).collect();
    assert!(!places.is_empty());
    assert!(places.iter().all(|&place| place == Some(ExpBarPlace { area, remembered: false })), "{places:?}");
    let crop = recording.crops.iter().find(|name| name.starts_with("crops/exp-bar-")).expect("a crop of the figures");
    let png = image::load_from_memory(&zip_bytes(&report.zip, crop)).expect("a PNG").to_rgba8();
    assert_eq!(png.dimensions(), (224, 32), "the figures, cut from the capture");
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
}

/// 20 crops of the EXP figures, of noise, a minute apart, the first a misread, after the level
/// is learned: over 5 MB of them, more than a report keeps.
fn read_noise(shell: &mut Shell) {
    let first = ExpBar { exp: 1000, percent_hundredths: 500 };
    shell.minerva.on_exp_bar_reading(ExpBarReading { bar: Some(first), at_ms: 30 * S, place: None });
    for n in 0..20 {
        shell.at += MIN;
        let bar = if n == 0 {
            ExpBar { exp: 84_350, percent_hundredths: 4172 }
        } else {
            ExpBar { exp: 1000 + n, percent_hundredths: percent(1000 + n, 20_000) }
        };
        // As much of the Status Bar as is captured: a crop of the figures as big as they come.
        let figures = noise_frame(n, shell.at);
        let strip = status_bar_strip();
        read_figures_at(shell, figures, bar, ScreenPoint { x: strip.x, y: strip.y });
        shell.step();
    }
}

#[test]
fn a_report_stays_under_5_mb_leaving_out_the_oldest_ordinary_crops_first() {
    let mut shell = Shell::new(FakeSource::osms());
    read_noise(&mut shell);

    let (report, recording) = shell.report();

    assert!(report.zip.len() <= MAX_TEXT_AND_CROPS_BYTES, "{} bytes", report.zip.len());
    assert!(recording.report.crops_left_out > 0);
    // The kept ones: the EXP bar's misread, and the Status Bar's first, its name not yet the
    // Character's.
    assert_eq!(recording.report.crops + recording.report.crops_left_out, 13, "the last 10 minutes and the kept ones");
    assert!(recording.crops.contains(&format!("crops/exp-bar-{:010}.png", MIN)), "the kept one stays");
    assert!(recording.crops.contains(&format!("crops/exp-bar-{:010}.png", 20 * MIN)), "the newest stays");
}

#[test]
fn errors_from_the_shell_and_the_windows_are_in_the_trace_and_the_summary() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.note_error("shell", "capture failed: the screen is locked");
    shell.play(20 * S, CONSTRUCTION_SITE, grind);

    let (report, recording) = shell.report();
    let summary = text_in(&report.zip, "summary.txt");

    assert!(recording.lines.iter().any(|line| line.event
        == Event::Error { from: "shell".into(), message: "capture failed: the screen is locked".into() }));
    assert!(summary.contains("shell error: capture failed: the screen is locked"), "{summary}");
}

#[test]
fn the_summary_says_what_showed_at_the_press_and_what_the_core_made_of_things() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (report, _) = shell.report();
    let summary = text_in(&report.zip, "summary.txt");

    assert!(summary.contains("Version: 0.7.0"), "{summary}");
    assert!(summary.contains("Saved: 2026-10-11 14:03:27, 0:02:00 after Minerva started"), "{summary}");
    assert!(summary.contains("Map: Victoria Road · Construction Site North of Kerning City (010003080)"), "{summary}");
    assert!(summary.contains("Grind Tracker: Active"), "{summary}");
    assert!(summary.contains("0:00:30  EXP bar read 18,325 [91.63%]: death: 400 EXP lost"), "{summary}");
    assert!(summary.contains("level-up"), "{summary}");
}

#[test]
fn the_report_holds_the_maps_shown_with_their_map_summaries_and_no_osms_files() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(MIN, CONSTRUCTION_SITE, grind);
    shell.play(2 * MIN, SUNSET_SKY, grind);

    let (report, recording) = shell.report();
    let ids: Vec<&str> = recording.report.maps.iter().map(|map| map.id.as_str()).collect();

    assert_eq!(ids, ["010003070", "010003080"]);
    assert!(recording.report.maps.iter().all(|map| map.summary.is_some()));
    assert_eq!(recording.report.osms_sets, [bug_report::fingerprint(&osms::game().files())]);
    let names = zip_names(&report.zip);
    assert!(names.iter().all(|name| !name.ends_with("maps.json") && !name.ends_with("monsters.json")), "{names:?}");
}

#[test]
fn the_report_is_named_by_version_and_local_time() {
    let mut shell = Shell::new(FakeSource::osms());

    let (report, recording) = shell.report();

    assert_eq!(report.file_name, "minerva-bug-report-0.7.0-2026-10-11-140327.zip");
    assert_eq!(recording.report.version, "0.7.0");
    assert_eq!(recording.report.os, "Windows 11, build 26100");
}

#[test]
fn a_run_over_a_day_keeps_the_last_day_and_the_lines_that_set_it_up() {
    let mut shell = Shell::new(FakeSource::osms());
    // A long line a minute for 30 hours: errors are the cheapest.
    for n in 0..30 * 60 {
        shell.at += MIN;
        shell.minerva.note_error("shell", &format!("{n:0>2000}"));
        shell.step();
    }

    let (_, recording) = shell.report();

    // The trace goes a chunk at a time, about 2 hours of these lines.
    assert!(recording.report.truncated);
    assert!((3 * HOUR..=6 * HOUR).contains(&recording.report.trace_from_ms), "{}", recording.report.trace_from_ms);
    assert!(matches!(recording.lines[0].event, Event::Restore { .. }), "{:?}", recording.lines[0]);
    assert!(matches!(recording.lines[1].event, Event::RestoreBests { .. }), "{:?}", recording.lines[1]);
    assert!(recording.lines[2].t >= recording.report.trace_from_ms);
}

#[test]
fn the_timeline_reads_what_came_in_and_what_came_of_it() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(2 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let timeline = recording.timeline(110 * S);

    assert!(timeline.iter().any(|line| line.starts_with("0:00:40  EXP bar read 84,350 [41.72%]  →  doesn't fit")), "{timeline:#?}");
    assert!(timeline.iter().any(|line| line.contains("map Victoria Road · Construction Site North of Kerning City (010003080)")));
    assert!(timeline.iter().any(|line| line.contains("level-up")));
    assert!(timeline.iter().any(|line| line.starts_with("0:01:55  EXP bar read")), "every reading from 1:50 on");
    assert!(!timeline.iter().any(|line| line.starts_with("0:01:05  EXP bar read")), "only what's notable before");
}

fn zip_names(zip: &[u8]) -> Vec<String> {
    let archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).expect("a zip");
    archive.file_names().map(String::from).collect()
}

fn text_in(zip: &[u8], name: &str) -> String {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).expect("a zip");
    let mut text = String::new();
    archive.by_name(name).expect(name).read_to_string(&mut text).expect("text");
    text
}

/// Made-up item data for the Item Card: two Blue Moons to pick between, and Bronze Ore.
fn items() -> ItemWorld {
    item_world(vec![
        etc(4_010_000, "Bronze Ore").sells_for(450),
        equip(1_041_078, "Blue Moon", "Top").sells_for(11_000),
        equip(1_032_011, "Blue Moon", "Earring").sells_for(10_500),
    ])
}

/// The Item Tooltip by the pointer read as `name`, after the player pressed Ctrl+Shift+I.
fn item_card(shell: &mut Shell, name: &str) {
    assert!(shell.minerva.on_hotkey("ctrl+shift+i").is_some());
    let tooltip = ItemTooltip::Found { area: ScreenRect { x: 900, y: 620, width: 200, height: 19 }, name: Some(name.into()) };
    let pointer = ScreenPoint { x: 900, y: 600 };
    shell.minerva.on_item_tooltip_reading(Some(ItemTooltipReading { tooltip, pointer, at_ms: shell.at }));
}

/// A run with Item Cards: the market prices kept restored (as 0.8.0 saved them), a card and
/// its market price typed (first not a price, then one), Market Lines added (first with a
/// name taken), renamed and removed, prices a tool of the player's own left (ADR 0013), a
/// choice and its pick, the card's MeowDB page and its ×.
fn run_with_item_cards() -> Shell {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    let kept = r#"{"version":1,"items":{"01041078":{"mesos":30000,"setAtUnixMs":1791700000000}}}"#;
    shell.minerva.restore_market_prices(serde_json::from_str(kept).unwrap());
    shell.minerva.set_item_card_size(Size { width: 280, height: 160 });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    item_card(&mut shell, "Bronze Ore");
    shell.minerva.edit_card_market_price("1");
    shell.minerva.enter_card_market("lots");
    shell.minerva.enter_card_market("450k");
    shell.minerva.act(Action::AddCardMarketLine);
    shell.minerva.enter_card_market_line("your price");
    shell.minerva.enter_card_market_line("Lowest ask");
    shell.minerva.edit_card_market_price("2");
    shell.minerva.enter_card_market("400k");
    shell.minerva.edit_card_market_line("2");
    shell.minerva.enter_card_market_line("Cheapest");
    shell.minerva.edit_card_market_line("1");
    shell.minerva.act(Action::AskRemoveCardMarketLine);
    shell.minerva.act(Action::CancelCardMarket);
    shell.minerva.edit_card_market_line("2");
    shell.minerva.act(Action::AskRemoveCardMarketLine);
    shell.minerva.act(Action::RemoveCardMarketLine);
    let import = r#"{"version":2,"lines":[{"id":"tool-typical","name":"Your price","prices":{"04010000":{"mesos":500000,"setAtUnixMs":1791700000000}}}]}"#;
    shell.minerva.import_market_prices(serde_json::from_str(import).unwrap());
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    item_card(&mut shell, "Blue Moon");
    shell.minerva.pick_card_item("01032011");
    shell.minerva.open_card_meowdb_page(shell.at);
    shell.play(30 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.close_item_card();
    shell.play(40 * S, CONSTRUCTION_SITE, grind);
    shell
}

#[test]
fn item_cards_replay_through_the_same_states_with_the_runs_item_files() {
    let (_, recording) = run_with_item_cards().report();

    let with = ReplayData { osms: Some(osms::game().files()), items: Some(items().files()) };
    let replay = bug_report::replay_with(&recording, with);

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
}

#[test]
fn searches_replay_through_the_same_states_with_the_runs_files() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.minerva.set_item_card_size(Size { width: 280, height: 160 });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.on_hotkey("ctrl+shift+f");
    shell.minerva.search_query("sunset");
    shell.minerva.search_pick_destination(SUNSET_SKY_ID);
    shell.play(20 * S, SUNSET_SKY, grind);
    shell.minerva.act(Action::OpenSearch);
    shell.minerva.search_query("blue moon");
    shell.minerva.act(Action::SearchNext);
    shell.minerva.act(Action::SearchEnter);
    shell.play(25 * S, SUNSET_SKY, grind);
    shell.minerva.act(Action::OpenSearch);
    shell.minerva.search_query("ore");
    shell.minerva.search_pick_item("04010000");
    shell.play(30 * S, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();
    let with = ReplayData { osms: Some(osms::game().files()), items: Some(items().files()) };
    let replay = bug_report::replay_with(&recording, with);

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let said: Vec<String> = recording.timeline(0).into_iter().filter(|line| line.contains("search picked")).collect();
    assert_eq!(said.len(), 2, "{said:?}");
}

#[test]
fn the_report_holds_only_the_item_files_fingerprint() {
    let (report, recording) = run_with_item_cards().report();

    let fingerprints: Vec<&str> = recording
        .lines
        .iter()
        .flat_map(|line| &line.answers)
        .filter_map(|answer| match answer {
            Answer::Items { answer: Ok(fingerprint) } => Some(fingerprint.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(fingerprints, [bug_report::item_fingerprint(&items().files())]);
    let names = zip_names(&report.zip);
    assert!(names.iter().all(|name| !name.ends_with("items.json") && !name.ends_with("cash_shop.json")), "{names:?}");
}

/// The game's client area as the shell captures it for an Item Card or Item Lookup press:
/// 1920x1200 at the screen's origin, each pixel's shade telling where it is, so a crop of
/// it can be told apart by its corner.
fn client_capture(pointer: ScreenPoint, at_ms: u64) -> ItemCapture {
    let img = image::RgbaImage::from_fn(1920, 1200, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, 40, 255]));
    ItemCapture { frame: frame(&img, at_ms), origin: ScreenPoint { x: 0, y: 0 }, pointer }
}

/// The player pressed Ctrl+Shift+I over (900,600) at `at_ms`, and the capture read as
/// `name` (`None`: unreadable).
fn press_item_card(shell: &mut Shell, name: Option<&str>, at_ms: u64) {
    let pointer = ScreenPoint { x: 900, y: 600 };
    assert!(shell.minerva.on_hotkey("ctrl+shift+i").is_some());
    let area = ScreenRect { x: 916, y: 650, width: 200, height: 19 };
    let reading =
        ItemTooltipReading { tooltip: ItemTooltip::Found { area, name: name.map(String::from) }, pointer, at_ms };
    shell.minerva.on_item_capture_read(client_capture(pointer, at_ms), reading);
}

fn item_crops(recording: &Recording) -> Vec<&str> {
    recording.crops.iter().map(String::as_str).filter(|name| name.starts_with("crops/item-tooltip-")).collect()
}

#[test]
fn an_item_tooltip_that_couldnt_be_read_keeps_its_capture_around_the_pointer_for_the_run() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    press_item_card(&mut shell, None, 10 * S);
    shell.play(20 * MIN, CONSTRUCTION_SITE, grind);

    let (report, recording) = shell.report();

    let name = format!("crops/item-tooltip-{:010}@400,400.png", 10 * S);
    assert_eq!(item_crops(&recording), [name.as_str()]);
    let line = recording.lines.iter().find(|line| matches!(line.event, Event::ItemTooltip { .. })).expect("its line");
    assert_eq!(line.crop.as_deref(), Some(name.as_str()), "the line names its crop");
    let crop = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (800, 800), "400 px each way from the pointer");
    assert_eq!(crop.get_pixel(0, 0).0, [(500 % 256) as u8, 200, 40, 255], "its corner at (500,200)");
}

#[test]
fn an_item_tooltip_read_as_no_item_keeps_its_capture_for_the_run() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    press_item_card(&mut shell, Some("Stlrge Wlng Zz"), 10 * S);
    shell.play(20 * MIN, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();

    assert_eq!(item_crops(&recording), [format!("crops/item-tooltip-{:010}@400,400.png", 10 * S)]);
}

#[test]
fn an_item_tooltip_read_as_an_item_keeps_its_capture_for_10_minutes() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    press_item_card(&mut shell, Some("Bronze Ore"), 10 * S);

    shell.play(5 * MIN, CONSTRUCTION_SITE, grind);
    let (_, recording) = shell.report();
    assert_eq!(item_crops(&recording).len(), 1, "kept 10 minutes, as the EXP bar's are");

    shell.play(20 * MIN, CONSTRUCTION_SITE, grind);
    let (_, recording) = shell.report();
    assert_eq!(item_crops(&recording), Vec::<&str>::new());
}

#[test]
fn a_capture_by_the_game_windows_edge_is_cropped_to_the_window() {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    assert!(shell.minerva.on_hotkey("ctrl+shift+i").is_some());
    let pointer = ScreenPoint { x: 1800, y: 100 };
    let tooltip = ItemTooltip::Found { area: ScreenRect { x: 1500, y: 150, width: 200, height: 19 }, name: None };
    shell.minerva.on_item_capture_read(client_capture(pointer, 10 * S), ItemTooltipReading { tooltip, pointer, at_ms: 10 * S });

    let (report, recording) = shell.report();

    let name = format!("crops/item-tooltip-{:010}@400,100.png", 10 * S);
    assert_eq!(item_crops(&recording), [name.as_str()]);
    let crop = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (520, 500), "from (1400,0) to the window's right edge, and 400 px down");
}

fn zip_bytes(zip: &[u8], name: &str) -> Vec<u8> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).expect("a zip");
    let mut bytes = Vec::new();
    archive.by_name(name).expect(name).read_to_end(&mut bytes).expect("bytes");
    bytes
}

/// A Player Shop's list in the Deal Finder's captures, at (561,513), its listings read as
/// `names` (`None`: unreadable), each for 999 mesos.
fn shop_reading(names: &[Option<&str>], at_ms: u64) -> ShopReading {
    let rows = names
        .iter()
        .enumerate()
        .map(|(slot, name)| ShopRow {
            area: ScreenRect { x: 619, y: 513 + 63 * slot as i32, width: 253, height: 54 },
            name: name.map(String::from),
            price: Some(ListedPrice { quantity: 1, mesos: 999 }),
            covered: false,
            tier: None,
        })
        .collect();
    ShopReading { shop: Some(ShopList { area: ScreenRect { x: 561, y: 513, width: 311, height: 366 }, rows }), hover: None, at_ms, near_miss: None }
}

/// The game's client area as the shell captures it for the Deal Finder.
fn shop_capture(at_ms: u64) -> ShopCapture {
    let ItemCapture { frame, origin, .. } = client_capture(ScreenPoint { x: 0, y: 0 }, at_ms);
    ShopCapture { frame, origin, pointer: None }
}

/// A run with the Deal Finder: turned on, a shop read the same a few times, then with a
/// listing unreadable, the Deal Line chosen, the margin set, a badge clicked, and off.
fn run_with_the_deal_finder() -> Shell {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.minerva.restore_market_prices(serde_json::from_str(r#"{"version":2,"lines":[{"id":"1","name":"Your price"},{"id":"2","name":"Lowest ask","prices":{"04010000":{"mesos":1200}}}]}"#).unwrap());
    shell.minerva.set_deal_badges_size(Size { width: 160, height: 0 });
    shell.minerva.set_item_card_size(Size { width: 280, height: 160 });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    assert!(shell.minerva.on_hotkey("ctrl+shift+d").is_none());
    for at in [10_400, 10_800, 11_200] {
        shell.minerva.on_player_shop_read(shop_capture(at), shop_reading(&[Some("Bronze Ore"), Some("Blue Moon")], at));
    }
    shell.minerva.on_player_shop_read(shop_capture(11_600), shop_reading(&[Some("Bronze Ore"), None], 11_600));
    shell.minerva.choose_deal_line("2");
    shell.minerva.act(Action::SetDealMargin { percent: 5 });
    shell.minerva.act(Action::OpenDealItem { row: 0 });
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.on_player_shop_reading(ShopReading { shop: None, hover: None, at_ms: 20 * S, near_miss: None });
    shell.minerva.act(Action::ToggleDealFinder);
    shell.play(30 * S, CONSTRUCTION_SITE, grind);
    shell
}

#[test]
fn the_deal_finder_replays_through_the_same_states_with_the_runs_item_files() {
    let (_, recording) = run_with_the_deal_finder().report();

    let with = ReplayData { osms: Some(osms::game().files()), items: Some(items().files()) };
    let replay = bug_report::replay_with(&recording, with);

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains(r#"Player Shop read: "Bronze Ore" 1 for 999, ? 1 for 999"#), "{timeline}");
    assert!(timeline.contains("market line 2 chosen as the Deal Line"), "{timeline}");
}

/// The pointer on a listing whose name the game cut short so that it could be either Pet
/// Equip Speed Scroll, and the Item Tooltip by it naming it: the Deal Finder's reading is
/// written down with the pointer and the tooltip, replays to the same badges, and the
/// report keeps the part of the capture around the pointer as an Item Card press's.
#[test]
fn a_shop_read_with_the_pointer_on_a_listing_replays_with_its_tooltip_and_keeps_the_tooltip() {
    let items = || {
        items()
            .with_scroll(scroll(2_048_000, "Pet Equip Speed Scroll: Legs"))
            .with_scroll(scroll(2_048_001, "Pet Equip Speed Scroll: Lungs"))
    };
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.minerva.set_deal_badges_size(Size { width: 160, height: 0 });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    assert!(shell.minerva.on_hotkey("ctrl+shift+d").is_none());
    let scroll_listed = || shop_reading(&[Some("Pet Equip Speed Scroll: L...")], 10_400);
    shell.minerva.on_player_shop_read(shop_capture(10_400), scroll_listed());
    let pointer = ScreenPoint { x: 580, y: 530 };
    let area = ScreenRect { x: 590, y: 570, width: 300, height: 18 };
    let hover = ShopHover { pointer, tooltip: ItemTooltip::Found { area, name: Some("Pet Equip Speed Scroll: Legs".into()) } };
    let capture = ShopCapture { pointer: Some(pointer), ..shop_capture(10_800) };
    let state = shell.minerva.on_player_shop_read(capture, ShopReading { hover: Some(hover), at_ms: 10_800, ..scroll_listed() });
    let badge = &state.deal_badges.as_ref().expect("badges").badges[0];
    assert!(matches!(&badge.verdict, Verdict::NoPrice { item, .. } if item.name == "Pet Equip Speed Scroll: Legs"), "{badge:?}");
    // The pointer moves along the listing, the tooltip with it, saying the same.
    let moved = ScreenPoint { x: 640, y: 540 };
    let area = ScreenRect { x: 650, y: 580, ..area };
    let hover = ShopHover { pointer: moved, tooltip: ItemTooltip::Found { area, name: Some("Pet Equip Speed Scroll: Legs".into()) } };
    let capture = ShopCapture { pointer: Some(moved), ..shop_capture(11_200) };
    shell.minerva.on_player_shop_read(capture, ShopReading { hover: Some(hover), at_ms: 11_200, ..scroll_listed() });

    let (_, recording) = shell.report();

    let replay = bug_report::replay_with(&recording, ReplayData { osms: Some(osms::game().files()), items: Some(items().files()) });
    assert_eq!(replay.divergence, None);
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains(r#"the pointer on it at 580, 530, its tooltip "Pet Equip Speed Scroll: Legs""#), "{timeline}");
    let crop = format!("crops/item-tooltip-{:010}@400,400.png", 10_800);
    let tooltips: Vec<&String> = recording.crops.iter().filter(|name| name.starts_with("crops/item-tooltip-")).collect();
    assert_eq!(tooltips, [&crop], "one crop while the tooltip says the same");
    assert!(timeline.contains(&format!("[{crop}]")), "{timeline}");
}

/// A listing whose name the game cut short so that it could be any of the four One-Handed
/// Sword Attack Scrolls, its picture a Greater's: the tier is written down with the reading,
/// replays to the same badge, and reads in the timeline.
#[test]
fn a_listings_picture_tier_replays_and_reads_in_the_timeline() {
    let with_swords = || {
        ["Lesser", "Intermediate", "Greater", "Chaos"].into_iter().zip(2_043_000..).fold(items(), |items, (tier, id)| {
            items.with_scroll(scroll(id, &format!("One-Handed Sword Attack Scroll: {tier}")).of_tier(tier))
        })
    };
    let mut shell = Shell::new(FakeSource::osms().with_items(with_swords()));
    shell.minerva.set_deal_badges_size(Size { width: 160, height: 0 });
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    assert!(shell.minerva.on_hotkey("ctrl+shift+d").is_none());
    let mut reading = shop_reading(&[Some("One-Handed Sword Attac..."), Some("Bronze Ore")], 10_400);
    reading.shop.as_mut().expect("a shop").rows[0].tier = Some(ScrollTier::Greater);
    let state = shell.minerva.on_player_shop_read(shop_capture(10_400), reading);
    let badge = &state.deal_badges.as_ref().expect("badges").badges[0];
    assert!(matches!(&badge.verdict, Verdict::NoPrice { item, .. } if item.name == "One-Handed Sword Attack Scroll: Greater"), "{badge:?}");

    let (_, recording) = shell.report();

    let replay = bug_report::replay_with(&recording, ReplayData { osms: Some(osms::game().files()), items: Some(with_swords().files()) });
    assert_eq!(replay.divergence, None);
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains(r#"Player Shop read: "One-Handed Sword Attac..." (a Greater scroll's picture) 1 for 999, "Bronze Ore" 1 for 999"#), "{timeline}");
}

#[test]
fn a_shop_read_the_same_as_before_is_written_as_such_with_no_crop() {
    let (_, recording) = run_with_the_deal_finder().report();

    let shops: Vec<&Event> = recording.lines.iter().map(|line| &line.event).filter(|e| matches!(e, Event::PlayerShop { .. })).collect();
    assert!(matches!(shops[0], Event::PlayerShop { shop: Some(_), same: false, .. }));
    assert!(matches!(shops[1], Event::PlayerShop { shop: None, same: true, .. }));
    assert!(matches!(shops[2], Event::PlayerShop { same: true, .. }));
    assert!(matches!(shops[3], Event::PlayerShop { shop: Some(_), same: false, .. }));
    let crops: Vec<&str> = recording.crops.iter().map(String::as_str).filter(|name| name.starts_with("crops/player-shop-")).collect();
    assert_eq!(crops.len(), 2, "the first reading's and the one that changed: {crops:?}");
}

#[test]
fn a_shop_with_a_listing_that_couldnt_be_read_keeps_its_list_for_the_run() {
    let mut shell = run_with_the_deal_finder();
    shell.play(20 * MIN, CONSTRUCTION_SITE, grind);

    let (report, recording) = shell.report();

    let name = format!("crops/player-shop-{:010}.png", 11_600);
    let crops: Vec<&str> = recording.crops.iter().map(String::as_str).filter(|name| name.starts_with("crops/player-shop-")).collect();
    assert_eq!(crops, [name.as_str()], "only the one with a listing unread outlives 10 minutes");
    let crop = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (362, 386), "the list, 10 px around it and past its scroll bar");
    assert_eq!(crop.get_pixel(0, 0).0, [(551 % 256) as u8, (503 % 256) as u8, 40, 255], "its corner at (551,503)");
}

/// The pointer on a listing breaks its band, and the game's tooltip by it covers those below,
/// so the bands found end above them (#135's INT scroll): the crop still reaches down to every
/// listing, the covered ones it is kept for included.
#[test]
fn a_shops_list_is_cut_down_to_its_covered_listings_past_the_bands_found() {
    let mut shell = run_with_the_deal_finder();
    let row = |slot: i32, name: Option<&str>| ShopRow {
        area: ScreenRect { x: 619, y: 513 + 63 * slot, width: 253, height: 54 },
        name: name.map(String::from),
        price: name.map(|_| ListedPrice { quantity: 1, mesos: 999 }),
        covered: name.is_none(),
        tier: None,
    };
    let list = ShopList { area: ScreenRect { x: 561, y: 513, width: 311, height: 70 }, rows: vec![row(0, Some("Bronze Ore")), row(1, None), row(2, None)] };
    assert!(shell.minerva.on_hotkey("ctrl+shift+d").is_none());
    shell.minerva.on_player_shop_read(shop_capture(31 * S), ShopReading { shop: Some(list), hover: None, at_ms: 31 * S, near_miss: None });

    let (report, _) = shell.report();

    let name = format!("crops/player-shop-{:010}.png", 31 * S);
    let crop = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (362, 200), "from 10 px above the list (503) to 10 px below its third listing (693)");
}

/// #130's UI Size (ADR 0020): a listing's band 162 px wide, not 253, so the crop's margin
/// and its reach past the scroll bar are as much smaller, and none of the shop's window
/// beyond is kept.
#[test]
fn a_shops_list_at_another_ui_size_is_cut_as_much_smaller() {
    let mut shell = run_with_the_deal_finder();
    let row = |slot: i32, name: Option<&str>| ShopRow {
        area: ScreenRect { x: 598, y: 513 + 40 * slot, width: 162, height: 35 },
        name: name.map(String::from),
        price: Some(ListedPrice { quantity: 1, mesos: 999 }),
        covered: false,
        tier: None,
    };
    let list = ShopList { area: ScreenRect { x: 561, y: 513, width: 199, height: 235 }, rows: vec![row(0, Some("Bronze Ore")), row(1, None)] };
    assert!(shell.minerva.on_hotkey("ctrl+shift+d").is_none());
    shell.minerva.on_player_shop_read(shop_capture(31 * S), ShopReading { shop: Some(list), hover: None, at_ms: 31 * S, near_miss: None });

    let (report, _) = shell.report();

    let name = format!("crops/player-shop-{:010}.png", 31 * S);
    let crop = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (231, 247), "the list, 6 px around it and 225 px right of it");
    assert_eq!(crop.get_pixel(0, 0).0, [(555 % 256) as u8, (507 % 256) as u8, 40, 255], "its corner at (555,507)");
}

/// The bottom of the game's client area as the shell captures it for the Status Bar.
fn status_bar_capture(at_ms: u64) -> StatusBarCapture {
    let strip = status_bar_strip();
    let img = image::RgbaImage::from_fn(strip.width, strip.height, |x, y| {
        image::Rgba([(x % 256) as u8, ((y + strip.y as u32) % 256) as u8, 40, 255])
    });
    StatusBarCapture { frame: frame(&img, at_ms), origin: ScreenPoint { x: strip.x, y: strip.y } }
}

/// The Status Bar read as a level 13 Magician named `name` (`None`: unreadable).
fn status_bar_reading(name: Option<&str>, at_ms: u64) -> StatusBarReading {
    let area = ScreenRect { x: 334, y: 1145, width: 331, height: 55 };
    StatusBarReading {
        bar: Some(StatusBar { area, level: Some(13), job: Some("Magician".into()), name: name.map(String::from), exp_figures: None }),
        at_ms,
        near_miss: None,
    }
}

/// A run with Characters: Egbert's Status Bar read every second with the EXP bar, then the
/// character select screen, then Bob's, three times to name him.
fn run_with_characters() -> Shell {
    let mut shell = Shell::new(FakeSource::osms());
    for second in 1..=10 {
        shell.play(second * S, CONSTRUCTION_SITE, grind);
        let name = if second == 5 { None } else { Some("Egbert") };
        shell.minerva.on_status_bar_read(status_bar_capture(second * S), status_bar_reading(name, second * S), None);
    }
    for second in 11..=15 {
        shell.play(second * S, CONSTRUCTION_SITE, |_| None);
        shell.minerva.on_status_bar_reading(StatusBarReading { bar: None, at_ms: second * S, near_miss: None });
    }
    for second in 16..=18 {
        shell.minerva.on_status_bar_read(status_bar_capture(second * S), status_bar_reading(Some("Bob"), second * S), None);
    }
    shell.play(30 * S, CONSTRUCTION_SITE, grind);
    shell
}

#[test]
fn characters_replay_through_the_same_states() {
    let (_, recording) = run_with_characters().report();

    let replay = bug_report::replay(&recording, Some(osms::game().files()));

    assert_eq!(replay.divergence, None);
    assert_eq!(replay.lines, recording.lines.len());
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains(r#"Status Bar read: level 13, "Magician", "Egbert""#), "{timeline}");
    assert!(timeline.contains(r#"Status Bar read: level 13, "Magician", ?"#), "{timeline}");
    assert!(timeline.contains(r#"the Status Bar names another character, "Bob": a new Session"#), "{timeline}");
    assert!(recording.lines.iter().any(|line| line.notes.contains(&Note::NameSwitch { name: "Bob".into() })));
}

#[test]
fn item_names_cut_short_and_matching_no_item_are_written_down_for_tuning_the_matching() {
    use minerva_core::{PickupLine, PickupLogReading, PickupLogSeen};
    let items = item_world(vec![
        common::items::use_item(2_060_000, "Arrows for Crossbow"),
        common::items::use_item(2_061_000, "Arrows for Bow"),
    ]);
    let mut shell = Shell::new(FakeSource::osms().with_items(items));
    shell.play(12 * S, CONSTRUCTION_SITE, grind);
    let line = |name: &str, cut_short| PickupLine::Item { name: name.into(), quantity: 2, tab: "Use".into(), cut_short };
    let arrived = vec![line("Arrows for", true), line("Zorg Dung", false), line("Arrows for Crossbow", false)];
    let area = ScreenRect { x: 1500, y: 800, width: 408, height: 160 };
    shell.minerva.on_pickup_log_reading(PickupLogReading {
        at_ms: 12 * S + 100,
        seen: PickupLogSeen::Clear { area },
        rows: vec![],
        shift: 0,
        arrived,
        chose: false,
        overflowed: false,
    });
    shell.play(13 * S, CONSTRUCTION_SITE, grind);

    let (_, recording) = shell.report();

    let notes: Vec<&Note> = recording.lines.iter().flat_map(|line| &line.notes).collect();
    assert!(notes.contains(&&Note::ItemCutShort { name: "Arrows for...".into() }), "{notes:?}");
    assert!(notes.contains(&&Note::ItemUnmatched { name: "Zorg Dung".into(), lines: 1 }), "{notes:?}");
    assert_eq!(notes.iter().filter(|note| matches!(note, Note::ItemCutShort { .. } | Note::ItemUnmatched { .. })).count(), 2);
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains(r#"Pickup Log item "Zorg Dung" matches no item"#), "{timeline}");
}

#[test]
fn a_status_bar_read_the_same_as_before_is_written_as_such_with_no_crop() {
    let (report, recording) = run_with_characters().report();

    let bars: Vec<&Event> = recording.lines.iter().map(|line| &line.event).filter(|e| matches!(e, Event::StatusBar { .. })).collect();
    assert!(matches!(bars[0], Event::StatusBar { bar: Some(_), same: false, .. }));
    assert!(matches!(bars[1], Event::StatusBar { bar: None, same: true, .. }));
    let crops: Vec<&str> = recording.crops.iter().map(String::as_str).filter(|name| name.starts_with("crops/status-bar-")).collect();
    // Egbert's, his name unread, Egbert's again, and Bob's.
    assert_eq!(crops.len(), 4, "{crops:?}");
    let crop = image::load_from_memory(&zip_bytes(&report.zip, crops[0])).expect("a PNG").to_rgba8();
    assert_eq!(crop.dimensions(), (351, 65), "the level, job and name, 10 px around them, to the client's bottom");
    assert_eq!(crop.get_pixel(0, 0).0, [(324 % 256) as u8, (1135 % 256) as u8, 40, 255], "its corner at (324,1135)");
}

#[test]
fn readings_for_a_region_no_longer_captured_are_left_and_say_so_and_replay() {
    let mut shell = Shell::new(FakeSource::osms());
    shell.play(5 * S, SUNSET_SKY, grind);
    let at_ms = shell.at;
    // The title, read in the whole client area after it was found at the minimap's top.
    let client = game().client;
    let blank = image::RgbaImage::new(client.width, client.height);
    let whole = minerva_core::TitleCapture { frame: frame(&blank, at_ms), origin: ScreenPoint { x: 0, y: 0 }, ours: Vec::new() };
    let lines = Some(TitleLines { street: CONSTRUCTION_SITE.0.into(), map: CONSTRUCTION_SITE.1.into() });
    let state = shell.minerva.on_title_read(whole, TitleReading { lines, found: Some(common::TITLE_FOUND), at_ms });
    assert_eq!(state.map.as_ref().map(|map| map.map_name.as_str()), Some(SUNSET_SKY.1));
    // The Status Bar, read in a capture taken before the game window moved.
    let capture = status_bar_capture(at_ms);
    shell.minerva.on_game_window(Some(common::window_on_wide_screen(common::GAME)));
    let state = shell.minerva.on_status_bar_read(capture, status_bar_reading(Some("Egbert"), at_ms), None);
    assert_eq!(state.character, None);
    shell.play(10 * S, SUNSET_SKY, grind);

    let (_, recording) = shell.report();
    let timeline = recording.timeline(0);
    let left = |what: &str| timeline.iter().any(|line| line.contains(what));
    assert!(
        left(r#"title read "Victoria Road" / "Construction Site North of Kerning City", for a region no longer captured (1920x1200 at (0, 0)): left"#),
        "{timeline:#?}"
    );
    assert!(
        left(r#"Status Bar read: level 13, "Magician", "Egbert", for a region no longer captured (1920x128 at (0, 1072)): left"#),
        "{timeline:#?}"
    );
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
}

/// The game's client area as the Deal Finder captures it, its blue telling `at_ms` apart, so
/// the capture a report keeps can be told by its pixels.
fn deal_finder_capture(at_ms: u64) -> ShopCapture {
    let shade = (at_ms / 400 % 251) as u8;
    let img = image::RgbaImage::from_fn(1920, 1200, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, shade, 255]));
    ShopCapture { frame: frame(&img, at_ms), origin: ScreenPoint { x: 0, y: 0 }, pointer: None }
}

/// A Deal Finder capture with no Player Shop found; with a `near` miss, a stack of bands too
/// wide for any UI Size.
fn no_shop(at_ms: u64, near: bool) -> ShopReading {
    let area = ScreenRect { x: 700, y: 400, width: 520, height: 300 };
    let near_miss = near.then_some(ShopNearMiss { area, rows: 60, brightness_hundredths: 94 });
    ShopReading { shop: None, hover: None, at_ms, near_miss }
}

/// The Deal Finder turned on, 10 s in, and its captures read as `readings` say, each taken
/// when its reading says.
fn deal_finder_reads(readings: impl IntoIterator<Item = ShopReading>) -> Shell {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    shell.play(10 * S, CONSTRUCTION_SITE, grind);
    assert!(shell.minerva.on_hotkey("ctrl+shift+d").is_none());
    for reading in readings {
        shell.minerva.on_player_shop_read(deal_finder_capture(reading.at_ms), reading);
    }
    shell.play(20 * S, CONSTRUCTION_SITE, grind);
    shell
}

/// The whole captures of `kind` (`client-area`, `status-bar-strip`) in a report.
fn whole_captures<'a>(recording: &'a Recording, kind: &str) -> Vec<&'a str> {
    let prefix = format!("crops/{kind}-");
    recording.crops.iter().map(String::as_str).filter(|name| name.starts_with(&prefix)).collect()
}

/// No shop open, then a shop the reader came near but couldn't find, twice, then no shop again
/// and a shop found: the report keeps the latest capture with the near miss, whole, and the
/// timeline says what the reader came near.
#[test]
fn a_deal_finder_capture_with_no_shop_found_is_kept_whole_the_latest_with_a_near_miss() {
    let mut readings: Vec<ShopReading> =
        [(10_400, false), (10_800, true), (11_200, true), (11_600, false)].into_iter().map(|(at, near)| no_shop(at, near)).collect();
    readings.push(shop_reading(&[Some("Bronze Ore")], 12_000));
    let mut shell = deal_finder_reads(readings);

    let (report, recording) = shell.report();

    let name = format!("crops/client-area-{:010}.png", 11_200);
    assert_eq!(whole_captures(&recording, "client-area"), [name.as_str()]);
    let png = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(png.dimensions(), (1920, 1200));
    assert!(png.as_raw() == &deal_finder_capture(11_200).frame.rgba, "the capture, pixel for pixel");
    let shops: Vec<&Event> = recording.lines.iter().map(|line| &line.event).filter(|e| matches!(e, Event::PlayerShop { .. })).collect();
    assert!(matches!(shops[1], Event::PlayerShop { near_miss: Some(_), same: false, .. }), "a near miss isn't the same as no shop");
    assert!(matches!(shops[2], Event::PlayerShop { near_miss: None, same: true, .. }), "the same near miss again");
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains("no Player Shop; nearest, 60 band rows 520 px wide at (700, 400), 520x300, at 94% brightness"), "{timeline}");
    assert!(timeline.contains(&format!("Player Shop read as before  [{name}]")), "{timeline}");
    let with = ReplayData { osms: Some(osms::game().files()), items: Some(items().files()) };
    assert_eq!(bug_report::replay_with(&recording, with).divergence, None);
}

#[test]
fn a_deal_finder_capture_with_no_shop_found_and_none_near_is_kept_whole_the_latest() {
    let readings = [no_shop(10_400, false), no_shop(10_800, false), shop_reading(&[Some("Bronze Ore")], 11_200)];
    let mut shell = deal_finder_reads(readings);

    let (report, recording) = shell.report();

    let name = format!("crops/client-area-{:010}.png", 10_800);
    assert_eq!(whole_captures(&recording, "client-area"), [name.as_str()]);
    let png = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert!(png.as_raw() == &deal_finder_capture(10_800).frame.rgba, "the capture, pixel for pixel");
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains(&format!("Player Shop read as before  [{name}]")), "{timeline}");
}

#[test]
fn deal_finder_captures_with_a_shop_found_or_none_found_over_10_minutes_ago_leave_no_whole_capture() {
    let (_, recording) = deal_finder_reads([shop_reading(&[Some("Bronze Ore")], 10_400)]).report();
    assert_eq!(whole_captures(&recording, "client-area"), Vec::<&str>::new(), "a shop found");

    let mut shell = deal_finder_reads([no_shop(10_400, true), no_shop(10_800, false)]);
    shell.play(10_800 + 10 * MIN + S, CONSTRUCTION_SITE, grind);
    let (_, recording) = shell.report();
    assert_eq!(whole_captures(&recording, "client-area"), Vec::<&str>::new(), "none found over 10 minutes ago");
}

/// The bottom of the client area as the Status Bar is captured, its blue telling `at_ms` apart.
fn strip_capture(at_ms: u64) -> StatusBarCapture {
    let strip = status_bar_strip();
    let shade = (at_ms / S % 251) as u8;
    let img = image::RgbaImage::from_fn(strip.width, strip.height, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, shade, 255]));
    StatusBarCapture { frame: frame(&img, at_ms), origin: ScreenPoint { x: strip.x, y: strip.y } }
}

/// A Status Bar capture with no bar found; with a `near` miss, two level tiles with no LV box.
fn no_bar(at_ms: u64, near: bool) -> StatusBarReading {
    let area = ScreenRect { x: 334, y: 1150, width: 35, height: 20 };
    StatusBarReading { bar: None, at_ms, near_miss: near.then_some(StatusBarNearMiss { area, tiles: 2, box_height: None }) }
}

/// No Status Bar found, the reader near one, then not, then Egbert's found: the report keeps
/// the latest strip with none found, whole, near miss or not.
#[test]
fn a_status_bar_capture_with_no_bar_found_is_kept_whole_the_latest() {
    let mut shell = Shell::new(FakeSource::osms());
    for (second, near) in [(1, true), (2, false)] {
        shell.play(second * S, CONSTRUCTION_SITE, grind);
        shell.minerva.on_status_bar_read(strip_capture(second * S), no_bar(second * S, near), None);
    }
    shell.play(3 * S, CONSTRUCTION_SITE, grind);
    shell.minerva.on_status_bar_read(status_bar_capture(3 * S), status_bar_reading(Some("Egbert"), 3 * S), None);
    shell.play(10 * S, CONSTRUCTION_SITE, grind);

    let (report, recording) = shell.report();

    let name = format!("crops/status-bar-strip-{:010}.png", 2 * S);
    assert_eq!(whole_captures(&recording, "status-bar-strip"), [name.as_str()]);
    let png = image::load_from_memory(&zip_bytes(&report.zip, &name)).expect("a PNG").to_rgba8();
    assert_eq!(png.dimensions(), (1920, STATUS_BAR_STRIP));
    assert!(png.as_raw() == &strip_capture(2 * S).frame.rgba, "the strip, pixel for pixel");
    let timeline = recording.timeline(0).join("\n");
    assert!(timeline.contains("no Status Bar; nearest, 2 level tiles at (334, 1150), 35x20, no LV box round them"), "{timeline}");
    assert!(timeline.contains(&format!("no Status Bar  [{name}]")), "{timeline}");
    assert_eq!(bug_report::replay(&recording, Some(osms::game().files())).divergence, None);
}

#[test]
fn status_bar_captures_with_a_bar_found_or_none_found_over_10_minutes_ago_leave_no_whole_capture() {
    let (_, recording) = run_with_characters().report();
    assert_eq!(whole_captures(&recording, "status-bar-strip"), Vec::<&str>::new(), "bars found");

    let mut shell = Shell::new(FakeSource::osms());
    shell.play(S, CONSTRUCTION_SITE, grind);
    shell.minerva.on_status_bar_read(strip_capture(S), no_bar(S, true), None);
    shell.play(S + 10 * MIN + S, CONSTRUCTION_SITE, grind);
    let (_, recording) = shell.report();
    assert_eq!(whole_captures(&recording, "status-bar-strip"), Vec::<&str>::new(), "none found over 10 minutes ago");
}

/// A Deal Finder capture, its top `noisy` rows noise, which no PNG compresses (4 bytes a
/// pixel), and the rest one colour.
fn noisy_deal_finder_capture(noisy: u32, at_ms: u64) -> ShopCapture {
    let mut state: u64 = 1;
    let img = image::RgbaImage::from_fn(1920, 1200, |_, y| {
        if y >= noisy {
            return image::Rgba([96, 150, 210, 255]);
        }
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let [r, g, b, ..] = state.to_be_bytes();
        image::Rgba([r, g, b, 255])
    });
    ShopCapture { frame: frame(&img, at_ms), origin: ScreenPoint { x: 0, y: 0 }, pointer: None }
}

/// Over 5 MB of crops, then the Deal Finder turned on and a capture read with no shop found.
fn crops_then_no_shop(noisy: u32) -> Shell {
    let mut shell = Shell::new(FakeSource::osms().with_items(items()));
    read_noise(&mut shell);
    assert!(shell.minerva.on_hotkey("ctrl+shift+d").is_none());
    shell.at += 400;
    let at = shell.at;
    shell.minerva.on_player_shop_read(noisy_deal_finder_capture(noisy, at), no_shop(at, true));
    shell.step();
    shell
}

#[test]
fn a_capture_with_no_shop_found_comes_on_top_of_5_mb_of_crops_and_the_report_stays_under_10_mb() {
    // About 2.3 MB as a PNG.
    let mut shell = crops_then_no_shop(300);
    let at = shell.at;

    let (report, recording) = shell.report();

    assert!(report.zip.len() > MAX_TEXT_AND_CROPS_BYTES, "{} bytes", report.zip.len());
    assert!(report.zip.len() <= MAX_REPORT_BYTES, "{} bytes", report.zip.len());
    assert!(recording.report.crops_left_out > 0, "the ordinary crops still kept to 5 MB");
    assert_eq!(whole_captures(&recording, "client-area"), [format!("crops/client-area-{at:010}.png").as_str()]);
    assert!(recording.report.captures_left_out.is_empty());
}

#[test]
fn a_capture_with_no_shop_found_that_wont_fit_is_left_out_and_the_summary_says_so() {
    // About 9.2 MB as a PNG.
    let mut shell = crops_then_no_shop(1200);
    let at = shell.at;

    let (report, recording) = shell.report();

    let name = format!("crops/client-area-{at:010}.png");
    assert!(report.zip.len() <= MAX_TEXT_AND_CROPS_BYTES, "{} bytes", report.zip.len());
    assert_eq!(whole_captures(&recording, "client-area"), Vec::<&str>::new());
    let left_out: Vec<&str> = recording.report.captures_left_out.iter().map(|left| left.name.as_str()).collect();
    assert_eq!(left_out, [name.as_str()]);
    let summary = text_in(&report.zip, "summary.txt");
    let says = format!("Left out: {name}, a whole capture where nothing was found, 9.2 MB as a PNG: too big for the report");
    assert!(summary.contains(&says), "{summary}");
}
