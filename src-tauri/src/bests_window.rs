//! The Personal Bests window: every map's Personal Best, over the game where the core puts
//! it, above the panel and the Grind Tracker's window. Clickable while it shows, and dragged
//! by its bar. One of our windows over the game ([`crate::panel_window`]); this says what its
//! page renders, and relays what the player does in it.

use crate::Command;
use crate::grind_tracker::GRIND_TRACKER;
use crate::panel_window::{Heights, PanelWindow, Shows, WindowSpec};
use minerva_core::{Action, DiscordLinkView, MapMark, Minerva, PanelState, PersonalBestsCharacter, VisitBest};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use tauri::State;

pub const PERSONAL_BESTS: &str = "personal-bests";

/// The width its page lays out at, in CSS px (logical); the core narrows the window to a
/// narrower game, and `src/personal-bests.css` lays out to whatever it gets. Its height
/// follows its content, as the page reports it: past the most, the table scrolls. Over the
/// panel and the Grind Tracker where it covers them: the player opened it to read it.
pub const SPEC: WindowSpec = WindowSpec {
    label: PERSONAL_BESTS,
    event: "personal-bests-state",
    watcher: "on-top-personal-bests",
    width: 920.0,
    height: Heights { default: 300.0, min: 80.0, max: 600.0 },
    sized: Minerva::set_personal_bests_size,
    moved: Some(|x, y| Action::PersonalBestsMoved { x, y }),
    beneath: &["overlay", GRIND_TRACKER],
};

/// What the Personal Bests page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BestsView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    /// Whose bests it can show, one at a time, as its picker offers them.
    characters: &'a [PersonalBestsCharacter],
    /// Every Map Mark, which each map names its own by.
    map_marks: &'a BTreeMap<String, MapMark>,
    /// Linking this install to the player's Account on the site with Discord.
    discord: &'a DiscordLinkView,
}

/// Show the window where the core says, or nothing; and send its page the rows.
pub fn sync(window: &mut PanelWindow, state: &PanelState) {
    let bests = &state.personal_bests;
    let placement = bests.placement;
    let view = BestsView { shown: placement.is_some(), characters: &bests.characters, map_marks: &state.map_marks, discord: &bests.discord };
    window.sync(&view, Shows { placement, clickable: placement.is_some(), takes_keyboard: false });
}

/// A click on a map's name in the window (its ID): open its MeowDB page.
#[tauri::command]
pub fn personal_bests_open_map(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenBestsMapPage(id));
}

/// A second click on a Visit's × in the window: remove it from the Visit History of the map
/// (its ID) of the Character named `character` (`None`: the Unassigned one), by its best as
/// the window lists it.
#[tauri::command]
pub fn personal_bests_remove_visit(
    character: Option<String>,
    map: String,
    visit: VisitBest,
    commands: State<'_, Sender<Command>>,
) {
    let _ = commands.send(Command::RemoveVisit { character, map, visit });
}

/// The player confirmed moving a Visit on the map (its ID) of the Character named `from`
/// (`None`: an Unassigned one), by its best as the window lists it, to the Character named
/// `to`.
#[tauri::command]
pub fn personal_bests_move_visit(
    from: Option<String>,
    map: String,
    visit: VisitBest,
    to: String,
    commands: State<'_, Sender<Command>>,
) {
    let _ = commands.send(Command::MoveVisit { from, map, visit, to });
}

/// The player confirmed moving a Visit on the map (its ID) of the Character named `character`
/// (`None`: an Unassigned one), by its best as the window lists it, to the Party Bests (`party`)
/// or the Solo Bests.
#[tauri::command]
pub fn personal_bests_set_visit_party(
    character: Option<String>,
    map: String,
    visit: VisitBest,
    party: bool,
    commands: State<'_, Sender<Command>>,
) {
    let _ = commands.send(Command::SetVisitParty { character, map, visit, party });
}

/// The player confirmed giving the Character named `to` every Unassigned Visit.
#[tauri::command]
pub fn personal_bests_assign_unassigned(to: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::AssignUnassigned(to));
}

/// The player sent the Character named `name` to the Map Leaderboards again (having confirmed,
/// for uploads held).
#[tauri::command]
pub fn personal_bests_send_again(name: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::SendToLeaderboardsAgain(name));
}

/// The player confirmed removing the Character named `name`, with their bests.
#[tauri::command]
pub fn personal_bests_remove_character(name: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::RemoveCharacter(name));
}

/// The player confirmed merging the Character named `from` into the one named `into`.
#[tauri::command]
pub fn personal_bests_merge_characters(from: String, into: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::MergeCharacters { from, into });
}

/// The player turned "Share" on (`on`) or off for the Character named `name`.
#[tauri::command]
pub fn personal_bests_show_on_leaderboards(name: String, on: bool, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::ShowOnLeaderboards { name, on });
}

/// The player pressed Link Discord: the core starts a link, and the link page opens.
#[tauri::command]
pub fn personal_bests_link_discord(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::LinkDiscord);
}

/// The player said not now to the prompt to link Discord.
#[tauri::command]
pub fn personal_bests_dismiss_link_prompt(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::DismissLinkPrompt);
}

/// A click on a map's Map Leaderboard link (its ID), among the bests of the Character named
/// `character` (`None`: the Unassigned ones): open it.
#[tauri::command]
pub fn personal_bests_open_leaderboard(character: Option<String>, id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenBestsLeaderboard { character, id });
}
