//! The tray icon: the owl in the notification area. Its menu opens setup (like
//! the panel's Help & Setup), saves a Bug Report (ADR 0009) and quits Minerva,
//! whether or not the game is open, the panel shows or the hotkeys are live. While a newer
//! version is out (ADR 0019), the menu starts with an item for it and its "What's new", and
//! the tooltip and Quit Minerva say how updating goes: the core decides all of it
//! ([`UpdateNotice`]), and this only puts it into words. Nothing here touches the game
//! (ADR 0001).

use crate::Command;
use crate::errors::error;
use minerva_core::{PanelControl, QuitItem, UpdateNotice, UpdateOffer, UpdateStage, Updates};
use std::sync::mpsc::Sender;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{App, Wry};

/// The owl in the tray, kept to show a newer version on.
pub struct Tray {
    icon: TrayIcon<Wry>,
    menu: Menu<Wry>,
    /// The item for the newer version, its "What's new", and the separator under them: in the
    /// menu only while a newer version is out.
    update: (MenuItem<Wry>, MenuItem<Wry>, PredefinedMenuItem<Wry>),
    quit: MenuItem<Wry>,
    /// What the menu and tooltip show.
    shown: Texts,
}

/// What the tray says of updates.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Texts {
    /// While a newer version is out: the menu's first item, and whether it does anything now.
    update: Option<(String, bool)>,
    /// "What's new in X.Y.Z", under it.
    whats_new: Option<String>,
    tooltip: String,
    quit: String,
}

impl Default for Texts {
    fn default() -> Self {
        Self { update: None, whats_new: None, tooltip: "Minerva".to_string(), quit: "Quit Minerva".to_string() }
    }
}

/// Put the owl in the tray, with its menu on either click; each choice goes to the
/// core loop.
pub fn install(app: &App, commands: Sender<Command>) -> tauri::Result<Tray> {
    let setup = MenuItem::with_id(app, "setup", "Help & Setup", true, None::<&str>)?;
    let bug_report = MenuItem::with_id(app, "bug-report", "Bug Report", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Minerva", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&setup, &bug_report, &PredefinedMenuItem::separator(app)?, &quit])?;
    let update = (
        MenuItem::with_id(app, "update", "Update", true, None::<&str>)?,
        MenuItem::with_id(app, "whats-new", "What's new", true, None::<&str>)?,
        PredefinedMenuItem::separator(app)?,
    );
    let mut tray = TrayIconBuilder::with_id("minerva")
        .tooltip(Texts::default().tooltip)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |_app, event| {
            let command = match event.id().as_ref() {
                // The core says what it does: update, ask first, or open the release page.
                "update" => Command::Press(PanelControl::Update),
                "whats-new" => Command::Press(PanelControl::OpenRelease),
                "setup" => Command::Press(PanelControl::Setup),
                "bug-report" => Command::BugReport,
                "quit" => Command::Quit,
                _ => return,
            };
            let _ = commands.send(command);
        });
    // The app icon, the owl, as the windows use it.
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let icon = tray.build(app)?;
    Ok(Tray { icon, menu, update, quit, shown: Texts::default() })
}

impl Tray {
    /// Show what `updates` says, if it changed since last shown.
    pub fn sync(&mut self, updates: &Updates) {
        let texts = texts(updates.notice.as_ref());
        if texts == self.shown {
            return;
        }
        if let Err(e) = self.show(&texts) {
            error!("tray: {e}");
        }
        self.shown = texts;
    }

    /// Take the owl out of the tray (`false`) before the installer ends Minerva with
    /// `process::exit(0)`, which leaves no time to remove it, or put it back (`true`) if the
    /// installer didn't start.
    pub fn set_visible(&self, visible: bool) {
        if let Err(e) = self.icon.set_visible(visible) {
            error!("tray: {e}");
        }
    }

    fn show(&self, texts: &Texts) -> tauri::Result<()> {
        let (item, whats_new, separator) = &self.update;
        match (self.shown.update.is_some(), &texts.update) {
            (false, Some(_)) => {
                self.menu.insert(item, 0)?;
                self.menu.insert(whats_new, 1)?;
                self.menu.insert(separator, 2)?;
            }
            (true, None) => {
                self.menu.remove(item)?;
                self.menu.remove(whats_new)?;
                self.menu.remove(separator)?;
            }
            _ => {}
        }
        if let Some((text, enabled)) = &texts.update {
            item.set_text(text)?;
            item.set_enabled(*enabled)?;
        }
        if let Some(text) = &texts.whats_new {
            whats_new.set_text(text)?;
        }
        self.quit.set_text(&texts.quit)?;
        self.icon.set_tooltip(Some(&texts.tooltip))
    }
}

/// What the tray says of the core's `notice`, word for word.
fn texts(notice: Option<&UpdateNotice>) -> Texts {
    let Some(UpdateNotice { version, stage, offer, quit, .. }) = notice else { return Texts::default() };
    let update = match stage {
        UpdateStage::NotifyOnly => format!("Minerva {version} is out…"),
        UpdateStage::Out
        | UpdateStage::Asking
        | UpdateStage::DownloadFailed
        | UpdateStage::InstallFailed
        | UpdateStage::Changed => format!("Update to {version}…"),
        UpdateStage::Downloading => format!("Downloading {version}…"),
        UpdateStage::DownloadingForQuit | UpdateStage::InstallsWhenYouQuit => format!("Update to {version} now…"),
        UpdateStage::FinishingBeforeQuit => "Finishing the update before quitting…".to_string(),
        UpdateStage::Installing => format!("Installing {version}…"),
    };
    let tooltip = match stage {
        UpdateStage::NotifyOnly
        | UpdateStage::Out
        | UpdateStage::Asking
        | UpdateStage::DownloadFailed
        | UpdateStage::InstallFailed
        | UpdateStage::Changed => format!("Minerva: version {version} is out"),
        UpdateStage::Downloading => format!("Minerva: downloading version {version}"),
        UpdateStage::DownloadingForQuit | UpdateStage::InstallsWhenYouQuit => {
            format!("Minerva: version {version} installs when you quit")
        }
        UpdateStage::FinishingBeforeQuit => format!("Minerva: finishing the download of version {version}, then quitting"),
        UpdateStage::Installing => format!("Minerva: installing version {version}"),
    };
    let quit = match quit {
        QuitItem::Quit => "Quit Minerva".to_string(),
        QuitItem::QuitAndUpdate => format!("Quit Minerva and update to {version}"),
        QuitItem::QuitWithoutUpdating => "Quit now, without updating".to_string(),
    };
    Texts {
        update: Some((update, *offer != UpdateOffer::Nothing)),
        whats_new: Some(format!("What's new in {version}")),
        tooltip,
        quit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(stage: UpdateStage, offer: UpdateOffer, quit: QuitItem) -> UpdateNotice {
        let release_url = "https://github.com/itechify/minerva/releases/tag/v0.13.0".to_string();
        UpdateNotice { version: "0.13.0".into(), release_url, stage, offer, quit }
    }

    #[test]
    fn with_no_notice_the_menu_has_no_update_and_the_tooltip_is_the_name() {
        assert_eq!(texts(None), Texts::default());
        assert_eq!(Texts::default().tooltip, "Minerva");
        assert_eq!(Texts::default().quit, "Quit Minerva");
    }

    #[test]
    fn a_notice_reads_as_the_core_decided_it() {
        let texts = texts(Some(&notice(UpdateStage::Out, UpdateOffer::Update, QuitItem::Quit)));
        assert_eq!(texts.update, Some(("Update to 0.13.0…".to_string(), true)));
        assert_eq!(texts.whats_new.as_deref(), Some("What's new in 0.13.0"));
        assert_eq!(texts.tooltip, "Minerva: version 0.13.0 is out");
        assert_eq!(texts.quit, "Quit Minerva");

        let notice = notice(UpdateStage::FinishingBeforeQuit, UpdateOffer::Nothing, QuitItem::QuitWithoutUpdating);
        let texts = super::texts(Some(&notice));
        assert_eq!(texts.update, Some(("Finishing the update before quitting…".to_string(), false)));
        assert_eq!(texts.quit, "Quit now, without updating");
    }
}
