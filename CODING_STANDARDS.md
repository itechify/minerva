# Coding standards

Read at review, not while implementing: `AGENTS.md` holds the rules every change follows, and this file the judgement calls a diff is checked against. A finding cites the rule and the lines.

## Logic lives in the core

Anything testable without an OS belongs in `crates/minerva-core`: a decision taken from `PanelState` (who takes the keyboard, whether a window is clickable, whether a reading is stale), a cache or retry policy, parsing, formatting shown text, arithmetic on game data. `src-tauri` only looks windows up, captures and renders (`AGENTS.md`, ADR 0001). A shell diff that compares, filters, times or formats game data is a finding, named with the core type it moves into.

## A test helper is written once

A helper that a test file defines and another under `crates/minerva-core/tests/` already defines (a game window, a settings value, a title reading, a made-up world, a sample's name) moves to `tests/common`. The check: the same body in two test files, or a name and signature that a helper in `tests/common` already has. A short name two files each use for their own setup (`started`, `read`) isn't one.

## One panel window adapter

A new panel window in `src-tauri` is a `WindowSpec` driving the shared `PanelWindow` (keep-on-top, click-through, drag and resize, the height clamp), not a copy of another panel's module. A module that repeats `grind_tracker.rs`'s or `item_card.rs`'s window plumbing is a finding.

## The docs follow the change

A diff that changes how a surface looks updates its entry in `DESIGN.md`. One that changes an earlier ADR's decision adds `amended by ADR-NNNN (<what>)` to that ADR's `status:` line. A diff that does neither is a finding naming the entry or the ADR.

## Player-facing text reads like a person wrote it

List every player-facing string the diff adds or changes: HTML text, `title`, `alt` and `aria-label`, TS strings drawn in a window, the FAQ, the home page, Discord posts. A string showing the `humanizer` skill's tells, or written in the shorthand of `GLOSSARY.md`, the ADRs and doc comments ("Off, it asks for none."), is a finding with a rewrite.

## What the core fills, a window draws

A `PanelState` field the diff adds that the TS mirror declares but no renderer reads is a finding.

## A reader is tested beyond its footage

A new or changed reader's samples test also feeds its samples scaled to the 1280×720 UI Size (about 0.64) and asserts that it reads fewer lines or none, never another reading. Its samples cover at least three backgrounds, one of them busy. New files under `samples/` are looked over for names other than the made-up ones `samples/README.md` allows.
