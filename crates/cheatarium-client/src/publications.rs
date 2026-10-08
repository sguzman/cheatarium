//! Read-only historical SNES multi-part code publication witnesses.
//!
//! An external listing is evidence of published source text, never proof of
//! cartridge compatibility, executable code grouping or observed gameplay.
use crate::{load_platform, Bundle, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_PUBLICATION_BYTES: u64 = 8 * 1024 * 1024;
const CLAIM_TYPE: &str = "externally-published-multi-part-source-code-text";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnesPublicationRegistry {
    pub schema_version: u32,
    pub platform: String,
    pub claim_type: String,
    pub evidence_limit: String,
    pub records: Vec<SnesPublication>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SnesPublication {
    pub candidate_game_key: String,
    pub source_record_id: String,
    pub source_git_blob_sha: String,
    pub source_ordinal: usize,
    pub raw_code: String,
    pub published_text_segments: Vec<String>,
    pub published_effect_description: String,
    pub publication: PublicationReference,
    pub execution_observed: bool,
    pub rom_match_verified: bool,
    pub safe_to_auto_apply: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PublicationReference {
    pub url: String,
    pub reference: String,
    pub source_revision: String,
}

fn nonblank(s: &str) -> bool {
    !s.trim().is_empty()
}

impl SnesPublicationRegistry {
    /// Resolve every publication witness against an original imported source
    /// and exact original ordinal. No positive execution claims are permitted.
    pub fn validate_for_bundle(&self, bundle: &Bundle) -> Result<()> {
        if self.schema_version != 1
            || self.platform != "snes"
            || bundle.platform != "snes"
            || bundle.schema_version != 1
            || self.claim_type != CLAIM_TYPE
            || !nonblank(&self.evidence_limit)
        {
            return Err("Unsupported SNES publication evidence schema".into());
        }
        let sources: BTreeMap<_, _> = bundle
            .records
            .iter()
            .map(|record| (record.id.as_str(), record))
            .collect();
        if sources.len() != bundle.records.len() {
            return Err("Repeated original source ID in SNES bundle".into());
        }
        let mut seen = BTreeSet::new();
        for witness in &self.records {
            if !seen.insert((&witness.source_record_id, witness.source_ordinal)) {
                return Err("Duplicate source ordinal in publication witnesses".into());
            }
            if witness.execution_observed
                || witness.rom_match_verified
                || witness.safe_to_auto_apply
                || !nonblank(&witness.published_effect_description)
                || !witness.publication.url.starts_with("https://")
                || !nonblank(&witness.publication.reference)
                || !nonblank(&witness.publication.source_revision)
                || !witness.raw_code.contains('+')
                || witness.published_text_segments.len() < 2
                || witness
                    .published_text_segments
                    .iter()
                    .any(|segment| !nonblank(segment) || segment != segment.trim())
            {
                return Err("Invalid external publication witness or invented verification".into());
            }
            let original_segments: Vec<_> = witness.raw_code.split('+').map(str::trim).collect();
            if witness
                .published_text_segments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != original_segments
            {
                return Err("Historical code components differ from original text".into());
            }
            let source = sources
                .get(witness.source_record_id.as_str())
                .ok_or("Historical witness source record not found")?;
            let candidate = if source.candidate_game_key.is_empty() {
                format!("unresolved:{}", source.id)
            } else {
                source.candidate_game_key.clone()
            };
            if candidate != witness.candidate_game_key
                || source.provenance.git_blob_sha != witness.source_git_blob_sha
            {
                return Err("Historical witness game candidate or Git blob mismatch".into());
            }
            let mut matches = source
                .codes
                .iter()
                .filter(|code| code.ordinal == witness.source_ordinal);
            let code = matches.next().ok_or("Historical witness ordinal not found")?;
            if matches.next().is_some()
                || code.role.as_deref() != Some("code")
                || code.code.as_deref() != Some(witness.raw_code.as_str())
                || code
                    .composition
                    .as_ref()
                    .is_none_or(|composition| composition.relation != "unresolved")
            {
                return Err("Historical witness is not an exact unresolved source code".into());
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn by_candidate_game_key(&self, game_key: &str) -> Vec<&SnesPublication> {
        self.records
            .iter()
            .filter(|entry| entry.candidate_game_key == game_key)
            .collect()
    }

    #[must_use]
    pub fn by_source_record(&self, record_id: &str) -> Vec<&SnesPublication> {
        self.records
            .iter()
            .filter(|entry| entry.source_record_id == record_id)
            .collect()
    }
}

/// Load a bounded local, versioned artifact and independently validate it
/// against the actual SNES source bundle. Pin the distribution commit for
/// authenticity; local JSON and checksums cannot authenticate themselves.
pub fn load_snes_publications(root: impl AsRef<Path>) -> Result<SnesPublicationRegistry> {
    let root = root.as_ref();
    let bundle = load_platform(root, "snes")?;
    let mut content = Vec::new();
    File::open(root.join("interpretations/snes-published-groups.json"))?
        .take(MAX_PUBLICATION_BYTES + 1)
        .read_to_end(&mut content)?;
    if content.len() as u64 > MAX_PUBLICATION_BYTES {
        return Err("SNES publication registry exceeds size limit".into());
    }
    let registry: SnesPublicationRegistry = serde_json::from_slice(&content)?;
    registry.validate_for_bundle(&bundle)?;
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Bundle, SnesPublicationRegistry) {
        let bundle: Bundle = serde_json::from_value(serde_json::json!({
            "schema_version":1,"platform":"snes","game_identity_rule":"advisory",
            "compatibility_rule":"unverified","records":[{
                "id":"source:game","title_hint":"Game","candidate_game_key":"game",
                "identity_confidence":"filename-candidate","raw_filename":"game.cht",
                "region_hint":"USA","format_hint":"game-genie","declared_cheats":1,
                "parse_warnings":[],"codes":[{
                    "ordinal":3,"description":"Example","code":"ABCD-EFGH+1234-5678",
                    "source_enabled":false,"verification":"unverified","role":"code",
                    "composition":{"relation":"unresolved","alternatives":[],
                        "evidence":[],"rom_match_verified":false,
                        "simultaneous_execution_confirmed":false}
                }],"provenance":{
                    "source_id":"archive","repository":"https://example.org",
                    "revision":"r1","license":"MIT","upstream_path":"game.cht",
                    "archive_path":"archive/game.cht",
                    "git_blob_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }
            }]
        })).unwrap();
        let registry: SnesPublicationRegistry = serde_json::from_value(serde_json::json!({
            "schema_version":1,"platform":"snes",
            "claim_type":"externally-published-multi-part-source-code-text",
            "evidence_limit":"Published text only; execution unverified",
            "records":[{
                "candidate_game_key":"game","source_record_id":"source:game",
                "source_git_blob_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "source_ordinal":3,"raw_code":"ABCD-EFGH+1234-5678",
                "published_text_segments":["ABCD-EFGH","1234-5678"],
                "published_effect_description":"Historical example",
                "publication":{"url":"https://example.org/list","reference":"Entry 4",
                               "source_revision":"r1"},
                "execution_observed":false,"rom_match_verified":false,
                "safe_to_auto_apply":false
            }]
        })).unwrap();
        (bundle, registry)
    }

    #[test]
    fn exact_original_source_is_required() {
        let (bundle, mut registry) = fixture();
        registry.validate_for_bundle(&bundle).unwrap();
        assert_eq!(registry.by_candidate_game_key("game").len(), 1);
        assert!(registry.by_candidate_game_key("other").is_empty());
        assert_eq!(registry.by_source_record("source:game").len(), 1);
        registry.records[0].source_ordinal = 10;
        assert!(registry.validate_for_bundle(&bundle).is_err());
    }

    #[test]
    fn rejects_fabricated_compatibility_and_tampered_code() {
        let (bundle, mut registry) = fixture();
        registry.records[0].execution_observed = true;
        assert!(registry.validate_for_bundle(&bundle).is_err());
        registry.records[0].execution_observed = false;
        registry.records[0].rom_match_verified = true;
        assert!(registry.validate_for_bundle(&bundle).is_err());
        registry.records[0].rom_match_verified = false;
        registry.records[0].safe_to_auto_apply = true;
        assert!(registry.validate_for_bundle(&bundle).is_err());
        registry.records[0].safe_to_auto_apply = false;
        registry.records[0].published_text_segments.reverse();
        assert!(registry.validate_for_bundle(&bundle).is_err());
    }

    #[test]
    fn rejects_wrong_blob_game_key_and_unsupported_publication_link() {
        let (bundle, mut registry) = fixture();
        registry.records[0].source_git_blob_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();
        assert!(registry.validate_for_bundle(&bundle).is_err());
        registry.records[0].source_git_blob_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
        registry.records[0].candidate_game_key = "unrelated".to_owned();
        assert!(registry.validate_for_bundle(&bundle).is_err());
        registry.records[0].candidate_game_key = "game".to_owned();
        registry.records[0].publication.url = "http://example.org".to_owned();
        assert!(registry.validate_for_bundle(&bundle).is_err());
    }

    #[test]
    fn rejects_duplicate_and_reviewed_source_entries() {
        let (mut bundle, mut registry) = fixture();
        registry.records.push(registry.records[0].clone());
        assert!(registry.validate_for_bundle(&bundle).is_err());
        registry.records.pop();
        bundle.records[0].codes[0]
            .composition.as_mut().unwrap().relation = "revision-alternatives".to_owned();
        assert!(registry.validate_for_bundle(&bundle).is_err());
    }
}
