//! Native Switch -> Eden per-cheat add-on planning and isolated staging.
//! Preview is read-only; explicit staging creates a new directory outside Eden.
//! Indexed source bytes are advisory, not installed-Build-ID evidence.
use cheatarium_client::{load_platform, verify_platform_distribution, Bundle, IndexedFile};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

type ErrBox = Box<dyn Error + Send + Sync>;
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

/// Explicitly staged output remains non-installed and is never linked into
/// Eden's mod tree. Source code must pass the same checks as its preview.
fn stage(
    bundle: &Bundle,
    selection: &Preview,
    stage_root: &Path,
    mods_root: &Path,
) -> Result<StagingReport, ErrBox> {
    safe_staging_destination(stage_root, mods_root)?;
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

fn run() -> Result<(), ErrBox> {
    let mut db = PathBuf::from("generated/v1");
    let mut mod_root = default_mod_root();
    let mut stage_dir = None;
    let mut title = None;
    let mut build = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--mods-root" => {
                mod_root = PathBuf::from(args.next().ok_or("--mods-root requires a path")?)
            }
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
                println!("Eden per-cheat addon preview (default read-only), optional isolated staging:\n  cheatarium-eden-plan --db generated/v1 --title-id 0100000000010000 --build-id B424BE150A8E7D78 [--mods-root /absolute/eden/load] [--stage-dir /absolute/new/staging-folder] [--json]\n--stage-dir creates only the named new folder outside Eden; never installs, activates, or verifies installed games.");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
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
    if let Some(stage_dir) = stage_dir {
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
