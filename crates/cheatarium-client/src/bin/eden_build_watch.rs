//! On-demand, bounded Eden log watcher that preserves *only* minimal Build ID history.
//! It never installs cheats or modifies Eden settings, game content, or save data.
use cheatarium_client::{load_game_candidates, verify_platform_distribution};
use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

#[path = "eden_builds.rs"]
#[allow(dead_code)]
mod eden_builds;

type ErrBox = Box<dyn Error + Send + Sync>;

const MAX_DURATION_SECS: u64 = 6 * 3600;
const MIN_POLL_MS: u64 = 250;
const MAX_POLL_MS: u64 = 10_000;

#[derive(Debug, Serialize)]
struct Outcome {
    format: &'static str,
    completed: bool,
    watched_seconds: u64,
    expected_title_ids: Vec<String>,
    captured_title_ids: Vec<String>,
    missing_title_ids: Vec<String>,
    history_path: String,
    eden_settings_written: bool,
    eden_game_files_written: bool,
    installed_cheats: usize,
    warning: &'static str,
}

fn valid_title_id(value: &str) -> bool {
    value.len() == 16
        && value.starts_with("01")
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_lowercase())
}

fn parse_targets(value: &str) -> Result<BTreeSet<String>, ErrBox> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 25 {
        return Err("Expected 1 to 25 comma-delimited Title IDs".into());
    }
    let mut out = BTreeSet::new();
    for value in parts {
        let title = value.trim().to_ascii_uppercase();
        if !valid_title_id(&title) || !out.insert(title) {
            return Err("Malformed or repeated requested Switch Title ID".into());
        }
    }
    Ok(out)
}

fn found_mains(history: &eden_builds::eden_history::History) -> BTreeSet<String> {
    history.entries.iter()
        .filter(|record| record.evidence_kind == "main_nso")
        .map(|record| record.title_id.clone())
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
struct Signature {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    inode: u64,
}
fn signature(path: &Path) -> Result<Option<Signature>, ErrBox> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("Eden log path is not a regular, nonsymlink file".into());
    }
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    Ok(Some(Signature {
        len: meta.len(),
        modified: meta.modified().ok(),
        #[cfg(unix)]
        inode: meta.ino(),
    }))
}

fn run() -> Result<(), ErrBox> {
    let mut db = PathBuf::from("generated/v1");
    let mut log = eden_builds::log_default();
    let mut history = eden_builds::eden_history::default_path();
    let mut duration = 7200u64;
    let mut poll_ms = 750u64;
    let mut requested = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--log" => log = PathBuf::from(args.next().ok_or("--log requires a path")?),
            "--history-file" => history = PathBuf::from(args.next().ok_or("--history-file requires a path")?),
            "--duration-seconds" => duration = args.next().ok_or("--duration-seconds requires an integer")?.parse()?,
            "--poll-ms" => poll_ms = args.next().ok_or("--poll-ms requires an integer")?.parse()?,
            "--until-title-ids" => requested = Some(parse_targets(&args.next().ok_or("--until-title-ids requires comma-delimited IDs")?)?),
            "--help" | "-h" => {
                println!("Watch Eden's log and preserve main Build IDs in private state. --db PATH --log PATH --history-file PATH --duration-seconds 1..21600 --poll-ms 250..10000 --until-title-ids ID,ID");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    if duration == 0 || duration > MAX_DURATION_SECS || !(MIN_POLL_MS..=MAX_POLL_MS).contains(&poll_ms) {
        return Err("Duration must be 1..21600 seconds and poll interval 250..10000 milliseconds".into());
    }
    let requested = requested.ok_or("--until-title-ids is required to avoid indefinite unscoped monitoring")?;
    verify_platform_distribution(&db, "switch")?;
    let index = load_game_candidates(&db, "switch")?;
    let (mut stored, _) = eden_builds::eden_history::load(&history)?;
    let started = Instant::now();
    let mut seen_signature = None;
    let mut observed = found_mains(&stored);
    eprintln!("Cheatarium watching Eden launch log for {} Title IDs (at most {} seconds; no Eden writes)", requested.len(), duration);

    while !requested.is_subset(&observed) && started.elapsed().as_secs() < duration {
        let current = signature(&log)?;
        if current.is_some() && current != seen_signature {
            let evidence = eden_builds::read_launch_evidence(&log, &index)?;
            let before = observed.clone();
            let (new_stored, _) = eden_builds::eden_history::capture(&history, &evidence)?;
            stored = new_stored;
            observed = found_mains(&stored);
            for found in observed.intersection(&requested).filter(|id| !before.contains(*id)) {
                eprintln!("Cheatarium captured main executable Build ID for game {found}");
            }
        }
        seen_signature = current;
        if requested.is_subset(&observed) {
            break;
        }
        let remaining = Duration::from_secs(duration).saturating_sub(started.elapsed());
        thread::sleep(Duration::from_millis(poll_ms).min(remaining));
    }
    let captured = requested.intersection(&observed).cloned().collect::<Vec<_>>();
    let missing = requested.difference(&observed).cloned().collect::<Vec<_>>();
    let report = Outcome {
        format: "cheatarium-eden-build-watch-v1",
        completed: missing.is_empty(),
        watched_seconds: started.elapsed().as_secs(),
        expected_title_ids: requested.into_iter().collect(),
        captured_title_ids: captured,
        missing_title_ids: missing,
        history_path: history.display().to_string(),
        eden_settings_written: false,
        eden_game_files_written: false,
        installed_cheats: 0,
        warning: "Stored Build IDs are historical launch evidence, not current installed executable proof or cheat runtime validation.",
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("cheatarium-eden-build-watch: {err}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_and_duplicate_title_ids() {
        assert!(parse_targets("").is_err());
        assert!(parse_targets("01002DA013484000,01002DA013484000").is_err());
        assert!(parse_targets("01002DA013484001").is_ok());
        assert!(parse_targets("hello").is_err());
    }

    #[test]
    fn weak_lookups_do_not_count_as_captured_main_ids() {
        let mut history = eden_builds::eden_history::History::default();
        history.entries.push(eden_builds::eden_history::Entry {
            title_id: "01002DA013484000".into(),
            build_id: "5441AF3C89F97E1D".into(),
            evidence_kind: "cheat_lookup".into(),
            first_captured_unix_seconds: 1,
            last_captured_unix_seconds: 1,
        });
        assert!(found_mains(&history).is_empty());
        history.entries[0].evidence_kind = "main_nso".into();
        assert!(found_mains(&history).contains("01002DA013484000"));
    }
}
