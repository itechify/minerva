//! The tray icon: the owl in the notification area. Its menu opens setup (like
//! the panel's Help & Setup), saves a Bug Report (ADR 0009) and quits Minerva,
//! whether or not the game is open, the panel shows or the hotkeys are live. While a newer
//! version is out (ADR 0019), the menu starts with "Update to X.Y.Z…" (which the core turns
//! into updating, asking first, or the release page), and the tooltip says so too, then how
//! updating goes; once the player chose Update when I quit, Quit Minerva says it updates.
//! Nothing here touches the game (ADR 0001).

use crate::Command;
use crate::errors::error;
use minerva_core::{PanelControl, Updates, Updating};
use std::sync::mpsc::Sender;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{App, Wry};

/// The owl in the tray, kept to show a newer version on.
pub struct Tray {
    icon: TrayIcon<Wry>,
    menu: Menu<Wry>,
    /// "Update to X.Y.Z…", and the separator under it: in the menu only while a newer
    /// version is out.
    update: (MenuItem<Wry>, PredefinedMenuItem<Wry>),
    quit: MenuItem<Wry>,
    /// What the menu and tooltip show.
    shown: Texts,
}

/// What the tray says of updates.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Texts {
    /// The menu's first item, while a newer version is out.
    update: Option<String>,
    tooltip: String,
    quit: String,
}

impl Default for Texts {
    fn default() -> Self {
        Self { update: None, tooltip: "Minerva".to_string(), quit: "Quit Minerva".to_string() }
    }
}

/// Put the owl in the tray, with its menu on either click; each choice goes to the
/// core loop.
pub fn install(app: &App, commands: Sender<Command>) -> tauri::Result<Tray> {
    let setup = MenuItem::with_id(app, "setup", "Help & Setup", true, None::<&str>)?;
    let bug_report = MenuItem::with_id(app, "bug-report", "Bug Report", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Minerva", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&setup, &bug_report, &PredefinedMenuItem::separator(app)?, &quit])?;
    let update = (MenuItem::with_id(app, "update", "Update", true, None::<&str>)?, PredefinedMenuItem::separator(app)?);
    let mut tray = TrayIconBuilder::with_id("minerva")
        .tooltip(Texts::default().tooltip)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |_app, event| {
            let command = match event.id().as_ref() {
                // The core says what it does: update, ask first, or open the release page.
                "update" => Command::Press(PanelControl::Update),
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
        let texts = texts(updates);
        if texts == self.shown {
            return;
        }
        if let Err(e) = self.show(&texts) {
            error!("tray: {e}");
        }
        self.shown = texts;
    }

    fn show(&self, texts: &Texts) -> tauri::Result<()> {
        let (item, separator) = &self.update;
        match (self.shown.update.is_some(), &texts.update) {
            (false, Some(_)) => {
                self.menu.insert(item, 0)?;
                self.menu.insert(separator, 1)?;
            }
            (true, None) => {
                self.menu.remove(item)?;
                self.menu.remove(separator)?;
            }
            _ => {}
        }
        if let Some(text) = &texts.update {
            item.set_text(text)?;
        }
        self.quit.set_text(&texts.quit)?;
        self.icon.set_tooltip(Some(&texts.tooltip))
    }
}

/// What the tray says of `updates`: a newer version while one is out, how updating to it is
/// going, and that Quit Minerva installs it once the player chose Update when I quit.
fn texts(updates: &Updates) -> Texts {
    let Some(version) = updates.newer.as_ref().map(|newer| newer.version.as_str()) else { return Texts::default() };
    if updates.notify_only {
        return Texts {
            update: Some(format!("Minerva {version} is out…")),
            tooltip: format!("Minerva: version {version} is out"),
            ..Texts::default()
        };
    }
    let (update, tooltip) = match updates.updating {
        Updating::Installing => (format!("Installing {version}…"), format!("Minerva: installing version {version}")),
        _ if updates.when_i_quit => {
            (format!("Update to {version} now…"), format!("Minerva: version {version} installs when you quit"))
        }
        Updating::Downloading => (format!("Downloading {version}…"), format!("Minerva: downloading version {version}")),
        _ => (format!("Update to {version}…"), format!("Minerva: version {version} is out")),
    };
    let quit = if updates.when_i_quit { format!("Quit Minerva and update to {version}") } else { "Quit Minerva".into() };
    Texts { update: Some(update), tooltip, quit }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minerva_core::NewerVersion;

    fn out(version: &str) -> Updates {
        let release_url = format!("https://github.com/itechify/minerva/releases/tag/v{version}");
        Updates { newer: Some(NewerVersion { version: version.into(), release_url }), ..Updates::default() }
    }

    #[test]
    fn a_newer_version_shows_in_the_menu_and_the_tooltip() {
        let texts = texts(&out("0.13.0"));
        assert_eq!(texts.update.as_deref(), Some("Update to 0.13.0…"));
        assert_eq!(texts.tooltip, "Minerva: version 0.13.0 is out");
        assert_eq!(texts.quit, "Quit Minerva");
    }

    #[test]
    fn a_build_that_cannot_install_only_says_it_is_out() {
        let texts = texts(&Updates { notify_only: true, ..out("0.13.0") });
        assert_eq!(texts.update.as_deref(), Some("Minerva 0.13.0 is out…"));
        assert_eq!(texts.tooltip, "Minerva: version 0.13.0 is out");
    }

    #[test]
    fn downloading_and_installing_show_in_the_menu_and_the_tooltip() {
        let texts = texts(&Updates { updating: Updating::Downloading, ..out("0.13.0") });
        assert_eq!(texts.update.as_deref(), Some("Downloading 0.13.0…"));
        assert_eq!(texts.tooltip, "Minerva: downloading version 0.13.0");
        let texts = super::texts(&Updates { updating: Updating::Installing, ..out("0.13.0") });
        assert_eq!(texts.update.as_deref(), Some("Installing 0.13.0…"));
    }

    #[test]
    fn update_when_i_quit_shows_on_quit_minerva_and_in_the_tooltip() {
        let texts = texts(&Updates { updating: Updating::Downloaded, when_i_quit: true, ..out("0.13.0") });
        assert_eq!(texts.update.as_deref(), Some("Update to 0.13.0 now…"));
        assert_eq!(texts.tooltip, "Minerva: version 0.13.0 installs when you quit");
        assert_eq!(texts.quit, "Quit Minerva and update to 0.13.0");
    }

    #[test]
    fn with_none_the_menu_has_no_update_and_the_tooltip_is_the_name() {
        assert_eq!(texts(&Updates::default()), Texts::default());
        assert_eq!(Texts::default().tooltip, "Minerva");
        assert_eq!(Texts::default().quit, "Quit Minerva");
    }
}
