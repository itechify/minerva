//! What the shell's data source fetches with: one HTTP client that says who is asking,
//! and fetches that run on the async runtime and never block the caller. A source starts
//! one with [`in_background`] and answers `Pending`, then [`collect`]s it on later calls.

use minerva_core::SourceError;
use std::sync::mpsc::{self, Receiver, TryRecvError};
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

/// What a background fetch sent, if it has finished; a fetch that ended without sending
/// anything was abandoned.
pub(crate) fn collect<T>(arriving: &Receiver<Result<T, SourceError>>) -> Poll<Result<T, SourceError>> {
    match arriving.try_recv() {
        Err(TryRecvError::Empty) => Poll::Pending,
        Ok(answer) => Poll::Ready(answer),
        Err(TryRecvError::Disconnected) => Poll::Ready(Err(SourceError::Unreachable("fetch abandoned".into()))),
    }
}
