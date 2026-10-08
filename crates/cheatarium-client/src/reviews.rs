//! Source-bound reviews of claimed cheat effects, independent of lexical tags.
//! Reports, observed runs, and failed reproduction attempts remain distinct.
//! No review result grants permission to auto-activate or assert universal ROM compatibility.
use crate::{load_platform, Result};
use crate::identity::sha256_valid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_REVIEW_REGISTRY: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectReviewRegistry {
    pub schema_version: u32,
    pub format: String,
    pub description: String,
    pub claims: Vec<EffectReview>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectReview {
    pub id: String,
    pub platform: String,
    pub source_record_id: String,
    pub source_ordinal: usize,
    pub source_revision: String,
    pub source_git_blob_sha: String,
    pub effect_category: String,
    pub assessment: String,
    pub reviewed_by: String,
    pub review_date: String,
    pub assessment_note: String,
    pub evidence: Vec<ReviewEvidence>,
    pub test_context: Option<ReviewTestContext>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewEvidence {
    pub url: String,
    pub reference: String,
    pub source_revision: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReviewTestContext {
    pub rom_sha256: String,
    pub emulator: String,
    pub emulator_version: String,
    pub code_device_or_core: String,
    pub test_date: String,
    pub outcome: String,
    pub observed_behavior: String,
}

fn nonblank(s: &str) -> bool {
    !s.trim().is_empty()
}

fn date_shape(date: &str) -> bool {
    let b = date.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

impl EffectReviewRegistry {
    /// Validate claim fields and all links for this exact platform. Must never
    /// infer an observation from lexical tags or parsed code syntax.
    pub fn validate_for_platform(
        &self,
        bundle: &crate::Bundle,
        categories: &BTreeSet<String>,
    ) -> Result<()> {
        if self.schema_version != 1 || self.format != "cheatarium-effect-reviews-v1" {
            return Err("Unsupported Cheatarium effect review registry".into());
        }
        let mut ids = BTreeSet::new();
        for claim in &self.claims {
            if !ids.insert(claim.id.as_str()) {
                return Err("Duplicate effect review claim ID".into());
            }
            if claim.platform != bundle.platform {
                return Err("Unexpected platform in scoped effect review registry".into());
            }
            if !nonblank(&claim.id) || !categories.contains(&claim.effect_category)
                || !nonblank(&claim.reviewed_by) || !nonblank(&claim.assessment_note)
                || !date_shape(&claim.review_date)
                || claim.evidence.is_empty()
                || !matches!(claim.assessment.as_str(), "reported" | "observed" | "not-reproduced")
            {
                return Err("Incomplete or unsupported effect assessment".into());
            }
            if claim.evidence.iter().any(|e| {
                !e.url.starts_with("https://") || !nonblank(&e.reference)
                    || !nonblank(&e.source_revision)
            }) {
                return Err("Missing public effect assessment evidence".into());
            }
            let record = bundle.records.iter()
                .find(|record| record.id == claim.source_record_id)
                .ok_or("Reviewed cheat source record is missing")?;
            if record.provenance.revision != claim.source_revision
                || record.provenance.git_blob_sha != claim.source_git_blob_sha
            {
                return Err("Reviewed cheat source revision or blob has changed".into());
            }
            let code = record.codes.iter().find(|c| c.ordinal == claim.source_ordinal)
                .ok_or("Reviewed original cheat ordinal is missing")?;
            if !(code.is_code() || code.is_memory_entry()) {
                return Err("A section heading cannot be a reviewed cheat".into());
            }
            match (&*claim.assessment, &claim.test_context) {
                ("reported", None) => (),
                ("observed" | "not-reproduced", Some(ctx)) => {
                    if !sha256_valid(&ctx.rom_sha256)
                        || !nonblank(&ctx.emulator)
                        || !nonblank(&ctx.emulator_version)
                        || !nonblank(&ctx.code_device_or_core)
                        || !nonblank(&ctx.observed_behavior)
                        || !date_shape(&ctx.test_date)
                        || ctx.outcome != claim.assessment
                    {
                        return Err("Observed claim lacks matching exact-ROM test evidence".into());
                    }
                }
                _ => return Err("Reported and observed cheat evidence cannot be conflated".into()),
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn by_source_record(&self, source_id: &str) -> Vec<&EffectReview> {
        self.claims.iter().filter(|c| c.source_record_id == source_id).collect()
    }

    #[must_use]
    pub fn by_effect_category(&self, category: &str) -> Vec<&EffectReview> {
        self.claims.iter().filter(|c| c.effect_category == category).collect()
    }
}

/// Read a bounded artifact, then resolve all scoped claims against the actual
/// original source entry and pinned source blob. Integrity still requires the
/// consumer to pin a trusted distribution commit separately.
pub fn load_effect_reviews(root: impl AsRef<Path>, platform: &str) -> Result<EffectReviewRegistry> {
    let root = root.as_ref();
    let bundle = load_platform(root, platform)?;
    let mut content = Vec::new();
    File::open(root.join("reviews.json"))?
        .take(MAX_REVIEW_REGISTRY + 1).read_to_end(&mut content)?;
    if content.len() as u64 > MAX_REVIEW_REGISTRY {
        return Err("Effect review registry exceeds allowed size".into());
    }
    let data: EffectReviewRegistry = serde_json::from_slice(&content)?;
    if data.schema_version != 1 || data.format != "cheatarium-effect-reviews-v1" {
        return Err("Unsupported effect review registry".into());
    }
    let mut taxonomy = Vec::new();
    File::open(root.join("taxonomy/effects-v1.json"))?
        .take(1024 * 1024).read_to_end(&mut taxonomy)?;
    let taxonomy: serde_json::Value = serde_json::from_slice(&taxonomy)?;
    let categories: BTreeSet<String> = taxonomy["categories"].as_array()
        .ok_or("Invalid effect taxonomy")?
        .iter()
        .filter_map(|entry| entry["id"].as_str().map(str::to_owned))
        .collect();
    let data = EffectReviewRegistry {
        claims: data.claims.into_iter().filter(|c| c.platform == platform).collect(),
        ..data
    };
    data.validate_for_platform(&bundle, &categories)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_bundle;
    use flate2::{Compression, GzBuilder};
    use std::io::Write;

    fn fixture_bundle() -> crate::Bundle {
        let raw = serde_json::json!({
            "schema_version":1, "platform":"snes", "game_identity_rule":"test",
            "compatibility_rule":"manual", "records":[{
                "id":"source:fixture", "title_hint":"Test", "candidate_game_key":"test",
                "identity_confidence":"filename_heuristic_only", "raw_filename":"test.cht",
                "region_hint":null, "format_hint":null, "declared_cheats":1,
                "parse_warnings":[], "codes":[
                    {"ordinal":1, "code":"ABCD", "description":"Infinite Lives",
                     "verification":"unverified","source_enabled":false,"role":"code"}
                ],
                "provenance":{"source_id":"test","repository":"https://example.org",
                  "revision":"r1","git_blob_sha":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                  "license":"test","archive_path":"archive/test","upstream_path":"test.cht"}
            }]
        });
        let mut gz = GzBuilder::new().mtime(0).write(Vec::new(), Compression::fast());
        gz.write_all(serde_json::to_string(&raw).unwrap().as_bytes()).unwrap();
        decode_bundle(gz.finish().unwrap().as_slice()).unwrap()
    }

    fn claim() -> EffectReview {
        EffectReview {
            id:"fixture-review".to_owned(), platform:"snes".to_owned(),
            source_record_id:"source:fixture".to_owned(), source_ordinal:1,
            source_revision:"r1".to_owned(),
            source_git_blob_sha:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
            effect_category:"lives".to_owned(), assessment:"reported".to_owned(),
            reviewed_by:"fixture".to_owned(), review_date:"2026-10-08".to_owned(),
            assessment_note:"Reported by another source, not tested".to_owned(),
            evidence:vec![ReviewEvidence {
                url:"https://example.org/ref".to_owned(),
                reference:"entry 1".to_owned(),source_revision:"fixture-r1".to_owned(),
            }],
            test_context:None,
        }
    }

    fn registry(claims: Vec<EffectReview>) -> EffectReviewRegistry {
        EffectReviewRegistry {
            schema_version:1,
            format:"cheatarium-effect-reviews-v1".to_owned(),
            description:"fixture".to_owned(),
            claims,
        }
    }

    #[test]
    fn empty_review_registry_cannot_promote_imported_cheats() {
        let data = registry(vec![]);
        let categories = BTreeSet::from(["lives".to_owned()]);
        data.validate_for_platform(&fixture_bundle(), &categories).unwrap();
        assert!(data.by_effect_category("lives").is_empty());
    }

    #[test]
    fn externally_reported_effect_is_not_an_observed_run() {
        let data = registry(vec![claim()]);
        data.validate_for_platform(&fixture_bundle(), &BTreeSet::from(["lives".to_owned()])).unwrap();
        assert_eq!(data.by_source_record("source:fixture")[0].assessment, "reported");
        assert!(data.claims[0].test_context.is_none());
    }

    #[test]
    fn observed_claims_require_compatible_evidence_and_correct_origin() {
        let mut c = claim();
        c.assessment = "observed".to_owned();
        let valid = ReviewTestContext {
            rom_sha256:"a".repeat(64), emulator:"Fixture Emulator".to_owned(),
            emulator_version:"0.1".to_owned(), code_device_or_core:"Fixture core".to_owned(),
            test_date:"2026-10-08".to_owned(), outcome:"observed".to_owned(),
            observed_behavior:"Lives counter remained stable".to_owned(),
        };
        let categories = BTreeSet::from(["lives".to_owned()]);
        assert!(registry(vec![c.clone()]).validate_for_platform(&fixture_bundle(), &categories).is_err());
        c.test_context = Some(valid.clone());
        registry(vec![c.clone()]).validate_for_platform(&fixture_bundle(), &categories).unwrap();
        c.source_ordinal = 99;
        assert!(registry(vec![c.clone()]).validate_for_platform(&fixture_bundle(), &categories).is_err());
        c.source_ordinal = 1;
        c.source_git_blob_sha = "altered".to_owned();
        assert!(registry(vec![c.clone()]).validate_for_platform(&fixture_bundle(), &categories).is_err());
        c.source_git_blob_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();
        c.test_context = Some(ReviewTestContext { outcome: "not-reproduced".to_owned(), ..valid });
        assert!(registry(vec![c]).validate_for_platform(&fixture_bundle(), &categories).is_err());
    }
}
