//! Link Discord (#152, ADR 0023): the Personal Bests window links this install's Leaderboard
//! Key to the player's Account on the site, so their Characters' names follow them to a new PC.
//! The core says when to read the link state and when to start a link; the shell does the HTTP
//! with the key, opens the link page itself, and tells the core each answer.

mod common;

use common::{CONSTRUCTION_SITE_ID, DAY, MIN, S, WALL, tracking};
use minerva_core::{
    BESTS_VERSION, CharacterBests, DiscordLinkAnswer, DiscordLinkAsk, DiscordLinkRequest, DiscordLinkView, LeaderboardAnswer,
    LeaderboardStatus, MapBests, Minerva, PanelControl, PanelState, SavedBests, Settings, VisitBest, read_link_state, read_link_token,
};

/// Egbert's one best, on the Construction Site, dated: something to upload.
fn bests() -> SavedBests {
    let visit = VisitBest { exp_per_hour: 6000, set_at_unix_ms: Some(WALL - DAY), level: Some(31), job: Some("Magician".to_string()), ..VisitBest::default() };
    let egbert = CharacterBests { name: "Egbert".to_string(), maps: vec![MapBests { id: CONSTRUCTION_SITE_ID.to_string(), visits: vec![visit] }] };
    SavedBests { version: BESTS_VERSION, unassigned: Vec::new(), characters: vec![egbert] }
}

/// A core at launch with Egbert's bests and the wall clock told, Egbert met at an earlier
/// launch and shown on the Map Leaderboards (`shown`) or not. The Personal Bests window closed.
fn egbert_at_launch(shown: bool) -> Minerva {
    let mut minerva = tracking();
    let settings = Settings { map_leaderboards_met: Some(vec!["Egbert".to_string()]), ..minerva.settings() };
    minerva.restore(settings);
    minerva.restore_bests(bests());
    minerva.on_wall_clock(WALL);
    if shown {
        minerva.show_on_leaderboards("Egbert", true);
    }
    minerva.tick(0);
    minerva
}

fn discord(state: &PanelState) -> DiscordLinkView {
    state.personal_bests.discord.clone()
}

fn asks(state: &PanelState) -> Vec<DiscordLinkAsk> {
    state.discord_link_requests.iter().map(|r| r.ask).collect()
}

/// The one request under way: its id.
fn request(state: &PanelState) -> DiscordLinkRequest {
    match state.discord_link_requests.as_slice() {
        [one] => one.clone(),
        other => panic!("one request under way, not {other:?}"),
    }
}

fn open_bests(minerva: &mut Minerva, at_ms: u64) -> PanelState {
    minerva.press(PanelControl::TogglePersonalBests);
    minerva.tick(at_ms).clone()
}

/// Answer the request under way with `answer`, then tick at `at_ms`.
fn answer(minerva: &mut Minerva, answer: DiscordLinkAnswer, at_ms: u64) -> PanelState {
    let id = request(minerva.panel_state()).id;
    minerva.on_discord_link_answer(id, answer);
    minerva.tick(at_ms).clone()
}

const NOT_LINKED: DiscordLinkView = DiscordLinkView { linked_as: None, offered: true, linking: false, failed: None, prompt: false };

#[test]
fn opening_personal_bests_reads_the_link_state_and_offers_link_discord_when_not_linked() {
    let mut minerva = egbert_at_launch(false);
    assert_eq!(minerva.panel_state().discord_link_requests, []);
    assert_eq!(discord(minerva.panel_state()), DiscordLinkView { offered: false, ..NOT_LINKED });

    let state = open_bests(&mut minerva, 10);
    assert_eq!(asks(&state), [DiscordLinkAsk::State]);
    // Not known until it answers: nothing offered.
    assert!(!discord(&state).offered);

    let state = answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20);
    assert_eq!(state.discord_link_requests, []);
    assert_eq!(discord(&state), NOT_LINKED);
}

#[test]
fn a_linked_install_shows_whose_discord_it_is_linked_to() {
    let mut minerva = egbert_at_launch(true);
    open_bests(&mut minerva, 10);

    let state = answer(&mut minerva, DiscordLinkAnswer::Linked { username: "egberticus".to_string() }, 20);

    assert_eq!(discord(&state), DiscordLinkView { linked_as: Some("egberticus".to_string()), offered: false, ..NOT_LINKED });
}

#[test]
fn the_link_state_is_read_again_at_an_opening_a_minute_after_the_last_read() {
    let mut minerva = egbert_at_launch(false);
    open_bests(&mut minerva, 10);
    answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20);
    minerva.press(PanelControl::ClosePersonalBests);
    minerva.tick(30 * S);

    assert_eq!(asks(&open_bests(&mut minerva, 40 * S)), []);
    minerva.press(PanelControl::ClosePersonalBests);
    minerva.tick(MIN + 15 * S);
    assert_eq!(asks(&open_bests(&mut minerva, MIN + 20 * S)), [DiscordLinkAsk::State]);
}

#[test]
fn a_failed_read_offers_nothing_and_is_tried_at_the_next_opening() {
    let mut minerva = egbert_at_launch(false);
    open_bests(&mut minerva, 10);

    let state = answer(&mut minerva, DiscordLinkAnswer::Failed { why: "offline".to_string() }, 20);
    assert!(!discord(&state).offered);
    assert_eq!(asks(&state), []);

    minerva.press(PanelControl::ClosePersonalBests);
    minerva.tick(5 * S);
    assert_eq!(asks(&open_bests(&mut minerva, 6 * S)), [DiscordLinkAsk::State]);
}

#[test]
fn link_discord_opens_the_link_page_then_reads_the_state_every_5_seconds_until_linked() {
    let mut minerva = egbert_at_launch(true);
    open_bests(&mut minerva, 10);
    answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20);

    minerva.link_discord();
    let state = minerva.tick(30).clone();
    assert_eq!(asks(&state), [DiscordLinkAsk::Start]);
    assert!(discord(&state).linking);

    // The shell asked for a token with the key and opened the page.
    let state = answer(&mut minerva, DiscordLinkAnswer::Opened {}, 40);
    assert_eq!(discord(&state), DiscordLinkView { linking: true, ..NOT_LINKED });
    assert_eq!(asks(&state), []);

    assert_eq!(asks(minerva.tick(5 * S)), []);
    assert_eq!(asks(minerva.tick(5 * S + 40)), [DiscordLinkAsk::State]);
    let state = answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 6 * S);
    assert!(discord(&state).linking);
    assert_eq!(asks(minerva.tick(10 * S + 40)), [DiscordLinkAsk::State]);

    let state = answer(&mut minerva, DiscordLinkAnswer::Linked { username: "egberticus".to_string() }, 11 * S);
    assert_eq!(discord(&state), DiscordLinkView { linked_as: Some("egberticus".to_string()), offered: false, ..NOT_LINKED });
    assert_eq!(asks(minerva.tick(20 * S)), []);
}

#[test]
fn reading_stops_2_minutes_after_the_link_page_opened_and_link_discord_is_offered_again() {
    let mut minerva = egbert_at_launch(false);
    open_bests(&mut minerva, 10);
    answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20);
    minerva.link_discord();
    minerva.tick(30);
    answer(&mut minerva, DiscordLinkAnswer::Opened {}, 40);

    let mut at = 40;
    while at < 2 * MIN + 40 {
        at += S;
        if !minerva.tick(at).discord_link_requests.is_empty() {
            answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, at);
        }
    }

    let state = minerva.tick(2 * MIN + 10 * S).clone();
    assert_eq!(asks(&state), []);
    assert_eq!(discord(&state), NOT_LINKED);
}

#[test]
fn a_link_page_that_couldnt_be_opened_says_why_and_link_discord_tries_again() {
    let mut minerva = egbert_at_launch(false);
    open_bests(&mut minerva, 10);
    answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20);
    minerva.link_discord();
    minerva.tick(30);

    let state = answer(&mut minerva, DiscordLinkAnswer::Failed { why: "HTTP 429: rate-limited".to_string() }, 40);
    assert_eq!(discord(&state), DiscordLinkView { failed: Some("HTTP 429: rate-limited".to_string()), ..NOT_LINKED });
    assert_eq!(asks(&state), []);

    minerva.link_discord();
    let state = minerva.tick(50).clone();
    assert_eq!(asks(&state), [DiscordLinkAsk::Start]);
    assert_eq!(discord(&state).failed, None);
}

#[test]
fn the_prompt_asks_once_a_player_with_a_character_on_the_map_leaderboards_who_hasnt_linked() {
    let mut minerva = egbert_at_launch(true);
    open_bests(&mut minerva, 10);
    let state = answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20);
    assert_eq!(discord(&state), DiscordLinkView { prompt: true, ..NOT_LINKED });

    minerva.dismiss_link_prompt();
    let state = minerva.tick(30).clone();
    assert_eq!(discord(&state), NOT_LINKED);

    // Kept with the settings: not at the next launch either.
    let mut again = tracking();
    again.restore(minerva.settings());
    again.restore_bests(bests());
    again.on_wall_clock(WALL);
    again.tick(0);
    open_bests(&mut again, 10);
    let state = answer(&mut again, DiscordLinkAnswer::NotLinked {}, 20);
    assert!(!discord(&state).prompt);
}

#[test]
fn no_prompt_without_a_character_on_the_map_leaderboards_nor_while_linking() {
    let mut minerva = egbert_at_launch(false);
    open_bests(&mut minerva, 10);
    assert!(!discord(&answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20)).prompt);

    minerva.show_on_leaderboards("Egbert", true);
    assert!(discord(minerva.tick(30)).prompt);
    minerva.link_discord();
    assert!(!discord(minerva.tick(40)).prompt);
}

#[test]
fn linking_sends_a_character_whose_name_was_claimed_again_at_once() {
    let mut minerva = egbert_at_launch(true);
    let upload = minerva.panel_state().leaderboard_uploads[0].id;
    minerva.on_leaderboard_answer(upload, LeaderboardAnswer::Claimed {});
    let state = open_bests(&mut minerva, 10);
    assert_eq!(state.leaderboard_uploads, []);
    let egbert = |state: &PanelState| state.personal_bests.characters.iter().find_map(|c| c.leaderboard.clone()).unwrap();
    assert_eq!(egbert(&state).status, LeaderboardStatus::Claimed);
    answer(&mut minerva, DiscordLinkAnswer::NotLinked {}, 20);
    minerva.link_discord();
    minerva.tick(30);
    answer(&mut minerva, DiscordLinkAnswer::Opened {}, 40);
    minerva.tick(5 * S + 40);

    let state = answer(&mut minerva, DiscordLinkAnswer::Linked { username: "egberticus".to_string() }, 6 * S);

    assert_eq!(state.leaderboard_uploads.len(), 1, "Egbert's set goes up again");
    assert_eq!(egbert(&state).status, LeaderboardStatus::Sending);
}

#[test]
fn an_answer_to_a_request_no_longer_under_way_changes_nothing() {
    let mut minerva = egbert_at_launch(false);
    open_bests(&mut minerva, 10);
    let id = request(minerva.panel_state()).id;
    minerva.on_discord_link_answer(id, DiscordLinkAnswer::NotLinked {});
    let before = minerva.tick(20).clone();

    minerva.on_discord_link_answer(id, DiscordLinkAnswer::Linked { username: "someone".to_string() });
    minerva.on_discord_link_answer(id + 7, DiscordLinkAnswer::Linked { username: "someone".to_string() });

    assert_eq!(discord(minerva.tick(30)), discord(&before));
}

#[test]
fn the_sites_answers_read_as_a_link_state_and_a_token() {
    assert_eq!(read_link_state(200, r#"{"linked":false}"#), DiscordLinkAnswer::NotLinked {});
    assert_eq!(read_link_state(200, r#"{"linked":true,"username":"egberticus"}"#), DiscordLinkAnswer::Linked { username: "egberticus".to_string() });
    assert_eq!(read_link_state(401, r#"{"error":"no-key"}"#), DiscordLinkAnswer::Failed { why: "HTTP 401: no-key".to_string() });
    assert_eq!(read_link_state(200, "not json"), DiscordLinkAnswer::Failed { why: "HTTP 200: an answer that doesn't read".to_string() });

    let token = "A".repeat(43);
    assert_eq!(read_link_token(201, &format!(r#"{{"token":"{token}","url":"https://evil.example/","expiresAt":1}}"#)), Ok(token));
    assert_eq!(read_link_token(201, r#"{"token":"not a token!"}"#), Err("HTTP 201: an answer that doesn't read".to_string()));
    assert_eq!(read_link_token(429, r#"{"error":"rate-limited"}"#), Err("HTTP 429: rate-limited".to_string()));
}

#[test]
fn the_request_names_the_sites_path_and_method() {
    let start = DiscordLinkRequest { id: 1, ask: DiscordLinkAsk::Start };
    let state = DiscordLinkRequest { id: 2, ask: DiscordLinkAsk::State };
    assert_eq!((start.path(), start.posts()), ("api/link", true));
    assert_eq!((state.path(), state.posts()), ("api/link", false));
}
