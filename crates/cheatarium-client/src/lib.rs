//! Local, read-only consumer for Cheatarium's versioned console cheat indexes.
//!
//! No network access, ROM reads, cartridge mutation, or code execution.
//! All name matching is *advisory* and must not auto-enable cheats.
pub mod identity;
pub mod inventory;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const MAX_CATALOG_SIZE: u64 = 5 * 1024 * 1024;
const MAX_UNCOMPRESSED_BUNDLE: u64 = 512 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub struct Catalog {
    pub schema_version: u32,
    pub format: String,
    pub matching_policy: String,
    pub bundles: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CatalogEntry {
    pub platform: String,
    pub artifact: String,
    #[serde(default)]
    pub game_index_artifact: Option<String>,
    #[serde(default)]
    pub repeat_index_artifact: Option<String>,
    #[serde(default)]
    pub repeat_groups: Option<usize>,
    #[serde(default)]
    pub tag_index_artifact: Option<String>,
    #[serde(default)]
    pub tag_matches: Option<usize>,
    #[serde(default)]
    pub identity_artifact: Option<String>,
    #[serde(default)]
    pub game_candidate_groups: Option<usize>,
    pub source_files: usize,
    pub code_fields: usize,
    #[serde(default)]
    pub native_memory_entries: usize,
    #[serde(default)]
    pub decoded_snes_code_fields: usize,
    pub warnings: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Bundle {
    pub schema_version: u32,
    pub platform: String,
    pub game_identity_rule: String,
    pub compatibility_rule: String,
    pub records: Vec<IndexedFile>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndexedFile {
    pub id: String,
    pub title_hint: String,
    pub candidate_game_key: String,
    pub identity_confidence: String,
    pub raw_filename: String,
    pub region_hint: Option<String>,
    pub format_hint: Option<String>,
    pub declared_cheats: Option<usize>,
    pub parse_warnings: Vec<String>,
    pub codes: Vec<Code>,
    pub provenance: Provenance,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Code {
    pub ordinal: usize,
    pub description: Option<String>,
    pub code: Option<String>,
    pub source_enabled: bool,
    pub verification: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub native_fields: Vec<NativeField>,
    #[serde(default)]
    pub snes_decode: Option<SnesDecoded>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SnesDecoded {
    pub format: String,
    pub address_space: String,
    pub compatibility: String,
    #[serde(default)]
    pub interpretation_basis: Option<String>,
    pub writes: Vec<SnesWrite>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SnesWrite {
    pub address_hex: String,
    pub value_hex: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NativeField {
    pub name: String,
    pub value: String,
}

impl Code {
    /// A native address/value cheat remains separate from encoded device codes.
    #[must_use]
    pub fn is_memory_entry(&self) -> bool {
        self.role.as_deref() == Some("memory-entry")
    }

    /// Description-only headings are not activatable codes.
    #[must_use]
    pub fn is_code(&self) -> bool {
        self.code.as_deref().is_some_and(|s| !s.trim().is_empty())
            && self.role.as_deref() != Some("section-heading")
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Provenance {
    pub source_id: String,
    pub repository: String,
    pub revision: String,
    pub license: String,
    pub upstream_path: String,
    pub archive_path: String,
    pub git_blob_sha: String,
}

fn check_schema(version: u32) -> Result<()> {
    if version != 1 {
        return Err(format!("Unsupported Cheatarium schema version {version}").into());
    }
    Ok(())
}

pub fn load_catalog(root: impl AsRef<Path>) -> Result<Catalog> {
    let path = root.as_ref().join("catalog.json");
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_CATALOG_SIZE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CATALOG_SIZE {
        return Err("Cheatarium catalog exceeds size limit".into());
    }
    let catalog: Catalog = serde_json::from_slice(&bytes)?;
    check_schema(catalog.schema_version)?;
    if catalog.format != "cheatarium-index-v1" {
        return Err("Unsupported Cheatarium index format".into());
    }
    Ok(catalog)
}

/// Decode a compressed bundle with an explicit uncompressed size limit.
pub fn decode_bundle(reader: impl Read) -> Result<Bundle> {
    let mut gzip = GzDecoder::new(reader).take(MAX_UNCOMPRESSED_BUNDLE + 1);
    let mut data = Vec::new();
    gzip.read_to_end(&mut data)?;
    if data.len() as u64 > MAX_UNCOMPRESSED_BUNDLE {
        return Err("Cheatarium bundle exceeds size limit".into());
    }
    let bundle: Bundle = serde_json::from_slice(&data)?;
    check_schema(bundle.schema_version)?;
    Ok(bundle)
}

/// Load one platform's local, compressed index.
/// The catalog cannot direct a consumer outside its own directory.
pub fn load_platform(root: impl AsRef<Path>, platform: &str) -> Result<Bundle> {
    let root = root.as_ref();
    let catalog = load_catalog(root)?;
    let entry = catalog
        .bundles
        .into_iter()
        .find(|item| item.platform == platform)
        .ok_or_else(|| format!("No Cheatarium index for console {platform}"))?;
    if entry.artifact != format!("{platform}.json.gz")
        || platform.is_empty()
        || !platform
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err("Unsafe or unexpected Cheatarium index path".into());
    }
    let bundle = decode_bundle(File::open(root.join(entry.artifact))?)?;
    if bundle.platform != platform || bundle.records.len() != entry.source_files {
        return Err("Cheatarium catalog/bundle mismatch".into());
    }
    Ok(bundle)
}

#[derive(Debug, Deserialize)]
pub struct DistributionManifest {
    pub schema_version: u32,
    pub format: String,
    pub files: Vec<ArtifactDigest>,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactDigest {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

fn sha256_reader(mut reader: impl Read) -> Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        bytes = bytes
            .checked_add(n as u64)
            .ok_or("Artifact size overflow")?;
    }
    Ok((format!("{:x}", hasher.finalize()), bytes))
}

/// Checks catalog and requested console bundles against the local v1 SHA-256
/// manifest. Does not authenticate the manifest itself: pin a trusted commit
/// or separately verify the origin of distribution.json.
pub fn verify_platform_distribution(root: impl AsRef<Path>, platform: &str) -> Result<()> {
    let root = root.as_ref();
    let catalog = load_catalog(root)?;
    let entry = catalog
        .bundles
        .iter()
        .find(|item| item.platform == platform)
        .ok_or_else(|| format!("No Cheatarium index for console {platform}"))?;
    if platform.is_empty()
        || !platform
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        || entry.artifact != format!("{platform}.json.gz")
    {
        return Err("Unsafe Cheatarium artifact path".into());
    }
    let mut artifacts = vec!["catalog.json".to_owned(), entry.artifact.clone()];
    if let Some(game_path) = &entry.game_index_artifact {
        if game_path != &format!("games/{platform}.json.gz") {
            return Err("Unsafe Cheatarium game-index path".into());
        }
        artifacts.push(game_path.clone());
    }
    if let Some(repeat_path) = &entry.repeat_index_artifact {
        if repeat_path != &format!("repeats/{platform}.json.gz") {
            return Err("Unsafe Cheatarium repetition-index path".into());
        }
        artifacts.push(repeat_path.clone());
    }
    if let Some(tag_path) = &entry.tag_index_artifact {
        if tag_path != &format!("tags/{platform}.json.gz") {
            return Err("Unsafe Cheatarium effect-tag index path".into());
        }
        artifacts.push(tag_path.clone());
        artifacts.push("taxonomy/effects-v1.json".to_owned());
    }
    if let Some(identity_path) = &entry.identity_artifact {
        if identity_path != &format!("identities/{platform}.json") {
            return Err("Unsafe Cheatarium ROM identity artifact path".into());
        }
        artifacts.push(identity_path.clone());
    }
    let mut content = Vec::new();
    File::open(root.join("distribution.json"))?
        .take(MAX_CATALOG_SIZE + 1)
        .read_to_end(&mut content)?;
    if content.len() as u64 > MAX_CATALOG_SIZE {
        return Err("Cheatarium distribution manifest exceeds size limit".into());
    }
    let manifest: DistributionManifest = serde_json::from_slice(&content)?;
    check_schema(manifest.schema_version)?;
    if manifest.format != "cheatarium-distribution-v1" {
        return Err("Unsupported Cheatarium distribution manifest".into());
    }
    for name in artifacts {
        let mut references = manifest.files.iter().filter(|file| file.path == name);
        let expected = references
            .next()
            .ok_or_else(|| format!("Unlisted Cheatarium artifact: {name}"))?;
        if references.next().is_some() {
            return Err(format!("Duplicate manifest artifact: {name}").into());
        }
        if expected.sha256.len() != 64 || !expected.sha256.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Malformed SHA-256 checksum for {name}").into());
        }
        let (actual_hash, actual_length) = sha256_reader(File::open(root.join(&name))?)?;
        if actual_hash != expected.sha256 || actual_length != expected.size_bytes {
            return Err(format!("Cheatarium checksum/length mismatch for {name}").into());
        }
    }
    Ok(())
}

/// Advisory grouping of source occurrences under filename-derived titles.
/// Neither an edition identifier nor a verified ROM/serial match.
#[derive(Debug, Deserialize, Serialize)]
pub struct GameIndex {
    pub schema_version: u32,
    pub platform: String,
    pub identity_rule: String,
    pub candidates: Vec<GameCandidate>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GameCandidate {
    pub key: String,
    pub title_hint: String,
    pub alternate_title_hints: Vec<String>,
    pub identity_confidence: String,
    pub possible_title_collision: bool,
    pub source_record_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub region_hints: Vec<String>,
    pub format_hints: Vec<String>,
    pub code_fields: usize,
    pub native_memory_entries: usize,
}

pub fn decode_game_index(reader: impl Read) -> Result<GameIndex> {
    let mut gzip = GzDecoder::new(reader).take(MAX_UNCOMPRESSED_BUNDLE + 1);
    let mut data = Vec::new();
    gzip.read_to_end(&mut data)?;
    if data.len() as u64 > MAX_UNCOMPRESSED_BUNDLE {
        return Err("Cheatarium game index exceeds size limit".into());
    }
    let index: GameIndex = serde_json::from_slice(&data)?;
    check_schema(index.schema_version)?;
    Ok(index)
}

/// Load the optional, additive v1 filename-group index for one platform.
pub fn load_game_candidates(root: impl AsRef<Path>, platform: &str) -> Result<GameIndex> {
    let root = root.as_ref();
    let catalog = load_catalog(root)?;
    let entry = catalog
        .bundles
        .into_iter()
        .find(|entry| entry.platform == platform)
        .ok_or_else(|| format!("No Cheatarium index for console {platform}"))?;
    let expected = format!("games/{platform}.json.gz");
    if platform.is_empty()
        || !platform
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        || entry.game_index_artifact.as_deref() != Some(expected.as_str())
    {
        return Err("No safe game-candidate index available".into());
    }
    let index = decode_game_index(File::open(root.join(expected))?)?;
    if index.platform != platform || Some(index.candidates.len()) != entry.game_candidate_groups {
        return Err("Cheatarium game catalog/bundle mismatch".into());
    }
    Ok(index)
}

impl GameIndex {
    /// Case-insensitive candidate title matching. Never a verified ROM match.
    #[must_use]
    pub fn search_title(&self, needle: &str) -> Vec<&GameCandidate> {
        let needle = needle.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        self.candidates
            .iter()
            .filter(|game| {
                game.title_hint.to_lowercase().contains(&needle)
                    || game
                        .alternate_title_hints
                        .iter()
                        .any(|title| title.to_lowercase().contains(&needle))
            })
            .collect()
    }

    #[must_use]
    pub fn by_candidate_key(&self, key: &str) -> Option<&GameCandidate> {
        self.candidates.iter().find(|game| game.key == key)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepeatedCodeIndex {
    pub schema_version: u32,
    pub platform: String,
    pub interpretation: String,
    pub groups: Vec<RepeatedCode>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepeatedCode {
    pub candidate_game_key: String,
    pub region_hint: Option<String>,
    pub revision_hint: Option<String>,
    pub declared_format: Option<String>,
    pub source_code: String,
    pub relation: String,
    pub confirmed_equivalent_cheat: bool,
    pub verified_rom_compatibility: bool,
    pub description_variants: usize,
    pub description_text_varies: bool,
    pub occurrences: Vec<RepeatOccurrence>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepeatOccurrence {
    pub source_record_id: String,
    pub ordinal: usize,
    pub description: Option<String>,
}

/// Read additive advisory repetition index. This does not establish a
/// verified game identity, equivalence of effects, or code compatibility.
pub fn load_repeated_codes(root: impl AsRef<Path>, platform: &str) -> Result<RepeatedCodeIndex> {
    let root = root.as_ref();
    let catalog = load_catalog(root)?;
    let entry = catalog
        .bundles
        .iter()
        .find(|entry| entry.platform == platform)
        .ok_or_else(|| format!("No Cheatarium index for console {platform}"))?;
    let expected = format!("repeats/{platform}.json.gz");
    if platform.is_empty()
        || !platform
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        || entry.repeat_index_artifact.as_deref() != Some(expected.as_str())
    {
        return Err("No safe repetition index available".into());
    }
    let mut gzip =
        GzDecoder::new(File::open(root.join(expected))?).take(MAX_UNCOMPRESSED_BUNDLE + 1);
    let mut data = Vec::new();
    gzip.read_to_end(&mut data)?;
    if data.len() as u64 > MAX_UNCOMPRESSED_BUNDLE {
        return Err("Cheatarium repetition index exceeds size limit".into());
    }
    let index: RepeatedCodeIndex = serde_json::from_slice(&data)?;
    check_schema(index.schema_version)?;
    if index.platform != platform || Some(index.groups.len()) != entry.repeat_groups {
        return Err("Cheatarium repeated-code index catalog mismatch".into());
    }
    if index.groups.iter().any(|group| {
        group.confirmed_equivalent_cheat
            || group.verified_rom_compatibility
            || group.description_variants
                != group
                    .occurrences
                    .iter()
                    .filter_map(|o| o.description.as_deref())
                    .filter(|s| !s.trim().is_empty())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
            || group.description_text_varies != (group.description_variants > 1)
            || group.relation != "identical-raw-code-text-within-advisory-filename-bucket"
            || group
                .occurrences
                .iter()
                .map(|c| &c.source_record_id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                < 2
    }) {
        return Err("Unsafe or malformed code-repetition interpretation".into());
    }
    Ok(index)
}

impl RepeatedCodeIndex {
    #[must_use]
    pub fn by_candidate_game_key(&self, key: &str) -> Vec<&RepeatedCode> {
        if key.is_empty() {
            return Vec::new();
        }
        self.groups
            .iter()
            .filter(|group| group.candidate_game_key == key)
            .collect()
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub struct EffectTagIndex {
    pub schema_version: u32,
    pub platform: String,
    pub taxonomy_id: String,
    pub interpretation: String,
    pub categories: Vec<TagCategory>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagCategory {
    pub id: String,
    pub matches: Vec<TagOccurrence>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagOccurrence {
    pub source_record_id: String,
    pub ordinal: usize,
    pub candidate_game_key: String,
    pub matched_phrase: String,
}

/// Read-only lexical signal index. The original source descriptions live in
/// the platform bundle; no signal is a verified gameplay-effect assertion.
pub fn load_effect_tags(root: impl AsRef<Path>, platform: &str) -> Result<EffectTagIndex> {
    let root = root.as_ref();
    let catalog = load_catalog(root)?;
    let entry = catalog.bundles.iter()
        .find(|entry| entry.platform == platform)
        .ok_or_else(|| format!("No Cheatarium index for console {platform}"))?;
    let expected = format!("tags/{platform}.json.gz");
    if platform.is_empty()
        || !platform.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        || entry.tag_index_artifact.as_deref() != Some(expected.as_str())
    {
        return Err("No safe effect-tag index available".into());
    }
    let mut gzip = GzDecoder::new(File::open(root.join(expected))?)
        .take(MAX_UNCOMPRESSED_BUNDLE + 1);
    let mut data = Vec::new();
    gzip.read_to_end(&mut data)?;
    if data.len() as u64 > MAX_UNCOMPRESSED_BUNDLE {
        return Err("Cheatarium effect-tag index exceeds size limit".into());
    }
    let index: EffectTagIndex = serde_json::from_slice(&data)?;
    check_schema(index.schema_version)?;
    if index.platform != platform
        || index.taxonomy_id != "cheatarium-effect-signals-en-v1"
        || index.interpretation != "lexical-source-description-signal-only; no verified game effect or cartridge compatibility"
        || Some(index.categories.iter().map(|c| c.matches.len()).sum::<usize>()) != entry.tag_matches
    {
        return Err("Cheatarium lexical tag catalog mismatch".into());
    }
    let mut seen_categories = std::collections::BTreeSet::new();
    if index.categories.iter().any(|category| {
        !seen_categories.insert(category.id.as_str())
            || category.matches.is_empty()
            || category.matches.iter().any(|hit| hit.source_record_id.is_empty() || hit.matched_phrase.is_empty())
    }) {
        return Err("Malformed Cheatarium lexical tag entries".into());
    }
    Ok(index)
}

impl EffectTagIndex {
    #[must_use]
    pub fn by_category(&self, id: &str) -> Option<&TagCategory> {
        self.categories.iter().find(|category| category.id == id)
    }
}
/// A non-executing cheat-description search result, with complete provenance.
#[derive(Debug, Serialize)]
pub struct EffectHit<'a> {
    pub source_record_id: &'a str,
    pub title_hint: &'a str,
    pub raw_filename: &'a str,
    pub region_hint: Option<&'a str>,
    pub format_hint: Option<&'a str>,
    pub cheat: &'a Code,
    pub provenance: &'a Provenance,
}

impl Bundle {
    /// Case-insensitive, *candidate-only* title lookup; no ROM identity check.
    #[must_use]
    pub fn search_title(&self, needle: &str) -> Vec<&IndexedFile> {
        let needle = needle.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        self.records
            .iter()
            .filter(|r| r.title_hint.to_lowercase().contains(&needle))
            .collect()
    }

    /// Search indexed gameplay-effect descriptions across one console.
    /// No title identity is established and no cheat is activated. Non-executable
    /// section headings are excluded from results.
    #[must_use]
    pub fn search_effect(&self, needle: &str) -> Vec<EffectHit<'_>> {
        let needle = needle.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for record in &self.records {
            for cheat in &record.codes {
                if !(cheat.is_code() || cheat.is_memory_entry()) {
                    continue;
                }
                if !cheat
                    .description
                    .as_deref()
                    .is_some_and(|description| description.to_lowercase().contains(&needle))
                {
                    continue;
                }
                hits.push(EffectHit {
                    source_record_id: &record.id,
                    title_hint: &record.title_hint,
                    raw_filename: &record.raw_filename,
                    region_hint: record.region_hint.as_deref(),
                    format_hint: record.format_hint.as_deref(),
                    cheat,
                    provenance: &record.provenance,
                });
            }
        }
        hits
    }

    /// Restrict gameplay-effect results to provenance-declared device format
    /// and exact source identity. Anonymous code-syntax interpretations do NOT
    /// pass a declared-format filter.
    #[must_use]
    pub fn search_effect_filtered(
        &self,
        needle: &str,
        declared_format: Option<&str>,
        source_id: Option<&str>,
    ) -> Vec<EffectHit<'_>> {
        self.search_effect(needle)
            .into_iter()
            .filter(|hit| {
                declared_format.is_none_or(|format| hit.format_hint == Some(format))
                    && source_id.is_none_or(|id| hit.provenance.source_id == id)
            })
            .collect()
    }

    /// Exact candidate-title key lookup; still not a cartridge identity test.
    #[must_use]
    pub fn by_candidate_game_key(&self, key: &str) -> Vec<&IndexedFile> {
        self.records
            .iter()
            .filter(|r| r.candidate_game_key == key)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, GzBuilder};
    use std::io::Write;

    fn fixture() -> Vec<u8> {
        let payload = r#"{"schema_version":1,"platform":"snes","game_identity_rule":"filename only","compatibility_rule":"manual confirmation","records":[{"id":"x","title_hint":"Super Mario World","candidate_game_key":"super-mario-world","identity_confidence":"filename_heuristic_only","raw_filename":"Super Mario World (USA).cht","region_hint":"USA","format_hint":null,"declared_cheats":1,"parse_warnings":[],"codes":[{"ordinal":0,"description":"Infinite Lives","code":"ABCD","source_enabled":false,"verification":"unverified","role":"code"},{"ordinal":1,"description":"A heading","code":null,"source_enabled":false,"verification":"unverified","role":"section-heading"}],"provenance":{"source_id":"libretro","repository":"https://example.com","revision":"abc","license":"CC-BY-SA-4.0","upstream_path":"cht/sample.cht","archive_path":"archive/sample.cht","git_blob_sha":"abcdef"}}]}"#;
        let mut gz = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::fast());
        gz.write_all(payload.as_bytes()).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn reads_bundle_and_never_activates_headings() {
        let bundle = decode_bundle(fixture().as_slice()).unwrap();
        assert_eq!(bundle.search_title("MARIO").len(), 1);
        assert_eq!(bundle.by_candidate_game_key("super-mario-world").len(), 1);
        assert_eq!(bundle.records[0].codes[0].code.as_deref(), Some("ABCD"));
        assert!(bundle.records[0].codes[0].is_code());
        assert!(!bundle.records[0].codes[1].is_code());
    }

    #[test]
    fn effect_search_preserves_source_and_excludes_nonexecuting_headings() {
        let bundle = decode_bundle(fixture().as_slice()).unwrap();
        let hits = bundle.search_effect("INFINITE");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source_record_id, "x");
        assert_eq!(hits[0].title_hint, "Super Mario World");
        assert_eq!(hits[0].cheat.ordinal, 0);
        assert_eq!(hits[0].cheat.code.as_deref(), Some("ABCD"));
        assert_eq!(hits[0].provenance.upstream_path, "cht/sample.cht");
        assert!(bundle.search_effect("A heading").is_empty());
        assert!(bundle.search_effect("").is_empty());
    }

    #[test]
    fn source_and_declared_format_filters_never_infer_device_provenance() {
        let bundle = decode_bundle(fixture().as_slice()).unwrap();
        assert_eq!(
            bundle
                .search_effect_filtered("Infinite", None, Some("libretro"))
                .len(),
            1
        );
        assert!(bundle
            .search_effect_filtered("Infinite", None, Some("other"))
            .is_empty());
        // The fixture has an unlabeled device format. Its code must not
        // become Game Genie through text search or an anonymous decoder.
        assert!(bundle
            .search_effect_filtered("Infinite", Some("game-genie"), None)
            .is_empty());
        assert!(bundle.search_effect_filtered("", None, None).is_empty());
    }

    #[test]
    fn game_candidate_lookup_preserves_all_source_links() {
        let payload = serde_json::json!({
            "schema_version": 1,
            "platform": "snes",
            "identity_rule": "unverified filename grouping",
            "candidates": [{
                "key": "super-mario-world",
                "title_hint": "Super Mario World",
                "alternate_title_hints": ["Super-Mario World"],
                "identity_confidence": "filename_candidate_only",
                "possible_title_collision": true,
                "source_record_ids": ["source:a", "source:b"],
                "source_ids": ["source"],
                "region_hints": ["USA"],
                "format_hints": ["game-genie"],
                "code_fields": 2,
                "native_memory_entries": 0
            }]
        });
        let mut gz = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::fast());
        gz.write_all(&serde_json::to_vec(&payload).unwrap())
            .unwrap();
        let index = decode_game_index(gz.finish().unwrap().as_slice()).unwrap();
        assert_eq!(index.search_title("MARIO").len(), 1);
        let group = index.by_candidate_key("super-mario-world").unwrap();
        assert_eq!(group.source_record_ids.len(), 2);
        assert!(group.possible_title_collision);
    }

    #[test]
    fn repetition_index_is_advisory_and_exactly_queryable() {
        let index = RepeatedCodeIndex {
            schema_version: 1,
            platform: "snes".to_owned(),
            interpretation: "source-text-only".to_owned(),
            groups: vec![RepeatedCode {
                candidate_game_key: "super-mario-world".to_owned(),
                region_hint: Some("USA".to_owned()),
                revision_hint: None,
                declared_format: None,
                source_code: "DDB4-6F07".to_owned(),
                relation: "identical-raw-code-text-within-advisory-filename-bucket".to_owned(),
                confirmed_equivalent_cheat: false,
                verified_rom_compatibility: false,
                description_variants: 0,
                description_text_varies: false,
                occurrences: vec![
                    RepeatOccurrence {
                        source_record_id: "a".to_owned(),
                        ordinal: 0,
                        description: None,
                    },
                    RepeatOccurrence {
                        source_record_id: "b".to_owned(),
                        ordinal: 1,
                        description: None,
                    },
                ],
            }],
        };
        assert_eq!(index.by_candidate_game_key("super-mario-world").len(), 1);
        assert!(index.by_candidate_game_key("other").is_empty());
        assert!(index.by_candidate_game_key("").is_empty());
        assert!(!index.groups[0].confirmed_equivalent_cheat);
    }

    #[test]
    fn signal_index_has_no_executable_or_verified_effect_claims() {
        let index = EffectTagIndex {
            schema_version: 1,
            platform: "snes".to_owned(),
            taxonomy_id: "cheatarium-effect-signals-en-v1".to_owned(),
            interpretation: "lexical-source-description-signal-only; no verified game effect or cartridge compatibility".to_owned(),
            categories: vec![TagCategory {
                id: "lives".to_owned(),
                matches: vec![TagOccurrence {
                    source_record_id: "source:a".to_owned(),
                    ordinal: 0,
                    candidate_game_key: "super-mario-world".to_owned(),
                    matched_phrase: "infinite lives".to_owned(),
                }],
            }],
        };
        let hit = &index.by_category("lives").unwrap().matches[0];
        assert_eq!(hit.source_record_id, "source:a");
        assert!(index.by_category("health").is_none());
        assert!(index.interpretation.contains("no verified"));
    }

    #[test]
    fn checksum_reader_matches_sha256_test_vector() {
        let (hash, length) = sha256_reader("abc".as_bytes()).unwrap();
        assert_eq!(length, 3);
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn rejects_unknown_schema() {
        let mut bundle = decode_bundle(fixture().as_slice()).unwrap();
        bundle.schema_version = 2;
        let payload = serde_json::to_vec(&bundle).unwrap();
        let mut gz = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::fast());
        gz.write_all(&payload).unwrap();
        assert!(decode_bundle(gz.finish().unwrap().as_slice()).is_err());
    }
}
