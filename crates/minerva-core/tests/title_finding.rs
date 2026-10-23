//! The Minimap Title is found wherever the player put the minimap (ADR 0021): the whole client
//! area is looked through, about once a second, until it is found; from then on only where it
//! was found is captured, three times a second, until it isn't there any more.
//!
//! Worked example: the Kerning City window sample, whose client area (1916x1197 physical px)
//! sits at (812,110) on screen, with its minimap open at the client area's top-left.

mod common;

use common::FakeSource;
use minerva_core::{
    BugReport, Foreground, GameWindow, Minerva, PanelState, ReportAbout, ScreenPoint, ScreenRect, TITLE_LOST_AFTER_MS,
    TITLE_UNREAD_SEARCH_MS, TitleCapture,
    TitleReading,
};
use std::io::Read;

const KERNING: &str = "victoria-road__kerning-city--window";
const PERION: &str = "victoria-road__perion-department-store--window";

/// The client area of a window sample shown at (812,110).
fn game(sample: &str) -> ScreenRect {
    let client = common::client_of(&common::load(sample));
    ScreenRect { x: 812, y: 110, width: client.width(), height: client.height() }
}

fn window(client: ScreenRect) -> Option<GameWindow> {
    let frame = ScreenRect { x: client.x - 1, y: client.y - 31, width: client.width + 2, height: client.height + 32 };
    Some(GameWindow { client, frame, screens: vec![ScreenRect { x: 0, y: 0, width: 3440, height: 1440 }] })
}

/// Set up, with the game in front on `sample`'s client area.
fn started(sample: &str) -> Minerva {
    let mut minerva = Minerva::new(FakeSource::osms());
    minerva.restore(minerva_core::Settings { setup_done: true, ..Default::default() });
    minerva.on_game_window(window(game(sample)));
    minerva.on_foreground(Foreground::Game);
    minerva
}

/// The capture the core asks for, taken of `sample`'s client area.
fn capture(minerva: &Minerva, sample: &str, at_ms: u64) -> TitleCapture {
    let client = common::client_of(&common::load(sample));
    let region = minerva.panel_state().title_capture.expect("a title capture with the game in front");
    let ours = minerva.panel_state().our_windows.clone();
    TitleCapture { frame: common::capture(&client, game(sample), region, at_ms), origin: ScreenPoint { x: region.x, y: region.y }, ours }
}

/// Capture what the core asks for of `sample`, and read it.
fn read(minerva: &mut Minerva, sample: &str, at_ms: u64) -> PanelState {
    let capture = capture(minerva, sample, at_ms);
    minerva.on_title_capture(capture).clone()
}

fn shown(state: &PanelState) -> Option<(&str, &str)> {
    state.map.as_ref().map(|m| (m.street_name.as_str(), m.map_name.as_str()))
}

fn inside(inner: ScreenRect, outer: ScreenRect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width as i32 <= outer.x + outer.width as i32
        && inner.y + inner.height as i32 <= outer.y + outer.height as i32
}

#[test]
fn until_the_title_is_found_the_whole_client_area_is_looked_through() {
    let minerva = started(KERNING);
    let state = minerva.panel_state();

    assert_eq!(state.title_capture, Some(game(KERNING)));
    assert!(state.title_searching);
    assert!(!state.title_lost);
}

#[test]
fn once_found_only_where_it_was_found_is_captured_and_it_reads_there() {
    let mut minerva = started(KERNING);

    let state = read(&mut minerva, KERNING, 0);

    let found = state.title_capture.expect("captured where it was found");
    assert!(!state.title_searching);
    assert!(inside(found, game(KERNING)), "{found:?}");
    assert!(found.width < 800 && found.height < 150, "the minimap's top, not the client area: {found:?}");
    // The open minimap is at the client area's top-left.
    assert_eq!((found.x, found.y), (812, 110));

    let state = read(&mut minerva, KERNING, 333);
    assert_eq!(shown(&state), Some(("Victoria Road", "Kerning City")));
    assert!(!state.not_sure);
    assert_eq!(state.title_capture, Some(found), "still there");
}

#[test]
fn a_map_without_a_minimap_is_found_by_its_one_line_title_and_read_where_it_was_found() {
    let mut minerva = started(PERION);

    read(&mut minerva, PERION, 0);
    let state = read(&mut minerva, PERION, 333);

    assert_eq!(shown(&state), Some(("Victoria Road", "Perion Department Store")));
    assert!(!state.title_searching);
}

#[test]
fn when_the_title_is_no_longer_where_it_was_found_the_whole_client_area_is_looked_through_again() {
    let mut minerva = started(KERNING);
    read(&mut minerva, KERNING, 0);
    read(&mut minerva, KERNING, 333);
    let found = minerva.panel_state().title_capture.unwrap();

    // The player moved the minimap: nothing of it is there now.
    let blank = image::RgbaImage::from_pixel(found.width, found.height, image::Rgba([0, 0, 0, 255]));
    let origin = ScreenPoint { x: found.x, y: found.y };
    let state = minerva.on_title_capture(TitleCapture { frame: common::frame(&blank, 666), origin, ours: Vec::new() });

    assert_eq!(state.title_capture, Some(game(KERNING)));
    assert!(state.title_searching);
    assert_eq!(shown(state), Some(("Victoria Road", "Kerning City")), "the map shown stays");
    assert!(state.not_sure);
}

/// The pointer over the title, say: it is still there.
#[test]
fn a_title_found_but_unreadable_is_still_captured_where_it_was_found() {
    let mut minerva = started(KERNING);
    read(&mut minerva, KERNING, 0);
    let found = minerva.panel_state().title_capture.unwrap();

    let state = minerva.on_reading(TitleReading { lines: None, found: Some(found), at_ms: 333 });

    assert_eq!(state.title_capture, Some(found));
    assert!(!state.title_searching);
}

#[test]
fn the_game_window_moving_has_the_title_looked_for_again() {
    let mut minerva = started(KERNING);
    read(&mut minerva, KERNING, 0);

    let moved = ScreenRect { x: 900, ..game(KERNING) };
    let state = minerva.on_game_window(window(moved));

    assert_eq!(state.title_capture, Some(moved));
    assert!(state.title_searching);
}

/// The panel shows in a capture of the screen, and looks like an open minimap: its map's name on
/// light blue, beside an orange NOT SURE where the WORLD button would be. Found there, it would
/// move the panel clear of itself, and back once the next capture missed it.
#[test]
fn the_panel_in_the_capture_is_never_read_as_the_minimap_title() {
    // A Bug Report's capture with the panel in it, shown at (1504,526), on a client area at
    // (1512,93) with the panel dragged to (1513,486) and no minimap showing.
    let client = ScreenRect { x: 1512, y: 93, width: 1920, height: 1200 };
    let mut minerva = Minerva::new(FakeSource::osms());
    let offset = minerva_core::InsideOffset { x: 321, y: 393 };
    minerva.restore(minerva_core::Settings { setup_done: true, inside_offset: Some(offset), ..Default::default() });
    let size = |width, height| minerva_core::Size { width, height };
    minerva.set_panel_sizes(minerva_core::PanelSizes { expanded: size(320, 338), collapsed: size(44, 44) });
    minerva.on_game_window(window(client));
    minerva.on_foreground(Foreground::Game);
    let panel = minerva.panel_state().placement.expect("the panel inside the game");
    assert_eq!((panel.x, panel.y), (1513, 486));
    let crop = image::open(common::fixtures_dir().join("our-panel-over-the-game.png")).expect("fixture").to_rgba8();
    let mut screen = image::RgbaImage::from_pixel(client.width, client.height, image::Rgba([120, 170, 220, 255]));
    image::imageops::overlay(&mut screen, &crop, (1504 - client.x) as i64, (526 - client.y) as i64);

    let ours = minerva.panel_state().our_windows.clone();
    let origin = ScreenPoint { x: client.x, y: client.y };
    let state = minerva.on_title_capture(TitleCapture { frame: common::frame(&screen, 0), origin, ours });

    assert!(state.title_searching, "no title found: {:?}", state.title_capture);
    assert_eq!(state.title_capture, Some(client));
    assert_eq!(state.placement, Some(panel), "the panel stays put");
}

#[test]
fn nothing_is_captured_while_another_app_is_in_front() {
    let mut minerva = started(KERNING);

    let state = minerva.on_foreground(Foreground::OtherApp);

    assert_eq!(state.title_capture, None);
    assert!(!state.title_searching);
}

fn not_found(at_ms: u64) -> TitleReading {
    TitleReading { lines: None, found: None, at_ms }
}

#[test]
fn with_no_title_found_for_a_while_the_panel_says_it_cant_find_it_until_one_is() {
    let mut minerva = started(KERNING);
    read(&mut minerva, KERNING, 0);
    read(&mut minerva, KERNING, 333);

    assert!(!minerva.on_reading(not_found(1_000)).title_lost);
    assert!(!minerva.on_reading(not_found(1_000 + TITLE_LOST_AFTER_MS - 1)).title_lost);
    let state = minerva.on_reading(not_found(1_000 + TITLE_LOST_AFTER_MS)).clone();
    assert!(state.title_lost);
    assert_eq!(shown(&state), Some(("Victoria Road", "Kerning City")), "the map shown stays");

    let state = read(&mut minerva, KERNING, 12_000);
    assert!(!state.title_lost);
}

#[test]
fn a_title_found_but_unreadable_is_not_lost() {
    let mut minerva = started(KERNING);

    minerva.on_reading(TitleReading { lines: None, found: Some(common::TITLE_FOUND), at_ms: 0 });
    let state = minerva.on_reading(TitleReading { lines: None, found: Some(common::TITLE_FOUND), at_ms: 20_000 });

    assert!(!state.title_lost);
}

#[test]
fn with_no_title_ever_found_the_panel_says_so_after_a_while_too() {
    let mut minerva = started(KERNING);

    minerva.on_reading(not_found(0));
    let state = minerva.on_reading(not_found(TITLE_LOST_AFTER_MS));

    assert!(state.title_lost);
}

/// The files in a Bug Report's .zip, by name.
fn crops(report: &BugReport) -> Vec<(String, image::RgbaImage)> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(report.zip.clone())).expect("a zip");
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let name = file.name().to_string();
        if name.starts_with("crops/minimap-title-") {
            let mut png = Vec::new();
            file.read_to_end(&mut png).unwrap();
            out.push((name, image::load_from_memory(&png).unwrap().to_rgba8()));
        }
    }
    out
}

/// A Bug Report keeps the title where it was found, as a sample of it in `samples/minimap`
/// keeps it: none of the rest of the client area, whose other players' names and chat aren't
/// the title's.
#[test]
fn a_bug_report_keeps_the_title_where_it_was_found_and_none_of_the_rest_of_the_client_area() {
    let mut minerva = started(KERNING);
    read(&mut minerva, KERNING, 0);
    let found = minerva.panel_state().title_capture.unwrap();
    read(&mut minerva, KERNING, 333);

    let report = minerva.bug_report(&ReportAbout::default());

    let crops = crops(&report);
    assert!(!crops.is_empty(), "a crop of the title");
    for (name, crop) in &crops {
        assert!(crop.width() <= found.width && crop.height() <= found.height, "{name}: {:?}", crop.dimensions());
    }
}

#[test]
fn the_panels_line_saying_it_cant_find_the_title_opens_the_faqs_answer_on_it() {
    let mut minerva = started(KERNING);
    minerva.on_reading(not_found(0));
    assert!(minerva.on_reading(not_found(TITLE_LOST_AFTER_MS)).title_lost);

    let request = minerva.press(minerva_core::PanelControl::OpenTitleHelp);

    let url = "https://owlofminerva.app/faq/#minimap-not-found".to_string();
    assert_eq!(request, Some(minerva_core::Request::OpenInBrowser { url }));
}

/// Found where it was, but read as no map for a while (a moved minimap whose title the box
/// cuts, say): it is looked for afresh in the whole client area.
#[test]
fn a_title_found_but_read_as_no_map_for_a_few_seconds_is_looked_for_afresh() {
    let mut minerva = started(KERNING);
    read(&mut minerva, KERNING, 0);
    read(&mut minerva, KERNING, 333);
    let found = minerva.panel_state().title_capture.unwrap();
    let misread = |at_ms| TitleReading {
        lines: Some(minerva_core::TitleLines { street: "Vxqz Rozd".into(), map: "Qwrtp Czty".into() }),
        found: Some(found),
        at_ms,
    };

    assert!(!minerva.on_reading(misread(1_000)).title_searching);
    assert!(!minerva.on_reading(misread(1_000 + TITLE_UNREAD_SEARCH_MS - 1)).title_searching);
    let state = minerva.on_reading(misread(1_000 + TITLE_UNREAD_SEARCH_MS));

    assert!(state.title_searching);
    assert_eq!(state.title_capture, Some(game(KERNING)));
}

#[test]
fn a_title_read_as_its_map_again_in_time_stays_where_it_was_found() {
    let mut minerva = started(KERNING);
    read(&mut minerva, KERNING, 0);
    let found = minerva.panel_state().title_capture.unwrap();

    minerva.on_reading(TitleReading { lines: None, found: Some(found), at_ms: 333 });
    read(&mut minerva, KERNING, 666);
    let state = minerva.on_reading(TitleReading { lines: None, found: Some(found), at_ms: 333 + TITLE_UNREAD_SEARCH_MS });

    assert!(!state.title_searching, "the read at 666 ms started the wait afresh");
}

/// Time behind another app isn't time the title wasn't found: none was looked for.
#[test]
fn the_cant_find_line_counts_afresh_after_another_app_was_in_front() {
    let mut minerva = started(KERNING);
    minerva.on_reading(not_found(0));

    minerva.on_foreground(Foreground::OtherApp);
    minerva.on_foreground(Foreground::Game);

    assert!(!minerva.on_reading(not_found(TITLE_LOST_AFTER_MS)).title_lost);
    assert!(minerva.on_reading(not_found(2 * TITLE_LOST_AFTER_MS)).title_lost);
}
