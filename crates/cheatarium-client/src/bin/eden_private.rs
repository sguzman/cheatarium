//! Adapt the already privately staged, exact-static-NSP-build Hamlet cheat files
//! to the existing Eden preview/staging/disabled-by-default installation pipeline.
//! No private cheat code or owned game inventory is committed to Git.
use super::{is_build_id, is_title_id, Bundle, ErrBox, IndexedFile};
use cheatarium_client::{Code, Provenance};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_CATALOG_BYTES: u64 = 2_000_000;
const MAX_PRIVATE_CHEAT_BYTES: u64 = 1_000_000;
const MAX_NATIVE_SECTIONS: usize = 2048;

#[derive(Deserialize)]
struct Manifest {
    format: String,
    upstream: Upstream,
    games: Vec<Game>,
}
#[derive(Deserialize)]
struct Upstream {
    snapshot_commit: String,
}
#[derive(Deserialize)]
struct Game {
    title_id: String,
    title: String,
    hamlet_builds: Vec<Build>,
    static_nsp_observation: StaticNsp,
}
#[derive(Deserialize)]
struct Build {
    build_id: String,
    strict_native_hex_sections: usize,
}
#[derive(Deserialize)]
struct StaticNsp {
    status: String,
    cheat_build_id: String,
    pinned_native_cheat_sections_matched: usize,
}

pub(super) fn default_root() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".local/share/cheatarium/private-switch-curation")
}

fn regular_file(path: &Path, max_size: u64) -> Result<(), ErrBox> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err("Private source path must be normalized and absolute".into());
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part);
        let meta = fs::symlink_metadata(&current)?;
        if meta.file_type().is_symlink() {
            return Err("Private source path contains a symlink".into());
        }
        if current == path {
            if !meta.is_file() || meta.len() > max_size {
                return Err("Private source must be a bounded regular file".into());
            }
        } else if !meta.is_dir() {
            return Err("Private source ancestor is not a directory".into());
        }
    }
    Ok(())
}

/// Only bracketed headings with nonempty native hexadecimal code bodies.
/// Parser is intentionally as strict as the original private curator.
fn parse_sections(input: &str) -> Result<Vec<(String, String)>, ErrBox> {
    let mut sections = Vec::<(String, String)>::new();
    let mut name: Option<String> = None;
    let mut payload = Vec::<String>::new();
    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(heading) = name.take() {
                if payload.is_empty() {
                    return Err("Private cheat has an empty native-code section".into());
                }
                sections.push((heading, payload.join("\n")));
                payload.clear();
            }
            let heading = &line[1..line.len() - 1];
            if heading.is_empty()
                || heading.len() > 128
                || heading.contains(['[', ']', '\r', '\n'])
                || heading.chars().any(char::is_control)
            {
                return Err("Private cheat section has an unsafe heading".into());
            }
            name = Some(heading.to_owned());
        } else {
            if name.is_none() {
                return Err("Private cheat code appears before a section heading".into());
            }
            let words: Vec<_> = line.split_ascii_whitespace().collect();
            if words.is_empty()
                || words.len() > 5
                || words
                    .iter()
                    .any(|w| w.len() != 8 || !w.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err("Private cheat has non-native or malformed hex code".into());
            }
            payload.push(line.to_owned());
        }
        if sections.len() >= MAX_NATIVE_SECTIONS {
            return Err("Too many private native-code sections".into());
        }
    }
    if let Some(heading) = name {
        if payload.is_empty() {
            return Err("Private cheat has an empty trailing section".into());
        }
        sections.push((heading, payload.join("\n")));
    }
    if sections.is_empty() {
        return Err("Private cheat source has no native sections".into());
    }
    Ok(sections)
}

pub(super) fn load_exact(
    manifest_path: &Path,
    private_root: &Path,
    title_id: &str,
) -> Result<(Bundle, String), ErrBox> {
    if !is_title_id(title_id) {
        return Err("Private curation requires exact base-game Title ID".into());
    }
    // Resolve ordinary relative repo manifest paths, but require absolute private root.
    let manifest_file = if manifest_path.is_absolute() {
        manifest_path.to_path_buf()
    } else {
        env::current_dir()?.join(manifest_path)
    };
    regular_file(&manifest_file, MAX_CATALOG_BYTES)?;
    let manifest: Manifest = serde_json::from_slice(&fs::read(&manifest_file)?)?;
    if manifest.format != "cheatarium-user-switch-cheat-source-curation-v1"
        || manifest.upstream.snapshot_commit.len() != 40
        || !manifest
            .upstream
            .snapshot_commit
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("Invalid pinned private curation manifest".into());
    }
    let mut selected = manifest
        .games
        .into_iter()
        .filter(|g| g.title_id == title_id);
    let game = selected
        .next()
        .ok_or("Title ID absent from six-game private curation")?;
    if selected.next().is_some() {
        return Err("Duplicate curated game Title ID".into());
    }
    let build_id = &game.static_nsp_observation.cheat_build_id;
    if game.static_nsp_observation.status != "main_nso_extracted_from_local_nsp"
        || !is_build_id(build_id)
        || !build_id.bytes().all(|b| !b.is_ascii_lowercase())
    {
        return Err("No valid static NSP main-executable Build ID".into());
    }
    let expected = game
        .hamlet_builds
        .iter()
        .find(|entry| entry.build_id == *build_id)
        .map_or(0, |entry| entry.strict_native_hex_sections);
    if expected == 0
        || expected
            != game
                .static_nsp_observation
                .pinned_native_cheat_sections_matched
    {
        return Err("No pinned native exact-build cheats for this NSP (Smash needs external source integration)".into());
    }
    let path = private_root
        .join(&game.title_id)
        .join(format!("{build_id}.txt"));
    regular_file(&path, MAX_PRIVATE_CHEAT_BYTES)?;
    let sections = parse_sections(&fs::read_to_string(&path)?)?;
    if sections.len() != expected {
        return Err("Private staged native cheat count disagrees with pinned manifest".into());
    }
    let codes = sections
        .into_iter()
        .enumerate()
        .map(|(i, (description, code))| Code {
            ordinal: i + 1,
            description: Some(description),
            code: Some(code),
            source_enabled: false,
            verification: "lexical_native_hex_only_not_gameplay_verified".into(),
            role: Some("code".into()),
            native_fields: Vec::new(),
            snes_decode: None,
            composition: None,
        })
        .collect();
    let archive_path = format!("privately_staged/{}/{}.txt", game.title_id, build_id);
    let file = IndexedFile {
        id: format!("private-hamlet-{}-{}", game.title_id, build_id),
        title_hint: game.title,
        candidate_game_key: format!(
            "title-id-{}-build-{}",
            game.title_id.to_ascii_lowercase(),
            build_id.to_ascii_lowercase()
        ),
        identity_confidence: "private_manifest_static_nsp_not_eden_runtime".into(),
        raw_filename: format!("{build_id}.txt"),
        region_hint: None,
        format_hint: Some("atmosphere_native_switch_cheats".into()),
        declared_cheats: Some(expected),
        parse_warnings: Vec::new(),
        codes,
        provenance: Provenance {
            source_id: "hamlet_private_curated".into(),
            repository: "https://github.com/HamletDuFromage/switch-cheats-db".into(),
            revision: manifest.upstream.snapshot_commit,
            license: "third_party_unverified_code_local_private_only".into(),
            upstream_path: format!("cheats/{}.json", game.title_id),
            archive_path,
            git_blob_sha: "not_verified".into(),
        },
    };
    Ok((
        Bundle {
            schema_version: 1,
            platform: "switch".into(),
            game_identity_rule: "explicit_title_id_from_private_manifest".into(),
            compatibility_rule: "static_nsp_build_id_only_not_runtime_verified".into(),
            records: vec![file],
        },
        build_id.clone(),
    ))
}

/// Namespace private cheat add-ons so ordinary existing Eden mods with
/// matching human-readable names are preserved rather than overwritten.
/// The derived names are stable across retries; no install paths are reused
/// from the unscoped archive or from older user-managed cheats.
pub(super) fn preview_isolated(
    bundle: &Bundle,
    title: &str,
    build: &str,
    mods_root: &Path,
) -> Result<super::Preview, ErrBox> {
    let mut preview = super::preview_readable(bundle, title, build, mods_root)?;
    let title_root = mods_root.join(title);
    let mut names = std::collections::BTreeSet::new();
    for cheat in &mut preview.proposed_addons {
        let original = Path::new(&cheat.proposed_mod_path);
        let relative = original.strip_prefix(&title_root)
            .map_err(|_| "Private add-on path unexpectedly outside Title ID directory")?;
        let mut parts = relative.components();
        let addon = match parts.next() {
            Some(Component::Normal(value)) => value.to_str()
                .ok_or("Private add-on name is not valid Unicode")?,
            _ => return Err("Private add-on path has no regular name".into()),
        };
        let expected_file = format!("{build}.txt");
        if !matches!(parts.next(), Some(Component::Normal(x)) if x == "cheats")
            || !matches!(parts.next(), Some(Component::Normal(x)) if x == std::ffi::OsStr::new(expected_file.as_str()))
            || parts.next().is_some()
        {
            return Err("Private add-on path has unexpected layout".into());
        }
        let isolated = format!("Cheatarium - {addon}");
        if !names.insert(isolated.clone()) {
            return Err("Duplicate private add-on name after namespacing".into());
        }
        cheat.proposed_mod_path = title_root
            .join(isolated).join("cheats").join(format!("{build}.txt"))
            .to_string_lossy().into_owned();
    }
    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_malformed_native_hex_and_trailing_headers() {
        assert!(parse_sections("[Valid]\n04000000 12345678 00000001\n").is_ok());
        assert!(parse_sections("[Empty]\n").is_err());
        assert!(parse_sections("[Code]\nGARBAGE\n").is_err());
        assert!(parse_sections("04000000 12345678\n").is_err());
        assert!(parse_sections("[Nested [Heading]]\n04000000 12345678\n").is_err());
    }
    #[test]
    fn private_cache_loads_only_exact_static_build() {
        let base = std::env::temp_dir().join(format!(
            "cheatarium-private-adapter-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let title = "0100000000010000";
        let build = "B424BE150A8E7D78";
        let root = base.join("private");
        fs::create_dir_all(root.join(title)).unwrap();
        let manifest = base.join("manifest.json");
        fs::write(&manifest, format!(
            r#"{{"format":"cheatarium-user-switch-cheat-source-curation-v1","upstream":{{"snapshot_commit":"{}"}},"games":[{{"title_id":"{}","title":"Test game","hamlet_builds":[{{"build_id":"{}","strict_native_hex_sections":1}}],"static_nsp_observation":{{"status":"main_nso_extracted_from_local_nsp","cheat_build_id":"{}","pinned_native_cheat_sections_matched":1}}}}]}}"#,
            "a".repeat(40), title, build, build
        )).unwrap();
        fs::write(
            root.join(title).join(format!("{build}.txt")),
            "[Infinite Health]\n04000000 12345678 00000001\n",
        )
        .unwrap();
        let (bundle, id) = load_exact(&manifest, &root, title).unwrap();
        assert_eq!(id, build);
        assert_eq!(bundle.records.len(), 1);
        assert_eq!(bundle.records[0].codes.len(), 1);
        assert_eq!(
            bundle.records[0].provenance.source_id,
            "hamlet_private_curated"
        );
        assert!(!bundle.records[0].codes[0].source_enabled);
        let mod_root = base.join("eden/load");
        fs::create_dir_all(&mod_root).unwrap();
        let config = base.join("qt-config.ini");
        fs::write(&config, "[General]\nexample=true\n").unwrap();
        let preview = preview_isolated(&bundle, title, &id, &mod_root).unwrap();
        assert_eq!(preview.proposed_addons.len(), 1);
        assert_eq!(
            preview.source_filter,
            "private_pinned_hamlet_exact_static_nsp"
        );
        assert!(!preview.installation_allowed);
        // An older user-managed add-on with the same readable heading
        // must survive the new private installation unmodified.
        let legacy = mod_root.join(title).join("Infinite Health")
            .join("cheats").join(format!("{id}.txt"));
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "[Existing user mod]\\n04000000 11111111 22222222\\n").unwrap();
        let unchanged_user_mod = fs::read(&legacy).unwrap();
        let installed =
            super::super::eden_install::install(&bundle, &preview, &mod_root, &config).unwrap();
        assert_eq!(fs::read(&legacy).unwrap(), unchanged_user_mod);
        let report = serde_json::to_value(installed).unwrap();
        assert_eq!(report["activation_state"], "installed_disabled_by_default");
        assert_eq!(report["installed_build_id_verified"], false);
        assert_eq!(report["files_written"], 1);
        assert_eq!(report["existing_mods_overwritten"], false);
        let config_after = fs::read_to_string(&config).unwrap();
        assert!(config_after.contains("[DisabledAddOns]"));
        assert!(config_after.contains("Infinite Health"));
        assert!(mod_root
            .join(title)
            .join("Cheatarium - Infinite Health")
            .join("cheats")
            .join(format!("{id}.txt"))
            .is_file());
        assert!(config_after.contains("Cheatarium - Infinite Health"));
        assert!(super::super::eden_install::already_installed_unchanged(
            &bundle, &preview, &mod_root, &config,
        ).unwrap());
        // The same data cannot silently replace an existing live mod.
        assert!(
            super::super::eden_install::install(&bundle, &preview, &mod_root, &config,).is_err()
        );
        assert_eq!(fs::read_to_string(&config).unwrap(), config_after);
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn missing_exact_static_build_fails_closed() {
        let root = std::env::temp_dir().join(format!(
            "cheatarium-private-smash-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        let manifest = root.join("manifest.json");
        fs::write(&manifest, format!(
            r#"{{"format":"cheatarium-user-switch-cheat-source-curation-v1","upstream":{{"snapshot_commit":"{}"}},"games":[{{"title_id":"01006A800016E000","title":"Smash","hamlet_builds":[],"static_nsp_observation":{{"status":"main_nso_extracted_from_local_nsp","cheat_build_id":"CBD5A9B56EA859BA","pinned_native_cheat_sections_matched":0}}}}]}}"#,
            "b".repeat(40)
        )).unwrap();
        assert!(load_exact(&manifest, &root, "01006A800016E000").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
