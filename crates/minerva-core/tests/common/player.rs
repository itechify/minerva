//! The player hunting with EXP tracking on, for the Personal Best: the EXP bar read every
//! 10 s, the panel confirming maps, the Grind Tracker's buttons pressed, and, if a test
//! asks, the shell's saves of the bests followed after each.

use super::S;
use minerva_core::{
    ExpBar, ExpBarReading, Minerva, PanelControl, PanelState, SavedBests, ScreenRect, StatusBar, StatusBarReading, VisitBest,
};

/// The wall clock at capture time 0 when the shell tells it: 9 Oct 2026, 12:00 UTC.
pub const WALL_AT_START: u64 = 1_791_547_200_000;

/// The percentage the game shows for `exp` on a 20,000 EXP level, rounded to hundredths.
fn percent(exp: u64) -> u32 {
    super::percent(exp, 20_000)
}

/// The player hunting on a 20,000 EXP level, the EXP bar read every 10 s. Readings at
/// whole half minutes of capture time are where the Grind Tracker's figures refresh.
pub struct Player {
    pub minerva: Minerva,
    pub exp: u64,
    pub at: u64,
    /// The wall clock at capture time 0, if the shell tells it.
    wall_at_start: Option<u64>,
    /// The shell's saves of the Personal Bests, if the test follows them.
    shell: Option<Shell>,
    /// What the Status Bar shows with each reading, once the test says: a Magician's name
    /// (`None`: unreadable) and level.
    status: Option<(Option<String>, u32)>,
}

impl Player {
    /// The bar read at 1,000 [5.00%] at 0 s, then 10 EXP more every 10 s on a map the
    /// panel doesn't know, up to 30 s: the Session has started.
    pub fn new(minerva: Minerva) -> Self {
        Self::start(minerva, None, None)
    }

    /// As [`Player::new`], with the shell telling the wall clock before every reading, as
    /// it does: [`WALL_AT_START`] at capture time 0, running with it.
    pub fn clocked(minerva: Minerva) -> Self {
        Self::clocked_at(minerva, WALL_AT_START)
    }

    /// As [`Player::clocked`], the wall clock at `wall_at_start` at capture time 0: a
    /// later launch.
    pub fn clocked_at(minerva: Minerva, wall_at_start: u64) -> Self {
        Self::start(minerva, Some(wall_at_start), None)
    }

    /// As [`Player::clocked`], following the shell's saves of the Personal Bests from
    /// the start.
    pub fn saved_by_the_shell(minerva: Minerva) -> Self {
        let shell = Shell::new(&minerva);
        Self::start(minerva, Some(WALL_AT_START), Some(shell))
    }

    fn start(minerva: Minerva, wall_at_start: Option<u64>, shell: Option<Shell>) -> Self {
        let mut player = Player { minerva, exp: 1000, at: 0, wall_at_start, shell, status: None };
        player.read();
        player.hunt(30 * S, 10);
        player
    }

    /// The wall clock now, as the shell last told it.
    pub fn wall_clock(&self) -> Option<u64> {
        self.wall_at_start.map(|start| start + self.at)
    }

    /// How many times the shell has saved the bests.
    pub fn saves(&self) -> u32 {
        self.shell.as_ref().map_or(0, |shell| shell.saves)
    }

    /// The shell's loop, after an input: the panel state then.
    fn shell_loop(&mut self, input: &str) -> PanelState {
        if let Some(shell) = &mut self.shell {
            shell.after(&self.minerva, &format!("{input} at {} ms", self.at));
        }
        self.minerva.panel_state().clone()
    }

    pub fn read(&mut self) -> PanelState {
        if let Some(now) = self.wall_clock() {
            self.minerva.on_wall_clock(now);
        }
        if let Some((name, level)) = &self.status {
            let area = ScreenRect { x: 334, y: 1145, width: 331, height: 55 };
            let bar = StatusBar { area, level: Some(*level), job: Some("Magician".to_string()), name: name.clone(), exp_figures: None };
            // Captured just before the EXP bar, so the refresh at a half minute takes in both.
            let at_ms = self.at.saturating_sub(500);
            self.minerva.on_status_bar_reading(StatusBarReading { bar: Some(bar), at_ms });
        }
        let bar = Some(ExpBar { exp: self.exp, percent_hundredths: percent(self.exp) });
        self.minerva.on_exp_bar_reading(ExpBarReading { bar, at_ms: self.at, place: None });
        self.shell_loop("a reading")
    }

    /// The panel confirms the map of a sample: a Visit starts now.
    pub fn arrive(&mut self, sample: &str) -> PanelState {
        super::arrive(&mut self.minerva, sample, self.at);
        self.shell_loop(sample)
    }

    /// Hunt for `ms` more, `gain` EXP every 10 s (`gain` × 360 EXP/h): the panel state at
    /// the last reading.
    pub fn hunt(&mut self, ms: u64, gain: u64) -> PanelState {
        let mut state = self.minerva.panel_state().clone();
        for _ in 0..ms / (10 * S) {
            self.at += 10 * S;
            self.exp += gain;
            state = self.read();
        }
        state
    }

    /// The bar doesn't read whole for `ms` more, a frame every 10 s: the panel state at
    /// the last.
    pub fn unread(&mut self, ms: u64) -> PanelState {
        let mut state = self.minerva.panel_state().clone();
        for _ in 0..ms / (10 * S) {
            self.at += 10 * S;
            self.minerva.on_exp_bar_reading(ExpBarReading { bar: None, at_ms: self.at, place: None });
            state = self.shell_loop("the bar unread");
        }
        state
    }

    /// Choose the next of the Candidates with the hotkey.
    pub fn choose_next(&mut self) -> PanelState {
        self.minerva.on_hotkey("ctrl+shift+n");
        self.shell_loop("choosing the next Candidate")
    }

    /// Press a control on the panel or the Grind Tracker.
    pub fn press(&mut self, control: PanelControl) -> PanelState {
        self.minerva.press(control);
        self.shell_loop(&format!("{control:?}"))
    }

    /// Remove the Unassigned Visit whose best was `visit` from `map`'s, in the Personal Bests
    /// window.
    pub fn remove_visit(&mut self, map: &str, visit: VisitBest) -> PanelState {
        self.remove_visit_of(None, map, visit)
    }

    /// Remove the Visit whose best was `visit` from `character`'s history of `map` (the
    /// Unassigned one for `None`), in the Personal Bests window.
    pub fn remove_visit_of(&mut self, character: Option<&str>, map: &str, visit: VisitBest) -> PanelState {
        self.minerva.remove_visit(character, map, &visit);
        self.shell_loop(&format!("removing {visit:?} from {character:?}'s {map}"))
    }

    /// From the next reading on, the Status Bar shows a Magician named `name` (`None`: the
    /// name unreadable) at `level`, with each EXP bar reading.
    pub fn status(&mut self, name: Option<&str>, level: u32) {
        self.status = Some((name.map(String::from), level));
    }

    /// Move the Visit whose best was `visit` on `map`, of the Character named `from` (`None`:
    /// an Unassigned one), to the Character named `to`, in the Personal Bests window.
    pub fn move_visit(&mut self, from: Option<&str>, map: &str, visit: VisitBest, to: &str) -> PanelState {
        self.minerva.move_visit(from, map, &visit, to);
        self.shell_loop(&format!("moving {visit:?} on {map} from {from:?} to {to}"))
    }

    /// Give the Character named `to` every Unassigned Visit, in the Personal Bests window.
    pub fn assign_unassigned(&mut self, to: &str) -> PanelState {
        self.minerva.assign_unassigned(to);
        self.shell_loop(&format!("assigning every Unassigned Visit to {to}"))
    }

    /// Remove the Character named `name`, in the Personal Bests window.
    pub fn remove_character(&mut self, name: &str) -> PanelState {
        self.minerva.remove_character(name);
        self.shell_loop(&format!("removing {name}"))
    }

    /// Merge the Character named `from` into the one named `into`, in the Personal Bests
    /// window.
    pub fn merge_characters(&mut self, from: &str, into: &str) -> PanelState {
        self.minerva.merge_characters(from, into);
        self.shell_loop(&format!("merging {from} into {into}"))
    }
}

/// The shell's saves of the Personal Bests, as its loop makes them: whenever the bests'
/// revision has changed since the last.
struct Shell {
    revision: u64,
    saved: SavedBests,
    saves: u32,
}

impl Shell {
    /// Just after startup, with what was restored.
    fn new(minerva: &Minerva) -> Self {
        Self { revision: minerva.bests_revision(), saved: minerva.bests(), saves: 0 }
    }

    /// The shell's loop after `input`: it saves if the revision changed, which it must
    /// exactly when the bests did.
    fn after(&mut self, minerva: &Minerva, input: &str) {
        let (revision, bests) = (minerva.bests_revision(), minerva.bests());
        let changed = revision != self.revision;
        assert_eq!(changed, bests != self.saved, "{input}: saved {:?}, now {bests:?}", self.saved);
        if changed {
            *self = Self { revision, saved: bests, saves: self.saves + 1 };
        }
    }
}
