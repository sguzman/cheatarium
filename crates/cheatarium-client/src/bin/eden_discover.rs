//! Read-only Eden game-directory discovery and Cheatarium Switch candidate matching.
//! Does not open game images, decrypt NCA metadata, run Eden, or write emulator state.
use cheatarium_client::{external, load_game_candidates, verify_platform_distribution, GameIndex};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_ENTRIES: usize = 50_000;
const MAX_FILES: usize = 20_000;
const MAX_DEPTH: usize = 5;
const MAX_CACHE_TITLE_BYTES: u64 = 4096;

#[derive(Debug, Serialize)]
struct LibraryFile {
    path: String,
    title_id: Option<String>,
    title_id_evidence: &'static str,
    /// A game-file suffix alone does not prove that this file is a base game.
    file_type: String,
}

#[derive(Debug, Serialize)]
struct CachedTitle {
    title_id: String,
    title_name: Option<String>,
    evidence: &'static str,
    installed_game_verified: bool,
}

#[derive(Debug, Serialize)]
struct GameRoot {
    path: String,
    deep_scan: bool,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct BuildCandidate {
    build_id: String,
    candidate_key: String,
    source_ids: Vec<String>,
    code_fields: usize,
    /// This is *always* false: no executable was opened to verify its build.
    installed_build_verified: bool,
}

#[derive(Debug, Serialize)]
struct CheatMatch {
    title_id: String,
    origin: &'static str,
    archived_build_candidates: Vec<BuildCandidate>,
    external_reference_total: usize,
    external_sampled: usize,
    external_sample_incomplete: bool,
    external_sources_sample: Vec<String>,
    /// External files are only pinned metadata; they are not locally usable codes.
    external_is_link_only: bool,
    compatible_with_installed_game: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u32,
    format: &'static str,
    read_only: bool,
    game_file_bytes_read: u32,
    emulator_files_written: u32,
    config_path: String,
    config_found: bool,
    cache_dir: String,
    cache_found: bool,
    cached_titles: Vec<CachedTitle>,
    game_roots: Vec<GameRoot>,
    game_files: Vec<LibraryFile>,
    title_id_matches: Vec<CheatMatch>,
    unresolved_game_files: usize,
    limitations: Vec<&'static str>,
    warnings: Vec<String>,
}

fn is_title_id(value: &str) -> bool {
    value.len() == 16 && value.starts_with("01") && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn title_id_in_filename(name: &str) -> Option<String> {
    let b = name.as_bytes();
    let mut found = None;
    for start in 0..=b.len().saturating_sub(16) {
        if b.len() < 16 {
            break;
        }
        let end = start + 16;
        if start > 0 && b[start - 1].is_ascii_hexdigit() {
            continue;
        }
        if end < b.len() && b[end].is_ascii_hexdigit() {
            continue;
        }
        let Ok(raw) = std::str::from_utf8(&b[start..end]) else {
            continue;
        };
        if !is_title_id(raw) {
            continue;
        }
        let id = raw.to_ascii_uppercase();
        if found.as_ref().is_some_and(|prev| prev != &id) {
            return None; // ambiguous filename: never pick the first match
        }
        found = Some(id);
    }
    found
}

fn normalize_root(value: &str) -> PathBuf {
    let value = value.trim().trim_matches('"');
    if value == "~" {
        return env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(rest);
    }
    PathBuf::from(value)
}

/// QSettings array under [Paths]: gamedirs\1\path, gamedirs\1\deep_scan.
/// Support '/' separators as well as the normal Qt backslash serialization.
fn configured_game_dirs(contents: &str) -> Vec<(PathBuf, bool)> {
    let mut in_paths = false;
    let mut entries: BTreeMap<usize, (Option<String>, bool)> = BTreeMap::new();
    let mut legacy = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_paths = line.eq_ignore_ascii_case("[Paths]");
            continue;
        }
        if !in_paths || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().replace('\\', "/");
        let value = value.trim();
        if key == "gameListRootDir" && value != "." {
            legacy = Some(value.to_owned());
        }
        let fields: Vec<&str> = key.split('/').collect();
        if fields.len() != 3 || fields[0] != "gamedirs" {
            continue;
        }
        let Ok(index) = fields[1].parse::<usize>() else {
            continue;
        };
        if index == 0 || index > 1000 {
            continue;
        }
        let item = entries.entry(index).or_insert((None, false));
        match fields[2] {
            "path" => item.0 = Some(value.to_owned()),
            "deep_scan" => item.1 = value.eq_ignore_ascii_case("true") || value == "1",
            _ => {}
        }
    }
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for (_index, (path, deep_scan)) in entries {
        let Some(path) = path else {
            continue;
        };
        if matches!(path.as_str(), "SDMC" | "SysNAND" | "UserNAND") {
            continue;
        }
        if path.trim().is_empty() {
            continue;
        }
        let path = normalize_root(&path);
        if seen.insert(path.clone()) {
            result.push((path, deep_scan));
        }
    }
    if result.is_empty() {
        if let Some(path) = legacy {
            let path = normalize_root(&path);
            if !path.as_os_str().is_empty() {
                result.push((path, false));
            }
        }
    }
    result
}

fn game_suffix(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if matches!(ext.as_str(), "nsp" | "xci" | "nca" | "nro" | "nsz" | "xcz") {
        Some(ext)
    } else {
        None
    }
}

/// Only filesystem directory entries are examined; game-file contents are never opened.
/// Never follows symlinks, even when deep-scan is enabled.
fn scan_root(
    root: &Path,
    deep: bool,
    out: &mut Vec<LibraryFile>,
    scanned: &mut usize,
) -> Result<(), ErrBox> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let mut entries = fs::read_dir(&dir)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            *scanned += 1;
            if *scanned > MAX_ENTRIES {
                return Err("Eden directory scan entry limit reached".into());
            }
            let kind = fs::symlink_metadata(entry.path())?.file_type();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if deep && depth < MAX_DEPTH {
                    stack.push((entry.path(), depth + 1));
                }
                continue;
            }
            if !kind.is_file() {
                continue;
            }
            let Some(file_type) = game_suffix(&entry.path()) else {
                continue;
            };
            if out.len() >= MAX_FILES {
                return Err("Eden game-file count limit reached".into());
            }
            let filename = entry.file_name().to_string_lossy().into_owned();
            let id = title_id_in_filename(&filename);
            out.push(LibraryFile {
                path: entry.path().to_string_lossy().into_owned(),
                title_id_evidence: if id.is_some() {
                    "filename_label_only"
                } else {
                    "unresolved_no_rom_read"
                },
                title_id: id,
                file_type,
            });
        }
    }
    Ok(())
}

fn exact_build_key(candidate_key: &str, id: &str) -> Option<String> {
    let prefix = format!("title-id-{}-build-", id.to_ascii_lowercase());
    let rest = candidate_key.strip_prefix(&prefix)?;
    if rest.len() != 16 || !rest.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(rest.to_ascii_uppercase())
}

fn archived_builds(index: &GameIndex, id: &str) -> Vec<BuildCandidate> {
    let mut matches: Vec<_> = index
        .candidates
        .iter()
        .filter_map(|candidate| {
            Some(BuildCandidate {
                build_id: exact_build_key(&candidate.key, id)?,
                candidate_key: candidate.key.clone(),
                source_ids: candidate.source_ids.clone(),
                code_fields: candidate.code_fields,
                installed_build_verified: false,
            })
        })
        .collect();
    matches.sort_by(|a, b| (&a.build_id, &a.candidate_key).cmp(&(&b.build_id, &b.candidate_key)));
    matches
}

/// Eden Qt caches loader-derived title IDs in game_list/<TITLEID>.appname.txt.
/// Cached data can survive game removal: never treat it as current installation.
fn cached_title_id(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".appname.txt")?;
    if !is_title_id(stem) {
        return None;
    }
    Some(stem.to_ascii_uppercase())
}

fn cache_titles(root: &Path, warnings: &mut Vec<String>) -> Vec<CachedTitle> {
    let mut result = Vec::new();
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return result;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        warnings.push("eden_game_cache_not_a_plain_directory".to_owned());
        return result;
    }
    let Ok(dir) = fs::read_dir(root) else {
        warnings.push("eden_game_cache_unreadable".to_owned());
        return result;
    };
    let mut names = Vec::new();
    for entry in dir {
        if names.len() >= MAX_ENTRIES {
            warnings.push("eden_game_cache_entry_limit_reached".to_owned());
            return result;
        }
        let Ok(entry) = entry else {
            warnings.push("eden_game_cache_entry_unreadable".to_owned());
            continue;
        };
        names.push(entry);
    }
    names.sort_by_key(|entry| entry.file_name());
    let mut seen = BTreeSet::new();
    for entry in names {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = cached_title_id(&name) else { continue; };
        if !seen.insert(id.clone()) {
            warnings.push(format!("duplicate_eden_cache_title_id: {id}"));
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else { continue; };
        if !metadata.is_file() || metadata.len() > MAX_CACHE_TITLE_BYTES {
            warnings.push(format!("eden_cache_title_skipped_nonregular_or_oversized: {id}"));
            continue;
        }
        let read_title = (|| -> std::io::Result<Option<String>> {
            let mut data = Vec::new();
            fs::File::open(entry.path())?
                .take(MAX_CACHE_TITLE_BYTES + 1)
                .read_to_end(&mut data)?;
            if data.len() as u64 > MAX_CACHE_TITLE_BYTES {
                return Ok(None);
            }
            Ok(String::from_utf8(data).ok().and_then(|s| {
                let cleaned = s.trim_matches('\0').trim().to_owned();
                if cleaned.is_empty() || cleaned.chars().any(char::is_control) {
                    None
                } else {
                    Some(cleaned)
                }
            }))
        })();
        match read_title {
            Ok(title_name) => result.push(CachedTitle {
                title_id: id,
                title_name,
                evidence: "eden_game_list_cache_filename_and_optional_title",
                installed_game_verified: false,
            }),
            Err(_) => warnings.push(format!("eden_cache_title_unreadable: {id}")),
        }
    }
    result
}

fn default_cache_dir() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|x| PathBuf::from(x).join(".cache")))
        .unwrap_or_default();
    base.join("eden/game_list")
}

fn default_config() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("eden/qt-config.ini");
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".config/eden/qt-config.ini")
}

fn run() -> Result<(), ErrBox> {
    let mut db = PathBuf::from("generated/v1");
    let mut config = default_config();
    let mut cache_dir = default_cache_dir();
    let mut extra_roots = Vec::new();
    let mut explicit_ids = BTreeSet::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--config" => config = PathBuf::from(args.next().ok_or("--config requires a path")?),
            "--cache-dir" => cache_dir = PathBuf::from(args.next().ok_or("--cache-dir requires a path")?),
            "--game-dir" => extra_roots.push((
                PathBuf::from(args.next().ok_or("--game-dir requires a path")?),
                false,
            )),
            "--title-id" => {
                let id = args
                    .next()
                    .ok_or("--title-id requires a 16-digit hexadecimal ID")?
                    .to_ascii_uppercase();
                if !is_title_id(&id) {
                    return Err(format!("Invalid Switch title ID: {id}").into());
                }
                explicit_ids.insert(id);
            }
            "--json" => {}
            "--help" | "-h" => {
                println!("Read-only Eden discovery:\n  cheatarium-eden-discover [--db generated/v1] [--config ~/.config/eden/qt-config.ini] [--cache-dir ~/.cache/eden/game_list] [--game-dir DIR] [--title-id 0100000000010000] [--json]\nNo ROM contents or emulator files are changed.");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    verify_platform_distribution(&db, "switch")?;
    let index = load_game_candidates(&db, "switch")?;
    let mut report = Report {
        schema_version: 1,
        format: "cheatarium-eden-readonly-discovery-v1",
        read_only: true,
        game_file_bytes_read: 0,
        emulator_files_written: 0,
        config_path: config.to_string_lossy().into_owned(),
        config_found: config.is_file(),
        cache_dir: cache_dir.to_string_lossy().into_owned(),
        cache_found: cache_dir.is_dir(),
        cached_titles: Vec::new(),
        game_roots: Vec::new(),
        game_files: Vec::new(),
        title_id_matches: Vec::new(),
        unresolved_game_files: 0,
        limitations: vec![
            "Filenames and configured folder listings are not authenticated Eden game-library identities.",
            "Switch game containers are not opened or decrypted; Title IDs absent from filenames remain unresolved.",
            "Build IDs listed are cheat-source candidates, not verified installed executable Build IDs.",
            "Eden cached Title IDs and names may refer to removed games and are not verified currently installed.",
            "External references are metadata-only; rights and compatibility are not established.",
            "No cheats are installed or activated.",
        ],
        warnings: Vec::new(),
    };
    let mut roots = if report.config_found {
        configured_game_dirs(&fs::read_to_string(&config)?)
    } else {
        Vec::new()
    };
    if !report.config_found {
        report.warnings.push(
            "eden_qt_config_not_found: supply --config for portable/custom installations".into(),
        );
    }
    roots.extend(extra_roots);
    report.cached_titles = cache_titles(&cache_dir, &mut report.warnings);
    let mut seen_roots = BTreeSet::new();
    let mut scanned = 0usize;
    for (root, deep_scan) in roots {
        if !seen_roots.insert(root.clone()) {
            continue;
        }
        let kind = fs::symlink_metadata(&root).map(|m| m.file_type());
        let status = match kind {
            Ok(kind) if kind.is_dir() && !kind.is_symlink() => "scanned",
            Ok(_) => "not_a_plain_directory",
            Err(_) => "unavailable",
        };
        if status == "scanned" {
            if let Err(e) = scan_root(&root, deep_scan, &mut report.game_files, &mut scanned) {
                report.warnings.push(format!(
                    "directory_scan_incomplete: {}: {e}",
                    root.display()
                ));
            }
        } else {
            report
                .warnings
                .push(format!("game_directory_{status}: {}", root.display()));
        }
        report.game_roots.push(GameRoot {
            path: root.to_string_lossy().into_owned(),
            deep_scan,
            status,
        });
    }
    report.game_files.sort_by(|a, b| a.path.cmp(&b.path));
    report.unresolved_game_files = report
        .game_files
        .iter()
        .filter(|item| item.title_id.is_none())
        .count();
    let mut ids = explicit_ids.clone();
    let mut filename_ids = BTreeSet::new();
    for item in &report.game_files {
        if let Some(id) = &item.title_id {
            filename_ids.insert(id.clone());
            ids.insert(id.clone());
        }
    }
    let mut cached_ids = BTreeSet::new();
    for item in &report.cached_titles {
        cached_ids.insert(item.title_id.clone());
        ids.insert(item.title_id.clone());
    }
    let external_coverage = external::switch_title_coverage(&db, &ids)?;
    for id in ids {
        let refs = external_coverage.get(&id).ok_or("Missing external coverage result")?;
        report.title_id_matches.push(CheatMatch {
            origin: if explicit_ids.contains(&id) {
                "explicit_lookup"
            } else if filename_ids.contains(&id) && cached_ids.contains(&id) {
                "filename_and_eden_cache"
            } else if filename_ids.contains(&id) {
                "filename_only"
            } else {
                "eden_cache_only_unverified_installed"
            },
            archived_build_candidates: archived_builds(&index, &id),
            title_id: id,
            external_reference_total: refs.total,
            external_sampled: refs.sampled,
            external_sample_incomplete: refs.total > refs.sampled,
            external_sources_sample: refs.source_ids.clone(),
            external_is_link_only: true,
            compatible_with_installed_game: false,
        });
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("cheatarium-eden-discover: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_qt_array_and_respects_deep_scan_and_pseudodirs() {
        let ini = "[General]\ngamedirs\\1\\path=/ignore\n[Paths]\ngamedirs\\size=4\ngamedirs\\1\\path=/switch/games\ngamedirs\\1\\deep_scan=true\ngamedirs\\2\\path=SDMC\ngamedirs\\3\\path=/switch/other\ngamedirs\\4\\path=/switch/games\n";
        let dirs = configured_game_dirs(ini);
        assert_eq!(
            dirs,
            vec![
                (PathBuf::from("/switch/games"), true),
                (PathBuf::from("/switch/other"), false)
            ]
        );
    }

    #[test]
    fn only_accepts_unique_complete_id_tokens() {
        assert_eq!(
            title_id_in_filename("The Legend [01006bb00c6f0000].nsp").as_deref(),
            Some("01006BB00C6F0000")
        );
        assert_eq!(
            title_id_in_filename("Game_0100000000010000.xci").as_deref(),
            Some("0100000000010000")
        );
        assert!(title_id_in_filename("01000000000100001.xci").is_none());
        assert!(title_id_in_filename("Title with no ID.nsp").is_none());
        assert!(title_id_in_filename("0100000000010000 and 01006BB00C6F0000.nsp").is_none());
        assert!(title_id_in_filename("ABCDEF1234567890.nsp").is_none());
    }

    #[test]
    fn matches_only_exact_build_candidate_key() {
        assert_eq!(
            exact_build_key(
                "title-id-0100000000010000-build-b424be150a8e7d78",
                "0100000000010000"
            )
            .as_deref(),
            Some("B424BE150A8E7D78")
        );
        assert!(exact_build_key(
            "title-id-0100152000022000-build-b424be150a8e7d78",
            "0100000000010000"
        )
        .is_none());
        assert!(
            exact_build_key("title-id-0100000000010000-build-short", "0100000000010000").is_none()
        );
    }

    #[test]
    fn recognizes_only_exact_eden_cache_title_labels() {
        assert_eq!(
            cached_title_id("0100000000010000.appname.txt").as_deref(),
            Some("0100000000010000")
        );
        assert!(cached_title_id("0100000000010000.jpeg").is_none());
        assert!(cached_title_id("0100000000010000.pv.txt").is_none());
        assert!(cached_title_id("foo0100000000010000.appname.txt").is_none());
        assert!(cached_title_id("0100000000010000.appname.txt.bak").is_none());
    }

    #[test]
    fn reads_small_cache_labels_without_claiming_installation() {
        let path = env::temp_dir().join(format!(
            "cheatarium-eden-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("0100000000010000.appname.txt"), b"Super Mario Odyssey").unwrap();
        fs::write(path.join("0100152000022000.jpeg"), b"not read").unwrap();
        fs::write(path.join("ignore.appname.txt"), b"ignored").unwrap();
        let mut warnings = Vec::new();
        let games = cache_titles(&path, &mut warnings);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].title_name.as_deref(), Some("Super Mario Odyssey"));
        assert_eq!(games[0].title_id, "0100000000010000");
        assert!(!games[0].installed_game_verified);
        assert!(warnings.is_empty());
        fs::remove_dir_all(&path).unwrap();
    }

    #[test]
    fn filters_extensions_and_rejects_other_files() {
        assert_eq!(game_suffix(Path::new("a.NSP")).as_deref(), Some("nsp"));
        assert!(game_suffix(Path::new("prod.keys")).is_none());
        assert!(game_suffix(Path::new("notes.txt")).is_none());
    }
}
