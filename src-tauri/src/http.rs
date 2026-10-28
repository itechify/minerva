//! What the shell's data source fetches with: one HTTP client that says who is asking,
//! and fetches that run on the async runtime and never block the caller. A source starts
//! one with an [`InFlight`] and answers `Pending`, then collects it on later calls.

use crate::errors::error;
use minerva_core::SourceError;
use crate::Command;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::task::Poll;
use std::time::Duration;

/// Who is asking: name, version, purpose and where to find out more. Never a browser UA.
pub(crate) const USER_AGENT: &str = concat!(
    "Minerva/",
    env!("CARGO_PKG_VERSION"),
    " (personal-use map helper; +https://github.com/itechify/minerva)"
);

/// An HTTP client that says who is asking, with Minerva's timeouts.
pub(crate) fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
}

/// The text at `url`, or why it couldn't be had (naming the URL).
pub(crate) async fn text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let response = client.get(url).send().await.and_then(|r| r.error_for_status());
    let text = match response {
        Ok(response) => response.text().await,
        Err(e) => Err(e),
    };
    text.map_err(|e| format!("{url}: {e}"))
}

/// What a site answered a request: its HTTP status, its `Retry-After` header and its body,
/// whatever the status. The caller reads what they mean.
pub(crate) struct Reply {
    pub status: u16,
    pub retry_after: Option<String>,
    pub body: String,
}

/// Send `method` to `url` with `key` as its bearer token, and `body` as JSON if there is one;
/// the reply, or why none came (naming the URL).
pub(crate) async fn send(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    key: &str,
    body: Option<String>,
) -> Result<Reply, String> {
    let mut request = client.request(method, url).bearer_auth(key);
    if let Some(body) = body {
        request = request.header(reqwest::header::CONTENT_TYPE, "application/json").body(body);
    }
    reply(request, url).await
}

/// `GET` `url`, with no key; the reply, whatever its status, or why none came (naming the URL).
pub(crate) async fn get(client: &reqwest::Client, url: &str) -> Result<Reply, String> {
    reply(client.get(url), url).await
}

async fn reply(request: reqwest::RequestBuilder, url: &str) -> Result<Reply, String> {
    let sent = async {
        let response = request.send().await?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .map(String::from);
        let body = response.text().await?;
        Ok::<_, reqwest::Error>(Reply { status, retry_after, body })
    };
    sent.await.map_err(|e| format!("{url}: {e}"))
}

/// Run `work` on the async runtime: its outcome arrives on the receiver.
pub(crate) fn in_background<T: Send + 'static>(work: impl Future<Output = T> + Send + 'static) -> Receiver<T> {
    let (tx, rx) = mpsc::channel();
    tauri::async_runtime::spawn(async move {
        let _ = tx.send(work.await);
    });
    rx
}

/// Run `work` on the async runtime, and send the core loop the command it comes to.
pub(crate) fn answer_in_background(commands: Sender<Command>, work: impl Future<Output = Command> + Send + 'static) {
    tauri::async_runtime::spawn(async move {
        let _ = commands.send(work.await);
    });
}

/// What a background fetch sent, if it has finished; a fetch that ended without sending
/// anything was abandoned.
fn collect<T>(arriving: &Receiver<Result<T, SourceError>>) -> Poll<Result<T, SourceError>> {
    match arriving.try_recv() {
        Err(TryRecvError::Empty) => Poll::Pending,
        Ok(answer) => Poll::Ready(answer),
        Err(TryRecvError::Disconnected) => Poll::Ready(Err(SourceError::Unreachable("fetch abandoned".into()))),
    }
}

/// A fetch on its way in the background, by what it fetches (its key: a name searched for, a
/// picture's path, or nothing for a set of files), for a source that answers `Pending` until
/// it has arrived. Each source says what to do while another key's fetch is on its way.
pub(crate) struct InFlight<T> {
    fetching: Option<(String, Receiver<Result<T, SourceError>>)>,
}

impl<T> Default for InFlight<T> {
    fn default() -> Self {
        Self { fetching: None }
    }
}

impl<T: Send + 'static> InFlight<T> {
    /// The fetch of `key`'s answer, once it has arrived (a failure reported as `what`'s), or
    /// `Pending` while it is on its way; `None` if no fetch of `key` is (another key's is left
    /// as it is).
    pub fn collect(&mut self, key: &str, what: impl std::fmt::Display) -> Option<Poll<Result<T, SourceError>>> {
        let (asked, arriving) = self.fetching.as_ref()?;
        if asked != key {
            return None;
        }
        let Poll::Ready(answer) = collect(arriving) else { return Some(Poll::Pending) };
        self.fetching = None;
        if let Err(e) = &answer {
            error!("{what}: {e}");
        }
        Some(Poll::Ready(answer))
    }

    /// Whether another key's fetch is still on its way; one that has arrived unheard is let
    /// go.
    pub fn busy(&mut self) -> bool {
        if self.fetching.as_ref().is_some_and(|(_, arriving)| collect(arriving).is_ready()) {
            self.fetching = None;
        }
        self.fetching.is_some()
    }

    /// Start fetching `key` with `work` in the background, in place of any fetch on its way:
    /// `Pending`, until [`InFlight::collect`] has its answer.
    pub fn start(
        &mut self,
        key: &str,
        work: impl Future<Output = Result<T, SourceError>> + Send + 'static,
    ) -> Poll<Result<T, SourceError>> {
        self.fetching = Some((key.to_string(), in_background(work)));
        Poll::Pending
    }
}
