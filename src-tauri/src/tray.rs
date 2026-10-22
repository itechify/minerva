//! The tray icon: the owl in the notification area. Its menu opens setup (like
//! the panel's Help & Setup), saves a Bug Report (ADR 0009) and quits Minerva,
//! whether or not the game is open, the panel shows or the hotkeys are live. While a newer
//! version is out (ADR 0019), the menu starts with "Update to X.Y.Z…", and the tooltip
//! says so too. Nothing here touches the game (ADR 0001).

use crate::Command;
use crate::errors::error;
use minerva_core::{PanelControl, Updates};
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
    /// The newer version the menu and tooltip show.
    shown: Option<String>,
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
        .tooltip(tooltip(None))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |_app, event| {
            let command = match event.id().as_ref() {
                // For now, every build opens the release page (#128 updates in place).
                "update" => Command::Press(PanelControl::OpenRelease),
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
    Ok(Tray { icon, menu, update, shown: None })
}

impl Tray {
    /// Show the newer version `updates` has, if it changed since last shown.
    pub fn sync(&mut self, updates: &Updates) {
        let newer = updates.newer.as_ref().map(|newer| newer.version.clone());
        if newer == self.shown {
            return;
        }
        if let Err(e) = self.show(newer.as_deref()) {
            error!("tray: {e}");
        }
        self.shown = newer;
    }

    fn show(&self, newer: Option<&str>) -> tauri::Result<()> {
        let (item, separator) = &self.update;
        match (self.shown.is_some(), newer) {
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
        if let Some(text) = item_text(newer) {
            item.set_text(text)?;
        }
        self.icon.set_tooltip(Some(tooltip(newer)))
    }
}

/// The menu's first item while version `newer` is out.
fn item_text(newer: Option<&str>) -> Option<String> {
    newer.map(|version| format!("Update to {version}…"))
}

/// The tray's tooltip: "Minerva", and a newer version while one is out.
fn tooltip(newer: Option<&str>) -> String {
    match newer {
        Some(version) => format!("Minerva: version {version} is out"),
        None => "Minerva".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_version_shows_in_the_menu_and_the_tooltip() {
        assert_eq!(item_text(Some("0.13.0")).as_deref(), Some("Update to 0.13.0…"));
        assert_eq!(tooltip(Some("0.13.0")), "Minerva: version 0.13.0 is out");
    }

    #[test]
    fn with_none_the_menu_has_no_update_and_the_tooltip_is_the_name() {
        assert_eq!(item_text(None), None);
        assert_eq!(tooltip(None), "Minerva");
    }
}
