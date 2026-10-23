# Coding standards

Read at review, not while implementing: `AGENTS.md` holds the rules every change follows, and this file the judgement calls a diff is checked against. A finding cites the rule and the lines.

## Logic lives in the core

Anything testable without an OS belongs in `crates/minerva-core`: a decision taken from `PanelState` (who takes the keyboard, whether a window is clickable, whether a reading is stale), a cache or retry policy, parsing, formatting shown text, arithmetic on game data. `src-tauri` only looks windows up, captures and renders (`AGENTS.md`, ADR 0001). A shell diff that compares, filters, times or formats game data is a finding, named with the core type it moves into.

## A test helper is written once

A helper that a test file defines and another under `crates/minerva-core/tests/` already defines (a game window, a settings value, a title reading, a made-up world, a sample's name) moves to `tests/common`. The check: the same name, or the same body, in two test files.

## One panel window adapter

A new panel window in `src-tauri` is a `WindowSpec` driving the shared `PanelWindow` (keep-on-top, click-through, drag and resize, the height clamp), not a copy of another panel's module. A module that repeats `grind_tracker.rs`'s or `item_card.rs`'s window plumbing is a finding.
