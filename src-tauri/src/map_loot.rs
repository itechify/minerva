//! Keeping the Map Loot waiting to be sent between launches (ADR 0033): the core's
//! [`SavedMapLoot`] as pretty JSON beside the settings (`%APPDATA%\com.itechify.minerva\map-loot.json`
//! on Windows), read at startup and written whenever the core's Map Loot revision changes, but
//! never over a file that was there and didn't read. Closing Minerva ends the Visit under way,
//! which the file holds as if it had ended, so it goes at the next launch.

use crate::files::{GuardedJsonFile, JsonShape};
use minerva_core::SavedMapLoot;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn path(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join("map-loot.json"))
}

/// The Map Loot file, as this launch found it.
pub type MapLootFile = GuardedJsonFile<SavedMapLoot>;

impl JsonShape for SavedMapLoot {
    const WHAT: &'static str = "map loot";

    fn version(&self) -> u32 {
        self.version
    }

    fn in_a_known_shape(&self) -> bool {
        SavedMapLoot::in_a_known_shape(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minerva_core::{LootItem, MapLootVisit};
    use std::fs;

    /// `map-loot.json` in a folder of `test`'s own under the temp folder, emptied first.
    fn file(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("minerva-map-loot-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("map-loot.json")
    }

    #[test]
    fn map_loot_saved_is_read_back_at_the_next_launch() {
        let path = file("round-trip");
        let (first, none) = MapLootFile::load(path.clone());
        assert_eq!(none, SavedMapLoot::default());

        let visit = MapLootVisit {
            name: "Egbert".into(),
            map: "010003070".into(),
            party: false,
            active_ms: 600_000,
            mesos: 3_000,
            items: vec![LootItem { name: "Red Potion".into(), count: 4 }],
        };
        let saved = SavedMapLoot { visits: vec![visit], ..SavedMapLoot::default() };
        first.save(&saved).unwrap();

        assert_eq!(MapLootFile::load(path).1, saved);
    }

    #[test]
    fn map_loot_saved_in_a_later_shape_is_set_aside() {
        let path = file("later-shape");
        fs::write(&path, br#"{"version":2,"visits":[]}"#).unwrap();

        let (_, map_loot) = MapLootFile::load(path.clone());

        assert_eq!(map_loot, SavedMapLoot::default());
        assert!(path.with_extension("json.bad").exists());
    }
}
