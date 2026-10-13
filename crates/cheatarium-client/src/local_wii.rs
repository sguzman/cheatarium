//! Read-only private Wii Gecko sidecar consumer.
//!
//! This is deliberately NOT the published Cheatarium index. Every code
//! belongs to a user-owned source cache and remains disabled and unverified.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
const FORMAT: &str = "cheatarium.private_wii_gecko_sidecar.v1";
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub struct WiiSidecar {
    pub schema_version: u32,
    pub format: String,
    pub platform: String,
    pub source_verified_original_files: usize,
    pub source_native_group_count: usize,
    pub native_instruction_pair_count: usize,
    pub game_builds_verified: usize,
    pub gameplay_tested_effects: usize,
    pub all_groups_disabled_by_default: bool,
    pub private_source_codes_present: bool,
    pub public_archive_import: bool,
    pub third_party_code_redistribution_rights_cleared: bool,
    pub records: Vec<WiiLocalGroup>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WiiLocalGroup {
    pub platform: String,
    pub claimed_disc_id: String,
    pub source_revision_token: Option<String>,
    pub source_variant_token: String,
    pub game_family_title_hint: String,
    pub source_compatibility_warning: String,
    pub original_source_id: String,
    pub original_local_path: String,
    pub original_git_blob_sha1: String,
    pub original_upstream_url: String,
    pub original_group_index: usize,
    pub source_group_title: String,
    pub source_notes: Vec<String>,
    pub source_marked_enabled: Option<bool>,
    pub code_format: String,
    pub native_code_lines: Vec<String>,
    pub native_code_pair_count: usize,
    pub enabled_for_execution: bool,
    pub game_build_authenticated: bool,
    pub gameplay_verified: bool,
    pub original_author_redistribution_rights_cleared: bool,
}

pub fn default_path() -> PathBuf {
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"));
    home.join("cheatarium/wii-local-sidecar/codes.json")
}

fn valid_hex_pair(line: &str) -> bool {
    let mut words = line.split_ascii_whitespace();
    let first = words.next();
    let second = words.next();
    let valid = |s: &str| s.len() == 8 && s.bytes().all(|c| c.is_ascii_hexdigit());
    first.is_some_and(valid) && second.is_some_and(valid) && words.next().is_none()
}

fn valid_disc_id(id: &str) -> bool {
    id.len() == 6 && id.bytes().all(|x| x.is_ascii_uppercase() || x.is_ascii_digit())
}

impl WiiSidecar {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || self.format != FORMAT
            || self.platform != "wii"
            || self.source_verified_original_files != 29
            || self.source_native_group_count != 45
            || self.native_instruction_pair_count != 5611
            || self.records.len() != 45
            || self.game_builds_verified != 0
            || self.gameplay_tested_effects != 0
            || !self.all_groups_disabled_by_default
            || !self.private_source_codes_present
            || self.public_archive_import
            || self.third_party_code_redistribution_rights_cleared
        {
            return Err("Unexpected or unsafe private Wii sidecar schema/policy".into());
        }
        let mut source_groups = HashSet::new();
        let mut source_files = HashSet::new();
        let mut lines = 0usize;
        for r in &self.records {
            if r.platform != "wii"
                || !valid_disc_id(&r.claimed_disc_id)
                || r.source_variant_token != "base"
                    && !matches!(r.source_variant_token.as_str(), "mp1" | "mp2" | "mp3")
                || r.source_revision_token.as_ref().is_some_and(|v| {
                    !v.starts_with('v')
                        || v.len() < 2
                        || !v[1..].bytes().all(|b| b.is_ascii_digit())
                })
                || r.original_local_path.starts_with('/')
                || !r.original_local_path.starts_with("wii/")
                || r.original_local_path.split('/').any(|x| x.is_empty() || x == ".." || x == ".")
                || r.original_git_blob_sha1.len() != 40
                || !r.original_git_blob_sha1.bytes().all(|b| b.is_ascii_hexdigit())
                || !r.original_upstream_url.starts_with("https://github.com/")
                || r.source_compatibility_warning.trim().is_empty()
                || r.game_family_title_hint.trim().is_empty()
                || r.source_group_title.trim().is_empty()
                || r.code_format != "gecko-native-eight-hex-word-pairs"
                || r.original_group_index == 0
                || r.native_code_lines.is_empty()
                || r.native_code_pair_count != r.native_code_lines.len()
                || !r.native_code_lines.iter().all(|line| valid_hex_pair(line))
                || r.enabled_for_execution
                || r.game_build_authenticated
                || r.gameplay_verified
                || r.original_author_redistribution_rights_cleared
            {
                return Err("Unsafe or malformed private Wii Gecko group".into());
            }
            if !source_groups.insert((
                r.original_source_id.as_str(),
                r.original_local_path.as_str(),
                r.original_group_index,
            )) {
                return Err("Duplicate private Wii Gecko source group identity".into());
            }
            source_files.insert((r.original_source_id.as_str(), r.original_local_path.as_str()));
            lines += r.native_code_pair_count;
        }
        if lines != 5611 || source_files.len() != 29 {
            return Err("Wii native source files or instruction pairs are missing".into());
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> Result<WiiSidecar> {
    let file_info = fs::symlink_metadata(path)?;
    if file_info.file_type().is_symlink() || !file_info.is_file() || file_info.len() > MAX_BYTES {
        return Err("Private Wii sidecar is a symlink, nonfile or too large".into());
    }
    let data = fs::read(path)?;
    if data.len() as u64 > MAX_BYTES {
        return Err("Private Wii sidecar exceeds size limit".into());
    }
    let sidecar: WiiSidecar = serde_json::from_slice(&data)?;
    sidecar.validate()?;
    Ok(sidecar)
}

#[derive(Default)]
pub struct Filters<'a> {
    pub game_id: Option<&'a str>,
    pub revision: Option<&'a str>,
    pub variant: Option<&'a str>,
    pub game_title: Option<&'a str>,
    pub group_title: Option<&'a str>,
    pub group_index: Option<usize>,
    pub show_code: bool,
    pub offset: usize,
    pub limit: usize,
}

pub fn query(sidecar: &WiiSidecar, f: &Filters<'_>) -> Result<Value> {
    sidecar.validate()?;
    if !(1..=100).contains(&f.limit) {
        return Err("--limit must be between 1 and 100".into());
    }
    let game_id = f.game_id.map(str::to_ascii_uppercase);
    if game_id.as_deref().is_some_and(|v| !valid_disc_id(v)) {
        return Err("--game-id must be a six-character source disc ID".into());
    }
    let revision = f.revision.map(|v| {
        let number = v.strip_prefix('v').unwrap_or(v);
        format!("v{number}")
    });
    if revision.as_deref().is_some_and(|r| {
        r.len() < 2 || !r[1..].bytes().all(|b| b.is_ascii_digit())
    }) {
        return Err("--revision needs a source revision number".into());
    }
    if f.variant.is_some_and(|v| !matches!(v, "base" | "mp1" | "mp2" | "mp3")) {
        return Err("--variant must be base, mp1, mp2 or mp3".into());
    }
    if f.show_code && (game_id.is_none() || f.group_index.is_none() || f.offset != 0) {
        return Err("--show-code requires --game-id, --group-index and offset 0".into());
    }
    if f.group_index.is_some() && !f.show_code {
        return Err("--group-index requires explicit --show-code".into());
    }
    if f.group_index == Some(0) {
        return Err("--group-index is one-based".into());
    }

    let candidates: Vec<_> = sidecar.records.iter().filter(|g| {
        game_id.as_deref().is_none_or(|id| g.claimed_disc_id == id)
            && revision.as_deref().is_none_or(|r| g.source_revision_token.as_deref() == Some(r))
            && f.variant.is_none_or(|v| g.source_variant_token == v)
            && f.game_title.is_none_or(|v| g.game_family_title_hint.to_lowercase().contains(&v.to_lowercase()))
    }).collect();
    let files: HashSet<_> = candidates.iter().map(|g| (&g.original_source_id, &g.original_local_path)).collect();
    if f.show_code && files.len() != 1 {
        return Err("Ambiguous native Wii disc revision or variant; specify --revision and/or --variant".into());
    }
    let matches: Vec<_> = candidates.into_iter().filter(|g| {
        f.group_title.is_none_or(|v| {
            g.source_group_title.to_lowercase().contains(&v.to_lowercase())
                || g.source_notes.iter().any(|x| x.to_lowercase().contains(&v.to_lowercase()))
        }) && (!f.show_code || f.group_index == Some(g.original_group_index))
    }).collect();
    if f.show_code && matches.len() != 1 {
        return Err("Source Gecko group index absent or filtered from selected file".into());
    }
    let total = matches.len();
    let sliced = if f.show_code { matches } else { matches.into_iter().skip(f.offset).take(f.limit).collect() };
    let rows: Vec<Value> = sliced.into_iter().map(|g| json!({
        "claimed_disc_id":g.claimed_disc_id,
        "source_revision_token":g.source_revision_token,
        "source_variant_token":g.source_variant_token,
        "game_family_title_hint":g.game_family_title_hint,
        "source_compatibility_warning":g.source_compatibility_warning,
        "original_source_id":g.original_source_id,
        "original_local_path":g.original_local_path,
        "original_git_blob_sha1":g.original_git_blob_sha1,
        "original_upstream_url":g.original_upstream_url,
        "group_index":g.original_group_index,
        "group_title":g.source_group_title,
        "source_notes":g.source_notes,
        "native_code_pair_count":g.native_code_pair_count,
        "source_marked_enabled":g.source_marked_enabled,
        "enabled_for_execution":false,
        "native_code_lines": if f.show_code { Some(&g.native_code_lines) } else { None },
    })).collect();
    Ok(json!({
        "schema":"cheatarium.private_wii_gecko_query.v1",
        "matched_source_files":files.len(),
        "matched_native_groups":total,
        "returned":rows.len(),
        "offset":f.offset,
        "limit":f.limit,
        "has_more": !f.show_code && f.offset.saturating_add(f.limit) < total,
        "original_code_text_included":f.show_code,
        "local_private_sidecar":true,
        "original_git_bytes_reverified_by_this_command":false,
        "original_source_verification_available_via":"python3 tools/export_wii_gecko_local.py --verify",
        "code_activated":false,
        "game_build_authenticated":false,
        "gameplay_verified":false,
        "redistribution_rights_cleared":false,
        "results":rows,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake() -> WiiSidecar {
        let mut rows = Vec::new();
        for i in 0..29 {
            let groups = if i < 16 { 2 } else { 1 };
            for j in 0..groups {
                let lines = if i == 0 && j == 0 { 5567 } else { 1 };
                rows.push(WiiLocalGroup {
                    platform: "wii".into(),
                    claimed_disc_id: if i < 2 { "RUUE01".into() } else { format!("X{i:05}") },
                    source_revision_token: if i == 0 { Some("v0".into()) }
                        else if i == 1 { Some("v1".into()) } else { None },
                    source_variant_token: "base".into(),
                    game_family_title_hint: "City Folk Synthetic".into(),
                    source_compatibility_warning: "Unverified revision".into(),
                    original_source_id: format!("synthetic-{i}"),
                    original_local_path: format!("wii/synthetic-{i}/gecko/ABCDEF{i}.txt"),
                    original_git_blob_sha1: "a".repeat(40),
                    original_upstream_url: "https://github.com/x/y/blob/a/source.txt".into(),
                    original_group_index: j + 1,
                    source_group_title: "Controller Code".into(),
                    source_notes: vec!["By synthetic author".into()],
                    source_marked_enabled: None,
                    code_format: "gecko-native-eight-hex-word-pairs".into(),
                    native_code_lines: vec!["01234567 89ABCDEF".into(); lines],
                    native_code_pair_count: lines,
                    enabled_for_execution: false,
                    game_build_authenticated: false,
                    gameplay_verified: false,
                    original_author_redistribution_rights_cleared: false,
                });
            }
        }
        WiiSidecar {
            schema_version: 1, format: FORMAT.into(), platform: "wii".into(),
            source_verified_original_files: 29, source_native_group_count: 45,
            native_instruction_pair_count: 5611, game_builds_verified: 0,
            gameplay_tested_effects: 0, all_groups_disabled_by_default: true,
            private_source_codes_present: true, public_archive_import: false,
            third_party_code_redistribution_rights_cleared: false, records: rows,
        }
    }

    #[test]
    fn fixture_validates_and_rejects_enabled_effects() {
        let mut d = fake();
        d.validate().unwrap();
        d.records[0].enabled_for_execution = true;
        assert!(d.validate().is_err());
        d.records[0].enabled_for_execution = false;
        d.records[0].native_code_lines[0] = "INVALID".into();
        assert!(d.validate().is_err());
    }

    #[test]
    fn explicit_revision_is_required_for_code_view() {
        let d = fake();
        assert_eq!(query(&d, &Filters {
            game_id: Some("RUUE01"), limit: 20, ..Default::default()
        }).unwrap()["matched_source_files"], 2);
        assert!(query(&d, &Filters {
            game_id: Some("RUUE01"), show_code: true, group_index: Some(1),
            limit: 20, ..Default::default()
        }).is_err());
        let one = query(&d, &Filters {
            game_id: Some("RUUE01"), revision: Some("0"), show_code: true,
            group_index: Some(1), limit: 20, ..Default::default()
        }).unwrap();
        assert_eq!(one["matched_source_files"], 1);
        assert_eq!(one["results"][0]["native_code_lines"].as_array().unwrap().len(), 5567);
    }

    #[test]
    fn default_hides_original_cheat_text() {
        let d = fake();
        let output = query(&d, &Filters {
            game_title: Some("city"), limit: 3, ..Default::default()
        }).unwrap();
        assert_eq!(output["returned"], 3);
        assert_eq!(output["original_code_text_included"], false);
        assert!(output["results"][0]["native_code_lines"].is_null());
    }
}
