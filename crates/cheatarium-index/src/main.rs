use cheatarium_codecs::decode_snes;
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
            if let Some(device_format) = format_hint(stem) {
                for code in &mut parsed.codes {
                    if code.role != "code" {
                        continue;
                    }
                    if let Some(source_code) = &code.code {
                        // Reject any wildcard or malformed compound as a whole.
                        code.snes_decode = decode_snes(device_format, source_code).ok();
                    }
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
