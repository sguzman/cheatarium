//! On-demand Eden cheat installation. Existing mods never overwritten.
//! Disable every new per-cheat addon in Eden's QSettings before creating it.
use super::{prepare_addons, Bundle, ErrBox, Preview, StagedFile};
use serde::Serialize;
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
    activation_state: &'static str,
    installed_build_id_verified: bool,
    compatible_cheats_verified: bool,
    existing_mods_overwritten: bool,
    emulator_game_files_written: usize,
    warning: &'static str,
}

fn validate_regular_ancestors(path: &Path, final_is_file: bool) -> Result<(), ErrBox> {
    if !path.is_absolute()
        || path.components().any(|c| matches!(c, Component::CurDir | Component::ParentDir))
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
            let Ok(comm) = fs::read_to_string(item.path().join("comm")) else { continue };
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
fn disabled_config(original: &str, title_id: &str, new_names: &[String]) -> Result<String, ErrBox> {
    let title_number = u64::from_str_radix(title_id, 16)?;
    if new_names.is_empty() || new_names.len() > MAX_DISABLED_NAMES {
        return Err("No valid per-cheat addon names to disable".into());
    }
    if new_names.iter().any(|n| n.is_empty() || n.contains(['\\', '\r', '\n', '='])
        || n.chars().any(char::is_control)) {
        return Err("Unsafe add-on name cannot be stored in Eden QSettings".into());
    }
    if new_names.iter().collect::<BTreeSet<_>>().len() != new_names.len() {
        return Err("Duplicate proposed Eden addon directory".into());
    }
    let newline = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = original.lines().map(|l| l.trim_end_matches('\r').to_owned()).collect();
    let mut start = None;
    let mut finish = lines.len();
    for (i, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        if t.eq_ignore_ascii_case("[DisabledAddOns]") {
            if start.is_some() { return Err("Repeated DisabledAddOns section in Eden config".into()); }
            start = Some(i);
        } else if start.is_some() && i > start.unwrap()
            && t.starts_with('[') && t.ends_with(']') {
            finish = i;
            break;
        }
    }
    let Some(begin) = start else {
        if !original.is_empty() && !original.ends_with('\n') { lines.push(String::new()); }
        lines.extend([
            "[DisabledAddOns]".into(),
            "size=1".into(),
            format!("1\\title_id={title_number}"),
            format!("1\\disabled\\size={}", new_names.len()),
        ]);
        for (i, name) in new_names.iter().enumerate() {
            lines.push(format!("1\\disabled\\{}\\d={name}", i+1));
        }
        return Ok(format!("{}{}", lines.join(newline), newline));
    };
    let mut size_line = None;
    let mut size = None;
    let mut rows = BTreeMap::<usize, DisabledRow>::new();
    let mut seen_keys = BTreeSet::new();
    for j in (begin+1)..finish {
        let t = lines[j].trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') { continue; }
        let (key, val) = t.split_once('=').ok_or("Malformed DisabledAddOns setting")?;
        if !seen_keys.insert(key.to_owned()) {
            return Err("Duplicate Eden DisabledAddOns QSettings key".into());
        }
        if key == "size" {
            size_line = Some(j);
            size = Some(val.parse::<usize>()?);
            continue;
        }
        let parts = key.split('\\').collect::<Vec<_>>();
        if parts.len() < 2 || parts.len() > 4 {
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
            _ => return Err("Unrecognized Eden DisabledAddOns QSettings field".into()),
        }
    }
    let declared = size.unwrap_or(0);
    if declared > MAX_DISABLED_GROUPS || rows.len() != declared
        || (1..=declared).any(|n| !rows.contains_key(&n))
    {
        return Err("Eden DisabledAddOns group count is inconsistent".into());
    }
    let mut matching = None;
    let mut unique_ids = BTreeSet::new();
    for (number, row) in &rows {
        let id = row.title_id.ok_or("Eden disabled add-on group has no title_id")?;
        if !unique_ids.insert(id) { return Err("Duplicate title ID in Eden DisabledAddOns".into()); }
        let count = row.count.unwrap_or(0);
        if count > MAX_DISABLED_NAMES || row.names.len() != count
            || (1..=count).any(|i| !row.names.contains_key(&i))
        {
            return Err("Eden disabled add-on array count is inconsistent".into());
        }
        if id == title_number { matching = Some(*number); }
    }
    if let Some(index) = matching {
        let row = &rows[&index];
        let existing: BTreeSet<&str> = row.names.values().map(String::as_str).collect();
        let additional = new_names.iter().filter(|n| !existing.contains(n.as_str())).collect::<Vec<_>>();
        if additional.is_empty() { return Ok(original.to_owned()); }
        let old_count = row.count.unwrap_or(0);
        let total = old_count.checked_add(additional.len()).ok_or("Disabled add-on count overflow")?;
        if total > MAX_DISABLED_NAMES { return Err("Too many disabled Eden add-ons".into()); }
        if let Some(line) = row.count_line {
            lines[line] = format!("{index}\\disabled\\size={total}");
        }
        let mut fresh = Vec::new();
        if row.count_line.is_none() {
            fresh.push(format!("{index}\\disabled\\size={total}"));
        }
        for (offset, name) in additional.iter().enumerate() {
            fresh.push(format!("{index}\\disabled\\{}\\d={name}", old_count+offset+1));
        }
        lines.splice(finish..finish, fresh);
    } else {
        let next = declared.checked_add(1).ok_or("Disabled title array overflow")?;
        if next > MAX_DISABLED_GROUPS { return Err("Too many disabled Eden title groups".into()); }
        if let Some(line) = size_line { lines[line] = format!("size={next}"); }
        let mut fresh = Vec::new();
        if size_line.is_none() { fresh.push(format!("size={next}")); }
        fresh.push(format!("{next}\\title_id={title_number}"));
        fresh.push(format!("{next}\\disabled\\size={}", new_names.len()));
        for (idx,name) in new_names.iter().enumerate() {
            fresh.push(format!("{next}\\disabled\\{}\\d={name}", idx+1));
        }
        lines.splice(finish..finish, fresh);
    }
    Ok(format!("{}{}", lines.join(newline), newline))
}

fn write_config_atomic(path: &Path, expected: &str, updated: &str) -> Result<(), ErrBox> {
    if expected == updated { return Ok(()); }
    let parent = path.parent().ok_or("Eden config has no parent")?;
    validate_regular_ancestors(path, true)?;
    if fs::read_to_string(path)? != expected {
        return Err("Eden config changed during installation; refusing to overwrite".into());
    }
    let temp = parent.join(format!(".qt-config.ini.cheatarium-{}", std::process::id()));
    let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
    let result = (|| -> Result<(), ErrBox> {
        file.set_permissions(fs::metadata(path)?.permissions())?;
        file.write_all(updated.as_bytes())?;
        file.sync_all()?;
        drop(file);
        eden_not_running()?;
        if fs::read_to_string(path)? != expected {
            return Err("Eden config changed before atomic update".into());
        }
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

pub(super) fn install(
    bundle: &Bundle,
    selection: &Preview,
    mods_root: &Path,
    config: &Path,
) -> Result<Installed, ErrBox> {
    eden_not_running()?;
    validate_regular_ancestors(config, true)?;
    validate_regular_ancestors(mods_root, false)?;
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
        if parts.len() != 4 || parts[0].as_os_str() != title
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
            return Err(format!("Refusing to overwrite an existing Eden mod: {}", destination.display()).into());
        }
        names.push(addon);
    }
    if prepared.is_empty() { return Err("No exact-build cheats to install".into()); }
    let original = fs::read_to_string(config)?;
    if original.len() as u64 > MAX_INI_BYTES {
        return Err("Eden configuration exceeds size limit".into());
    }
    let updated = disabled_config(&original, title, &names)?;
    // Ensure QSettings disables every new addon before the first file appears.
    eden_not_running()?;
    write_config_atomic(config, &original, &updated)?;
    if !title_dir.exists() { fs::create_dir(&title_dir)?; }
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
        activation_state: "installed_disabled_by_default",
        installed_build_id_verified: false,
        compatible_cheats_verified: false,
        existing_mods_overwritten: false,
        emulator_game_files_written: 0,
        warning: "Cheat files are installed but disabled in Eden. Enable only desired per-cheat add-ons in Eden after confirming your game's executable Build ID. Eden must have been closed during installation.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_disabled_qsettings_preserves_existing_mods_and_unrelated_settings() {
        let ini = "[UI]\nfoo=bar\n[DisabledAddOns]\nsize=2\n1\\title_id=72057594037993472\n1\\disabled\\size=1\n1\\disabled\\1\\d=Existing Mod\n2\\title_id=72081146186022912\n2\\disabled\\size=0\n[Audio]\nvolume=22\n";
        let title = format!("{:016X}",72057594037993472u64);
        let next = disabled_config(ini,&title,&["Cheatarium-0000-Test".into()]).unwrap();
        assert!(next.contains("1\\disabled\\size=2"));
        assert!(next.contains("1\\disabled\\2\\d=Cheatarium-0000-Test"));
        assert!(next.contains("1\\disabled\\1\\d=Existing Mod"));
        assert!(next.contains("[Audio]\nvolume=22"));
        assert_eq!(disabled_config(&next,&title,&["Cheatarium-0000-Test".into()]).unwrap(),next);
    }
    #[test]
    fn create_new_disabled_addon_array() {
        let ini = "[UI]\nfoo=bar\n";
        let result = disabled_config(ini,"0100000000010000",&["Cheatarium-Test".into()]).unwrap();
        assert!(result.contains("[DisabledAddOns]\nsize=1"));
        assert!(result.contains("1\\title_id=72057594037993472"));
        assert!(result.contains("1\\disabled\\1\\d=Cheatarium-Test"));
        let second = disabled_config(&result,"0100152000022000",&["Cheatarium-Kart".into()]).unwrap();
        assert!(second.contains("size=2"));
        assert!(second.contains("2\\disabled\\1\\d=Cheatarium-Kart"));
    }
    #[test]
    fn rejects_corrupt_disable_array() {
        let ini = "[DisabledAddOns]\nsize=1\n1\\title_id=72057594037993472\n1\\disabled\\size=2\n1\\disabled\\1\\d=One\n";
        assert!(disabled_config(ini,"0100000000010000",&["New".into()]).is_err());
    }
}
