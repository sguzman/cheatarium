//! Conservative, read-only association of Eden launch logs with main NSO Build IDs.
//! A log-derived build is historical evidence, not proof of the current game installation.
use cheatarium_client::{load_game_candidates, verify_platform_distribution, GameIndex};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_LOG_BYTES: u64 = 32 * 1024 * 1024;
const MAX_CONTEXT_LINES: usize = 20_000;
const MAX_OBSERVATIONS: usize = 2_000;

#[derive(Debug, Clone, Serialize)]
struct ObservedBuild {
    title_id: String,
    main_build_id: String,
    original_build_hex_length: usize,
    launch_context_ordinal: usize,
    evidence: &'static str,
    archived_build_candidate_found: bool,
    current_installed_build_verified: bool,
    compatible_cheats_verified: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u32,
    format: &'static str,
    read_only: bool,
    emulator_files_written: u32,
    game_file_bytes_read: u32,
    log_path: String,
    log_found: bool,
    log_bytes_read: usize,
    historical_main_build_observations: Vec<ObservedBuild>,
    observed_title_ids: usize,
    multiple_builds_for_same_title: Vec<String>,
    main_builds_without_launch_context: usize,
    core_launch_contexts_seen: usize,
    main_nso_patch_queries_seen: usize,
    direct_cheat_lookup_observations: Vec<CheatLookupObservation>,
    log_has_main_executable_build_evidence: bool,
    installation_allowed: bool,
    warnings: Vec<&'static str>,
}

/// Eden emits this paired identifier while looking for an installed cheat
/// file. Such a record does NOT prove it was the main executable's Build ID.
#[derive(Debug, Serialize)]
struct CheatLookupObservation {
    title_id: String,
    build_id: String,
    event: &'static str,
    archived_build_candidate_found: bool,
    main_executable_build_verified: bool,
    current_installed_build_verified: bool,
}

#[derive(Debug, Clone)]
struct BuildWitness {
    title_id: String,
    build_id: String,
    original_length: usize,
    launch_ordinal: usize,
}

#[derive(Default)]
struct ParsedLog {
    witness: Vec<BuildWitness>,
    orphan_main_builds: usize,
    core_launches: usize,
    main_queries: usize,
    cheat_lookup_witness: BTreeSet<(String, String, &'static str)>,
}

fn hex_token(s: &str, n: usize) -> Option<String> {
    let token = s.get(..n)?;
    if token.len() != n || !token.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    if s.as_bytes().get(n).is_some_and(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(token.to_ascii_uppercase())
}

fn title_after(line: &str, marker: &str) -> Option<String> {
    let suffix = line.split_once(marker)?.1;
    let candidate = hex_token(suffix, 16)?;
    candidate.starts_with("01").then_some(candidate)
}

fn loading_title(line: &str) -> Option<String> {
    // Eden core.cpp writes "Loading <game name> (<TITLE_ID>) ...".
    // Don't interpret unrelated log messages containing "Loading".
    if !line.contains("Core <Info>") && !line.contains("core/core.cpp") {
        return None;
    }
    let (_, suffix) = line.split_once(": Loading ")?;
    let index = suffix.rfind('(')?;
    let candidate = suffix.get(index + 1..)?;
    let title = hex_token(candidate, 16)?;
    if !candidate.get(16..)?.starts_with(')') || !title.starts_with("01") {
        return None;
    }
    Some(title)
}

fn main_nso_build(line: &str) -> Option<(String, usize)> {
    let suffix = line
        .split_once("Querying NSO patch existence for build_id=")?
        .1;
    let (raw, name) = suffix.split_once(", name=")?;
    if name.trim() != "main"
        || raw.len() < 16
        || raw.len() > 64
        || raw.len() % 2 != 0
        || !raw.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    Some((raw[..16].to_ascii_uppercase(), raw.len()))
}

/// Eden patch_manager.cpp's ReadCheatFileFromFolder may emit one of these
/// title+build pairs if an enabled mod folder tries to find a cheat file.
/// This is a separate, weaker signal than a logged main NSO query.
fn direct_cheat_lookup(line: &str) -> Option<(String, String, &'static str)> {
    const PATTERNS: [(&str, &str); 2] = [
        (
            "No cheats file found for title_id=",
            "missing_cheat_file_lookup",
        ),
        (
            "Failed to read cheats file for title_id=",
            "unreadable_cheat_file_lookup",
        ),
    ];
    for (marker, kind) in PATTERNS {
        let Some((_, suffix)) = line.split_once(marker) else {
            continue;
        };
        let Some(title) = hex_token(suffix, 16) else {
            continue;
        };
        if !title.starts_with("01") {
            continue;
        }
        let Some(rest) = suffix.get(16..).and_then(|x| x.strip_prefix(", build_id=")) else {
            continue;
        };
        let Some(build) = hex_token(rest, 16) else {
            continue;
        };
        if !rest
            .get(16..)
            .is_some_and(|x| x.is_empty() || x.starts_with(|c: char| c.is_whitespace()))
        {
            continue;
        }
        return Some((title, build, kind));
    }
    None
}

fn parse_log(log: &str) -> Result<ParsedLog, ErrBox> {
    let mut out = ParsedLog::default();
    let mut active: Option<(String, usize, usize)> = None; // title, ordinal, last launch line
    let mut launch_ordinal = 0usize;
    let mut seen = BTreeSet::new();
    for (line_index, line) in log.lines().enumerate() {
        let newly_loading = loading_title(line);
        let is_new_launch = newly_loading.is_some();
        if is_new_launch {
            out.core_launches += 1;
        }
        if let Some(witness) = direct_cheat_lookup(line) {
            out.cheat_lookup_witness.insert(witness);
        }
        let announced =
            title_after(line, "Booting game: ").or_else(|| title_after(line, "Now running: "));
        if let Some(id) = newly_loading.or(announced) {
            // Core Loading marks an explicit new launch even for the same title.
            if is_new_launch || active.as_ref().is_none_or(|(title, _, _)| title != &id) {
                launch_ordinal += 1;
                active = Some((id, launch_ordinal, line_index));
            }
        }
        let Some((build_id, original_length)) = main_nso_build(line) else {
            continue;
        };
        out.main_queries += 1;
        let Some((title_id, ordinal, start_index)) = active.as_ref() else {
            out.orphan_main_builds += 1;
            continue;
        };
        if line_index.saturating_sub(*start_index) > MAX_CONTEXT_LINES {
            out.orphan_main_builds += 1;
            continue;
        }
        // Duplicate messages in the same launch do not count as new builds.
        if seen.insert((*ordinal, title_id.clone(), build_id.clone())) {
            if out.witness.len() >= MAX_OBSERVATIONS {
                return Err("Too many Eden log Build ID observations".into());
            }
            out.witness.push(BuildWitness {
                title_id: title_id.clone(),
                build_id,
                original_length,
                launch_ordinal: *ordinal,
            });
        }
    }
    Ok(out)
}

pub(crate) fn log_default() -> PathBuf {
    let data = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_default();
    data.join("eden/log/eden_log.txt")
}

fn archived_match(index: &GameIndex, title: &str, build: &str) -> bool {
    let key = format!(
        "title-id-{}-build-{}",
        title.to_ascii_lowercase(),
        build.to_ascii_lowercase()
    );
    index.by_candidate_key(&key).is_some()
}

fn report_log(path: &Path, index: &GameIndex) -> Result<Report, ErrBox> {
    let metadata = fs::symlink_metadata(path);
    let mut report = Report {
        schema_version: 1,
        format: "cheatarium-eden-log-build-observations-v1",
        read_only: true,
        emulator_files_written: 0,
        game_file_bytes_read: 0,
        log_path: path.to_string_lossy().into_owned(),
        log_found: metadata.is_ok(),
        log_bytes_read: 0,
        historical_main_build_observations: Vec::new(),
        observed_title_ids: 0,
        multiple_builds_for_same_title: Vec::new(),
        main_builds_without_launch_context: 0,
        core_launch_contexts_seen: 0,
        main_nso_patch_queries_seen: 0,
        direct_cheat_lookup_observations: Vec::new(),
        log_has_main_executable_build_evidence: false,
        installation_allowed: false,
        warnings: vec![
            "Log-derived builds only describe prior launch activity, not the current installed executable.",
            "A matching archived cheat Build ID does not prove that the cheat works or is safe.",
            "Boot/Loading title context can be interrupted or stale; manual verification is still required.",
            "Cheat-file lookup logs pair title and build but are not proof that the queried executable is main.",
            "No game containers, encryption keys, saves, or emulator configuration are accessed.",
            "No cheat files are exported, installed or activated.",
        ],
    };
    let meta = match metadata {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(report),
        Err(e) => return Err(e.into()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("Eden log must be a normal file, not a symlink or directory".into());
    }
    if meta.len() > MAX_LOG_BYTES {
        return Err("Eden log exceeds read-only size limit (32 MiB)".into());
    }
    let bytes = fs::read(path)?;
    if bytes.len() as u64 > MAX_LOG_BYTES {
        return Err("Eden log changed during read and exceeded size limit".into());
    }
    report.log_bytes_read = bytes.len();
    let log = std::str::from_utf8(&bytes)?;
    let parsed = parse_log(log)?;
    report.main_builds_without_launch_context = parsed.orphan_main_builds;
    report.core_launch_contexts_seen = parsed.core_launches;
    report.main_nso_patch_queries_seen = parsed.main_queries;
    report.direct_cheat_lookup_observations = parsed
        .cheat_lookup_witness
        .into_iter()
        .map(|(title_id, build_id, event)| CheatLookupObservation {
            archived_build_candidate_found: archived_match(index, &title_id, &build_id),
            title_id,
            build_id,
            event,
            main_executable_build_verified: false,
            current_installed_build_verified: false,
        })
        .collect();
    let mut titles = BTreeMap::<String, BTreeSet<String>>::new();
    for w in parsed.witness {
        titles
            .entry(w.title_id.clone())
            .or_default()
            .insert(w.build_id.clone());
        report
            .historical_main_build_observations
            .push(ObservedBuild {
                archived_build_candidate_found: archived_match(index, &w.title_id, &w.build_id),
                title_id: w.title_id,
                main_build_id: w.build_id,
                original_build_hex_length: w.original_length,
                launch_context_ordinal: w.launch_ordinal,
                evidence: "same_log_launch_context_and_main_nso_patch_query",
                current_installed_build_verified: false,
                compatible_cheats_verified: false,
            });
    }
    report.observed_title_ids = titles.len();
    report.log_has_main_executable_build_evidence =
        !report.historical_main_build_observations.is_empty();
    report.multiple_builds_for_same_title = titles
        .into_iter()
        .filter_map(|(id, builds)| (builds.len() > 1).then_some(id))
        .collect();
    Ok(report)
}

#[derive(Debug, Clone)]
pub(crate) struct MainBuildObservation {
    pub(crate) title_id: String,
    pub(crate) build_id: String,
    pub(crate) archived_candidate: bool,
    pub(crate) launch_context_ordinal: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct WeakLookupObservation {
    pub(crate) title_id: String,
    pub(crate) build_id: String,
}

#[derive(Debug)]
pub(crate) struct LaunchEvidence {
    pub(crate) log_found: bool,
    pub(crate) log_bytes_read: usize,
    pub(crate) core_launch_contexts: usize,
    pub(crate) main_queries: usize,
    pub(crate) orphan_queries: usize,
    pub(crate) main: Vec<MainBuildObservation>,
    pub(crate) weak_lookup: Vec<WeakLookupObservation>,
}

/// Reuse the same bounded Eden log parser in discovery; never reinterpret
/// a cheat-file lookup as main executable evidence or an installed Build ID.
pub(crate) fn read_launch_evidence(
    path: &Path,
    index: &GameIndex,
) -> Result<LaunchEvidence, ErrBox> {
    let report = report_log(path, index)?;
    Ok(LaunchEvidence {
        log_found: report.log_found,
        log_bytes_read: report.log_bytes_read,
        core_launch_contexts: report.core_launch_contexts_seen,
        main_queries: report.main_nso_patch_queries_seen,
        orphan_queries: report.main_builds_without_launch_context,
        main: report
            .historical_main_build_observations
            .into_iter()
            .map(|record| MainBuildObservation {
                title_id: record.title_id,
                build_id: record.main_build_id,
                archived_candidate: record.archived_build_candidate_found,
                launch_context_ordinal: record.launch_context_ordinal,
            })
            .collect(),
        weak_lookup: report
            .direct_cheat_lookup_observations
            .into_iter()
            .map(|record| WeakLookupObservation {
                title_id: record.title_id,
                build_id: record.build_id,
            })
            .collect(),
    })
}

fn run() -> Result<(), ErrBox> {
    let mut log = log_default();
    let mut db = PathBuf::from("generated/v1");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--log" => log = PathBuf::from(args.next().ok_or("--log requires a path")?),
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--json" => {}
            "--help" | "-h" => {
                println!("cheatarium-eden-builds --db generated/v1 [--log ~/.local/share/eden/log/eden_log.txt] [--json]\nOnly reads Eden's log; outputs historical, unverified main executable Build IDs.");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    verify_platform_distribution(&db, "switch")?;
    let index = load_game_candidates(&db, "switch")?;
    let report = report_log(&log, &index)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("cheatarium-eden-builds: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn associates_main_build_with_matching_core_title_launch() {
        let log = concat!(
            "[  1.00] Core <Info> core/core.cpp:Load:351: Loading Super Mario Odyssey (0100000000010000) ...\n",
            "[  1.10] Loader <Info> patch_manager.cpp:HasNSOPatch:475: Querying NSO patch existence for build_id=B424BE150A8E7D780000000000000000, name=main\n",
            "[  1.11] Loader <Info> patch_manager.cpp:HasNSOPatch:475: Querying NSO patch existence for build_id=F5DCCDDb37e977240000, name=subsdk0\n",
            "[  1.13] Frontend <Info> main_window.cpp: Booting game: 0100000000010000 | Mario Odyssey | 1.0\n"
        );
        let data = parse_log(log).unwrap();
        assert_eq!(data.witness.len(), 1);
        assert_eq!(data.witness[0].title_id, "0100000000010000");
        assert_eq!(data.witness[0].build_id, "B424BE150A8E7D78");
        assert_eq!(data.orphan_main_builds, 0);
    }

    #[test]
    fn different_game_launch_does_not_inherit_previous_title_id() {
        let log = concat!(
            "[  0.00] Loader <Info>: Querying NSO patch existence for build_id=1234567890ABCDEF, name=main\n",
            "[  1.00] Core <Info>: Loading Super Mario Odyssey (0100000000010000) ...\n",
            "[  1.10] Loader <Info>: Querying NSO patch existence for build_id=B424BE150A8E7D78, name=main\n",
            "[  3.00] Frontend <Info>: Booting game: 0100152000022000 | Mario Kart\n",
            "[  3.10] Loader <Info>: Querying NSO patch existence for build_id=96dfd1769e560c27, name=main\n"
        );
        let data = parse_log(log).unwrap();
        assert_eq!(data.orphan_main_builds, 1);
        assert_eq!(data.witness.len(), 2);
        assert_eq!(data.witness[0].title_id, "0100000000010000");
        assert_eq!(data.witness[1].title_id, "0100152000022000");
    }

    #[test]
    fn paired_cheat_lookup_is_reported_separately_from_main_executable() {
        let log = concat!(
            "[  0.0] Common.Filesystem <Info> patch_manager.cpp:93: No cheats file found for title_id=0100000000010000, build_id=B424BE150A8E7D78\n",
            "[  0.1] Common.Filesystem <Info> patch_manager.cpp:93: No cheats file found for title_id=0100000000010000, build_id=B424BE150A8E7D78\n",
            "[  0.2] Common.Filesystem <Info> patch_manager.cpp:100: Failed to read cheats file for title_id=0100152000022000, build_id=96DFD1769E560C27\n",
            "[  0.3] Loader <Info> patch_manager.cpp:475: Querying NSO patch existence for build_id=B424BE150A8E7D78, name=main\n"
        );
        let parsed = parse_log(log).unwrap();
        assert_eq!(parsed.main_queries, 1);
        assert_eq!(parsed.orphan_main_builds, 1);
        assert_eq!(parsed.witness.len(), 0);
        assert_eq!(parsed.cheat_lookup_witness.len(), 2);
        assert_eq!(parsed.core_launches, 0);
        assert!(direct_cheat_lookup(
            "No cheats file found for title_id=0100000000010000, build_id=BAD"
        )
        .is_none());
        assert!(direct_cheat_lookup(
            "No cheats file found for title_id=0100000000010000, build_id=B424BE150A8E7D7800"
        )
        .is_none());
        assert!(direct_cheat_lookup(
            "Failed to read cheats file for title_id=FOOBAR, build_id=B424BE150A8E7D78"
        )
        .is_none());
    }

    #[test]
    fn rejects_short_fake_builds_and_missing_context() {
        assert!(
            main_nso_build("Querying NSO patch existence for build_id=DEADBEEF, name=main")
                .is_none()
        );
        assert!(main_nso_build(
            "Querying NSO patch existence for build_id=B424BE150A8E7D78, name=other"
        )
        .is_none());
        assert!(loading_title("Unrelated loading (0100000000010000)").is_none());
        assert_eq!(
            parse_log("Querying NSO patch existence for build_id=B424BE150A8E7D78, name=main")
                .unwrap()
                .witness
                .len(),
            0
        );
        assert!(title_after("Booting game: 01000000000100001", "Booting game: ").is_none());
    }
}
