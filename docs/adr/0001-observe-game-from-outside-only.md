---
status: accepted, amended by ADR-0010
---

# Observe the game from outside only

Minerva only reads pixels through ordinary operating-system screen capture and draws in its own windows. It must not interact with the MapleStory client: no process access, game memory or runtime file access, injected code, renderer hooks, messages or input. `PrintWindow` is ruled out too. Account safety is the constraint, even where direct access would be more accurate.

Overlay windows are clickable and no-activate (`WS_EX_NOACTIVATE`), so a click on Minerva leaves the game's keyboard focus alone. They are topmost only while the game or one of our windows is in front, and hide while another app is. Visibility is decided by polling `GetForegroundWindow`; only our windows are restacked, without activation. A cross-process owner or parent (`GWLP_HWNDPARENT`, `SetParent`) would attach our input queue to the game's, so neither is allowed. Window-event hooks (`SetWinEventHook`) are avoided for the same reason.

Search takes the keyboard after its hotkey or a click opens it, using `SetForegroundWindow` on our own window. Closing Search restores no-activate behaviour and, if it still has the foreground, hides and shows the window without activation so Windows returns focus below it. Settings takes the keyboard in its own window, including after a click on Change or Deal Margin. The Item Card's market fields take it after a click and give it back when closed. Nothing is sent to the game.

Global hotkeys use `RegisterHotKey`, never low-level keyboard hooks or key-state polling. They are registered only while the game or our windows are in front, so other apps retain their shortcuts. The defaults use Ctrl+Shift+letter: Win+Alt combinations conflict with Windows and other apps. A registered shortcut is lost to the game while active, so the player can rebind ours.

Our windows appear in captures and streams. The core places them clear of the located Minimap Title and Status Bar readings (ADRs 0016, 0017, 0021). Before an item-hotkey capture the shell moves the Item Card out of the way, since it may cover the Item Tooltip; the core then restores it. A click outside the card goes to the game and Minerva never hears it, so it cannot close the card.
