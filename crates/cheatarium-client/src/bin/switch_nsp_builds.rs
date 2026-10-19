//! One-shot, offline NSP program Build ID inspection using local nstool and hactool.
//! Reads only user-selected packages, stages files in a private temporary directory,
//! and never changes Eden, installed cheats, saves, or the source NSP.
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_NSP_COUNT: usize = 4096;
const DEFAULT_MAX_NCA_GIB: u64 = 32;
const ABSOLUTE_MAX_NCA_GIB: u64 = 64;
const MAX_META_BYTES: u64 = 48 * 1024 * 1024;
const MAX_COMMAND_OUTPUT: usize = 1024 * 1024;

#[derive(Debug)]
struct NcaEntry {
    name: String,
    size: u64,
}
#[derive(Debug, Clone)]
struct Package {
    file: PathBuf,
    title: String,
    version: u32,
    kind: String,
    program: String,
    program_size: u64,
}
#[derive(Serialize)]
struct Found {
    title_id: String,
    source_kind: String,
    version: u32,
    build_id: Option<String>,
    /// 16 hexadecimal characters used for Switch cheat filenames.
    cheat_build_id: Option<String>,
    status: String,
}
#[derive(Serialize)]
struct Report {
    format: &'static str,
    read_only: bool,
    source_packages_examined: usize,
    eden_files_written: usize,
    source_files_modified: usize,
    temporary_files_removed_on_exit: bool,
    results: Vec<Found>,
    warnings: Vec<String>,
    /// Files found in formats this version does not decrypt (e.g. XCI).
    source_file_counts_by_extension: BTreeMap<String, usize>,
}

struct Scratch(PathBuf);
impl Scratch {
    fn default_root() -> Result<PathBuf, ErrBox> {
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME_not_set")?);
        Ok(home.join(".cache/cheatarium/nsp-temporary"))
    }
    fn new() -> Result<Self, ErrBox> {
        Self::new_at(&Self::default_root()?)
    }
    fn new_at(cache: &Path) -> Result<Self, ErrBox> {
        if !cache.is_absolute()
            || cache.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            return Err("scratch_dir_must_be_absolute_and_normalized".into());
        }
        // Refuse pre-existing symlinks in an external scratch path.
        let mut parent = PathBuf::new();
        for component in cache.components() {
            parent.push(component);
            match fs::symlink_metadata(&parent) {
                Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
                Ok(_) => return Err("scratch_dir_contains_symlink_or_non_directory".into()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        fs::create_dir_all(cache)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = cache.join(format!("cheatarium-nsp-{}-{now}", std::process::id()));
        fs::create_dir(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self(root))
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn safe_output(tool: &str, args: &[String]) -> Result<String, String> {
    let result = Command::new(tool)
        .args(args)
        .output()
        .map_err(|_| format!("{tool}_missing_or_cannot_execute"))?;
    // Do not expose stdout/stderr: external tools may include keys or raw package paths.
    if !result.status.success() {
        return Err(format!("{tool}_exit_failure"));
    }
    if result.stdout.len() > MAX_COMMAND_OUTPUT {
        return Err(format!("{tool}_output_too_large"));
    }
    String::from_utf8(result.stdout).map_err(|_| format!("{tool}_non_utf8_output"))
}
fn hex_identifier(x: &str, n: usize) -> Option<String> {
    let v = x.trim().trim_start_matches("0x");
    (v.len() == n && v.bytes().all(|c| c.is_ascii_hexdigit())).then(|| v.to_ascii_uppercase())
}
fn line_value<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(label).map(str::trim))
}
fn parse_content_metadata(out: &str) -> Option<(String, u32, String, String)> {
    let id = hex_identifier(line_value(out, "TitleId:")?, 16)?;
    let type_name = line_value(out, "Type:")?;
    let kind = type_name.split_whitespace().next()?.to_ascii_lowercase();
    if kind != "application" && kind != "patch" {
        return None;
    }
    let ver_text = line_value(out, "Version:")?;
    let raw = ver_text.split("(v").nth(1)?.split(')').next()?;
    let version = raw.parse::<u32>().ok()?;
    let title = if kind == "patch" {
        hex_identifier(line_value(out, "ApplicationId:")?, 16)?
    } else {
        id.clone()
    };
    if !title.starts_with("01") {
        return None;
    }
    let contents = out.split("ContentInfo:").nth(1)?;
    let mut program = None;
    let mut awaiting_id = false;
    for line in contents.lines() {
        let part = line.trim();
        if part.starts_with("ContentMetaInfo:") || part.starts_with("Digest:") {
            break;
        }
        if let Some(ty) = part.strip_prefix("Type:") {
            awaiting_id = ty.trim().starts_with("Program ");
        } else if awaiting_id {
            if let Some(value) = part.strip_prefix("Id:") {
                program = hex_identifier(value, 32).map(|s| format!("{}.nca", s.to_lowercase()));
                break;
            }
        }
    }
    Some((title, version, kind, program?))
}
fn scan_pfs0(path: &Path) -> Result<Vec<NcaEntry>, ErrBox> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut hdr = [0u8; 16];
    f.read_exact(&mut hdr)?;
    if &hdr[..4] != b"PFS0" {
        return Err("not_pfs0_nsp".into());
    }
    let count = u32::from_le_bytes(hdr[4..8].try_into()?) as usize;
    let str_size = u32::from_le_bytes(hdr[8..12].try_into()?) as usize;
    if count == 0 || count > 16384 || str_size > 4 * 1024 * 1024 {
        return Err("invalid_pfs0_header".into());
    }
    let entries_size = count.checked_mul(24).ok_or("entry_count_overflow")?;
    let payload_offset = 16usize
        .checked_add(entries_size)
        .and_then(|n| n.checked_add(str_size))
        .ok_or("nsp_header_overflow")?;
    if payload_offset as u64 > len {
        return Err("truncated_pfs0_header".into());
    }
    let mut table = vec![0u8; entries_size + str_size];
    f.read_exact(&mut table)?;
    let names = &table[entries_size..];
    let mut out = Vec::new();
    for i in 0..count {
        let p = &table[i * 24..(i + 1) * 24];
        let at = u64::from_le_bytes(p[..8].try_into()?);
        let size = u64::from_le_bytes(p[8..16].try_into()?);
        let index = u32::from_le_bytes(p[16..20].try_into()?) as usize;
        if (index >= names.len())
            || at
                .checked_add(size)
                .and_then(|n| n.checked_add(payload_offset as u64))
                .is_none_or(|end| end > len)
        {
            return Err("invalid_pfs0_entry_bounds".into());
        }
        let end = names[index..]
            .iter()
            .position(|c| *c == 0)
            .ok_or("invalid_pfs0_name")?
            + index;
        let name = std::str::from_utf8(&names[index..end])?;
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err("unsafe_pfs0_entry_name".into());
        }
        if name.to_ascii_lowercase().ends_with(".nca")
            || name.to_ascii_lowercase().ends_with(".tik")
        {
            out.push(NcaEntry {
                name: name.to_owned(),
                size,
            });
        }
    }
    Ok(out)
}
fn keyfile(specified: Option<PathBuf>) -> Result<PathBuf, ErrBox> {
    let mut candidates = Vec::new();
    if let Some(path) = specified {
        candidates.push(path);
    } else {
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME_not_set")?);
        candidates.extend([
            home.join(".switch/prod.keys"),
            home.join(".local/share/eden/keys/prod.keys"),
            home.join(".config/eden/keys/prod.keys"),
        ]);
    }
    for path in candidates {
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if meta.is_file()
                && !meta.file_type().is_symlink()
                && meta.len() > 0
                && meta.len() < 256 * 1024
            {
                return Ok(path);
            }
        }
    }
    Err("missing_local_prod_keys_supply_--keyset_path_without_printing_keys".into())
}
fn extract_nca(
    nsp: &Path,
    entry: &NcaEntry,
    output: &Path,
    limit_bytes: u64,
) -> Result<(), String> {
    if entry.size > limit_bytes {
        return Err("nca_exceeds_configured_safety_limit".into());
    }
    let args = vec![
        "-t".into(),
        "nsp".into(),
        "-x".into(),
        format!("/{}", entry.name),
        output.to_string_lossy().to_string(),
        nsp.to_string_lossy().to_string(),
    ];
    safe_output("nstool", &args)?;
    let size = fs::symlink_metadata(output).map_err(|_| "nca_not_extracted".to_owned())?;
    if !size.is_file() || size.is_symlink() || size.len() != entry.size {
        return Err("nca_extract_size_mismatch".into());
    }
    Ok(())
}
// NSTool's --fstree text is useful for users but is not a machine-readable
// manifest. Extract only the bounded metadata NCA into the private scratch tree
// when filename discovery or the single-file extraction path fails.
fn bounded_cnmt_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut entries = 0usize;
    while let Some((dir, depth)) = pending.pop() {
        if depth > 5 {
            return Err("metadata_directory_too_deep".into());
        }
        for entry in fs::read_dir(&dir).map_err(|_| "cannot_read_extracted_metadata".to_owned())? {
            entries += 1;
            if entries > 4096 {
                return Err("too_many_extracted_metadata_entries".into());
            }
            let entry = entry.map_err(|_| "cannot_read_extracted_metadata_entry".to_owned())?;
            let ty = entry
                .file_type()
                .map_err(|_| "cannot_stat_extracted_metadata_entry".to_owned())?;
            if ty.is_symlink() {
                return Err("extracted_metadata_symlink_refused".into());
            }
            if ty.is_dir() {
                pending.push((entry.path(), depth + 1));
            } else if ty.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .ends_with(".cnmt")
            {
                let meta = entry
                    .metadata()
                    .map_err(|_| "cannot_stat_cnmt".to_owned())?;
                if meta.len() > MAX_META_BYTES {
                    return Err("extracted_cnmt_too_large".into());
                }
                files.push(entry.path());
                if files.len() > 64 {
                    return Err("too_many_extracted_cnmt_files".into());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

// The content metadata NCA can itself be rights-encrypted. Supply the
// containing NSP's tickets to NSTool before attempting its filesystem,
// exactly as we already do for Program NCA extraction.
fn nca_nstool_args(keys: &Path, tickets: &[PathBuf]) -> Vec<String> {
    let mut args = vec![
        "-t".into(),
        "nca".into(),
        "-k".into(),
        keys.to_string_lossy().to_string(),
    ];
    for ticket in tickets {
        args.extend(["--tik".into(), ticket.to_string_lossy().to_string()]);
    }
    args
}
// Do not include raw tool output or key material in user-visible reports.
// Distinguish genuinely absent game metadata from an encrypted partition that
// the installed keyset cannot open. No cryptographic key values are reported.
fn discovery_blocked_status(reason: &str) -> &'static str {
    if reason.contains("_missing_keys_")
        && reason
            .rsplit("_missing_keys_")
            .next()
            .and_then(|count| count.parse::<usize>().ok())
            .is_some_and(|count| count > 0)
    {
        "blocked_metadata_decryption_key_unavailable"
    } else if reason.starts_with("metadata_unresolved") {
        "blocked_metadata_unresolved"
    } else if reason == "metadata_missing_cnmt_nca" {
        "blocked_missing_metadata_nca"
    } else if reason == "invalid_nsp_pfs0" {
        "blocked_invalid_nsp"
    } else {
        "blocked_metadata_inspection_failed"
    }
}
fn metadata_partition_diagnosis(output: &str) -> (usize, usize) {
    let unreadable = output
        .lines()
        .filter(|line| line.contains("NCA Partition") && line.contains("not readable"))
        .count();
    let missing_keys = output
        .lines()
        .filter(|line| {
            line.contains("AES-CTR Key was not determined")
                || line.contains("key could not be loaded")
                || line.contains("could not determine key")
        })
        .count();
    (unreadable, missing_keys)
}

fn discover_package(
    path: &Path,
    keys: &Path,
    scratch: &Scratch,
) -> Result<Option<Package>, String> {
    let items = scan_pfs0(path).map_err(|_| "invalid_nsp_pfs0".to_owned())?;
    let metas = items
        .iter()
        .filter(|it| it.name.to_ascii_lowercase().ends_with(".cnmt.nca"))
        .collect::<Vec<_>>();
    if metas.is_empty() {
        return Err("metadata_missing_cnmt_nca".into());
    }
    let mut found = Vec::new();
    let mut extracted_meta = 0usize;
    let mut readable_tree = 0usize;
    let mut cnmt_paths = 0usize;
    let mut extracted_cnmt = 0usize;
    let mut parsed_cnmt = 0usize;
    let mut matched_program = 0usize;
    let mut fallback_cnmt_paths = 0usize;
    let mut fallback_parsed_cnmts = 0usize;
    let mut fallback_matched_programs = 0usize;
    let mut fallback_extractions = 0usize;
    let mut unreadable_metadata_partitions = 0usize;
    let mut missing_metadata_decryption_keys = 0usize;
    let tickets = ticket_sources(path, scratch, "metadata")?;
    let nstool_base_args = nca_nstool_args(keys, &tickets);
    for (i, meta) in metas.iter().enumerate() {
        if meta.size > MAX_META_BYTES {
            continue;
        }
        let dest = scratch.path(&format!("meta-{i}.nca"));
        if extract_nca(path, meta, &dest, MAX_META_BYTES).is_err() {
            continue;
        }
        extracted_meta += 1;
        let mut tree_args = nstool_base_args.clone();
        tree_args.extend(["--fstree".into(), dest.to_string_lossy().to_string()]);
        let tree = match safe_output("nstool", &tree_args) {
            Ok(tree) => {
                readable_tree += 1;
                tree
            }
            Err(_) => continue,
        };
        let (bad_partitions, missing_keys) = metadata_partition_diagnosis(&tree);
        unreadable_metadata_partitions += bad_partitions;
        missing_metadata_decryption_keys += missing_keys;
        // Metadata NCAs have a small CNMT in one of the archive partitions.
        // A known virtual file tree is required; do not guess content IDs.
        let names = tree
            .lines()
            .filter_map(|line| {
                line.split_whitespace()
                    .find(|token| token.ends_with(".cnmt"))
                    .map(|token| {
                        token
                            .trim_matches(|c: char| c == '|' || c == '-' || c == '\\')
                            .to_owned()
                    })
            })
            .collect::<Vec<_>>();
        cnmt_paths += names.len();
        for (j, name) in names.iter().enumerate() {
            let cnmt = scratch.path(&format!("metadata-{i}-{j}.cnmt"));
            let virtual_paths = [format!("/0/{name}"), format!("/1/{name}")];
            for vp in virtual_paths {
                let _ = fs::remove_file(&cnmt);
                let mut args = nstool_base_args.clone();
                args.extend([
                    "-x".into(),
                    vp,
                    cnmt.to_string_lossy().to_string(),
                    dest.to_string_lossy().to_string(),
                ]);
                if safe_output("nstool", &args).is_err() {
                    continue;
                }
                extracted_cnmt += 1;
                let info = safe_output(
                    "nstool",
                    &[
                        "-t".into(),
                        "cnmt".into(),
                        cnmt.to_string_lossy().to_string(),
                    ],
                )
                .unwrap_or_default();
                if let Some((title, version, kind, program)) = parse_content_metadata(&info) {
                    parsed_cnmt += 1;
                    if let Some(entry) = items
                        .iter()
                        .find(|item| item.name.eq_ignore_ascii_case(&program))
                    {
                        matched_program += 1;
                        found.push(Package {
                            file: path.to_path_buf(),
                            title,
                            version,
                            kind,
                            program: entry.name.clone(),
                            program_size: entry.size,
                        });
                        break;
                    }
                }
            }
        }
        // When stdout's human-readable tree has no recognizable .cnmt name
        // (as observed on Super Mario RPG), or single-file path discovery
        // fails, let NSTool itself extract the bounded metadata filesystem.
        // This avoids assuming CNMT filenames have a fixed printed layout.
        if found.iter().all(|pkg| pkg.file != path.to_path_buf()) {
            let extracted_root = scratch.path(&format!("meta-{i}-extracted"));
            if fs::create_dir(&extracted_root).is_ok() {
                let mut full_meta_args = nstool_base_args.clone();
                full_meta_args.extend([
                    "-x".into(),
                    extracted_root.to_string_lossy().to_string(),
                    dest.to_string_lossy().to_string(),
                ]);
                if safe_output("nstool", &full_meta_args).is_ok() {
                    fallback_extractions += 1;
                    if let Ok(extracted_cnmts) = bounded_cnmt_files(&extracted_root) {
                        fallback_cnmt_paths += extracted_cnmts.len();
                        for cnmt in extracted_cnmts {
                            let info = safe_output(
                                "nstool",
                                &[
                                    "-t".into(),
                                    "cnmt".into(),
                                    cnmt.to_string_lossy().to_string(),
                                ],
                            )
                            .unwrap_or_default();
                            if let Some((title, version, kind, program)) =
                                parse_content_metadata(&info)
                            {
                                fallback_parsed_cnmts += 1;
                                if let Some(entry) = items
                                    .iter()
                                    .find(|item| item.name.eq_ignore_ascii_case(&program))
                                {
                                    fallback_matched_programs += 1;
                                    found.push(Package {
                                        file: path.to_path_buf(),
                                        title,
                                        version,
                                        kind,
                                        program: entry.name.clone(),
                                        program_size: entry.size,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            let _ = fs::remove_dir_all(extracted_root);
        }
        let _ = fs::remove_file(dest);
    }
    found.sort_by(|a, b| b.version.cmp(&a.version));
    match found.into_iter().next() {
        Some(pkg) => Ok(Some(pkg)),
        None => Err(format!(
            "metadata_unresolved_meta_ncas_{}_extracted_{}_trees_{}_cnmt_paths_{}_extracted_cnmts_{}_parsed_cnmts_{}_matched_programs_{}_fallback_extractions_{}_fallback_cnmt_paths_{}_fallback_parsed_{}_fallback_matched_{}_ticket_count_{}_unreadable_partitions_{}_missing_keys_{}",
            metas.len(), extracted_meta, readable_tree, cnmt_paths,
            extracted_cnmt, parsed_cnmt, matched_program,
            fallback_extractions, fallback_cnmt_paths, fallback_parsed_cnmts, fallback_matched_programs,
            tickets.len(), unreadable_metadata_partitions, missing_metadata_decryption_keys
        )),
    }
}
fn scan_paths(root: &Path, max: usize) -> Result<Vec<PathBuf>, ErrBox> {
    if !root.is_dir() {
        return Err("NSP source directory is not readable".into());
    }
    let mut files = Vec::new();
    let mut dirs = vec![(root.to_owned(), 0usize)];
    while let Some((dir, depth)) = dirs.pop() {
        if depth > 10 {
            continue;
        }
        for item in fs::read_dir(dir)? {
            let item = item?;
            let ty = item.file_type()?;
            if ty.is_symlink() {
                continue;
            }
            if ty.is_dir() {
                // DLC packages don't contain a base game's Program NCA.
                if item
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("dlc")
                {
                    continue;
                }
                dirs.push((item.path(), depth + 1));
            } else if ty.is_file()
                && item
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("nsp"))
            {
                files.push(item.path());
                if files.len() > max {
                    return Err("too_many_nsp_packages_use_--limit".into());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}
fn discover_other_formats(root: &Path) -> Result<BTreeMap<String, usize>, ErrBox> {
    let mut counts = BTreeMap::new();
    let mut dirs = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = dirs.pop() {
        if depth > 10 {
            continue;
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if !entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("dlc")
                {
                    dirs.push((entry.path(), depth + 1));
                }
            } else if kind.is_file() {
                if let Some(ext) = entry.path().extension().and_then(|x| x.to_str()) {
                    let ext = ext.to_ascii_lowercase();
                    if ["xci", "nsz", "xcz", "nsp"].contains(&ext.as_str()) {
                        *counts.entry(ext).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    Ok(counts)
}
fn main_nso_build(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|_| "missing_main_nso".to_owned())?;
    if f.metadata()
        .map_err(|_| "missing_main_nso".to_owned())?
        .len()
        < 0x60
    {
        return Err("short_main_nso".into());
    }
    let mut hdr = [0u8; 0x60];
    f.read_exact(&mut hdr)
        .map_err(|_| "cannot_read_main_nso".to_owned())?;
    if &hdr[..4] != b"NSO0" {
        return Err("invalid_main_nso_magic".into());
    }
    let id = &hdr[0x40..0x50];
    if id.iter().all(|b| *b == 0) {
        return Err("zero_main_build_id".into());
    }
    Ok(id.iter().map(|b| format!("{b:02X}")).collect())
}
fn ticket_sources(nsp: &Path, scratch: &Scratch, prefix: &str) -> Result<Vec<PathBuf>, String> {
    let entries = scan_pfs0(nsp).map_err(|_| "invalid_nsp".to_owned())?;
    let mut out = Vec::new();
    for (i, ticket) in entries
        .iter()
        .filter(|entry| entry.name.to_ascii_lowercase().ends_with(".tik"))
        .enumerate()
    {
        if i >= 16 {
            return Err("too_many_package_tickets".into());
        }
        if ticket.size > 64 * 1024 {
            return Err("oversized_package_ticket".into());
        }
        let dest = scratch.path(&format!("{prefix}-ticket-{i}.tik"));
        extract_nca(nsp, ticket, &dest, 64 * 1024)?;
        out.push(dest);
    }
    Ok(out)
}
fn extract_build(
    package: &Package,
    base: Option<&Package>,
    keys: &Path,
    scratch: &Scratch,
    limit_bytes: u64,
) -> Result<String, String> {
    let source_item = NcaEntry {
        name: package.program.clone(),
        size: package.program_size,
    };
    let update_nca = scratch.path("selected-program.nca");
    extract_nca(&package.file, &source_item, &update_nca, limit_bytes)?;
    let base_file = scratch.path("base-program.nca");
    if let Some(original) = base {
        let base_entry = NcaEntry {
            name: original.program.clone(),
            size: original.program_size,
        };
        extract_nca(&original.file, &base_entry, &base_file, limit_bytes)?;
    }
    let mut tickets = ticket_sources(&package.file, scratch, "selected")?;
    if let Some(original) = base {
        tickets.extend(ticket_sources(&original.file, scratch, "base")?);
    }
    let output = scratch.path("exefs");
    fs::create_dir(&output).map_err(|_| "cannot_create_private_exefs".to_owned())?;
    // First try NSTool. Unlike raw hactool this accepts NSP-local encrypted tickets,
    // so an ordinary legally dumped game need not have external title.keys.
    let mut nstool_args = vec![
        "-t".into(),
        "nca".into(),
        "-k".into(),
        keys.to_string_lossy().to_string(),
    ];
    if base.is_some() {
        nstool_args.extend(["--basenca".into(), base_file.to_string_lossy().to_string()]);
    }
    for ticket in tickets {
        nstool_args.extend(["--tik".into(), ticket.to_string_lossy().to_string()]);
    }
    let main = output.join("main");
    let mut nstool_status = "nstool_cannot_extract_main_nso".to_owned();
    for partition in [0, 1, 2, 3] {
        let _ = fs::remove_file(&main);
        let mut opts = nstool_args.clone();
        opts.extend([
            "-x".into(),
            format!("/{partition}/main"),
            main.to_string_lossy().to_string(),
            update_nca.to_string_lossy().to_string(),
        ]);
        if safe_output("nstool", &opts).is_ok() {
            if let Ok(build) = main_nso_build(&main) {
                return Ok(build);
            }
        }
    }
    // Fallback for NCAs which hactool can extract without an NSP-local ticket.
    let mut opts = vec![
        "-i".into(),
        "-t".into(),
        "nca".into(),
        "-k".into(),
        keys.to_string_lossy().to_string(),
        "--suppresskeys".into(),
        format!("--exefsdir={}", output.display()),
    ];
    if base.is_some() {
        opts.push(format!("--basenca={}", base_file.display()));
    }
    opts.push(update_nca.to_string_lossy().to_string());
    if safe_output("hactool", &opts).is_ok() {
        if let Ok(build) = main_nso_build(&main) {
            return Ok(build);
        }
    }
    nstool_status.push_str("_and_hactool_fallback_failed");
    Err(nstool_status)
}
fn run() -> Result<(), ErrBox> {
    let mut roms = PathBuf::from("/drive/roms/switch/roms");
    let mut specified_keys = None;
    let mut limit = 250usize;
    let mut max_nca_gib = DEFAULT_MAX_NCA_GIB;
    let mut scratch_dir: Option<PathBuf> = None;
    let mut inspect_file: Option<PathBuf> = None;
    let mut metadata_only = false;
    let mut selected = BTreeSet::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--roms" => roms = PathBuf::from(args.next().ok_or("--roms needs a directory")?),
            "--keyset" => {
                specified_keys = Some(PathBuf::from(args.next().ok_or("--keyset needs a path")?))
            }
            "--limit" => limit = args.next().ok_or("--limit needs a number")?.parse()?,
            "--max-nca-gib" => {
                max_nca_gib = args
                    .next()
                    .ok_or("--max-nca-gib needs an integer")?
                    .parse()?
            }
            "--scratch-dir" => {
                scratch_dir = Some(PathBuf::from(
                    args.next().ok_or("--scratch-dir needs a path")?,
                ))
            }
            "--title-id" => {
                let id = args
                    .next()
                    .ok_or("--title-id needs 16-hex ID")?
                    .to_ascii_uppercase();
                if hex_identifier(&id, 16).is_none() || !id.starts_with("01") {
                    return Err("invalid_title_id".into());
                }
                selected.insert(id);
            }
            "--inspect-file" => {
                inspect_file = Some(PathBuf::from(
                    args.next().ok_or("--inspect-file requires an NSP path")?,
                ))
            }
            "--metadata-only" => metadata_only = true,
            "--json" => {}
            "--help" | "-h" => {
                println!("Offline one-shot NSP Build ID inspector: --roms DIRECTORY --keyset PROD_KEYS --title-id ID (repeat) --limit COUNT --max-nca-gib 1..64 (default 32) --scratch-dir ABSOLUTE_PATH --inspect-file SINGLE_NSP --metadata-only --json. Requires nstool and hactool; XCI/NSZ/ XCZ are inventoried but not decrypted. Never writes to Eden or source game files.");
                return Ok(());
            }
            _ => return Err("unknown_option".into()),
        }
    }
    if limit == 0 || limit > MAX_NSP_COUNT {
        return Err("invalid_nsp_scan_limit".into());
    }
    if max_nca_gib == 0 || max_nca_gib > ABSOLUTE_MAX_NCA_GIB {
        return Err("max_nca_gib_must_be_1_to_64".into());
    }
    let max_nca_bytes = max_nca_gib * 1024 * 1024 * 1024;
    let keys = keyfile(specified_keys)?;
    let files = if let Some(path) = &inspect_file {
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nsp"))
        {
            return Err("--inspect-file requires an .nsp file".into());
        }
        let meta = fs::symlink_metadata(path)?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("--inspect-file requires a nonsymlink regular NSP file".into());
        }
        vec![path.to_owned()]
    } else {
        scan_paths(&roms, limit)?
    };
    let scratch_root = scratch_dir.unwrap_or(Scratch::default_root()?);
    let scratch = Scratch::new_at(&scratch_root)?;
    let mut packages = Vec::new();
    let mut warnings = Vec::new();
    let mut inspected_file_error: Option<String> = None;
    for (i, file) in files.iter().enumerate() {
        let temp = Scratch::new_at(&scratch_root)?;
        match discover_package(file, &keys, &temp) {
            Ok(Some(pkg)) if selected.is_empty() || selected.contains(&pkg.title) => {
                packages.push(pkg)
            }
            Ok(_) => {}
            Err(kind) => {
                if inspect_file.is_some() {
                    inspected_file_error = Some(kind.clone());
                }
                warnings.push(format!("package_{i}: {kind}"));
            }
        }
    }
    let mut grouped = BTreeMap::<String, Vec<Package>>::new();
    for package in packages {
        grouped
            .entry(package.title.clone())
            .or_default()
            .push(package);
    }
    let mut results = Vec::new();
    for (title, group) in grouped {
        let mut bases = group
            .iter()
            .filter(|p| p.kind == "application")
            .collect::<Vec<_>>();
        let mut patches = group
            .iter()
            .filter(|p| p.kind == "patch")
            .collect::<Vec<_>>();
        bases.sort_by(|a, b| b.version.cmp(&a.version));
        patches.sort_by(|a, b| b.version.cmp(&a.version));
        let selected_pkg = patches
            .first()
            .copied()
            .or_else(|| bases.first().copied())
            .ok_or("no_program_source")?;
        let base = if selected_pkg.kind == "patch" {
            bases.first().copied()
        } else {
            None
        };
        if patches.len() > 1
            && patches[0].version == patches[1].version
            && patches[0].program != patches[1].program
        {
            results.push(Found {
                title_id: title,
                source_kind: "patch".into(),
                version: selected_pkg.version,
                build_id: None,
                cheat_build_id: None,
                status: "ambiguous_same_version_updates".into(),
            });
            continue;
        }
        if selected_pkg.kind == "patch" && base.is_none() {
            results.push(Found {
                title_id: title,
                source_kind: "patch".into(),
                version: selected_pkg.version,
                build_id: None,
                cheat_build_id: None,
                status: "missing_base_program_nca_for_update".into(),
            });
            continue;
        }
        // Read-only against source and Eden: all extraction is into temporary files.
        if metadata_only {
            results.push(Found {
                title_id: title,
                source_kind: selected_pkg.kind.clone(),
                version: selected_pkg.version,
                build_id: None,
                cheat_build_id: None,
                status: "metadata_discovered_program_extraction_skipped".into(),
            });
            continue;
        }
        let res = extract_build(selected_pkg, base, &keys, &scratch, max_nca_bytes);
        results.push(Found {
            title_id: title,
            source_kind: selected_pkg.kind.clone(),
            version: selected_pkg.version,
            build_id: res.as_ref().ok().cloned(),
            cheat_build_id: res
                .as_ref()
                .ok()
                .and_then(|id| id.get(..16))
                .map(str::to_owned),
            status: res
                .err()
                .unwrap_or_else(|| "extracted_main_nso_build_id".into()),
        });
        for name in ["selected-program.nca", "base-program.nca"] {
            let _ = fs::remove_file(scratch.path(name));
        }
        let _ = fs::remove_dir_all(scratch.path("exefs"));
    }
    for title in selected {
        if !results.iter().any(|r| r.title_id == title) {
            results.push(Found {
                title_id: title,
                source_kind: "none".into(),
                version: 0,
                build_id: None,
                cheat_build_id: None,
                status: if inspect_file.is_some() {
                    inspected_file_error
                        .as_deref()
                        .map(discovery_blocked_status)
                        .unwrap_or("blocked_title_not_found_in_inspected_metadata")
                } else {
                    "not_found_in_scanned_nsp_content_metadata"
                }
                .into(),
            });
        }
    }
    results.sort_by(|a, b| a.title_id.cmp(&b.title_id));
    let source_file_counts_by_extension = discover_other_formats(&roms)?;
    let report = Report {
        format: "cheatarium-static-nsp-build-inspection-v1",
        read_only: true,
        source_packages_examined: files.len(),
        eden_files_written: 0,
        source_files_modified: 0,
        temporary_files_removed_on_exit: true,
        results,
        warnings,
        source_file_counts_by_extension,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("cheatarium-switch-nsp-builds: {e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_nested_cnmt_files_without_parsing_human_readable_tree() {
        let scratch = Scratch::new().unwrap();
        let root = scratch.path("extracted-metadata");
        fs::create_dir_all(root.join("0")).unwrap();
        let cnmt = root.join("0/Application_0100bc0018138000.cnmt");
        fs::write(&cnmt, b"synthetic CNMT fixture").unwrap();
        let files = bounded_cnmt_files(&root).unwrap();
        assert_eq!(files, vec![cnmt]);
    }
    #[cfg(unix)]
    #[test]
    fn refuses_symlink_in_extracted_metadata() {
        let scratch = Scratch::new().unwrap();
        let root = scratch.path("extracted-metadata");
        fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", root.join("untrusted.cnmt")).unwrap();
        assert_eq!(
            bounded_cnmt_files(&root),
            Err("extracted_metadata_symlink_refused".into())
        );
    }
    #[test]
    fn classifies_missing_content_key_as_blocked_not_absent_rom() {
        assert_eq!(
            discovery_blocked_status("metadata_unresolved_meta_ncas_1_ticket_count_0_unreadable_partitions_1_missing_keys_1"),
            "blocked_metadata_decryption_key_unavailable"
        );
        assert_eq!(discovery_blocked_status("metadata_unresolved_meta_ncas_1_ticket_count_0_unreadable_partitions_1_missing_keys_0"), "blocked_metadata_unresolved");
        assert_eq!(
            discovery_blocked_status("metadata_missing_cnmt_nca"),
            "blocked_missing_metadata_nca"
        );
        assert_eq!(
            discovery_blocked_status("invalid_nsp_pfs0"),
            "blocked_invalid_nsp"
        );
    }
    #[test]
    fn metadata_nca_commands_include_package_tickets() {
        let keys = PathBuf::from("/example/keys/prod.keys");
        let tickets = [PathBuf::from("/example/scratch/base-ticket.tik")];
        assert_eq!(
            nca_nstool_args(&keys, &tickets),
            vec![
                "-t",
                "nca",
                "-k",
                "/example/keys/prod.keys",
                "--tik",
                "/example/scratch/base-ticket.tik"
            ]
        );
    }
    #[test]
    fn diagnosis_redacts_key_values_and_reports_only_reason_counts() {
        let output = "[WARNING] NCA Partition 0 not readable. (AES-CTR Key was not determined)\nKey = abcdef";
        assert_eq!(metadata_partition_diagnosis(output), (1, 1));
    }
    #[test]
    fn identifies_program_from_nstool_cnmt() {
        let doc = "[ContentMeta]\n  TitleId:               0x01002da013484800\n  Version:               1.0.1 (v65536)\n  Type:                  Patch (129)\n  PatchMetaExtendedHeader:\n    ApplicationId:         0x01002da013484000\n  ContentInfo:\n    0\n      Type:         Program (1)\n      Id:           0123456789abcdef0123456789abcdef\n      Size:         0x123456\n  Digest: abc";
        assert_eq!(
            parse_content_metadata(doc),
            Some((
                "01002DA013484000".into(),
                65536,
                "patch".into(),
                "0123456789abcdef0123456789abcdef.nca".into()
            ))
        );
    }
    #[test]
    fn rejects_metadata_without_program() {
        let doc="[ContentMeta]\nTitleId: 0x01002da013484000\nVersion: 1.0.0 (v0)\nType: Application (128)\nContentInfo:\n 0\n Type: Control (3)\n Id: 0123456789abcdef0123456789abcdef\n";
        assert_eq!(parse_content_metadata(doc), None);
    }
    #[test]
    fn parses_minimal_pfs0_and_rejects_oob_bounds() {
        let scratch = Scratch::new().unwrap();
        let path = scratch.path("fixture.nsp");
        let name = b"0123456789abcdef0123456789abcdef.cnmt.nca\0";
        let mut raw = Vec::new();
        raw.extend(b"PFS0");
        raw.extend(1u32.to_le_bytes());
        raw.extend((name.len() as u32).to_le_bytes());
        raw.extend([0u8; 4]);
        raw.extend(0u64.to_le_bytes());
        raw.extend(4u64.to_le_bytes());
        raw.extend(0u32.to_le_bytes());
        raw.extend(0u32.to_le_bytes());
        raw.extend(name);
        raw.extend([1u8; 4]);
        fs::write(&path, &raw).unwrap();
        let found = scan_pfs0(&path).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].size, 4);
        raw.pop();
        fs::write(&path, &raw).unwrap();
        assert!(scan_pfs0(&path).is_err());
    }
    #[test]
    fn inventories_xci_and_nsz_without_claiming_build_evidence() {
        let scratch = Scratch::new().unwrap();
        fs::write(scratch.path("Super Mario RPG.xci"), b"not a parsed XCI").unwrap();
        fs::write(scratch.path("Other Game.nsz"), b"unparsed").unwrap();
        fs::write(scratch.path("update.nsp"), b"unparsed").unwrap();
        let formats = discover_other_formats(&scratch.0).unwrap();
        assert_eq!(formats.get("xci"), Some(&1));
        assert_eq!(formats.get("nsz"), Some(&1));
        assert_eq!(formats.get("nsp"), Some(&1));
    }
    #[test]
    fn program_extraction_respects_explicit_size_cap() {
        let scratch = Scratch::new().unwrap();
        let package = scratch.path("not_a_real_game.nsp");
        fs::write(&package, b"PFS0").unwrap();
        let oversized = NcaEntry {
            name: "large.nca".into(),
            size: 13 * 1024 * 1024 * 1024,
        };
        let dest = scratch.path("unwritten.nca");
        assert_eq!(
            extract_nca(&package, &oversized, &dest, 12 * 1024 * 1024 * 1024),
            Err("nca_exceeds_configured_safety_limit".into())
        );
        assert!(!dest.exists());
    }
    #[test]
    fn reads_nso_build_id_at_expected_offset() {
        let scratch = Scratch::new().unwrap();
        let path = scratch.path("main");
        let mut data = [0u8; 0x60];
        data[..4].copy_from_slice(b"NSO0");
        data[0x40..0x50].copy_from_slice(&[
            0x54, 0x41, 0xAF, 0x3C, 0x89, 0xF9, 0x7E, 0x1D, 0, 1, 2, 3, 4, 5, 6, 7,
        ]);
        fs::write(&path, data).unwrap();
        assert_eq!(
            main_nso_build(&path).unwrap(),
            "5441AF3C89F97E1D0001020304050607"
        );
    }
}
