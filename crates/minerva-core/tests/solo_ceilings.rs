//! The Map Leaderboards' cap (ADR 0015) is 5× each map's solo EXP/h ceiling, from a table
//! `probe ceilings` writes from OSMS's set with the core's own Map Summary arithmetic, so
//! the site never works it out again. Beside each ceiling the table holds the levels of the
//! middle half of the map's spawns, which Where to train matches maps to a Level Band by
//! (ADR 0024), and its Map Mark, which Where to train picks an area by. Read from made-up
//! worlds in OSMS's shape.
//!
//! Unlike the other tests, these assert on the table rather than on `PanelState`: the
//! table is what the site's Worker loads, and the panel never shows it.

mod common;

use common::osms::{self, map, mob};
use minerva_core::LevelRange;
use minerva_core::osms::{SiteMark, SoloCeiling, read};
use std::collections::BTreeMap;

/// The ranged-timer world of `map_summary.rs` (269 Ghost Stumps and Pigs an hour, 2,810
/// EXP alone), beside a map whose only mob gives no EXP and a map with no mobs.
fn world() -> osms::World {
    osms::world(
        vec![
            map(100_000_000, "Made-up Street", "Ghost Field")
                .timed(20, 2, "180.00-1800.00 [720.00]")
                .timed(21, 3, "20.00-60.00 [41.76]"),
            map(100_000_001, "Made-up Street", "Training Field").spawns(22, 4),
            map(100_000_002, "Made-up Street", "Empty Field"),
        ],
        vec![mob(20, "Ghost Stump", 19, 300, 100), mob(21, "Pig", 7, 80, 7), mob(22, "Dummy", 1, 10, 0)],
    )
}

#[test]
fn each_map_with_a_solo_ceiling_is_keyed_by_its_nine_digit_id() {
    let set = read(&world().files()).expect("reads");

    let ghost_field = SoloCeiling { exp_per_hour: 2810, mob_levels: LevelRange { lowest: 7, highest: 19 }, mark: None };
    assert_eq!(set.solo_ceilings(), BTreeMap::from([("100000000".to_string(), ghost_field)]));
}

/// Henesys Hunting Ground I's solo ceiling is the spec's 153,216 (`map_summary.rs`), the
/// middle half of its 39 spawns levels 2 to 8 (its 5 Green Mushrooms, level 14, are above
/// it), and Kerning City, a Town, has none.
#[test]
fn the_ceiling_is_the_map_summarys_solo_exp_per_hour_and_a_town_has_none() {
    let set = read(&osms::game().files()).expect("reads");

    let ceilings = set.solo_ceilings();

    let hunting_ground = SoloCeiling { exp_per_hour: 153_216, mob_levels: LevelRange { lowest: 2, highest: 8 }, mark: None };
    assert_eq!(ceilings.get("010001010"), Some(&hunting_ground));
    assert_eq!(ceilings.get("010003000"), None);
}

/// A mob a few of a map's spawns are, far above the rest, leaves its levels as they are: a
/// map of level-10 Pigs with one level-50 Golem is a level-10 map, not one for levels 10 to
/// 50. So is a stray one far below.
#[test]
fn the_levels_are_the_middle_half_of_the_spawns_so_a_stray_mob_moves_nothing() {
    let world = osms::world(
        vec![
            map(100_000_000, "Made-up Street", "Pig Field").spawns(20, 9).spawns(21, 1),
            map(100_000_001, "Made-up Street", "Golem Field").spawns(22, 1).spawns(21, 3).spawns(23, 3),
        ],
        vec![mob(20, "Pig", 10, 80, 7), mob(21, "Golem", 50, 9_000, 600), mob(22, "Snail", 1, 10, 1), mob(23, "Boar", 55, 9_500, 650)],
    );
    let set = read(&world.files()).expect("reads");

    let levels: BTreeMap<String, LevelRange> = set.solo_ceilings().into_iter().map(|(id, c)| (id, c.mob_levels)).collect();

    assert_eq!(
        levels,
        BTreeMap::from([
            ("100000000".to_string(), LevelRange { lowest: 10, highest: 10 }),
            ("100000001".to_string(), LevelRange { lowest: 50, highest: 55 }),
        ])
    );
}

/// A map's row names its Map Mark, and the site is given each mark's area as players know it
/// and where OSMS keeps its picture. A mark OSMS has no picture of is no mark.
#[test]
fn each_row_names_its_map_mark_and_the_site_is_given_each_marks_area_and_picture() {
    let world = osms::world(
        vec![
            map(100_000_000, "Made-up Street", "Harbor Field").spawns(20, 4).marked("Rith"),
            map(100_000_001, "Made-up Street", "Plain Field").spawns(20, 4),
            map(100_000_002, "Made-up Street", "Lost Field").spawns(20, 4).marked("Nowhere"),
        ],
        vec![mob(20, "Pig", 10, 80, 7)],
    )
    .with_mark_icon("Rith");
    let set = read(&world.files()).expect("reads");

    let marks: BTreeMap<String, Option<String>> = set.solo_ceilings().into_iter().map(|(id, c)| (id, c.mark)).collect();

    assert_eq!(
        marks,
        BTreeMap::from([
            ("100000000".to_string(), Some("Rith".to_string())),
            ("100000001".to_string(), None),
            ("100000002".to_string(), None),
        ])
    );
    assert_eq!(
        set.site_marks(),
        BTreeMap::from([(
            "Rith".to_string(),
            SiteMark {
                name: "Lith Harbor".to_string(),
                picture_url: "https://osmsdataexplorer.com/data/current/images/map_marks/Rith.png".to_string(),
            },
        )])
    );
}
