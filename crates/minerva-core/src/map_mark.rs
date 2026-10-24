//! Map Marks (GLOSSARY): the emblem the game's Mini Map draws beside a map's title, for the
//! area the map belongs to (Henesys's shield, Kerning City's stone, Sleepywood's skull). The
//! panel shows it beside a map's name, so a glance says where in the world a map is.
//!
//! OSMS's map data names each map's mark and where its picture is (ADR 0008); a map names
//! its mark by OSMS's name for it, and the panel state carries one table of the marks, each
//! with the area's name as players know it and its picture once it's in. There are a few
//! (11 today), so every one is asked for first, before any other picture, and kept.

use crate::osms::{DATA_URL, MapSet, PICTURES_URL};
use crate::pictures::Pictures;
use serde::Serialize;
use std::collections::BTreeMap;

/// A Map Mark, as the panel shows it beside a map's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapMark {
    /// The area it stands for, as players know it: "Lith Harbor" for OSMS's `Rith`. Its
    /// picture's alternative text.
    pub name: String,
    /// Where the overlay loads its picture from, through Minerva's own URL scheme;
    /// `None` until it's in, or if it can't be had.
    pub image: Option<String>,
}

/// Every Map Mark in `set`, by OSMS's name for it, with its picture as `pictures` has it.
pub(crate) fn table(set: &MapSet, pictures: &Pictures) -> BTreeMap<String, MapMark> {
    set.marks()
        .map(|(mark, path)| {
            let image = pictures.url(path).map(String::from);
            (mark.to_string(), MapMark { name: area_name(mark), image })
        })
        .collect()
}

/// The paths of every Map Mark's picture in `set`, as [`crate::MapDataSource::picture`]
/// takes them.
pub(crate) fn pictures(set: &MapSet) -> impl Iterator<Item = String> + '_ {
    set.marks().map(|(_, path)| path.to_string())
}

/// The area mark `mark` stands for, as players know it. OSMS names marks as the game's
/// files do, some of them not as players call the place; any other is its name in words
/// (`KerningCity` is "Kerning City").
pub(crate) fn area_name(mark: &str) -> String {
    match mark {
        "Rith" => "Lith Harbor".into(),
        "MushroomVillage" => "Mushroom Town".into(),
        "SouthPerry" => "Southperry".into(),
        other => in_words(other),
    }
}

/// `ForgottenHollow` as "Forgotten Hollow": a space before each capital that follows a
/// lowercase letter or a digit.
fn in_words(name: &str) -> String {
    let mut words = String::new();
    let mut after_lower = false;
    for c in name.chars() {
        if c.is_uppercase() && after_lower {
            words.push(' ');
        }
        after_lower = c.is_lowercase() || c.is_ascii_digit();
        words.push(c);
    }
    words
}

/// The path, below [`PICTURES_URL`], of a mark's picture that `maps.json` gives below
/// [`DATA_URL`] (`"images/map_marks/Henesys.png"`): unlike the animations, the marks are the
/// data set's own. Only a plain relative path is taken.
pub(crate) fn picture_path(path: &str) -> Result<String, String> {
    let plain = |part: &str| {
        !part.is_empty() && part != "." && part != ".." && part.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    };
    if !path.split('/').all(plain) {
        return Err(format!("{path:?} isn't a picture's path"));
    }
    let set_dir = DATA_URL.strip_prefix(PICTURES_URL).ok_or("the data set isn't below the pictures")?;
    Ok(format!("{set_dir}{path}"))
}
