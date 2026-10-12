use cheatarium_codecs::{decode_snes, decode_snes_unlabelled};
use cheatarium_index::effect_signals::{classify, EffectTaxonomy};
use cheatarium_index::{
    candidate_game_key, format_hint, parse_artemis_ncl, parse_cht, parse_cwcheat_ini,
    parse_gecko_flat, parse_gecko_ini, parse_gecko_markdown, parse_goldhen_json, parse_goldhen_mc4,
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

fn parse_mohasi_ps3_compiled(text: &str) -> cheatarium_index::ParsedCheats {
    // The upstream plugin syntax uses name/author/mode/score headers and raw
    // w8/w16/w32/aob commands. Preserve the *entire* source-native block for
    // each named entry, including annotations, rather than guessing opcodes.
    // Scores are upstream community reports, NOT observed Cheatarium tests.
    fn flush(codes: &mut Vec<Code>, current: &mut Option<String>, raw: &mut String) {
        if let Some(title) = current.take() {
            codes.push(Code {
                ordinal: codes.len(),
                description: Some(title),
                code: Some(std::mem::take(raw)),
                source_enabled: false,
                verification: "unverified",
                role: "code",
                native_fields: Vec::new(),
                snes_decode: None,
                composition: None,
            });
        }
    }
    let mut codes = Vec::new();
    let mut name = None;
    let mut raw = String::new();
    for line in text.split_inclusive('\n') {
        if let Some(title) = line
            .strip_prefix("name: ")
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            flush(&mut codes, &mut name, &mut raw);
            name = Some(title.to_owned());
        }
        if name.is_some() {
            raw.push_str(line);
        }
    }
    flush(&mut codes, &mut name, &mut raw);
    cheatarium_index::ParsedCheats {
        declared_count: None,
        codes,
        warnings: vec!["Artemis-derived PS3 source converted for another plugin; aliases, per-serial variants and upstream vote scores do not establish new effects, working cheats or verified compatibility".into()],
    }
}

fn parse_cheatdevice_ps2_games(text: &str) -> Vec<(String, Vec<Code>)> {
    fn is_native_code_line(line: &str) -> bool {
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next(), parts.next()) {
            (Some(a), Some(b), None) => {
                a.len() == 8
                    && b.len() == 8
                    && a.bytes().all(|c| c.is_ascii_hexdigit())
                    && b.bytes().all(|c| c.is_ascii_hexdigit())
            }
            _ => false,
        }
    }
    fn parse_game_body(body: &str) -> Vec<Code> {
        fn flush(result: &mut Vec<Code>, description: &str, code: &mut String) {
            if code.is_empty() {
                return;
            }
            result.push(Code {
                ordinal: result.len(),
                description: Some(description.to_owned()),
                code: Some(std::mem::take(code)),
                source_enabled: false,
                verification: "unverified",
                role: "code",
                native_fields: Vec::new(),
                snes_decode: None,
                composition: None,
            });
        }
        let mut out = Vec::new();
        let mut description = "(unnamed original device code)".to_owned();
        let mut pending = String::new();
        for line in body.split_inclusive('\n') {
            let value = line.trim();
            if is_native_code_line(value) {
                pending.push_str(line);
            } else {
                flush(&mut out, &description, &mut pending);
                if !value.is_empty() && !value.starts_with("//") && !value.starts_with('#') {
                    description = value.to_owned();
                }
            }
        }
        flush(&mut out, &description, &mut pending);
        out
    }
    let mut games = Vec::new();
    let mut name: Option<String> = None;
    let mut body = String::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            if let Some(prior) = name.replace(trimmed[1..trimmed.len() - 1].to_owned()) {
                games.push((prior, parse_game_body(&body)));
                body.clear();
            }
        } else if name.is_some() {
            body.push_str(line);
        }
    }
    if let Some(final_name) = name {
        games.push((final_name, parse_game_body(&body)));
    }
    games
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
    known_platforms.insert("msx-family".to_owned());
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

    // CC BY-SA Libretro-derived fork delta: only source blobs *not already
    // present* in our pinned 27,915-file Libretro collection are archived.
    // Combined MSX/MSX2/MSX2+/Turbo R original folders stay in a distinct
    // source-family platform until actual machine compatibility is evidenced.
    if root.join("sources/bithex-libretro-delta.json").exists() {
        let id = "bithex-libretro-delta";
        let extra: SourceManifest =
            serde_json::from_slice(&fs::read(root.join(format!("sources/{id}.json")))?)?;
        if extra.id != id {
            return Err("Mismatched pinned Libretro-fork source manifest".into());
        }
        let mut original_codes = 0usize;
        for item in extra.files {
            if item.upstream_path == "LICENSE" || item.upstream_path == "README.md" {
                continue;
            }
            let source_path = item.upstream_path.strip_prefix("cht/")
                .ok_or("Invalid original Libretro-fork source path")?;
            let (dir, file_name) = source_path.split_once('/')
                .ok_or("Missing original Libretro-fork collection directory")?;
            if !file_name.ends_with(".cht") || file_name.contains('/') {
                return Err("Invalid source-native Libretro-fork filename".into());
            }
            let platform = if matches!(dir,
                "Microsoft - MSX - MSX2 - MSX2P - MSX Turbo R"
                | "Microsoft - MSX - MSX2 - MSX2P - MSX Turbo R (fMSX core)"
            ) {
                "msx-family"
            } else {
                source_directories.get(dir)
                    .ok_or("Unmapped Libretro-fork source collection")?
                    .platform.as_str()
            };
            original_codes += 1;
            if wanted.as_ref().is_some_and(|v| !v.contains(platform)) {
                continue;
            }
            let raw = fs::read(checked_path(&root, &item.archive_path)?)?;
            let mut parsed = parse_cht(&String::from_utf8_lossy(&raw));
            let stem = file_name.trim_end_matches(".cht");
            if platform == "snes" {
                let declared_format = format_hint(stem);
                for code in &mut parsed.codes {
                    if code.role != "code" {
                        continue;
                    }
                    if let Some(source_code) = &code.code {
                        if source_code.contains('+') {
                            // A join has no demonstrated simultaneous execution
                            // semantics and cannot be folded into memory writes.
                            code.composition = Some(SourceComposition::unresolved());
                        } else {
                            code.snes_decode = match declared_format {
                                Some(format) => decode_snes(format, source_code).ok(),
                                None => decode_snes_unlabelled(source_code).ok(),
                            };
                        }
                    }
                }
            }
            let label = title_hint(stem);
            let record = IndexedFile {
                id: format!("{id}:{}", item.upstream_path),
                title_hint: label.clone(),
                candidate_game_key: candidate_game_key(&label),
                identity_confidence: "filename_heuristic_only",
                raw_filename: file_name.to_owned(),
                region_hint: region_hint(stem),
                format_hint: format_hint(stem),
                declared_cheats: parsed.declared_count,
                parse_warnings: parsed.warnings,
                codes: parsed.codes,
                provenance: Provenance {
                    source_id: id.to_owned(),
                    repository: extra.repository.clone(),
                    revision: extra.snapshot_commit.clone(),
                    license: extra.license.clone(),
                    upstream_path: item.upstream_path,
                    archive_path: item.archive_path,
                    git_blob_sha: item.git_blob_sha,
                },
            };
            by_platform.entry(platform.to_owned()).or_default().push(record);
        }
        if original_codes != 1125 {
            return Err(format!(
                "Pinned Libretro-fork source count drift: {original_codes}"
            ).into());
        }
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

    // Entire pinned NXCheats source text collection, including historical
    // single-effect files. Original files remain separate. Grouping is advisory
    // by upstream title-ID/build-ID names, NEVER proof of installed-game identity.
    // The manifest is created by the separate bulk importer workflow.
    if root.join("sources/bad1dea-nxcheats-switch.json").exists() {
        let nx: SourceManifest = serde_json::from_slice(&fs::read(
            root.join("sources/bad1dea-nxcheats-switch.json"),
        )?)?;
        if nx.id != "bad1dea-nxcheats-switch" {
            return Err("Unexpected source ID in pinned NXCheats manifest".into());
        }
        for item in nx.files {
            let parts: Vec<&str> = item.upstream_path.split('/').collect();
            let file_name = parts.last().copied().unwrap_or("");
            let Some(build) = file_name.strip_suffix(".txt") else {
                continue;
            };
            if !parts.contains(&"cheats")
                || build.len() != 16
                || !build.bytes().all(|c| c.is_ascii_hexdigit())
            {
                continue;
            }
            let title_id = parts
                .iter()
                .find_map(|part| {
                    let prefix = part.split_whitespace().next().unwrap_or(part);
                    if prefix.len() == 16 && prefix.bytes().all(|c| c.is_ascii_hexdigit()) {
                        Some(prefix)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    format!(
                        "Missing source-native Switch title ID: {}",
                        item.upstream_path
                    )
                })?;
            if wanted.as_ref().is_some_and(|v| !v.contains("switch")) {
                continue;
            }
            let raw = fs::read(checked_path(&root, &item.archive_path)?)?;
            let decoded = String::from_utf8_lossy(&raw);
            let mut parsed = parse_native_sections(&decoded);
            if std::str::from_utf8(&raw).is_err() {
                parsed
                    .warnings
                    .push("Non-UTF8 source bytes remain preserved in the original archive".into());
            }
            parsed.warnings.push(
                "Legacy one-effect files may repeat complete build packs; no effect deduplication or game verification is inferred".into(),
            );
            let source_label = if parts.first() == Some(&"_old") {
                parts.get(1).copied().unwrap_or("Unlabeled Switch source")
            } else {
                parts[0].split_once(' ').map_or(parts[0], |(_, name)| name)
            };
            let title_hint =
                format!("{source_label} [source title] / title ID {title_id} / build {build}");
            let record = IndexedFile {
                id: format!("{}:{}", nx.id, item.upstream_path),
                title_hint,
                candidate_game_key: format!(
                    "title-id-{}-build-{}",
                    title_id.to_ascii_lowercase(),
                    build.to_ascii_lowercase(),
                ),
                identity_confidence: "source-path-title-and-build-id-unverified",
                raw_filename: item.upstream_path.clone(),
                region_hint: None,
                format_hint: Some("atmosphere"),
                declared_cheats: None,
                parse_warnings: parsed.warnings,
                codes: parsed.codes,
                provenance: Provenance {
                    source_id: nx.id.clone(),
                    repository: nx.repository.clone(),
                    revision: nx.snapshot_commit.clone(),
                    license: nx.license.clone(),
                    upstream_path: item.upstream_path,
                    archive_path: item.archive_path,
                    git_blob_sha: item.git_blob_sha,
                },
            };
            by_platform.entry("switch".into()).or_default().push(record);
        }
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
        "mohasi-ps3-compiled",
        "goldhen",
        "etahen-ps5",
        "rincheat-vita",
        "emucorex-cheatdevice-pnach",
        "admentus-enhancement-codes",
        "mkwcat-gecko-codes",
        "shrub-kar-gecko",
        "cookieplmonster-console-cheat-codes",
        "igor-misc-pcsx2-cheats",
        "aerowidescreen-pcsx2-cheats",
        "salustlab-ps2-mods",
        "fmil95-cvxpacchi-ps2",
        "made-in-slovakia-ratchet-ps2",
        "xan1242-sonic-unleashed-ps2",
        "xan1242-nfsmw-opm-demo-ps2",
        "exceen-mhfu-psp-transmog",
        "kh1fm-ps2-drop-rate",
        "igor-txr-controller-tweaks",
        "saupernova-bt3-60fps",
        "e-the-real-gecko",
        "foxy-dementium-3ds",
        "foxy-kirby-clash-3ds",
        "foxy-terraria-3ds",
        "foxy-ocarina-3ds",
        "foxy-battleminers-3ds",
        "foxy-battleminerz-3ds",
        "foxy-mgs3ds-3ds",
        "foxy-badge-arcade-3ds",
        "foxy-re-revelations-3ds",
        "foxy-fnaf-remake-3ds",
        "foxy-mii-maker-3ds",
        "andshrew-sly2-pnach",
        "wii-zero-gravity-controller",
        "wii-krusty-krab-controller",
        "wii-okami-controller",
        "wii-robinsons-controller",
        "wii-punchout-controller",
        "wii-tetris-party-gecko",
        "wii-sonic-secret-rings-gecko",
        "wii-mario-galaxy-gecko",
        "wii-cars-trilogy-gecko",
        "wii-pikmin-gecko",
        "wii-kirby-dreamland-gecko",
        "wii-super-paper-mario-gecko",
        "wii-pikmin2-gecko",
        "wii-rhythm-heaven-gecko",
        "wii-mario-sports-mix-gecko",
        "wii-barrel-blast-gecko",
        "wii-excite-truck-gecko",
        "wii-mario-power-tennis-gecko",
        "wii-super-sluggers-gecko",
        "wii-galaxy2-gecko",
        "wii-twilight-princess-gecko",
        "wii-wario-land-gecko",
        "wii-country-returns-gecko",
        "wii-strikers-gecko",
        "wii-fluidity-gecko",
        "gamecubing-gecko",
        "two-gifts-melee-random",
        "shaigem-melee-gecko",
        "foxy-isz-item-chat",
        "foxy-isz-wtw-3ds",
        "foxy-isz-first-person",
        "foxy-settings-app-3ds",
        "cracko-mc3ds-spawn-cap",
        "cracko-mc3ds-better-save",
        "xan1242-nfsc-ps3",
        "gc-007-high-fps",
        "foxy-ice-station-z-itemchanger",
        "xan1242-nfsu-ps2",
        "xan1242-gt3-no-interlace",
        "xan1242-gt3-freecam",
        "dolphin-game-cheats",
    ] {
        // The separately dispatched bulk importer creates this optional manifest.
        // The initial implementation commit can still build the existing archive.
        if manifest_id == "mohasi-ps3-compiled"
            && !root.join(format!("sources/{manifest_id}.json")).exists()
        {
            continue;
        }
        let native: SourceManifest =
            serde_json::from_slice(&fs::read(root.join(format!("sources/{manifest_id}.json")))?)?;
        if native.id != manifest_id {
            return Err(format!("Wrong source manifest identity for {manifest_id}").into());
        }
        for item in native.files {
            let original = &item.upstream_path;
            let (platform, format, title) = match manifest_id {
                "igor-misc-pcsx2-cheats"
                | "aerowidescreen-pcsx2-cheats"
                | "salustlab-ps2-mods"
                | "fmil95-cvxpacchi-ps2"
                | "made-in-slovakia-ratchet-ps2"
                | "xan1242-sonic-unleashed-ps2"
                | "xan1242-nfsmw-opm-demo-ps2"
                | "kh1fm-ps2-drop-rate"
                | "igor-txr-controller-tweaks"
                | "saupernova-bt3-60fps"
                | "andshrew-sly2-pnach"
                | "xan1242-nfsu-ps2"
                | "xan1242-gt3-no-interlace"
                | "xan1242-gt3-freecam"
                    if original.ends_with(".pnach") =>
                {
                    ("ps2", "pcsx2-pnach", original.to_owned())
                }
                "e-the-real-gecko"
                    if original.ends_with(".ini")
                        && original.trim_end_matches(".ini").len() == 6 =>
                {
                    // Preserve native six-character Wii title/region IDs as source hints.
                    (
                        "wii",
                        "dolphin-ini",
                        original.trim_end_matches(".ini").to_owned(),
                    )
                }
                "wii-zero-gravity-controller"
                | "wii-krusty-krab-controller"
                | "wii-okami-controller"
                | "wii-robinsons-controller"
                | "wii-punchout-controller"
                | "wii-tetris-party-gecko"
                | "wii-sonic-secret-rings-gecko"
                | "wii-mario-galaxy-gecko"
                | "wii-cars-trilogy-gecko"
                | "wii-pikmin-gecko"
                | "wii-kirby-dreamland-gecko"
                | "wii-super-paper-mario-gecko"
                | "wii-pikmin2-gecko"
                | "wii-rhythm-heaven-gecko"
                | "wii-mario-sports-mix-gecko"
                | "wii-barrel-blast-gecko"
                | "wii-excite-truck-gecko"
                | "wii-mario-power-tennis-gecko"
                | "wii-super-sluggers-gecko"
                | "wii-galaxy2-gecko"
                | "wii-wario-land-gecko"
                    if original.starts_with("codes/") && original.ends_with(".ini") =>
                {
                    let disc = original
                        .rsplit('/')
                        .next()
                        .unwrap_or(original)
                        .trim_end_matches(".ini");
                    if disc.len() != 6 || !disc.chars().all(|c| c.is_ascii_alphanumeric()) {
                        continue;
                    }
                    // The exact original disc ID is only an unverified source hint.
                    ("wii", "dolphin-ini", disc.to_owned())
                }
                "foxy-dementium-3ds"
                | "foxy-kirby-clash-3ds"
                | "foxy-terraria-3ds"
                | "foxy-ocarina-3ds"
                | "foxy-battleminers-3ds"
                | "foxy-battleminerz-3ds"
                | "foxy-mgs3ds-3ds"
                | "foxy-badge-arcade-3ds"
                | "foxy-re-revelations-3ds"
                | "foxy-fnaf-remake-3ds"
                | "foxy-mii-maker-3ds"
                | "foxy-settings-app-3ds"
                | "cracko-mc3ds-spawn-cap"
                | "cracko-mc3ds-better-save"
                    if original.ends_with(".txt")
                        && original.trim_end_matches(".txt").len() == 16
                        && original
                            .trim_end_matches(".txt")
                            .chars()
                            .all(|c| c.is_ascii_hexdigit()) =>
                {
                    // Native title-ID labels only; not independently confirmed game identity.
                    (
                        "3ds",
                        "gateshark-text",
                        original.trim_end_matches(".txt").to_owned(),
                    )
                }
                "wii-fluidity-gecko"
                    if original.starts_with("codes/") && original.ends_with(".txt") =>
                {
                    // WiiWare native text filenames are source hints only, not verified titles.
                    ("wii", "gecko-plain-text", original.to_owned())
                }
                "wii-country-returns-gecko" | "wii-strikers-gecko"
                    if original.starts_with("codes/") && original.ends_with(".ini") =>
                {
                    // Keep native revision suffix in provenance, not game identity.
                    let filename = original.rsplit('/').next().unwrap_or(original);
                    let base = filename.trim_end_matches(".ini");
                    let disc = base.split('_').next().unwrap_or(base);
                    if disc.len() != 6 || !disc.chars().all(|c| c.is_ascii_alphanumeric()) {
                        continue;
                    }
                    ("wii", "dolphin-ini", disc.to_owned())
                }
                "wii-twilight-princess-gecko"
                    if original.starts_with("codes/") && original.ends_with(".ini") =>
                {
                    let filename = original.rsplit('/').next().unwrap_or(original);
                    let disc = filename.split('.').next().unwrap_or(filename);
                    // Game ID is only an unverified hint; revision stays in full source path.
                    if disc.len() != 6 || !disc.chars().all(|c| c.is_ascii_alphanumeric()) {
                        continue;
                    }
                    ("wii", "dolphin-ini", disc.to_owned())
                }
                "shaigem-melee-gecko"
                    if original.starts_with("output/codes_") && original.ends_with(".txt") =>
                {
                    // Original source labels effects under $Gecko-style headings.
                    // No verified game revision association is synthesized.
                    ("gamecube", "gecko-plain-text", original.to_owned())
                }
                "two-gifts-melee-random"
                    if original.starts_with("dist/") && original.ends_with(".ini") =>
                {
                    // Both original distribution filenames contain the same Gecko blob.
                    // GALE01 is a source-provided title ID, not a verified identity.
                    ("gamecube", "dolphin-ini", "GALE01".to_owned())
                }
                "gamecubing-gecko" if original == "Geckos.txt" => {
                    // Multi-game original author source; do not invent a common game ID.
                    ("gamecube", "gecko-plain-text", original.to_owned())
                }
                "foxy-isz-wtw-3ds" if original == "No CopyRight ISZ .WTW.txt" => {
                    // Author-titled Beta file with explicit reuse statement in original text.
                    // No verified retail title/build ID is implied.
                    ("3ds", "gateshark-text", original.to_owned())
                }
                "foxy-isz-first-person" if original == "First person Mode ISZ -Beta.txt" => {
                    // Author-published beta title only; no verified 3DS build identifier.
                    ("3ds", "gateshark-text", original.to_owned())
                }
                "xan1242-nfsc-ps3" if original == "patches/BLUS30016/Patches.md" => {
                    // Source markdown retains individual PowerPC patch effects and code fences.
                    // Source BLUS30016 is not a verified dump/build identity.
                    ("ps3", "ps3-ppc-markdown", original.to_owned())
                }
                "foxy-isz-item-chat" if original == "Item Id Chat (Test).txt" => {
                    // Author's test code has no verified retail title/build identifier.
                    ("3ds", "gateshark-text", original.to_owned())
                }
                "foxy-ice-station-z-itemchanger" if original == "ISZ Item Changer Slot1.txt" => {
                    // Original author-titled code file, no verified game build/title ID.
                    ("3ds", "gateshark-text", original.to_owned())
                }
                "gc-007-high-fps" if original == "007 Agent under fire (GW7E69)" => {
                    // Source includes distinct REV0/REV1 and FPS alternatives in plain text.
                    ("gamecube", "gecko-plain-text", original.to_owned())
                }
                "foxy-re-revelations-3ds" if original == "ReR Fps Codes.txt" => {
                    // Native author-supplied auxiliary title; do not invent a game/build ID.
                    ("3ds", "gateshark-text", original.to_owned())
                }
                "dolphin-game-cheats"
                    if original.starts_with("Data/Sys/GameSettings/")
                        && original.ends_with(".ini") =>
                {
                    let name = original
                        .rsplit('/')
                        .next()
                        .unwrap_or(original)
                        .trim_end_matches(".ini");
                    // Dolphin's six-character source game-ID convention is a
                    // source hint, not a confirmed disc/region/build identity.
                    // Unknown prefixes remain archived but unassigned.
                    if name.len() != 6 {
                        continue;
                    }
                    let system = match name.chars().next() {
                        Some('G' | 'D') => "gamecube",
                        Some('R' | 'S' | 'W' | 'H') => "wii",
                        _ => continue,
                    };
                    (system, "dolphin-ini", name.to_owned())
                }
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
                "exceen-mhfu-psp-transmog"
                    if original == "archive/cheat.db"
                        || original == "archive/universal_transmog.txt" =>
                {
                    // Native CWCheat cheat.db; upstream _S ULJM-05500 is a source hint
                    // for an FUComplete mod, not an independently verified game build.
                    ("psp", "psp-cwcheat-ini", original.to_owned())
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
                "emucorex-cheatdevice-pnach"
                    if original.starts_with("files/root670-cheatdevice/")
                        && original.ends_with(".pnach") =>
                {
                    (
                        "ps2",
                        "pcsx2-pnach",
                        original
                            .trim_start_matches("files/root670-cheatdevice/")
                            .to_owned(),
                    )
                }
                "rincheat-vita"
                    if original.starts_with("cheats_db/") && original.ends_with(".txt") =>
                {
                    (
                        "vita",
                        "vita-rincheat-memory",
                        original.trim_start_matches("cheats_db/").to_owned(),
                    )
                }
                "rincheat-vita"
                    if original.starts_with("SE_cheats_db/") && original.ends_with(".lua") =>
                {
                    (
                        "vita",
                        "vita-rincheat-save-lua",
                        original.trim_start_matches("SE_cheats_db/").to_owned(),
                    )
                }
                "mohasi-ps3-compiled"
                    if original.starts_with("compiled/") && original.ends_with(".txt") =>
                {
                    (
                        "ps3",
                        "ps3-mohasi-compiled",
                        original.trim_start_matches("compiled/").to_owned(),
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
                "etahen-ps5" if original.starts_with("json/") && original.ends_with(".json") => (
                    "ps5",
                    "goldhen-json",
                    original
                        .trim_start_matches("json/")
                        .trim_end_matches(".json")
                        .to_owned(),
                ),
                "etahen-ps5" if original.starts_with("shn/") && original.ends_with(".shn") => (
                    "ps5",
                    "goldhen-shn",
                    original.trim_start_matches("shn/").to_owned(),
                ),
                "etahen-ps5"
                    if original.ends_with(".mc4")
                        && (original.starts_with("shn/") || original.starts_with("mc4/")) =>
                {
                    (
                        "ps5",
                        "goldhen-mc4",
                        original
                            .split_once('/')
                            .map(|(_, p)| p)
                            .unwrap_or(original)
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
                "shrub-kar-gecko"
                    if original.starts_with("codes/") && original.ends_with(".md") =>
                {
                    // Nine original Kirby Air Ride Gecko Markdown documents.
                    // Source README labels US and possible PAL; none independently verified.
                    ("gamecube", "gecko-markdown", original.to_owned())
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
                "vita-rincheat-memory" | "vita-rincheat-save-lua" => cheatarium_index::ParsedCheats {
                    declared_count: None,
                    // Source-exact opaque text; not an invented count of individual cheats.
                    codes: vec![Code {
                        ordinal: 0,
                        description: Some("Complete original Vita cheat source; source program only".to_owned()),
                        code: Some(decoded.clone()),
                        source_enabled: false,
                        verification: "unverified",
                        role: "code",
                        native_fields: Vec::new(),
                        snes_decode: None,
                        composition: None,
                    }],
                    warnings: vec!["Original Vita runtime/save-edit source is opaque; do not infer game-build compatibility or activate automatically".to_owned()],
                },
                "ps3-mohasi-compiled" => parse_mohasi_ps3_compiled(&decoded),
                "artemis-ncl" => parse_artemis_ncl(&decoded),
                "goldhen-json" => parse_goldhen_json(&decoded),
                "goldhen-mc4" => parse_goldhen_mc4(&decoded),
                "goldhen-shn" => parse_goldhen_shn(&decoded),
                "goldhen-mc4-xml" => parse_goldhen_shn(&decoded),
                "gecko-plain-text" if manifest_id == "wii-fluidity-gecko" => {
                    parse_gecko_flat(&decoded)
                }
                "dolphin-ini" if manifest_id == "wii-country-returns-gecko" => {
                    parse_gecko_flat(&decoded)
                }
                "dolphin-ini" | "gecko-plain-text" => parse_gecko_ini(&decoded),
                "gecko-markdown" | "ps3-ppc-markdown" => parse_gecko_markdown(&decoded),
                "pcsx2-pnach" | "ps1-native-cht" | "gateshark-text" => {
                    parse_native_sections(&decoded)
                }
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
            } else if manifest_id == "mohasi-ps3-compiled" {
                decoded.lines().find_map(|line| {
                    line.strip_prefix("# ")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                })
            } else if manifest_id == "emucorex-cheatdevice-pnach" {
                decoded.lines().find_map(|line| {
                    line.strip_prefix("gametitle=")
                        .map(str::trim)
                        .filter(|label| !label.is_empty())
                        .map(str::to_owned)
                })
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

    // Entire original root670 CheatDevicePS2 database: 1 source file, 1,483
    // source-declared game sections. Each section is indexed separately without
    // manufacturing ROM/build identity or treating individual memory writes
    // as known-good. Original bytes and historic code-author uncertainty remain.
    if wanted
        .as_ref()
        .is_none_or(|platforms| platforms.contains("ps2"))
    {
        let id = "root670-cheatdevice-ps2";
        let manifest_path = root.join(format!("sources/{id}.json"));
        let manifest: SourceManifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
        if manifest.id != id {
            return Err("Mismatched source manifest for PS2 CheatDevice".into());
        }
        let db = manifest
            .files
            .iter()
            .find(|x| x.upstream_path == "CheatDatabase.txt")
            .ok_or("Missing indexed CheatDevicePS2 database source")?;
        let bytes = fs::read(checked_path(&root, &db.archive_path)?)?;
        let text = String::from_utf8_lossy(&bytes);
        let games = parse_cheatdevice_ps2_games(&text);
        if games.len() != 1483 {
            return Err(
                format!("Pinned CheatDevicePS2 game count mismatch: {}", games.len()).into(),
            );
        }
        for (index, (title, codes)) in games.into_iter().enumerate() {
            if codes.is_empty() {
                return Err(format!(
                    "Missing raw device code lines in CheatDevicePS2 game {}",
                    index + 1
                )
                .into());
            }
            let record = IndexedFile {
                id: format!("{id}:CheatDatabase.txt#game-{:04}", index + 1),
                title_hint: title,
                candidate_game_key: format!("{id}:game-{:04}", index + 1),
                identity_confidence: "source-declared-game-name-only",
                raw_filename: format!("CheatDatabase.txt#game-{:04}", index + 1),
                region_hint: None,
                format_hint: Some("cheatdevice-ps2-raw"),
                declared_cheats: None,
                parse_warnings: vec![
                    "Device-code groups are source text, not executed/tested cheats; release/region compatibility unknown".to_owned(),
                ],
                codes,
                provenance: Provenance {
                    source_id: manifest.id.clone(),
                    repository: manifest.repository.clone(),
                    revision: manifest.snapshot_commit.clone(),
                    license: manifest.license.clone(),
                    upstream_path: db.upstream_path.clone(),
                    archive_path: db.archive_path.clone(),
                    git_blob_sha: db.git_blob_sha.clone(),
                },
            };
            by_platform
                .entry("ps2".to_owned())
                .or_default()
                .push(record);
        }
    }

    // Import all source-native game sections from the four pinned ZIP packs.
    // Original ZIP blobs and exact extracted member bytes remain separately
    // archived; game names, mastercodes, region hints and revisions are source
    // claims, never verified ROM matches or activation authorizations.
    if wanted
        .as_ref()
        .is_none_or(|platforms| platforms.contains("ps2"))
    {
        let id = "israpps-cheatdevice-ps2-extras";
        let manifest: SourceManifest =
            serde_json::from_slice(&fs::read(root.join(format!("sources/{id}.json")))?)?;
        if manifest.id != id {
            return Err("Mismatched PS2 ZIP source manifest".into());
        }
        let mut count_sources = 0usize;
        for item in manifest.files {
            let original = &item.upstream_path;
            let (format, region) = if original.starts_with("zip-members/NTSC_CHEATS.zip/")
                && original.to_ascii_lowercase().ends_with(".cht")
            {
                ("ps2-zip-native-ntsc", Some("NTSC".to_owned()))
            } else if original.starts_with("zip-members/PAL_CHEATS.zip/")
                && original.to_ascii_lowercase().ends_with(".cht")
            {
                ("ps2-zip-native-pal", Some("PAL".to_owned()))
            } else if original.starts_with("zip-members/MASTERCODES.zip/")
                && original.to_ascii_lowercase().ends_with(".txt")
            {
                ("ps2-zip-mastercode", None)
            } else if original.starts_with("zip-members/WIDESCREEN.zip/")
                && original.to_ascii_lowercase().ends_with(".txt")
            {
                ("ps2-zip-widescreen", None)
            } else {
                continue;
            };
            count_sources += 1;
            let bytes = fs::read(checked_path(&root, &item.archive_path)?)?;
            let decoded = String::from_utf8_lossy(&bytes);
            let mut groups = parse_cheatdevice_ps2_games(&decoded);
            // Some complete source pack members have no quoted game title:
            // their filename remains the only supplied identity evidence.
            // Do not discard otherwise valid original codes or invent a title.
            let filename_only = groups.is_empty();
            if filename_only {
                let filename = original.rsplit('/').next().unwrap_or(original);
                let label = filename.replace('_', " ").replace('"', "'");
                groups = parse_cheatdevice_ps2_games(&format!("\"{label}\"\n{decoded}"));
            }
            if groups.is_empty() {
                return Err(format!("Unindexable original PS2 ZIP member: {original}").into());
            }
            for (section, (title, codes)) in groups.into_iter().enumerate() {
                let source_name = original.rsplit('/').next().unwrap_or(original);
                let record = IndexedFile {
                    id: format!("{id}:{original}#section-{:04}", section + 1),
                    title_hint: title,
                    candidate_game_key: format!("{id}:{original}#section-{:04}", section + 1),
                    identity_confidence: if filename_only {
                        "source-file-name-only"
                    } else {
                        "source-declared-game-name-only"
                    },
                    raw_filename: source_name.to_owned(),
                    region_hint: region.clone(),
                    format_hint: Some(format),
                    declared_cheats: None,
                    parse_warnings: if filename_only {
                        vec!["Original lacks quoted game-title header; filename is not a verified game identity, build or compatible code".to_owned()]
                    } else {
                        vec!["Pinned CheatDevice ZIP source text; compatibility/effects not gameplay-verified, including required mastercode conditions".to_owned()]
                    },
                    codes,
                    provenance: Provenance {
                        source_id: manifest.id.clone(),
                        repository: manifest.repository.clone(),
                        revision: manifest.snapshot_commit.clone(),
                        license: manifest.license.clone(),
                        upstream_path: item.upstream_path.clone(),
                        archive_path: item.archive_path.clone(),
                        git_blob_sha: item.git_blob_sha.clone(),
                    },
                };
                by_platform
                    .entry("ps2".to_owned())
                    .or_default()
                    .push(record);
            }
        }
        if count_sources != 7243 {
            return Err(format!(
                "Pinned CheatDevice ZIP index source count mismatch: {count_sources}"
            )
            .into());
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

    #[test]
    fn converted_ps3_code_parser_preserves_origins_and_never_promotes_scores() {
        let input = "# Example (source-declared)\nname: Infinite Health\n\
author: original creator\nscore BCES00001 01.00: 80 2+ 0-\nmode: once\n\
w32 0010ABCD 00000064\n\nname: Special\n\
author: another author\nmode: once\naob 0123 4567\n";
        let parsed = parse_mohasi_ps3_compiled(input);
        assert_eq!(parsed.codes.len(), 2);
        assert_eq!(parsed.codes[0].ordinal, 0);
        assert_eq!(
            parsed.codes[0].description.as_deref(),
            Some("Infinite Health")
        );
        assert!(parsed.codes[0]
            .code
            .as_ref()
            .unwrap()
            .contains("score BCES00001"));
        assert!(parsed.codes[0]
            .code
            .as_ref()
            .unwrap()
            .contains("author: original creator"));
        assert!(parsed.codes[1]
            .code
            .as_ref()
            .unwrap()
            .contains("aob 0123 4567"));
        assert!(parsed.codes.iter().all(|c| c.verification == "unverified"));
        assert!(parsed.codes.iter().all(|c| !c.source_enabled));
    }

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
