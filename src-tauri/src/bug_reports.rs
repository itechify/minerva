//! The tray's Bug Report (ADR 0009): the core's report of the run, saved under `Bug
//! Reports` in the app data folder (the 10 newest kept), shown in Explorer, and the bug
//! issue form opened in the player's browser for it. If it can't be saved, the form opens
//! all the same, saying why.

use crate::errors::error;
use minerva_core::{Minerva, ReportAbout};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

/// GitHub's new-issue page for Minerva, which takes the bug form and its fields.
const ISSUE_FORM: &str = "https://github.com/itechify/minerva/issues/new";
/// How many Bug Reports are kept.
const KEPT: usize = 10;
/// Every Bug Report's name starts so.
const PREFIX: &str = "minerva-bug-report-";

/// Save a Bug Report, show it in Explorer, and open the bug form for it.
pub fn save(app: &AppHandle, minerva: &mut Minerva, overlay: &WebviewWindow) {
    let about = ReportAbout {
        version: app.package_info().version.to_string(),
        os: os(),
        display_scale: overlay.scale_factor().unwrap_or(1.0),
        local_time: local_time(),
    };
    let report = minerva.bug_report(&about);
    let saved = folder(app).and_then(|folder| {
        let path = folder.join(&report.file_name);
        crate::files::write_atomically(&path, &report.zip).map_err(|e| format!("couldn't write {}: {e}", path.display()))?;
        prune(&folder);
        Ok(path)
    });
    let filled = match &saved {
        Ok(path) => {
            if let Err(e) = app.opener().reveal_item_in_dir(path) {
                error!("couldn't show the Bug Report in its folder: {e}");
            }
            format!("Drag {} here. It's selected in the folder Minerva just opened.", report.file_name)
        }
        Err(e) => {
            error!("the Bug Report couldn't be saved: {e}");
            format!("The Bug Report couldn't be saved: {e}")
        }
    };
    let params = [("template", "bug.yml"), ("version", about.version.as_str()), ("report", filled.as_str())];
    match reqwest::Url::parse_with_params(ISSUE_FORM, &params) {
        Ok(url) => {
            if let Err(e) = app.opener().open_url(url.as_str(), None::<&str>) {
                error!("couldn't open the bug form: {e}");
            }
        }
        Err(e) => error!("couldn't make the bug form's address: {e}"),
    }
}

/// `Bug Reports` in the app data folder, made if it isn't there.
fn folder(app: &AppHandle) -> Result<PathBuf, String> {
    let folder = app.path().app_data_dir().map_err(|e| format!("couldn't find the app data folder: {e}"))?.join("Bug Reports");
    fs::create_dir_all(&folder).map_err(|e| format!("couldn't make {}: {e}", folder.display()))?;
    Ok(folder)
}

/// Delete all but the [`KEPT`] newest Bug Reports in `folder`.
fn prune(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else { return };
    let mut reports: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(PREFIX))
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .collect();
    reports.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, path) in reports.into_iter().skip(KEPT) {
        if let Err(e) = fs::remove_file(&path) {
            error!("couldn't delete an old Bug Report, {}: {e}", path.display());
        }
    }
}

/// The operating system, as the report states it: "Windows 11 24H2, build 26100".
#[cfg(windows)]
fn os() -> String {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::{HSTRING, PCWSTR};
    let value = |name: &str| -> Option<String> {
        let key = HSTRING::from(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
        let name = HSTRING::from(name);
        let mut buffer = [0u16; 64];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        // SAFETY: the buffer and its size in bytes are passed together; Windows writes
        // within them.
        let read = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if read.is_err() {
            return None;
        }
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..len]))
    };
    let Some(build) = value("CurrentBuild") else { return "Windows".to_string() };
    let windows = if build.parse::<u32>().is_ok_and(|build| build >= 22_000) { "Windows 11" } else { "Windows 10" };
    match value("DisplayVersion") {
        Some(version) => format!("{windows} {version}, build {build}"),
        None => format!("{windows}, build {build}"),
    }
}

#[cfg(not(windows))]
fn os() -> String {
    std::env::consts::OS.to_string()
}

/// The local date and time now, `yyyy-mm-dd hh:mm:ss`.
#[cfg(windows)]
fn local_time() -> String {
    // SAFETY: GetLocalTime only fills in the time it returns.
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

/// UTC, off Windows (where Minerva is only built to test).
#[cfg(not(windows))]
fn local_time() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, time) = (secs / 86_400, secs % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", time / 3600, time / 60 % 60, time % 60)
}
