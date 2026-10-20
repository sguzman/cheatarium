//! Read-only lookup of Cheatarium's own human-curated game records.
//!
//! These are repository-native source-backed codes, not an emulator
//! installation interface or a replacement for the full source bundles.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_GAME_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct CuratedGame {
    schema_version: u32,
    platform: String,
    game: Game,
    editions: Vec<Edition>,
    cheats: Vec<Cheat>,
}
#[derive(Debug, Deserialize)]
struct Game {
    id: String,
    title: String,
    #[serde(default)]
    title_id: Option<String>,
}
#[derive(Debug, Deserialize, Clone, Serialize)]
struct Edition {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    build_id: Option<String>,
    region: String,
    revision: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Cheat {
    id: String,
    name: String,
    kind: String,
    #[serde(default)]
    notes: Option<String>,
    variants: Vec<Variant>,
}
#[derive(Debug, Deserialize)]
struct Variant {
    edition: String,
    format: String,
    code: String,
    status: String,
    #[serde(default)]
    notes: Option<String>,
    source: Provenance,
}
#[derive(Debug, Deserialize, Clone, Serialize)]
struct Provenance {
    source: String,
    archive_path: String,
}
#[derive(Debug, Serialize)]
struct Match {
    platform: String,
    game_id: String,
    game_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title_id: Option<String>,
    edition: Edition,
    cheat_id: String,
    name: String,
    kind: String,
    format: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}
#[derive(Default)]
struct Options {
    root: PathBuf,
    platform: Option<String>,
    title: Option<String>,
    title_id: Option<String>,
    effect: Option<String>,
    build_id: Option<String>,
    source_id: Option<String>,
    with_code: bool,
    coverage: bool,
    only_missing: bool,
    json: bool,
    limit: usize,
    offset: usize,
}
impl Options {
    fn defaults() -> Self {
        Self {
            root: PathBuf::from("."),
            limit: 25,
            ..Self::default()
        }
    }
}

fn clean_directory(path: &Path) -> Result<bool, ErrBox> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(
                    format!("Curated path must not be a symlink: {}", path.display()).into(),
                );
            }
            Ok(meta.is_dir())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn sorted_directories(path: &Path) -> Result<Vec<PathBuf>, ErrBox> {
    if !clean_directory(path)? {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if clean_directory(&p)? {
            paths.push(p);
        }
    }
    paths.sort();
    Ok(paths)
}

fn build_matches(edition: &Edition, requested: Option<&str>) -> bool {
    requested.is_none_or(|build| {
        edition
            .build_id
            .as_deref()
            .is_some_and(|id| id.eq_ignore_ascii_case(build))
    })
}
fn includes(haystack: &str, needle: Option<&str>) -> bool {
    needle.is_none_or(|text| haystack.to_lowercase().contains(&text.to_lowercase()))
}

fn search(opts: &Options) -> Result<Vec<Match>, ErrBox> {
    let top = opts.root.join("curated");
    if !clean_directory(&top)? {
        return Err(format!("Curated collection missing: {}", top.display()).into());
    }
    let mut results = Vec::new();
    for platform in sorted_directories(&top)? {
        let platform_id = platform
            .file_name()
            .and_then(|p| p.to_str())
            .ok_or("Invalid platform name")?;
        if opts.platform.as_deref().is_some_and(|p| p != platform_id) {
            continue;
        }
        for game_dir in sorted_directories(&platform)? {
            let file = game_dir.join("cheats.json");
            let meta = match fs::symlink_metadata(&file) {
                Ok(meta) => meta,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > MAX_GAME_BYTES {
                return Err(format!("Invalid curated file: {}", file.display()).into());
            }
            let document: CuratedGame = serde_json::from_slice(&fs::read(&file)?)?;
            if document.schema_version != 1
                || document.platform != platform_id
                || game_dir.file_name().and_then(|p| p.to_str()) != Some(document.game.id.as_str())
            {
                return Err(format!("Curated identity mismatch: {}", file.display()).into());
            }
            if opts
                .title_id
                .as_deref()
                .is_some_and(|tid| document.game.title_id.as_deref() != Some(tid))
            {
                continue;
            }
            if !includes(&document.game.title, opts.title.as_deref())
                && !includes(&document.game.id, opts.title.as_deref())
            {
                continue;
            }
            let editions: BTreeMap<&str, &Edition> = document
                .editions
                .iter()
                .map(|e| (e.id.as_str(), e))
                .collect();
            if editions.len() != document.editions.len() {
                return Err(format!("Duplicate curated edition: {}", file.display()).into());
            }
            for cheat in document.cheats {
                if !includes(&cheat.name, opts.effect.as_deref())
                    && !includes(&cheat.id, opts.effect.as_deref())
                {
                    continue;
                }
                for variant in cheat.variants {
                    let edition = editions.get(variant.edition.as_str()).ok_or_else(|| {
                        format!("Unknown edition in curated record {}", file.display())
                    })?;
                    if !build_matches(edition, opts.build_id.as_deref())
                        || opts
                            .source_id
                            .as_deref()
                            .is_some_and(|source| source != variant.source.source)
                    {
                        continue;
                    }
                    if !variant.source.archive_path.starts_with("archive/")
                        || variant
                            .source
                            .archive_path
                            .split('/')
                            .any(|s| s == ".." || s == "." || s.is_empty())
                    {
                        return Err("Unsafe archive reference in curated record".into());
                    }
                    let notes = match (cheat.notes.as_ref(), variant.notes.as_ref()) {
                        (Some(a), Some(b)) => Some(format!("{a} {b}")),
                        (Some(a), None) => Some(a.clone()),
                        (None, Some(b)) => Some(b.clone()),
                        (None, None) => None,
                    };
                    results.push(Match {
                        platform: platform_id.to_owned(),
                        game_id: document.game.id.clone(),
                        game_title: document.game.title.clone(),
                        title_id: document.game.title_id.clone(),
                        edition: (*edition).clone(),
                        cheat_id: cheat.id.clone(),
                        name: cheat.name.clone(),
                        kind: cheat.kind.clone(),
                        format: variant.format,
                        status: variant.status,
                        notes,
                        provenance: variant.source,
                        code: opts.with_code.then_some(variant.code),
                    });
                }
            }
        }
    }
    Ok(results)
}

fn coverage_rows(opts: &Options) -> Result<Vec<serde_json::Value>, ErrBox> {
    if opts.platform.as_deref().is_some_and(|p| p != "switch") {
        return Ok(Vec::new());
    }
    if opts.source_id.is_some() || opts.with_code {
        return Err("Coverage metadata does not support --source-id or --with-code".into());
    }
    let file = opts
        .root
        .join("research/switch-exact-build-coverage-v1.json");
    let meta = fs::symlink_metadata(&file)?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > MAX_GAME_BYTES {
        return Err("Invalid Switch coverage report file".into());
    }
    let data: serde_json::Value = serde_json::from_slice(&fs::read(file)?)?;
    if data["format"] != "cheatarium-switch-exact-build-coverage-v1" {
        return Err("Unexpected Switch coverage format".into());
    }
    let mut rows = Vec::new();
    for game in data["games"]
        .as_array()
        .ok_or("Missing Switch coverage games")?
    {
        let game_title = game["title"].as_str().ok_or("Missing game title")?;
        let game_id = game["title_id"].as_str().ok_or("Missing game Title ID")?;
        let game_build = game["build_id"].as_str().ok_or("Missing Build ID")?;
        if opts.title_id.as_deref().is_some_and(|tid| tid != game_id)
            || !(includes(game_title, opts.title.as_deref())
                || includes(game_id, opts.title.as_deref()))
            || opts.build_id.as_deref().is_some_and(|b| b != game_build)
        {
            continue;
        }
        for section in game["sections"].as_array().ok_or("Missing section list")? {
            let heading = section["heading"]
                .as_str()
                .ok_or("Missing source heading")?;
            let state = section["state"].as_str().ok_or("Missing archival state")?;
            if !includes(heading, opts.effect.as_deref())
                || opts.only_missing && state != "linked_only_no_public_code_body"
            {
                continue;
            }
            rows.push(serde_json::json!({
                "game_title": game_title,
                "title_id": game_id,
                "build_id": game_build,
                "heading": heading,
                "state": state,
                "curated_cheat_id": section["curated_cheat_id"],
                "archive_path": section["archive_path"],
                "source": game["source"]
            }));
        }
    }
    Ok(rows)
}

fn show_coverage(opts: &Options) -> Result<(), ErrBox> {
    let all = coverage_rows(opts)?;
    let count = all.len();
    let missing = all
        .iter()
        .filter(|row| row["state"] == "linked_only_no_public_code_body")
        .count();
    let returned: Vec<_> = all.into_iter().skip(opts.offset).take(opts.limit).collect();
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "cheatarium.curated_coverage.v1",
                "read_only": true,
                "metadata_only_for_missing_codes": true,
                "active_emulator_modified": false,
                "matched_sections": count,
                "archived_sections": count - missing,
                "source_only_sections": missing,
                "offset": opts.offset,
                "returned": returned.len(),
                "has_more": opts.offset.saturating_add(returned.len()) < count,
                "results": returned
            }))?
        );
    } else {
        println!(
            "{} exact-build sections: {} archived, {} source-only; none activated",
            count,
            count - missing,
            missing
        );
        for row in returned {
            println!(
                "{} | {} | {} | {}",
                row["game_title"].as_str().unwrap_or("Unknown game"),
                row["build_id"].as_str().unwrap_or("?"),
                row["heading"].as_str().unwrap_or("?"),
                row["state"].as_str().unwrap_or("?")
            );
        }
    }
    Ok(())
}

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), ErrBox> {
    let mut opts = Options::defaults();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                opts.root = PathBuf::from(args.next().ok_or("--root requires a directory")?)
            }
            "--platform" => opts.platform = Some(args.next().ok_or("--platform requires a value")?),
            "--title" => opts.title = Some(args.next().ok_or("--title requires text")?),
            "--title-id" => {
                let value = args.next().ok_or("--title-id requires a 16-digit hex ID")?;
                if value.len() != 16 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("--title-id must be 16 hexadecimal digits".into());
                }
                opts.title_id = Some(value.to_ascii_uppercase());
            }
            "--effect" => opts.effect = Some(args.next().ok_or("--effect requires text")?),
            "--source-id" => opts.source_id = Some(args.next().ok_or("--source-id requires text")?),
            "--build-id" => {
                let value = args.next().ok_or("--build-id requires a 16-digit hex ID")?;
                if value.len() != 16 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("--build-id must be 16 hexadecimal digits".into());
                }
                opts.build_id = Some(value.to_ascii_uppercase());
            }
            "--with-code" => opts.with_code = true,
            "--coverage" => opts.coverage = true,
            "--only-missing" => opts.only_missing = true,
            "--json" => opts.json = true,
            "--offset" => {
                opts.offset = args.next().ok_or("--offset requires an integer")?.parse()?
            }
            "--limit" => {
                opts.limit = args.next().ok_or("--limit requires an integer")?.parse()?;
                if !(1..=200).contains(&opts.limit) {
                    return Err("--limit must be between 1 and 200".into());
                }
            }
            "--help" => {
                println!("Usage: cheatarium-query curated [--root .] [--platform switch] [--title Mario] [--title-id 0100BC0018138000] [--effect Health] [--build-id E968832CADE2AD7C] [--source-id sharkive] [--with-code] [--coverage --only-missing] [--offset 0] [--limit 25] [--json]");
                return Ok(());
            }
            _ => return Err(format!("Unknown curated option: {arg}").into()),
        }
    }
    if opts.coverage {
        return show_coverage(&opts);
    }
    if opts.only_missing {
        return Err("--only-missing requires --coverage".into());
    }
    let rows = search(&opts)?;
    let total = rows.len();
    let result: Vec<_> = rows
        .into_iter()
        .skip(opts.offset)
        .take(opts.limit)
        .collect();
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "cheatarium.curated_search.v1",
                "read_only": true,
                "source_originals_preserved": true,
                "gameplay_verified_by_search": false,
                "active_emulator_modified": false,
                "code_visible": opts.with_code,
                "total": total,
                "offset": opts.offset,
                "returned": result.len(),
                "has_more": opts.offset.saturating_add(result.len()) < total,
                "results": result
            }))?
        );
    } else {
        println!("{total} curated code variants match; no cheats activated");
        for hit in result {
            println!(
                "{} | {} | {} | {} [{}] — {}",
                hit.platform,
                hit.game_title,
                hit.edition.build_id.as_deref().unwrap_or(&hit.edition.id),
                hit.name,
                hit.status,
                hit.provenance.source
            );
            if let Some(code) = hit.code {
                println!("{code}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repository() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
    #[test]
    fn retrieves_exact_rpg_build_without_leaking_code_by_default() {
        let opts = Options {
            root: repository(),
            platform: Some("switch".into()),
            title: Some("Mario RPG".into()),
            build_id: Some("E968832CADE2AD7C".into()),
            ..Options::defaults()
        };
        let results = search(&opts).expect("repository curated collection");
        assert!(results.len() >= 53);
        assert!(results
            .iter()
            .all(|r| r.title_id.as_deref() == Some("0100BC0018138000")));
        assert!(results.iter().all(|r| r.code.is_none()));
        assert!(results.iter().all(|r| r.status == "unverified"));
        assert!(results
            .iter()
            .all(|r| r.edition.build_id.as_deref() == Some("E968832CADE2AD7C")));
        let with_code = search(&Options {
            with_code: true,
            ..opts
        })
        .unwrap();
        assert_eq!(with_code.len(), results.len());
        assert!(with_code
            .iter()
            .all(|r| r.code.as_deref().is_some_and(|c| !c.is_empty())));
    }
    #[test]
    fn prevents_mislabeling_other_game_builds() {
        let results = search(&Options {
            root: repository(),
            platform: Some("switch".into()),
            title: Some("Mario Kart".into()),
            build_id: Some("E968832CADE2AD7C".into()),
            ..Options::defaults()
        })
        .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn exact_build_coverage_reports_all_six_games_archived() {
        let opts = Options {
            root: repository(),
            platform: Some("switch".into()),
            coverage: true,
            ..Options::defaults()
        };
        let rows = coverage_rows(&opts).expect("six-title coverage");
        assert_eq!(rows.len(), 174);
        assert!(rows
            .iter()
            .all(|row| row["state"] == "archived_code_exact_match"));
        let skyward = coverage_rows(&Options {
            title: Some("Skyward Sword".into()),
            ..Options {
                root: repository(),
                platform: Some("switch".into()),
                coverage: true,
                ..Options::defaults()
            }
        })
        .unwrap();
        assert_eq!(skyward.len(), 47);
        assert!(skyward
            .iter()
            .all(|row| row["build_id"] == "D824D4B1BBD45D03"));
        let missing = coverage_rows(&Options {
            only_missing: true,
            ..opts
        })
        .unwrap();
        assert!(missing.is_empty());
    }
}
