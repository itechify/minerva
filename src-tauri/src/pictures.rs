//! OSMS's pictures (ADR 0008): mob animations and map pictures, each named by its content
//! at a path below [`PICTURES_URL`]. They are fetched one at a time with Minerva's
//! own user agent, kept under the cache's [`FOLDER`] at the same path, and served to the
//! overlay from there through the [`SCHEME`] URL scheme: the webview never asks OSMS
//! itself.
//!
//! A path names a picture by its content, so a saved one never goes stale. A picture is
//! touched whenever it is reused or served, and one untouched for [`KEPT_FOR`] (30 days)
//! is deleted at launch ([`sweep`]).

use crate::errors::error;
use crate::http;
use minerva_core::SourceError;
use minerva_core::osms::PICTURES_URL;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::task::Poll;
use std::time::{Duration, SystemTime};

/// The URL scheme the overlay loads OSMS's pictures from.
pub const SCHEME: &str = "osmsimage";

/// The folder in the cache the pictures are kept in, each at its path.
const FOLDER: &str = "osms-images";

/// How long a picture nobody has asked for is kept.
const KEPT_FOR: Duration = Duration::from_secs(30 * 24 * 60 * 60);

type Answer = Result<String, SourceError>;

/// OSMS's pictures: saved, and fetched when asked for.
pub struct Pictures {
    client: reqwest::Client,
    /// Where the pictures are kept.
    dir: PathBuf,
    /// The picture being fetched, by path, and where its answer will arrive.
    in_flight: Option<(String, Receiver<Answer>)>,
}

impl Pictures {
    /// Pictures kept under `cache_dir`, fetched with `client`.
    pub fn new(client: reqwest::Client, cache_dir: &Path) -> Self {
        Self { client, dir: cache_dir.join(FOLDER), in_flight: None }
    }

    /// Where the overlay loads the picture at `path` from: at once if it is saved,
    /// otherwise once it is fetched, in the background. One fetch at a time: a picture
    /// asked for while another is on its way waits for it.
    pub fn poll(&mut self, path: &str) -> Poll<Answer> {
        let Some(file) = file(&self.dir, path) else {
            return Poll::Ready(Err(SourceError::FormatChanged(format!("picture path {path:?}"))));
        };
        if let Some((asked, arriving)) = &self.in_flight {
            let mine = asked == path;
            match (http::collect(arriving), mine) {
                (Poll::Pending, true) => return Poll::Pending,
                (Poll::Pending, false) => {}
                (Poll::Ready(answer), true) => {
                    self.in_flight = None;
                    if let Err(e) = &answer {
                        error!("OSMS picture {path}: {e}");
                    }
                    return Poll::Ready(answer);
                }
                // Another picture's, saved unheard.
                (Poll::Ready(_), false) => self.in_flight = None,
            }
        }
        if file.is_file() {
            touch(&file);
            return Poll::Ready(Ok(url(path)));
        }
        if self.in_flight.is_some() {
            return Poll::Pending;
        }
        let (client, owned) = (self.client.clone(), path.to_string());
        let arriving = http::in_background(async move { fetch(&client, &owned, &file).await });
        self.in_flight = Some((path.to_string(), arriving));
        Poll::Pending
    }
}

/// Where the overlay loads the picture at `path` from; [`serve`] answers it.
fn url(path: &str) -> String {
    if cfg!(windows) { format!("http://{SCHEME}.localhost/{path}") } else { format!("{SCHEME}://localhost/{path}") }
}

/// Where the picture at `path` is kept under `dir`. `None` for a path that isn't one of
/// OSMS's picture paths: segments of letters, digits, `.`, `-` and `_`, none of them `.`
/// or `..`.
fn file(dir: &Path, path: &str) -> Option<PathBuf> {
    let mut file = dir.to_path_buf();
    for segment in path.split('/') {
        let plain = |b: u8| b.is_ascii_alphanumeric() || b"._-".contains(&b);
        if segment.is_empty() || segment == "." || segment == ".." || !segment.bytes().all(plain) {
            return None;
        }
        file.push(segment);
    }
    Some(file)
}

/// Fetch the picture at `path` from OSMS and save it at `file`.
async fn fetch(client: &reqwest::Client, path: &str, file: &Path) -> Answer {
    let address = format!("{PICTURES_URL}{path}");
    let response = client.get(&address).send().await.map_err(|e| SourceError::Unreachable(e.to_string()))?;
    let status = response.status();
    if matches!(status, reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::GONE) {
        return Err(SourceError::FormatChanged(format!("{address}: {status}")));
    }
    let response = response.error_for_status().map_err(|e| SourceError::Unreachable(e.to_string()))?;
    let is_image = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|t| t.to_str().ok())
        .is_some_and(|t| t.starts_with("image/"));
    if !is_image {
        return Err(SourceError::FormatChanged(format!("{address}: not a picture")));
    }
    let bytes = response.bytes().await.map_err(|e| SourceError::Unreachable(e.to_string()))?;
    crate::files::write_atomically(file, &bytes)
        .map_err(|e| SourceError::Unreachable(format!("couldn't save {}: {e}", file.display())))?;
    Ok(url(path))
}

/// Answer the overlay's request for a picture ([`url`]) from the pictures kept under
/// `cache_dir`, and touch it.
pub fn serve(cache_dir: &Path, request: &tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode, header};
    let path = request.uri().path().trim_start_matches('/');
    let file = file(&cache_dir.join(FOLDER), path);
    let response = match file.as_ref().and_then(|file| Some((file, std::fs::read(file).ok()?))) {
        Some((file, bytes)) => {
            touch(file);
            Response::builder().header(header::CONTENT_TYPE, content_type(path)).body(bytes)
        }
        None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
    };
    response.unwrap_or_default()
}

/// A picture's type, by its file name's extension.
fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension.to_ascii_lowercase()).as_deref() {
        Some("webp") => "image/webp",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    }
}

/// Mark the picture at `file` as used now, so it is kept another [`KEPT_FOR`].
fn touch(file: &Path) {
    let touched = std::fs::File::options().write(true).open(file).and_then(|f| f.set_modified(SystemTime::now()));
    if let Err(e) = touched {
        error!("OSMS picture: couldn't touch {}: {e}", file.display());
    }
}

/// Delete every picture kept under `cache_dir` that hasn't been used for [`KEPT_FOR`].
/// Slow with many pictures: call it off the core loop.
pub fn sweep(cache_dir: &Path) {
    sweep_older_than(&cache_dir.join(FOLDER), SystemTime::now() - KEPT_FOR);
}

/// Delete every file under `dir`, at any depth, last touched before `cutoff`.
fn sweep_older_than(dir: &Path, cutoff: SystemTime) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            sweep_older_than(&entry.path(), cutoff);
        } else if entry.metadata().and_then(|m| m.modified()).is_ok_and(|touched| touched < cutoff)
            && let Err(e) = std::fs::remove_file(entry.path()) {
                error!("OSMS picture: couldn't delete {}: {e}", entry.path().display());
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty folder for one test.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minerva-pictures-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn only_osmss_own_picture_paths_are_kept_below_the_folder() {
        let dir = Path::new("cache").join(FOLDER);
        assert_eq!(file(&dir, "images/monsters/43d306.webp"), Some(dir.join("images").join("monsters").join("43d306.webp")));
        assert_eq!(file(&dir, "maps/a1-b_2.webp"), Some(dir.join("maps").join("a1-b_2.webp")));
        for path in ["", "/maps/a.webp", "maps//a.webp", "../a.webp", "maps/../../a.webp", "maps/./a.webp", r"maps\a.webp", "maps/a b.webp", "C:/a.webp"] {
            assert_eq!(file(&dir, path), None, "{path:?}");
        }
    }

    #[test]
    fn the_overlay_loads_a_picture_through_the_osmsimage_scheme() {
        let expected = if cfg!(windows) {
            "http://osmsimage.localhost/images/monsters/43d306.webp"
        } else {
            "osmsimage://localhost/images/monsters/43d306.webp"
        };
        assert_eq!(url("images/monsters/43d306.webp"), expected);
    }

    #[test]
    fn a_picture_untouched_for_30_days_is_deleted_and_a_younger_one_kept() {
        let cache = scratch("sweep");
        let monsters = cache.join(FOLDER).join("images").join("monsters");
        std::fs::create_dir_all(&monsters).unwrap();
        let (old, young) = (monsters.join("old.webp"), monsters.join("young.webp"));
        for file in [&old, &young] {
            std::fs::write(file, b"webp").unwrap();
        }
        let file = std::fs::File::options().write(true).open(&old).unwrap();
        file.set_modified(SystemTime::now() - KEPT_FOR - Duration::from_secs(60)).unwrap();
        drop(file);

        sweep(&cache);

        assert!(!old.exists());
        assert!(young.exists());
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn a_served_picture_is_touched() {
        let cache = scratch("serve");
        let maps = cache.join(FOLDER).join("maps");
        std::fs::create_dir_all(&maps).unwrap();
        let picture = maps.join("a1.webp");
        std::fs::write(&picture, b"webp").unwrap();
        let file = std::fs::File::options().write(true).open(&picture).unwrap();
        file.set_modified(SystemTime::now() - KEPT_FOR - Duration::from_secs(60)).unwrap();
        drop(file);

        let request = tauri::http::Request::builder().uri(url("maps/a1.webp")).body(Vec::new()).unwrap();
        let response = serve(&cache, &request);

        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "image/webp");
        assert_eq!(response.body(), b"webp");
        sweep(&cache);
        assert!(picture.exists());
        let missing = tauri::http::Request::builder().uri(url("maps/none.webp")).body(Vec::new()).unwrap();
        assert_eq!(serve(&cache, &missing).status(), 404);
        let _ = std::fs::remove_dir_all(&cache);
    }
}
