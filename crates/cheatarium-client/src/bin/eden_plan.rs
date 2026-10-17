//! Native Switch -> Eden per-cheat add-on planning; absolutely no filesystem writes.
//! Planned destinations and indexed source bytes are advisory, not installed-Build-ID evidence.
use cheatarium_client::{load_platform, verify_platform_distribution, Bundle, IndexedFile};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_PLANNED: usize = 2_000;

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
            if result.len() >= 40 { break; }
            result.push(c);
            separator = false;
        } else {
            separator = true;
        }
    }
    if result.is_empty() { "Unnamed".to_owned() } else { result }
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
        || payload.lines().any(|line| line.trim_start().starts_with('['))
    {
        return None;
    }
    Some(format!("[{name}]\n{payload}\n"))
}

fn ascii_opcode_lines(body: &str) -> bool {
    let mut count = 0;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let words = line.split_ascii_whitespace().collect::<Vec<_>>();
        if words.is_empty() || words.len() > 5
            || words.iter().any(|word| word.len() != 8 || !word.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return false;
        }
        count += 1;
    }
    count > 0
}

fn select<'a>(bundle: &'a Bundle, title: &str, build: &str) -> Vec<&'a IndexedFile> {
    let key = format!("title-id-{}-build-{}", title.to_ascii_lowercase(), build.to_ascii_lowercase());
    bundle.records.iter()
        .filter(|file| file.candidate_game_key == key && file.provenance.source_id == "sharkive")
        .collect()
}

fn preview(
    bundle: &Bundle,
    title: &str,
    build: &str,
    mod_root: &Path,
) -> Result<Preview, ErrBox> {
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

fn run() -> Result<(), ErrBox> {
    let mut db = PathBuf::from("generated/v1");
    let mut mod_root = default_mod_root();
    let mut title = None;
    let mut build = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--mods-root" => mod_root = PathBuf::from(args.next().ok_or("--mods-root requires a path")?),
            "--title-id" => title = Some(args.next().ok_or("--title-id requires a value")?.to_ascii_uppercase()),
            "--build-id" => build = Some(args.next().ok_or("--build-id requires a value")?.to_ascii_uppercase()),
            "--json" => {},
            "--help" | "-h" => {
                println!("Read-only Eden per-cheat addon preview:\n  cheatarium-eden-plan --db generated/v1 --title-id 0100000000010000 --build-id B424BE150A8E7D78 [--mods-root /absolute/eden/load] [--json]\nDoes not install, alter, activate, or verify installed games.");
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
    println!("{}", serde_json::to_string_pretty(&output)?);
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
        let out = preview(&db, "0100000000010000", "B424BE150A8E7D78", Path::new("/tmp/eden/load")).unwrap();
        assert!(out.read_only && !out.installation_allowed);
        assert_eq!(out.emulator_files_written, 0);
        assert!(out.exact_archived_build_candidate_found);
        assert_eq!(out.proposed_addons.len(), 2);
        assert_eq!(out.rejected_entries.len(), 1);
        assert!(out.proposed_addons[0].proposed_mod_path.contains("/0100000000010000/Cheatarium-"));
        assert!(out.proposed_addons[0].proposed_mod_path.ends_with("/cheats/B424BE150A8E7D78.txt"));
        assert!(!out.installed_build_id_verified);
    }

    #[test]
    fn mismatched_build_cannot_reuse_codes() {
        let db = fixture();
        let out = preview(&db, "0100000000010000", "AAAAAAAAAAAAAAAA", Path::new("/tmp/eden/load")).unwrap();
        assert!(!out.exact_archived_build_candidate_found);
        assert!(out.proposed_addons.is_empty());
    }

    #[test]
    fn filenames_are_bounded_and_path_safe() {
        assert_eq!(safe_label("../ab///cd"), "ab-cd");
        assert_eq!(safe_label("💖"), "Unnamed");
        assert!(render("Unsafe\nHeading", "00000000 00000000").is_none());
        assert!(render("Safe Heading", "[Injected]\nAAAA BBBB").is_none());
        assert!(!ascii_opcode_lines("not a code"));
        assert!(ascii_opcode_lines("800000C0\n01100000 5C3BE7DC 00000006\n20000000\n"));
    }
}
