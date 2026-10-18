//! Native Switch -> Eden per-cheat add-on planning and isolated staging.
//! Preview is read-only; explicit staging creates a new directory outside Eden.
//! Indexed source bytes are advisory, not installed-Build-ID evidence.
use cheatarium_client::{
    load_game_candidates, load_platform, verify_platform_distribution, Bundle, IndexedFile,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

type ErrBox = Box<dyn Error + Send + Sync>;

#[path = "eden_auto.rs"]
mod eden_auto;
#[path = "eden_builds.rs"]
#[allow(dead_code)]
mod eden_builds;
#[path = "eden_dry_run.rs"]
mod eden_dry_run;
#[path = "eden_install.rs"]
mod eden_install;
const MAX_PLANNED: usize = 2_000;
const MAX_STAGED_TOTAL_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Serialize)]
struct PlannedCheat {
    source_record_id: String,
    source_archive_path: String,
    source_ordinal: usize,
    description: String,
    proposed_mod_path: String,
    rendered_file_sha256: String,
    code_lines: usize,
    syntax_check: &'static str,
    source_enabled_ignored: bool,
    compatible_with_installed_build_verified: bool,
}

#[derive(Debug, Serialize)]
struct RejectedCheat {
    source_record_id: String,
    source_ordinal: usize,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct Preview {
    schema_version: u32,
    format: &'static str,
    read_only: bool,
    emulator_files_written: u32,
    source_game_files_read: u32,
    title_id: String,
    explicitly_selected_build_id: String,
    installed_build_id_verified: bool,
    source_filter: &'static str,
    proposed_mod_root: String,
    exact_archived_build_candidate_found: bool,
    proposed_addons: Vec<PlannedCheat>,
    rejected_entries: Vec<RejectedCheat>,
    installation_allowed: bool,
    existing_files_checked: bool,
    activation_state_written: bool,
    warnings: Vec<&'static str>,
}

fn is_title_id(s: &str) -> bool {
    s.len() == 16 && s.starts_with("01") && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_build_id(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Deliberately ASCII-only and bounded to avoid filesystem separators, dot
/// segments, control characters, unstable locale-specific normalization.
fn safe_label(s: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if separator && !result.is_empty() && result.len() < 40 {
                result.push('-');
            }
            if result.len() >= 40 {
                break;
            }
            result.push(c);
            separator = false;
        } else {
            separator = true;
        }
    }
    if result.is_empty() {
        "Unnamed".to_owned()
    } else {
        result
    }
}

fn default_config() -> PathBuf {
    if let Some(config) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config).join("eden/qt-config.ini");
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".config/eden/qt-config.ini")
}

fn default_mod_root() -> PathBuf {
    if let Some(data) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data).join("eden/load");
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".local/share/eden/load")
}

fn render(description: &str, body: &str) -> Option<String> {
    let name = description.trim();
    let payload = body.trim();
    if name.is_empty()
        || name.contains(['\r', '\n', '[', ']'])
        || name.chars().any(char::is_control)
        || payload.is_empty()
        || payload.contains('\0')
        || payload
            .lines()
            .any(|line| line.trim_start().starts_with('['))
    {
        return None;
    }
    Some(format!("[{name}]\n{payload}\n"))
}

fn ascii_opcode_lines(body: &str) -> bool {
    let mut count = 0;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let words = line.split_ascii_whitespace().collect::<Vec<_>>();
        if words.is_empty()
            || words.len() > 5
            || words
                .iter()
                .any(|word| word.len() != 8 || !word.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return false;
        }
        count += 1;
    }
    count > 0
}

fn select<'a>(bundle: &'a Bundle, title: &str, build: &str) -> Vec<&'a IndexedFile> {
    let key = format!(
        "title-id-{}-build-{}",
        title.to_ascii_lowercase(),
        build.to_ascii_lowercase()
    );
    bundle
        .records
        .iter()
        .filter(|file| file.candidate_game_key == key && file.provenance.source_id == "sharkive")
        .collect()
}

fn preview(bundle: &Bundle, title: &str, build: &str, mod_root: &Path) -> Result<Preview, ErrBox> {
    if !is_title_id(title) || !is_build_id(build) {
        return Err("Expected exact 16-hex Switch Title ID (01...) and Build ID".into());
    }
    if !mod_root.is_absolute() {
        return Err("Mods root must be an absolute path; no destination is written".into());
    }
    let files = select(bundle, title, build);
    let mut addons = Vec::new();
    let mut rejected = Vec::new();
    let exact_found = !files.is_empty();
    for file in files {
        for entry in &file.codes {
            if entry.role.as_deref() != Some("code") || !entry.is_code() {
                continue;
            }
            if addons.len() + rejected.len() >= MAX_PLANNED {
                return Err("Too many source cheat sections for a bounded preview".into());
            }
            let description = entry.description.as_deref().unwrap_or("");
            let body = entry.code.as_deref().unwrap_or("");
            let Some(text) = render(description, body) else {
                rejected.push(RejectedCheat {
                    source_record_id: file.id.clone(),
                    source_ordinal: entry.ordinal,
                    reason: "unsafe_heading_or_missing_body",
                });
                continue;
            };
            // Stable per-source identity; duplicate effect titles never overwrite
            // one another because they get distinct add-on directories.
            let ident = format!("{}:{}:{}", file.id, entry.ordinal, text);
            let digest = format!("{:x}", Sha256::digest(ident.as_bytes()));
            let filename = format!(
                "Cheatarium-{:04}-{}-{}",
                entry.ordinal,
                safe_label(description),
                &digest[..12]
            );
            let path = mod_root
                .join(title)
                .join(filename)
                .join("cheats")
                .join(format!("{build}.txt"));
            addons.push(PlannedCheat {
                source_record_id: file.id.clone(),
                source_archive_path: file.provenance.archive_path.clone(),
                source_ordinal: entry.ordinal,
                description: description.to_owned(),
                proposed_mod_path: path.to_string_lossy().into_owned(),
                rendered_file_sha256: format!("{:x}", Sha256::digest(text.as_bytes())),
                code_lines: body.lines().filter(|line| !line.trim().is_empty()).count(),
                syntax_check: if ascii_opcode_lines(body) {
                    "lexical_hex_tokens_only_not_execution_validated"
                } else {
                    "unrecognized_source_syntax_manual_review_required"
                },
                source_enabled_ignored: true,
                compatible_with_installed_build_verified: false,
            });
        }
    }
    addons.sort_by(|a, b| a.proposed_mod_path.cmp(&b.proposed_mod_path));
    Ok(Preview {
        schema_version: 1,
        format: "cheatarium-eden-addon-preview-v1",
        read_only: true,
        emulator_files_written: 0,
        source_game_files_read: 0,
        title_id: title.to_owned(),
        explicitly_selected_build_id: build.to_owned(),
        installed_build_id_verified: false,
        source_filter: "sharkive_native_switch_archive",
        proposed_mod_root: mod_root.to_string_lossy().into_owned(),
        exact_archived_build_candidate_found: exact_found,
        proposed_addons: addons,
        rejected_entries: rejected,
        installation_allowed: false,
        existing_files_checked: false,
        activation_state_written: false,
        warnings: vec![
            "The selected Build ID comes from cheat archive metadata, not the currently installed executable.",
            "Syntax checks only examine source hex-token shape and do not establish emulator execution compatibility.",
            "No existing paths or activation flags are inspected or modified.",
            "Eden may enable newly discovered add-ons by default: a writer must atomically ensure disable-by-default first.",
            "No transfer or installation is implemented in this command.",
        ],
    })
}

#[derive(Debug, Serialize)]
struct StagedFile {
    source_record_id: String,
    source_ordinal: usize,
    relative_path: String,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct StagingReport {
    schema_version: u32,
    format: &'static str,
    stage_root: String,
    title_id: String,
    explicitly_selected_build_id: String,
    staged_count: usize,
    staged_files: Vec<StagedFile>,
    staged_manifest: &'static str,
    staging_directory_created: bool,
    emulator_files_written: u32,
    activation_state_written: bool,
    installed_build_id_verified: bool,
    installation_allowed: bool,
    all_output_is_outside_eden: bool,
    warning: &'static str,
}

/// Fail closed if any existing ancestor uses a symlink: an explicit staging
/// request must never traverse a redirect into the emulator's live directory.
fn safe_staging_destination(path: &Path, mods_root: &Path) -> Result<(), ErrBox> {
    if !path.is_absolute() || path.parent().is_none() {
        return Err("Staging destination must be an absolute, new, non-root directory".into());
    }
    if path
        .components()
        .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("eden"))
    {
        return Err("Refusing to stage inside an Eden-named directory".into());
    }
    let mut parent = path.parent().ok_or("Missing staging parent")?;
    if parent == Path::new("/") {
        return Err("Refusing to create a new directory directly at filesystem root".into());
    }
    // Only accept an already existing direct parent; avoids implicit mkdir -p
    // crossing unsupported links or unexpectedly creating a hierarchy.
    while !parent.as_os_str().is_empty() {
        let meta = fs::symlink_metadata(parent)?;
        if meta.file_type().is_symlink() {
            return Err("Staging parent path contains a symbolic link".into());
        }
        if !meta.is_dir() {
            return Err("Staging parent path is not a directory".into());
        }
        let Some(next) = parent.parent() else { break };
        parent = next;
    }
    if fs::symlink_metadata(path).is_ok() {
        return Err("Staging destination already exists: refusing to reuse or overwrite".into());
    }
    let canonical_parent = path
        .parent()
        .ok_or("Missing staging parent")?
        .canonicalize()?;
    let canonical_candidate =
        canonical_parent.join(path.file_name().ok_or("Missing staging name")?);
    for banned in [
        mods_root.to_path_buf(),
        default_mod_root(),
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|x| PathBuf::from(x).join(".config")))
            .unwrap_or_default()
            .join("eden"),
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|x| PathBuf::from(x).join(".cache")))
            .unwrap_or_default()
            .join("eden"),
    ] {
        if banned.is_absolute() && canonical_candidate.starts_with(&banned) {
            return Err(
                "Refusing staging within Eden's live data/config/cache/mod directory".into(),
            );
        }
    }
    Ok(())
}

fn write_new(path: &Path, contents: &[u8]) -> Result<(), ErrBox> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(())
}

/// Shared preflight and byte-for-byte rendering for staging and installation.
/// No filesystem writes. The exact archived title/build source must match.
fn prepare_addons(
    bundle: &Bundle,
    selection: &Preview,
    mods_root: &Path,
) -> Result<Vec<(StagedFile, Vec<u8>)>, ErrBox> {
    if selection.proposed_addons.is_empty() || !selection.exact_archived_build_candidate_found {
        return Err("Nothing to stage: no exact archived Title ID + Build ID match".into());
    }
    if !selection.rejected_entries.is_empty() {
        return Err("Staging blocked: original source has rejected sections".into());
    }
    if selection
        .proposed_addons
        .iter()
        .any(|a| a.syntax_check != "lexical_hex_tokens_only_not_execution_validated")
    {
        return Err("Staging blocked: unknown native source syntax requires review".into());
    }

    let title = selection.title_id.as_str();
    let build = selection.explicitly_selected_build_id.as_str();
    let sources = select(bundle, title, build);
    let mut prepared = Vec::<(StagedFile, Vec<u8>)>::new();
    let mut planned_paths = BTreeSet::new();
    let mut total_bytes = 0usize;
    for source in sources {
        for entry in &source.codes {
            if entry.role.as_deref() != Some("code") || !entry.is_code() {
                continue;
            }
            let Some(text) = render(
                entry.description.as_deref().unwrap_or(""),
                entry.code.as_deref().unwrap_or(""),
            ) else {
                return Err("Staging blocked: unrenderable native cheat".into());
            };
            if !ascii_opcode_lines(entry.code.as_deref().unwrap_or("")) {
                return Err("Staging blocked: unsupported native cheat opcode shape".into());
            }
            let code = selection
                .proposed_addons
                .iter()
                .find(|item| {
                    item.source_record_id == source.id && item.source_ordinal == entry.ordinal
                })
                .ok_or("Staging mismatch with preview source identity")?;
            let relative = Path::new(&code.proposed_mod_path)
                .strip_prefix(mods_root)
                .map_err(|_| "Planned target does not belong to selected mods root")?;
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                || relative.components().count() != 4
            {
                return Err("Invalid planned relative cheat path".into());
            }
            let rel_text = relative.to_string_lossy().replace('\\', "/");
            if !planned_paths.insert(rel_text.clone()) {
                return Err("Duplicate staged cheat path".into());
            }
            if format!("{:x}", Sha256::digest(text.as_bytes())) != code.rendered_file_sha256 {
                return Err("Staging checksum does not match preview".into());
            }
            total_bytes = total_bytes
                .checked_add(text.len())
                .ok_or("Staging size overflow")?;
            if total_bytes > MAX_STAGED_TOTAL_BYTES {
                return Err("Staging size limit exceeded".into());
            }
            prepared.push((
                StagedFile {
                    source_record_id: source.id.clone(),
                    source_ordinal: entry.ordinal,
                    relative_path: rel_text,
                    sha256: code.rendered_file_sha256.clone(),
                },
                text.into_bytes(),
            ));
        }
    }
    if prepared.len() != selection.proposed_addons.len() {
        return Err("Staging count mismatches the preview".into());
    }
    Ok(prepared)
}

/// Explicitly staged output remains non-installed and is never linked into
/// Eden's mod tree. Source code must pass the same checks as its preview.
fn stage(
    bundle: &Bundle,
    selection: &Preview,
    stage_root: &Path,
    mods_root: &Path,
) -> Result<StagingReport, ErrBox> {
    safe_staging_destination(stage_root, mods_root)?;
    let title = selection.title_id.as_str();
    let build = selection.explicitly_selected_build_id.as_str();
    let prepared = prepare_addons(bundle, selection, mods_root)?;
    // All checks above happen BEFORE the first filesystem mutation.
    fs::create_dir(stage_root)?; // no overwrite, no reuse
    let title_dir = stage_root.join(title);
    fs::create_dir(&title_dir)?;
    for (entry, bytes) in &prepared {
        let rel = Path::new(&entry.relative_path);
        let mod_dir = title_dir.join(rel.components().nth(1).ok_or("No addon name")?.as_os_str());
        fs::create_dir(&mod_dir)?;
        let cheats_dir = mod_dir.join("cheats");
        fs::create_dir(&cheats_dir)?;
        let target = stage_root.join(rel);
        write_new(&target, bytes)?;
    }
    let report = StagingReport {
        schema_version: 1,
        format: "cheatarium-eden-isolated-staging-v1",
        stage_root: stage_root.to_string_lossy().into_owned(),
        title_id: title.to_owned(),
        explicitly_selected_build_id: build.to_owned(),
        staged_count: prepared.len(),
        staged_files: prepared.into_iter().map(|(file, _)| file).collect(),
        staged_manifest: "cheatarium-staging-manifest.json",
        staging_directory_created: true,
        emulator_files_written: 0,
        activation_state_written: false,
        installed_build_id_verified: false,
        installation_allowed: false,
        all_output_is_outside_eden: true,
        warning: "This is an isolated, non-installed export. Do not copy these cheats into Eden before exact installed Build ID verification and explicit disabled-by-default handling.",
    };
    let manifest = serde_json::to_vec_pretty(&report)?;
    write_new(&stage_root.join(report.staged_manifest), &manifest)?;
    Ok(report)
}

#[derive(Debug, Serialize)]
struct SkippedAutomaticBuild {
    title_id: String,
    build_id: Option<String>,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct AutomaticReport {
    schema_version: u32,
    format: &'static str,
    mode: &'static str,
    discovery: eden_auto::Discovery,
    archived_title_build_pairs: usize,
    candidate_build_scope: &'static str,
    installed_title_build_pairs: usize,
    already_installed_title_build_pairs: usize,
    prepared_title_build_pairs: usize,
    installed: Vec<eden_install::Installed>,
    planned: Vec<Preview>,
    skipped: Vec<SkippedAutomaticBuild>,
    game_executable_build_verified: bool,
    warnings: Vec<&'static str>,
}

/// An archived Build ID is a filename discriminator, not evidence of the
/// executable version Eden currently runs. Installing all available variants
/// disabled allows Eden to select the matching filename later without asking
/// the user to discover or type arbitrary IDs.
fn automatic_pairs(bundle: &Bundle, ids: &[String]) -> Result<Vec<(String, String)>, ErrBox> {
    let known = ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut pairs = BTreeSet::new();
    for record in &bundle.records {
        if record.provenance.source_id != "sharkive" {
            continue;
        }
        let Some(rest) = record.candidate_game_key.strip_prefix("title-id-") else {
            continue;
        };
        let Some((title, build)) = rest.split_once("-build-") else {
            continue;
        };
        let title = title.to_ascii_uppercase();
        let build = build.to_ascii_uppercase();
        if known.contains(title.as_str()) && is_title_id(&title) && is_build_id(&build) {
            pairs.insert((title, build));
        }
    }
    if pairs.len() > 512 {
        return Err("Automatic installation has more than 512 archived title/build variants; refusing unbounded work".into());
    }
    Ok(pairs.into_iter().collect())
}

/// Limit the preview to games with exactly one main executable Build ID
/// observed in the supplied Eden launch log. This is historical evidence
/// only, never a verification of the current installed game executable.
/// Ambiguity fails closed; do not pick the newest or first observation.
fn single_historical_main_pairs(
    pairs: Vec<(String, String)>,
    observations: &[eden_builds::MainBuildObservation],
) -> (Vec<(String, String)>, Vec<SkippedAutomaticBuild>) {
    let mut by_title = BTreeMap::<String, BTreeSet<String>>::new();
    for observed in observations {
        by_title
            .entry(observed.title_id.clone())
            .or_default()
            .insert(observed.build_id.clone());
    }
    let mut selected = Vec::new();
    let mut skipped = Vec::new();
    for (title, build) in pairs {
        let reason = match by_title.get(&title) {
            None => Some("no_historical_main_build_observed"),
            Some(builds) if builds.len() != 1 => {
                Some("multiple_historical_main_builds_ambiguous")
            }
            Some(builds) if !builds.contains(&build) => {
                Some("archived_build_not_historically_observed_as_main")
            }
            _ => None,
        };
        if let Some(reason) = reason {
            skipped.push(SkippedAutomaticBuild {
                title_id: title,
                build_id: Some(build),
                reason,
            });
        } else {
            selected.push((title, build));
        }
    }
    (selected, skipped)
}

fn run_automatic(
    bundle: &Bundle,
    config: &Path,
    cache_dir: &Path,
    mod_root: &Path,
    installing: bool,
    historical_main_only: Option<&eden_builds::LaunchEvidence>,
) -> Result<AutomaticReport, ErrBox> {
    let discovery = eden_auto::discover(config, cache_dir)?;
    if discovery.titles.is_empty() {
        return Err(
            "Eden registered directories and cache contain no identifiable base games".into(),
        );
    }
    let pairs = automatic_pairs(bundle, &discovery.titles)?;
    let known_with_archive = pairs
        .iter()
        .map(|(t, _)| t.as_str())
        .collect::<BTreeSet<_>>();
    let mut skipped = Vec::new();
    for title in &discovery.titles {
        if !known_with_archive.contains(title.as_str()) {
            skipped.push(SkippedAutomaticBuild {
                title_id: title.clone(),
                build_id: None,
                reason: "no_archived_sharkive_cheats_for_discovered_game",
            });
        }
    }
    let total_pairs = pairs.len();
    let (pairs, history_skipped) = match historical_main_only {
        Some(evidence) => single_historical_main_pairs(pairs, &evidence.main),
        None => (pairs, Vec::new()),
    };
    skipped.extend(history_skipped);
    let mut planned = Vec::new();
    let mut installed = Vec::new();
    let mut already_installed = 0usize;
    for (title, build) in pairs {
        let candidate = preview(bundle, &title, &build, mod_root)?;
        if candidate.proposed_addons.is_empty() {
            skipped.push(SkippedAutomaticBuild {
                title_id: title,
                build_id: Some(build),
                reason: "no_renderable_cheat_sections",
            });
            continue;
        }
        if !candidate.rejected_entries.is_empty()
            || candidate
                .proposed_addons
                .iter()
                .any(|x| x.syntax_check != "lexical_hex_tokens_only_not_execution_validated")
        {
            skipped.push(SkippedAutomaticBuild {
                title_id: title,
                build_id: Some(build),
                reason: "unvalidated_or_unsafe_source_cheat",
            });
            continue;
        }
        if installing {
            // Do not rewrite identical disabled Cheatarium add-ons; a changed
            // or unrecognized pre-existing mod is still protected by install().
            if eden_install::already_installed_unchanged(bundle, &candidate, mod_root, config)? {
                already_installed += 1;
                continue;
            }
            let result = eden_install::install(bundle, &candidate, mod_root, config)?;
            installed.push(result);
        } else {
            planned.push(candidate);
        }
    }
    Ok(AutomaticReport {
        schema_version: 1,
        format: "cheatarium-eden-auto-cheat-synthesis-v1",
        mode: if installing { "direct_install_disabled" } else { "read_only_preview" },
        discovery, archived_title_build_pairs: total_pairs,
        candidate_build_scope: if historical_main_only.is_some() {
            "unique_historical_main_nso_build_only_not_current_verified"
        } else {
            "all_archived_builds_unverified"
        },
        installed_title_build_pairs: installed.len(),
        already_installed_title_build_pairs: already_installed,
        prepared_title_build_pairs: planned.len(),
        installed, planned, skipped,
        game_executable_build_verified: false,
        warnings: vec![
            "Title IDs discovered from Eden game directories and cache are advisory, not proof of installation.",
            "Archived Build IDs do not establish installed executable versions. The historical-main-only mode narrows candidates using launch logs; it never claims current-version verification.",
            "No runtime Build ID was guessed or required from the user.",
            "Cheat effects and compatibility are unverified. Enable only desired cheats in Eden.",
        ]
    })
}

fn run() -> Result<(), ErrBox> {
    let mut db = PathBuf::from("generated/v1");
    let mut mod_root = default_mod_root();
    let mut stage_dir = None;
    let mut config = default_config();
    let mut cache_dir = eden_auto::default_cache_dir();
    let mut log_path = eden_builds::log_default();
    let mut auto = false;
    let mut dry_run = false;
    let mut human_report = false;
    let mut historical_main_only = false;
    let mut install = false;
    let mut title = None;
    let mut build = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--mods-root" => {
                mod_root = PathBuf::from(args.next().ok_or("--mods-root requires a path")?)
            }
            "--config" => config = PathBuf::from(args.next().ok_or("--config requires a path")?),
            "--install" => install = true,
            "--auto" => auto = true,
            "--dry-run" => {
                dry_run = true;
                auto = true;
            }
            "--report" => human_report = true,
            "--historical-main-only" => historical_main_only = true,
            "--cache-dir" => {
                cache_dir = PathBuf::from(args.next().ok_or("--cache-dir requires a path")?)
            }
            "--log" => log_path = PathBuf::from(args.next().ok_or("--log requires a path")?),
            "--stage-dir" => {
                stage_dir = Some(PathBuf::from(
                    args.next().ok_or("--stage-dir requires an absolute path")?,
                ))
            }
            "--title-id" => {
                title = Some(
                    args.next()
                        .ok_or("--title-id requires a value")?
                        .to_ascii_uppercase(),
                )
            }
            "--build-id" => {
                build = Some(
                    args.next()
                        .ok_or("--build-id requires a value")?
                        .to_ascii_uppercase(),
                )
            }
            "--json" => {}
            "--help" | "-h" => {
                println!("Review every game and cheat without writing:\n  cheatarium-eden-plan --db generated/v1 --dry-run --report\nConservative historical main-only preview: --dry-run --historical-main-only --report\nAfter review, the identical selection policy is available with --install --historical-main-only.\nHistorical evidence is NOT proof of the currently installed executable.\nDetailed JSON: --dry-run --json\nOnly after review, run --install to create disabled cheat add-ons. No IDs required. Options: --config, --cache-dir, --mods-root, --log for portable Eden setups.");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    if dry_run && (install || stage_dir.is_some()) {
        return Err("--dry-run refuses --install and --stage-dir; it never writes".into());
    }
    if human_report && !dry_run {
        return Err("--report requires --dry-run".into());
    }
    if historical_main_only && !dry_run && !install {
        return Err("--historical-main-only requires --dry-run or --install".into());
    }
    if install && stage_dir.is_some() {
        return Err("Choose --install or --stage-dir, not both".into());
    }
    let automatic = auto || (install && title.is_none() && build.is_none());
    if automatic && (title.is_some() || build.is_some() || stage_dir.is_some()) {
        return Err("Automatic discovery cannot be combined with manual IDs or staging".into());
    }
    if !automatic && (title.is_none() || build.is_none()) {
        return Err(
            "Provide both --title-id and --build-id, or use --install for automatic Eden discovery"
                .into(),
        );
    }
    if automatic {
        verify_platform_distribution(&db, "switch")?;
        let bundle = load_platform(&db, "switch")?;
        let evidence = if dry_run || historical_main_only {
            let index = load_game_candidates(&db, "switch")?;
            Some(eden_builds::read_launch_evidence(&log_path, &index)?)
        } else {
            None
        };
        let report = run_automatic(
            &bundle,
            &config,
            &cache_dir,
            &mod_root,
            install,
            if historical_main_only {
                evidence.as_ref()
            } else {
                None
            },
        )?;
        if dry_run {
            let evidence = evidence.as_ref().ok_or("Missing read-only Eden evidence")?;
            let detailed =
                eden_dry_run::analyze(&report, &bundle, &config, &mod_root, &log_path, evidence)?;
            if human_report {
                print!("{}", eden_dry_run::render_human(&detailed));
            } else {
                println!("{}", serde_json::to_string_pretty(&detailed)?);
            }
        } else {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        return Ok(());
    }
    let title = title.ok_or("Missing --title-id")?;
    let build = build.ok_or("Missing --build-id")?;
    // Reject malformed arguments before loading a large database.
    if !is_title_id(&title) || !is_build_id(&build) {
        return Err("Expected exact 16-hex Switch Title ID and Build ID".into());
    }
    verify_platform_distribution(&db, "switch")?;
    let bundle = load_platform(&db, "switch")?;
    let output = preview(&bundle, &title, &build, &mod_root)?;
    if install {
        let installed = eden_install::install(&bundle, &output, &mod_root, &config)?;
        println!("{}", serde_json::to_string_pretty(&installed)?);
    } else if let Some(stage_dir) = stage_dir {
        let staged = stage(&bundle, &output, &stage_dir, &mod_root)?;
        println!("{}", serde_json::to_string_pretty(&staged)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&output)?);
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cheatarium-eden-plan: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_main_only_never_selects_ambiguous_or_unobserved_builds() {
        let pairs = vec![
            ("0100000000010000".into(), "3CA12DFAAF9C82DA".into()),
            ("0100000000010000".into(), "B424BE150A8E7D78".into()),
            ("0100152000022000".into(), "1CD9216A21737FB4".into()),
            ("0100152000022000".into(), "96DFD1769E560C27".into()),
            ("01006A800016E000".into(), "2EF410F4D4D2D279".into()),
        ];
        let observed = |title_id: &str, build_id: &str| eden_builds::MainBuildObservation {
            title_id: title_id.into(),
            build_id: build_id.into(),
            archived_candidate: true,
            launch_context_ordinal: 1,
        };
        let main = vec![
            observed("0100000000010000", "B424BE150A8E7D78"),
            observed("0100000000010000", "B424BE150A8E7D78"),
            observed("0100152000022000", "1CD9216A21737FB4"),
            observed("0100152000022000", "96DFD1769E560C27"),
        ];
        let (selected, skipped) = single_historical_main_pairs(pairs, &main);
        assert_eq!(selected, vec![("0100000000010000".into(), "B424BE150A8E7D78".into())]);
        assert_eq!(skipped.len(), 4);
        assert_eq!(skipped.iter().filter(|s| s.reason == "multiple_historical_main_builds_ambiguous").count(), 2);
        assert!(skipped.iter().any(|s| s.reason == "no_historical_main_build_observed"));
        let (none, all_skipped) = single_historical_main_pairs(
            vec![("0100000000010000".into(), "3CA12DFAAF9C82DA".into())],
            &[],
        );
        assert!(none.is_empty());
        assert_eq!(all_skipped[0].reason, "no_historical_main_build_observed");
    }

    fn fixture() -> Bundle {
        serde_json::from_value(serde_json::json!({
            "schema_version": 1,
            "platform": "switch",
            "game_identity_rule": "advisory",
            "compatibility_rule": "unverified",
            "records": [{
                "id": "sharkive:0100000000010000/b424be150a8e7d78.txt",
                "title_hint": "Odyssey",
                "candidate_game_key": "title-id-0100000000010000-build-b424be150a8e7d78",
                "identity_confidence": "unverified",
                "raw_filename": "b424be150a8e7d78.txt",
                "region_hint": null,
                "format_hint": "native",
                "declared_cheats": null,
                "parse_warnings": [],
                "codes": [
                    {"ordinal": 0, "description": "Infinite Health (L+R)", "code": "800000C0\n01100000 5C3BE7DC 00000006\n20000000\n", "source_enabled": false, "verification": "unverified", "role": "code"},
                    {"ordinal": 1, "description": "../../unsafe", "code": "01234567 ABCDEF12\n", "source_enabled": false, "verification": "unverified", "role": "code"},
                    {"ordinal": 2, "description": "Header [unsafe]", "code": "01234567 ABCDEF12\n", "source_enabled": false, "verification": "unverified", "role": "code"}
                ],
                "provenance": {
                    "source_id": "sharkive", "repository": "https://github.com/example",
                    "revision": "abc", "license": "unknown",
                    "upstream_path": "switch/0100000000010000/b424be150a8e7d78.txt",
                    "archive_path": "archive/sharkive/switch/0100000000010000/b424be150a8e7d78.txt",
                    "git_blob_sha": "123"
                }
            }]
        })).unwrap()
    }

    #[test]
    fn exact_build_only_and_individual_paths() {
        let db = fixture();
        let out = preview(
            &db,
            "0100000000010000",
            "B424BE150A8E7D78",
            Path::new("/tmp/eden/load"),
        )
        .unwrap();
        assert!(out.read_only && !out.installation_allowed);
        assert_eq!(out.emulator_files_written, 0);
        assert!(out.exact_archived_build_candidate_found);
        assert_eq!(out.proposed_addons.len(), 2);
        assert_eq!(out.rejected_entries.len(), 1);
        assert!(out.proposed_addons[0]
            .proposed_mod_path
            .contains("/0100000000010000/Cheatarium-"));
        assert!(out.proposed_addons[0]
            .proposed_mod_path
            .ends_with("/cheats/B424BE150A8E7D78.txt"));
        assert!(!out.installed_build_id_verified);
    }

    #[test]
    fn mismatched_build_cannot_reuse_codes() {
        let db = fixture();
        let out = preview(
            &db,
            "0100000000010000",
            "AAAAAAAAAAAAAAAA",
            Path::new("/tmp/eden/load"),
        )
        .unwrap();
        assert!(!out.exact_archived_build_candidate_found);
        assert!(out.proposed_addons.is_empty());
    }

    #[test]
    fn stages_once_only_outside_eden_and_never_overwrites() {
        let db = fixture();
        let root = env::temp_dir().join(format!(
            "cheatarium-stage-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let selection = preview(
            &db,
            "0100000000010000",
            "B424BE150A8E7D78",
            Path::new("/tmp/eden/load"),
        )
        .unwrap();
        // Fixture has an unsafe heading: all-or-nothing stage refuses without
        // even creating a destination folder.
        assert!(stage(&db, &selection, &root, Path::new("/tmp/eden/load")).is_err());
        assert!(!root.exists());
        let mut clean = db;
        clean.records[0].codes.retain(|code| code.ordinal == 0);
        let selection = preview(
            &clean,
            "0100000000010000",
            "B424BE150A8E7D78",
            Path::new("/tmp/eden/load"),
        )
        .unwrap();
        let staged = stage(&clean, &selection, &root, Path::new("/tmp/eden/load")).unwrap();
        assert_eq!(staged.staged_count, 1);
        assert_eq!(staged.emulator_files_written, 0);
        assert!(!staged.installation_allowed);
        let output = root.join(&staged.staged_files[0].relative_path);
        assert!(output.is_file());
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(output).unwrap())),
            staged.staged_files[0].sha256
        );
        assert!(root.join("cheatarium-staging-manifest.json").is_file());
        assert!(stage(&clean, &selection, &root, Path::new("/tmp/eden/load")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stage_rejects_live_eden_tree_and_nonexistent_parent() {
        assert!(safe_staging_destination(
            Path::new("/tmp/eden/load/cheatarium-stage"),
            Path::new("/tmp/eden/load")
        )
        .is_err());
        assert!(safe_staging_destination(
            Path::new("/tmp/absent-parent-f73ab928/stage"),
            Path::new("/tmp/eden/load")
        )
        .is_err());
    }

    #[test]
    fn filenames_are_bounded_and_path_safe() {
        assert_eq!(safe_label("../ab///cd"), "ab-cd");
        assert_eq!(safe_label("💖"), "Unnamed");
        assert!(render("Unsafe\nHeading", "00000000 00000000").is_none());
        assert!(render("Safe Heading", "[Injected]\nAAAA BBBB").is_none());
        assert!(!ascii_opcode_lines("not a code"));
        assert!(ascii_opcode_lines(
            "800000C0\n01100000 5C3BE7DC 00000006\n20000000\n"
        ));
    }
}
