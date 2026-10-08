use cheatarium_codecs::{decode_snes, decode_snes_unlabelled};
use cheatarium_index::{candidate_game_key, format_hint, parse_cht, region_hint, title_hint, Code};
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
        identity_rule: "unverified filename grouping; keys are not ROM or edition identities",
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
        let description_variants = occurrences.iter()
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
    if let Some(ref filter) = wanted {
        for id in filter {
            if !known_platforms.contains(id) {
                return Err(format!("Unrecognized platform requested: {id}").into());
            }
        }
    }

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
        if system.platform == "snes" {
            let declared_format = format_hint(stem);
            for code in &mut parsed.codes {
                if code.role != "code" {
                    continue;
                }
                if let Some(source_code) = &code.code {
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
            id: format!("{}:{}", manifest.id, item.upstream_path),
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

    fs::create_dir_all(&out)?;
    fs::create_dir_all(out.join("games"))?;
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
            game_identity_rule: "filename-derived suggestion, not verified ROM identity",
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
        assert_eq!(index.groups[0].occurrences[1].description.as_deref(),
            Some("An unrelated source description"));
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
