//! Private, bounded Cheatarium-owned snapshots of Eden Build ID observations.
//! Captures only Title ID/Build ID/evidence kind, never full emulator log lines.
use super::LaunchEvidence;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_HISTORY_BYTES: u64 = 1024 * 1024;
const MAX_HISTORY_RECORDS: usize = 4096;
const SCHEMA_VERSION: u32 = 1;
const HISTORY_FORMAT: &str = "cheatarium-eden-minimal-build-history-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Entry {
    pub(super) title_id: String,
    pub(super) build_id: String,
    pub(super) evidence_kind: String,
    /// Time this observation was captured from a log, NOT game launch time.
    pub(super) first_captured_unix_seconds: u64,
    pub(super) last_captured_unix_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct History {
    pub(super) schema_version: u32,
    pub(super) format: String,
    pub(super) last_log_sha256: Option<String>,
    pub(super) entries: Vec<Entry>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            format: HISTORY_FORMAT.to_owned(),
            last_log_sha256: None,
            entries: Vec::new(),
        }
    }
}

pub(super) fn default_path() -> PathBuf {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_default();
    base.join("cheatarium/eden-build-history.json")
}

fn hex_id(value: &str) -> bool {
    value.len() == 16 && value.starts_with("01")
        && value.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn validate(history: &History) -> Result<(), ErrBox> {
    if history.schema_version != SCHEMA_VERSION || history.format != HISTORY_FORMAT {
        return Err("Unsupported Cheatarium Eden history schema".into());
    }
    if history.entries.len() > MAX_HISTORY_RECORDS {
        return Err("Cheatarium Eden history exceeds entry limit".into());
    }
    if history.last_log_sha256.as_deref().is_some_and(|x| !valid_hash(x)) {
        return Err("Invalid Cheatarium Eden history log fingerprint".into());
    }
    let mut keys = BTreeSet::new();
    for item in &history.entries {
        if !hex_id(&item.title_id)
            || item.build_id.len() != 16
            || !item.build_id.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase())
            || !matches!(item.evidence_kind.as_str(), "main_nso" | "cheat_lookup")
            || item.first_captured_unix_seconds > item.last_captured_unix_seconds
            || !keys.insert((&item.title_id, &item.build_id, &item.evidence_kind))
        {
            return Err("Invalid or duplicate Cheatarium Eden history entry".into());
        }
    }
    Ok(())
}

/// Guard parent directories, not only the leaf, to avoid symlink-based
/// redirection into Eden or another unrelated application.
fn check_path(path: &Path, creating: bool) -> Result<(), ErrBox> {
    if !path.is_absolute() || path.file_name().is_none()
        || path.components().any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("Cheatarium history path must be an absolute normalized file path".into());
    }
    if path.components().any(|part| matches!(part, Component::Normal(name)
        if name.to_string_lossy().eq_ignore_ascii_case("eden")))
    {
        return Err("Refusing history path inside an Eden directory".into());
    }
    let mut existing = path.to_path_buf();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err("History file or parent is a symlink".into());
                }
                if existing == path && !meta.is_file() {
                    return Err("History destination is not a regular file".into());
                }
                if existing != path && !meta.is_dir() {
                    return Err("History parent is not a directory".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !creating && existing == path {
                    // Absent store means a normal, empty history.
                }
            }
            Err(e) => return Err(e.into()),
        }
        if !existing.pop() { break; }
    }
    Ok(())
}

pub(super) fn load(path: &Path) -> Result<(History, bool), ErrBox> {
    check_path(path, false)?;
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((History::default(), false)),
        Err(e) => return Err(e.into()),
    };
    if !meta.is_file() || meta.len() > MAX_HISTORY_BYTES {
        return Err("Cheatarium Eden history is not a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(MAX_HISTORY_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_HISTORY_BYTES {
        return Err("Cheatarium Eden history exceeds size limit".into());
    }
    let history: History = serde_json::from_slice(&bytes)?;
    validate(&history)?;
    Ok((history, true))
}

fn snapshot_items(log: &LaunchEvidence, captured: u64) -> Vec<Entry> {
    let mut found = BTreeSet::new();
    for item in &log.main {
        found.insert((item.title_id.clone(), item.build_id.clone(), "main_nso"));
    }
    for item in &log.weak_lookup {
        found.insert((item.title_id.clone(), item.build_id.clone(), "cheat_lookup"));
    }
    found.into_iter().map(|(title_id, build_id, kind)| Entry {
        title_id, build_id, evidence_kind: kind.to_owned(),
        first_captured_unix_seconds: captured,
        last_captured_unix_seconds: captured,
    }).collect()
}

fn merge(history: &mut History, log: &LaunchEvidence, captured: u64) -> Result<bool, ErrBox> {
    let Some(fingerprint) = &log.log_sha256 else { return Ok(false) };
    if history.last_log_sha256.as_deref() == Some(fingerprint.as_str()) {
        return Ok(false);
    }
    let additions = snapshot_items(log, captured);
    if additions.is_empty() {
        // An empty or rotated log must never erase saved Build ID evidence.
        return Ok(false);
    }
    let mut map = std::collections::BTreeMap::new();
    for record in history.entries.drain(..) {
        map.insert((record.title_id.clone(), record.build_id.clone(), record.evidence_kind.clone()), record);
    }
    for record in additions {
        let key = (record.title_id.clone(), record.build_id.clone(), record.evidence_kind.clone());
        if let Some(old) = map.get_mut(&key) {
            old.last_captured_unix_seconds = old.last_captured_unix_seconds.max(captured);
        } else {
            map.insert(key, record);
        }
    }
    if map.len() > MAX_HISTORY_RECORDS {
        return Err("Cheatarium Eden history capacity reached; no entries were discarded".into());
    }
    history.entries = map.into_values().collect();
    history.last_log_sha256 = Some(fingerprint.clone());
    validate(history)?;
    Ok(true)
}

struct CaptureLock(PathBuf);
impl Drop for CaptureLock {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

#[cfg(unix)]
fn private_options() -> OpenOptions {
    use std::os::unix::fs::OpenOptionsExt;
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true).mode(0o600);
    opts
}
#[cfg(not(unix))]
fn private_options() -> OpenOptions {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    opts
}

fn prepare_dir(parent: &Path) -> Result<(), ErrBox> {
    if !parent.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700).create(parent)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Only explicit capture writes to Cheatarium's own state directory. Reads
/// stay read-only, and Eden's log/config/mods are never touched.
pub(super) fn capture(
    path: &Path,
    log: &LaunchEvidence,
) -> Result<(History, bool), ErrBox> {
    if log.main.is_empty() && log.weak_lookup.is_empty() {
        return load(path).map(|(history, _)| (history, false));
    }
    check_path(path, true)?;
    let parent = path.parent().ok_or("History has no parent directory")?;
    prepare_dir(parent)?;
    check_path(path, true)?;
    let name = path.file_name().ok_or("History has no filename")?.to_string_lossy();
    let lock_path = parent.join(format!(".{name}.lock"));
    let lock = private_options().open(&lock_path)?;
    let _lock = CaptureLock(lock_path);
    drop(lock);
    let (mut history, _) = load(path)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if !merge(&mut history, log, now)? { return Ok((history, false)); }
    let encoded = serde_json::to_vec_pretty(&history)?;
    if encoded.len() as u64 > MAX_HISTORY_BYTES {
        return Err("Cheatarium Eden history serialized size limit exceeded".into());
    }
    let temp = parent.join(format!(".{name}.tmp-{}", std::process::id()));
    let mut out = private_options().open(&temp)?;
    let outcome = (|| -> Result<(), ErrBox> {
        out.write_all(&encoded)?;
        out.sync_all()?;
        drop(out);
        check_path(path, true)?;
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if outcome.is_err() { let _ = fs::remove_file(&temp); }
    outcome?;
    Ok((history, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{MainBuildObservation, WeakLookupObservation};
    use std::time::SystemTime;

    fn witness() -> LaunchEvidence {
        LaunchEvidence {
            log_found: true,
            log_bytes_read: 333,
            log_sha256: Some("a".repeat(64)),
            core_launch_contexts: 1,
            main_queries: 1,
            orphan_queries: 0,
            main: vec![MainBuildObservation {
                title_id: "0100000000010000".into(), build_id: "B424BE150A8E7D78".into(),
                archived_candidate: true, launch_context_ordinal: 1,
            }],
            weak_lookup: vec![WeakLookupObservation {
                title_id: "0100152000022000".into(), build_id: "96DFD1769E560C27".into(),
            }],
        }
    }

    #[test]
    fn merge_is_idempotent_and_survives_new_logs_and_empty_rotation() {
        let mut history = History::default();
        let first = witness();
        assert!(merge(&mut history, &first, 100).unwrap());
        assert_eq!(history.entries.len(), 2);
        assert!(!merge(&mut history, &first, 110).unwrap());
        assert_eq!(history.entries[0].last_captured_unix_seconds, 100);
        let mut rotated = first;
        rotated.log_sha256 = Some("b".repeat(64));
        rotated.main.clear();
        rotated.weak_lookup.clear();
        assert!(!merge(&mut history, &rotated, 200).unwrap());
        assert_eq!(history.entries.len(), 2);
        rotated.main.push(MainBuildObservation {
            title_id: "0100000000010000".into(), build_id: "B424BE150A8E7D78".into(),
            archived_candidate: true, launch_context_ordinal: 1,
        });
        assert!(merge(&mut history, &rotated, 210).unwrap());
        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.entries.iter().find(|r| r.evidence_kind == "main_nso").unwrap().last_captured_unix_seconds, 210);
    }

    #[test]
    fn writes_only_minimal_cheatarium_state_and_reloads() {
        let unique = format!("cheatarium-history-test-{}-{:?}-{:?}",
            std::process::id(), std::thread::current().id(), SystemTime::now());
        let root = env::temp_dir().join(unique);
        let path = root.join("history.json");
        let log = witness();
        let (stored, written) = capture(&path, &log).unwrap();
        assert!(written);
        assert_eq!(stored.entries.len(), 2);
        let (reloaded, exists) = load(&path).unwrap();
        assert!(exists);
        assert_eq!(reloaded.entries.len(), 2);
        let serialized = fs::read_to_string(&path).unwrap();
        assert!(!serialized.contains("eden_log.txt"));
        assert!(!serialized.contains("/drive/roms/"));
        assert_eq!(capture(&path, &log).unwrap().1, false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_symlink_and_eden_destinations_and_corrupt_history() {
        let root = env::temp_dir().join(format!("cheatarium-history-safety-{}-{:?}",
            std::process::id(), std::thread::current().id()));
        fs::create_dir_all(&root).unwrap();
        assert!(capture(&root.join("eden/history.json"), &witness()).is_err());
        let path = root.join("history.json");
        fs::write(&path, b"not valid JSON").unwrap();
        assert!(capture(&path, &witness()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not valid JSON");
        #[cfg(unix)]
        {
            let target = root.join("target.json");
            fs::write(&target, b"{}").unwrap();
            let link = root.join("link.json");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(capture(&link, &witness()).is_err());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
