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
//! more than one distinct read are here, bar the `V` as `M`, `W` or `t` and the `D` as `Li`, once
//! each. A `?` is the
//! recogniser giving up on one glyph, any glyph. Each costs well under an edit, so that two cheap
//! slips are still less than a name over, but not so little that a name is only slips away from
//! many. Names are compared as [`crate::item_lookup::item_key`] writes them: lower case, with `l`,
//! `1` and `|` as `i`, and the punctuation the recogniser also adds (`'ery`, `"er`, `(ery`) gone.

use crate::item_lookup::closest_within;

/// What one plain edit costs: costs are in tenths of an edit.
const EDIT: u32 = 10;
/// A slip, or a `?` for a glyph.
const SLIP: u32 = 4;
/// The closest name must be at least this much further from the next, in tenths of an edit: a
/// whole edit, so that one more slip between them is not a lead.
const LEAD_MARGIN: u32 = EDIT;
/// And the next name must be at least this many times as far (in halves: 3 is one and a half
/// times), so that a read a few slips from one name is not named when it is also a few from another.
const LEAD_RATIO_HALVES: u32 = 3;
/// How similar a read must be to a name: 1 minus its cost over the longer's length in edits.
const CLOSE_ENOUGH: f32 = 0.75;

/// One way the recogniser reads a letter of an Item's name wrong, again and again.
struct Slip {
    /// The letter of the name.
    letter: char,
    /// What the recogniser reads it as: nothing, or one glyph or more.
    read_as: &'static str,
    /// Whether it only happens at the name's start.
    only_at_start: bool,
}

const fn slip(letter: char, read_as: &'static str, only_at_start: bool) -> Slip {
    Slip { letter, read_as, only_at_start }
}

const SLIPS: &[Slip] = &[
    slip('d', "ci", false),
    slip('d', "ii", false),
    slip('d', "i", false),
    slip('b', "bi", false),
    slip('b', "ei", false),
    slip('b', "e", false),
    slip('b', "a", false),
    slip('n', "ri", false),
    slip('r', "i", false),
    slip('r', "", false),
    slip('r', "f", true),
    slip('y', "v", false),
    slip('y', "", false),
    slip('p', "f", false),
    slip('o', "n", false),
    slip('o', "u", false),
    slip('f', "t", false),
    slip('v', "", true),
    slip('v', "m", true),
    slip('v', "w", true),
    slip('v', "t", true),
];

/// The glyph the recogniser draws for one it can't read.
const UNKNOWN: char = '?';

/// What turning `name` into `read` costs, in tenths of an edit: a plain edit is 10, a slip of
/// `slips` is 4.
fn cost(read: &[char], name: &[char], slips: &[Slip]) -> u32 {
    let (read_len, name_len) = (read.len(), name.len());
    // The least it costs to turn the first `i` glyphs of the read into the first `j` of the name.
    let mut cheapest = vec![u32::MAX; (read_len + 1) * (name_len + 1)];
    let cell = |i: usize, j: usize| i * (name_len + 1) + j;
    cheapest[cell(0, 0)] = 0;
    for i in 0..=read_len {
        for j in 0..=name_len {
            let so_far = cheapest[cell(i, j)];
            if so_far == u32::MAX {
                continue;
            }
            let mut step_to = |i: usize, j: usize, step: u32| {
                let to = &mut cheapest[cell(i, j)];
                *to = (*to).min(so_far + step);
            };
            if i < read_len && j < name_len {
                let step = match read[i] == name[j] {
                    true => 0,
                    false if read[i] == UNKNOWN => SLIP,
                    false => EDIT,
                };
                step_to(i + 1, j + 1, step);
            }
            if i < read_len {
                step_to(i + 1, j, EDIT);
            }
            if j < name_len {
                step_to(i, j + 1, EDIT);
                for slip in slips {
                    let glyphs = slip.read_as.chars().count();
                    let fits = slip.letter == name[j]
                        && (!slip.only_at_start || j == 0)
                        && read.len() >= i + glyphs
                        && read[i..i + glyphs].iter().copied().eq(slip.read_as.chars());
                    if fits {
                        step_to(i + glyphs, j + 1, SLIP);
                    }
                }
            }
        }
    }
    cheapest[cell(read_len, name_len)]
}

/// Which of `names` (as [`crate::item_lookup::item_key`] writes them, each once) the name `read`
/// is: the one it is, or the one it is closest to by the recogniser's usual slips when that one
/// is close enough and clearly leads the next. Failing that, the one plain edits make it (within
/// 75%, two edits nearer than any other), but only if it is that same closest one: where the slips
/// leave a read unclear, plain edits don't name an Item the slips put second. `None` when none is,
/// and so when two are as near.
pub(crate) fn closest_by_slips(read: &[char], names: &[&[char]]) -> Option<usize> {
    let Closest { index, clear } = closest_by_cost(read, names)?;
    if clear {
        return Some(index);
    }
    closest_plain(read, names).filter(|&plain| plain == index)
}

/// The same, by plain edits alone.
pub(crate) fn closest_plain(read: &[char], names: &[&[char]]) -> Option<usize> {
    closest_within(read, names, CLOSE_ENOUGH)
}

/// The name a read is closest to by slips, and whether that is clearly so.
struct Closest {
    index: usize,
    /// Close enough to the name, and clearly nearer it than the next.
    clear: bool,
}

/// The one name nearest `read` by [`cost`]; `None` if two are as near, or `read` is one.
fn closest_by_cost(read: &[char], names: &[&[char]]) -> Option<Closest> {
    if let Some(same) = names.iter().position(|name| *name == read) {
        return Some(Closest { index: same, clear: true });
    }
    let mut costs: Vec<(usize, u32)> = names.iter().map(|name| cost(read, name, SLIPS)).enumerate().collect();
    costs.sort_by_key(|&(_, cost)| cost);
    let &(index, least) = costs.first()?;
    if costs.get(1).is_some_and(|&(_, next)| next == least) {
        return None;
    }
    let len = read.len().max(names[index].len());
    let similar = len > 0 && 1.0 - least as f32 / (EDIT as usize * len) as f32 >= CLOSE_ENOUGH;
    let leads = |next: u32| next >= least + LEAD_MARGIN && 2 * next >= LEAD_RATIO_HALVES * least;
    let clear = costs.get(1).is_none_or(|&(_, next)| leads(next));
    Some(Closest { index, clear: similar && clear })
}

