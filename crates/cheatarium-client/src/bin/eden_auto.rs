//! Discover candidate base-game Title IDs from Eden's own registered folders and
//! its game-list cache. This on-demand path only reads directory entries and Qt
//! registration settings: no NSP contents, decryption, keys, daemon, or GUI.
use super::{is_title_id, ErrBox};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 50_000;
const MAX_DEPTH: usize = 64;
const MAX_TITLES: usize = 1024;

#[derive(Debug, serde::Serialize)]
pub(super) struct Discovery {
    pub(super) titles: Vec<String>,
    pub(super) cache_titles: usize,
    pub(super) filename_titles: usize,
    pub(super) recognized_game_files: usize,
    pub(super) skipped_symlinks: usize,
    pub(super) scanned_directories: usize,
    pub(super) registered_roots: usize,
    pub(super) warnings: Vec<String>,
}

pub(super) fn default_cache_dir() -> PathBuf {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_default()
        .join("eden/game_list")
}

fn is_base_id(s: &str) -> bool {
    if !is_title_id(s) { return false; }
    u64::from_str_radix(s, 16).is_ok_and(|id| id & 0x1fff == 0)
}

fn labeled_title(filename: &str) -> Option<String> {
    let bytes = filename.as_bytes();
    let mut found = None;
    if bytes.len() < 16 { return None; }
    for offset in 0..=bytes.len()-16 {
        let end = offset+16;
        if (offset > 0 && bytes[offset-1].is_ascii_hexdigit())
            || (end < bytes.len() && bytes[end].is_ascii_hexdigit()) {
            continue;
        }
        let Ok(value) = std::str::from_utf8(&bytes[offset..end]) else { continue };
        if !is_base_id(value) { continue; }
        let candidate = value.to_ascii_uppercase();
        if found.as_ref().is_some_and(|prior| prior != &candidate) { return None; }
        found = Some(candidate);
    }
    found
}

fn root_path(value: &str) -> PathBuf {
    let raw = value.trim().trim_matches('"');
    if let Some(rest) = raw.strip_prefix("~/") {
        return env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(rest);
    }
    PathBuf::from(raw)
}

fn configured_roots(contents: &str) -> Vec<(PathBuf, bool)> {
    let mut section = "";
    let mut roots = BTreeMap::<usize, (Option<String>, bool)>::new();
    for raw in contents.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = if line.eq_ignore_ascii_case("[UI]") { "UI" }
                else if line.eq_ignore_ascii_case("[Paths]") { "Paths" } else { "" };
            continue;
        }
        if section.is_empty() || line.starts_with('#') || line.starts_with(';') { continue; }
        let Some((key, value)) = line.split_once('=') else { continue };
        let normalized = key.trim().replace('\\', "/");
        let key = if section == "UI" {
            let Some(key) = normalized.strip_prefix("Paths/") else { continue };
            key
        } else { normalized.as_str() };
        let parts: Vec<_> = key.split('/').collect();
        if parts.len() != 3 || parts[0] != "gamedirs" { continue; }
        let Ok(index) = parts[1].parse::<usize>() else { continue };
        if index == 0 || index > 1000 { continue; }
        let value = value.trim();
        let entry = roots.entry(index).or_insert((None, false));
        match parts[2] {
            "path" => entry.0 = Some(value.to_owned()),
            "deep_scan" => entry.1 = value == "1" || value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    let mut seen = BTreeSet::new();
    roots.into_values().filter_map(|(path, deep)| {
        let path = path?;
        if path.is_empty() || matches!(path.as_str(), "SDMC" | "SysNAND" | "UserNAND") {return None;}
        let parsed = root_path(&path);
        if !parsed.is_absolute() || !seen.insert(parsed.clone()) {return None;}
        Some((parsed, deep))
    }).collect()
}

/// Fail rather than returning a partial, silently misleading inventory.
pub(super) fn discover(config: &Path, cache: &Path) -> Result<Discovery, ErrBox> {
    let mut out = Discovery {
        titles: Vec::new(),
        cache_titles: 0, filename_titles: 0, recognized_game_files: 0,
        skipped_symlinks: 0, scanned_directories: 0, registered_roots: 0,
        warnings: Vec::new()
    };
    let mut ids = BTreeSet::<String>::new();
    let mut cache_ids = BTreeSet::<String>::new();
    let mut filename_ids = BTreeSet::<String>::new();
    if let Ok(meta) = fs::symlink_metadata(cache) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err("Eden game-list cache is not a plain directory".into());
        }
        let mut entries = 0usize;
        for entry in fs::read_dir(cache)? {
            entries += 1;
            if entries > MAX_ENTRIES {return Err("Eden cache directory entry limit".into());}
            let entry = entry?;
            let meta = fs::symlink_metadata(entry.path())?;
            if meta.file_type().is_symlink() || !meta.is_file() {continue;}
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".pv.txt").or_else(|| name.strip_suffix(".appname.txt"));
            let Some(stem) = stem else { continue };
            if !is_base_id(stem) { continue; }
            cache_ids.insert(stem.to_ascii_uppercase());
        }
    } else {
        out.warnings.push("Eden game-list cache absent".into());
    }
    ids.extend(cache_ids.iter().cloned());
    if !config.is_file() { return Err("Eden qt-config.ini is missing; cannot discover registered game directories".into()); }
    let metadata = fs::symlink_metadata(config)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 8*1024*1024 {
        return Err("Eden Qt config must be an ordinary bounded file".into());
    }
    let roots = configured_roots(&fs::read_to_string(config)?);
    if roots.is_empty() && ids.is_empty() {
        return Err("No Eden registered game directories or game-list cache Title IDs found".into());
    }
    out.registered_roots = roots.len();
    let mut entry_count = 0usize;
    for (root, deep_scan) in roots {
        let meta = fs::symlink_metadata(&root)?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(format!("Eden registered root is not a plain directory: {}", root.display()).into());
        }
        let mut stack = vec![(root, 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            out.scanned_directories += 1;
            for entry in fs::read_dir(&dir)? {
                entry_count += 1;
                if entry_count > MAX_ENTRIES {return Err("Eden directory scan limit reached".into());}
                let entry = entry?;
                let meta = fs::symlink_metadata(entry.path())?;
                if meta.file_type().is_symlink() {
                    out.skipped_symlinks += 1;
                    continue;
                }
                if meta.is_dir() {
                    if deep_scan {
                        if depth >= MAX_DEPTH {return Err("Eden directory scan depth limit reached".into());}
                        stack.push((entry.path(), depth+1));
                    }
                    continue;
                }
                if !meta.is_file() {continue;}
                let extension = entry.path().extension().and_then(|x| x.to_str())
                    .map(str::to_ascii_lowercase);
                if !matches!(extension.as_deref(), Some("nsp" | "xci" | "nca" | "nro" | "nsz" | "xcz")) {
                    continue;
                }
                out.recognized_game_files += 1;
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(id) = labeled_title(&name) {
                    filename_ids.insert(id);
                }
            }
        }
    }
    ids.extend(filename_ids.iter().cloned());
    if ids.len() > MAX_TITLES {return Err("Too many discovered Eden base Title IDs".into());}
    out.titles = ids.into_iter().collect();
    out.cache_titles = cache_ids.len();
    out.filename_titles = filename_ids.len();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_real_eden_ui_registration_and_nested_base_ids() {
        let ini = "[UI]\nPaths\\gamedirs\\size=1\nPaths\\gamedirs\\1\\path=/tmp/switch-roms\nPaths\\gamedirs\\1\\deep_scan=true\n";
        assert_eq!(configured_roots(ini), vec![(PathBuf::from("/tmp/switch-roms"), true)]);
        assert_eq!(labeled_title("Smash DLC [01006A8000161001].nsp"),None);
        assert_eq!(labeled_title("Mario Update [0100152000022800].nsp"),None);
        assert_eq!(labeled_title("Mario Kart 8 [0100152000022000].nsp"),Some("0100152000022000".into()));
        assert_eq!(labeled_title("E [0100000000010000] [0100152000022000].nsp"),None);
    }
    #[test]
    fn discovers_cached_games_and_filenames_without_rom_reads() {
        let root = std::env::temp_dir().join(format!("cheatarium-auto-test-{}-{:?}",std::process::id(),std::thread::current().id()));
        fs::create_dir_all(root.join("cache")).unwrap();
        fs::create_dir_all(root.join("roms/deep/a/b")).unwrap();
        fs::write(root.join("cache/0100152000022000.pv.txt"),"Update").unwrap();
        fs::write(root.join("roms/deep/a/b/Game [0100000000010000].nsp"),"not a real NSP").unwrap();
        fs::write(root.join("roms/deep/a/b/DLC [01006A8000161001].nsp"),"not a real NSP").unwrap();
        let config = root.join("qt-config.ini");
        fs::write(&config,format!("[UI]\nPaths\\gamedirs\\size=1\nPaths\\gamedirs\\1\\path={}\nPaths\\gamedirs\\1\\deep_scan=true\n",root.join("roms").display())).unwrap();
        let found=discover(&config,&root.join("cache")).unwrap();
        assert_eq!(found.titles,vec!["0100000000010000","0100152000022000"]);
        assert_eq!(found.recognized_game_files,2);
        fs::remove_dir_all(root).unwrap();
    }
}
