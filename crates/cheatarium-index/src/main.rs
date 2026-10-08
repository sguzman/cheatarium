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
    source_files: usize,
    code_fields: usize,
    warnings: usize,
}

#[derive(Serialize)]
struct Catalog {
    schema_version: u32,
    format: &'static str,
    matching_policy: &'static str,
    bundles: Vec<CatalogItem>,
}

fn checked_path(root: &Path, rel: &str) -> Result<PathBuf> {
    let path = Path::new(rel);
    if path.components().any(|c| !matches!(c, Component::Normal(_))) {
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
                let s = args.next().ok_or("--systems requires comma-separated platforms")?;
                wanted = Some(s.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect());
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
    let mapping: LibretroMapping = serde_json::from_slice(
        &fs::read(root.join("platforms/libretro-mapping.json"))?,
    )?;
    let manifest: SourceManifest = serde_json::from_slice(
        &fs::read(root.join("sources/libretro-database.json"))?,
    )?;
    if manifest.id != "libretro-database" {
        return Err("Expected Libretro source manifest".into());
    }
    let mut source_directories = HashMap::<String, System>::new();
    let mut known_platforms = BTreeSet::new();
    for system in mapping.systems {
        if source_directories.insert(system.source_directory.clone(), system.clone()).is_some() {
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
        if wanted.as_ref().is_some_and(|v| !v.contains(&system.platform)) {
            continue;
        }
        let path = checked_path(&root, &item.archive_path)?;
        let content = fs::read(&path)?;
        let parsed = parse_cht(&String::from_utf8_lossy(&content));
        let stem = filename.strip_suffix(".cht").unwrap_or(filename);
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
        by_platform.entry(system.platform.clone()).or_default().push(record);
    }

    fs::create_dir_all(&out)?;
    let mut catalog = Vec::new();
    for (platform, mut records) in by_platform {
        records.sort_by(|a, b| a.id.cmp(&b.id));
        let code_fields: usize = records.iter().flat_map(|x| &x.codes).filter(|x| x.code.is_some()).count();
        let warnings: usize = records.iter().map(|x| x.parse_warnings.len()).sum();
        let source_files = records.len();
        let filename = format!("{platform}.json.gz");
        let bundle = Bundle {
            schema_version: 1,
            platform: platform.clone(),
            game_identity_rule: "filename-derived suggestion, not verified ROM identity",
            compatibility_rule: "never auto-apply a code without confirmed release/build compatibility",
            records,
        };
        let json = serde_json::to_vec(&bundle)?;
        let target = out.join(&filename);
        let mut gz = GzBuilder::new().mtime(0).write(File::create(target)?, Compression::default());
        gz.write_all(&json)?;
        gz.finish()?;
        println!("{platform}: {source_files} source files, {code_fields} codes, {warnings} warnings");
        catalog.push(CatalogItem {
            platform,
            artifact: filename,
            source_files,
            code_fields,
            warnings,
        });
    }
    let catalog = Catalog {
        schema_version: 1,
        format: "cheatarium-index-v1",
        matching_policy: "candidate titles only; no ROM hashes or verified execution",
        bundles: catalog,
    };
    fs::write(out.join("catalog.json"), serde_json::to_vec_pretty(&catalog)?)?;
    println!("Wrote {} index bundles", catalog.bundles.len());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cheatarium-index: {error}");
        std::process::exit(1);
    }
}
