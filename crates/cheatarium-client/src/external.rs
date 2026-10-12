//! Integrity-checked, filename-only third-party bulk references.
//! These are not archived code entries, executable patches, or gameplay tests.
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;

use crate::Result;

const FORMAT: &str = "cheatarium-external-references-v1";
const STATUS: &str = "link_only_rights_review";
const MAX_META: usize = 5 * 1024 * 1024;
const MAX_JSON: u64 = 24 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalCatalog {
    schema_version: u32,
    format: String,
    metadata_only: bool,
    rights_status: String,
    gameplay_verified: bool,
    archive_included: bool,
    reference_files: usize,
    platforms: Vec<ExternalPlatform>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalPlatform {
    platform: String,
    artifact: String,
    source_files: usize,
    source_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalBundle {
    schema_version: u32,
    format: String,
    platform: String,
    metadata_only: bool,
    rights_status: String,
    gameplay_verified: bool,
    records: Vec<ExternalReference>,
}

/// A path and source identity, never a cheat code or executable patch.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalReference {
    pub source_id: String,
    pub kind: String,
    pub path: String,
    pub git_blob_sha: String,
    pub bytes: u64,
    pub repository: String,
    pub revision: String,
    pub upstream_url: String,
    pub rights_status: String,
}

#[derive(Debug, Deserialize)]
struct Dist {
    schema_version: u32,
    format: String,
    files: Vec<DistFile>,
}

#[derive(Debug, Deserialize)]
struct DistFile {
    path: String,
    sha256: String,
    size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ExternalSearch {
    pub schema: &'static str,
    pub query: String,
    pub platform_filter: Option<String>,
    pub source_filter: Option<String>,
    pub external_reference_files_total: usize,
    pub external_platforms: usize,
    pub total: usize,
    pub offset: usize,
    pub returned: usize,
    pub has_more: bool,
    pub metadata_only: bool,
    pub redistribution: &'static str,
    pub gameplay_verified: bool,
    pub archive_included: bool,
    pub artifact_checksums_verified: bool,
    pub manifest_authenticated: bool,
    pub results: Vec<ExternalReference>,
}

fn checked_file(root: &Path, dist: &Dist, rel: &str) -> Result<Vec<u8>> {
    // Relatives are program-constructed from validated platform slugs.
    if !(rel == "external/catalog.json"
        || (rel.starts_with("external/")
            && rel.ends_with(".json.gz")
            && rel[9..rel.len() - 8]
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')))
    {
        return Err("Unsafe external artifact path".into());
    }
    let digest = dist
        .files
        .iter()
        .find(|f| f.path == rel)
        .ok_or("External artifact is absent from the distribution manifest")?;
    let bytes = fs::read(root.join(rel))?;
    if bytes.len() as u64 != digest.size_bytes || bytes.len() > MAX_JSON as usize {
        return Err("External reference artifact size mismatch".into());
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != digest.sha256 {
        return Err("External reference artifact SHA-256 mismatch".into());
    }
    Ok(bytes)
}

fn read_bundle(root: &Path, dist: &Dist, platform: &ExternalPlatform) -> Result<ExternalBundle> {
    let path = format!("external/{}.json.gz", platform.platform);
    if platform.artifact != path {
        return Err("External platform artifact path mismatch".into());
    }
    let bytes = checked_file(root, dist, &path)?;
    let mut data = Vec::new();
    GzDecoder::new(&bytes[..])
        .take(MAX_JSON + 1)
        .read_to_end(&mut data)?;
    if data.len() as u64 > MAX_JSON {
        return Err("Uncompressed external reference bundle exceeds size limit".into());
    }
    let bundle: ExternalBundle = serde_json::from_slice(&data)?;
    if bundle.schema_version != 1
        || bundle.format != FORMAT
        || bundle.platform != platform.platform
        || !bundle.metadata_only
        || bundle.rights_status != STATUS
        || bundle.gameplay_verified
        || bundle.records.len() != platform.source_files
    {
        return Err("Incorrect metadata-only external reference bundle".into());
    }
    let sources: BTreeSet<_> = platform.source_ids.iter().map(String::as_str).collect();
    if sources.len() != platform.source_ids.len() {
        return Err("Duplicate external source IDs".into());
    }
    let mut previous: Option<(&str, &str)> = None;
    for record in &bundle.records {
        if !sources.contains(record.source_id.as_str())
            || !matches!(record.kind.as_str(), "cheat" | "patch")
            || record.rights_status != STATUS
            || record.git_blob_sha.len() != 40
            || !record.git_blob_sha.bytes().all(|c| c.is_ascii_hexdigit())
            || !record.repository.starts_with("https://github.com/")
            || record.revision.len() != 40
            || !record.revision.bytes().all(|c| c.is_ascii_hexdigit())
            || record.path.is_empty()
            || record.path.starts_with('/')
            || record.path.contains('\\')
            || record.path.contains('\0')
            || record
                .path
                .split('/')
                .any(|seg| seg.is_empty() || seg == "." || seg == "..")
            || !record
                .upstream_url
                .starts_with(&format!("{}/blob/{}/", record.repository, record.revision))
        {
            return Err("Invalid or unsafe external source metadata".into());
        }
        let now = (record.source_id.as_str(), record.path.as_str());
        if previous.is_some_and(|prior| prior >= now) {
            return Err("External file references are not strictly ordered".into());
        }
        previous = Some(now);
    }
    Ok(bundle)
}

/// Searches only pinned metadata using a local, checksum-checked export.
/// Checksums do not authenticate the manifest: consumers must pin a trusted commit.
pub fn search(
    root: impl AsRef<Path>,
    query: &str,
    platform: Option<&str>,
    source_id: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<ExternalSearch> {
    if query.trim().is_empty() || !(1..=100).contains(&limit) {
        return Err("External reference query must be nonblank; limit must be 1..100".into());
    }
    let root = root.as_ref();
    let dist_bytes = fs::read(root.join("distribution.json"))?;
    if dist_bytes.len() > MAX_META {
        return Err("External distribution manifest too large".into());
    }
    let dist: Dist = serde_json::from_slice(&dist_bytes)?;
    if dist.schema_version != 1 || dist.format != "cheatarium-distribution-v1" {
        return Err("Wrong Cheatarium distribution manifest".into());
    }
    let catalog_bytes = checked_file(root, &dist, "external/catalog.json")?;
    if catalog_bytes.len() > MAX_META {
        return Err("External catalog too large".into());
    }
    let catalog: ExternalCatalog = serde_json::from_slice(&catalog_bytes)?;
    if catalog.schema_version != 1
        || catalog.format != FORMAT
        || !catalog.metadata_only
        || catalog.rights_status != STATUS
        || catalog.gameplay_verified
        || catalog.archive_included
    {
        return Err("Invalid or unsafe external reference catalog".into());
    }
    let mut counted = 0;
    let mut hits = Vec::new();
    let query_lower = query.to_lowercase();
    let mut matching_platform = false;
    for entry in &catalog.platforms {
        if entry.platform.is_empty()
            || !entry
                .platform
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err("Invalid external platform slug".into());
        }
        counted += entry.source_files;
        if platform.is_some_and(|requested| requested != entry.platform) {
            continue;
        }
        matching_platform = true;
        let bundle = read_bundle(root, &dist, entry)?;
        for reference in bundle.records {
            if !reference.path.to_lowercase().contains(&query_lower)
                || source_id.is_some_and(|id| id != reference.source_id)
            {
                continue;
            }
            hits.push(reference);
        }
    }
    if counted != catalog.reference_files
        || catalog.platforms.is_empty()
        || (platform.is_some() && !matching_platform)
    {
        return Err("External catalog count or requested platform mismatch".into());
    }
    let total = hits.len();
    let results = hits.into_iter().skip(offset).take(limit).collect();
    Ok(ExternalSearch {
        schema: FORMAT,
        query: query.to_owned(),
        platform_filter: platform.map(str::to_owned),
        source_filter: source_id.map(str::to_owned),
        external_reference_files_total: catalog.reference_files,
        external_platforms: catalog.platforms.len(),
        total,
        offset,
        returned: total.saturating_sub(offset).min(limit),
        has_more: offset.saturating_add(limit) < total,
        metadata_only: true,
        redistribution: STATUS,
        gameplay_verified: false,
        archive_included: false,
        artifact_checksums_verified: true,
        manifest_authenticated: false,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_payload_fields_and_fake_licensing() {
        let record = serde_json::json!({
            "source_id": "x", "kind": "cheat", "path": "a.txt",
            "git_blob_sha": "a".repeat(40), "bytes": 5, "repository": "https://github.com/a/b",
            "revision": "b".repeat(40), "upstream_url": "https://github.com/a/b",
            "rights_status": STATUS, "code": "DO NOT REHOST"
        });
        assert!(serde_json::from_value::<ExternalReference>(record).is_err());
        assert!(
            serde_json::from_value::<ExternalCatalog>(serde_json::json!({
                "schema_version": 1, "format": FORMAT, "metadata_only": false,
                "rights_status": STATUS, "gameplay_verified": false,
                "archive_included": false, "reference_files": 1, "platforms": []
            }))
            .is_ok()
        );
    }
}
