//! On-demand Eden cheat installation. Existing mods never overwritten.
//! Disable every new per-cheat addon in Eden's QSettings before creating it.
use super::{prepare_addons, Bundle, ErrBox, Preview};
use serde::Serialize;
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const MAX_INI_BYTES: u64 = 8 * 1024 * 1024;
const MAX_DISABLED_GROUPS: usize = 4096;
const MAX_DISABLED_NAMES: usize = 8192;

#[derive(Debug, Serialize)]
pub(super) struct Installed {
    schema_version: u32,
    format: &'static str,
    title_id: String,
    selected_build_id: String,
    installed_addons: Vec<String>,
    files_written: usize,
    eden_config_updated: bool,
    eden_config_backup_path: Option<String>,
    activation_state: &'static str,
    installed_build_id_verified: bool,
    compatible_cheats_verified: bool,
    existing_mods_overwritten: bool,
    emulator_game_files_written: usize,
    warning: &'static str,
}

fn validate_regular_ancestors(path: &Path, final_is_file: bool) -> Result<(), ErrBox> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err("Eden destination must be an absolute normalized path".into());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        let meta = fs::symlink_metadata(&current)?;
        if meta.file_type().is_symlink() {
            return Err("Eden destination/ancestor contains a symlink".into());
        }
        if current == path {
            if (final_is_file && !meta.is_file()) || (!final_is_file && !meta.is_dir()) {
                return Err("Eden destination has unexpected filesystem type".into());
            }
        } else if !meta.is_dir() {
            return Err("Eden destination ancestor is not a directory".into());
        }
    }
    Ok(())
}

/// Validate an absent first-use Eden load directory without following
/// symlinks. Create it only after the disabled settings are committed.
fn validate_mod_root_plan(path: &Path) -> Result<(), ErrBox> {
    if !path.is_absolute()
        || path == Path::new("/")
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err("Eden mod root must be a normalized absolute non-root path".into());
    }
    let mut current = PathBuf::new();
    let mut missing = false;
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                if missing || meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err("Eden mod root contains a symlink or non-directory ancestor".into());
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => missing = true,
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}

fn eden_not_running() -> Result<(), ErrBox> {
    // Eden rewrites qt-config.ini from its in-memory QSettings. Writing while
    // its process is running would risk lost settings and enabled-by-default mods.
    #[cfg(target_os = "linux")]
    {
        for item in fs::read_dir("/proc")? {
            let item = item?;
            let pid = item.file_name();
            if !pid.to_string_lossy().bytes().all(|b| b.is_ascii_digit())
                || pid.to_string_lossy() == std::process::id().to_string()
            {
                continue;
            }
            let Ok(comm) = fs::read_to_string(item.path().join("comm")) else {
                continue;
            };
            let proc_name = comm.trim().to_ascii_lowercase();
            if proc_name == "eden"
                || proc_name == "eden-qt"
                || proc_name.starts_with("eden-app")
                || proc_name.starts_with("eden-emu")
            {
                return Err("Close Eden completely before installing cheats; its running settings would overwrite the disabled-by-default configuration".into());
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct DisabledRow {
    title_id: Option<u64>,
    title_line: Option<usize>,
    count: Option<usize>,
    count_line: Option<usize>,
    names: BTreeMap<usize, String>,
}

/// Preserve all unrelated Qt settings and existing disabled names verbatim.
/// Refuse malformed arrays rather than reconstructing/losing the user's data.
pub(super) fn disabled_config(
    original: &str,
    title_id: &str,
    new_names: &[String],
) -> Result<String, ErrBox> {
    let title_number = u64::from_str_radix(title_id, 16)?;
    if new_names.is_empty() || new_names.len() > MAX_DISABLED_NAMES {
        return Err("No valid per-cheat addon names to disable".into());
    }
    if new_names.iter().any(|n| {
        n.is_empty() || n.contains(['\\', '\r', '\n', '=']) || n.chars().any(char::is_control)
    }) {
        return Err("Unsafe add-on name cannot be stored in Eden QSettings".into());
    }
    if new_names.iter().collect::<BTreeSet<_>>().len() != new_names.len() {
        return Err("Duplicate proposed Eden addon directory".into());
    }
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines: Vec<String> = original
        .lines()
        .map(|l| l.trim_end_matches('\r').to_owned())
        .collect();
    let mut start = None;
    let mut finish = lines.len();
    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if t.eq_ignore_ascii_case("[DisabledAddOns]") {
            if start.is_some() {
                return Err("Repeated DisabledAddOns section in Eden config".into());
            }
            start = Some(i);
        } else if start.is_some() && i > start.unwrap() && t.starts_with('[') && t.ends_with(']') {
            finish = i;
            break;
        }
    }
    let Some(begin) = start else {
        if !original.is_empty() && !original.ends_with('\n') {
            lines.push(String::new());
        }
        lines.extend([
            "[DisabledAddOns]".into(),
            "size=1".into(),
            "1\\title_id\\default=false".into(),
            format!("1\\title_id={title_number}"),
            format!("1\\disabled\\size={}", new_names.len()),
        ]);
        for (i, name) in new_names.iter().enumerate() {
            lines.push(format!("1\\disabled\\{}\\d\\default=false", i + 1));
            lines.push(format!("1\\disabled\\{}\\d={name}", i + 1));
        }
        return Ok(format!("{}{}", lines.join(newline), newline));
    };
    let mut size_line = None;
    let mut size = None;
    let mut rows = BTreeMap::<usize, DisabledRow>::new();
    let mut seen_keys = BTreeSet::new();
    for j in (begin + 1)..finish {
        let t = lines[j].trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        let (key, val) = t
            .split_once('=')
            .ok_or("Malformed DisabledAddOns setting")?;
        if !seen_keys.insert(key.to_owned()) {
            return Err("Duplicate Eden DisabledAddOns QSettings key".into());
        }
        if key == "size" {
            size_line = Some(j);
            size = Some(val.parse::<usize>()?);
            continue;
        }
        let parts = key.split('\\').collect::<Vec<_>>();
        if parts.len() < 2 || parts.len() > 5 {
            return Err("Unknown Eden DisabledAddOns setting format".into());
        }
        let row_index = parts[0].parse::<usize>()?;
        if row_index == 0 || row_index > MAX_DISABLED_GROUPS {
            return Err("Invalid Eden disabled add-on array index".into());
        }
        let row = rows.entry(row_index).or_default();
        match parts.as_slice() {
            [_, "title_id"] => {
                row.title_id = Some(val.parse::<u64>()?);
                row.title_line = Some(j);
            }
            // Qt WriteSetting produces companion /default keys. Eden ignores
            // explicit values when the flag is true, so refuse that state.
            [_, "title_id", "default"] if val == "false" => {}
            [_, "disabled", "size"] => {
                row.count = Some(val.parse::<usize>()?);
                row.count_line = Some(j);
            }
            [_, "disabled", name_index, "d"] => {
                let index = name_index.parse::<usize>()?;
                if index == 0 || index > MAX_DISABLED_NAMES || val.is_empty() {
                    return Err("Invalid existing disabled add-on entry".into());
                }
                row.names.insert(index, val.to_owned());
            }
            [_, "disabled", name_index, "d", "default"] if val == "false" => {
                let index = name_index.parse::<usize>()?;
                if index == 0 || index > MAX_DISABLED_NAMES {
                    return Err("Invalid Eden disabled add-on default entry".into());
                }
            }
            _ => {
                return Err(
                    format!("Unrecognized Eden DisabledAddOns QSettings field: {key}").into(),
                )
            }
        }
    }
    let declared = size.unwrap_or(0);
    if declared > MAX_DISABLED_GROUPS
        || rows.len() != declared
        || (1..=declared).any(|n| !rows.contains_key(&n))
    {
        return Err("Eden DisabledAddOns group count is inconsistent".into());
    }
    let mut matching = None;
    let mut unique_ids = BTreeSet::new();
    for (number, row) in &rows {
        let id = row
            .title_id
            .ok_or("Eden disabled add-on group has no title_id")?;
        if !unique_ids.insert(id) {
            return Err("Duplicate title ID in Eden DisabledAddOns".into());
        }
        let count = row.count.unwrap_or(0);
        if count > MAX_DISABLED_NAMES
            || row.names.len() != count
            || (1..=count).any(|i| !row.names.contains_key(&i))
        {
            return Err("Eden disabled add-on array count is inconsistent".into());
        }
        if id == title_number {
            matching = Some(*number);
        }
    }
    if let Some(index) = matching {
        let row = &rows[&index];
        let existing: BTreeSet<&str> = row.names.values().map(String::as_str).collect();
        let additional = new_names
            .iter()
            .filter(|n| !existing.contains(n.as_str()))
            .collect::<Vec<_>>();
        if additional.is_empty() {
            return Ok(original.to_owned());
        }
        let old_count = row.count.unwrap_or(0);
        let total = old_count
            .checked_add(additional.len())
            .ok_or("Disabled add-on count overflow")?;
        if total > MAX_DISABLED_NAMES {
            return Err("Too many disabled Eden add-ons".into());
        }
        if let Some(line) = row.count_line {
            lines[line] = format!("{index}\\disabled\\size={total}");
        }
        let mut fresh = Vec::new();
        if row.count_line.is_none() {
            fresh.push(format!("{index}\\disabled\\size={total}"));
        }
        for (offset, name) in additional.iter().enumerate() {
            fresh.push(format!(
                "{index}\\disabled\\{}\\d\\default=false",
                old_count + offset + 1
            ));
            fresh.push(format!(
                "{index}\\disabled\\{}\\d={name}",
                old_count + offset + 1
            ));
        }
        lines.splice(finish..finish, fresh);
    } else {
        let next = declared
            .checked_add(1)
            .ok_or("Disabled title array overflow")?;
        if next > MAX_DISABLED_GROUPS {
            return Err("Too many disabled Eden title groups".into());
        }
        if let Some(line) = size_line {
            lines[line] = format!("size={next}");
        }
        let mut fresh = Vec::new();
        if size_line.is_none() {
            fresh.push(format!("size={next}"));
        }
        fresh.push(format!("{next}\\title_id\\default=false"));
        fresh.push(format!("{next}\\title_id={title_number}"));
        fresh.push(format!("{next}\\disabled\\size={}", new_names.len()));
        for (idx, name) in new_names.iter().enumerate() {
            fresh.push(format!("{next}\\disabled\\{}\\d\\default=false", idx + 1));
            fresh.push(format!("{next}\\disabled\\{}\\d={name}", idx + 1));
        }
        lines.splice(finish..finish, fresh);
    }
    Ok(format!("{}{}", lines.join(newline), newline))
}

/// Persist a private, create-new recovery copy of the unmodified Eden
/// settings before replacing them. No existing backups are overwritten.
fn save_config_backup(path: &Path, expected: &str) -> Result<PathBuf, ErrBox> {
    let parent = path.parent().ok_or("Eden config has no parent")?;
    validate_regular_ancestors(path, true)?;
    for serial in 0..1024usize {
        let backup = parent.join(format!(
            ".qt-config.ini.cheatarium-backup-{}-{serial}",
            std::process::id()
        ));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
        {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.into()),
        };
        let outcome = (|| -> Result<(), ErrBox> {
            file.set_permissions(fs::metadata(path)?.permissions())?;
            file.write_all(expected.as_bytes())?;
            file.sync_all()?;
            drop(file);
            if fs::read_to_string(&backup)? != expected {
                return Err("Eden configuration backup verification failed".into());
            }
            Ok(())
        })();
        if let Err(err) = outcome {
            let _ = fs::remove_file(&backup);
            return Err(err);
        }
        return Ok(backup);
    }
    Err("Eden config backup slots are exhausted; refusing installation".into())
}

fn write_config_atomic(
    path: &Path,
    expected: &str,
    updated: &str,
) -> Result<Option<PathBuf>, ErrBox> {
    if expected == updated {
        return Ok(None);
    }
    let parent = path.parent().ok_or("Eden config has no parent")?;
    validate_regular_ancestors(path, true)?;
    if fs::read_to_string(path)? != expected {
        return Err("Eden config changed during installation; refusing to overwrite".into());
    }
    let temp = parent.join(format!(".qt-config.ini.cheatarium-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| -> Result<PathBuf, ErrBox> {
        file.set_permissions(fs::metadata(path)?.permissions())?;
        file.write_all(updated.as_bytes())?;
        file.sync_all()?;
        drop(file);
        eden_not_running()?;
        if fs::read_to_string(path)? != expected {
            return Err("Eden config changed before atomic update".into());
        }
        // The original settings are preserved independently before rename,
        // so an interrupted later mod write remains recoverable.
        let backup = save_config_backup(path, expected)?;
        if fs::read_to_string(path)? != expected {
            return Err("Eden config changed after backup; refusing to overwrite".into());
        }
        eden_not_running()?;
        fs::rename(&temp, path)?;
        Ok(backup)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map(Some)
}

/// Idempotent on-demand auto-run: a prior Cheatarium install is considered
/// complete only when *every* proposed file matches its exact source bytes
/// and Eden's disabled list already contains every addon name. Never touch
/// existing files here; any mismatch falls through to the no-overwrite path.
pub(super) fn already_installed_unchanged(
    bundle: &Bundle,
    selection: &Preview,
    mods_root: &Path,
    config: &Path,
) -> Result<bool, ErrBox> {
    validate_regular_ancestors(config, true)?;
    if !mods_root.exists() {
        return Ok(false);
    }
    validate_regular_ancestors(mods_root, false)?;
    let prepared = prepare_addons(bundle, selection, mods_root)?;
    if prepared.is_empty() {
        return Ok(false);
    }
    let mut names = Vec::new();
    for (entry, bytes) in &prepared {
        let relative = Path::new(&entry.relative_path);
        let parts = relative.components().collect::<Vec<_>>();
        if parts.len() != 4
            || parts[0].as_os_str() != std::ffi::OsStr::new(&selection.title_id)
            || parts[2].as_os_str() != "cheats"
        {
            return Err("Unexpected prior Eden addon relative path".into());
        }
        let name = parts[1].as_os_str().to_string_lossy().into_owned();
        let target = mods_root.join(relative);
        if !target.exists() {
            return Ok(false);
        }
        validate_regular_ancestors(&target, true)?;
        let metadata = fs::metadata(&target)?;
        if metadata.len() > super::MAX_STAGED_TOTAL_BYTES as u64 {
            return Ok(false);
        }
        if fs::read(&target)? != *bytes {
            return Ok(false);
        }
        names.push(name);
    }
    let original = fs::read_to_string(config)?;
    if original.len() as u64 > MAX_INI_BYTES {
        return Err("Eden Qt configuration too large".into());
    }
    Ok(disabled_config(&original, &selection.title_id, &names)? == original)
}

pub(super) fn install(
    bundle: &Bundle,
    selection: &Preview,
    mods_root: &Path,
    config: &Path,
) -> Result<Installed, ErrBox> {
    eden_not_running()?;
    validate_regular_ancestors(config, true)?;
    validate_mod_root_plan(mods_root)?;
    let prepared = prepare_addons(bundle, selection, mods_root)?;
    let title = &selection.title_id;
    let title_dir = mods_root.join(title);
    if fs::symlink_metadata(&title_dir).is_ok() {
        validate_regular_ancestors(&title_dir, false)?;
    }
    let mut names = Vec::new();
    let mut existing_paths = BTreeSet::new();
    for (entry, _) in &prepared {
        let relative = Path::new(&entry.relative_path);
        let parts = relative.components().collect::<Vec<_>>();
        if parts.len() != 4
            || parts[0].as_os_str() != std::ffi::OsStr::new(title)
            || parts[2].as_os_str() != "cheats"
        {
            return Err("Unrecognized planned Eden addon structure".into());
        }
        let addon = parts[1].as_os_str().to_string_lossy().into_owned();
        if !existing_paths.insert(addon.clone()) {
            return Err("Duplicate planned Eden addon directory".into());
        }
        let destination = title_dir.join(&addon);
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(format!(
                "Refusing to overwrite an existing Eden mod: {}",
                destination.display()
            )
            .into());
        }
        names.push(addon);
    }
    if prepared.is_empty() {
        return Err("No exact-build cheats to install".into());
    }
    let original = fs::read_to_string(config)?;
    if original.len() as u64 > MAX_INI_BYTES {
        return Err("Eden configuration exceeds size limit".into());
    }
    let updated = disabled_config(&original, title, &names)?;
    // Ensure QSettings disables every new addon before the first file appears.
    eden_not_running()?;
    let backup = write_config_atomic(config, &original, &updated)?;
    fs::create_dir_all(mods_root)?;
    validate_regular_ancestors(mods_root, false)?;
    if !title_dir.exists() {
        fs::create_dir(&title_dir)?;
    }
    let mut written = 0usize;
    for ((entry, bytes), addon) in prepared.iter().zip(names.iter()) {
        let root = title_dir.join(addon);
        fs::create_dir(&root)?; // create_new semantics: refuses existing mods
        let cheats = root.join("cheats");
        fs::create_dir(&cheats)?;
        let dest = cheats.join(format!("{}.txt", selection.explicitly_selected_build_id));
        super::write_new(&dest, bytes)?;
        if !dest.is_file() || format!("{:x}", sha2::Sha256::digest(bytes)) != entry.sha256 {
            return Err("Installed cheat checksum verification failed".into());
        }
        written += 1;
    }
    Ok(Installed {
        schema_version: 1,
        format: "cheatarium-eden-direct-install-v1",
        title_id: title.clone(),
        selected_build_id: selection.explicitly_selected_build_id.clone(),
        installed_addons: names,
        files_written: written,
        eden_config_updated: updated != original,
        eden_config_backup_path: backup.map(|path| path.to_string_lossy().into_owned()),
        activation_state: "installed_disabled_by_default",
        installed_build_id_verified: false,
        compatible_cheats_verified: false,
        existing_mods_overwritten: false,
        emulator_game_files_written: 0,
        warning: "Cheat files are installed but disabled in Eden. Enable only desired per-cheat add-ons in Eden after confirming your game's executable Build ID. Eden must have been closed during installation.",
    })
}

/// Explicit file-only mode: install native per-cheat directories and never
/// read, rewrite, or back up Eden's Qt settings. New add-ons are usually
/// enabled by Eden; actual activation remains under the user's control.
pub(super) fn install_files_only(
    bundle: &Bundle,
    selection: &Preview,
    mods_root: &Path,
) -> Result<Installed, ErrBox> {
    eden_not_running()?;
    validate_mod_root_plan(mods_root)?;
    if !selection.rejected_entries.is_empty() {
        return Err("Refusing file-only install: source has rejected cheat entries".into());
    }
    let prepared = prepare_addons(bundle, selection, mods_root)?;
    if prepared.is_empty() {
        return Err("No exact-build cheats to install".into());
    }
    let title = &selection.title_id;
    let title_dir = mods_root.join(title);
    if fs::symlink_metadata(&title_dir).is_ok() {
        validate_regular_ancestors(&title_dir, false)?;
    }
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for (entry, bytes) in &prepared {
        if format!("{:x}", sha2::Sha256::digest(bytes)) != entry.sha256 {
            return Err("Source cheat checksum changed before installation".into());
        }
        let relative = Path::new(&entry.relative_path);
        let parts = relative.components().collect::<Vec<_>>();
        let expected_name = format!("{}.txt", selection.explicitly_selected_build_id);
        if parts.len() != 4
            || parts[0].as_os_str() != std::ffi::OsStr::new(title)
            || parts[2].as_os_str() != "cheats"
            || parts[3].as_os_str() != std::ffi::OsStr::new(expected_name.as_str())
        {
            return Err("Unrecognized file-only Eden addon structure".into());
        }
        let addon = parts[1].as_os_str().to_string_lossy().into_owned();
        if !seen.insert(addon.clone()) {
            return Err("Duplicate planned Eden addon directory".into());
        }
        if fs::symlink_metadata(title_dir.join(&addon)).is_ok() {
            return Err(format!("Refusing to overwrite existing Eden mod: {addon}").into());
        }
        names.push(addon);
    }
    eden_not_running()?;
    // All destinations checked before creating the first addon directory.
    fs::create_dir_all(mods_root)?;
    validate_regular_ancestors(mods_root, false)?;
    if !title_dir.exists() {
        fs::create_dir(&title_dir)?;
    }
    let mut written = 0usize;
    for ((entry, bytes), name) in prepared.iter().zip(names.iter()) {
        let addon_root = title_dir.join(name);
        fs::create_dir(&addon_root)?;
        let cheats = addon_root.join("cheats");
        fs::create_dir(&cheats)?;
        let target = cheats.join(format!("{}.txt", selection.explicitly_selected_build_id));
        super::write_new(&target, bytes)?;
        if fs::read(&target)? != *bytes
            || format!("{:x}", sha2::Sha256::digest(bytes)) != entry.sha256
        {
            return Err("Installed file-only cheat checksum verification failed".into());
        }
        written += 1;
    }
    Ok(Installed {
        schema_version: 1,
        format: "cheatarium-eden-file-only-install-v1",
        title_id: title.clone(),
        selected_build_id: selection.explicitly_selected_build_id.clone(),
        installed_addons: names,
        files_written: written,
        eden_config_updated: false,
        eden_config_backup_path: None,
        activation_state: "files_installed_no_activation_settings_changed",
        installed_build_id_verified: false,
        compatible_cheats_verified: false,
        existing_mods_overwritten: false,
        emulator_game_files_written: 0,
        warning: "Created only Eden cheat folders. No Qt configuration was read or modified. Eden normally enables new add-ons by default; review Add-ons before playing. Actual effects remain unverified.",
    })
}

/// Exact existing files are repeatable, but an incomplete or modified addon
/// must be handled as a collision, never overwritten.
pub(super) fn already_installed_files_unchanged(
    bundle: &Bundle,
    selection: &Preview,
    mods_root: &Path,
) -> Result<bool, ErrBox> {
    if !mods_root.exists() {
        return Ok(false);
    }
    validate_regular_ancestors(mods_root, false)?;
    let prepared = prepare_addons(bundle, selection, mods_root)?;
    if prepared.is_empty() {
        return Ok(false);
    }
    for (entry, bytes) in prepared {
        let dest = mods_root.join(&entry.relative_path);
        let metadata = match fs::symlink_metadata(&dest) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err.into()),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("Existing cheat destination is not a regular file".into());
        }
        validate_regular_ancestors(&dest, true)?;
        if fs::read(dest)? != bytes {
            return Ok(false);
        }
    }
    Ok(true)
}


#[derive(Debug, Serialize)]
pub(super) struct RenameItem {
    previous_addon: String,
    readable_addon: String,
    status: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct RenameReport {
    format: &'static str,
    title_id: String,
    selected_build_id: String,
    mode: &'static str,
    eden_config_updated: bool,
    would_rename: usize,
    renamed: usize,
    already_readable: usize,
    addons: Vec<RenameItem>,
}

/// Rename only Cheatarium's precise legacy per-cheat folder names, without
/// changing Qt configuration or regenerating any cheat code. Every file is
/// verified against the same source snapshot before a single rename begins.
pub(super) fn rename_legacy_file_only(
    bundle: &Bundle,
    selection: &Preview,
    mods_root: &Path,
    config: &Path,
    dry_run: bool,
) -> Result<RenameReport, ErrBox> {
    if !dry_run {
        eden_not_running()?;
    }
    validate_regular_ancestors(config, true)?;
    validate_regular_ancestors(mods_root, false)?;
    if !selection.rejected_entries.is_empty() || selection.proposed_addons.is_empty() {
        return Err("Cannot rename unvalidated or empty source cheat selection".into());
    }
    let prepared = prepare_addons(bundle, selection, mods_root)?;
    let title_dir = mods_root.join(&selection.title_id);
    validate_regular_ancestors(&title_dir, false)?;
    let config_meta = fs::metadata(config)?;
    if config_meta.len() > MAX_INI_BYTES {
        return Err("Eden Qt settings too large to verify activation state".into());
    }
    let ini = fs::read_to_string(config)?;
    let mut addons = Vec::new();
    let mut moves = Vec::<(PathBuf, PathBuf)>::new();
    let mut already_readable = 0usize;
    for (entry, expected_bytes) in &prepared {
        let record = selection.proposed_addons.iter().find(|x| {
            x.source_record_id == entry.source_record_id && x.source_ordinal == entry.source_ordinal
        }).ok_or("Legacy rename lost source cheat identity")?;
        if record.syntax_check != "lexical_hex_tokens_only_not_execution_validated" {
            return Err("Unrecognized source opcode syntax: refusing rename".into());
        }
        let rendered = String::from_utf8(expected_bytes.clone())?;
        let ident = format!("{}:{}:{}", entry.source_record_id, entry.source_ordinal, rendered);
        let digest = format!("{:x}", sha2::Sha256::digest(ident.as_bytes()));
        let old_name = format!(
            "Cheatarium-{:04}-{}-{}",
            entry.source_ordinal,
            super::safe_label(&record.description),
            &digest[..12]
        );
        let new_name = Path::new(&entry.relative_path)
            .components().nth(1).ok_or("Missing readable Eden addon name")?
            .as_os_str().to_string_lossy().into_owned();
        if old_name == new_name {
            return Err("Legacy and readable addon names are identical".into());
        }
        let old_dir = title_dir.join(&old_name);
        let new_dir = title_dir.join(&new_name);
        let old_exists = fs::symlink_metadata(&old_dir);
        let new_exists = fs::symlink_metadata(&new_dir);
        if old_exists.is_ok() && new_exists.is_ok() {
            return Err(format!("Both old and new Eden addons exist: {old_name} / {new_name}").into());
        }
        let (target, status) = if old_exists.is_ok() {
            (old_dir.as_path(), "would_rename")
        } else if new_exists.is_ok() {
            already_readable += 1;
            (new_dir.as_path(), "already_readable")
        } else {
            return Err(format!("Neither expected legacy nor readable addon exists: {old_name}").into());
        };
        validate_regular_ancestors(target, false)?;
        let cheats = target.join("cheats");
        validate_regular_ancestors(&cheats, false)?;
        let file = cheats.join(format!("{}.txt", selection.explicitly_selected_build_id));
        validate_regular_ancestors(&file, true)?;
        if fs::read(&file)? != *expected_bytes {
            return Err(format!("Cheat bytes differ from the archived original: {old_name}").into());
        }
        // Do not rename a user-modified directory containing other content.
        if fs::read_dir(target)?.count() != 1 || fs::read_dir(&cheats)?.count() != 1 {
            return Err(format!("Unexpected contents in Eden addon: {old_name}").into());
        }
        // A renamed disabled addon could silently become active. Refuse to
        // migrate if the old name appears in *any* disabled-addon setting.
        if ini.lines().any(|line| {
            line.trim().split_once('=').is_some_and(|(key, value)| {
                key.trim().ends_with(r"\d") && value.trim() == old_name
            })
        }) {
            return Err(format!("Addon is recorded as disabled in Eden settings; rename would change its activation: {old_name}").into());
        }
        if status == "would_rename" {
            moves.push((old_dir, new_dir));
        }
        addons.push(RenameItem {
            previous_addon: old_name,
            readable_addon: new_name,
            status,
        });
    }
    let to_rename = moves.len();
    if !dry_run {
        let mut completed = Vec::<(PathBuf, PathBuf)>::new();
        for (old, new) in &moves {
            if let Err(problem) = eden_not_running().and_then(|_| fs::rename(old, new).map_err(Into::into)) {
                let mut rollback_failed = false;
                for (previous, current) in completed.into_iter().rev() {
                    rollback_failed |= fs::rename(current, previous).is_err();
                }
                return Err(format!(
                    "Cheat rename interrupted: {problem}; rollback {}",
                    if rollback_failed { "INCOMPLETE, inspect Eden folders" } else { "completed" }
                ).into());
            }
            completed.push((old.clone(), new.clone()));
        }
        for item in &mut addons {
            if item.status == "would_rename" {
                item.status = "renamed";
            }
        }
    }
    Ok(RenameReport {
        format: "cheatarium-eden-readable-addon-rename-v1",
        title_id: selection.title_id.clone(),
        selected_build_id: selection.explicitly_selected_build_id.clone(),
        mode: if dry_run { "read_only_rename_preview" } else { "rename_verified_legacy_folders" },
        eden_config_updated: false,
        would_rename: if dry_run { to_rename } else { 0 },
        renamed: if dry_run { 0 } else { to_rename },
        already_readable,
        addons,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_disabled_qsettings_preserves_existing_mods_and_unrelated_settings() {
        let ini = "[UI]\nfoo=bar\n[DisabledAddOns]\nsize=2\n1\\title_id=72057594037993472\n1\\disabled\\size=1\n1\\disabled\\1\\d=Existing Mod\n2\\title_id=72081146186022912\n2\\disabled\\size=0\n[Audio]\nvolume=22\n";
        let title = format!("{:016X}", 72057594037993472u64);
        let next = disabled_config(ini, &title, &["Cheatarium-0000-Test".into()]).unwrap();
        assert!(next.contains("1\\disabled\\size=2"));
        assert!(next.contains("1\\disabled\\2\\d=Cheatarium-0000-Test"));
        assert!(next.contains("1\\disabled\\1\\d=Existing Mod"));
        assert!(next.contains("[Audio]\nvolume=22"));
        assert_eq!(
            disabled_config(&next, &title, &["Cheatarium-0000-Test".into()]).unwrap(),
            next
        );
    }
    #[test]
    fn create_new_disabled_addon_array() {
        let ini = "[UI]\nfoo=bar\n";
        let result = disabled_config(ini, "0100000000010000", &["Cheatarium-Test".into()]).unwrap();
        assert!(result.contains("[DisabledAddOns]\nsize=1"));
        assert!(result.contains("1\\title_id\\default=false"));
        assert!(result.contains("1\\title_id=72057594037993472"));
        assert!(result.contains("1\\disabled\\1\\d\\default=false"));
        assert!(result.contains("1\\disabled\\1\\d=Cheatarium-Test"));
        let second =
            disabled_config(&result, "0100152000022000", &["Cheatarium-Kart".into()]).unwrap();
        assert!(second.contains("size=2"));
        assert!(second.contains("2\\title_id\\default=false"));
        assert!(second.contains("2\\disabled\\1\\d\\default=false"));
        assert!(second.contains("2\\disabled\\1\\d=Cheatarium-Kart"));
    }
    #[test]
    fn accepts_real_qt_default_metadata_and_preserves_existing_addons() {
        let ini = r#"[UI]
foo=bar
[DisabledAddOns]
size=1
1\title_id\default=false
1\title_id=72057594037993472
1\disabled\size=1
1\disabled\1\d\default=false
1\disabled\1\d=Existing Mod
[Audio]
volume=22
"#;
        let next = disabled_config(ini, "0100000000010000", &["Cheatarium-Test".into()]).unwrap();
        assert!(next.contains("1\\disabled\\1\\d=Existing Mod"));
        assert!(next.contains("1\\disabled\\1\\d\\default=false"));
        assert!(next.contains("1\\disabled\\2\\d\\default=false"));
        assert!(next.contains("1\\disabled\\2\\d=Cheatarium-Test"));
        assert!(next.contains("[Audio]\nvolume=22\n"));
        assert_eq!(
            disabled_config(&next, "0100000000010000", &["Cheatarium-Test".into()]).unwrap(),
            next
        );
    }

    #[test]
    fn refuses_ignored_or_unknown_qsettings_fields() {
        for field in [
            r"1\title_id\default=true",
            r"1\disabled\1\d\default=true",
            r"1\unexpected=value",
        ] {
            let ini = format!(
                "[DisabledAddOns]\nsize=1\n1\\title_id=72057594037993472\n1\\disabled\\size=1\n1\\disabled\\1\\d=Existing\n{field}\n",
            );
            assert!(disabled_config(&ini, "0100000000010000", &["New".into()]).is_err());
        }
    }

    #[test]
    fn rejects_corrupt_disable_array() {
        let ini = "[DisabledAddOns]\nsize=1\n1\\title_id=72057594037993472\n1\\disabled\\size=2\n1\\disabled\\1\\d=One\n";
        assert!(disabled_config(ini, "0100000000010000", &["New".into()]).is_err());
    }
}
