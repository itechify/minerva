//! Sending each of our pages but the overlay (setup, the Grind Tracker's, the Personal
//! Bests', the Owl Bubble's, the Item Card's and the Deal Badges') the view it
//! renders: only when it changed, since the loop wakes up 20 times a second, and kept by its
//! window's label for a page that loads after it was sent ([`page_view`]).

use crate::errors::error;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

/// The view last sent to each page, by its window's label, for the page to ask for when it
/// loads late.
#[derive(Default)]
pub struct PageViews(Mutex<HashMap<&'static str, Value>>);

/// Sends one page its view as an event.
pub struct ViewSender {
    label: &'static str,
    event: &'static str,
    sent: Option<Value>,
}

impl ViewSender {
    pub fn new(label: &'static str, event: &'static str) -> Self {
        Self { label, event, sent: None }
    }

    /// Send `view` if it differs from the last one sent, and keep it in the app's
    /// [`PageViews`]. Whether it was sent.
    pub fn send(&mut self, app: &AppHandle, view: &impl Serialize) -> bool {
        let Ok(view) = serde_json::to_value(view) else { return false };
        if self.sent.as_ref() == Some(&view) {
            return false;
        }
        if let Err(e) = app.emit_to(self.label, self.event, &view) {
            error!("emit failed: {e}");
        }
        app.state::<PageViews>().0.lock().unwrap().insert(self.label, view.clone());
        self.sent = Some(view);
        true
    }
}

/// What the asking window's page renders, as last sent (`None` before the first).
#[tauri::command]
pub async fn page_view(window: tauri::Window, views: State<'_, PageViews>) -> Result<Option<Value>, String> {
    Ok(views.0.lock().unwrap().get(window.label()).cloned())
}
