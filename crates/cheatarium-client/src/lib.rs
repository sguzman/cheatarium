//! Local, read-only consumer for Cheatarium's versioned console cheat indexes.
//!
//! No network access, ROM reads, cartridge mutation, or code execution.
//! All name matching is *advisory* and must not auto-enable cheats.
pub mod identity;
pub mod inventory;
pub mod publications;
pub mod reviews;
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
    #[serde(default)]
    pub composition: Option<SourceComposition>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SourceComposition {
    pub relation: String,
    pub alternatives: Vec<Vec<String>>,
    pub evidence: Vec<CompositionEvidence>,
    pub rom_match_verified: bool,
    pub simultaneous_execution_confirmed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompositionEvidence {
    pub url: String,
    pub reference: String,
    pub source_revision: String,
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
    #[must_use]
    pub fn requires_composition_review(&self) -> bool {
        self.composition
            .as_ref()
            .is_some_and(|x| x.relation == "unresolved" || x.relation == "revision-alternatives")
    }

    /// A source '+' character is NOT a declaration of an executable combination.
    /// No version-alternative record is safe to flatten into combined writes.
    #[must_use]
    pub fn may_consumer_treat_as_verified_combination(&self) -> bool {
        false
    }

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
    let mut artifacts = vec![
        "catalog.json".to_owned(),
        "reviews.json".to_owned(),
        entry.artifact.clone(),
    ];
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
    if platform == "snes" {
        artifacts.push("interpretations/snes.json".to_owned());
        artifacts.push("interpretations/snes-published-groups.json".to_owned());
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

    /// Resolve an exact advisory key; a collision in the *index* is invalid,
    /// distinct from the possible-title-collision warning on a single group.
    pub fn find_candidate(&self, key: &str) -> Result<&GameCandidate> {
        if key.is_empty() {
            return Err("Candidate game key must not be empty".into());
        }
        let mut matches = self.candidates.iter().filter(|game| game.key == key);
        let candidate = matches.next().ok_or("Candidate game key not found")?;
        if matches.next().is_some() {
            return Err("Duplicate game candidate keys in index".into());
        }
        Ok(candidate)
    }
}

/// One unchanged original cheat entry within a filename-derived game group.
/// This is not evidence of executable compatibility or identical effects.
#[derive(Debug, Serialize)]
pub struct GameEntryHit<'a> {
    pub source_record_id: &'a str,
    pub source_title_hint: &'a str,
    pub raw_filename: &'a str,
    pub region_hint: Option<&'a str>,
    pub declared_format_hint: Option<&'a str>,
    pub entry: &'a Code,
    pub provenance: &'a Provenance,
}

impl Bundle {
    /// Follow exactly the original source links in a candidate game group.
    /// Both the candidate and the source bundle are required: filename
    /// grouping is advisory and never validates cartridge/ROM identity.
    pub fn sources_for_candidate<'a>(
        &'a self,
        candidate: &GameCandidate,
    ) -> Result<Vec<&'a IndexedFile>> {
        use std::collections::{BTreeMap, BTreeSet};

        let by_id: BTreeMap<_, _> = self
            .records
            .iter()
            .map(|source| (source.id.as_str(), source))
            .collect();
        if by_id.len() != self.records.len() {
            return Err("Duplicate original source IDs in platform bundle".into());
        }
        let mut seen = BTreeSet::new();
        let mut original_archives = BTreeSet::new();
        let mut original_regions = BTreeSet::new();
        let mut original_formats = BTreeSet::new();
        let mut selected = Vec::with_capacity(candidate.source_record_ids.len());
        let mut code_fields = 0usize;
        let mut memory_entries = 0usize;
        if candidate.source_record_ids.is_empty() {
            return Err("Candidate game has no original source links".into());
        }
        for source_id in &candidate.source_record_ids {
            if !seen.insert(source_id.as_str()) {
                return Err("Duplicate original source link in candidate game".into());
            }
            let source = by_id
                .get(source_id.as_str())
                .ok_or("Missing linked original source record")?;
            let derived_key = if source.candidate_game_key.is_empty() {
                format!("unresolved:{}", source.id)
            } else {
                source.candidate_game_key.clone()
            };
            if derived_key != candidate.key {
                return Err("Linked source has another candidate game key".into());
            }
            original_archives.insert(source.provenance.source_id.as_str());
            if let Some(region) = source.region_hint.as_deref() {
                original_regions.insert(region);
            }
            if let Some(format) = source.format_hint.as_deref() {
                original_formats.insert(format);
            }
            code_fields = code_fields
                .checked_add(
                    source
                        .codes
                        .iter()
                        .filter(|code| code.role.as_deref() == Some("code"))
                        .count(),
                )
                .ok_or("Candidate source code count overflow")?;
            memory_entries = memory_entries
                .checked_add(
                    source
                        .codes
                        .iter()
                        .filter(|code| code.role.as_deref() == Some("memory-entry"))
                        .count(),
                )
                .ok_or("Candidate memory entry count overflow")?;
            selected.push(*source);
        }
        let declared_archives: BTreeSet<_> =
            candidate.source_ids.iter().map(String::as_str).collect();
        let declared_regions: BTreeSet<_> =
            candidate.region_hints.iter().map(String::as_str).collect();
        let declared_formats: BTreeSet<_> =
            candidate.format_hints.iter().map(String::as_str).collect();
        if declared_archives.len() != candidate.source_ids.len()
            || declared_regions.len() != candidate.region_hints.len()
            || declared_formats.len() != candidate.format_hints.len()
            || declared_archives != original_archives
            || declared_regions != original_regions
            || declared_formats != original_formats
        {
            return Err("Candidate game source/region/device hints disagree with originals".into());
        }
        if code_fields != candidate.code_fields || memory_entries != candidate.native_memory_entries
        {
            return Err("Candidate game source totals disagree with original source bundle".into());
        }
        Ok(selected)
    }

    /// Browse original source entries across an advisory game candidate.
    /// Preserves duplicate text, original source ordinals, section headings and
    /// imported enable bits as *data*, never activation instructions.
    pub fn entries_for_candidate<'a>(
        &'a self,
        candidate: &GameCandidate,
        source_record_id: Option<&str>,
        role: Option<&str>,
    ) -> Result<Vec<GameEntryHit<'a>>> {
        self.search_entries_for_candidate(candidate, source_record_id, role, None)
    }

    /// Case-insensitive *description text* lookup within an advisory game
    /// grouping. Matching a label is not verification of the gameplay effect;
    /// no original description, code, or source identity is rewritten.
    pub fn search_entries_for_candidate<'a>(
        &'a self,
        candidate: &GameCandidate,
        source_record_id: Option<&str>,
        role: Option<&str>,
        description_contains: Option<&str>,
    ) -> Result<Vec<GameEntryHit<'a>>> {
        let needle = description_contains.map(str::trim);
        if needle == Some("") {
            return Err("Description search text must not be blank".into());
        }
        let needle = needle.map(str::to_lowercase);
        if role.is_some_and(|name| !matches!(name, "code" | "memory-entry" | "section-heading")) {
            return Err("Unsupported original cheat entry role".into());
        }
        if source_record_id == Some("") {
            return Err("Original source record filter cannot be empty".into());
        }
        let sources = self.sources_for_candidate(candidate)?;
        let mut source_found = source_record_id.is_none();
        let mut hits = Vec::new();
        for source in sources {
            if let Some(filter) = source_record_id {
                if filter != source.id {
                    continue;
                }
            }
            source_found = true;
            for code in &source.codes {
                if let Some(role_filter) = role {
                    if code.role.as_deref() != Some(role_filter) {
                        continue;
                    }
                }
                if let Some(search_text) = needle.as_deref() {
                    if !code
                        .description
                        .as_deref()
                        .is_some_and(|description| description.to_lowercase().contains(search_text))
                    {
                        continue;
                    }
                }
                hits.push(GameEntryHit {
                    source_record_id: &source.id,
                    source_title_hint: &source.title_hint,
                    raw_filename: &source.raw_filename,
                    region_hint: source.region_hint.as_deref(),
                    declared_format_hint: source.format_hint.as_deref(),
                    entry: code,
                    provenance: &source.provenance,
                });
            }
        }
        if !source_found {
            return Err("Original source record is not linked to candidate game".into());
        }
        Ok(hits)
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
    let entry = catalog
        .bundles
        .iter()
        .find(|entry| entry.platform == platform)
        .ok_or_else(|| format!("No Cheatarium index for console {platform}"))?;
    let expected = format!("tags/{platform}.json.gz");
    if platform.is_empty()
        || !platform
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        || entry.tag_index_artifact.as_deref() != Some(expected.as_str())
    {
        return Err("No safe effect-tag index available".into());
    }
    let mut gzip =
        GzDecoder::new(File::open(root.join(expected))?).take(MAX_UNCOMPRESSED_BUNDLE + 1);
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
            || category
                .matches
                .iter()
                .any(|hit| hit.source_record_id.is_empty() || hit.matched_phrase.is_empty())
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

/// Literal original code-text occurrence, not a deduplicated or verified cheat.
#[derive(Debug, Serialize)]
pub struct ExactCodeHit<'a> {
    pub source_record_id: &'a str,
    pub candidate_game_key: &'a str,
    pub title_hint: &'a str,
    pub raw_filename: &'a str,
    pub region_hint: Option<&'a str>,
    pub format_hint: Option<&'a str>,
    pub cheat: &'a Code,
    pub provenance: &'a Provenance,
}

impl Bundle {
    /// Resolve one exact original source ID. A duplicate source ID is treated
    /// as malformed index data rather than silently choosing an occurrence.
    pub fn find_source_record(&self, source_record_id: &str) -> Result<&IndexedFile> {
        if source_record_id.is_empty() {
            return Err("Original source record ID must not be empty".into());
        }
        let mut matches = self
            .records
            .iter()
            .filter(|record| record.id == source_record_id);
        let first = matches.next().ok_or("Original source record not found")?;
        if matches.next().is_some() {
            return Err("Duplicate original source record IDs in platform index".into());
        }
        Ok(first)
    }

    /// Match raw device code text *exactly*, including case and whitespace.
    /// A repeated string is not proof of identical effects or ROM compatibility.
    /// Each occurrence retains its original source and ordinal. Never executes.
    #[must_use]
    pub fn search_exact_code(&self, raw_code: &str) -> Vec<ExactCodeHit<'_>> {
        if raw_code.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for record in &self.records {
            for cheat in &record.codes {
                if cheat.role.as_deref() != Some("code") || cheat.code.as_deref() != Some(raw_code)
                {
                    continue;
                }
                hits.push(ExactCodeHit {
                    source_record_id: &record.id,
                    candidate_game_key: &record.candidate_game_key,
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
    fn game_candidate_source_links_preserve_exact_provenance() {
        let bundle = decode_bundle(fixture().as_slice()).unwrap();
        let candidate = GameCandidate {
            key: "super-mario-world".to_owned(),
            title_hint: "Super Mario World".to_owned(),
            alternate_title_hints: vec![],
            identity_confidence: "filename_candidate_only".to_owned(),
            possible_title_collision: false,
            source_record_ids: vec!["x".to_owned()],
            source_ids: vec!["libretro".to_owned()],
            region_hints: vec!["USA".to_owned()],
            format_hints: vec![],
            code_fields: 1,
            native_memory_entries: 0,
        };
        let sources = bundle.sources_for_candidate(&candidate).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "x");
        assert_eq!(sources[0].provenance.upstream_path, "cht/sample.cht");
        let mut broken = serde_json::to_value(&candidate).unwrap();

        broken["source_record_ids"] = serde_json::json!(["x", "x"]);
        assert!(bundle
            .sources_for_candidate(&serde_json::from_value(broken.clone()).unwrap())
            .is_err());
        broken["source_record_ids"] = serde_json::json!(["missing"]);
        assert!(bundle
            .sources_for_candidate(&serde_json::from_value(broken.clone()).unwrap())
            .is_err());
        broken["source_record_ids"] = serde_json::json!(["x"]);
        broken["code_fields"] = serde_json::json!(2);
        assert!(bundle
            .sources_for_candidate(&serde_json::from_value(broken.clone()).unwrap())
            .is_err());
        broken["code_fields"] = serde_json::json!(1);
        broken["source_ids"] = serde_json::json!(["unknown-archive"]);
        assert!(bundle
            .sources_for_candidate(&serde_json::from_value(broken.clone()).unwrap())
            .is_err());
        broken["source_ids"] = serde_json::json!(["libretro"]);
        broken["region_hints"] = serde_json::json!(["Europe"]);
        assert!(bundle
            .sources_for_candidate(&serde_json::from_value(broken.clone()).unwrap())
            .is_err());
        broken["region_hints"] = serde_json::json!(["USA"]);
        broken["format_hints"] = serde_json::json!(["game-genie"]);
        assert!(bundle
            .sources_for_candidate(&serde_json::from_value(broken.clone()).unwrap())
            .is_err());
        broken["format_hints"] = serde_json::json!([]);
        broken["key"] = serde_json::json!("another-game");
        assert!(bundle
            .sources_for_candidate(&serde_json::from_value(broken).unwrap())
            .is_err());
    }

    #[test]
    fn candidate_entry_browsing_preserves_original_ordinals_and_roles() {
        let bundle = decode_bundle(fixture().as_slice()).unwrap();
        let candidate: GameCandidate = serde_json::from_value(serde_json::json!({
            "key": "super-mario-world", "title_hint": "Super Mario World",
            "alternate_title_hints": [], "identity_confidence": "filename_candidate_only",
            "possible_title_collision": false, "source_record_ids": ["x"],
            "source_ids": ["libretro"], "region_hints": ["USA"],
            "format_hints": [], "code_fields": 1, "native_memory_entries": 0
        }))
        .unwrap();
        let all = bundle
            .entries_for_candidate(&candidate, None, None)
            .unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].entry.ordinal, 0);
        assert_eq!(all[0].entry.code.as_deref(), Some("ABCD"));
        assert_eq!(all[0].source_record_id, "x");
        assert_eq!(all[0].provenance.source_id, "libretro");
        assert_eq!(all[1].entry.ordinal, 1);
        assert_eq!(all[1].entry.role.as_deref(), Some("section-heading"));

        let only_codes = bundle
            .entries_for_candidate(&candidate, Some("x"), Some("code"))
            .unwrap();
        assert_eq!(only_codes.len(), 1);
        assert_eq!(only_codes[0].entry.ordinal, 0);
        let headings = bundle
            .entries_for_candidate(&candidate, None, Some("section-heading"))
            .unwrap();
        assert_eq!(headings.len(), 1);
        assert!(bundle
            .entries_for_candidate(&candidate, Some("other"), None)
            .is_err());
        assert!(bundle
            .entries_for_candidate(&candidate, Some(""), None)
            .is_err());
        assert!(bundle
            .entries_for_candidate(&candidate, None, Some("verified"))
            .is_err());
    }

    #[test]
    fn description_search_preserves_source_text_and_filter_semantics() {
        let bundle = decode_bundle(fixture().as_slice()).unwrap();
        let candidate: GameCandidate = serde_json::from_value(serde_json::json!({
            "key": "super-mario-world", "title_hint": "Super Mario World",
            "alternate_title_hints": [], "identity_confidence": "filename_candidate_only",
            "possible_title_collision": false, "source_record_ids": ["x"],
            "source_ids": ["libretro"], "region_hints": ["USA"],
            "format_hints": [], "code_fields": 1, "native_memory_entries": 0
        }))
        .unwrap();

        let matched = bundle
            .search_entries_for_candidate(&candidate, None, None, Some("  INFINITE lives "))
            .unwrap();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].entry.description.as_deref(), Some("Infinite Lives"));
        assert_eq!(matched[0].entry.code.as_deref(), Some("ABCD"));
        assert_eq!(matched[0].entry.ordinal, 0);
        assert_eq!(matched[0].provenance.upstream_path, "cht/sample.cht");
        assert_eq!(
            bundle
                .search_entries_for_candidate(
                    &candidate,
                    Some("x"),
                    Some("code"),
                    Some("lIvEs"),
                )
                .unwrap()
                .len(),
            1
        );
        assert!(bundle
            .search_entries_for_candidate(
                &candidate,
                None,
                Some("section-heading"),
                Some("lives"),
            )
            .unwrap()
            .is_empty());
        assert_eq!(
            bundle
                .search_entries_for_candidate(
                    &candidate,
                    None,
                    Some("section-heading"),
                    Some("heading"),
                )
                .unwrap()
                .len(),
            1
        );
        assert!(bundle
            .search_entries_for_candidate(&candidate, None, None, Some("ABCD"))
            .unwrap()
            .is_empty());
        assert!(bundle
            .search_entries_for_candidate(&candidate, None, None, Some("nonexistent"))
            .unwrap()
            .is_empty());
        assert!(bundle
            .search_entries_for_candidate(&candidate, None, None, Some("  "))
            .is_err());
        assert!(bundle
            .search_entries_for_candidate(&candidate, Some("missing"), None, Some("lives"))
            .is_err());
        assert_eq!(
            bundle
                .search_entries_for_candidate(&candidate, None, None, None)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn candidate_lookup_preserves_filename_unresolved_source_groups() {
        let mut bundle = decode_bundle(fixture().as_slice()).unwrap();
        bundle.records[0].candidate_game_key.clear();
        let candidate: GameCandidate = serde_json::from_value(serde_json::json!({
            "key": "unresolved:x", "title_hint": "Unresolved filename",
            "alternate_title_hints": [], "identity_confidence": "filename_candidate_only",
            "possible_title_collision": false, "source_record_ids": ["x"],
            "source_ids": ["libretro"], "region_hints": ["USA"],
            "format_hints": [], "code_fields": 1, "native_memory_entries": 0
        }))
        .unwrap();
        let records = bundle.sources_for_candidate(&candidate).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "x");
        assert!(records[0].candidate_game_key.is_empty());
    }

    #[test]
    fn exact_candidate_lookup_rejects_ambiguous_keys() {
        let candidate: GameCandidate = serde_json::from_value(serde_json::json!({
            "key": "game", "title_hint": "Game", "alternate_title_hints": [],
            "identity_confidence": "filename_candidate_only",
            "possible_title_collision": false, "source_record_ids": ["source"],
            "source_ids": ["archive"], "region_hints": [], "format_hints": [],
            "code_fields": 1, "native_memory_entries": 0
        }))
        .unwrap();
        let mut index = GameIndex {
            schema_version: 1,
            platform: "snes".to_owned(),
            identity_rule: "filename candidates".to_owned(),
            candidates: vec![candidate],
        };
        assert!(index.find_candidate("game").is_ok());
        assert!(index.find_candidate("").is_err());
        assert!(index.find_candidate("missing").is_err());
        let clone: GameCandidate =
            serde_json::from_value(serde_json::to_value(&index.candidates[0]).unwrap()).unwrap();
        index.candidates.push(clone);
        assert!(index.find_candidate("game").is_err());
    }

    #[test]
    fn exact_source_lookup_rejects_missing_and_duplicate_source_ids() {
        let mut bundle = decode_bundle(fixture().as_slice()).unwrap();
        assert_eq!(
            bundle.find_source_record("x").unwrap().title_hint,
            "Super Mario World"
        );
        assert!(bundle.find_source_record("").is_err());
        assert!(bundle.find_source_record("nonexistent").is_err());

        let copy: IndexedFile =
            serde_json::from_value(serde_json::to_value(&bundle.records[0]).unwrap()).unwrap();
        bundle.records.push(copy);
        assert!(bundle.find_source_record("x").is_err());
    }

    #[test]
    fn exact_code_search_preserves_text_ordinals_and_provenance() {
        let bundle = decode_bundle(fixture().as_slice()).unwrap();
        let hits = bundle.search_exact_code("ABCD");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source_record_id, "x");
        assert_eq!(hits[0].candidate_game_key, "super-mario-world");
        assert_eq!(hits[0].cheat.ordinal, 0);
        assert_eq!(hits[0].cheat.description.as_deref(), Some("Infinite Lives"));
        assert_eq!(hits[0].provenance.upstream_path, "cht/sample.cht");
        assert!(bundle.search_exact_code("abcd").is_empty());
        assert!(bundle.search_exact_code("ABCD ").is_empty());
        assert!(bundle.search_exact_code("").is_empty());
        assert!(bundle.search_exact_code("A heading").is_empty());
    }

    #[test]
    fn exact_code_search_keeps_distinct_ordinals_within_one_original_file() {
        let mut bundle = decode_bundle(fixture().as_slice()).unwrap();
        let mut repeated = serde_json::to_value(&bundle.records[0].codes[0]).unwrap();
        repeated["ordinal"] = serde_json::json!(101);
        repeated["description"] = serde_json::json!("Repeated original code text");
        bundle.records[0]
            .codes
            .push(serde_json::from_value(repeated).unwrap());

        let hits = bundle.search_exact_code("ABCD");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].source_record_id, hits[1].source_record_id);
        assert_eq!(
            hits.iter().map(|hit| hit.cheat.ordinal).collect::<Vec<_>>(),
            vec![0, 101]
        );
        assert_ne!(hits[0].cheat.description, hits[1].cheat.description);
    }

    #[test]
    fn exact_code_search_does_not_merge_independent_source_occurrences() {
        let mut bundle = decode_bundle(fixture().as_slice()).unwrap();
        let mut duplicate = serde_json::to_value(&bundle.records[0]).unwrap();
        duplicate["id"] = serde_json::json!("other-source");
        duplicate["raw_filename"] = serde_json::json!("Other Edition.cht");
        duplicate["codes"][0]["ordinal"] = serde_json::json!(37);
        duplicate["codes"][0]["description"] = serde_json::json!("Different claimed effect");
        duplicate["provenance"]["upstream_path"] = serde_json::json!("cht/other-edition.cht");
        bundle
            .records
            .push(serde_json::from_value(duplicate).unwrap());

        let hits = bundle.search_exact_code("ABCD");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].source_record_id, "x");
        assert_eq!(hits[0].cheat.ordinal, 0);
        assert_eq!(hits[1].source_record_id, "other-source");
        assert_eq!(hits[1].cheat.ordinal, 37);
        assert_eq!(hits[0].cheat.description.as_deref(), Some("Infinite Lives"));
        assert_eq!(
            hits[1].cheat.description.as_deref(),
            Some("Different claimed effect")
        );
        assert_ne!(
            hits[0].provenance.upstream_path,
            hits[1].provenance.upstream_path
        );
        assert!(bundle.search_exact_code("abcd").is_empty());
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
