//! Name matcher: snaps a raw title reading to the closest street name and map name
//! pair in the map list, scoring both lines together.
//!
//! Many map names differ only in a trailing Roman numeral ("The Tree That Grew I",
//! "II", "III"), one edit apart, so edit distance alone would pick the shortest when
//! the numeral is misread. Between such maps the numeral read decides; when it can't
//! be made out, the reading is one of them, not any one in particular.

use crate::map_index::MapEntry;

/// Lowest score (0..=1) that counts as a confident match. A whole-letter slip or two
/// (e.g. "Keming City" for "Kerning City") stays well above it; a half-covered or
/// unrelated title falls below.
const CONFIDENT: f32 = 0.8;

/// What a key holds for a glyph ocrs couldn't name: it gives `?` for those.
const UNKNOWN: char = '?';

/// A map from the index, with its names trimmed and normalised for matching.
pub struct KnownMap {
    pub entry: MapEntry,
    street_key: Vec<char>,
    map_key: Vec<char>,
    /// The map name's key without its trailing Roman numeral, and that numeral's key
    /// (empty when the name has none).
    stem_key: Vec<char>,
    numeral_key: Vec<char>,
}

/// A confident match: which map, and how well it matched (0..=1).
#[derive(Debug, Clone, Copy)]
pub struct Snap {
    pub index: usize,
    pub score: f32,
}

/// What a reading matches.
#[derive(Debug, Clone)]
pub enum Match {
    /// One title.
    Sure(Snap),
    /// One of these maps, whose names differ only in a trailing Roman numeral that
    /// the reading doesn't make out. One map stands for each title, as in [`Snap`].
    OneOf { maps: Vec<usize>, score: f32 },
}

pub struct Matcher {
    maps: Vec<KnownMap>,
}

impl Matcher {
    pub fn new(index: Vec<MapEntry>) -> Self {
        let maps = index
            .into_iter()
            .map(|e| {
                // OSMS's names can carry trailing spaces ("A Hill West of Henesys ").
                let entry = MapEntry {
                    id: e.id.trim().to_string(),
                    street_name: e.street_name.trim().to_string(),
                    map_name: e.map_name.trim().to_string(),
                    region: e.region.trim().to_string(),
                    mark: e.mark,
                };
                let (stem, numeral) = split_numeral(&entry.map_name);
                KnownMap {
                    street_key: key(&entry.street_name),
                    map_key: key(&entry.map_name),
                    stem_key: key(stem),
                    numeral_key: key(numeral),
                    entry,
                }
            })
            .collect();
        Self { maps }
    }

    pub fn map(&self, index: usize) -> &MapEntry {
        &self.maps[index].entry
    }

    /// The index of the map with this ID, if it is listed.
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.maps.iter().position(|known| known.entry.id == id)
    }

    /// Every map with the same title as map `index` (itself included), in index order.
    pub fn same_title_as(&self, index: usize) -> Vec<usize> {
        (0..self.maps.len()).filter(|&i| self.same_title(i, index)).collect()
    }

    /// The map whose street name and map name best match the reading, if the match
    /// is confident and no other title matches as well; or, when the best is one of
    /// several names differing only in their trailing numeral and the reading's
    /// numeral doesn't single one out, all of those.
    ///
    /// Maps sharing one title score the same, and the first in the index stands for
    /// them all: [`Matcher::same_title_as`] gives the rest.
    pub fn snap(&self, street: &str, map: &str) -> Option<Match> {
        let (street, map) = (key(street), key(map));
        let scores: Vec<f32> = self.maps.iter().map(|known| score(&street, &map, known)).collect();
        // The first of the best-scoring maps.
        let best = (0..scores.len()).fold(None, |best: Option<usize>, i| match best {
            Some(b) if scores[b] >= scores[i] => Some(b),
            _ => Some(i),
        })?;
        let family: Vec<usize> = (0..self.maps.len()).filter(|&i| self.same_stem(i, best)).collect();
        let runner_up = (0..self.maps.len())
            .filter(|i| !family.contains(i))
            .map(|i| scores[i])
            .fold(0.0, f32::max);
        // One map per title, in index order.
        let titles: Vec<usize> =
            family.iter().copied().filter(|&i| self.same_title_as(i)[0] == i).collect();
        let score = scores[best];
        if score < CONFIDENT || score <= runner_up {
            return None;
        }
        if titles.len() == 1 {
            return Some(Match::Sure(Snap { index: titles[0], score }));
        }
        let read = read_numeral(&map, &self.maps[best].stem_key);
        let numbered = |i: usize| read.as_ref() == Some(&self.maps[i].numeral_key);
        match titles.iter().find(|&&i| numbered(i) && scores[i] >= CONFIDENT) {
            Some(&index) => Some(Match::Sure(Snap { index, score: scores[index] })),
            None => Some(Match::OneOf { maps: titles, score }),
        }
    }

    fn same_title(&self, a: usize, b: usize) -> bool {
        let (a, b) = (&self.maps[a], &self.maps[b]);
        a.street_key == b.street_key && a.map_key == b.map_key
    }

    /// Same street name, and map names the same but for a trailing Roman numeral.
    fn same_stem(&self, a: usize, b: usize) -> bool {
        let (a, b) = (&self.maps[a], &self.maps[b]);
        a.street_key == b.street_key && a.stem_key == b.stem_key
    }
}

/// A map name and its trailing Roman numeral word ("The Tree That Grew", "II"); the
/// numeral is empty when the name doesn't end in one.
fn split_numeral(name: &str) -> (&str, &str) {
    match name.rsplit_once(' ') {
        Some((stem, last)) if !last.is_empty() && last.chars().all(|c| matches!(c, 'I' | 'V' | 'X')) => {
            (stem, last)
        }
        _ => (name, ""),
    }
}

/// The numeral a map line reading ends in, given the name it shares with others but
/// for that numeral: what is left once the closest match to `stem` is taken off its
/// front. `None` when that isn't plain numeral letters, or isn't clear-cut.
fn read_numeral(reading: &[char], stem: &[char]) -> Option<Vec<char>> {
    let is_tail = |c: &char| matches!(c, 'i' | 'v' | 'x') || *c == UNKNOWN;
    // Every split of the reading into a front and a tail of numeral letters (or
    // unknown glyphs), by how far the front is from the stem.
    let splits: Vec<(usize, &[char])> = (0..=reading.len())
        .rev()
        .take_while(|&at| at == reading.len() || is_tail(&reading[at]))
        .map(|at| (levenshtein(&reading[..at], stem), &reading[at..]))
        .collect();
    let closest = splits.iter().map(|(edits, _)| *edits).min()?;
    let mut tails = splits.iter().filter(|(edits, _)| *edits == closest).map(|(_, tail)| *tail);
    match (tails.next(), tails.next()) {
        (Some(tail), None) if !tail.contains(&UNKNOWN) => Some(tail.to_vec()),
        _ => None,
    }
}

/// Letters and digits only, lower-cased: OCR often drops or adds spaces ("Rocky
/// RoadI") and punctuation carries little signal. Glyphs the game font draws as one
/// upright stroke (I, l, 1, |, !) are folded together, on both sides of a match, and
/// an unknown glyph is kept, so a misread numeral counts as misread rather than absent.
pub(crate) fn key(s: &str) -> Vec<char> {
    s.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            'l' | '1' | '|' | '!' => Some('i'),
            UNKNOWN => Some(UNKNOWN),
            c if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

/// 1 minus the edits needed on both lines, over their combined length.
fn score(street: &[char], map: &[char], known: &KnownMap) -> f32 {
    let edits = levenshtein(street, &known.street_key) + levenshtein(map, &known.map_key);
    let len = street.len().max(known.street_key.len()) + map.len().max(known.map_key.len());
    if len == 0 { 0.0 } else { 1.0 - edits as f32 / len as f32 }
}

pub(crate) fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = prev[j] + usize::from(ca != cb);
            cur[j + 1] = substitute.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}
