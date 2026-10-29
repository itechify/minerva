//! Keeping the Personal Bests between launches: the core's [`SavedBests`] as pretty JSON
//! beside the settings (`%APPDATA%\com.itechify.minerva\personal-bests.json` on
//! Windows), read at startup and written whenever the core's bests revision changes, but
//! never over a file that was there and didn't read.

use crate::errors::error;
use crate::files::{GuardedJsonFile, JsonShape};
use minerva_core::{Minerva, SavedBests};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("personal-bests.json"))
}

/// The bests file, as this launch found it. A file saved in an earlier shape reads as the
/// core takes it (version 1's bests are Unassigned), and is saved over in this one; one
/// that was there but didn't read is what the core is told
/// ([`minerva_core::Minerva::bests_lost`]).
pub type BestsFile = GuardedJsonFile<SavedBests>;

impl JsonShape for SavedBests {
    const WHAT: &'static str = "personal bests";

    fn version(&self) -> u32 {
        self.version
    }

    fn in_a_known_shape(&self) -> bool {
        SavedBests::in_a_known_shape(self)
    }
}

/// Give `minerva`, its settings restored from `settings_path`, the bests this launch found:
/// `(file, bests)` from [`BestsFile::load`]. A file that was there but didn't read is told to
/// the core, which holds the Map Leaderboards' uploads of the Characters shown there, and the
/// settings saying so are saved at once: the bests file saved from now on reads at the next
/// launch, which wouldn't hold them again, and the settings are otherwise saved only when
/// something else changes them. The file, for saving the bests to.
pub fn restore(minerva: &mut Minerva, (file, bests): (BestsFile, SavedBests), settings_path: &Path) -> BestsFile {
    minerva.restore_bests(bests);
    if file.lost() {
        minerva.bests_lost();
        if let Err(e) = crate::settings::save(settings_path, &minerva.settings()) {
            error!("saving settings failed: {e}");
        }
    }
    file
}

#[cfg(test)]
mod tests {
    use super::*;
    use minerva_core::BESTS_VERSION;
    use std::fs;
    use minerva_core::{LeaderboardCharacter, MapBests, Settings, VisitBest};

    use std::task::Poll;

    /// `personal-bests.json` in a folder of `test`'s own under the temp folder, emptied
    /// first.
    fn file(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minerva-bests-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("personal-bests.json")
    }

    /// The bests a launch finds at `path`.
    fn load(path: &Path) -> SavedBests {
        BestsFile::load(path.to_path_buf()).1
    }

    /// A launch that finds no file at `path` saves `bests` there.
    fn save(path: &Path, bests: &SavedBests) -> std::io::Result<()> {
        BestsFile::load(path.to_path_buf()).0.save(bests)
    }

    /// Sunset Sky's two best Visits, one set before the shell told the wall clock.
    fn sunset_sky_bests() -> SavedBests {
        let visits = vec![
            VisitBest { exp_per_hour: 7200, set_at_unix_ms: Some(1_791_547_200_000), ..VisitBest::default() },
            VisitBest { exp_per_hour: 3600, set_at_unix_ms: None, ..VisitBest::default() },
        ];
        SavedBests { unassigned: vec![MapBests { id: "010003070".into(), visits }], ..SavedBests::default() }
    }

    #[test]
    fn saved_bests_load_as_they_were_saved() {
        let path = file("saved");
        save(&path, &sunset_sky_bests()).unwrap();
        assert_eq!(load(&path), sunset_sky_bests());
    }

    #[test]
    fn with_no_file_yet_there_are_no_bests() {
        let path = file("none");
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.with_extension("json.bad").exists());
    }

    #[test]
    fn an_unreadable_file_is_set_aside_and_there_are_no_bests() {
        let path = file("unreadable");
        let unreadable = r#"{ "version": 1, "maps": ["#;
        fs::write(&path, unreadable).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert_eq!(fs::read_to_string(path.with_extension("json.bad")).unwrap(), unreadable);
    }

    #[test]
    fn a_file_that_is_not_utf8_is_set_aside_and_there_are_no_bests() {
        // Re-saved as UTF-16 (PowerShell 5.1's `>`, say): a byte-order mark, then two
        // bytes a character.
        let path = file("utf-16");
        let json = serde_json::to_string_pretty(&sunset_sky_bests()).unwrap();
        let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain(json.encode_utf16().flat_map(u16::to_le_bytes)).collect();
        fs::write(&path, &utf16).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert_eq!(fs::read(path.with_extension("json.bad")).unwrap(), utf16);
    }

    #[test]
    fn a_file_that_cannot_be_read_is_set_aside_and_there_are_no_bests() {
        // A folder where the file should be: it is there, but reading it fails.
        let path = file("not-a-file");
        fs::create_dir(&path).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert!(path.with_extension("json.bad").is_dir());
    }

    #[test]
    fn a_file_set_aside_again_keeps_the_one_set_aside_before() {
        // A later version's bests, set aside by this one; then this launch's own, saved and
        // set aside by another; then a third.
        let path = file("again");
        let later = serde_json::to_string_pretty(&SavedBests { version: BESTS_VERSION + 1, ..sunset_sky_bests() }).unwrap();
        fs::write(&path, &later).unwrap();
        load(&path);
        fs::write(&path, r#"{ "version": 1, "maps": ["#).unwrap();
        load(&path);
        fs::write(&path, "third").unwrap();
        load(&path);

        assert_eq!(fs::read_to_string(path.with_extension("json.bad")).unwrap(), later);
        assert_eq!(fs::read_to_string(path.with_extension("json.2.bad")).unwrap(), r#"{ "version": 1, "maps": ["#);
        assert_eq!(fs::read_to_string(path.with_extension("json.3.bad")).unwrap(), "third");
    }

    /// `path` held open by another program that lets no one else read, move or delete it,
    /// while it is held.
    fn held(path: &Path) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new().read(true).share_mode(0).open(path).unwrap()
    }

    #[test]
    fn an_unreadable_file_that_cannot_be_set_aside_is_not_written_over() {
        let path = file("kept");
        let unreadable = r#"{ "version": 1, "maps": ["#;
        fs::write(&path, unreadable).unwrap();
        let holder = held(&path);
        let (launch, bests) = BestsFile::load(path.clone());
        assert_eq!(bests, SavedBests::default());
        drop(holder);

        // The launch's bests aren't saved over it.
        launch.save(&sunset_sky_bests()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), unreadable);
    }

    #[test]
    fn a_file_saved_before_characters_loads_with_every_visit_unassigned_and_is_saved_over_in_the_new_shape() {
        let path = file("version-1");
        let version_1 = r#"{
  "version": 1,
  "maps": [
    { "id": "010003070", "visits": [
      { "expPerHour": 7200, "setAtUnixMs": 1791547200000 },
      { "expPerHour": 3600, "setAtUnixMs": null }
    ] }
  ]
}"#;
        fs::write(&path, version_1).unwrap();
        let (launch, bests) = BestsFile::load(path.clone());
        assert_eq!(bests, SavedBests { version: 1, ..sunset_sky_bests() });
        assert!(!path.with_extension("json.bad").exists());

        // The core hands them back in the new shape, which the launch saves over it.
        launch.save(&sunset_sky_bests()).unwrap();
        assert_eq!(load(&path), sunset_sky_bests());
    }

    /// Whether a launch that finds `path` as it is now says its bests were lost.
    fn lost(path: &Path) -> bool {
        BestsFile::load(path.to_path_buf()).0.lost()
    }

    #[test]
    fn a_file_that_was_there_but_did_not_read_says_the_bests_were_lost() {
        // No file yet, a file that reads, and one from before Characters: nothing lost.
        let path = file("lost");
        assert!(!lost(&path));
        save(&path, &sunset_sky_bests()).unwrap();
        assert!(!lost(&path));
        fs::write(&path, r#"{ "version": 1, "maps": [] }"#).unwrap();
        assert!(!lost(&path));

        // Unreadable, set aside or not, or saved in a later shape: lost.
        fs::write(&path, r#"{ "version": 1, "maps": ["#).unwrap();
        assert!(lost(&path));
        let path = file("lost-kept");
        fs::write(&path, r#"{ "version": 1, "maps": ["#).unwrap();
        let holder = held(&path);
        assert!(lost(&path));
        drop(holder);
        let path = file("lost-later-shape");
        save(&path, &SavedBests { version: BESTS_VERSION + 1, ..sunset_sky_bests() }).unwrap();
        assert!(lost(&path));
    }

    /// A source with nothing: the core here never needs OSMS or MeowDB.
    struct NoSource;

    impl minerva_core::MapDataSource for NoSource {
        fn saved_osms_files(&mut self) -> Option<minerva_core::osms::SavedFiles> {
            None
        }
        fn fetch_osms_files(&mut self) -> Poll<Result<minerva_core::osms::OsmsFiles, minerva_core::SourceError>> {
            Poll::Pending
        }
        fn picture(&mut self, _: &str) -> Poll<Result<String, minerva_core::SourceError>> {
            Poll::Pending
        }
        fn item_search(&mut self, _: &str) -> Poll<Result<minerva_core::Fetched<Vec<minerva_core::ItemHit>>, minerva_core::SourceError>> {
            Poll::Pending
        }
        fn saved_item_files(&mut self) -> Option<minerva_core::osms_items::SavedItemFiles> {
            None
        }
        fn fetch_item_files(&mut self) -> Poll<Result<minerva_core::osms_items::ItemFiles, minerva_core::SourceError>> {
            Poll::Pending
        }
    }

    #[test]
    fn bests_lost_at_launch_hold_the_uploads_in_the_settings_saved_at_once() {
        // Egbert was shown on the Map Leaderboards, and the bests file doesn't read.
        let path = file("lost-held");
        fs::write(&path, r#"{ "version": 1, "maps": ["#).unwrap();
        let settings_path = path.with_file_name("settings.json");
        let egbert = LeaderboardCharacter { name: "Egbert".into(), shown: true, uploaded_at_unix_ms: None, held: false };
        crate::settings::save(&settings_path, &Settings { map_leaderboards: vec![egbert], ..Settings::default() }).unwrap();
        let mut minerva = Minerva::new(NoSource);
        minerva.restore(crate::settings::load(&settings_path));

        restore(&mut minerva, BestsFile::load(path.clone()), &settings_path);

        // On disk before anything else is: the bests file saved from now on reads at the next
        // launch, which wouldn't hold them again.
        let held: Vec<bool> = crate::settings::load(&settings_path).map_leaderboards.iter().map(|c| c.held).collect();
        assert_eq!(held, [true]);
    }

    #[test]
    fn a_file_saved_in_another_shape_is_set_aside_and_there_are_no_bests() {
        let path = file("another-shape");
        save(&path, &SavedBests { version: BESTS_VERSION + 1, ..sunset_sky_bests() }).unwrap();
        assert_eq!(load(&path), SavedBests::default());
        assert!(!path.exists());
        assert!(path.with_extension("json.bad").exists());
    }
}
