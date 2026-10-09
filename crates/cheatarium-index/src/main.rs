use cheatarium_codecs::{decode_snes, decode_snes_unlabelled};
use cheatarium_index::effect_signals::{classify, EffectTaxonomy};
use cheatarium_index::{
    candidate_game_key, format_hint, parse_artemis_ncl, parse_cht, parse_cwcheat_ini,
    parse_gecko_ini, parse_gecko_markdown, parse_goldhen_json, parse_goldhen_mc4,
    parse_goldhen_shn, parse_native_sections, region_hint, title_hint, Code, CompositionEvidence,
    SourceComposition,
};
use flate2::{Compression, GzBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
struct LibretroMapping {
    systems: Vec<System>,
}

#[derive(Clone, Deserialize)]
struct System {
    import_id: String,
    platform: String,
    source_directory: String,
}

#[derive(Deserialize)]
struct SourceManifest {
    id: String,
    repository: String,
    snapshot_commit: String,
    license: String,
    files: Vec<ManifestFile>,
}

#[derive(Deserialize)]
struct ManifestFile {
    upstream_path: String,
    archive_path: String,
    git_blob_sha: String,
}

#[derive(Serialize)]
struct Provenance {
    source_id: String,
    repository: String,
    revision: String,
    license: String,
    upstream_path: String,
    archive_path: String,
    git_blob_sha: String,
}

#[derive(Serialize)]
struct IndexedFile {
    id: String,
    title_hint: String,
    candidate_game_key: String,
    identity_confidence: &'static str,
    raw_filename: String,
    region_hint: Option<String>,
    format_hint: Option<&'static str>,
    declared_cheats: Option<usize>,
    parse_warnings: Vec<String>,
    codes: Vec<Code>,
    provenance: Provenance,
}

#[derive(Serialize)]
struct Bundle {
    schema_version: u32,
    platform: String,
    game_identity_rule: &'static str,
    compatibility_rule: &'static str,
    records: Vec<IndexedFile>,
}

#[derive(Serialize)]
struct CatalogItem {
    platform: String,
    artifact: String,
    game_index_artifact: String,
    repeat_index_artifact: String,
    repeat_groups: usize,
    tag_index_artifact: String,
    tag_matches: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity_artifact: Option<String>,
    game_candidate_groups: usize,
    source_files: usize,
    code_fields: usize,
    native_memory_entries: usize,
    decoded_snes_code_fields: usize,
    warnings: usize,
}

#[derive(Serialize)]
struct Catalog {
    schema_version: u32,
    format: &'static str,
    matching_policy: &'static str,
    bundles: Vec<CatalogItem>,
}

#[derive(Default)]
struct CandidateAccum {
    titles: BTreeSet<String>,
    regions: BTreeSet<String>,
    formats: BTreeSet<String>,
    sources: BTreeSet<String>,
    source_record_ids: Vec<String>,
    code_fields: usize,
    native_memory_entries: usize,
}

#[derive(Serialize)]
struct GameCandidate {
    key: String,
    title_hint: String,
    alternate_title_hints: Vec<String>,
    identity_confidence: &'static str,
    possible_title_collision: bool,
    source_record_ids: Vec<String>,
    source_ids: Vec<String>,
    region_hints: Vec<String>,
    format_hints: Vec<String>,
    code_fields: usize,
    native_memory_entries: usize,
}

#[derive(Serialize)]
struct GameIndex {
    schema_version: u32,
    platform: String,
    identity_rule: &'static str,
    candidates: Vec<GameCandidate>,
}

fn build_game_index(platform: &str, records: &[IndexedFile]) -> GameIndex {
    let mut groups: BTreeMap<String, CandidateAccum> = BTreeMap::new();
    for record in records {
        // Never merge records that lack a usable filename-derived key.
        let key = if record.candidate_game_key.is_empty() {
            format!("unresolved:{}", record.id)
        } else {
            record.candidate_game_key.clone()
        };
        let entry = groups.entry(key).or_default();
        entry.titles.insert(record.title_hint.clone());
        if let Some(region) = &record.region_hint {
            entry.regions.insert(region.clone());
        }
        if let Some(format) = record.format_hint {
            entry.formats.insert(format.to_owned());
        }
        entry.sources.insert(record.provenance.source_id.clone());
        entry.source_record_ids.push(record.id.clone());
        entry.code_fields += record.codes.iter().filter(|c| c.role == "code").count();
        entry.native_memory_entries += record
            .codes
            .iter()
            .filter(|c| c.role == "memory-entry")
            .count();
    }
    let candidates = groups
        .into_iter()
        .map(|(key, group)| {
            let title_hint = group.titles.iter().next().cloned().unwrap_or_default();
            let possible_title_collision = group.titles.len() > 1;
            GameCandidate {
                key,
                title_hint,
                alternate_title_hints: group.titles.into_iter().skip(1).collect(),
                identity_confidence: "filename_candidate_only",
                possible_title_collision,
                source_record_ids: group.source_record_ids,
                source_ids: group.sources.into_iter().collect(),
                region_hints: group.regions.into_iter().collect(),
                format_hints: group.formats.into_iter().collect(),
                code_fields: group.code_fields,
                native_memory_entries: group.native_memory_entries,
            }
        })
        .collect();
    GameIndex {
        schema_version: 1,
        platform: platform.to_owned(),
        identity_rule: "unverified source-path-derived grouping, possibly title/build IDs; keys are not verified ROM identities",
        candidates,
    }
}

/// An occurrence of an *identical original code string*, not a verified
/// equivalence of gameplay effects or ROM compatibility.
#[derive(Serialize)]
struct RepeatOccurrence {
    source_record_id: String,
    ordinal: usize,
    description: Option<String>,
}

#[derive(Serialize)]
struct RepeatedCode {
    candidate_game_key: String,
    region_hint: Option<String>,
    revision_hint: Option<String>,
    declared_format: Option<String>,
    source_code: String,
    relation: &'static str,
    confirmed_equivalent_cheat: bool,
    verified_rom_compatibility: bool,
    description_variants: usize,
    description_text_varies: bool,
    occurrences: Vec<RepeatOccurrence>,
}

#[derive(Serialize)]
struct RepeatedCodeIndex {
    schema_version: u32,
    platform: String,
    interpretation: &'static str,
    groups: Vec<RepeatedCode>,
}

/// Preserve explicitly marked revisions/builds. Unknown is *not* presumed
/// to mean "Rev 0". Exclude only parenthetical tags denoting a build variant,
/// leaving game key and region hints to their own separate fields.
fn revision_hint(filename: &str) -> Option<String> {
    for segment in filename.split('(').skip(1) {
        let Some((inside, _)) = segment.split_once(')') else {
            continue;
        };
        let tag = inside.trim().to_ascii_lowercase();
        if tag.starts_with("rev ")
            || tag.starts_with("revision ")
            || tag.starts_with("version ")
            || tag.starts_with("beta")
            || tag.starts_with("proto")
            || tag.starts_with("demo")
            || tag.starts_with("v1.")
            || tag.starts_with("v2.")
            || tag == "unl"
            || tag.contains("hack")
            || tag.contains("translation")
            || tag == "virtual console"
        {
            return Some(tag);
        }
    }
    None
}

/// An exact-text repeat is scoped narrowly by platform, *candidate* game
/// title, region, declared device format and explicit build-marker text.
/// Never canonicalize device codes, infer a code format, strip a header or
/// merge dissimilar descriptions into an asserted shared gameplay effect.
/// Keep ONLY groups spanning distinct original source files.
fn build_repeated_code_index(platform: &str, records: &[IndexedFile]) -> RepeatedCodeIndex {
    type RepeatKey = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    );
    let mut buckets: BTreeMap<RepeatKey, Vec<RepeatOccurrence>> = BTreeMap::new();
    for record in records {
        if record.candidate_game_key.is_empty() {
            continue;
        }
        let build = revision_hint(&record.raw_filename);
        for cheat in &record.codes {
            if cheat.role != "code" {
                continue;
            }
            let Some(code) = cheat.code.as_ref().filter(|s| !s.trim().is_empty()) else {
                continue;
            };
            // Keep exact original source text; case, whitespace and compound
            // ordering are potentially meaningful.
            let key = (
                record.candidate_game_key.clone(),
                record.region_hint.clone(),
                build.clone(),
                record.format_hint.map(str::to_owned),
                code.clone(),
            );
            buckets.entry(key).or_default().push(RepeatOccurrence {
                source_record_id: record.id.clone(),
                ordinal: cheat.ordinal,
                description: cheat.description.clone(),
            });
        }
    }
    let mut groups = Vec::new();
    for (
        (candidate_game_key, region_hint, revision_hint, declared_format, source_code),
        mut occurrences,
    ) in buckets
    {
        occurrences.sort_by(|a, b| {
            (&a.source_record_id, a.ordinal).cmp(&(&b.source_record_id, b.ordinal))
        });
        if occurrences
            .iter()
            .map(|occ| occ.source_record_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            < 2
        {
            continue;
        }
        let description_variants = occurrences
            .iter()
            .filter_map(|occ| occ.description.as_deref())
            .filter(|value| !value.trim().is_empty())
            .collect::<BTreeSet<_>>()
            .len();
        groups.push(RepeatedCode {
            candidate_game_key,
            region_hint,
            revision_hint,
            declared_format,
            source_code,
            relation: "identical-raw-code-text-within-advisory-filename-bucket",
            confirmed_equivalent_cheat: false,
            verified_rom_compatibility: false,
            description_variants,
            description_text_varies: description_variants > 1,
            occurrences,
        });
    }
    RepeatedCodeIndex {
        schema_version: 1,
        platform: platform.to_owned(),
        interpretation:
            "exact source-text repetition only; not a game, edition, effect, or compatibility match",
        groups,
    }
}

#[derive(Serialize)]
struct EffectTagOccurrence {
    source_record_id: String,
    ordinal: usize,
    candidate_game_key: String,
    matched_phrase: String,
}

#[derive(Serialize)]
struct EffectTagCategory {
    id: String,
    matches: Vec<EffectTagOccurrence>,
}

#[derive(Serialize)]
struct EffectTagIndex {
    schema_version: u32,
    platform: String,
    taxonomy_id: String,
    interpretation: &'static str,
    categories: Vec<EffectTagCategory>,
}

/// Index source-description phrases only. Region, file name and decoded code
/// text never supply a classification; emitted references point to originals.
fn build_effect_tag_index(
    platform: &str,
    records: &[IndexedFile],
    taxonomy: &EffectTaxonomy,
) -> EffectTagIndex {
    let mut groups: BTreeMap<String, Vec<EffectTagOccurrence>> = BTreeMap::new();
    for record in records {
        for code in &record.codes {
            if code.role != "code" && code.role != "memory-entry" {
                continue;
            }
            let Some(description) = &code.description else {
                continue;
            };
            for (category, phrase) in classify(description, taxonomy) {
                groups
                    .entry(category.to_owned())
                    .or_default()
                    .push(EffectTagOccurrence {
                        source_record_id: record.id.clone(),
                        ordinal: code.ordinal,
                        candidate_game_key: record.candidate_game_key.clone(),
                        matched_phrase: phrase.to_owned(),
                    });
            }
        }
    }
    let categories = groups
        .into_iter()
        .map(|(id, matches)| EffectTagCategory { id, matches })
        .collect();
    EffectTagIndex {
        schema_version: 1,
        platform: platform.to_owned(),
        taxonomy_id: taxonomy.id.clone(),
        interpretation: "lexical-source-description-signal-only; no verified game effect or cartridge compatibility",
        categories,
    }
}

#[derive(Deserialize)]
struct CompositionRegistry {
    schema_version: u32,
    platform: String,
    interpretation: String,
    records: Vec<CompositionOverride>,
}

#[derive(Deserialize)]
struct CompositionOverride {
    source_record_id: String,
    source_ordinal: usize,
    source_git_blob_sha: String,
    raw_code: String,
    relation: String,
    alternatives: Vec<Vec<String>>,
    evidence: Vec<CompositionEvidence>,
    rom_match_verified: bool,
    simultaneous_execution_confirmed: bool,
}

fn composition_from_override(
    source: &str,
    blob: &str,
    code: &str,
    candidate: CompositionOverride,
) -> Result<SourceComposition> {
    if candidate.source_record_id != source
        || candidate.source_git_blob_sha != blob
        || candidate.raw_code != code
        || candidate.relation != "revision-alternatives"
        || candidate.rom_match_verified
        || candidate.simultaneous_execution_confirmed
        || candidate.alternatives.len() < 2
        || candidate.evidence.is_empty()
        || candidate.evidence.iter().any(|e| {
            !e.url.starts_with("https://")
                || e.reference.trim().is_empty()
                || e.source_revision.trim().is_empty()
        })
    {
        return Err("Invalid source composition evidence or attribution".into());
    }
    let components: Vec<String> = candidate
        .alternatives
        .iter()
        .flat_map(|group| group.iter().map(|c| c.trim().to_owned()))
        .collect();
    let original: Vec<String> = code.split('+').map(|c| c.trim().to_owned()).collect();
    if candidate
        .alternatives
        .iter()
        .any(|group| group.is_empty() || group.iter().any(|c| c.trim().is_empty() || c.trim() != c))
        || components != original
    {
        return Err("Source composition alternatives do not partition original code".into());
    }
    Ok(SourceComposition {
        relation: candidate.relation,
        alternatives: candidate.alternatives,
        evidence: candidate.evidence,
        rom_match_verified: false,
        simultaneous_execution_confirmed: false,
    })
}

fn checked_path(root: &Path, rel: &str) -> Result<PathBuf> {
    let path = Path::new(rel);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("Unsafe archive path: {rel}").into());
    }
    Ok(root.join(path))
}

fn parse_args() -> Result<(PathBuf, PathBuf, Option<BTreeSet<String>>)> {
    let mut args = env::args().skip(1);
    let mut root = PathBuf::from(".");
    let mut out = PathBuf::from("generated/v1");
    let mut wanted = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
            "--out" => out = PathBuf::from(args.next().ok_or("--out requires a path")?),
            "--systems" => {
                let s = args
                    .next()
                    .ok_or("--systems requires comma-separated platforms")?;
                wanted = Some(
                    s.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect(),
                );
            }
            "--help" | "-h" => {
                println!("cheatarium-index [--root PATH] [--out PATH] [--systems nes,snes]");
                println!("Produces deterministic per-console JSON.gz bundles and a catalog.json.");
                std::process::exit(0);
            }
            _ => return Err(format!("Unknown argument {arg}").into()),
        }
    }
    Ok((root, out, wanted))
}

fn run() -> Result<()> {
    let (root, out, wanted) = parse_args()?;
    let mapping: LibretroMapping =
        serde_json::from_slice(&fs::read(root.join("platforms/libretro-mapping.json"))?)?;
    let manifest: SourceManifest =
        serde_json::from_slice(&fs::read(root.join("sources/libretro-database.json"))?)?;
    if manifest.id != "libretro-database" {
        return Err("Expected Libretro source manifest".into());
    }
    let mut source_directories = HashMap::<String, System>::new();
    let mut known_platforms = BTreeSet::new();
    for system in mapping.systems {
        if source_directories
            .insert(system.source_directory.clone(), system.clone())
            .is_some()
        {
            return Err("Duplicate Libretro directory mapping".into());
        }
        known_platforms.insert(system.platform);
    }
    known_platforms.insert("3ds".to_owned());
    known_platforms.insert("switch".to_owned());
    for platform in ["ps2", "ps3", "ps4", "gamecube", "wii", "wii-u"] {
        known_platforms.insert(platform.to_owned());
    }
    if let Some(ref filter) = wanted {
        for id in filter {
            if !known_platforms.contains(id) {
                return Err(format!("Unrecognized platform requested: {id}").into());
            }
        }
    }

    let composition_path = root.join("interpretations/v1/snes.json");
    let source_compositions: CompositionRegistry =
        serde_json::from_slice(&fs::read(&composition_path)?)?;
    if source_compositions.schema_version != 1
        || source_compositions.platform != "snes"
        || source_compositions.interpretation
            != "evidenced-source-code-layout-not-ROM-compatibility"
    {
        return Err("Unsupported SNES source-composition registry".into());
    }
    let mut overrides: BTreeMap<(String, usize), CompositionOverride> = BTreeMap::new();
    for item in source_compositions.records {
        let key = (item.source_record_id.clone(), item.source_ordinal);
        if overrides.insert(key, item).is_some() {
            return Err("Duplicate source composition override".into());
        }
    }
    let scan_snes = wanted
        .as_ref()
        .is_none_or(|systems| systems.contains("snes"));
    let mut by_platform: BTreeMap<String, Vec<IndexedFile>> = BTreeMap::new();
    for item in manifest.files {
        let Some(rest) = item.upstream_path.strip_prefix("cht/") else {
            continue;
        };
        let Some((source_dir, filename)) = rest.split_once('/') else {
            continue;
        };
        let Some(system) = source_directories.get(source_dir) else {
            // Unmapped source collections are not silently attributed to a console.
            continue;
        };
        if wanted
            .as_ref()
            .is_some_and(|v| !v.contains(&system.platform))
        {
            continue;
        }
        let path = checked_path(&root, &item.archive_path)?;
        let content = fs::read(&path)?;
        let mut parsed = parse_cht(&String::from_utf8_lossy(&content));
        let stem = filename.strip_suffix(".cht").unwrap_or(filename);
        let source_id = format!("{}:{}", manifest.id, item.upstream_path);
        if system.platform == "snes" {
            let declared_format = format_hint(stem);
            for code in &mut parsed.codes {
                if code.role != "code" {
                    continue;
                }
                if let Some(source_code) = &code.code {
                    let override_entry = overrides.remove(&(source_id.clone(), code.ordinal));
                    if source_code.contains('+') {
                        code.composition = Some(match override_entry {
                            Some(entry) => composition_from_override(
                                &source_id,
                                &item.git_blob_sha,
                                source_code,
                                entry,
                            )?,
                            None => SourceComposition::unresolved(),
                        });
                    } else if override_entry.is_some() {
                        return Err(
                            "Composition override must reference a '+'-joined source code".into(),
                        );
                    }
                    // Known revision-alternatives are not a simultaneous set
                    // of writes: preserve only the documented per-version groups.
                    if code
                        .composition
                        .as_ref()
                        .is_some_and(|c| c.relation == "revision-alternatives")
                    {
                        continue;
                    }
                    // Unknown filename formats never override declared formats.
                    // An unlabelled code is interpreted only if its entire
                    // compound text uses a single unambiguous syntax.
                    code.snes_decode = match declared_format {
                        Some(format) => decode_snes(format, source_code).ok(),
                        None => decode_snes_unlabelled(source_code).ok(),
                    };
                }
            }
        }
        let guess = title_hint(stem);
        let record = IndexedFile {
            id: source_id,
            candidate_game_key: candidate_game_key(&guess),
            title_hint: guess,
            identity_confidence: "filename_heuristic_only",
            raw_filename: filename.to_owned(),
            region_hint: region_hint(stem),
            format_hint: format_hint(stem),
            declared_cheats: parsed.declared_count,
            parse_warnings: parsed.warnings,
            codes: parsed.codes,
            provenance: Provenance {
                source_id: manifest.id.clone(),
                repository: manifest.repository.clone(),
                revision: manifest.snapshot_commit.clone(),
                license: manifest.license.clone(),
                upstream_path: item.upstream_path,
                archive_path: item.archive_path,
                git_blob_sha: item.git_blob_sha,
            },
        };
        by_platform
            .entry(system.platform.clone())
            .or_default()
            .push(record);
    }

    // Sharkive files are native Gateshark/Atmosphère sections indexed by
    // literal upstream title/build IDs, NEVER by guessed human game names.
    // The root LICENSE/README retain legal/credit metadata outside this index.
    let sharkive: SourceManifest =
        serde_json::from_slice(&fs::read(root.join("sources/sharkive.json"))?)?;
    if sharkive.id != "sharkive" {
        return Err("Expected pinned Sharkive source manifest".into());
    }
    for item in sharkive.files {
        let parts: Vec<&str> = item.upstream_path.split('/').collect();
        let (platform, title_id, build_id) = match parts.as_slice() {
            ["3ds", original] if original.ends_with(".txt") => {
                ("3ds", original.trim_end_matches(".txt"), None)
            }
            ["switch", title_id, original] if original.ends_with(".txt") => {
                ("switch", *title_id, Some(original.trim_end_matches(".txt")))
            }
            _ => continue, // license, README and non-code upstream metadata
        };
        if wanted.as_ref().is_some_and(|v| !v.contains(platform)) {
            continue;
        }
        if title_id.len() != 16
            || !title_id.bytes().all(|c| c.is_ascii_hexdigit())
            || build_id.is_some_and(|id| {
                id.is_empty() || id.len() > 64 || !id.bytes().all(|c| c.is_ascii_hexdigit())
            })
        {
            return Err(format!(
                "Malformed original Sharkive title/build path: {}",
                item.upstream_path
            )
            .into());
        }
        let source_file_path = checked_path(&root, &item.archive_path)?;
        let raw = fs::read(source_file_path)?;
        let content = String::from_utf8_lossy(&raw);
        let mut parsed = parse_native_sections(&content);
        if matches!(content, std::borrow::Cow::Owned(_)) {
            parsed.warnings.push("Original source contains non-UTF8 bytes; original is preserved verbatim in archive".into());
        }
        if build_id.is_some_and(|id| id.len() != 16) {
            parsed.warnings.push("Upstream Switch build-ID filename has nonstandard length; preserved verbatim, not normalized or verified".into());
        }
        let title_hint = match build_id {
            Some(build) => format!("Title ID {title_id} / build {build}"),
            None => format!("Title ID {title_id}"),
        };
        let key = match build_id {
            Some(build) => format!(
                "title-id-{}-build-{}",
                title_id.to_ascii_lowercase(),
                build.to_ascii_lowercase()
            ),
            None => format!("title-id-{}", title_id.to_ascii_lowercase()),
        };
        let source_file_name = item
            .upstream_path
            .strip_prefix(&format!("{platform}/"))
            .unwrap_or(&item.upstream_path)
            .to_owned();
        let record = IndexedFile {
            id: format!("sharkive:{}", item.upstream_path),
            title_hint,
            candidate_game_key: key,
            identity_confidence: "source-path-title-and-build-id-unverified",
            raw_filename: source_file_name,
            region_hint: None,
            format_hint: Some(if platform == "3ds" {
                "gateshark"
            } else {
                "atmosphere"
            }),
            declared_cheats: None,
            parse_warnings: parsed.warnings,
            codes: parsed.codes,
            provenance: Provenance {
                source_id: sharkive.id.clone(),
                repository: sharkive.repository.clone(),
                revision: sharkive.snapshot_commit.clone(),
                license: sharkive.license.clone(),
                upstream_path: item.upstream_path,
                archive_path: item.archive_path,
                git_blob_sha: item.git_blob_sha,
            },
        };
        by_platform
            .entry(platform.to_owned())
            .or_default()
            .push(record);
    }

    // Original GoldHEN filename-to-title lists supply game labels.
    // These labels are source claims, not independently verified game builds.
    let mut goldhen_titles = BTreeMap::<String, String>::new();
    for (directory, expected) in [("json", 1878usize), ("mc4", 706), ("shn", 1702)] {
        let raw = fs::read_to_string(root.join(format!("archive/goldhen/{directory}.txt")))?;
        let mut count = 0;
        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            let Some((filename, label)) = line.split_once('=') else {
                return Err(format!("Malformed original {directory} source title list").into());
            };
            if filename.is_empty() || label.trim().is_empty() || filename.contains('/') {
                return Err(format!("Malformed source title for GoldHEN {directory}").into());
            }
            let key = format!("{directory}/{filename}");
            if goldhen_titles
                .insert(key.clone(), label.trim().to_owned())
                .is_some()
            {
                return Err(format!("Duplicate GoldHEN source name {key}").into());
            }
            count += 1;
        }
        if count != expected {
            return Err(
                format!("GoldHEN {directory} original title count {count} != {expected}").into(),
            );
        }
    }

    // Pinned later-generation sources are independently inventoried, and
    // copied unchanged. Build ONLY source-derived game/title/build candidates.
    // In particular, GoldHEN PS2-looking IDs are not claims of PS2 support.
    for manifest_id in [
        "artemis-ps3",
        "goldhen",
        "admentus-enhancement-codes",
        "mkwcat-gecko-codes",
        "cookieplmonster-console-cheat-codes",
    ] {
        let native: SourceManifest =
            serde_json::from_slice(&fs::read(root.join(format!("sources/{manifest_id}.json")))?)?;
        if native.id != manifest_id {
            return Err(format!("Wrong source manifest identity for {manifest_id}").into());
        }
        for item in native.files {
            let original = &item.upstream_path;
            let (platform, format, title) = match manifest_id {
                "cookieplmonster-console-cheat-codes"
                    if original.starts_with("PS2/") && original.ends_with(".pnach") =>
                {
                    (
                        "ps2",
                        "pcsx2-pnach",
                        original.trim_start_matches("PS2/").to_owned(),
                    )
                }
                "cookieplmonster-console-cheat-codes"
                    if original.starts_with("PS1/") && original.ends_with(".cht") =>
                {
                    (
                        "ps1",
                        "ps1-native-cht",
                        original.trim_start_matches("PS1/").to_owned(),
                    )
                }
                "cookieplmonster-console-cheat-codes"
                    if original.starts_with("PSP/") && original.ends_with(".ini") =>
                {
                    (
                        "psp",
                        "psp-cwcheat-ini",
                        original.trim_start_matches("PSP/").to_owned(),
                    )
                }
                "cookieplmonster-console-cheat-codes"
                    if original.starts_with("GC/") && original.ends_with(".ini") =>
                {
                    (
                        "gamecube",
                        "dolphin-ini",
                        original.trim_start_matches("GC/").to_owned(),
                    )
                }
                "artemis-ps3"
                    if original.starts_with("docs/codes/") && original.ends_with(".ncl") =>
                {
                    (
                        "ps3",
                        "artemis-ncl",
                        original
                            .trim_start_matches("docs/codes/")
                            .trim_end_matches(".ncl")
                            .to_owned(),
                    )
                }
                "goldhen" if original.starts_with("json/") && original.ends_with(".json") => (
                    "ps4",
                    "goldhen-json",
                    original
                        .trim_start_matches("json/")
                        .trim_end_matches(".json")
                        .to_owned(),
                ),
                "goldhen" if original.starts_with("mc4/") && original.ends_with(".xml") => (
                    "ps4",
                    "goldhen-mc4-xml",
                    original.trim_start_matches("mc4/").to_owned(),
                ),
                "goldhen"
                    if original.starts_with("mc4/")
                        && original.to_ascii_lowercase().ends_with(".mc4") =>
                {
                    (
                        "ps4",
                        "goldhen-mc4",
                        original.trim_start_matches("mc4/").to_owned(),
                    )
                }
                "goldhen"
                    if original.starts_with("shn/")
                        && (original.ends_with(".shn") || original.ends_with(".xml")) =>
                {
                    (
                        "ps4",
                        "goldhen-shn",
                        original.trim_start_matches("shn/").to_owned(),
                    )
                }
                "admentus-enhancement-codes" if original.ends_with(".ini") => {
                    let platform = if original.contains("(GC)/") {
                        "gamecube"
                    } else {
                        "wii"
                    };
                    (platform, "dolphin-ini", original.to_owned())
                }
                "mkwcat-gecko-codes"
                    if original.ends_with(".md")
                        && ["mkw/", "nsmbw/", "nsmbu/"]
                            .iter()
                            .any(|prefix| original.starts_with(prefix))
                        && original != "nsmbu/README.md" =>
                {
                    let platform = if original.starts_with("nsmbu/") {
                        "wii-u"
                    } else {
                        "wii"
                    };
                    (platform, "gecko-markdown", original.to_owned())
                }
                _ => continue,
            };
            if wanted.as_ref().is_some_and(|v| !v.contains(platform)) {
                continue;
            }
            let raw = fs::read(checked_path(&root, &item.archive_path)?)?;
            // Native SHN files may be UTF-16. Do not claim text rendering
            // is exact byte preservation: the archive remains authoritative.
            let decoded = if raw.starts_with(&[0xff, 0xfe]) {
                String::from_utf16_lossy(
                    &raw[2..]
                        .chunks_exact(2)
                        .map(|p| u16::from_le_bytes([p[0], p[1]]))
                        .collect::<Vec<_>>(),
                )
            } else if raw.starts_with(&[0xfe, 0xff]) {
                String::from_utf16_lossy(
                    &raw[2..]
                        .chunks_exact(2)
                        .map(|p| u16::from_be_bytes([p[0], p[1]]))
                        .collect::<Vec<_>>(),
                )
            } else {
                String::from_utf8_lossy(&raw).into_owned()
            };
            let mut parsed = match format {
                "artemis-ncl" => parse_artemis_ncl(&decoded),
                "goldhen-json" => parse_goldhen_json(&decoded),
                "goldhen-mc4" => parse_goldhen_mc4(&decoded),
                "goldhen-shn" => parse_goldhen_shn(&decoded),
                "goldhen-mc4-xml" => parse_goldhen_shn(&decoded),
                "dolphin-ini" => parse_gecko_ini(&decoded),
                "gecko-markdown" => parse_gecko_markdown(&decoded),
                "pcsx2-pnach" | "ps1-native-cht" => parse_native_sections(&decoded),
                "psp-cwcheat-ini" => parse_cwcheat_ini(&decoded),
                _ => unreachable!(),
            };
            if !raw.starts_with(&[0xff, 0xfe])
                && !raw.starts_with(&[0xfe, 0xff])
                && std::str::from_utf8(&raw).is_err()
            {
                parsed.warnings.push(
                    "Original non-UTF8 bytes preserved in archive; source index is lossy text"
                        .into(),
                );
            }
            let source_declared_title = if manifest_id == "goldhen" {
                goldhen_titles.get(original).cloned()
            } else {
                None
            };
            let json_title = if format == "goldhen-json" {
                serde_json::from_str::<serde_json::Value>(&decoded)
                    .ok()
                    .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_owned))
            } else {
                None
            };
            let title = json_title
                .or(source_declared_title)
                .map(|label| format!("{label} [{title}]"))
                .unwrap_or(title);
            let file_name = original.rsplit('/').next().unwrap_or(original).to_owned();
            let record = IndexedFile {
                id: format!("{manifest_id}:{original}"),
                title_hint: title,
                candidate_game_key: format!("{manifest_id}:{original}"),
                identity_confidence: "original-source-path-and-claimed-title-only",
                raw_filename: file_name,
                region_hint: None,
                format_hint: Some(format),
                declared_cheats: None,
                parse_warnings: parsed.warnings,
                codes: parsed.codes,
                provenance: Provenance {
                    source_id: native.id.clone(),
                    repository: native.repository.clone(),
                    revision: native.snapshot_commit.clone(),
                    license: native.license.clone(),
                    upstream_path: item.upstream_path,
                    archive_path: item.archive_path,
                    git_blob_sha: item.git_blob_sha,
                },
            };
            by_platform
                .entry(platform.to_owned())
                .or_default()
                .push(record);
        }
    }

    if scan_snes && !overrides.is_empty() {
        return Err(format!(
            "{} unreferenced SNES composition overrides",
            overrides.len()
        )
        .into());
    }
    fs::create_dir_all(&out)?;
    fs::create_dir_all(out.join("interpretations"))?;
    fs::copy(&composition_path, out.join("interpretations/snes.json"))?;
    fs::create_dir_all(out.join("games"))?;
    fs::create_dir_all(out.join("tags"))?;
    fs::create_dir_all(out.join("taxonomy"))?;
    let taxonomy_path = root.join("taxonomy/effects-v1.json");
    let taxonomy: EffectTaxonomy = serde_json::from_slice(&fs::read(&taxonomy_path)?)?;
    taxonomy.validate()?;
    fs::copy(&taxonomy_path, out.join("taxonomy/effects-v1.json"))?;
    fs::create_dir_all(out.join("repeats"))?;
    fs::create_dir_all(out.join("identities"))?;
    let mut catalog = Vec::new();
    for (platform, mut records) in by_platform {
        records.sort_by(|a, b| a.id.cmp(&b.id));
        let code_fields: usize = records
            .iter()
            .flat_map(|x| &x.codes)
            .filter(|x| x.role == "code")
            .count();
        let native_memory_entries: usize = records
            .iter()
            .flat_map(|x| &x.codes)
            .filter(|x| x.role == "memory-entry")
            .count();
        let decoded_snes_code_fields: usize = records
            .iter()
            .flat_map(|x| &x.codes)
            .filter(|x| x.snes_decode.is_some())
            .count();
        let warnings: usize = records.iter().map(|x| x.parse_warnings.len()).sum();
        let source_files = records.len();
        let filename = format!("{platform}.json.gz");
        let game_index_artifact = format!("games/{platform}.json.gz");
        let repeat_index_artifact = format!("repeats/{platform}.json.gz");
        let tag_index_artifact = format!("tags/{platform}.json.gz");
        let tags = build_effect_tag_index(&platform, &records, &taxonomy);
        let tag_matches = tags
            .categories
            .iter()
            .map(|category| category.matches.len())
            .sum();
        let mut tag_gzip = GzBuilder::new().mtime(0).write(
            File::create(out.join(&tag_index_artifact))?,
            Compression::default(),
        );
        tag_gzip.write_all(&serde_json::to_vec(&tags)?)?;
        tag_gzip.finish()?;
        let repeats = build_repeated_code_index(&platform, &records);
        let repeat_groups = repeats.groups.len();
        let mut repeat_gzip = GzBuilder::new().mtime(0).write(
            File::create(out.join(&repeat_index_artifact))?,
            Compression::default(),
        );
        repeat_gzip.write_all(&serde_json::to_vec(&repeats)?)?;
        repeat_gzip.finish()?;
        // Evidence-driven ROM fingerprints are distributed separately from
        // filename candidates. No ROM hash is ever inferred from a title.
        let identity_artifact = if platform == "snes" {
            let name = format!("identities/{platform}.json");
            fs::copy(
                root.join(format!("identities/v1/{platform}.json")),
                out.join(&name),
            )?;
            Some(name)
        } else {
            None
        };
        let games = build_game_index(&platform, &records);
        let game_candidate_groups = games.candidates.len();
        let mut game_gzip = GzBuilder::new().mtime(0).write(
            File::create(out.join(&game_index_artifact))?,
            Compression::default(),
        );
        game_gzip.write_all(&serde_json::to_vec(&games)?)?;
        game_gzip.finish()?;
        let bundle = Bundle {
            schema_version: 1,
            platform: platform.clone(),
            game_identity_rule: "source filename/title-ID hints only; not independently verified ROM or game-build identity",
            compatibility_rule:
                "never auto-apply a code without confirmed release/build compatibility",
            records,
        };
        let json = serde_json::to_vec(&bundle)?;
        let target = out.join(&filename);
        let mut gz = GzBuilder::new()
            .mtime(0)
            .write(File::create(target)?, Compression::default());
        gz.write_all(&json)?;
        gz.finish()?;
        println!("{platform}: {game_candidate_groups} candidate games, {source_files} source files, {code_fields} device codes, {native_memory_entries} native memory entries, {warnings} warnings");
        catalog.push(CatalogItem {
            platform,
            artifact: filename,
            game_index_artifact,
            repeat_index_artifact,
            repeat_groups,
            tag_index_artifact,
            tag_matches,
            identity_artifact,
            game_candidate_groups,
            source_files,
            code_fields,
            native_memory_entries,
            decoded_snes_code_fields,
            warnings,
        });
    }
    let catalog = Catalog {
        schema_version: 1,
        format: "cheatarium-index-v1",
        matching_policy: "candidate titles only; no ROM hashes or verified execution",
        bundles: catalog,
    };
    fs::write(
        out.join("catalog.json"),
        serde_json::to_vec_pretty(&catalog)?,
    )?;
    println!("Wrote {} index bundles", catalog.bundles.len());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cheatarium-index: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn lexical_effect_index_points_to_original_code_without_claiming_effect() {
        let a = mock("SMW (USA).cht", "a", Some("USA"), None);
        let mut b = mock("SMW (USA).cht", "b", Some("USA"), None);
        b.codes[0].description = Some("A mystery code".to_owned());
        let taxonomy: EffectTaxonomy =
            serde_json::from_str(include_str!("../../../taxonomy/effects-v1.json")).unwrap();
        let tags = build_effect_tag_index("snes", &[a, b], &taxonomy);
        let lives = tags
            .categories
            .iter()
            .find(|category| category.id == "lives")
            .unwrap();
        assert_eq!(lives.matches.len(), 1);
        assert_eq!(lives.matches[0].source_record_id, "a");
        assert_eq!(lives.matches[0].ordinal, 0);
        assert_eq!(lives.matches[0].matched_phrase, "infinite lives");
        assert!(tags.interpretation.contains("lexical"));
    }

    use super::*;

    fn mock(
        filename: &str,
        id: &str,
        region: Option<&str>,
        format: Option<&'static str>,
    ) -> IndexedFile {
        let codes =
            parse_cht("cheat0_desc = \"Infinite Lives\"\ncheat0_code = \"DDB4-6F07\"").codes;
        IndexedFile {
            id: id.to_owned(),
            title_hint: "Super Mario World".to_owned(),
            candidate_game_key: "super-mario-world".to_owned(),
            identity_confidence: "filename_heuristic_only",
            raw_filename: filename.to_owned(),
            region_hint: region.map(str::to_owned),
            format_hint: format,
            declared_cheats: Some(1),
            parse_warnings: vec![],
            codes,
            provenance: Provenance {
                source_id: "fixture".to_owned(),
                repository: "https://example.invalid/fixture".to_owned(),
                revision: "test".to_owned(),
                license: "test".to_owned(),
                upstream_path: id.to_owned(),
                archive_path: id.to_owned(),
                git_blob_sha: "test".to_owned(),
            },
        }
    }

    #[test]
    fn requires_multiple_distinct_source_records_for_repeat() {
        let a = mock("SMW (USA).cht", "record-a", Some("USA"), None);
        assert!(build_repeated_code_index("snes", &[a]).groups.is_empty());
        let a = mock("SMW (USA).cht", "record-a", Some("USA"), None);
        let b = mock("SMW (USA) (Alternative).cht", "record-b", Some("USA"), None);
        let index = build_repeated_code_index("snes", &[b, a]);
        assert_eq!(index.groups.len(), 1);
        let group = &index.groups[0];
        assert_eq!(group.occurrences.len(), 2);
        assert_eq!(group.occurrences[0].source_record_id, "record-a");
        assert_eq!(
            group.occurrences[0].description.as_deref(),
            Some("Infinite Lives")
        );
        assert!(!group.confirmed_equivalent_cheat);
        assert!(!group.verified_rom_compatibility);
        assert_eq!(group.source_code, "DDB4-6F07");
        assert_eq!(group.description_variants, 1);
        assert!(!group.description_text_varies);
    }

    #[test]
    fn preserve_description_differences_without_claiming_shared_effect() {
        let a = mock("SMW (USA).cht", "a", Some("USA"), None);
        let mut b = mock("SMW (USA).cht", "b", Some("USA"), None);
        b.codes[0].description = Some("An unrelated source description".to_owned());
        let index = build_repeated_code_index("snes", &[a, b]);
        assert_eq!(index.groups.len(), 1);
        assert_eq!(index.groups[0].description_variants, 2);
        assert!(index.groups[0].description_text_varies);
        assert!(!index.groups[0].confirmed_equivalent_cheat);
        assert_eq!(
            index.groups[0].occurrences[1].description.as_deref(),
            Some("An unrelated source description")
        );
    }

    #[test]
    fn revision_alternative_groups_reject_lost_or_reordered_source_components() {
        let base = CompositionOverride {
            source_record_id: "source:1".to_owned(),
            source_ordinal: 21,
            source_git_blob_sha: "some-pinned-blob".to_owned(),
            raw_code: "A+B+C+D".to_owned(),
            relation: "revision-alternatives".to_owned(),
            alternatives: vec![
                vec!["A".to_owned(), "B".to_owned()],
                vec!["C".to_owned(), "D".to_owned()],
            ],
            evidence: vec![CompositionEvidence {
                url: "https://example.org/faq".to_owned(),
                reference: "entry 1".to_owned(),
                source_revision: "2026-test".to_owned(),
            }],
            rom_match_verified: false,
            simultaneous_execution_confirmed: false,
        };
        let raw = "A+B+C+D";
        let ok = composition_from_override("source:1", "some-pinned-blob", raw, base);
        assert!(ok.is_ok());
        let composition = ok.unwrap();
        assert_eq!(composition.relation, "revision-alternatives");
        assert!(!composition.simultaneous_execution_confirmed);
        assert_eq!(composition.alternatives[0], vec!["A", "B"]);

        let bad = CompositionOverride {
            source_record_id: "source:1".to_owned(),
            source_ordinal: 21,
            source_git_blob_sha: "some-pinned-blob".to_owned(),
            raw_code: raw.to_owned(),
            relation: "revision-alternatives".to_owned(),
            alternatives: vec![
                vec!["A".to_owned(), "B".to_owned()],
                vec!["D".to_owned(), "C".to_owned()],
            ],
            evidence: vec![CompositionEvidence {
                url: "https://example.org/faq".to_owned(),
                reference: "entry 1".to_owned(),
                source_revision: "2026-test".to_owned(),
            }],
            rom_match_verified: false,
            simultaneous_execution_confirmed: false,
        };
        assert!(composition_from_override("source:1", "some-pinned-blob", raw, bad).is_err());
    }

    #[test]
    fn never_merge_region_revision_or_declared_device_format() {
        let baseline = mock("SMW (USA).cht", "a", Some("USA"), None);
        let europe = mock("SMW (Europe).cht", "b", Some("Europe"), None);
        let revised = mock("SMW (USA) (Rev 1).cht", "c", Some("USA"), None);
        let device = mock(
            "SMW (USA) (Game Genie).cht",
            "d",
            Some("USA"),
            Some("game-genie"),
        );
        let uncertain = mock("SMW (USA) (Rev 2).cht", "e", Some("USA"), None);
        let index =
            build_repeated_code_index("snes", &[baseline, europe, revised, device, uncertain]);
        assert!(index.groups.is_empty());
        assert_eq!(
            revision_hint("SMW (USA) (Rev 1).cht").as_deref(),
            Some("rev 1")
        );
        assert_eq!(revision_hint("SMW (USA).cht"), None);
    }

    #[test]
    fn never_fold_unequal_raw_codes_or_unresolved_title() {
        let a = mock("SMW (USA).cht", "a", Some("USA"), None);
        let mut b = mock("SMW (USA).cht", "b", Some("USA"), None);
        b.codes[0].code = Some("ddb4-6f07".to_owned());
        assert!(build_repeated_code_index("snes", &[a, b]).groups.is_empty());
        let mut unresolved = mock("SMW (USA).cht", "b", Some("USA"), None);
        unresolved.candidate_game_key.clear();
        assert!(build_repeated_code_index("snes", &[unresolved])
            .groups
            .is_empty());
    }
}
