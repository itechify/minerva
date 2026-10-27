//! The recogniser's usual slips on the Pickup Log's font, as an edit distance (ADR 0026): an
//! Item line's name is read a few glyphs wrong in the same ways again and again, and a read that
//! is only such slips away from an Item's name is that Item, where plain edits would call it
//! three or four away.
//!
//! The slips are the ones the Bug Report of 0.15.0, 2026-10-26 23:05 shows (a quest handed in 100
//! Very Dry Branches, and 10 of the lines read as no Item), found by aligning its 117 distinct
//! Item reads with the Items they plainly are: a `D` read as `Ci`, `Li` or `[i`; a `B` as `Bl`,
//! `El`, `E` or `A`; a `V` at the start of a name lost or read as `M`, `W` or `t`; a `y` as a
//! `v`; an `n` as `r` or `ri`; a `p` as `f`, an `o` as `n` or `u`; and so on. Only those seen in
//! more than one read are here, bar the `V` as `M`, `W` or `t` and the `D` as `Li`, once each. A `?` is the
//! recogniser giving up on one glyph, any glyph. Each costs well under an edit, so that two cheap
//! slips are still less than a name over, but not so little that a name is only slips away from
//! many. Names are compared as [`crate::item_lookup::item_key`] writes them: lower case, with `l`,
//! `1` and `|` as `i`, and the punctuation the recogniser also adds (`'ery`, `"er`, `(ery`) gone.

use crate::item_lookup::closest_within;

/// What one plain edit costs: costs are in tenths of an edit.
const EDIT: u32 = 10;
/// A slip, or a `?` for a glyph.
const SLIP: u32 = 4;
/// How much further the next name must be than the closest, for the closest to be the one.
const LEAD: u32 = 18;
/// How similar a read must be to a name: 1 minus its cost over the longer's length in edits.
const CLOSE_ENOUGH: f32 = 0.75;

/// `(the letter of the name, what the recogniser reads it as, only at the name's start)`.
const SLIPS: &[(char, &str, bool)] = &[
    ('d', "ci", false),
    ('d', "ii", false),
    ('d', "i", false),
    ('b', "bi", false),
    ('b', "ei", false),
    ('b', "e", false),
    ('b', "a", false),
    ('b', "i", false),
    ('n', "r", false),
    ('n', "ri", false),
    ('r', "i", false),
    ('r', "", false),
    ('r', "f", true),
    ('c', "e", false),
    ('c', "i", false),
    ('y', "v", false),
    ('y', "", false),
    ('p', "f", false),
    ('o', "n", false),
    ('o', "u", false),
    ('o', "c", true),
    ('o', "d", true),
    ('o', "p", true),
    ('f', "t", false),
    ('s', "e", false),
    ('v', "", true),
    ('v', "m", true),
    ('v', "w", true),
    ('v', "t", true),
];

/// The glyph the recogniser draws for one it can't read.
const UNKNOWN: char = '?';

/// What turning `name` into `read` costs, in tenths of an edit: a plain edit is 10, a slip is 4.
fn cost(read: &[char], name: &[char]) -> u32 {
    let (n, m) = (read.len(), name.len());
    let mut at = vec![u32::MAX; (n + 1) * (m + 1)];
    let ix = |i: usize, j: usize| i * (m + 1) + j;
    at[ix(0, 0)] = 0;
    for i in 0..=n {
        for j in 0..=m {
            let so_far = at[ix(i, j)];
            if so_far == u32::MAX {
                continue;
            }
            let mut reach = |i: usize, j: usize, more: u32| {
                let to = &mut at[ix(i, j)];
                *to = (*to).min(so_far + more);
            };
            if i < n && j < m {
                let more = match read[i] == name[j] {
                    true => 0,
                    false if read[i] == UNKNOWN => SLIP,
                    false => EDIT,
                };
                reach(i + 1, j + 1, more);
            }
            if i < n {
                reach(i + 1, j, EDIT);
            }
            if j < m {
                reach(i, j + 1, EDIT);
                for &(from, to, at_start) in SLIPS {
                    let read_as = to.chars().count();
                    let fits = from == name[j]
                        && (!at_start || j == 0)
                        && read.len() >= i + read_as
                        && read[i..i + read_as].iter().copied().eq(to.chars());
                    if fits {
                        reach(i + read_as, j + 1, SLIP);
                    }
                }
            }
        }
    }
    at[ix(n, m)]
}

/// Which of `names` (as [`crate::item_lookup::item_key`] writes them, each once) the name `read`
/// is: the one it is, or the one it is closest to by the recogniser's usual slips when that one
/// is close enough and clearly leads the next, or else the one that plain edits make it (within
/// 75%, two edits nearer than any other), as before slips were known: the slips only add to what
/// is named. `None` when none is, and so when two are as near.
pub(crate) fn closest_by_slips(read: &[char], names: &[&[char]]) -> Option<usize> {
    by_slips(read, names).or_else(|| closest_plain(read, names))
}

/// The same, by plain edits alone.
pub(crate) fn closest_plain(read: &[char], names: &[&[char]]) -> Option<usize> {
    closest_within(read, names, CLOSE_ENOUGH)
}

fn by_slips(read: &[char], names: &[&[char]]) -> Option<usize> {
    if let Some(same) = names.iter().position(|name| *name == read) {
        return Some(same);
    }
    let mut costs: Vec<(usize, u32)> = names.iter().map(|name| cost(read, name)).enumerate().collect();
    costs.sort_by_key(|&(_, cost)| cost);
    let &(closest, least) = costs.first()?;
    let len = read.len().max(names[closest].len());
    let similar = len > 0 && 1.0 - least as f32 / (EDIT as usize * len) as f32 >= CLOSE_ENOUGH;
    let clear = costs.get(1).is_none_or(|&(_, next)| next >= least + LEAD);
    (similar && clear).then_some(closest)
}
