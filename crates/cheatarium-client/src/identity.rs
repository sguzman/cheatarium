//! Local SHA-256 ROM fingerprint evidence. No ROM assets are distributed.
//! Hash equality is not proof that a cheat is compatible with a cartridge.
//! Filename-derived Cheatarium game candidates are never used as fallbacks.
use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Take};
use std::path::Path;

const MAX_REGISTRY_BYTES: u64 = 8 * 1024 * 1024;
pub const HASH_SCOPE: &str = "sha256-entire-file-unaltered";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    pub sha256: String,
    pub byte_length: u64,
    pub hash_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub url: String,
    /// Location within the reference (table, record, or other reproducible locator).
    pub reference: String,
    pub source_revision: String,
    /// Review is an assessment of the evidence, NOT cheat execution validation.
    pub review_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseClaim {
    pub sha256: String,
    pub byte_length: u64,
    pub game_id: String,
    pub title: String,
    pub edition_id: String,
    pub region: Option<String>,
    pub revision: Option<String>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRegistry {
    pub schema_version: u32,
    pub platform: String,
    pub hash_scope: String,
    pub records: Vec<ReleaseClaim>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FingerprintLookup<'a> {
    pub schema_version: u32,
    pub platform: &'a str,
    pub hash_scope: &'a str,
    pub sha256: &'a str,
    pub status: &'static str,
    pub candidate_only: bool,
    pub cheat_compatibility_verified: bool,
    pub matching_release_claims: Vec<&'a ReleaseClaim>,
}

pub fn sha256_valid(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

pub fn fingerprint_reader(mut reader: impl Read) -> Result<Fingerprint> {
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        size = size.checked_add(n as u64).ok_or("Fingerprint byte length overflow")?;
    }
    Ok(Fingerprint {
        sha256: format!("{:x}", hasher.finalize()),
        byte_length: size,
        hash_scope: HASH_SCOPE.to_owned(),
    })
}

/// Hash user-selected local bytes exactly, including any SNES copier header.
/// No header normalization, ROM parsing, uploads or modifications.
pub fn fingerprint_file(path: impl AsRef<Path>) -> Result<Fingerprint> {
    fingerprint_reader(File::open(path)?)
}

fn valid_platform(platform: &str) -> bool {
    !platform.is_empty()
        && platform.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl IdentityRegistry {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || !valid_platform(&self.platform) || self.hash_scope != HASH_SCOPE {
            return Err("Unsupported or malformed ROM identity registry header".into());
        }
        for record in &self.records {
            if !sha256_valid(&record.sha256)
                || record.byte_length == 0
                || record.game_id.trim().is_empty()
                || record.title.trim().is_empty()
                || record.edition_id.trim().is_empty()
                || record.evidence.is_empty()
            {
                return Err("Malformed ROM hash/release evidence record".into());
            }
            for evidence in &record.evidence {
                if !evidence.url.starts_with("https://")
                    || evidence.reference.trim().is_empty()
                    || evidence.source_revision.trim().is_empty()
                    || !matches!(evidence.review_state.as_str(), "candidate" | "reviewed")
                {
                    return Err("Malformed ROM identity provenance reference".into());
                }
            }
        }
        Ok(())
    }

    /// A documented SHA claim never silently becomes a verified cheat binding.
    /// Multiple claims for the same hash are reported as conflicts.
    pub fn lookup(&self, hash: &str) -> Result<FingerprintLookup<'_>> {
        self.lookup_with_length(hash, None)
    }

    /// Require the exact byte length when it is known. Any disagreement in
    /// reported size remains an explicit conflict, never a silent selection.
    pub fn lookup_with_length(&self, hash: &str, known_length: Option<u64>) -> Result<FingerprintLookup<'_>> {
        if !sha256_valid(hash) {
            return Err("Expected lowercase 64-character SHA-256 digest".into());
        }
        self.validate()?;
        let matching_release_claims: Vec<&ReleaseClaim> =
            self.records.iter().filter(|record| record.sha256 == hash).collect();
        let length_conflict = known_length.is_some_and(|size| {
            matching_release_claims.iter().any(|claim| claim.byte_length != size)
        });
        let status = if length_conflict {
            "fingerprint_length_conflict"
        } else {
            match matching_release_claims.len() {
                0 => "no_evidence",
                1 => "documented_release_claim",
                _ => "conflicting_release_claims",
            }
        };
        Ok(FingerprintLookup {
            schema_version: 1,
            platform: &self.platform,
            hash_scope: HASH_SCOPE,
            sha256: hash,
            status,
            candidate_only: true,
            cheat_compatibility_verified: false,
            matching_release_claims,
        })
    }
}

pub fn load_registry(root: impl AsRef<Path>, platform: &str) -> Result<IdentityRegistry> {
    if !valid_platform(platform) {
        return Err("Unsafe ROM identity platform identifier".into());
    }
    let file = File::open(root.as_ref().join(format!("{platform}.json")))?;
    let mut data = Vec::new();
    let mut limited: Take<File> = file.take(MAX_REGISTRY_BYTES + 1);
    limited.read_to_end(&mut data)?;
    if data.len() as u64 > MAX_REGISTRY_BYTES {
        return Err("ROM identity registry exceeds allowed size".into());
    }
    let registry: IdentityRegistry = serde_json::from_slice(&data)?;
    registry.validate()?;
    if registry.platform != platform {
        return Err("ROM identity platform mismatch".into());
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC_SHA: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn claim(edition: &str) -> ReleaseClaim {
        ReleaseClaim {
            sha256: ABC_SHA.to_owned(), byte_length: 3,
            game_id: "fixture-game".to_owned(),
            title: "Fixture game (not a real ROM)".to_owned(),
            edition_id: edition.to_owned(),
            region: None, revision: None,
            evidence: vec![Evidence {
                url: "https://example.invalid/test-evidence".to_owned(),
                reference: "fixture row 1".to_owned(),
                source_revision: "test-only".to_owned(),
                review_state: "candidate".to_owned(),
            }],
        }
    }

    fn registry(records: Vec<ReleaseClaim>) -> IdentityRegistry {
        IdentityRegistry {
            schema_version: 1, platform: "snes".to_owned(),
            hash_scope: HASH_SCOPE.to_owned(), records,
        }
    }

    #[test]
    fn hashes_exact_original_bytes_without_guessing_copier_headers() {
        let bytes = b"abc";
        let fp = fingerprint_reader(&bytes[..]).unwrap();
        assert_eq!(fp.sha256, ABC_SHA);
        assert_eq!(fp.byte_length, 3);
        assert_eq!(fp.hash_scope, HASH_SCOPE);
        let with_header = fingerprint_reader([vec![0; 512], bytes.to_vec()].concat().as_slice()).unwrap();
        assert_ne!(fp.sha256, with_header.sha256);
        assert_eq!(with_header.byte_length, 515);
    }

    #[test]
    fn zero_evidence_is_not_a_rom_match() {
        let reg = registry(vec![]);
        let result = reg.lookup(ABC_SHA).unwrap();
        assert_eq!(result.status, "no_evidence");
        assert!(result.candidate_only);
        assert!(!result.cheat_compatibility_verified);
    }

    #[test]
    fn documented_hash_claim_does_not_verify_cheat_compatibility() {
        let reg = registry(vec![claim("test-revision")]);
        let result = reg.lookup(ABC_SHA).unwrap();
        assert_eq!(result.status, "documented_release_claim");
        assert_eq!(result.matching_release_claims.len(), 1);
        assert!(!result.cheat_compatibility_verified);
    }

    #[test]
    fn mismatching_file_length_cannot_be_considered_an_identity_hit() {
        let reg = registry(vec![claim("release-a")]);
        let result = reg.lookup_with_length(ABC_SHA, Some(8)).unwrap();
        assert_eq!(result.status, "fingerprint_length_conflict");
        assert!(!result.cheat_compatibility_verified);
        let result = reg.lookup_with_length(ABC_SHA, Some(3)).unwrap();
        assert_eq!(result.status, "documented_release_claim");
    }

    #[test]
    fn multiple_release_claims_must_not_be_silently_resolved() {
        let reg = registry(vec![claim("release-a"), claim("release-b")]);
        let result = reg.lookup(ABC_SHA).unwrap();
        assert_eq!(result.status, "conflicting_release_claims");
        assert_eq!(result.matching_release_claims.len(), 2);
    }

    #[test]
    fn rejects_bad_hashes_provenance_and_platform() {
        let mut reg = registry(vec![claim("e1")]);
        reg.records[0].evidence[0].url = "file:///tmp/personal.rom".to_owned();
        assert!(reg.validate().is_err());
        let mut reg = registry(vec![claim("e1")]);
        reg.records[0].sha256 = "zzzz".to_owned();
        assert!(reg.validate().is_err());
        assert!(registry(vec![]).lookup("SUPER-MARIO-WORLD").is_err());
        assert!(!valid_platform("../snes"));
        assert!(!valid_platform("SNES"));
    }
}
