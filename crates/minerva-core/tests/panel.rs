//! The panel stays out of the way: it collapses to an icon, and the player drags it by
//! its header to a spot inside the game window that it keeps as the game window moves.
//! It can also dock against the game window's left or right edge, or detach into a
//! free window anywhere.
//!
//! Worked example used throughout: the game's client area at (812,110), 1920x1200
//! physical px (measured on the dev machine), on one 3440x1440 monitor; an expanded
//! panel of 340x72 and a collapsed icon of 44x44. As measured on Windows 11, the game's
//! visible frame adds 1 px at the sides and bottom of the client area and 31 px on top.

mod common;

use common::{FakeSource, GAME, SCREEN, game, overlaps, rect};
use common::world::SEARCH;
use minerva_core::{
    Action, Minerva, DEFAULT_PANEL_WIDTH, Foreground, GameWindow, Layer, MIN_PANEL_WIDTH, PanelSizes,
    PlacementMode, ScreenRect, TitleLines, TitleReading,
    Settings, Size,
};

fn minerva() -> Minerva {
    // Layout never asks for the map index, so any source will do.
    let mut minerva = Minerva::new(FakeSource::osms_offline());
    minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 72 },
        collapsed: Size { width: 44, height: 44 },
    });
    minerva
}

#[test]
fn the_panel_starts_expanded_on_the_right_edge_of_the_game_a_third_of_the_way_down() {
    let mut minerva = minerva();
    let state = minerva.on_game_window(game(GAME));
    assert!(state.expanded);
    // 16 px in from the client's right edge (2732), its top 400 px (a third) down.
    assert_eq!(state.placement, Some(rect(2376, 510, 340, 72)));
}

#[test]
fn the_default_panel_grows_downward_so_its_top_stays_put() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    let state = minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 450 },
        collapsed: Size { width: 44, height: 44 },
    });
    assert_eq!(state.placement, Some(rect(2376, 510, 340, 450)));
}

#[test]
fn a_default_panel_too_tall_for_the_space_below_its_spot_moves_up_to_fit() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    let state = minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height: 900 },
        collapsed: Size { width: 44, height: 44 },
    });
    // Its bottom flush with the client's, at 1310.
    assert_eq!(state.placement, Some(rect(2376, 410, 340, 900)));
}

#[test]
fn the_collapse_hotkey_shrinks_the_panel_to_an_icon_and_expands_it_again() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    let state = minerva.act(Action::ToggleExpanded);
    assert!(!state.expanded);
    // The icon also sits on the right edge, at the same height.
    assert_eq!(state.placement, Some(rect(2672, 510, 44, 44)));

    let state = minerva.act(Action::ToggleExpanded);
    assert!(state.expanded);
    assert_eq!(state.placement, Some(rect(2376, 510, 340, 72)));
}

#[test]
fn a_panel_dragged_inside_at_any_time_keeps_its_spot_relative_to_the_game_window() {
    let mut minerva = minerva();
    let state = minerva.on_game_window(game(GAME));
    assert!(state.draggable, "no edit mode needed");

    // 188 px right of and 190 px below the client's top-left corner.
    let state = minerva.act(Action::PanelMoved { x: 1000, y: 300 });
    assert_eq!(state.placement, Some(rect(1000, 300, 340, 72)));

    let moved_game = ScreenRect { x: 100, y: 50, ..GAME };
    let state = minerva.on_game_window(game(moved_game));
    assert_eq!(state.placement, Some(rect(288, 240, 340, 72)));
}

#[test]
fn a_dragged_panel_collapses_to_an_icon_at_its_top_right_corner() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    minerva.act(Action::PanelMoved { x: 1000, y: 300 });

    // The expanded panel spans x 1000..1340, so the icon ends at 1340 too.
    let state = minerva.act(Action::ToggleExpanded);
    assert_eq!(state.placement, Some(rect(1296, 300, 44, 44)));
}

#[test]
fn the_panel_stays_inside_the_game_window_when_the_window_shrinks() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    // Near the bottom-right corner of the 1920x1200 client.
    minerva.act(Action::PanelMoved { x: 2376, y: 1200 });

    let smaller_game = ScreenRect { width: 1280, height: 720, ..GAME };
    let state = minerva.on_game_window(game(smaller_game));
    // Flush with the 1280x720 client's bottom-right corner at (2092, 830).
    assert_eq!(state.placement, Some(rect(1752, 758, 340, 72)));
}

/// What the shell does on restart: save the settings as JSON, load them into a new core.
fn restart(minerva: &Minerva) -> Minerva {
    let mut restarted = self::minerva();
    restarted.restore(common::settings_after_restart(minerva));
    restarted
}

#[test]
fn the_dragged_spot_and_collapsed_state_survive_a_restart() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    minerva.act(Action::PanelMoved { x: 1000, y: 300 });
    minerva.act(Action::ToggleExpanded);

    let mut restarted = restart(&minerva);
    let moved_game = ScreenRect { x: 100, y: 50, ..GAME };
    let state = restarted.on_game_window(game(moved_game));
    assert!(!state.expanded);
    // Icon top-right at client (528, 190), as dragged.
    assert_eq!(state.placement, Some(rect(584, 240, 44, 44)));
}

#[test]
fn an_empty_settings_file_gives_the_default_panel_and_hotkeys() {
    let mut minerva = minerva();
    minerva.restore(serde_json::from_str("{}").unwrap());

    let state = minerva.on_game_window(game(GAME));
    assert_eq!(state.hotkeys.toggle_panel, "ctrl+shift+c");
    assert_eq!(state.hotkeys.cycle_placement, "ctrl+shift+p");
    assert!(state.expanded);
    assert_eq!(state.placement_mode, PlacementMode::Inside);
    assert_eq!(state.placement, Some(rect(2376, 510, 340, 72)));
}

#[test]
fn without_a_game_window_the_panel_has_no_placement() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    assert_eq!(minerva.on_game_window(None).placement, None);
}

// Docked and detached placements. The placement hotkey cycles
// inside -> docked right -> docked left -> detached -> inside.

#[test]
fn docked_right_the_panel_sits_against_the_game_windows_right_edge_level_with_its_top() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    let state = minerva.act(Action::CyclePlacement);
    // Flush with the frame's right edge (client right 2732 + 1 px border), level with
    // the top of the client area.
    assert_eq!(state.placement, Some(rect(2733, 110, 340, 72)));
}

#[test]
fn docked_left_the_panel_sits_against_the_game_windows_left_edge_level_with_its_top() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    minerva.act(Action::CyclePlacement);

    let state = minerva.act(Action::CyclePlacement);
    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
    // Ends at the frame's left edge, 811 (client left 812 - 1 px border).
    assert_eq!(state.placement, Some(rect(471, 110, 340, 72)));
}

#[test]
fn a_docked_panel_without_room_on_screen_falls_back_inside_the_game_window() {
    let mut minerva = minerva();
    minerva.act(Action::CyclePlacement);
    // The frame now ends at 3421, 19 px short of the screen's right edge.
    let near_the_edge = ScreenRect { x: 1500, ..GAME };

    let state = minerva.on_game_window(game(near_the_edge));
    assert_eq!(state.placement_mode, PlacementMode::DockedRight);
    // The default inside spot: 16 px in from the client's right edge (3420), a third down.
    assert_eq!(state.placement, Some(rect(3064, 510, 340, 72)));
}

#[test]
fn a_docked_panel_can_sit_on_the_monitor_beside_the_game() {
    let mut minerva = minerva();
    minerva.act(Action::CyclePlacement);
    // The frame ends exactly at the main screen's right edge, 3440; a 1920x1080
    // monitor continues to the right of it.
    let at_the_edge = ScreenRect { x: 1519, ..GAME };
    let frame = rect(1518, 79, 1922, 1232);
    let second = rect(3440, 0, 1920, 1080);
    let window = GameWindow { client: at_the_edge, frame, screens: vec![SCREEN, second] };

    let state = minerva.on_game_window(Some(window));
    assert_eq!(state.placement, Some(rect(3440, 110, 340, 72)));
}

#[test]
fn a_docked_panel_follows_the_game_window_as_it_moves_and_resizes() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    minerva.act(Action::CyclePlacement);
    minerva.act(Action::CyclePlacement);

    let moved = rect(1200, 300, 1280, 720);
    let state = minerva.on_game_window(game(moved));
    // Docked left of the frame at 1199.
    assert_eq!(state.placement, Some(rect(859, 300, 340, 72)));
}

#[test]
fn a_docked_panel_collapses_to_an_icon_against_the_same_edge() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    minerva.act(Action::CyclePlacement);
    minerva.act(Action::CyclePlacement);

    let state = minerva.act(Action::ToggleExpanded);
    // The icon still ends at the frame's left edge, 811.
    assert_eq!(state.placement, Some(rect(767, 110, 44, 44)));
}

#[test]
fn the_placement_hotkey_cycles_through_every_placement_and_back_inside() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    let modes: Vec<_> =
        (0..4).map(|_| minerva.act(Action::CyclePlacement).placement_mode).collect();
    use PlacementMode::*;
    assert_eq!(modes, [DockedRight, DockedLeft, Detached, Inside]);
    assert_eq!(minerva.panel_state().placement, Some(rect(2376, 510, 340, 72)));
}

/// Inside -> docked right -> docked left -> detached.
fn detach(minerva: &mut Minerva) {
    for _ in 0..3 {
        minerva.act(Action::CyclePlacement);
    }
}

#[test]
fn a_detached_panel_stays_where_it_was_when_the_game_window_moves() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    detach(&mut minerva);
    let state = minerva.on_game_window(game(ScreenRect { x: 100, y: 50, ..GAME }));
    assert_eq!(state.placement_mode, PlacementMode::Detached);
    // Where it was last, docked left of the game, until the player moves it.
    assert_eq!(state.placement, Some(rect(471, 110, 340, 72)));
}

#[test]
fn a_detached_panel_moved_to_another_monitor_stays_there_across_a_restart() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    detach(&mut minerva);

    // Dragged onto a monitor left of the main one.
    let state = minerva.act(Action::PanelMoved { x: -1500, y: 200 });
    assert_eq!(state.placement, Some(rect(-1500, 200, 340, 72)));

    let mut restarted = restart(&minerva);
    let state = restarted.on_game_window(game(GAME));
    assert_eq!(state.placement_mode, PlacementMode::Detached);
    assert_eq!(state.placement, Some(rect(-1500, 200, 340, 72)));
}

/// Earlier versions saved the placement under `placement`.
#[test]
fn a_placement_saved_by_an_earlier_version_is_kept() {
    let mut minerva = minerva();
    minerva.restore(serde_json::from_str(r#"{"version":1,"placement":"dockedLeft"}"#).unwrap());

    let state = minerva.on_game_window(game(GAME));

    assert_eq!(state.placement_mode, PlacementMode::DockedLeft);
}

#[test]
fn the_chosen_placement_survives_a_restart() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    minerva.act(Action::CyclePlacement);

    let mut restarted = restart(&minerva);
    let state = restarted.on_game_window(game(GAME));
    assert_eq!(state.placement_mode, PlacementMode::DockedRight);
    assert_eq!(state.placement, Some(rect(2733, 110, 340, 72)));
}

// Z-order and the mouse. Wherever it is placed, the overlay belongs to the game: it shows
// on top of everything, but only while the game or one of Minerva's own windows is
// in front, and it disappears with the game or behind another app.

#[test]
fn with_another_app_in_front_of_the_game_the_overlay_hides() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    let state = minerva.on_foreground(Foreground::OtherApp);

    assert_eq!(state.layer, Layer::Hidden);
    assert!(!state.draggable);
}

#[test]
fn a_detached_panel_hides_with_another_app_in_front_too_wherever_it_is() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    detach(&mut minerva);
    // On a monitor of its own, say.
    minerva.act(Action::PanelMoved { x: 3000, y: 100 });

    let state = minerva.on_foreground(Foreground::OtherApp);

    assert_eq!(state.layer, Layer::Hidden);
    assert_eq!(state.placement, Some(rect(3000, 100, 340, 72)), "it keeps its spot");
}

#[test]
fn with_the_game_or_minerva_back_in_front_the_overlay_shows_again_where_it_was() {
    let mut minerva = minerva();
    let shown = minerva.on_game_window(game(GAME)).clone();

    for foreground in [Foreground::Game, Foreground::Minerva] {
        minerva.on_foreground(Foreground::OtherApp);
        let state = minerva.on_foreground(foreground);

        assert_eq!(state.layer, Layer::OnTop, "{foreground:?}");
        assert!(state.draggable, "{foreground:?}");
        assert_eq!(state.placement, shown.placement, "{foreground:?}");
    }
}

#[test]
fn without_the_game_the_overlay_hides_whatever_is_in_front() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    detach(&mut minerva);
    minerva.on_game_window(None);

    for foreground in [Foreground::Game, Foreground::Minerva, Foreground::OtherApp] {
        let state = minerva.on_foreground(foreground);

        assert_eq!(state.layer, Layer::Hidden, "detached, {foreground:?}");
        assert!(!state.draggable, "{foreground:?}");
    }
}

#[test]
fn until_the_shell_says_what_is_in_front_the_overlay_shows_with_the_game() {
    let mut minerva = minerva();

    let state = minerva.on_game_window(game(GAME));

    assert_eq!(state.layer, Layer::OnTop, "no hidden flash at launch");
}

#[test]
fn inside_or_docked_the_overlay_shows_on_top_with_the_game_and_disappears_without_it() {
    let mut minerva = minerva();
    let state = minerva.on_game_window(game(GAME));
    assert!(state.game_found);
    assert_eq!(state.layer, Layer::OnTop);

    minerva.act(Action::CyclePlacement);
    assert_eq!(minerva.panel_state().layer, Layer::OnTop);

    // Minimised or closed: the shell reports no game window.
    let state = minerva.on_game_window(None);
    assert!(!state.game_found);
    assert_eq!(state.layer, Layer::Hidden);
}

#[test]
fn a_detached_panel_also_shows_on_top_with_the_game_and_disappears_without_it() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    detach(&mut minerva);
    let state = minerva.panel_state().clone();
    assert_eq!(state.layer, Layer::OnTop);
    assert!(state.draggable);

    let state = minerva.on_game_window(None);
    assert!(!state.game_found);
    assert_eq!(state.layer, Layer::Hidden);
    assert!(!state.draggable);

    // It comes back where it was with the game.
    let state = minerva.on_game_window(game(GAME));
    assert_eq!(state.layer, Layer::OnTop);
    assert_eq!(state.placement, Some(rect(471, 110, 340, 72)));
}

#[test]
fn inside_and_detached_the_panel_can_be_dragged_at_any_time_docked_never() {
    let mut minerva = minerva();
    let state = minerva.on_game_window(game(GAME));
    assert!(state.draggable, "inside");
    minerva.on_hotkey(SEARCH);
    assert!(minerva.panel_state().draggable, "with the search view open too");

    use PlacementMode::*;
    let draggable: Vec<_> = (0..3)
        .map(|_| {
            let state = minerva.act(Action::CyclePlacement);
            (state.placement_mode, state.draggable)
        })
        .collect();
    assert_eq!(draggable, [(DockedRight, false), (DockedLeft, false), (Detached, true)]);
}

#[test]
fn a_docked_panel_cannot_be_dragged_off_its_edge() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    minerva.act(Action::CyclePlacement);

    let state = minerva.act(Action::PanelMoved { x: 1000, y: 300 });

    assert_eq!(state.placement, Some(rect(2733, 110, 340, 72)));
}

#[test]
fn a_hidden_overlay_cannot_be_dragged() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));

    let state = minerva.on_game_window(None);
    assert_eq!(state.layer, Layer::Hidden);
    assert!(!state.draggable);
}

// The overlay shows up in screenshots and streams, so it must never cover the Minimap Title
// where it was found: the core would read the panel instead of the title. Worked example: a
// small 640x200 client at (812,110) whose title was found at screen x 882..1452, y 150..198.

const SMALL_GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 640, height: 200 };

/// Where the Minimap Title was found on `SMALL_GAME`.
const TITLE: ScreenRect = ScreenRect { x: 882, y: 150, width: 570, height: 48 };

/// The Minimap Title read where it was found, at [`TITLE`], on the game window as it is.
fn found_title(minerva: &mut Minerva) {
    let lines = Some(TitleLines { street: "Made-up Street".into(), map: "Field".into() });
    minerva.on_reading(TitleReading { lines, found: Some(TITLE), at_ms: 0 });
}

/// Set up on `SMALL_GAME`, with the minimap title found at [`TITLE`].
fn set_up_on_small_game() -> Minerva {
    let mut minerva = minerva();
    minerva.restore(common::set_up());
    minerva.on_game_window(game(SMALL_GAME));
    found_title(&mut minerva);
    minerva
}

#[test]
fn the_default_spot_moves_below_the_minimap_title_when_it_would_cover_it() {
    let mut minerva = set_up_on_small_game();

    let state = minerva.on_game_window(game(SMALL_GAME));

    // Centred on the right edge it would span y 174..246; just below the title instead.
    assert_eq!(state.placement, Some(rect(1096, 198, 340, 72)));
}

// The panel is as tall as its content, but never taller than the space where it is
// placed: the game's client area inside, the monitor's work area docked or detached.
// The overlay then scrolls its long part.

/// The panel's content is `height` px tall (and 340 px wide).
fn content_height(minerva: &mut Minerva, height: u32) {
    minerva.set_panel_sizes(PanelSizes {
        expanded: Size { width: 340, height },
        collapsed: Size { width: 44, height: 44 },
    });
}

#[test]
fn inside_the_panel_is_never_taller_than_the_game_client() {
    let mut minerva = minerva();
    content_height(&mut minerva, 2_000);

    let state = minerva.on_game_window(game(GAME));

    assert_eq!(state.placement, Some(rect(2376, 110, 340, 1200)));
}

#[test]
fn inside_a_tall_panel_still_keeps_clear_of_the_minimap_title_in_the_space_below_it() {
    let mut minerva = set_up_on_small_game();
    content_height(&mut minerva, 500);

    let state = minerva.on_game_window(game(SMALL_GAME));

    // The title spans the client's width (y 150..198), so the panel has the 112 px
    // below it, down to the client's bottom at 310.
    let placement = state.placement.unwrap();
    assert_eq!(placement, rect(1096, 198, 340, 112));
    assert!(!overlaps(placement, TITLE));
}

#[test]
fn docked_the_panel_is_never_taller_than_the_work_area_below_its_top() {
    let mut minerva = minerva();
    content_height(&mut minerva, 2_000);
    minerva.act(Action::CyclePlacement);
    // A taskbar takes the bottom 40 px of the screen.
    let frame = rect(811, 79, 1922, 1232);
    let work_area = rect(0, 0, 3440, 1400);

    let state = minerva.on_game_window(Some(GameWindow { client: GAME, frame, screens: vec![work_area] }));

    // From the client's top at 110 down to 1400.
    assert_eq!(state.placement, Some(rect(2733, 110, 340, 1290)));
}

#[test]
fn detached_the_panel_is_never_taller_than_the_work_area_below_its_top_even_without_the_game() {
    let mut minerva = minerva();
    minerva.on_game_window(game(GAME));
    detach(&mut minerva);
    minerva.act(Action::PanelMoved { x: 100, y: 400 });
    content_height(&mut minerva, 2_000);

    assert_eq!(minerva.panel_state().placement, Some(rect(100, 400, 340, 1040)));
    let state = minerva.on_game_window(None);
    assert_eq!(state.placement, Some(rect(100, 400, 340, 1040)), "the monitors last known");
}

#[test]
fn a_panel_dragged_onto_the_minimap_title_is_moved_clear_of_it() {
    let mut minerva = set_up_on_small_game();
    minerva.on_game_window(game(SMALL_GAME));

    let state = minerva.act(Action::PanelMoved { x: 892, y: 140 });

    let placement = state.placement.unwrap();
    assert_eq!(placement, rect(892, 198, 340, 72));
    assert!(!overlaps(placement, TITLE));
}

/// While the title is looked for again (no capture found it where it was), the panel still
/// keeps clear of where it was last found: over it, the panel would hide it from the search.
#[test]
fn a_panel_dragged_onto_where_the_title_was_last_found_is_moved_clear_of_it_while_it_is_looked_for() {
    let mut minerva = set_up_on_small_game();
    minerva.on_reading(TitleReading { lines: None, found: None, at_ms: 333 });

    let state = minerva.act(Action::PanelMoved { x: 892, y: 140 });

    let placement = state.placement.unwrap();
    assert!(!overlaps(placement, TITLE), "{placement:?}");
}

#[test]
fn the_collapsed_icon_never_covers_the_minimap_title_either() {
    let mut minerva = set_up_on_small_game();
    minerva.on_game_window(game(SMALL_GAME));
    minerva.act(Action::PanelMoved { x: 892, y: 140 });

    let state = minerva.act(Action::ToggleExpanded);

    let placement = state.placement.unwrap();
    assert!(!overlaps(placement, TITLE), "{placement:?}");
}

// ...nor the EXP figures where they were found in the Status Bar. Worked example: a 640x400
// client at (812,110) with the title found as above (client y 40..88) and the
// EXP figures found at client (100,150) 312x26 (client x 100..412, y 150..176).

const TALLER_SMALL_GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 640, height: 400 };

/// The EXP figures read where they were found, at client (100,150) 312x26.
fn found_exp_figures(minerva: &mut Minerva) -> ScreenRect {
    let area = rect(912, 260, 312, 26);
    common::found_exp_figures(minerva, area);
    area
}

#[test]
fn a_panel_dragged_onto_the_minimap_title_is_moved_clear_of_the_exp_bar_too() {
    let mut minerva = minerva();
    minerva.restore(common::set_up());
    minerva.on_game_window(game(TALLER_SMALL_GAME));
    found_title(&mut minerva);
    let figures = found_exp_figures(&mut minerva);

    let state = minerva.act(Action::PanelMoved { x: 892, y: 140 }).clone();

    // Just below the title (client y 88..160) would cover the EXP bar; below that instead.
    let placement = state.placement.unwrap();
    assert_eq!(placement, rect(892, 110 + 176, 340, 72));
    assert!(!overlaps(placement, TITLE));
    assert!(!overlaps(placement, figures));
}

#[test]
fn a_panel_dragged_onto_the_exp_bar_is_moved_clear_of_it() {
    let mut minerva = minerva();
    minerva.restore(common::set_up());
    minerva.on_game_window(game(TALLER_SMALL_GAME));
    found_title(&mut minerva);
    let figures = found_exp_figures(&mut minerva);

    // Client (150,140): over the EXP bar (client y 150..176) and clear of the title.
    let state = minerva.act(Action::PanelMoved { x: 962, y: 250 }).clone();

    // Just below the EXP bar is the nearest clear spot.
    let placement = state.placement.unwrap();
    assert_eq!(placement, rect(962, 110 + 176, 340, 72));
    assert!(!overlaps(placement, TITLE));
    assert!(!overlaps(placement, figures));
}

#[test]
fn the_panel_starts_at_its_default_width() {
    let minerva = minerva();

    assert_eq!(minerva.panel_state().panel_width, DEFAULT_PANEL_WIDTH);
    assert_eq!(DEFAULT_PANEL_WIDTH, 380);
}

#[test]
fn a_panel_widened_by_its_bottom_corner_keeps_that_width_across_restarts() {
    let mut minerva = minerva();

    let state = minerva.act(Action::PanelResized { width: 460 });
    assert_eq!(state.panel_width, 460);
    assert_eq!(minerva.settings().panel_width, Some(460));

    let mut restarted = restored(minerva.settings());
    assert_eq!(restarted.panel_state().panel_width, 460);
    // Narrowed again later, the new width holds.
    assert_eq!(restarted.act(Action::PanelResized { width: 400 }).panel_width, 400);
}

#[test]
fn the_panel_is_never_narrower_than_its_minimum_width() {
    let mut minerva = minerva();

    let state = minerva.act(Action::PanelResized { width: 120 });
    assert_eq!(state.panel_width, MIN_PANEL_WIDTH);
    assert_eq!(MIN_PANEL_WIDTH, 320);

    // Nor when a settings file says otherwise.
    let narrow = Settings { panel_width: Some(90), ..Settings::default() };
    assert_eq!(restored(narrow).panel_state().panel_width, MIN_PANEL_WIDTH);
}

/// A core restored from these settings.
fn restored(settings: Settings) -> Minerva {
    let mut minerva = minerva();
    minerva.restore(settings);
    minerva
}
