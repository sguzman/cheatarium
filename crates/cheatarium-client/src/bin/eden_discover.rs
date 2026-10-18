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
const MAX_NSP_HEADER_BYTES: u64 = 256 * 1024;
const MAX_NSP_HEADER_ENTRIES: usize = 2048;
const MAX_NSP_HEADER_INSPECTIONS: usize = 24;

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

#[derive(Debug, Serialize)]
struct InventoryGame {
    title_id: String,
    filename_labeled_base_files: usize,
    in_eden_cache: bool,
    archived_build_candidate_count: usize,
    external_reference_count: usize,
    update_files_in_registered_folders: usize,
    installed_base_game_verified: bool,
    installed_update_verified: bool,
    installed_build_id_verified: bool,
}

#[derive(Debug, Serialize)]
struct InventoryUpdate {
    file_path: String,
    update_title_id: String,
    probable_base_title_id: String,
    relationship_evidence: &'static str,
    applied_by_eden_verified: bool,
}

#[derive(Debug, Serialize)]
struct InventoryDlcFolder {
    path: String,
    file_count: usize,
    installed_in_eden_verified: bool,
}

#[derive(Debug, Serialize)]
struct NspHeaderInspection {
    file_path: String,
    status: &'static str,
    bytes_read: usize,
    container_file_entries: usize,
    nca_file_entries: usize,
    cnmt_nca_file_entries: usize,
    ticket_filename_title_id_candidates: Vec<String>,
    executable_build_id_found: bool,
    installed_build_id_verified: bool,
}

/// NSP's PFS0 outer header is unencrypted. Tickets, keys and NCA payloads
/// are NOT opened or parsed; title hints here come only from ticket *names*.
/// This is a deliberately small read-only slice of Eden's PFS0 implementation.
fn inspect_unresolved_nsp(path: &str) -> NspHeaderInspection {
    let mut report = NspHeaderInspection {
        file_path: path.to_owned(),
        status: "unavailable",
        bytes_read: 0,
        container_file_entries: 0,
        nca_file_entries: 0,
        cnmt_nca_file_entries: 0,
        ticket_filename_title_id_candidates: Vec::new(),
        executable_build_id_found: false,
        installed_build_id_verified: false,
    };
    let path = Path::new(path);
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => meta,
        Ok(_) => {
            report.status = "not_a_regular_file";
            return report;
        }
        Err(_) => return report,
    };
    if meta.len() < 16 {
        report.status = "too_short";
        return report;
    }
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return report,
    };
    // Read exactly the header, not the fixed-size prefix: the bytes
    // immediately after PFS0 metadata can be ticket or NCA contents.
    let mut first = [0u8; 16];
    if file.read_exact(&mut first).is_err() {
        report.status = "read_error";
        return report;
    }
    report.bytes_read = 16;
    if &first[..4] != b"PFS0" {
        report.status = "not_pfs0";
        return report;
    }
    let read_u32 = |a: &[u8]| u32::from_le_bytes(a.try_into().expect("4-byte field"));
    let read_u64 = |a: &[u8]| u64::from_le_bytes(a.try_into().expect("8-byte field"));
    let entries = read_u32(&first[4..8]) as usize;
    let table_bytes = read_u32(&first[8..12]) as usize;
    if entries > MAX_NSP_HEADER_ENTRIES {
        report.status = "too_many_entries";
        return report;
    }
    let Some(names_start) = entries.checked_mul(24).and_then(|x| x.checked_add(16)) else {
        report.status = "malformed";
        return report;
    };
    let Some(data_start) = names_start.checked_add(table_bytes) else {
        report.status = "malformed";
        return report;
    };
    if (data_start as u64) > meta.len() {
        report.status = "malformed";
        return report;
    }
    if (data_start as u64) > MAX_NSP_HEADER_BYTES {
        report.status = "header_exceeds_prefix_limit";
        return report;
    }
    let mut buffer = Vec::with_capacity(data_start);
    buffer.extend_from_slice(&first);
    let mut rest = vec![0u8; data_start - 16];
    let mut read = 0;
    while read < rest.len() {
        match file.read(&mut rest[read..]) {
            Ok(0) | Err(_) => {
                report.status = "read_error";
                report.bytes_read += read;
                return report;
            }
            Ok(count) => read += count,
        }
    }
    report.bytes_read += rest.len();
    buffer.extend_from_slice(&rest);
    let names = &buffer[names_start..data_start];
    let mut ticket_ids = BTreeSet::new();
    for idx in 0..entries {
        let e = &buffer[(16 + 24 * idx)..(16 + 24 * (idx + 1))];
        let offset = read_u64(&e[0..8]);
        let size = read_u64(&e[8..16]);
        let name_start = read_u32(&e[16..20]) as usize;
        let Some(last) = name_start.checked_add(1) else {
            report.status = "malformed";
            return report;
        };
        if last > names.len() {
            report.status = "malformed";
            return report;
        }
        let Some(end) = names[name_start..].iter().position(|b| *b == 0) else {
            report.status = "malformed";
            return report;
        };
        let Ok(name) = std::str::from_utf8(&names[name_start..name_start + end]) else {
            report.status = "malformed";
            return report;
        };
        let Some(content_end) = (data_start as u64)
            .checked_add(offset)
            .and_then(|x| x.checked_add(size))
        else {
            report.status = "malformed";
            return report;
        };
        if content_end > meta.len()
            || name.contains(['/', '\\'])
            || name.is_empty()
            || name.chars().any(char::is_control)
        {
            report.status = "malformed";
            return report;
        }
        if name.ends_with(".nca") {
            report.nca_file_entries += 1;
            if name.ends_with(".cnmt.nca") {
                report.cnmt_nca_file_entries += 1;
            }
        }
        // Ticket filenames often start with 32 hex chars representing a
        // rights ID, whose first 16 are a *hint* to the owning title.
        // Never inspect ticket contents or treat this as authenticated.
        if let Some(ident) = name.strip_suffix(".tik") {
            if ident.len() == 32 && ident.bytes().all(|b| b.is_ascii_hexdigit()) {
                let prefix = ident[..16].to_ascii_uppercase();
                if is_title_id(&prefix) {
                    ticket_ids.insert(prefix);
                }
            }
        }
    }
    report.container_file_entries = entries;
    report.ticket_filename_title_id_candidates = ticket_ids.into_iter().collect();
    report.status = "outer_pfs0_metadata_only";
    report
}

#[derive(Debug, Serialize)]
struct InventorySummary {
    schema_version: u32,
    format: &'static str,
    read_only: bool,
    registered_game_roots: Vec<GameRoot>,
    game_file_bytes_read: u32,
    emulator_files_written: u32,
    total_game_container_files: usize,
    candidate_base_titles: Vec<InventoryGame>,
    probable_update_files: Vec<InventoryUpdate>,
    dlc_folder_groups: Vec<InventoryDlcFolder>,
    filename_ids_not_classified: usize,
    unresolved_filename_title_ids: Vec<String>,
    cached_title_ids_without_labeled_base_file: Vec<String>,
    unresolved_nsp_header_inspections: Vec<NspHeaderInspection>,
    unresolved_nsp_inspections_truncated: bool,
    inspected_nsp_file_bytes_read: usize,
    saved_disabled_addons: Vec<SavedDisabledAddons>,
    cached_update_displays: Vec<CachedUpdateDisplay>,
    installed_build_id_verified: bool,
    warnings: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct SavedDisabledAddons {
    title_id: String,
    disabled_addon_names: Vec<String>,
    disabled_external_update_versions: Vec<u32>,
    generic_update_disabled: bool,
    runtime_update_applied_verified: bool,
}

#[derive(Debug, Serialize)]
struct CachedUpdateLine {
    displayed_line: String,
    displayed_disabled: bool,
    current_update_applied_verified: bool,
}

#[derive(Debug, Serialize)]
struct CachedUpdateDisplay {
    title_id: String,
    cached_update_lines: Vec<CachedUpdateLine>,
    cached_other_patch_lines: usize,
    cache_may_be_stale: bool,
}

/// Eden's Config::ReadDisabledAddOnValues uses [DisabledAddOns],
/// an unnamed QSettings array, and nested disabled/1/d entries.
/// Saved *disabled* selections do not establish the installed update set.
fn saved_disabled_addons(contents: &str) -> Vec<SavedDisabledAddons> {
    let mut in_section = false;
    let mut entries: BTreeMap<usize, (Option<u64>, BTreeMap<usize, String>)> = BTreeMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line.eq_ignore_ascii_case("[DisabledAddOns]");
            continue;
        }
        if !in_section || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let path = key.trim().replace('\\', "/");
        let fields: Vec<_> = path.split('/').filter(|p| !p.is_empty()).collect();
        if fields.is_empty() {
            continue;
        }
        let Ok(idx) = fields[0].parse::<usize>() else {
            continue;
        };
        if idx == 0 || idx > 1000 {
            continue;
        }
        let value = val.trim();
        match fields.as_slice() {
            [_, "title_id"] => {
                let raw = if let Some(hex) = value
                    .strip_prefix("0x")
                    .or_else(|| value.strip_prefix("0X"))
                {
                    u64::from_str_radix(hex, 16).ok()
                } else {
                    value.parse::<u64>().ok()
                };
                if let Some(raw) = raw {
                    entries.entry(idx).or_default().0 = Some(raw);
                }
            }
            [_, "disabled", sub, "d"] => {
                let Ok(sub) = sub.parse::<usize>() else {
                    continue;
                };
                if sub == 0
                    || sub > 1000
                    || value.is_empty()
                    || value.len() > 180
                    || value.chars().any(char::is_control)
                {
                    continue;
                }
                entries
                    .entry(idx)
                    .or_default()
                    .1
                    .insert(sub, value.to_owned());
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (_idx, (id, values)) in entries {
        let Some(id) = id else { continue };
        let title = format!("{id:016X}");
        if !is_title_id(&title) {
            continue;
        }
        let names: Vec<String> = values.into_values().collect();
        let mut versions = Vec::new();
        for value in &names {
            if let Some(v) = value
                .strip_prefix("Update@")
                .and_then(|v| v.parse::<u32>().ok())
            {
                versions.push(v);
            }
        }
        versions.sort_unstable();
        versions.dedup();
        out.push(SavedDisabledAddons {
            title_id: title,
            generic_update_disabled: names.iter().any(|x| x == "Update"),
            disabled_external_update_versions: versions,
            disabled_addon_names: names,
            runtime_update_applied_verified: false,
        });
    }
    out.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    out
}

/// Eden GameListWorker's FormatPatchNameVersions writes "\n"-separated
/// "Update (version)" or "[D] Update (version)" display rows.
/// Those rows can be stale and are not authenticated program state.
fn parse_cached_patch_display(title_id: &str, text: &str) -> CachedUpdateDisplay {
    let mut update_lines = Vec::new();
    let mut other = 0usize;
    for line in text.lines().take(200) {
        let line = line.trim();
        if line.is_empty() || line.len() > 180 || line.chars().any(char::is_control) {
            continue;
        }
        let (disabled, rest) = if let Some(rest) = line.strip_prefix("[D] ") {
            (true, rest)
        } else {
            (false, line)
        };
        if rest == "Update" || (rest.starts_with("Update (") && rest.ends_with(')')) {
            update_lines.push(CachedUpdateLine {
                displayed_line: line.to_owned(),
                displayed_disabled: disabled,
                current_update_applied_verified: false,
            });
        } else {
            other += 1;
        }
    }
    CachedUpdateDisplay {
        title_id: title_id.to_owned(),
        cached_update_lines: update_lines,
        cached_other_patch_lines: other,
        cache_may_be_stale: true,
    }
}

fn read_cached_update_displays(root: &Path, ids: &[InventoryGame]) -> Vec<CachedUpdateDisplay> {
    let mut displays = Vec::new();
    for title in ids {
        let path = root.join(format!("{}.pv.txt", title.title_id));
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_CACHE_TITLE_BYTES
        {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        if bytes.len() as u64 > MAX_CACHE_TITLE_BYTES {
            continue;
        }
        let Ok(contents) = std::str::from_utf8(&bytes) else {
            continue;
        };
        displays.push(parse_cached_patch_display(&title.title_id, contents));
    }
    displays
}

fn parsed_switch_id(value: &str) -> Option<u64> {
    if !is_title_id(value) {
        return None;
    }
    u64::from_str_radix(value, 16).ok()
}

/// Eden's GetUpdateTitleID(base) uses base | 0x800. This is
/// a *label-level* relation, never proof that an update was applied.
fn base_for_update_label(id: &str) -> Option<String> {
    let raw = parsed_switch_id(id)?;
    if raw & 0xfff != 0x800 {
        return None;
    }
    Some(format!("{:016X}", raw & !0x800))
}

fn is_base_label(id: &str) -> bool {
    parsed_switch_id(id).is_some_and(|raw| raw & 0xfff == 0)
}

/// Recognize directory labels only. This isn't content-type verification.
fn dlc_directory(path: &str) -> Option<String> {
    let mut folder = PathBuf::new();
    for component in Path::new(path).components() {
        folder.push(component.as_os_str());
        let name = component.as_os_str().to_string_lossy();
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "dlc" | "aoc" | "add-ons" | "addons" | "downloadable-content"
        ) {
            return Some(folder.to_string_lossy().into_owned());
        }
    }
    None
}

fn summarize_report(report: &Report) -> InventorySummary {
    let mut base_counts = BTreeMap::<String, usize>::new();
    let cached_ids: BTreeSet<String> = report
        .cached_titles
        .iter()
        .filter(|item| is_base_label(&item.title_id))
        .map(|item| item.title_id.clone())
        .collect();
    for id in &cached_ids {
        base_counts.entry(id.clone()).or_default();
    }

    let mut updates = Vec::new();
    let mut dlc_counts = BTreeMap::<String, usize>::new();
    let mut unresolved = Vec::new();
    let mut not_classified = 0;
    for file in &report.game_files {
        // A user-supplied /dlc/ directory is advisory evidence of add-on
        // organization, not a claim about the game's executable identity.
        if let Some(folder) = dlc_directory(&file.path) {
            *dlc_counts.entry(folder).or_default() += 1;
            continue;
        }
        match file.title_id.as_deref() {
            Some(id) if base_for_update_label(id).is_some() => {
                let base = base_for_update_label(id).expect("previous check");
                base_counts.entry(base.clone()).or_default();
                updates.push(InventoryUpdate {
                    file_path: file.path.clone(),
                    update_title_id: id.to_owned(),
                    probable_base_title_id: base,
                    relationship_evidence: "eden_get_update_title_id_bitmask_on_filename_label",
                    applied_by_eden_verified: false,
                });
            }
            Some(id) if is_base_label(id) => {
                *base_counts.entry(id.to_owned()).or_default() += 1;
            }
            Some(_) => not_classified += 1,
            None => unresolved.push(file.path.clone()),
        }
    }
    let mut per_base_updates = BTreeMap::<String, usize>::new();
    for item in &updates {
        *per_base_updates
            .entry(item.probable_base_title_id.clone())
            .or_default() += 1;
    }
    let candidate_base_titles = base_counts
        .into_iter()
        .map(|(id, count)| {
            let matching = report.title_id_matches.iter().find(|m| m.title_id == id);
            InventoryGame {
                title_id: id.clone(),
                filename_labeled_base_files: count,
                in_eden_cache: cached_ids.contains(&id),
                archived_build_candidate_count: matching
                    .map_or(0, |m| m.archived_build_candidates.len()),
                external_reference_count: matching.map_or(0, |m| m.external_reference_total),
                update_files_in_registered_folders: per_base_updates.get(&id).copied().unwrap_or(0),
                installed_base_game_verified: false,
                installed_update_verified: false,
                installed_build_id_verified: false,
            }
        })
        .collect();
    let uncorrelated_cached = cached_ids
        .into_iter()
        .filter(|id| {
            !report.game_files.iter().any(|file| {
                file.title_id.as_deref() == Some(id.as_str()) && dlc_directory(&file.path).is_none()
            })
        })
        .collect();
    InventorySummary {
        schema_version: 1,
        format: "cheatarium-eden-inventory-summary-v1",
        read_only: true,
        registered_game_roots: report.game_roots.iter().map(|root| GameRoot {
            path: root.path.clone(),
            deep_scan: root.deep_scan,
            status: root.status,
        }).collect(),
        game_file_bytes_read: report.game_file_bytes_read,
        emulator_files_written: report.emulator_files_written,
        total_game_container_files: report.game_files.len(),
        candidate_base_titles,
        probable_update_files: updates,
        dlc_folder_groups: dlc_counts.into_iter().map(|(path, file_count)| InventoryDlcFolder {
            path, file_count, installed_in_eden_verified: false
        }).collect(),
        filename_ids_not_classified: not_classified,
        unresolved_filename_title_ids: unresolved,
        cached_title_ids_without_labeled_base_file: uncorrelated_cached,
        unresolved_nsp_header_inspections: Vec::new(),
        unresolved_nsp_inspections_truncated: false,
        inspected_nsp_file_bytes_read: 0,
        saved_disabled_addons: Vec::new(),
        cached_update_displays: Vec::new(),
        installed_build_id_verified: false,
        warnings: vec![
            "Directory/filename and cached Title IDs are advisory; files may be renamed or removed.",
            "Folder labels such as dlc are user organization, not authenticated Switch content types.",
            "An update NSP in a registered folder does not mean Eden installed or selected it.",
            "Only main executable metadata from the runtime or authenticated game content can verify installed Build IDs.",
            "Eden settings and cached update display are read-only snapshots; neither verifies which executable or update was applied.",
            "No ROM contents or keys were accessed and no emulator settings were modified.",
        ],
    }
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
    // Eden's Qt file normally puts Paths\gamedirs keys inside [UI].
    // Older/custom Qt configurations can instead use a dedicated [Paths].
    let mut section = "";
    let mut entries: BTreeMap<usize, (Option<String>, bool)> = BTreeMap::new();
    let mut legacy = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = if line.eq_ignore_ascii_case("[UI]") {
                "UI"
            } else if line.eq_ignore_ascii_case("[Paths]") {
                "Paths"
            } else {
                ""
            };
            continue;
        }
        if section.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let normalized = key.trim().replace('\\', "/");
        let key = if section == "UI" {
            let Some(key) = normalized.strip_prefix("Paths/") else {
                continue;
            };
            key
        } else {
            normalized.as_str()
        };
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
fn cached_title_id(name: &str) -> Option<(String, bool)> {
    let (stem, is_appname) = if let Some(stem) = name.strip_suffix(".appname.txt") {
        (stem, true)
    } else if let Some(stem) = name.strip_suffix(".pv.txt") {
        // Eden's version/patch display cache is title-ID keyed too.
        (stem, false)
    } else {
        return None;
    };
    if !is_title_id(stem) {
        return None;
    }
    Some((stem.to_ascii_uppercase(), is_appname))
}

fn cache_titles(root: &Path, warnings: &mut Vec<String>) -> Vec<CachedTitle> {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return Vec::new();
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        warnings.push("eden_game_cache_not_a_plain_directory".to_owned());
        return Vec::new();
    }
    let Ok(dir) = fs::read_dir(root) else {
        warnings.push("eden_game_cache_unreadable".to_owned());
        return Vec::new();
    };
    let mut names = Vec::new();
    for entry in dir {
        if names.len() >= MAX_ENTRIES {
            warnings.push("eden_game_cache_entry_limit_reached".to_owned());
            return Vec::new();
        }
        let Ok(entry) = entry else {
            warnings.push("eden_game_cache_entry_unreadable".to_owned());
            continue;
        };
        names.push(entry);
    }
    names.sort_by_key(|entry| entry.file_name());
    let mut matches = BTreeMap::<String, CachedTitle>::new();
    for entry in names {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((id, appname)) = cached_title_id(&name) else {
            continue;
        };
        // Never follow symlinks into unrelated files.
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > MAX_CACHE_TITLE_BYTES {
            warnings.push(format!(
                "eden_cache_title_skipped_nonregular_or_oversized: {id}"
            ));
            continue;
        }
        // A pv.txt is only a filename witness: its contents are patch/version
        // presentation text, NOT the game's name or executable Build ID.
        if !appname {
            match matches.entry(id.clone()) {
                std::collections::btree_map::Entry::Vacant(v) => {
                    v.insert(CachedTitle {
                        title_id: id,
                        title_name: None,
                        evidence: "eden_game_list_cache_pv_filename",
                        installed_game_verified: false,
                    });
                }
                std::collections::btree_map::Entry::Occupied(mut o) => {
                    o.get_mut().evidence = "eden_game_list_cache_appname_and_pv";
                }
            }
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
            Ok(title_name) => {
                let prior_pv = matches.contains_key(&id);
                matches.insert(
                    id.clone(),
                    CachedTitle {
                        title_id: id,
                        title_name,
                        evidence: if prior_pv {
                            "eden_game_list_cache_appname_and_pv"
                        } else {
                            "eden_game_list_cache_appname_filename_and_optional_title"
                        },
                        installed_game_verified: false,
                    },
                );
            }
            Err(_) => warnings.push(format!("eden_cache_title_unreadable: {id}")),
        }
    }
    matches.into_values().collect()
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
    let mut summary = false;
    let mut inspect_nsp_headers = false;
    let mut explicit_ids = BTreeSet::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--config" => config = PathBuf::from(args.next().ok_or("--config requires a path")?),
            "--cache-dir" => {
                cache_dir = PathBuf::from(args.next().ok_or("--cache-dir requires a path")?)
            }
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
            "--summary" => summary = true,
            "--inspect-nsp-headers" => inspect_nsp_headers = true,
            "--help" | "-h" => {
                println!("Read-only Eden discovery:\n  cheatarium-eden-discover [--db generated/v1] [--config ~/.config/eden/qt-config.ini] [--cache-dir ~/.cache/eden/game_list] [--game-dir DIR] [--title-id 0100000000010000] [--summary] [--inspect-nsp-headers] [--json]\nNo ROM contents or emulator files are changed.");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    if inspect_nsp_headers && !summary {
        return Err("--inspect-nsp-headers requires --summary".into());
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
        let refs = external_coverage
            .get(&id)
            .ok_or("Missing external coverage result")?;
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
    if summary {
        let mut summary = summarize_report(&report);
        if report.config_found {
            summary.saved_disabled_addons = saved_disabled_addons(&fs::read_to_string(&config)?);
        }
        summary.cached_update_displays =
            read_cached_update_displays(&cache_dir, &summary.candidate_base_titles);
        if inspect_nsp_headers {
            let filenames: Vec<_> = report
                .game_files
                .iter()
                .filter(|f| f.title_id.is_none() && f.file_type == "nsp")
                .take(MAX_NSP_HEADER_INSPECTIONS + 1)
                .collect();
            summary.unresolved_nsp_inspections_truncated =
                filenames.len() > MAX_NSP_HEADER_INSPECTIONS;
            summary.unresolved_nsp_header_inspections = filenames
                .into_iter()
                .take(MAX_NSP_HEADER_INSPECTIONS)
                .map(|file| inspect_unresolved_nsp(&file.path))
                .collect();
            summary.inspected_nsp_file_bytes_read = summary
                .unresolved_nsp_header_inspections
                .iter()
                .map(|entry| entry.bytes_read)
                .sum();
            summary.game_file_bytes_read = summary.inspected_nsp_file_bytes_read as u32;
        }
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
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
            cached_title_id("0100000000010000.appname.txt"),
            Some(("0100000000010000".to_owned(), true))
        );
        assert_eq!(
            cached_title_id("0100152000022000.pv.txt"),
            Some(("0100152000022000".to_owned(), false))
        );
        assert!(cached_title_id("0100000000010000.jpeg").is_none());
        assert!(cached_title_id("foo0100000000010000.pv.txt").is_none());
        assert!(cached_title_id("0100000000010000.pv.txt.bak").is_none());
    }

    #[test]
    fn parses_real_eden_ui_namespaced_qsettings_keys() {
        let ini = "[UI]\nPaths\\gamedirs\\size=4\nUIGameList\\cache_game_list=true\nPaths\\gamedirs\\1\\path=SDMC\nPaths\\gamedirs\\2\\path=UserNAND\nPaths\\gamedirs\\3\\path=SysNAND\nPaths\\gamedirs\\4\\path=/drive/roms/switch/roms\nPaths\\gamedirs\\4\\deep_scan\\default=false\nPaths\\gamedirs\\4\\deep_scan=true\n";
        assert_eq!(
            configured_game_dirs(ini),
            vec![(PathBuf::from("/drive/roms/switch/roms"), true)]
        );
    }

    #[test]
    fn reads_eden_patch_cache_even_without_game_name() {
        let path = env::temp_dir().join(format!(
            "cheatarium-eden-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&path).unwrap();
        // This is exactly the cache-file shape present in the local Eden report.
        fs::write(path.join("0100000000010000.pv.txt"), b"v1.0.0").unwrap();
        fs::write(path.join("0100152000022000.pv.txt"), b"v3.0.0").unwrap();
        fs::write(path.join("01006A800016E000.pv.txt"), b"").unwrap();
        fs::write(
            path.join("0100152000022000.appname.txt"),
            b"Mario Kart 8 Deluxe",
        )
        .unwrap();
        fs::write(path.join("0100152000022000.jpeg"), b"not read").unwrap();
        fs::write(path.join("ignore.appname.txt"), b"ignored").unwrap();
        let mut warnings = Vec::new();
        let games = cache_titles(&path, &mut warnings);
        assert_eq!(games.len(), 3);
        let odyssey = games
            .iter()
            .find(|g| g.title_id == "0100000000010000")
            .unwrap();
        assert_eq!(odyssey.title_name, None);
        assert_eq!(odyssey.evidence, "eden_game_list_cache_pv_filename");
        let kart = games
            .iter()
            .find(|g| g.title_id == "0100152000022000")
            .unwrap();
        assert_eq!(kart.title_name.as_deref(), Some("Mario Kart 8 Deluxe"));
        assert_eq!(kart.evidence, "eden_game_list_cache_appname_and_pv");
        assert!(games.iter().all(|g| !g.installed_game_verified));
        assert!(warnings.is_empty());
        fs::remove_dir_all(&path).unwrap();
    }

    #[test]
    fn update_title_label_uses_edens_base_or_0x800_rule() {
        assert_eq!(
            base_for_update_label("0100000000010800").as_deref(),
            Some("0100000000010000")
        );
        assert_eq!(
            base_for_update_label("01006A800016E800").as_deref(),
            Some("01006A800016E000")
        );
        assert!(base_for_update_label("01006A800016F006").is_none());
        assert!(base_for_update_label("garbage").is_none());
        assert!(is_base_label("0100000000010000"));
    }

    #[test]
    fn dlc_directory_is_a_path_hint_only() {
        assert_eq!(
            dlc_directory("/games/super-smash/dlc/Cosmetic [01006A800016F040].nsp").as_deref(),
            Some("/games/super-smash/dlc")
        );
        assert!(dlc_directory("/games/super-smash/base-game/Game.nsp").is_none());
    }

    #[test]
    fn reads_exact_eden_disabled_addon_array_without_implying_runtime_state() {
        let ini = r#"[UI]
Paths\gamedirs\1\path=/games
[DisabledAddOns]
size=2
1\title_id=72057594037993472
1\disabled\size=3
1\disabled\1\d=Update@262144
1\disabled\2\d=Update
1\disabled\3\d=Some Cosmetic Mod
2\title_id=garbage
2\disabled\1\d=Update@1
[Audio]
volume=100
"#;
        // Use a decimal title_id as Eden's QSettings serializer does.
        let title_id = format!("{:016X}", 72057594037993472u64);
        let parsed = saved_disabled_addons(ini);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title_id, title_id);
        assert_eq!(parsed[0].disabled_external_update_versions, vec![262144]);
        assert!(parsed[0].generic_update_disabled);
        assert_eq!(parsed[0].disabled_addon_names.len(), 3);
        assert!(!parsed[0].runtime_update_applied_verified);
    }

    #[test]
    fn parses_cached_update_rows_but_preserves_staleness_warning() {
        let entry = parse_cached_patch_display(
            "0100000000010000",
            "Update (1.3.0)\n[D] Update (1.1.0)\nSome Mod (0.2)\n",
        );
        assert_eq!(entry.cached_update_lines.len(), 2);
        assert_eq!(
            entry.cached_update_lines[0].displayed_line,
            "Update (1.3.0)"
        );
        assert!(entry.cached_update_lines[1].displayed_disabled);
        assert_eq!(entry.cached_other_patch_lines, 1);
        assert!(entry.cache_may_be_stale);
        assert!(entry
            .cached_update_lines
            .iter()
            .all(|x| !x.current_update_applied_verified));
    }

    fn write_synthetic_nsp(path: &Path, names: &[(&str, &[u8])]) {
        let mut strings = Vec::new();
        let mut entries = Vec::new();
        let mut data = Vec::new();
        for &(name, bytes) in names {
            let offset = data.len() as u64;
            let name_at = strings.len() as u32;
            strings.extend_from_slice(name.as_bytes());
            strings.push(0);
            entries.extend_from_slice(&offset.to_le_bytes());
            entries.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            entries.extend_from_slice(&name_at.to_le_bytes());
            entries.extend_from_slice(&[0u8; 4]);
            data.extend_from_slice(bytes);
        }
        let mut out = b"PFS0".to_vec();
        out.extend_from_slice(&(names.len() as u32).to_le_bytes());
        out.extend_from_slice(&(strings.len() as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 4]);
        out.extend(entries);
        out.extend(strings);
        out.extend(data);
        fs::write(path, out).unwrap();
    }

    #[test]
    fn bounded_pfs0_inspection_only_recovers_untrusted_ticket_filename_hints() {
        let p = env::temp_dir().join(format!(
            "cheatarium-pfs0-{}-{:?}.nsp",
            std::process::id(),
            std::thread::current().id()
        ));
        write_synthetic_nsp(
            &p,
            &[
                (
                    "0100152000022000abcdef1234567890.tik",
                    b"private ticket data",
                ),
                ("abcd.cnmt.nca", b"encrypted meta"),
                ("abcd.nca", b"encrypted executable"),
            ],
        );
        let r = inspect_unresolved_nsp(p.to_str().unwrap());
        assert_eq!(r.status, "outer_pfs0_metadata_only");
        assert_eq!(r.nca_file_entries, 2);
        assert_eq!(r.cnmt_nca_file_entries, 1);
        assert_eq!(
            r.ticket_filename_title_id_candidates,
            vec!["0100152000022000"]
        );
        assert!(!r.executable_build_id_found);
        assert!(!r.installed_build_id_verified);
        assert!(r.bytes_read > 16);
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn rejects_malformed_outer_pfs0_without_reading_past_prefix() {
        let p = env::temp_dir().join(format!(
            "cheatarium-pfs0-malformed-{}-{:?}.nsp",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::write(&p, b"PFS0").unwrap();
        assert_eq!(
            inspect_unresolved_nsp(p.to_str().unwrap()).status,
            "too_short"
        );
        // Oversized count is rejected before any table allocation.
        let mut bytes = b"PFS0".to_vec();
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        fs::write(&p, &bytes).unwrap();
        assert_eq!(
            inspect_unresolved_nsp(p.to_str().unwrap()).status,
            "too_many_entries"
        );
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn filters_extensions_and_rejects_other_files() {
        assert_eq!(game_suffix(Path::new("a.NSP")).as_deref(), Some("nsp"));
        assert!(game_suffix(Path::new("prod.keys")).is_none());
        assert!(game_suffix(Path::new("notes.txt")).is_none());
    }
}
