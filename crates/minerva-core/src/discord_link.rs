//! Link Discord (#152, ADR 0023): the Personal Bests window links this install's Leaderboard
//! Key to the player's Account on the site, so a name their Account holds may be written from
//! any PC they linked. The shell does the HTTP with the key it keeps (never the core), and
//! tells the core each answer.
//!
//! - **The link state** (linked, as whom, or not) is read when the Personal Bests window opens,
//!   at most once a minute, and every [`POLL_MS`] for [`LINKING_MS`] after the link page opened.
//!   Until a read answers, nothing is offered: a linked player never sees Link Discord flash by.
//! - **Link Discord** asks the shell to start a link: it asks the site for a one-time token
//!   with the key and opens the link page itself, so the token is never the core's and never
//!   in a Bug Report. The player confirms on the site; the core reads the state until it says
//!   linked, or the time is up.
//! - **The prompt**: once, quietly, to a player with a Character on the Map Leaderboards who
//!   hasn't linked, until they link or say not now (kept with the settings).
//! - **Linked**, a Character whose name another key held goes up again at once: the name may
//!   be theirs through the Account now.

use serde::{Deserialize, Serialize};

/// How often the link state is read while the player may be linking in the browser.
const POLL_MS: u64 = 5_000;
/// How long after the link page opened the link state is read for: the token lasts 5 minutes,
/// but a player who links does so within moments, and opening Personal Bests reads it again.
const LINKING_MS: u64 = 2 * 60_000;
/// How soon an opening of the Personal Bests window reads the link state again.
const REREAD_MS: u64 = 60_000;

/// What the shell is to ask the site, with the Leaderboard Key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscordLinkAsk {
    /// Start a link: ask for a one-time token (`POST`) and open the link page with it,
    /// answering [`DiscordLinkAnswer::Opened`] or [`DiscordLinkAnswer::Failed`].
    Start,
    /// Read the link state (`GET`): [`DiscordLinkAnswer::Linked`], `NotLinked` or `Failed`.
    State,
}

/// A request to the site about linking, from when the core asks for it until the shell tells
/// it the answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordLinkRequest {
    /// Which request it is: one more with each the core asks for. The answer names it.
    pub id: u64,
    pub ask: DiscordLinkAsk,
}

impl DiscordLinkRequest {
    /// Where on the site it goes, from [`crate::SITE_URL`].
    pub fn path(&self) -> &'static str {
        "api/link"
    }

    /// Whether it is a `POST` (starting a link); otherwise a `GET`.
    pub fn posts(&self) -> bool {
        self.ask == DiscordLinkAsk::Start
    }
}

/// What a request about linking came to, as the shell tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DiscordLinkAnswer {
    /// The link page opened in the browser, with a token for this install's key.
    Opened {},
    /// The key is linked to the Account that signs in with this Discord username.
    Linked { username: String },
    /// The key is linked to no Account.
    NotLinked {},
    /// The site couldn't be reached, answered otherwise, or the page couldn't be opened: why,
    /// in words.
    Failed { why: String },
}

/// What the Personal Bests window says of linking.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordLinkView {
    /// Linked to the Account that signs in with this Discord username; `None` while not, or
    /// not known.
    pub linked_as: Option<String>,
    /// Link Discord is offered: the link state read as not linked. Beside the Share, on the
    /// note of a name another key holds, and of bests that didn't read.
    pub offered: bool,
    /// The link page is opening, or open, and the core reads whether it linked.
    pub linking: bool,
    /// The link page couldn't be opened, as Link Discord last tried: why.
    pub failed: Option<String>,
    /// The prompt shows: offered, to a player with a Character on the Map Leaderboards, who
    /// hasn't said not now.
    pub prompt: bool,
}

/// The link state, as the site last said it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Known {
    #[default]
    Unknown,
    NotLinked,
    Linked(String),
}

/// Linking this install's key to an Account, this launch.
#[derive(Debug, Default)]
pub(crate) struct DiscordLink {
    known: Known,
    /// When the link state was last asked for.
    read_at_ms: Option<u64>,
    /// The link page opened: the link state is read until then.
    linking_until_ms: Option<u64>,
    /// Link Discord was pressed: a start goes at the next tick.
    start_due: bool,
    failed: Option<String>,
    in_flight: Option<DiscordLinkRequest>,
    /// The Personal Bests window was open at the last tick.
    window_open: bool,
    last_id: u64,
    now_ms: u64,
}

impl DiscordLink {
    /// The player pressed Link Discord.
    pub fn start(&mut self) {
        self.start_due = true;
        self.failed = None;
    }

    /// Time passed, to `at_ms`, the Personal Bests window open or not: ask for what is due.
    pub fn tick(&mut self, at_ms: u64, window_open: bool) {
        self.now_ms = self.now_ms.max(at_ms);
        let opened = window_open && !self.window_open;
        self.window_open = window_open;
        if self.linking_until_ms.is_some_and(|until| self.now_ms >= until) {
            self.linking_until_ms = None;
        }
        if self.in_flight.is_some() {
            return;
        }
        let since_read = |gap: u64| self.read_at_ms.is_none_or(|at| self.now_ms >= at + gap);
        // While linking, every poll's length; otherwise at an opening, unknown or a minute on.
        let poll = self.linking_until_ms.is_some() && since_read(POLL_MS);
        let reread = opened && (self.known == Known::Unknown || since_read(REREAD_MS));
        let ask = if self.start_due {
            self.start_due = false;
            Some(DiscordLinkAsk::Start)
        } else if poll || reread {
            Some(DiscordLinkAsk::State)
        } else {
            None
        };
        if let Some(ask) = ask {
            if ask == DiscordLinkAsk::State {
                self.read_at_ms = Some(self.now_ms);
            }
            self.last_id += 1;
            self.in_flight = Some(DiscordLinkRequest { id: self.last_id, ask });
        }
    }

    /// The shell's answer to request `id`: whether it says the key has just been linked.
    pub fn answer(&mut self, id: u64, answer: DiscordLinkAnswer) -> bool {
        let Some(request) = self.in_flight.take_if(|request| request.id == id) else { return false };
        match (request.ask, answer) {
            (DiscordLinkAsk::Start, DiscordLinkAnswer::Opened {}) => {
                self.linking_until_ms = Some(self.now_ms + LINKING_MS);
                // The first read a poll's length after it opened, not at once.
                self.read_at_ms = Some(self.now_ms);
                false
            }
            (DiscordLinkAsk::Start, DiscordLinkAnswer::Failed { why }) => {
                self.failed = Some(why);
                false
            }
            (DiscordLinkAsk::State, DiscordLinkAnswer::Linked { username }) => {
                let newly = !matches!(self.known, Known::Linked(_));
                self.known = Known::Linked(username);
                self.linking_until_ms = None;
                newly
            }
            (DiscordLinkAsk::State, DiscordLinkAnswer::NotLinked {}) => {
                self.known = Known::NotLinked;
                false
            }
            // Failed, or an answer that doesn't fit the request: what was known stands.
            (DiscordLinkAsk::Start, _) => {
                self.failed = Some("an answer that doesn't fit".to_string());
                false
            }
            (DiscordLinkAsk::State, _) => false,
        }
    }

    /// The request under way, for the shell to make.
    pub fn requests(&self) -> Vec<DiscordLinkRequest> {
        self.in_flight.iter().cloned().collect()
    }

    /// What the Personal Bests window says, with `any_shown` a Character on the Map
    /// Leaderboards and `prompt_dismissed` the player having said not now.
    pub fn view(&self, any_shown: bool, prompt_dismissed: bool) -> DiscordLinkView {
        let starting = self.start_due || self.in_flight.as_ref().is_some_and(|request| request.ask == DiscordLinkAsk::Start);
        let linking = starting || self.linking_until_ms.is_some();
        let offered = self.known == Known::NotLinked;
        DiscordLinkView {
            linked_as: match &self.known {
                Known::Linked(username) => Some(username.clone()),
                _ => None,
            },
            offered,
            linking: linking && offered,
            failed: self.failed.clone().filter(|_| offered),
            prompt: offered && !linking && self.failed.is_none() && any_shown && !prompt_dismissed,
        }
    }
}

/// What the site's answer to a read of the link state means, from its HTTP `status` and
/// `body` (ADR 0023).
pub fn read_link_state(status: u16, body: &str) -> DiscordLinkAnswer {
    #[derive(Deserialize)]
    struct State {
        linked: bool,
        username: Option<String>,
    }
    if status != 200 {
        return DiscordLinkAnswer::Failed { why: refusal(status, body) };
    }
    match serde_json::from_str::<State>(body) {
        Ok(State { linked: false, .. }) => DiscordLinkAnswer::NotLinked {},
        Ok(State { linked: true, username }) => DiscordLinkAnswer::Linked { username: username.unwrap_or_default() },
        Err(_) => DiscordLinkAnswer::Failed { why: unread(status) },
    }
}

/// The one-time token in the site's answer to starting a link, or why there is none. The
/// shell opens the link page on the site it asked, with the token, never an address the
/// answer names.
pub fn read_link_token(status: u16, body: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Token {
        token: String,
    }
    if status != 201 {
        return Err(refusal(status, body));
    }
    let token = serde_json::from_str::<Token>(body).map(|t| t.token).map_err(|_| unread(status))?;
    let fits = (20..=256).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if fits { Ok(token) } else { Err(unread(status)) }
}

/// A refusal in words: "HTTP 429: rate-limited".
fn refusal(status: u16, body: &str) -> String {
    #[derive(Deserialize)]
    struct Refusal {
        error: String,
    }
    match serde_json::from_str::<Refusal>(body) {
        Ok(refusal) if !refusal.error.is_empty() => format!("HTTP {status}: {}", refusal.error),
        _ => format!("HTTP {status}"),
    }
}

fn unread(status: u16) -> String {
    format!("HTTP {status}: an answer that doesn't read")
}
