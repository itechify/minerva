//! First-run setup: the Title Region the player marks, its test read, and when to ask for
//! setup again. The EXP figures need no mark: they are found in the Status Bar (ADR 0017).

use crate::panel::{ScreenRect, Size};
use crate::{CurrentMap, Rect, default_title_region};
use serde::{Deserialize, Serialize};

/// The credit for a recognised test read: the names it shows are OSMS Data Explorer's.
const MAP_NAMES_CREDIT: &str = "Map names from OSMS Data Explorer";

/// Why the player needs to (re)do setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SetupReason {
    /// No Title Region has been marked yet.
    FirstRun,
    /// The game's client area is no longer the size it had when the Title Region was
    /// marked, so the title has probably moved.
    ResolutionChanged,
    /// The player asked to redo setup. The saved Title Region is read until setup is
    /// finished again.
    Requested,
}

/// The test read of a freshly marked Title Region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum TestRead {
    /// Waiting for the first frame captured from the marked region.
    Reading,
    /// The latest frame snapped to this real map.
    Recognised { map: CurrentMap },
    /// The latest frame didn't match a real map confidently (or the map list couldn't
    /// be had).
    Failed,
}

/// Setup, as the overlay and the setup window show it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupState {
    /// Why setup is needed; `None` once the Title Region is marked and still fits.
    pub needed: Option<SetupReason>,
    /// The test read of the region just marked; `None` when none is being marked.
    pub test_read: Option<TestRead>,
    /// "Map names from OSMS Data Explorer" when the test read shows a map.
    pub credit: Option<&'static str>,
    /// The setup window shows: when setup becomes needed, and when the player asks
    /// for it, until it is finished or closed.
    pub window_open: bool,
    /// The keyboard-adjustable region in Minerva's own marking window, before it
    /// is committed for a test read.
    pub selection: Option<Rect>,
    /// What `selection` is being marked around.
    pub selection_target: Option<MarkTarget>,
}

/// What the player marks in the marking window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarkTarget {
    /// The minimap title: the Title Region.
    Title,
}

/// An arrow key in Minerva's own marking window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionDirection {
    Left,
    Right,
    Up,
    Down,
}

/// A marked Title Region and the size of the game client area it was marked on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleRegion {
    /// What the player marked, relative to the client area's top-left corner.
    pub marked: Rect,
    /// The client area's size when it was marked.
    pub client: Size,
}

/// Before the map list is loaded, the longest name in it is taken to be as long as the
/// longest in OSMS's map list, "Physical Fitness Test <Normal Waiting Room>".
const FALLBACK_LONGEST_NAME: usize = 43;

/// Title width allowed per character, per px of marked height. On the samples the two
/// title lines span 36 px and average 8.9 px per character (0.25); the extra fifth
/// covers wide letters and a mark cut tighter than the text.
const CHAR_WIDTH_PER_MARKED_HEIGHT: f32 = 0.3;

/// The smallest mark, either way.
const MIN_MARK_SIZE: u32 = 8;

#[derive(Debug, Default)]
pub(crate) struct Setup {
    pub game: Option<ScreenRect>,
    /// The Title Region kept when setup was last finished.
    pub saved: Option<TitleRegion>,
    /// The region being test-read, and how that is going.
    marking: Option<(TitleRegion, TestRead)>,
    /// The keyboard-adjustable selection in the marking window, what it is around, and
    /// the client area's size it was made on.
    selection: Option<(MarkTarget, Rect, Size)>,
    /// The player asked to redo setup and hasn't finished or cancelled it.
    requested: bool,
    /// The setup window is showing.
    window_open: bool,
    /// Why setup was needed when the window was last opened for it (or closed).
    opened_for: Option<SetupReason>,
}

impl Setup {
    /// The player marked the Title Region: test-read it.
    pub fn mark(&mut self, marked: Rect) {
        if let Some((client, marked)) = self.fitted(marked) {
            self.marking = Some((TitleRegion { marked, client }, TestRead::Reading));
            self.selection = None;
        }
    }

    /// `marked` kept inside the game's client area, and that area's size.
    fn fitted(&self, marked: Rect) -> Option<(Size, Rect)> {
        let game = self.game?;
        let marked = fit_mark(marked, game)?;
        Some((Size { width: game.width, height: game.height }, marked))
    }

    /// Open the marking window on `target`, with a keyboard-adjustable selection where
    /// it was last marked (being marked, or kept, if that fits), or else where it
    /// usually is.
    pub fn begin_selection(&mut self, target: MarkTarget) {
        let Some(game) = self.game.filter(|_| self.window_open) else { return };
        let client = Size { width: game.width, height: game.height };
        let fits = |marked: Rect, on: Size| (on == client).then_some(marked);
        let marked = match target {
            MarkTarget::Title => self.marking.as_ref().map(|(region, _)| region).or(self.saved.as_ref())
                .and_then(|region| fits(region.marked, region.client))
                .unwrap_or_else(|| default_title_region(game.width, game.height)),
        };
        self.selection = fit_mark(marked, game).map(|marked| (target, marked, client));
    }

    pub fn adjust_selection(&mut self, direction: SelectionDirection, resize: bool, coarse: bool) {
        let Some(game) = self.game else { return };
        let Some((_, r, _)) = &mut self.selection else { return };
        let step = if coarse { 10 } else { 1 };
        let (value, limit, increasing) = match (direction, resize) {
            (SelectionDirection::Left, false) => (&mut r.x, game.width - r.width, false),
            (SelectionDirection::Right, false) => (&mut r.x, game.width - r.width, true),
            (SelectionDirection::Up, false) => (&mut r.y, game.height - r.height, false),
            (SelectionDirection::Down, false) => (&mut r.y, game.height - r.height, true),
            (SelectionDirection::Left, true) => (&mut r.width, game.width - r.x, false),
            (SelectionDirection::Right, true) => (&mut r.width, game.width - r.x, true),
            (SelectionDirection::Up, true) => (&mut r.height, game.height - r.y, false),
            (SelectionDirection::Down, true) => (&mut r.height, game.height - r.y, true),
        };
        *value = if increasing { value.saturating_add(step).min(limit) }
            else { value.saturating_sub(step).max(if resize { MIN_MARK_SIZE } else { 0 }) };
    }

    /// Test-read the selection, as a mark around what it was opened on. Returns that.
    pub fn confirm_selection(&mut self) -> Option<MarkTarget> {
        let (target, marked, _) = self.selection?;
        match target {
            MarkTarget::Title => self.mark(marked),
        }
        Some(target)
    }

    pub fn cancel_selection(&mut self) {
        self.selection = None;
    }

    /// Finish setup, keeping the marked Title Region if its test read recognised a map, or
    /// with none being marked, the kept one if it still fits. Nothing changes while the
    /// mark's test read hasn't succeeded.
    pub fn finish(&mut self) {
        let title = match &self.marking {
            Some((region, TestRead::Recognised { .. })) => *region,
            Some(_) => return,
            None => match self.saved {
                Some(saved) if self.fits(saved.client) => saved,
                _ => return,
            },
        };
        self.saved = Some(title);
        self.marking = None;
        self.selection = None;
        self.requested = false;
    }

    /// The re-setup hotkey: open setup, even if it was needed before and closed.
    pub fn request(&mut self) {
        self.requested = true;
        self.window_open = true;
    }

    /// The player closed setup without finishing it.
    pub fn cancel(&mut self) {
        self.marking = None;
        self.selection = None;
        self.requested = false;
        self.window_open = false;
    }

    /// Open the setup window when setup becomes needed (or needed for a new reason),
    /// and close it once it isn't.
    pub fn follow_needed(&mut self) {
        if self.selection.is_some_and(|(_, _, client)| self.game.is_none() || !self.fits(client)) {
            self.selection = None;
        }
        let needed = self.needed();
        if needed.is_none() {
            self.window_open = false;
        } else if needed != self.opened_for {
            self.window_open = true;
        }
        self.opened_for = needed;
    }

    /// The map a frame snapped to (`None`: unreadable or not confident). While a region
    /// is being marked, that frame is its test read.
    pub fn on_read(&mut self, map: Option<CurrentMap>) {
        if let Some((_, read)) = &mut self.marking {
            *read = map.map_or(TestRead::Failed, |map| TestRead::Recognised { map });
        }
    }

    pub fn state(&self) -> SetupState {
        let test_read = self.marking.as_ref().map(|(_, read)| read.clone());
        SetupState {
            needed: self.needed(),
            credit: matches!(test_read, Some(TestRead::Recognised { .. })).then_some(MAP_NAMES_CREDIT),
            test_read,
            window_open: self.window_open,
            selection: self.selection.map(|(_, marked, _)| marked),
            selection_target: self.selection.map(|(target, _, _)| target),
        }
    }

    fn needed(&self) -> Option<SetupReason> {
        match self.saved {
            None => Some(SetupReason::FirstRun),
            Some(saved) if !self.fits(saved.client) => Some(SetupReason::ResolutionChanged),
            Some(_) if self.requested => Some(SetupReason::Requested),
            Some(_) => None,
        }
    }

    /// Whether a region was marked on a client area the size of the game's now. Without
    /// a game window there is nothing to compare, so it is taken to fit.
    fn fits(&self, client: Size) -> bool {
        self.game.is_none_or(|g| (g.width, g.height) == (client.width, client.height))
    }

    /// Where to capture the Title Region on screen: the region being marked, else the
    /// saved one. `None` while there is no game window, or no region marked on a client
    /// area of the game's current size: reading the wrong spot would go unnoticed.
    pub fn title_region(&self, longest_name: Option<usize>) -> Option<ScreenRect> {
        let game = self.game?;
        let region = match &self.marking {
            Some((region, _)) => region,
            None => self.saved.as_ref()?,
        };
        if !self.fits(region.client) {
            return None;
        }
        let fit = (longest_name.unwrap_or(FALLBACK_LONGEST_NAME) as f32
            * region.marked.height as f32
            * CHAR_WIDTH_PER_MARKED_HEIGHT)
            .ceil() as u32;
        Some(on_screen(extend(region.marked, game, fit), game))
    }
}

/// Both pointer and keyboard marks stay inside the game client area.
fn fit_mark(marked: Rect, game: ScreenRect) -> Option<Rect> {
    if game.width < MIN_MARK_SIZE || game.height < MIN_MARK_SIZE {
        return None;
    }
    let width = marked.width.clamp(MIN_MARK_SIZE, game.width);
    let height = marked.height.clamp(MIN_MARK_SIZE, game.height);
    Some(Rect {
        x: marked.x.min(game.width - width), y: marked.y.min(game.height - height), width, height,
    })
}

/// The region to capture for a mark: the marked top-left corner and height, widened to
/// at least `fit` px (the minimap panel widens for long map names), all within the client
/// area.
fn extend(marked: Rect, client: ScreenRect, fit: u32) -> Rect {
    Rect {
        x: marked.x,
        y: marked.y,
        width: marked.width.max(fit).min(client.width.saturating_sub(marked.x)),
        height: marked.height.min(client.height.saturating_sub(marked.y)),
    }
}

/// A client-relative rectangle on screen.
fn on_screen(r: Rect, game: ScreenRect) -> ScreenRect {
    ScreenRect { x: game.x + r.x as i32, y: game.y + r.y as i32, width: r.width, height: r.height }
}
