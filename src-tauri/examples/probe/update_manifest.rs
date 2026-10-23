//! The updater's `latest.json` (#124): the version, when it was published, and for each
//! installer its download address and its `.sig`. `probe update-manifest` reads the
//! files; what goes in the manifest, and what it refuses, is here.

use serde::Serialize;
use std::collections::BTreeMap;

/// Where a release's files are downloaded from: `<RELEASES>/vX.Y.Z/<file>`.
const RELEASES: &str = "https://github.com/itechify/minerva/releases/download";

/// One installer of a release, as the probe found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installer {
    /// The updater's key for it, `windows-x86_64-nsis` or `windows-x86_64-msi`.
    pub platform: String,
    /// The bundler's folder for it under `target/release/bundle`.
    pub folder: &'static str,
    /// The installer's file name, as it is uploaded.
    pub file: String,
    /// What its `.sig` holds, or `None` with no `.sig` beside it.
    pub signature: Option<String>,
}

/// The two installers of version `version`, with no signatures yet.
pub fn installers(version: &str) -> Vec<Installer> {
    let installer = |platform: &str, folder, file| Installer { platform: platform.into(), folder, file, signature: None };
    vec![
        installer(NSIS, "nsis", format!("Minerva_{version}_x64-setup.exe")),
        installer(MSI, "msi", format!("Minerva_{version}_x64_en-US.msi")),
    ]
}

const NSIS: &str = "windows-x86_64-nsis";
const MSI: &str = "windows-x86_64-msi";
/// The key the updater falls back to when it finds neither of the others.
const BARE: &str = "windows-x86_64";

/// `latest.json` for `version`, published at `now` (Unix seconds), or why it can't be.
/// The installers are downloaded from `base` (for the update smoke test's local server),
/// or with `None` from version `version`'s GitHub Release.
///
/// It lists exactly the setup `.exe` and the MSI, each signed as `version`. It never
/// lists a bare `windows-x86_64`: the portable `.exe` falls back to it, and would run
/// the setup it points at, installing a second copy.
pub fn manifest(version: &str, now: u64, base: Option<&str>, installers: &[Installer]) -> Result<String, String> {
    let base = match base {
        Some(base) => base.trim_end_matches('/').to_owned(),
        None => format!("{RELEASES}/v{version}"),
    };
    let mut platforms = BTreeMap::new();
    for installer in installers {
        let key = installer.platform.as_str();
        if key == BARE {
            return Err(format!(
                "refusing a bare {BARE} key: the portable .exe would take it and run the setup it points at"
            ));
        }
        if key != NSIS && key != MSI {
            return Err(format!("{key:?} isn't an installer the updater should be offered"));
        }
        let sig = format!("{}.sig", installer.file);
        let signature = installer.signature.as_deref().ok_or(format!("no {sig} beside {}", installer.file))?.trim();
        match signed_version(signature) {
            Some(signed) if signed == version => {}
            Some(signed) => return Err(format!("{sig} was signed as version {signed}, not {version}")),
            None => return Err(format!("{sig} says no version: it isn't a signature Tauri's CLI 2.12 or later made")),
        }
        let url = format!("{base}/{}", installer.file);
        if platforms.insert(key, Platform { signature, url }).is_some() {
            return Err(format!("{key} is listed twice"));
        }
    }
    for key in [NSIS, MSI] {
        if !platforms.contains_key(key) {
            return Err(format!("no {key} installer"));
        }
    }
    let manifest = Manifest { version, notes: "", pub_date: rfc3339(now), platforms };
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    Ok(json + "\n")
}

/// The version a `.sig`'s trusted comment binds it to: the `version:` field of its
/// tab-separated `key:value` fields. A `.sig` is the base64 of minisign's text form,
/// whose third line is `trusted comment: ...`.
fn signed_version(signature: &str) -> Option<String> {
    use base64::Engine;
    let text = base64::engine::general_purpose::STANDARD.decode(signature).ok()?;
    let text = String::from_utf8(text).ok()?;
    let comment = text.lines().find_map(|line| line.strip_prefix("trusted comment: "))?;
    comment.split('\t').find_map(|field| field.strip_prefix("version:")).map(str::to_owned)
}

/// Unix seconds as `yyyy-mm-ddThh:mm:ssZ`.
fn rfc3339(secs: u64) -> String {
    let (days, time) = (secs / 86_400, secs % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", time / 3600, time / 60 % 60, time % 60)
}

#[derive(Serialize)]
struct Manifest<'a> {
    version: &'a str,
    notes: &'a str,
    pub_date: String,
    platforms: BTreeMap<&'a str, Platform<'a>>,
}

#[derive(Serialize)]
struct Platform<'a> {
    signature: &'a str,
    url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    /// A `.sig` the Tauri CLI made, with a throwaway key, for `Minerva_0.13.0_x64-setup.exe`
    /// signed as version 0.13.0.
    const SETUP_0_13_0: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRRmFsajY5S1ErZEsvQm96RTZxZ053eURwcHBnMXZBNTByVGNaQytHNHFyWkJQNUxRK1pIK2JtQmtxZGRjT0c3am15anpvYW9rUXFrdTBmV2tROVFWV1dkekZ4MmJCS3c0PQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkyNjQ1MzU5CWZpbGU6TWluZXJ2YV8wLjEzLjBfeDY0LXNldHVwLmV4ZQl2ZXJzaW9uOjAuMTMuMApkcTFjMlFQemthYzQ2aHNxYU8ycitUcTU5UWs5Nm5DUXJIUU5HVmhCNXBwRjNncHIySFlaSE9Kdndod01KdCtKNEEvQVFGSFN0UVZ1N2tPMmdJQ0xEQT09Cg==";
    /// The same for `Minerva_0.12.0_x64_en-US.msi`, signed as version 0.12.0.
    const MSI_0_12_0: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRRmFsajY5S1ErZEtEKzlpenpkYzdsMURLMm5IRTNyYzdaaFlOb05LcndubW1GOVZMbVNyT2VZY3JIbGJEZlRCVnlpTERYVzJ3RE5IY3gxWDVFNG9DWVZFc3MwUHYzUVFzPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkyNjQ1NDE4CWZpbGU6TWluZXJ2YV8wLjEyLjBfeDY0X2VuLVVTLm1zaQl2ZXJzaW9uOjAuMTIuMApoZTc3bWViRWFPZjVhaFo4bmI1K0JUWGJ6RjNpZGl4aWpHWGhTTy9sVUpOTC9ja3NZaloyOVV4S2daWTRHMldRT1B3RDNHVlEwNGZwc2hWMzZXV3dCUT09Cg==";
    /// 2026-10-22T05:02:39Z.
    const NOW: u64 = 1_792_645_359;

    /// A `.sig` in the CLI's shape whose trusted comment is `comment`.
    fn signed(comment: &str) -> String {
        use base64::Engine;
        let text = format!(
            "untrusted comment: signature from tauri secret key\nRUQFalj69KQ+dK/BozE6qgNwyDpppg1vA50rTcZC+G4qrZBP5LQ+ZH+bmBkqddcOG7jmyjzoaokQqku0fWkQ9QVWWdzFx2bBKw4=\ntrusted comment: {comment}\ndq1c2QPzkac46hsqaO2r+Tq59Qk96nCQrHQNGVhB5ppF3gpr2HYZHOJvwhwMJt+J4A/AQFHStQVu7kO2gICLDA==\n"
        );
        base64::engine::general_purpose::STANDARD.encode(text)
    }

    /// Both installers of `version`, each signed as `signed_as`.
    fn built(version: &str, signed_as: &str) -> Vec<Installer> {
        installers(version)
            .into_iter()
            .map(|i| {
                let sig = signed(&format!("timestamp:1792645359\tfile:{}\tversion:{signed_as}", i.file));
                Installer { signature: Some(sig), ..i }
            })
            .collect()
    }

    #[test]
    fn the_manifest_lists_the_setup_exe_and_the_msi_each_with_its_address_and_signature() {
        let mut built = built("0.13.0", "0.13.0");
        built[0].signature = Some(SETUP_0_13_0.into());
        let msi = built[1].signature.clone().unwrap();
        let written: Value = serde_json::from_str(&manifest("0.13.0", NOW, None, &built).unwrap()).unwrap();
        assert_eq!(
            written,
            json!({
                "version": "0.13.0",
                "notes": "",
                "pub_date": "2026-10-22T05:02:39Z",
                "platforms": {
                    "windows-x86_64-nsis": {
                        "signature": SETUP_0_13_0,
                        "url": "https://github.com/itechify/minerva/releases/download/v0.13.0/Minerva_0.13.0_x64-setup.exe",
                    },
                    "windows-x86_64-msi": {
                        "signature": msi,
                        "url": "https://github.com/itechify/minerva/releases/download/v0.13.0/Minerva_0.13.0_x64_en-US.msi",
                    },
                },
            })
        );
    }

    #[test]
    fn for_the_smoke_test_the_installers_can_be_downloaded_from_a_local_server_instead() {
        let built = built("0.13.0", "0.13.0");
        for base in ["http://127.0.0.1:8000", "http://127.0.0.1:8000/"] {
            let written = manifest("0.13.0", NOW, Some(base), &built).unwrap();
            let written: Value = serde_json::from_str(&written).unwrap();
            assert_eq!(written["platforms"]["windows-x86_64-nsis"]["url"], "http://127.0.0.1:8000/Minerva_0.13.0_x64-setup.exe");
            assert_eq!(written["platforms"]["windows-x86_64-msi"]["url"], "http://127.0.0.1:8000/Minerva_0.13.0_x64_en-US.msi");
        }
    }

    #[test]
    fn the_installers_are_where_the_bundler_puts_them() {
        let found: Vec<_> = installers("0.13.0").into_iter().map(|i| (i.platform, i.folder, i.file)).collect();
        assert_eq!(
            found,
            [
                ("windows-x86_64-nsis".into(), "nsis", "Minerva_0.13.0_x64-setup.exe".into()),
                ("windows-x86_64-msi".into(), "msi", "Minerva_0.13.0_x64_en-US.msi".into()),
            ]
        );
    }

    #[test]
    fn a_bare_windows_key_is_refused_since_the_portable_would_run_the_setup_it_points_at() {
        let mut built = built("0.13.0", "0.13.0");
        let bare = Installer { platform: "windows-x86_64".into(), ..built[0].clone() };
        built.push(bare);
        let why = manifest("0.13.0", NOW, None, &built).unwrap_err();
        assert!(why.contains("windows-x86_64") && why.contains("portable"), "{why}");
    }

    #[test]
    fn a_manifest_without_both_installers_is_refused() {
        let built = built("0.13.0", "0.13.0");
        let why = manifest("0.13.0", NOW, None, &built[..1]).unwrap_err();
        assert!(why.contains("windows-x86_64-msi"), "{why}");
    }

    #[test]
    fn a_missing_sig_is_refused_by_its_name() {
        let mut built = built("0.13.0", "0.13.0");
        built[1].signature = None;
        let why = manifest("0.13.0", NOW, None, &built).unwrap_err();
        assert!(why.contains("Minerva_0.13.0_x64_en-US.msi.sig"), "{why}");
    }

    #[test]
    fn a_sig_made_for_another_version_is_refused() {
        let mut built = built("0.13.0", "0.13.0");
        built[1].signature = Some(MSI_0_12_0.into());
        let why = manifest("0.13.0", NOW, None, &built).unwrap_err();
        assert!(why.contains("0.12.0") && why.contains("Minerva_0.13.0_x64_en-US.msi.sig"), "{why}");
    }

    #[test]
    fn a_sig_bound_to_no_version_is_refused() {
        let mut built = built("0.13.0", "0.13.0");
        built[0].signature = Some(signed("timestamp:1792645359\tfile:Minerva_0.13.0_x64-setup.exe"));
        let why = manifest("0.13.0", NOW, None, &built).unwrap_err();
        assert!(why.contains("no version"), "{why}");
    }

    #[test]
    fn a_sig_that_is_not_a_signature_is_refused() {
        let mut built = built("0.13.0", "0.13.0");
        built[0].signature = Some("not base64!".into());
        assert!(manifest("0.13.0", NOW, None, &built).is_err());
    }

    #[test]
    fn a_sig_written_with_a_line_break_after_it_goes_in_without_one() {
        let mut built = built("0.13.0", "0.13.0");
        built[0].signature = Some(format!("{SETUP_0_13_0}\r\n"));
        let written: Value = serde_json::from_str(&manifest("0.13.0", NOW, None, &built).unwrap()).unwrap();
        assert_eq!(written["platforms"]["windows-x86_64-nsis"]["signature"], SETUP_0_13_0);
    }
}
