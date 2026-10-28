//! The Item Card's window (ADR 0010): the hovered item's card, at the pointer where the
//! core puts it, or where the player dragged it by its bar. Without a card its page draws
//! nothing, and clicks pass through it. While a card shows it takes the clicks on it (its
//! bar, its ×, its name, its Market Lines and buttons, an item to pick), and none reach the
//! game. While a price or line name field is open it takes the keyboard too, as the search
//! view does: it comes to the front right after the player's click, and gives the game the
//! keyboard back once the field closes. Before a capture of the game it moves off the screen
//! ([`OUT_OF_SIGHT`]), and the core puts it back. One of our windows over the game
//! ([`crate::panel_window`]); this says what its page renders, and relays what the player does
//! in it.

use crate::Command;
use crate::panel_window::{Heights, PanelWindow, Shows, WindowSpec};
use minerva_core::{Action, Minerva, PanelState, ShownItemCard};
use serde::Serialize;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tauri::{PhysicalPosition, State};

pub const ITEM_CARD: &str = "item-card";

/// Its width in CSS px (logical), which `src/item-card.css` lays the card out to. Its height
/// follows the card, as the page reports it: past the most, the card's body scrolls under its
/// bar.
pub const SPEC: WindowSpec = WindowSpec {
    label: ITEM_CARD,
    event: "item-card-state",
    watcher: "on-top-item-card",
    width: 280.0,
    height: Heights { default: 160.0, min: 40.0, max: 420.0 },
    sized: Minerva::set_item_card_size,
    moved: Some(|x, y| Action::ItemCardMoved { x, y }),
    beneath: &[],
};

/// Where the card (and the Deal Badges) wait while a capture is taken: off every screen, as
/// Windows parks a minimised window.
pub(crate) const OUT_OF_SIGHT: PhysicalPosition<i32> = PhysicalPosition::new(-32_000, -32_000);
/// How long a move of the card takes to leave the screen: two frames at 60 Hz, and a
/// little more, for the desktop's compositor to draw the screen without it.
pub(crate) const OFF_SCREEN_SETTLE: Duration = Duration::from_millis(50);

/// What the card's page renders.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CardView<'a> {
    /// The window shows: otherwise the page draws nothing.
    shown: bool,
    card: Option<&'a ShownItemCard>,
}

/// Show the card where the core says, or nothing; take clicks only while it shows, and the
/// keyboard while a price or line name field is open on it.
pub fn sync(window: &mut PanelWindow, state: &PanelState) {
    let placement = state.item_card_placement;
    let card = state.item_card.as_ref().filter(|_| placement.is_some());
    let view = CardView { shown: card.is_some(), card };
    window.sync(&view, Shows { placement, clickable: card.is_some(), takes_keyboard: state.item_card_takes_keyboard });
}

/// The player clicked the card's ×.
#[tauri::command]
pub fn item_card_close(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::CloseItemCard);
}

/// The player picked the item with this game ID from those the card lists.
#[tauri::command]
pub fn item_card_pick(id: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::PickCardItem(id));
}

/// The player clicked the card's name: its MeowDB page.
#[tauri::command]
pub fn item_card_open_meowdb(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenCardMeowdbPage);
}

/// The player clicked one of the card's buttons that look the item up on maplestory.market.
#[tauri::command]
pub fn item_card_open_market(commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::OpenCardMarket);
}

/// The player clicked the price on the Market Line with this ID, to type one.
#[tauri::command]
pub fn item_card_edit_market(line: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::EditCardMarketPrice(line));
}

/// The player pressed Enter in the price field, or left it, holding `typed`.
#[tauri::command]
pub fn item_card_enter_market(typed: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::EnterCardMarket(typed));
}

/// The player clicked the name of the Market Line with this ID, to rename or remove it.
#[tauri::command]
pub fn item_card_edit_market_line(line: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::EditCardMarketLine(line));
}

/// The player pressed Enter in a line name field, or left it with something typed,
/// holding `typed`.
#[tauri::command]
pub fn item_card_enter_market_line(typed: String, commands: State<'_, Sender<Command>>) {
    let _ = commands.send(Command::EnterCardMarketLine(typed));
}

