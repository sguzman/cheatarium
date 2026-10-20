//! Search source-native Nintendo Switch sections by exact Title ID and Build ID.
//! This is the existing read-only source archive, not human-reviewed curation.
use cheatarium_client::{load_platform, Bundle};
use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::path::PathBuf;

type ErrBox = Box<dyn Error + Send + Sync>;

struct Options {
    db: PathBuf,
    title_id: String,
    build_id: Option<String>,
    source_id: Option<String>,
    effect: Option<String>,
    with_code: bool,
    json: bool,
    limit: usize,
    offset: usize,
}
#[derive(Serialize)]
struct NativeHit {
    title_id: String,
    build_id: String,
    source_id: String,
    source_record_id: String,
    archive_path: String,
    upstream_path: String,
    revision: String,
    heading: Option<String>,
    ordinal: usize,
    verification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

fn is_hex16(text: &str) -> bool {
    text.len() == 16 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Only recognized source-native layouts, never arbitrary path substring hits.
fn archive_identity(source: &str, path: &str) -> Option<(String, String)> {
    let parts: Vec<_> = path.split('/').collect();
    match source {
        "sharkive" if parts.len() == 5 &&
            parts[0] == "archive" && parts[1] == "sharkive" && parts[2] == "switch" => {
            let build = parts[4].strip_suffix(".txt")?;
            if !is_hex16(parts[3]) || !is_hex16(build) { return None; }
            Some((parts[3].to_ascii_uppercase(), build.to_ascii_uppercase()))
        }
        "bad1dea-nxcheats-switch" if parts.len() >= 6 &&
            parts[0] == "archive" && parts[1] == "bad1dea-nxcheats-switch" &&
            parts[parts.len() - 2] == "cheats" => {
            let build = parts.last()?.strip_suffix(".txt")?;
            if !is_hex16(build) { return None; }
            let title = parts[2..parts.len() - 2].iter()
                .find_map(|name| {
                    let prefix = name.get(..16)?;
                    if is_hex16(prefix) && prefix.starts_with("01") &&
                        (name.len() == 16 || name[16..].starts_with(' ')) {
                        Some(prefix.to_ascii_uppercase())
                    } else { None }
                })?;
            Some((title, build.to_ascii_uppercase()))
        }
        _ => None,
    }
}

fn search(bundle: &Bundle, opts: &Options) -> Vec<NativeHit> {
    let mut hits = Vec::new();
    for record in &bundle.records {
        let source = &record.provenance;
        if opts.source_id.as_deref().is_some_and(|id| id != source.source_id) {
            continue;
        }
        let Some((title_id, build_id)) = archive_identity(&source.source_id, &source.archive_path)
        else { continue; };
        if title_id != opts.title_id ||
            opts.build_id.as_deref().is_some_and(|b| b != build_id) {
            continue;
        }
        for entry in &record.codes {
            if entry.role.as_deref() != Some("code") {
                continue;
            }
            let Some(raw) = entry.code.as_deref() else { continue; };
            if opts.effect.as_deref().is_some_and(|needle|
                !entry.description.as_deref().unwrap_or("").to_lowercase().contains(&needle.to_lowercase())) {
                continue;
            }
            hits.push(NativeHit {
                title_id: title_id.clone(),
                build_id: build_id.clone(),
                source_id: source.source_id.clone(),
                source_record_id: record.id.clone(),
                archive_path: source.archive_path.clone(),
                upstream_path: source.upstream_path.clone(),
                revision: source.revision.clone(),
                heading: entry.description.clone(),
                ordinal: entry.ordinal,
                verification: entry.verification.clone(),
                code: opts.with_code.then(|| raw.to_owned()),
            });
        }
    }
    hits
}

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), ErrBox> {
    let mut opts = Options {
        db: PathBuf::from("generated/v1"), title_id: String::new(),
        build_id: None, source_id: None, effect: None, with_code: false,
        json: false, limit: 25, offset: 0,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => opts.db = PathBuf::from(args.next().ok_or("--db requires a directory")?),
            "--title-id" => opts.title_id = args.next().ok_or("--title-id requires an ID")?.to_ascii_uppercase(),
            "--build-id" => opts.build_id = Some(args.next().ok_or("--build-id requires an ID")?.to_ascii_uppercase()),
            "--source-id" => opts.source_id = Some(args.next().ok_or("--source-id requires a source ID")?),
            "--effect" => opts.effect = Some(args.next().ok_or("--effect requires text")?),
            "--with-code" => opts.with_code = true,
            "--json" => opts.json = true,
            "--offset" => opts.offset = args.next().ok_or("--offset requires an integer")?.parse()?,
            "--limit" => {
                opts.limit = args.next().ok_or("--limit requires an integer")?.parse()?;
                if !(1..=200).contains(&opts.limit) { return Err("--limit must be 1..200".into()); }
            }
            "--help" => {
                println!("Usage: cheatarium-query native-switch --title-id 0100000000010000 [--build-id B424BE150A8E7D78] [--source-id sharkive] [--effect Health] [--db generated/v1] [--with-code] [--offset 0] [--limit 25] [--json]");
                return Ok(());
            }
            _ => return Err(format!("Unknown native-switch option: {arg}").into()),
        }
    }
    if !is_hex16(&opts.title_id) || !opts.title_id.starts_with("01") {
        return Err("native-switch requires --title-id as a 16-digit Switch Title ID".into());
    }
    if opts.build_id.as_deref().is_some_and(|value| !is_hex16(value)) {
        return Err("--build-id must be 16 hexadecimal digits".into());
    }
    let bundle = load_platform(&opts.db, "switch")?;
    let hits = search(&bundle, &opts);
    let matched_files = hits.iter().map(|hit| hit.archive_path.as_str()).collect::<BTreeSet<_>>().len();
    let total = hits.len();
    let returned: Vec<_> = hits.into_iter().skip(opts.offset).take(opts.limit).collect();
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "schema": "cheatarium.native_switch_archive.v1",
            "read_only": true,
            "source_archive_not_curated": true,
            "gameplay_verified": false,
            "active_emulator_modified": false,
            "code_visible": opts.with_code,
            "title_id": opts.title_id,
            "exact_build_filter": opts.build_id,
            "source_files_matching": matched_files,
            "total": total,
            "offset": opts.offset,
            "returned": returned.len(),
            "has_more": opts.offset.saturating_add(returned.len()) < total,
            "results": returned
        }))?);
    } else {
        println!("{total} archived native Switch code sections in {matched_files} original source files; unverified");
        for hit in returned {
            println!("{} {} [{}] {} :: {}", hit.title_id, hit.build_id, hit.source_id,
                hit.heading.as_deref().unwrap_or("(no heading)"), hit.archive_path);
            if let Some(code) = hit.code { println!("{code}"); }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_native_source_paths_only() {
        assert_eq!(archive_identity("sharkive",
            "archive/sharkive/switch/0100000000010000/b424be150a8e7d78.txt"),
            Some(("0100000000010000".into(), "B424BE150A8E7D78".into())));
        assert_eq!(archive_identity("bad1dea-nxcheats-switch",
            "archive/bad1dea-nxcheats-switch/_old/Super Mario RPG/Super Mario RPG (US)/0100BC0018138000/cheats/E968832CADE2AD7C.txt"),
            Some(("0100BC0018138000".into(), "E968832CADE2AD7C".into())));
        assert!(archive_identity("sharkive", "archive/sharkive/switch/../0100000000010000/ABCDE.txt").is_none());
        assert!(archive_identity("libretro-database", "archive/sharkive/switch/0100000000010000/B424BE150A8E7D78.txt").is_none());
    }

    #[test]
    fn finds_actual_indexed_originals_by_title_and_build() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bundle = load_platform(root.join("generated/v1"), "switch").unwrap();
        let opts = Options {
            db: root.join("generated/v1"), title_id: "0100000000010000".into(),
            build_id: Some("B424BE150A8E7D78".into()), source_id: Some("sharkive".into()),
            effect: None, with_code: false, json: false, limit: 25, offset: 0,
        };
        let hits = search(&bundle, &opts);
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|hit| hit.title_id == opts.title_id &&
            hit.build_id == "B424BE150A8E7D78" && hit.source_id == "sharkive" &&
            hit.code.is_none() && hit.verification == "unverified"));
        let with_code = search(&bundle, &Options { with_code: true, ..opts });
        assert!(with_code.iter().all(|hit| hit.code.as_deref().is_some_and(|text| !text.is_empty())));
    }
}
