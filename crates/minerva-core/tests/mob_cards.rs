//! Every mob on the map, all of which the panel lists, has a Mob Card, shown when the
//! player hovers its name: OSMS's animation of the mob, and the figures from OSMS's set
//! that the row doesn't show. The figures come with the set, so a card is ready as soon
//! as its map is shown. Clicking a mob's name opens its MeowDB page.

mod common;

use common::world::tick_through;

use common::osms;
use common::{FakeSource, arrive, fresh_files, requests_for, SUNSET_SAMPLE, KERNING_SAMPLE, CONSTRUCTION_SAMPLE};
use minerva_core::osms::OsmsFiles;
use minerva_core::{
    Minerva, DamageRange, Elements, HpPerExp, MapDataStatus, MobCard, MobDetails, OSMS_AND_MEOWDB_CREDIT, PanelState,
    Recognition, Request, SourceError,
};

/// The card of the listed mob with this ID.
fn card<'a>(state: &'a PanelState, id: &str) -> &'a MobCard {
    state.mob_cards.iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no card for mob {id}"))
}

/// The figures on the card of the listed mob with this ID.
fn details<'a>(state: &'a PanelState, id: &str) -> &'a MobDetails {
    &card(state, id).details
}

fn listed_ids(state: &PanelState) -> Vec<String> {
    let data = state.data.data().expect("the map's data");
    data.mobs.iter().map(|m| m.id.clone()).collect()
}

/// The made-up game world, but Sunset Sky lists these mobs and nothing else, each on 4
/// spawn points.
fn sunset_listing(mobs: Vec<osms::Mob>) -> FakeSource {
    let mut world = osms::game().changing(osms::SUNSET_SKY, |m| osms::Map { spawns: Vec::new(), ..m });
    for mob in mobs {
        let id = mob.id;
        world = world.with_mob(mob).changing(osms::SUNSET_SKY, |m| m.spawns(id, 4));
    }
    FakeSource::with_world(world)
}

#[test]
fn a_listed_mobs_card_shows_its_figures_from_the_set_as_soon_as_the_map_is_shown() {
    let blue_snail = osms::mob(3, "Blue Snail", 2, 51, 4).p_atk(5).p_def(0).mp(30).acc(39).speed(-50);
    let mushmom = osms::mob(700_000, "Mushmom", 40, 34_440, 340)
        .p_atk(532)
        .p_def(15)
        .m_def(25)
        .mp(200)
        .acc(130)
        .avoid(29)
        .speed(5);
    let mut minerva = Minerva::new(sunset_listing(vec![blue_snail, mushmom]));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

    // In the order the panel lists them.
    let ids: Vec<&str> = state.mob_cards.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["3", "700000"]);
    let blue_snail = details(&state, "3");
    assert_eq!(blue_snail.touch_damage, Some(DamageRange { min: 5, max: 7 }));
    assert_eq!((blue_snail.mp, blue_snail.accuracy), (Some(30), Some(39)));
    // P.DEF is 0, and there's no AVOID or M.DEF at all.
    assert_eq!((blue_snail.avoid, blue_snail.physical_defense, blue_snail.magic_defense), (None, None, None));
    assert_eq!(blue_snail.speed, "Slow");
    let mushmom = details(&state, "700000");
    assert_eq!(mushmom.touch_damage, Some(DamageRange { min: 585, max: 797 }));
    assert_eq!((mushmom.mp, mushmom.accuracy, mushmom.avoid), (Some(200), Some(130), Some(29)));
    assert_eq!((mushmom.physical_defense, mushmom.magic_defense), (Some(15), Some(25)));
    assert_eq!(mushmom.speed, "Fast");
}

#[test]
fn touch_dmg_runs_from_1_1_times_p_atk_rounded_down_to_1_5_times_rounded_up_less_1() {
    let p_atks = [7, 11, 34, 54];
    let mobs = p_atks.iter().map(|&p| osms::mob(9000 + p, &format!("P.ATK {p}"), p as u32, 100, 10).p_atk(p));
    let mut minerva = Minerva::new(sunset_listing(mobs.collect()));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

    let touch = |p: u64| details(&state, &(9000 + p).to_string()).touch_damage;
    assert_eq!(touch(7), Some(DamageRange { min: 7, max: 10 }));
    assert_eq!(touch(11), Some(DamageRange { min: 12, max: 16 }));
    assert_eq!(touch(34), Some(DamageRange { min: 37, max: 50 }));
    assert_eq!(touch(54), Some(DamageRange { min: 59, max: 80 }));
}

#[test]
fn a_figure_a_mob_lacks_or_has_as_0_shows_as_none_and_speed_without_one_is_normal() {
    let mut minerva = Minerva::new(sunset_listing(vec![
        osms::mob(9001, "Nothing", 10, 100, 10),
        osms::mob(9002, "Zeros", 11, 100, 10).p_atk(0).p_def(0).m_def(0).mp(0).acc(0).avoid(0).speed(0),
    ]));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

    for id in ["9001", "9002"] {
        let mob = details(&state, id);
        assert_eq!(mob.touch_damage, None, "{id}");
        assert_eq!((mob.mp, mob.accuracy, mob.avoid), (None, None, None), "{id}");
        assert_eq!((mob.physical_defense, mob.magic_defense), (None, None), "{id}");
        assert_eq!(mob.speed, "Normal", "{id}");
    }
}

#[test]
fn hp_per_exp_is_ranked_among_all_the_sets_mobs() {
    let mut minerva = Minerva::new(sunset_listing(vec![
        // 0.125, rounded half up.
        osms::mob(9001, "Lightest", 10, 1, 8),
        osms::mob(9002, "Tied", 11, 20, 10),
        osms::mob(9003, "Also Tied", 12, 40, 20),
        // 101.2941…
        osms::mob(700_000, "Mushmom", 40, 34_440, 340),
        // No EXP, so no HP per EXP: below every other mob.
        osms::mob(9004, "No EXP", 13, 500, 0),
    ]));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

    // The set's mobs: the made-up game's, all between 8 and 14 HP per EXP, and these 5.
    let ranked = osms::game_mobs().len() as u32 + 5;
    let hp_per_exp = |id: &str| details(&state, id).hp_per_exp;
    assert_eq!(hp_per_exp("9001"), Some(HpPerExp { hundredths: 13, rank: 1, ranked }));
    assert_eq!(hp_per_exp("9002"), Some(HpPerExp { hundredths: 200, rank: 2, ranked }));
    assert_eq!(hp_per_exp("9003"), Some(HpPerExp { hundredths: 200, rank: 2, ranked }));
    assert_eq!(hp_per_exp("700000"), Some(HpPerExp { hundredths: 10129, rank: ranked - 1, ranked }));
    assert_eq!(hp_per_exp("9004"), None);
}

#[test]
fn a_cards_elements_read_as_weak_strong_and_immune() {
    let mut minerva = Minerva::new(sunset_listing(vec![
        osms::mob(9001, "Ice Sentinel", 30, 100, 10).element("Fire", "Weak").element("Ice", "Strong"),
        osms::mob(9002, "Green Mushroom", 15, 100, 10).element("Poison", "Immune"),
        osms::mob(9003, "Plain Mushroom", 10, 100, 10),
    ]));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);

    let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
    let elements = |id: &str| details(&state, id).elements.clone();
    assert_eq!(elements("9001"), Elements { weak: names(&["Fire"]), strong: names(&["Ice"]), immune: vec![] });
    assert_eq!(elements("9002"), Elements { weak: vec![], strong: vec![], immune: names(&["Poison"]) });
    assert_eq!(elements("9003"), Elements::default());
}

#[test]
fn a_mob_record_not_as_expected_reads_as_a_changed_format_never_as_a_partial_card() {
    let odd_affinity = osms::mob(9001, "Odd", 10, 100, 10).element("Fire", "Absorbs");
    let odd_animation = osms::mob(9003, "Odd", 10, 100, 10).animation("../../hash");
    let files = osms::game().with_mob(osms::mob(9002, "Odd", 10, 100, 10).acc(39)).files();
    let acc_as_text = OsmsFiles { monsters: files.monsters.replace(r#""acc":39"#, r#""acc":"39""#), ..files };
    assert_ne!(acc_as_text.monsters, osms::game().files().monsters);

    let sources = [
        sunset_listing(vec![odd_affinity]),
        sunset_listing(vec![odd_animation]),
        FakeSource::answering_osms(vec![fresh_files(acc_as_text)]),
    ];
    for source in sources {
        let mut minerva = Minerva::new(source);

        let state = minerva.tick(0).clone();

        assert_eq!(state.recognition, Recognition::FormatChanged);
    }
}

/// Where OSMS keeps the animation with this hash, below its data folder.
fn animation(hash: &str) -> String {
    format!("images/monsters/{hash}.webp")
}

#[test]
fn a_cards_image_is_osmss_default_animation_of_its_mob_asked_for_by_its_path() {
    let blue_snail = osms::mob(3, "Blue Snail", 2, 51, 4).animation("43d30603411fc517");
    let mut minerva = Minerva::new(sunset_listing(vec![blue_snail]));
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    let state = minerva.tick(400).clone();

    // The fake source's answer: where the overlay loads it from.
    assert_eq!(card(&state, "3").image, Some(format!("picture:{}", animation("43d30603411fc517"))));
}

#[test]
fn saved_animations_are_on_the_cards_by_the_next_tick() {
    let mut minerva = Minerva::new(FakeSource::osms());
    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);
    assert_eq!(state.mob_cards.len(), 5);

    let state = minerva.tick(400).clone();

    for id in listed_ids(&state) {
        assert_eq!(card(&state, &id).image, Some(format!("picture:{}", animation(&format!("gif{id}")))));
    }
}

#[test]
fn while_its_animation_is_on_its_way_a_card_shows_its_figures_without_it() {
    let mut minerva = Minerva::new(FakeSource::osms().pictures_taking(20));

    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);
    assert!(state.mob_cards.iter().all(|c| c.image.is_none()), "{:?}", state.mob_cards);
    assert_eq!(details(&state, "3").speed, "Normal");

    let state = tick_through(&mut minerva, 400, 20_000);
    assert!(state.mob_cards.iter().all(|c| c.image.is_some()), "{:?}", state.mob_cards);
}

#[test]
fn a_mob_without_an_animation_still_has_its_card() {
    let source = sunset_listing(vec![
        osms::mob(3, "Blue Snail", 2, 51, 4),
        osms::mob(6, "Stump", 5, 133, 10).without_animation(),
        osms::mob(9, "Orange Mushroom", 8, 142, 15),
    ])
    .with_pictures(|path| match path.contains("gif3") {
        true => Err(SourceError::FormatChanged("not found".into())),
        false => Ok(format!("picture:{path}")),
    });
    let requests = source.picture_requests.clone();
    let mut minerva = Minerva::new(source);
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    let state = tick_through(&mut minerva, 400, 20_000);

    // OSMS couldn't give Blue Snail's; it names none for the Stump.
    assert_eq!((card(&state, "3").image.as_ref(), card(&state, "6").image.as_ref()), (None, None));
    assert_eq!((details(&state, "3").speed.as_str(), details(&state, "6").speed.as_str()), ("Normal", "Normal"));
    assert_eq!(card(&state, "9").image, Some(format!("picture:{}", animation("gif9"))));
    assert_eq!(animations_asked(&requests), [animation("gif3"), animation("gif9")]);
}

/// The animations asked for, in order, leaving out the Map Preview's picture, which goes
/// first.
fn animations_asked(requests: &std::sync::Mutex<Vec<String>>) -> Vec<String> {
    requests.lock().unwrap().iter().filter(|path| path.starts_with("images/monsters/")).cloned().collect()
}

#[test]
fn an_animation_that_couldnt_be_fetched_is_asked_for_again_a_few_minutes_later() {
    let mut first = true;
    let source = FakeSource::osms().with_pictures(move |path| match (path.contains("gif3"), first) {
        (true, true) => {
            first = false;
            Err(common::unreachable())
        }
        _ => Ok(format!("picture:{path}")),
    });
    let mut minerva = Minerva::new(source);
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    let mut at = 400;
    let mut image_at = |minerva: &mut Minerva, until: u64| {
        while at < until {
            minerva.tick(at);
            at += 1_000;
        }
        card(minerva.panel_state(), "3").image.clone()
    };

    assert_eq!(image_at(&mut minerva, 60_000), None);
    assert_eq!(image_at(&mut minerva, 6 * 60_000), Some(format!("picture:{}", animation("gif3"))));
}

#[test]
fn animations_are_asked_for_one_at_a_time_with_no_other_pace() {
    // Each takes a second to come (20 polls at 50 ms).
    let source = FakeSource::osms().pictures_taking(20);
    let requests = source.picture_requests.clone();
    let mut minerva = Minerva::new(source);
    let state = arrive(&mut minerva, SUNSET_SAMPLE, 0);
    let listed = listed_ids(&state).len();

    let mut starts = Vec::new();
    for at in (50..=20_000).step_by(50) {
        let before = animations_asked(&requests).len();
        minerva.tick(at);
        starts.extend(std::iter::repeat_n(at, animations_asked(&requests).len() - before));
    }

    assert_eq!(starts.len(), listed, "{starts:?}");
    // Each asked for just as the one before arrives.
    assert!(starts.windows(2).all(|w| w[1] - w[0] == 1_000), "{starts:?}");
}

#[test]
fn an_animation_on_its_way_when_the_player_leaves_its_map_is_seen_through_before_the_next() {
    // Each takes a second to come (20 polls at 50 ms).
    let source = FakeSource::osms().pictures_taking(20);
    let requests = source.picture_requests.clone();
    let mut minerva = Minerva::new(source);
    arrive(&mut minerva, CONSTRUCTION_SAMPLE, 0);
    minerva.tick(400);
    let first = requests.lock().unwrap().clone();
    assert_eq!(first.len(), 1, "{first:?}");

    arrive(&mut minerva, SUNSET_SAMPLE, 450);
    let mut starts = vec![400];
    for at in (800..=20_000).step_by(50) {
        let before = requests.lock().unwrap().len();
        minerva.tick(at);
        starts.extend(std::iter::repeat_n(at, requests.lock().unwrap().len() - before));
    }

    assert!(starts.windows(2).all(|w| w[1] - w[0] >= 1_000), "one at a time: {starts:?}");
    assert_eq!(requests_for(&requests, &first[0]), 1);
}

#[test]
fn every_shown_maps_mobs_animations_are_asked_for_and_each_only_once() {
    let source = FakeSource::osms();
    let requests = source.picture_requests.clone();
    let mut minerva = Minerva::new(source);

    let state = arrive(&mut minerva, CONSTRUCTION_SAMPLE, 0);
    let construction_site = listed_ids(&state);
    assert!(construction_site.len() > 5, "a map with more mobs than the panel once listed");
    tick_through(&mut minerva, 400, 20_000);
    arrive(&mut minerva, KERNING_SAMPLE, 20_050);
    tick_through(&mut minerva, 20_400, 40_000);
    let state = arrive(&mut minerva, SUNSET_SAMPLE, 40_050);
    let sunset = listed_ids(&state);
    tick_through(&mut minerva, 40_400, 60_000);
    arrive(&mut minerva, CONSTRUCTION_SAMPLE, 60_050);
    tick_through(&mut minerva, 60_400, 80_000);

    let shown: Vec<&String> = construction_site.iter().chain(&sunset).collect();
    for id in &shown {
        assert_eq!(requests_for(&requests, &animation(&format!("gif{id}"))), 1, "mob {id}'s animation");
    }
    let asked = animations_asked(&requests);
    assert!(
        asked.iter().all(|path| shown.iter().any(|id| *path == animation(&format!("gif{id}")))),
        "only the shown maps' mobs: {asked:?}"
    );
}

#[test]
fn without_a_map_or_its_mobs_there_are_no_cards() {
    let mut minerva = Minerva::new(FakeSource::osms());
    assert!(minerva.tick(0).mob_cards.is_empty());

    // Kerning City is a town.
    let state = arrive(&mut minerva, KERNING_SAMPLE, 100);
    assert!(matches!(state.data, MapDataStatus::Ready(_)));
    assert!(state.mob_cards.is_empty());
}

#[test]
fn clicking_a_listed_mobs_name_opens_its_meowdb_page() {
    let mut minerva = Minerva::new(FakeSource::osms());
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    assert_eq!(card(minerva.panel_state(), "3").url, "https://meowdb.com/msclassic/monsters/3");
    assert_eq!(
        minerva.open_mob_page("3"),
        Some(Request::OpenInBrowser { url: "https://meowdb.com/msclassic/monsters/3".into() })
    );
    // Only the mobs the panel lists.
    assert_eq!(minerva.open_mob_page("700000"), None);
}

#[test]
fn with_mob_cards_the_panel_credits_only_osms_and_meowdb() {
    let mut minerva = Minerva::new(FakeSource::osms());
    arrive(&mut minerva, SUNSET_SAMPLE, 0);

    let state = minerva.tick(400).clone();

    assert!(state.mob_cards.iter().all(|c| c.image.is_some()));
    assert_eq!(state.credit, Some(OSMS_AND_MEOWDB_CREDIT));
    assert_eq!(OSMS_AND_MEOWDB_CREDIT, "Data: OSMS Data Explorer (osmsdataexplorer.com) · Pages: NiaMeowDB (meowdb.com)");
}
