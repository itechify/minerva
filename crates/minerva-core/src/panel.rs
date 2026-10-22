//! Panel layout: expanded or collapsed, dragging, and where the panel sits on screen.

use serde::{Deserialize, Serialize};

/// A rectangle in physical screen pixels (virtual-screen coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// The game window as the shell sees it, in physical screen pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameWindow {
    /// The client area: what the game draws in, without caption or borders.
    pub client: ScreenRect,
    /// The visible window frame (caption and borders, without the invisible resize
    /// borders). A docked panel sits flush against it.
    pub frame: ScreenRect,
    /// The work area (screen minus taskbar) of every monitor. A docked panel must fit
    /// wholly inside one of them.
    pub screens: Vec<ScreenRect>,
}

/// Which window is in front: the game, one of Minerva's own, or another app's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Foreground {
    Game,
    Minerva,
    OtherApp,
}

/// The overlay's size when expanded and when collapsed to an icon (physical px).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelSizes {
    pub expanded: Size,
    pub collapsed: Size,
}

/// Something the player did (through a hotkey or the overlay).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    /// Collapse the panel to its icon, or expand it again.
    ToggleExpanded,
    /// The player dragged the overlay window so its top-left corner is at (x, y),
    /// physical screen px. It moves an inside or detached panel, and a docked panel
    /// never.
    PanelMoved { x: i32, y: i32 },
    /// The player dragged a bottom corner of the expanded panel so it is `width` CSS px
    /// (logical) wide. Its height follows its content all the same.
    PanelResized { width: u32 },
    /// Switch to the next placement: inside, docked right, docked left, detached.
    CyclePlacement,
    /// Show the next of the maps sharing the shown map's title, when the core couldn't
    /// tell which one the player is on (see [`crate::Candidates`]).
    ChooseCandidate,
    /// The player marked the Title Region (relative to the game client area) and wants
    /// a test read of it.
    MarkTitleRegion(crate::Rect),
    /// Open the marking window on the Title Region, with a keyboard-adjustable selection.
    BeginSelection(crate::MarkTarget),
    /// Arrow keys move the selection; Alt resizes it, Shift uses ten-pixel steps.
    AdjustSelection { direction: crate::SelectionDirection, resize: bool, coarse: bool },
    /// Test-read the keyboard selection, as a mark around what it was opened on.
    ConfirmSelection,
    /// Close marking without changing the previous test read or saved region.
    CancelSelection,
    /// Finish setup: keep the marked Title Region if its test read recognised a map (or,
    /// with none marked, the saved one if it still fits). Refused while its test read
    /// hasn't succeeded.
    FinishSetup,
    /// The player wants to redo setup (the re-setup hotkey).
    RequestSetup,
    /// The player started Minerva again while it runs; only one runs at a time, so
    /// this one shows itself: the panel, expanded, while there is a game window to show
    /// it on (once the game is back in front, if another app is now); setup otherwise.
    /// An open setup window stays as it is (the shell brings it in front).
    LaunchedAgain,
    /// The player closed setup without finishing it. On the first run that quits
    /// (`PanelState::quit`): there is nothing to read without a Title Region.
    CancelSetup,
    /// Start measuring afresh: the Session goes back to waiting for an EXP change. Until
    /// the EXP next changes, the reset can be undone.
    ResetSession,
    /// Bring back the Session the last reset ended, if the EXP hasn't changed since.
    UndoReset,
    /// Remove the shown map's Personal Best the Best row shows, this Visit's or one from its
    /// history: one that came from a bug. The map's next-best Visit takes over. There is no
    /// undo.
    RemoveBest,
    /// Open the Grind Tracker's window, or close it (the panel's button, or its hotkey).
    ToggleGrindTracker,
    /// Close the Grind Tracker's window (its ×). The Session carries on.
    CloseGrindTracker,
    /// The player dragged the Grind Tracker's window so its top-left corner is at
    /// (x, y), physical screen px.
    GrindTrackerMoved { x: i32, y: i32 },
    /// Open the Personal Bests window, or close it (the Grind Tracker's star, or its Best
    /// row's "Best"). It opens only while some map has a best.
    TogglePersonalBests,
    /// Close the Personal Bests window (its ×).
    ClosePersonalBests,
    /// The player dragged the Personal Bests window so its top-left corner is at (x, y),
    /// physical screen px.
    PersonalBestsMoved { x: i32, y: i32 },
    /// The player dragged the Item Card by its bar so its top-left corner is at (x, y),
    /// physical screen px.
    ItemCardMoved { x: i32, y: i32 },
    /// The first Market Line's price field opens on the Item Card: what a click on its one
    /// market price did before there were lines (0.8.0), kept so those Bug Reports read.
    /// The card now asks for a line's field by its ID ([`crate::Minerva::edit_card_market_price`]).
    EditCardMarket,
    /// The player pressed Esc in a field of the Item Card's market section, or Keep at the
    /// question before a line goes: it closes, everything as it was.
    CancelCardMarket,
    /// The player clicked the + on the Item Card's market heading: a new line's name
    /// field opens.
    AddCardMarketLine,
    /// The player clicked Remove beside an open line name field on the Item Card: the card
    /// asks first, saying how many of the line's prices go with it.
    AskRemoveCardMarketLine,
    /// The player confirmed removing the line the Item Card asked about: it goes, with its
    /// prices on every item.
    RemoveCardMarketLine,
    /// Open the search view on its mobs, with nothing typed yet (the search hotkey, or
    /// the 🔍 at the top of the panel). A collapsed panel expands to show it.
    OpenSearch,
    /// Move the search view's highlight up the suggestions (or the map list).
    SearchPrevious,
    /// Move the search view's highlight down the suggestions (or the map list).
    SearchNext,
    /// Enter in the search view: pick the highlighted mob and list its maps, or pick
    /// the highlighted map and make it the Hunting Target.
    SearchEnter,
    /// Esc or the back arrow in the search view, or focus moving to another window: back
    /// to the normal view, with nothing changed.
    SearchEscape,
    /// The Navigation section's ×: there is no Hunting Target any more.
    ClearHuntingTarget,
    /// The Navigation section's 🔍: open the search view on the Hunting Target mob's map
    /// list, to pick another of its maps.
    ReopenMapList,
    /// Turn taxis on or off for Distance (the Taxis checkbox in the map list).
    ToggleTaxis,
    /// Turn Teleport Scrolls on or off for Distance (the Teleport Scrolls checkbox in
    /// the map list).
    ToggleTeleportScrolls,
    /// A click on a section's heading: collapse the section to it, or open it again.
    ToggleSection(Section),
    /// Open the settings view (the gear at the top of the panel). A collapsed panel
    /// expands to show it, and an open search view closes: they take over the panel alike.
    OpenSettings,
    /// Esc or the back arrow in the settings view, or focus moving to another window: back
    /// to the normal view. A rebinding under way is cancelled.
    CloseSettings,
    /// A hotkey's Change button in the settings view: wait for the player to press its new
    /// combination ([`crate::Minerva::rebind_key`]). The hotkeys aren't live meanwhile.
    BeginRebind(crate::Hotkey),
    /// Stop waiting for a new combination, changing nothing.
    CancelRebind,
    /// The waiting row's Turn off: the hotkey has no combination any more, and its action
    /// stays on its button.
    TurnOffHotkey,
    /// Swap, after the player pressed another hotkey's combination: the hotkey being rebound
    /// takes it, and the other takes its old keys.
    SwapHotkeys,
    /// The settings view's Restore defaults: every hotkey back to its default, and on.
    ResetHotkeys,
    /// Undo, after Restore defaults: the hotkeys from before it are back.
    UndoResetHotkeys,
    /// Turn the Deal Finder on or off (its hotkey, or the panel's Deal Finder line).
    ToggleDealFinder,
    /// The settings view's Deal Margin: how far below the Deal Line, in percent, a price must
    /// be to be a deal; at most 99.
    SetDealMargin { percent: u32 },
    /// The settings view's Grind Tracker switch: on, the EXP figures are read from the Status
    /// Bar and the Session runs; off, neither, and the Grind Tracker doesn't show.
    SetGrindTracker { on: bool },
    /// The settings view's Refresh: fetch OSMS's file set and item files afresh now, for
    /// new maps, mobs and items OSMS has since they were fetched.
    RefreshGameData,
    /// A click on a Deal Badge: the card of the item its listing sells shows beside it.
    /// `row` counts the listings not sold out from the top, as [`crate::DealBadge::row`].
    OpenDealItem { row: u32 },
}

/// A section of the normal view that the player can collapse to its heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Section {
    /// The Map section: the Map Preview and the Hidden Portals (named as it was before it
    /// held the Map Preview, as the settings name it).
    HiddenPortals,
    Mobs,
    /// The Navigation section: the Hunting Target and the way there.
    Navigation,
}

/// Which sections are collapsed to their headings: all open until the player collapses
/// one, and the same on every map. Saved in the settings as `collapsedSections`; a section
/// a file doesn't name is open.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CollapsedSections {
    /// The Map section.
    pub hidden_portals: bool,
    pub mobs: bool,
    pub navigation: bool,
}

impl CollapsedSections {
    /// The same, with `section` collapsed if it was open and open if it was collapsed.
    pub(crate) fn toggled(self, section: Section) -> Self {
        match section {
            Section::HiddenPortals => Self { hidden_portals: !self.hidden_portals, ..self },
            Section::Mobs => Self { mobs: !self.mobs, ..self },
            Section::Navigation => Self { navigation: !self.navigation, ..self },
        }
    }
}

/// Where the player wants the panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacementMode {
    /// Over the game, at a spot relative to the game window.
    #[default]
    Inside,
    /// Outside the game window, against its right edge.
    DockedRight,
    /// Outside the game window, against its left edge.
    DockedLeft,
    /// A free window the player puts anywhere, e.g. on another monitor.
    Detached,
}

/// Whether the overlay window shows, on top of everything, or not at all. Wherever the
/// panel is placed (inside, docked or detached), it belongs to the game window: it shows
/// only while there is a game window and the game or one of Minerva's own windows
/// is in front ([`Foreground`]). The Grind Tracker's window and the Item Lookup alert
/// show only while the overlay can (their placements are `None` otherwise).
///
/// Until the shell first reports what is in front, the overlay counts as shown with the
/// game: no hidden flash at launch. (The hotkeys are the other way round, not live until
/// then: a combination registered by mistake would be lost to another app, while an
/// overlay shown by mistake goes as soon as the shell reports, within a tick.)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Layer {
    /// Above every other window on screen (always on top), while the game or one of
    /// ours is in front. When another app comes in front the overlay hides, and bringing
    /// the game back brings the overlay with it.
    OnTop,
    /// Nothing to show: the game is minimised or gone, or another app is in front. The
    /// overlay draws nothing and lets every click through.
    #[default]
    Hidden,
}

impl PlacementMode {
    fn next(self) -> Self {
        match self {
            Self::Inside => Self::DockedRight,
            Self::DockedRight => Self::DockedLeft,
            Self::DockedLeft => Self::Detached,
            Self::Detached => Self::Inside,
        }
    }
}

/// Where the inside panel sits: its top-right corner, relative to the game client's
/// top-left corner (physical px). The expanded panel and the collapsed icon share
/// this corner, so collapsing a panel on the right edge leaves the icon on that edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsideOffset {
    pub x: i32,
    pub y: i32,
}

/// A point in physical screen pixels (virtual-screen coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

/// Gap between the default panel and the game client's right edge (physical px).
const DEFAULT_RIGHT_MARGIN: i32 = 16;

/// The default panel's top sits a third of the client's height down.
const DEFAULT_TOP_FRACTION: i32 = 3;

/// Where a panel detached before it was ever placed goes: near the top-left of the
/// primary monitor, which always has its origin at (0, 0).
const DEFAULT_DETACHED_AT: ScreenPoint = ScreenPoint { x: 100, y: 100 };

/// The expanded panel's width until the player resizes it (CSS px, logical).
pub const DEFAULT_PANEL_WIDTH: u32 = 380;
/// The narrowest the player can make the panel (CSS px, logical): its header's buttons
/// and the Map Summary's figures still fit side by side.
pub const MIN_PANEL_WIDTH: u32 = 320;

#[derive(Debug, Clone)]
pub(crate) struct Panel {
    pub sizes: PanelSizes,
    /// The expanded panel's width as the player set it (CSS px, logical); the shell
    /// turns it into [`PanelSizes::expanded`] for the display.
    pub width: u32,
    pub game: Option<GameWindow>,
    pub expanded: bool,
    /// `None` until the player drags the panel: then it uses the default position.
    pub inside_offset: Option<InsideOffset>,
    pub mode: PlacementMode,
    /// The detached panel's top-left corner; `None` until it is first detached.
    pub detached_at: Option<ScreenPoint>,
    /// Where the Title Region is captured on screen, and where the Status Bar's Character
    /// and EXP figures were found. A panel inside the game is kept off them, or the capture
    /// would read the panel instead.
    pub avoid: Vec<ScreenRect>,
    /// The monitors' work areas, as last known with the game window: a detached panel
    /// stays within them after the game is gone.
    pub screens: Vec<ScreenRect>,
    /// Which window is in front, as the shell last reported it; `None` until it first
    /// does (see [`Layer`]).
    pub foreground: Option<Foreground>,
}

impl Default for Panel {
    fn default() -> Self {
        let zero = Size { width: 0, height: 0 };
        Self {
            sizes: PanelSizes { expanded: zero, collapsed: zero },
            width: DEFAULT_PANEL_WIDTH,
            game: None,
            expanded: true,
            inside_offset: None,
            mode: PlacementMode::Inside,
            detached_at: None,
            avoid: Vec::new(),
            screens: Vec::new(),
            foreground: None,
        }
    }
}

impl Panel {
    /// The overlay window was moved so its top-left corner is at (x, y): see
    /// [`Action::PanelMoved`].
    pub fn moved_to(&mut self, x: i32, y: i32) {
        match (self.mode, &self.game) {
            (PlacementMode::Detached, _) => self.detached_at = Some(ScreenPoint { x, y }),
            (PlacementMode::Inside, Some(game)) => {
                let right = x + self.size().width as i32;
                let client = game.client;
                self.inside_offset = Some(InsideOffset { x: right - client.x, y: y - client.y });
            }
            // Docked panels stay docked.
            _ => {}
        }
    }

    /// The player resized the panel to `width` (see [`Action::PanelResized`]): never
    /// narrower than [`MIN_PANEL_WIDTH`].
    pub fn resized_to(&mut self, width: u32) {
        self.width = width.max(MIN_PANEL_WIDTH);
    }

    pub fn cycle_placement(&mut self) {
        let was = self.placement();
        self.mode = self.mode.next();
        // Detaching for the first time leaves the panel where it was.
        if self.mode == PlacementMode::Detached && self.detached_at.is_none() {
            self.detached_at = was.map(|r| ScreenPoint { x: r.x, y: r.y });
        }
    }

    /// See [`Layer`]. The shell's window watcher (`keep_on_top_while_in_front`) restates
    /// this rule as a fast path while its main loop stalls: change both together.
    pub fn layer(&self) -> Layer {
        let in_front = self.foreground != Some(Foreground::OtherApp);
        if self.game.is_some() && in_front { Layer::OnTop } else { Layer::Hidden }
    }

    /// The player can drag the panel by its header: inside or detached, at any time; a
    /// docked or hidden one never.
    pub fn draggable(&self) -> bool {
        self.layer() == Layer::OnTop && matches!(self.mode, PlacementMode::Inside | PlacementMode::Detached)
    }

    fn size(&self) -> Size {
        if self.expanded { self.sizes.expanded } else { self.sizes.collapsed }
    }

    /// Where the overlay goes on screen. `None` while there is no game window, unless
    /// the panel is detached: a detached panel doesn't depend on the game. Never taller
    /// than the space where it goes: the game's client area inside, the monitor's work
    /// area below its top docked or detached.
    pub fn placement(&self) -> Option<ScreenRect> {
        if self.mode == PlacementMode::Detached {
            let at = self.detached_at.unwrap_or(DEFAULT_DETACHED_AT);
            let size = self.size();
            let panel = ScreenRect { x: at.x, y: at.y, width: size.width, height: size.height };
            // On the monitor its top-left corner is on (the last ones known, as it
            // doesn't depend on the game).
            let on = self.screens.iter().copied().find(|screen| holds(*screen, at));
            return Some(on.map_or(panel, |screen| below_top(panel, screen)));
        }
        let game = self.game.as_ref()?;
        match self.mode {
            PlacementMode::DockedRight | PlacementMode::DockedLeft => {
                let docked = self.docked(game);
                // Never off-screen: without room beside the game, it goes inside.
                let fits = game.screens.iter().find_map(|&screen| {
                    let docked = below_top(docked, screen);
                    (docked.height > 0 && contains(screen, docked)).then_some(docked)
                });
                Some(fits.unwrap_or_else(|| self.inside(game.client)))
            }
            _ => Some(self.inside(game.client)),
        }
    }

    /// Against the outside of the game window's frame, level with the client's top.
    fn docked(&self, game: &GameWindow) -> ScreenRect {
        let size = self.size();
        let x = if self.mode == PlacementMode::DockedLeft {
            game.frame.x - size.width as i32
        } else {
            game.frame.x + game.frame.width as i32
        };
        ScreenRect { x, y: game.client.y, width: size.width, height: size.height }
    }

    /// Inside the game's client area, no taller than it, and clear of the captured
    /// regions: beside them at full height if there is room, or else in the tallest of
    /// the spaces above, between and below them.
    fn inside(&self, game: ScreenRect) -> ScreenRect {
        let height = self.size().height.min(game.height);
        let panel = clear_of(self.inside_at(game, height), &self.avoid, game);
        if clear(panel, &self.avoid) {
            return panel;
        }
        let height = height.min(tallest_gap(&self.avoid, game));
        clear_of(self.inside_at(game, height), &self.avoid, game)
    }

    /// The panel `height` px tall at its spot inside the game's client area: where the
    /// player dragged it, or the default one.
    fn inside_at(&self, game: ScreenRect, height: u32) -> ScreenRect {
        let size = Size { height, ..self.size() };
        let (w, h) = (size.width as i32, size.height as i32);
        let (x, y) = match self.inside_offset {
            Some(offset) => (game.x + offset.x - w, game.y + offset.y),
            // On the right edge, its top a third of the way down: clear of the minimap
            // (top-left) and the in-game Quest Helper (top-right). The top stays put as
            // the panel grows and shrinks with its content, so the map name doesn't move.
            None => (
                game.x + game.width as i32 - DEFAULT_RIGHT_MARGIN - w,
                game.y + game.height as i32 / DEFAULT_TOP_FRACTION,
            ),
        };
        // Keep the whole panel inside the client (its top-left wins if it can't fit).
        let x = x.min(game.x + game.width as i32 - w).max(game.x);
        let y = y.min(game.y + game.height as i32 - h).max(game.y);
        ScreenRect { x, y, width: size.width, height: size.height }
    }
}

/// `rect`, cut short at the bottom of `screen` if it reaches past it.
fn below_top(rect: ScreenRect, screen: ScreenRect) -> ScreenRect {
    let room = (bottom(screen) - rect.y as i64).max(0);
    ScreenRect { height: rect.height.min(room as u32), ..rect }
}

fn bottom(r: ScreenRect) -> i64 {
    r.y as i64 + r.height as i64
}

/// Whether pixel `at` lies inside `rect`.
pub(crate) fn holds(rect: ScreenRect, at: ScreenPoint) -> bool {
    let (x, y) = (at.x as i64, at.y as i64);
    (rect.x as i64) <= x && x < rect.x as i64 + rect.width as i64 && (rect.y as i64) <= y && y < bottom(rect)
}

/// The tallest stretch of `within`, top to bottom, that none of `avoid` reaches into:
/// above the first, between two, or below the last.
fn tallest_gap(avoid: &[ScreenRect], within: ScreenRect) -> u32 {
    let mut spans: Vec<(i64, i64)> = avoid.iter().map(|r| (r.y as i64, bottom(*r))).collect();
    spans.sort();
    let (mut from, mut tallest) = (within.y as i64, 0);
    for (top, end) in spans {
        tallest = tallest.max(top - from);
        from = from.max(end);
    }
    tallest.max(bottom(within) - from).max(0) as u32
}

/// `panel`, moved the shortest way (below, above, right or left of one of the `avoid`
/// rectangles) so it overlaps none of them while staying inside `within`. Left where it
/// is when it overlaps none, or when no such spot fits.
pub(crate) fn clear_of(panel: ScreenRect, avoid: &[ScreenRect], within: ScreenRect) -> ScreenRect {
    if clear(panel, avoid) {
        return panel;
    }
    let (w, h) = (panel.width as i32, panel.height as i32);
    avoid
        .iter()
        .flat_map(|r| {
            [
                ScreenRect { y: r.y + r.height as i32, ..panel },
                ScreenRect { y: r.y - h, ..panel },
                ScreenRect { x: r.x + r.width as i32, ..panel },
                ScreenRect { x: r.x - w, ..panel },
            ]
        })
        .filter(|spot| contains(within, *spot) && clear(*spot, avoid))
        .min_by_key(|spot| (spot.x - panel.x).abs() + (spot.y - panel.y).abs())
        .unwrap_or(panel)
}

/// Our window at `spot` (the Grind Tracker's or the Personal Bests window), kept wholly on
/// a monitor and moved the shortest way off any of `avoid`: within the game's client area
/// if it lies inside it, else within its monitor.
pub(crate) fn in_view(game: &GameWindow, spot: ScreenRect, avoid: &[ScreenRect]) -> ScreenRect {
    let (w, h) = (spot.width as i32, spot.height as i32);
    let screen = screen_at(game, spot.x, spot.y);
    let spot = ScreenRect {
        x: spot.x.min(screen.x + screen.width as i32 - w).max(screen.x),
        y: spot.y.min(screen.y + screen.height as i32 - h).max(screen.y),
        ..spot
    };
    let within = if contains(game.client, spot) { game.client } else { screen };
    clear_of(spot, avoid, within)
}

/// The monitor's work area holding point (x, y), or else the one holding the game's
/// client area's top-left corner, or else the client area itself.
pub(crate) fn screen_at(game: &GameWindow, x: i32, y: i32) -> ScreenRect {
    let holding = |x, y| game.screens.iter().copied().find(|screen| holds(*screen, ScreenPoint { x, y }));
    holding(x, y).or_else(|| holding(game.client.x, game.client.y)).unwrap_or(game.client)
}

/// Whether `panel` overlaps none of `avoid`.
fn clear(panel: ScreenRect, avoid: &[ScreenRect]) -> bool {
    avoid.iter().all(|r| !overlaps(panel, *r))
}

fn overlaps(a: ScreenRect, b: ScreenRect) -> bool {
    let right = |r: ScreenRect| r.x as i64 + r.width as i64;
    let bottom = |r: ScreenRect| r.y as i64 + r.height as i64;
    (a.x as i64) < right(b) && (b.x as i64) < right(a) && (a.y as i64) < bottom(b) && (b.y as i64) < bottom(a)
}

/// Whether `inner` lies wholly inside `outer`.
pub(crate) fn contains(outer: ScreenRect, inner: ScreenRect) -> bool {
    let right = |r: ScreenRect| r.x as i64 + r.width as i64;
    let bottom = |r: ScreenRect| r.y as i64 + r.height as i64;
    inner.x >= outer.x && inner.y >= outer.y && right(inner) <= right(outer) && bottom(inner) <= bottom(outer)
}
