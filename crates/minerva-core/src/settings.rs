//! What Minerva remembers between sessions. Plain data: the shell stores it as
//! JSON in the app's config directory and hands it back to the core at startup.

use crate::grind_tracker::ClientOffset;
use crate::leaderboards::LeaderboardCharacter;
use crate::panel::{CollapsedSections, InsideOffset, PlacementMode, ScreenPoint};
use crate::world::Travel;
use serde::{Deserialize, Serialize};

/// The settings version this core writes. Version 1 had other Ctrl+Shift letters, and
/// versions 2 and 3 had Win+Alt hotkeys (3 was written by the EXP tracking branch);
/// [`crate::Minerva::restore`] replaces all of them with the defaults. Version 4's Item
/// Lookup key is the Item Card's from version 5 on ([`Hotkeys::give_item_lookup_to_card`]).
/// Version 6 brought the Deal Finder's hotkey, off for an older file whose hotkeys have its
/// default ([`Hotkeys::free_deal_finder`]). The EXP Bar Region files from before ADR 0017
/// hold (`expBarRegion`) is ignored: the Grind Tracker is on for every file without
/// `grindTrackerOn`. Version 7 brought [`Settings::map_leaderboards_met`], which an older
/// file doesn't have, so none of the Characters it knew starts shared. Version 8 has no Title
/// Region (ADR 0021): a file with one (`titleRegion`) has finished setup, and the region is
/// dropped, as the Minimap Title is found wherever it is.
pub(crate) const VERSION: u32 = 8;

/// The player's settings. Missing fields (an older or empty file) take their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Bumped when the shape changes incompatibly.
    pub version: u32,
    /// Expanded to the full panel, or collapsed to its icon.
    pub expanded: bool,
    /// Where the player dragged the inside panel; `None` for the default position.
    pub inside_offset: Option<InsideOffset>,
    /// Inside, docked right or left, or detached. (Version 1 called it `placement`.)
    #[serde(alias = "placement")]
    pub placement_mode: PlacementMode,
    /// Where the player put the detached panel (its top-left corner, physical screen
    /// px); `None` if it was never detached.
    pub detached_at: Option<ScreenPoint>,
    /// The expanded panel's width (CSS px, logical) as the player set it; `None` for the
    /// default width.
    pub panel_width: Option<u32>,
    pub hotkeys: Hotkeys,
    /// Setup has been finished. A file from before version 8 has finished it if it has a Title
    /// Region (`titleRegion`), which is dropped.
    #[serde(alias = "titleRegion", deserialize_with = "done_or_marked")]
    pub setup_done: bool,
    /// The Grind Tracker is on (Settings' switch): the EXP figures are read from the Status
    /// Bar, and the Session runs. Files from before it have it on.
    pub grind_tracker_on: bool,
    /// The title of the game window the player picked by hand; `None` to look for the
    /// "MapleStory" window.
    pub game_window_title: Option<String>,
    /// What Distance counts besides portals and ships (the map list's
    /// checkboxes). Saved as top-level `taxis` and `teleportScrolls`.
    #[serde(flatten)]
    pub travel: Travel,
    /// The sections the player collapsed to their headings; files from before this
    /// have both open.
    pub collapsed_sections: CollapsedSections,
    /// The player wants the Grind Tracker's window open; files from before it had a
    /// window have it open.
    pub grind_tracker_open: bool,
    /// Where the player dragged the Grind Tracker's window, relative to the game client;
    /// `None` for just below the panel.
    pub grind_tracker_at: Option<ClientOffset>,
    /// Where the player dragged the Personal Bests window, relative to the game client;
    /// `None` for centred over the game.
    pub personal_bests_at: Option<ClientOffset>,
    /// Where the player dragged the Settings window, relative to the game client; `None` for
    /// beside the panel.
    pub settings_at: Option<ClientOffset>,
    /// The Deal Margin (ADR 0014): how far below the Deal Line, in percent, a listing's
    /// price must be for the Deal Finder to call it a deal. 0: any price below it.
    pub deal_margin_percent: u32,
    /// The Characters the player shows on the Map Leaderboards (#110), and those taken off
    /// whose entries the site hasn't yet deleted. Not the Leaderboard Key: the shell keeps it
    /// apart, since everything here goes into a Bug Report.
    pub map_leaderboards: Vec<LeaderboardCharacter>,
    /// Every Character met with a best that can go on the Map Leaderboards: one met for the
    /// first time starts shared there. `None` in a file from before version 7, until the bests
    /// read at a launch: everyone in them counts as met then, so none of them starts shared.
    #[serde(default)]
    pub map_leaderboards_met: Option<Vec<String>>,
    /// "Check for updates" (Settings' switch): Minerva checks for a newer version by itself,
    /// at launch and every 6 hours (ADR 0019). Files from before it have it on.
    pub check_for_updates: bool,
    /// The player said not now to the Personal Bests window's prompt to link Discord (#152):
    /// it doesn't ask again.
    pub link_prompt_dismissed: bool,
    /// The Grind Tracker's Loot section is open, showing the mesos and Items picked up; it starts
    /// collapsed (#171).
    pub loot_open: bool,
    /// "Market prices from maplestory.market" (Settings' switch, ADR 0030): Item Cards show its
    /// Going rate and Cheapest now, and the Deal Finder may judge against them. Off, Minerva
    /// asks for none. Files from before it have it on.
    pub market_prices: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            expanded: true,
            inside_offset: None,
            placement_mode: PlacementMode::Inside,
            detached_at: None,
            panel_width: None,
            hotkeys: Hotkeys::default(),
            setup_done: false,
            grind_tracker_on: true,
            game_window_title: None,
            travel: Travel::default(),
            collapsed_sections: CollapsedSections::default(),
            grind_tracker_open: true,
            grind_tracker_at: None,
            personal_bests_at: None,
            settings_at: None,
            deal_margin_percent: 0,
            map_leaderboards: Vec::new(),
            map_leaderboards_met: Some(Vec::new()),
            check_for_updates: true,
            link_prompt_dismissed: false,
            loot_open: false,
            market_prices: true,
        }
    }
}

/// Global hotkeys, as accelerator strings such as `"ctrl+shift+c"`; an empty one is off.
///
/// They fire while the game has focus and the game never sees them. The defaults use
/// Ctrl+Shift+letter: Windows and other apps already hold many Win+Alt letters
/// (Win+Alt+F among them), so a player who binds one of these in the game rebinds it
/// here instead, or turns it off. Files saved before edit mode was retired also have `editMode`, and
/// files saved before the Grind Tracker had a window have `resetSession` (Ctrl+Shift+R,
/// too easily pressed while attacking with Ctrl); both are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    /// Collapse the panel to its icon, or expand it again.
    pub toggle_panel: String,
    /// Switch placement: inside, docked right, docked left, detached, and round again.
    pub cycle_placement: String,
    /// Open the shown map's MeowDB page in the browser.
    pub open_meowdb_page: String,
    /// Open setup again.
    pub setup: String,
    /// Show the next of the maps sharing the shown map's title, when the core couldn't
    /// tell which one the player is on.
    pub choose_candidate: String,
    /// Open the search view.
    pub search: String,
    /// Open the Grind Tracker's window, or close it.
    pub grind_tracker: String,
    /// The Item Card: show what the game's Item Tooltip by the pointer doesn't say about
    /// its item. Files from before it get the default.
    pub item_card: String,
    /// Item Lookup: open the MeowDB page of the item whose Item Tooltip shows by the
    /// pointer. Files from before it get the default.
    pub item_lookup: String,
    /// Turn the Deal Finder on or off. Files from before it get the default, unless
    /// another hotkey has it ([`Hotkeys::free_deal_finder`]).
    pub deal_finder: String,
}

impl Hotkeys {
    /// A version 4 file's hotkeys, from before the Item Card: the key the player pressed
    /// over an item (Item Lookup's) shows its card now, and Item Lookup, which opens the
    /// MeowDB page, gets its default, Ctrl+Shift+U, unless another hotkey has it: then it
    /// is off. A file without Item Lookup's key has the defaults for both.
    pub(crate) fn give_item_lookup_to_card(&mut self) {
        let lookup_default = Hotkeys::default().item_lookup;
        if self.item_lookup != lookup_default {
            self.item_card = std::mem::replace(&mut self.item_lookup, lookup_default);
        }
        self.free(Hotkey::ItemLookup);
    }

    /// A file's hotkeys from before the Deal Finder (version 5 and earlier), which gave it
    /// its default: off, if another hotkey has that.
    pub(crate) fn free_deal_finder(&mut self) {
        self.free(Hotkey::DealFinder);
    }

    /// Turn `hotkey` off if another hotkey has its combination.
    fn free(&mut self, hotkey: Hotkey) {
        let taken = Hotkey::ALL
            .into_iter()
            .filter(|&other| other != hotkey)
            .any(|other| crate::same_hotkey(self.get(other), self.get(hotkey)));
        if taken {
            self.set(hotkey, String::new());
        }
    }

    /// Every hotkey that is on, for the shell to register: one list, so none is forgotten.
    pub fn all(&self) -> Vec<&str> {
        Hotkey::ALL.into_iter().map(|hotkey| self.get(hotkey)).filter(|combination| !combination.is_empty()).collect()
    }

    /// The combination bound to `hotkey`; empty while it is off.
    pub fn get(&self, hotkey: Hotkey) -> &str {
        match hotkey {
            Hotkey::TogglePanel => &self.toggle_panel,
            Hotkey::CyclePlacement => &self.cycle_placement,
            Hotkey::OpenMeowdbPage => &self.open_meowdb_page,
            Hotkey::Setup => &self.setup,
            Hotkey::ChooseCandidate => &self.choose_candidate,
            Hotkey::GrindTracker => &self.grind_tracker,
            Hotkey::ItemCard => &self.item_card,
            Hotkey::ItemLookup => &self.item_lookup,
            Hotkey::Search => &self.search,
            Hotkey::DealFinder => &self.deal_finder,
        }
    }

    /// Bind `hotkey` to `combination` instead; an empty one turns it off.
    pub(crate) fn set(&mut self, hotkey: Hotkey, combination: String) {
        let bound = match hotkey {
            Hotkey::TogglePanel => &mut self.toggle_panel,
            Hotkey::CyclePlacement => &mut self.cycle_placement,
            Hotkey::OpenMeowdbPage => &mut self.open_meowdb_page,
            Hotkey::Setup => &mut self.setup,
            Hotkey::ChooseCandidate => &mut self.choose_candidate,
            Hotkey::GrindTracker => &mut self.grind_tracker,
            Hotkey::ItemCard => &mut self.item_card,
            Hotkey::ItemLookup => &mut self.item_lookup,
            Hotkey::Search => &mut self.search,
            Hotkey::DealFinder => &mut self.deal_finder,
        };
        *bound = combination;
    }
}

/// One of the player's hotkeys, by what it does: named as in [`Hotkeys`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Hotkey {
    TogglePanel,
    CyclePlacement,
    OpenMeowdbPage,
    Setup,
    ChooseCandidate,
    GrindTracker,
    ItemCard,
    ItemLookup,
    Search,
    DealFinder,
}

impl Hotkey {
    /// Every hotkey, in the order [`Hotkeys::all`] lists them.
    pub const ALL: [Hotkey; 10] = [
        Hotkey::TogglePanel,
        Hotkey::CyclePlacement,
        Hotkey::OpenMeowdbPage,
        Hotkey::Setup,
        Hotkey::ChooseCandidate,
        Hotkey::GrindTracker,
        Hotkey::ItemCard,
        Hotkey::ItemLookup,
        Hotkey::Search,
        Hotkey::DealFinder,
    ];
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle_panel: "ctrl+shift+c".into(),
            cycle_placement: "ctrl+shift+p".into(),
            open_meowdb_page: "ctrl+shift+m".into(),
            setup: "ctrl+shift+s".into(),
            choose_candidate: "ctrl+shift+n".into(),
            search: "ctrl+shift+f".into(),
            grind_tracker: "ctrl+shift+g".into(),
            item_card: "ctrl+shift+i".into(),
            item_lookup: "ctrl+shift+u".into(),
            deal_finder: "ctrl+shift+d".into(),
        }
    }
}

/// `setupDone`, or a Title Region from before version 8 (ADR 0021): setup was finished if one
/// was kept.
fn done_or_marked<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DoneOrMarked {
        Done(bool),
        Marked(Option<serde::de::IgnoredAny>),
    }
    Ok(match DoneOrMarked::deserialize(deserializer)? {
        DoneOrMarked::Done(done) => done,
        DoneOrMarked::Marked(marked) => marked.is_some(),
    })
}
