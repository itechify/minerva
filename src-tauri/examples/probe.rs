//! Read-only smoke test of the shell's adapters without any UI: find the "MapleStory"
//! window, capture its whole client area a few times, as the app does while it looks for the
//! Minimap Title (ADR 0021), and print where the title was found, what the recogniser read
//! (the street name and map name, before they are snapped to a map) and what the core shows. It only reads window geometry and screen pixels (ADR 0001), and fetches
//! OSMS Data Explorer's file set once (saved for a day, ADR 0008).
//!
//! `cargo run -p minerva --example probe [reads] [save.png]`
//! With a path, the last client area captured is saved there for inspection; as a sample in
//! `samples/minimap` it is read exactly as the probe read it.
//!
//! `cargo run -p minerva --example probe record <out-dir> <name>=<x>,<y>,<w>,<h>@<hz> ... [--seconds <n>]`
//! Record mode: capture each named rectangle of the game's client area (physical px,
//! relative to its top-left corner) at its own rate, until stopped (Ctrl+C) or for `n`
//! seconds, saving every crop as `<out-dir>/<name>/<ms>.png`. `<ms>` is zero-padded
//! milliseconds since the recording started, so names sort in time order. For example
//! `exp-bar=1015,1150,200,22@1 pickup-log=1630,830,285,135@10`. Nothing is read or
//! fetched: it only finds the game window and copies screen pixels.
//!
//! `cargo run -p minerva --example probe item <out-dir> "<item name>" [--desc <tag>] [--delay <s>]`
//! Item mode, for `samples/item-tooltip`: count down (3 s by default) while the player
//! hovers the item in the game, then save the whole client area as
//! `<out-dir>/<item-name>@<x>,<y>[--<tag>].png`, where `(x, y)` is the mouse pointer in
//! the client area (physical px) and the name is lower case with anything but letters
//! and digits as `-`. Nothing is read or fetched: it only finds the game window, reads
//! the pointer position and copies screen pixels.
//!
//! `cargo run -p minerva --example probe osms`
//! OSMS mode: fetch OSMS Data Explorer's live file set and the Item Card's item files
//! (whatever is saved) and read them in full with the core's readers, reporting what they
//! hold, or what didn't read. Run it by hand when OSMS changes and before a release. No
//! game window is needed.
//!
//! `cargo run -p minerva --example probe ceilings <out.json> [--osms <osms-set.json>]`
//! Ceilings mode, for the Map Leaderboards' cap (ADR 0015): fetch OSMS Data Explorer's
//! live file set (or read the `osms-set.json` given), read it with the core's reader, and
//! write each map's solo EXP/h ceiling, by its 9-digit ID, to `out.json`: the table the
//! site's Worker loads (`site/worker/osms-ceilings.json`). A map with no Map Summary, or a
//! ceiling of 0, is left out. No game window is needed.
//!
//! `cargo run -p minerva --example probe replay <report.zip> [--all] [--osms <osms-set.json>] [--items <osms-items.json>]`
//! Replay mode, for a Bug Report (ADR 0009): print what it holds and its timeline (from 10
//! minutes before it was saved, every line the core made something of before that; with
//! `--all`, every line), then feed its inputs to a new core and say whether it goes
//! through the same states. The report has no OSMS set, only its fingerprint: the replay
//! fetches OSMS's live one, or reads the one given (an `osms-set.json` from the cache of
//! the PC the report came from). So with the Item Card's OSMS item files (ADR 0010),
//! fetched live only if the run had them, or read from the `osms-items.json` given. No game
//! window is needed.
//!
//! `cargo run -p minerva --example probe update-manifest <out> [--bundle-dir <dir>] [--base-url <url>]`
//! Update manifest mode, for releasing (`docs/releasing.md`): write the updater's
//! `latest.json` to `out` for the version in `src-tauri/tauri.conf.json`, published now,
//! with the setup `.exe` and the MSI each at its GitHub Release address (or under the
//! `--base-url` given, for the update smoke test's local server) and with its `.sig` (from
//! `nsis/` and `msi/` under the bundle folder, `target/release/bundle` by default). It refuses a bare `windows-x86_64` key, and a `.sig` that is missing or was signed as
//! another version. No game window is needed.

use minerva_core::{Minerva, GAME_WINDOW_TITLE, PortalKind, Rect, TitleReader};
use minerva_lib::{capture, capture_title, gamewin, source::WebSource};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("record") {
        if let Err(e) = record(&args[1..]) {
            eprintln!("{e}");
            eprintln!("usage: probe record <out-dir> <name>=<x>,<y>,<w>,<h>@<hz> ... [--seconds <n>]");
            std::process::exit(2);
        }
        return;
    }
    if args.first().map(String::as_str) == Some("osms") {
        if let Err(e) = osms() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if args.first().map(String::as_str) == Some("ceilings") {
        if let Err(e) = ceilings(&args[1..]) {
            eprintln!("{e}");
            eprintln!("usage: probe ceilings <out.json> [--osms <osms-set.json>]");
            std::process::exit(2);
        }
        return;
    }
    if args.first().map(String::as_str) == Some("update-manifest") {
        if let Err(e) = update_manifest(&args[1..]) {
            eprintln!("{e}");
            eprintln!("usage: probe update-manifest <out> [--bundle-dir <dir>] [--base-url <url>]");
            std::process::exit(2);
        }
        return;
    }
    if args.first().map(String::as_str) == Some("replay") {
        if let Err(e) = replay(&args[1..]) {
            eprintln!("{e}");
            eprintln!("usage: probe replay <report.zip> [--all] [--osms <osms-set.json>]");
            std::process::exit(2);
        }
        return;
    }
    if args.first().map(String::as_str) == Some("item") {
        if let Err(e) = item(&args[1..]) {
            eprintln!("{e}");
            eprintln!("usage: probe item <out-dir> \"<item name>\" [--desc <tag>] [--delay <seconds>]");
            std::process::exit(2);
        }
        return;
    }
    let mut args = args.into_iter();
    let reads: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(5);
    let save = args.next();
    // Its own cache, outside the app's: the probe has no Tauri path resolver.
    let cache = std::env::temp_dir().join("minerva-probe-cache");
    let mut minerva = Minerva::new(WebSource::new(cache).expect("HTTP client"));
    // The core goes by the wall clock to tell whether the set saved is a day old.
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    minerva.on_wall_clock(since_epoch.as_millis() as u64);
    let mut reader = TitleReader::bundled();
    let title = minerva.panel_state().game_window_title.clone();
    gamewin::look_for(&title);
    let Some(client) = gamewin::find_game_client() else {
        println!("No visible \"{title}\" window found.");
        return;
    };
    println!("Game client area: {client:?}");
    let start = Instant::now();
    let mut last = None;
    for _ in 0..reads {
        let t = Instant::now();
        match capture_title(client, start.elapsed().as_millis() as u64) {
            Ok(capture) => {
                let reading = reader.read(&capture);
                last = Some(capture.frame);
                let ms = t.elapsed().as_millis();
                match (&reading.lines, reading.found) {
                    (Some(lines), Some(found)) => println!("{ms:>4} ms  read {:?} / {:?} at {found:?}", lines.street, lines.map),
                    (None, Some(found)) => println!("{ms:>4} ms  found at {found:?}, unreadable"),
                    _ => println!("{ms:>4} ms  no minimap title found"),
                }
                let state = minerva.on_reading(reading);
                println!("         {:?} {:?} not sure: {}", state.recognition, state.map, state.not_sure);
            }
            Err(e) => println!("capture failed: {e}"),
        }
        std::thread::sleep(Duration::from_millis(333));
    }
    if let (Some(path), Some(f)) = (save, last) {
        let img = image::RgbaImage::from_raw(f.width, f.height, f.rgba).expect("frame size");
        img.save(&path).expect("save png");
        println!("saved {path}");
    }
}

/// Fetch OSMS's live set, read it in full, and report its counts.
fn osms() -> Result<(), String> {
    let files = minerva_lib::osms::fetch_live()?;
    let sizes = [&files.maps, &files.portals, &files.monsters, &files.map_manifest, &files.lookups];
    for (name, text) in minerva_core::osms::OsmsFiles::NAMES.iter().zip(sizes) {
        println!("{name:>18}: {} KB", text.len() / 1024);
    }
    let listed = serde_json::from_str::<serde_json::Value>(&files.maps).ok().and_then(|maps| maps["total"].as_u64());
    let set = minerva_core::osms::read(&files).map_err(|e| format!("OSMS's set doesn't read: {e}"))?;
    let maps: Vec<_> = set.maps().collect();
    let mut regions: Vec<(&str, usize)> = Vec::new();
    for (entry, _) in &maps {
        match regions.iter_mut().find(|(region, _)| *region == entry.region) {
            Some((_, n)) => *n += 1,
            None => regions.push((&entry.region, 1)),
        }
    }
    let portals = || maps.iter().flat_map(|(_, data)| &data.portals);
    let kinds = |kind: PortalKind| portals().filter(|p| p.kind == kind).count();
    let hidden: Vec<_> = maps.iter().flat_map(|(_, data)| &data.hidden_portals).collect();
    let mut mobs: Vec<&str> = maps.iter().flat_map(|(_, data)| data.mobs.iter().map(|m| m.id.as_str())).collect();
    mobs.sort_unstable();
    mobs.dedup();
    println!("Read in full.");
    println!(
        "Maps: {} in the map list (of {} OSMS lists; a map with no name or street is left out)",
        maps.len(),
        listed.map_or("?".into(), |n| n.to_string())
    );
    for (region, n) in regions {
        println!("  {region}: {n}");
    }
    println!("  with mobs and a Map Summary: {}", maps.iter().filter(|(_, data)| data.summary.is_some()).count());
    println!("  with a nearest town: {}", maps.iter().filter(|(_, data)| data.nearest_town.is_some()).count());
    let marks: Vec<&str> = set.marks().map(|(mark, _)| mark).collect();
    println!(
        "  with a Map Mark: {} ({} marks: {})",
        maps.iter().filter(|(entry, _)| entry.mark.is_some()).count(),
        marks.len(),
        marks.join(", ")
    );
    println!(
        "Portals to other maps: {} ({} visible, {} Doors, {} hidden, {} Quest)",
        portals().count(),
        kinds(PortalKind::Visible),
        kinds(PortalKind::Door),
        kinds(PortalKind::Hidden),
        kinds(PortalKind::Quest)
    );
    println!("Hidden Portal rows: {} (to a Hidden Street: {})", hidden.len(), hidden.iter().filter(|p| p.to_hidden_street).count());
    println!("Mobs spawning: {}", mobs.len());

    let files = minerva_lib::osms_items::fetch_live()?;
    let sizes = [&files.items, &files.cash_shop];
    for (name, text) in minerva_core::osms_items::ItemFiles::NAMES.iter().zip(sizes) {
        println!("{name:>18}: {} KB", text.len() / 1024);
    }
    let items = minerva_core::osms_items::read(&files).map_err(|e| format!("OSMS's item files don't read: {e}"))?;
    let counts = items.counts();
    println!("Item files read in full.");
    println!(
        "Items: {} ({} with a shop price, {} a shop won't buy, {} Cash items)",
        counts.items, counts.shop_priced, counts.not_sold, counts.cash
    );
    Ok(())
}

/// `<out.json> [--osms <osms-set.json>]`: write each map's solo EXP/h ceiling, by its
/// 9-digit ID, one map to a line.
fn ceilings(args: &[String]) -> Result<(), String> {
    let mut rest = args.iter();
    let out = rest.next().ok_or("no file to write")?;
    let mut set_file = None;
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--osms" => set_file = Some(rest.next().ok_or("--osms needs a file")?),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let files = match set_file {
        Some(file) => {
            let json = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
            serde_json::from_slice::<minerva_core::osms::OsmsFiles>(&json).map_err(|e| format!("{file}: {e}"))?
        }
        None => minerva_lib::osms::fetch_live()?,
    };
    let set = minerva_core::osms::read(&files).map_err(|e| format!("OSMS's set doesn't read: {e}"))?;
    let ceilings = set.solo_ceilings();
    let rows: Vec<String> = ceilings.iter().map(|(id, solo)| format!("  \"{id}\": {solo}")).collect();
    std::fs::write(out, format!("{{\n{}\n}}\n", rows.join(",\n"))).map_err(|e| format!("{out}: {e}"))?;
    println!("Wrote the solo ceilings of {} maps (of {} in the map list) to {out}", ceilings.len(), set.maps().count());
    Ok(())
}

/// `<out> [--bundle-dir <dir>] [--base-url <url>]`: write `latest.json` from the built installers' `.sig`s.
fn update_manifest(args: &[String]) -> Result<(), String> {
    use minerva_lib::update_manifest::{installers, manifest};
    let mut rest = args.iter();
    let out = rest.next().ok_or("no file to write")?;
    let mut bundle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/bundle");
    let mut base = None;
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--bundle-dir" => bundle = PathBuf::from(rest.next().ok_or("--bundle-dir needs a folder")?),
            "--base-url" => base = Some(rest.next().ok_or("--base-url needs an address")?.as_str()),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let conf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let json = std::fs::read(&conf).map_err(|e| format!("{}: {e}", conf.display()))?;
    let conf: serde_json::Value = serde_json::from_slice(&json).map_err(|e| format!("tauri.conf.json: {e}"))?;
    let version = conf["version"].as_str().ok_or("tauri.conf.json has no version")?;
    let mut found = installers(version);
    for installer in &mut found {
        let sig = bundle.join(installer.folder).join(format!("{}.sig", installer.file));
        installer.signature = match std::fs::read_to_string(&sig) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("{}: {e}", sig.display())),
        };
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let written = manifest(version, now, base, &found).map_err(|why| format!("{}: {why}", bundle.display()))?;
    std::fs::write(out, written).map_err(|e| format!("{out}: {e}"))?;
    let files: Vec<&str> = found.iter().map(|i| i.file.as_str()).collect();
    println!("Wrote {out} for Minerva {version}: {}", files.join(" and "));
    Ok(())
}

/// `<report.zip> [--all] [--osms <osms-set.json>] [--items <osms-items.json>]`
fn replay(args: &[String]) -> Result<(), String> {
    use minerva_core::bug_report;
    let mut rest = args.iter();
    let path = rest.next().ok_or("no Bug Report")?;
    let (mut all, mut set_file, mut items_file) = (false, None, None);
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--all" => all = true,
            "--osms" => set_file = Some(rest.next().ok_or("--osms needs a file")?),
            "--items" => items_file = Some(rest.next().ok_or("--items needs a file")?),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let zip = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let recording = bug_report::read(&zip)?;
    let report = &recording.report;
    println!("Minerva {} on {}, display scale {}", report.version, report.os, report.display_scale);
    println!("Saved {} ({} ms after launch); {} lines, {} crops", report.saved_at, report.saved_at_ms, recording.lines.len(), recording.crops.len());
    if report.truncated {
        println!("The run outlasted a day: the trace starts at {} ms, after the lines that set the run up", report.trace_from_ms);
    }
    for map in &report.maps {
        let ceiling = map.summary.map_or("no Map Summary".to_string(), |s| format!("solo ceiling {} EXP/h", s.exp_per_hour.solo));
        println!("  map {} · {} ({}): {ceiling}", map.street_name, map.map_name, map.id);
    }
    println!();
    let from = if all { 0 } else { report.saved_at_ms.saturating_sub(10 * 60_000) };
    for line in recording.timeline(from) {
        println!("{line}");
    }
    println!();
    let files = match set_file {
        Some(file) => {
            let json = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
            Some(serde_json::from_slice::<minerva_core::osms::OsmsFiles>(&json).map_err(|e| format!("{file}: {e}"))?)
        }
        None => match minerva_lib::osms::fetch_live() {
            Ok(files) => Some(files),
            Err(e) => {
                println!("OSMS's set couldn't be fetched ({e}): replaying without it.");
                None
            }
        },
    };
    if let Some(files) = &files {
        let fingerprint = bug_report::fingerprint(files);
        let same = report.osms_sets.contains(&fingerprint);
        println!("OSMS's set {fingerprint}: {}", if same { "one the run took" } else { "not one the run took, so maps may replay otherwise" });
    }
    // The Item Card's item files, only if the run had them: none are asked for otherwise.
    let answers: Vec<&bug_report::Answer> = recording.lines.iter().flat_map(|line| &line.answers).collect();
    let had = |had: fn(&bug_report::Answer) -> bool| answers.iter().any(|answer| had(answer));
    let items = match items_file {
        Some(file) => {
            let json = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
            Some(serde_json::from_slice::<minerva_core::osms_items::ItemFiles>(&json).map_err(|e| format!("{file}: {e}"))?)
        }
        None if had(|a| matches!(a, bug_report::Answer::Items { answer: Ok(_) } | bug_report::Answer::SavedItems { saved: Some(_) })) => {
            match minerva_lib::osms_items::fetch_live() {
                Ok(files) => Some(files),
                Err(e) => {
                    println!("OSMS's item files couldn't be fetched ({e}): replaying without them.");
                    None
                }
            }
        }
        None => None,
    };
    let replay = bug_report::replay_with(&recording, bug_report::ReplayData { osms: files, items });
    match replay.divergence {
        None => println!("Replayed {} lines: the core went through the same states.", replay.lines),
        Some(divergence) => {
            println!("Replayed {} lines: it diverged at line {} (from 0).", replay.lines, divergence.index);
            println!("  recorded: {}", divergence.recorded.as_deref().unwrap_or("(past the end)"));
            println!("  replayed: {}", divergence.replayed.as_deref().unwrap_or("(past the end)"));
        }
    }
    Ok(())
}

/// `<out-dir> "<item name>" [--desc <tag>] [--delay <seconds>]`
fn item(args: &[String]) -> Result<(), String> {
    let mut rest = args.iter();
    let out = PathBuf::from(rest.next().ok_or("no output directory")?);
    let name = rest.next().ok_or("no item name")?;
    let (mut desc, mut delay) = (None, 3.0_f64);
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--desc" => desc = Some(slug(rest.next().ok_or("--desc needs a tag")?)),
            "--delay" => {
                delay = rest.next().and_then(|s| s.parse().ok()).filter(|s: &f64| *s >= 0.0).ok_or("--delay needs seconds")?;
            }
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    let item = slug(name);
    if item.is_empty() || desc.as_ref().is_some_and(String::is_empty) {
        return Err("the item name and tag need letters or digits".into());
    }

    gamewin::look_for(GAME_WINDOW_TITLE);
    gamewin::find_game_client().ok_or(format!("No visible \"{GAME_WINDOW_TITLE}\" window found."))?;
    println!("Hover {name:?} in the game so its tooltip shows.");
    let start = Instant::now();
    let total = Duration::from_secs_f64(delay);
    while let Some(left) = total.checked_sub(start.elapsed()).filter(|left| !left.is_zero()) {
        print!("{}… ", left.as_secs_f64().ceil());
        let _ = std::io::stdout().flush();
        std::thread::sleep(left.min(Duration::from_secs(1)));
    }
    println!();

    // The pointer first: the tooltip follows it, and the capture comes right after.
    let at = gamewin::cursor_position().ok_or("no pointer position")?;
    let client = gamewin::find_game_client().ok_or(format!("No visible \"{GAME_WINDOW_TITLE}\" window found."))?;
    let (x, y) = (at.x - client.x, at.y - client.y);
    println!("Game client area: {client:?}; pointer at client ({x}, {y})");
    if x < 0 || y < 0 || x >= client.width as i32 || y >= client.height as i32 {
        return Err("the pointer is outside the game's client area: nothing saved".into());
    }
    let rgba = capture::capture_rgba(client.x, client.y, client.width as i32, client.height as i32)?;
    let img = image::RgbaImage::from_raw(client.width, client.height, rgba).ok_or("capture size")?;
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let tag = desc.map(|d| format!("--{d}")).unwrap_or_default();
    let path = out.join(format!("{item}@{x},{y}{tag}.png"));
    img.save(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("saved {}", path.display());
    Ok(())
}

/// A file-name form of a name: lower case, apostrophes dropped, anything else but
/// letters and digits as one `-` ("Hero's Gladius" -> "heros-gladius").
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().filter(|c| !matches!(c, '\'' | '’')) {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// How often record mode looks for the game window again, so crops follow it.
const FOLLOW_WINDOW: Duration = Duration::from_millis(250);
/// The fastest rate a region may be recorded at.
const MAX_HZ: f64 = 30.0;

/// One named rectangle of the client area, recorded at its own rate.
struct Track {
    name: String,
    /// Relative to the client area's top-left corner.
    rect: Rect,
    period: Duration,
    dir: PathBuf,
    /// When the next crop is due.
    next: Instant,
    saved: u64,
    /// Crops skipped because the recording fell behind (or the game window was gone).
    dropped: u64,
}

/// `name=x,y,w,h@hz`
fn parse_track(spec: &str, out: &Path, start: Instant) -> Result<Track, String> {
    let bad = || format!("bad region \"{spec}\": expected <name>=<x>,<y>,<w>,<h>@<hz>");
    let (name, rest) = spec.split_once('=').ok_or_else(bad)?;
    let (rect, hz) = rest.split_once('@').ok_or_else(bad)?;
    let nums: Vec<u32> = rect.split(',').map(|n| n.trim().parse()).collect::<Result<_, _>>().map_err(|_| bad())?;
    let [x, y, width, height] = nums[..] else { return Err(bad()) };
    let hz: f64 = hz.parse().map_err(|_| bad())?;
    let valid_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_name || width == 0 || height == 0 || !(hz > 0.0 && hz <= MAX_HZ) {
        return Err(format!("{}: names are letters, digits, - and _; sizes above 0; rates up to {MAX_HZ} Hz", bad()));
    }
    Ok(Track {
        name: name.into(),
        rect: Rect { x, y, width, height },
        period: Duration::from_secs_f64(1.0 / hz),
        dir: out.join(name),
        next: start,
        saved: 0,
        dropped: 0,
    })
}

fn record(args: &[String]) -> Result<(), String> {
    let (out, rest) = args.split_first().ok_or("no output directory")?;
    let out = PathBuf::from(out);
    let start = Instant::now();
    let mut seconds = None;
    let mut tracks = Vec::new();
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        if arg == "--seconds" {
            let n: f64 = rest.next().and_then(|s| s.parse().ok()).ok_or("--seconds needs a number")?;
            seconds = Some(Duration::from_secs_f64(n));
        } else {
            tracks.push(parse_track(arg, &out, start)?);
        }
    }
    if tracks.is_empty() {
        return Err("no regions to record".into());
    }

    gamewin::look_for(GAME_WINDOW_TITLE);
    let mut client = gamewin::find_game_client().ok_or(format!("No visible \"{GAME_WINDOW_TITLE}\" window found."))?;
    println!("Game client area: {client:?}");
    for t in &tracks {
        let r = t.rect;
        if r.x + r.width > client.width || r.y + r.height > client.height {
            return Err(format!("region {} doesn't fit the {}x{} client area", t.name, client.width, client.height));
        }
        std::fs::create_dir_all(&t.dir).map_err(|e| format!("{}: {e}", t.dir.display()))?;
    }

    // PNGs are encoded on their own thread, so a slow save never delays a capture.
    let (crops, to_save) = mpsc::channel::<(PathBuf, u32, u32, Vec<u8>)>();
    let writer = std::thread::spawn(move || {
        for (path, width, height, rgba) in to_save {
            let img = image::RgbaImage::from_raw(width, height, rgba).expect("crop size");
            // Written whole, then renamed: stopping mid-save leaves no broken PNG.
            let part = path.with_extension("png.part");
            if let Err(e) = img.save_with_format(&part, image::ImageFormat::Png).and_then(|_| {
                std::fs::rename(&part, &path).map_err(image::ImageError::IoError)
            }) {
                eprintln!("saving {} failed: {e}", path.display());
            }
        }
    });

    println!("Recording to {} (Ctrl+C to stop)", out.display());
    let mut next_follow = start + FOLLOW_WINDOW;
    let mut game_found = true;
    let mut next_report = start + Duration::from_secs(1);
    loop {
        let now = Instant::now();
        if seconds.is_some_and(|s| now - start >= s) {
            break;
        }
        if now >= next_follow {
            next_follow = now + FOLLOW_WINDOW;
            let found = gamewin::find_game_client();
            if found.is_some() != game_found {
                println!("{}", if found.is_some() { "Game window back." } else { "Game window gone: skipping crops." });
            }
            game_found = found.is_some();
            client = found.unwrap_or(client);
        }
        for t in tracks.iter_mut().filter(|t| now >= t.next) {
            // Behind by more than a period: those crops are dropped, not taken late.
            let behind = (now - t.next).as_secs_f64() / t.period.as_secs_f64();
            t.dropped += behind as u64;
            t.next += t.period.mul_f64(behind.floor() + 1.0);
            if !game_found {
                t.dropped += 1;
                continue;
            }
            let r = t.rect;
            let (x, y) = (client.x + r.x as i32, client.y + r.y as i32);
            match capture::capture_rgba(x, y, r.width as i32, r.height as i32) {
                Ok(rgba) => {
                    let ms = (now - start).as_millis();
                    let path = t.dir.join(format!("{ms:08}.png"));
                    let _ = crops.send((path, r.width, r.height, rgba));
                    t.saved += 1;
                }
                Err(e) => {
                    eprintln!("capture of {} failed: {e}", t.name);
                    t.dropped += 1;
                }
            }
        }
        if now >= next_report {
            next_report += Duration::from_secs(1);
            let counts: Vec<String> =
                tracks.iter().map(|t| format!("{} {} (dropped {})", t.name, t.saved, t.dropped)).collect();
            println!("{:>4} s  {}", (now - start).as_secs(), counts.join("  "));
        }
        let due = tracks.iter().map(|t| t.next).chain([next_follow, next_report]).min().unwrap_or(now);
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    drop(crops);
    let _ = writer.join();
    for t in &tracks {
        println!("{}: {} crops saved, {} dropped", t.name, t.saved, t.dropped);
    }
    Ok(())
}
