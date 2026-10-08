//! Offline, read-only Cheatarium source and candidate-game search.
use cheatarium_client::{load_game_candidates, load_platform, verify_platform_distribution};
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut root = PathBuf::from("generated/v1");
    let mut platform = None;
    let mut title = None;
    let mut effect = None;
    let mut json = false;
    let mut limit = 10usize;
    let mut args = env::args().skip(1);
    let mode = match args.next().as_deref() {
        Some("search") => "search",
        Some("games") => "games",
        Some("verify") => "verify",
        Some("effects") => "effects",
        _ => {
            eprintln!("Usage: cheatarium-query <search|games|effects|verify> --db generated/v1 --platform snes [--title Mario] [--effect Infinite] [--limit 10] [--json]");
            return Err("Expected search or games subcommand".into());
        }
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => root = PathBuf::from(args.next().ok_or("--db needs a path")?),
            "--platform" => platform = Some(args.next().ok_or("--platform needs a value")?),
            "--title" => title = Some(args.next().ok_or("--title needs a value")?),
            "--effect" => effect = Some(args.next().ok_or("--effect needs a value")?),
            "--limit" => {
                limit = args.next().ok_or("--limit needs an integer")?.parse()?;
                if !(1..=100).contains(&limit) {
                    return Err("--limit must be 1..100".into());
                }
            }
            "--json" => json = true,
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    let platform = platform.ok_or("Please provide --platform")?;
    if mode == "verify" {
        verify_platform_distribution(&root, &platform)?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.integrity.v1",
                    "platform": platform,
                    "artifact_checksums_match": true,
                    "manifest_authenticated": false,
                    "cheats_verified": false
                }))?
            );
        } else {
            println!("OK: {platform} artifacts match local distribution.json SHA-256 checksums");
            println!("The manifest itself is not authenticated, and cheats remain unverified.");
        }
        return Ok(());
    }
    if mode == "effects" {
        let effect = effect.ok_or("Please provide --effect for effects search")?;
        if effect.trim().is_empty() {
            return Err("--effect cannot be empty".into());
        }
        let bundle = load_platform(root, &platform)?;
        let hits = bundle.search_effect(&effect);
        let title_filter = title.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let filtered = hits
            .into_iter()
            .filter(|hit| {
                title_filter
                    .is_none_or(|name| hit.title_hint.to_lowercase().contains(&name.to_lowercase()))
            })
            .collect::<Vec<_>>();
        let total = filtered.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.effects.v1",
                    "platform": platform,
                    "effect_query": effect,
                    "title_filter": title_filter,
                    "candidate_only": true,
                    "cheats_activated": false,
                    "total_matches": total,
                    "hits": filtered.into_iter().take(limit).collect::<Vec<_>>()
                }))?
            );
        } else {
            println!("{total} unverified effect matches for {effect:?} on {platform}");
            println!("Source text search only: no ROM/build matching or code activation.");
            for hit in filtered.into_iter().take(limit) {
                println!(
                    "- {} — {} ({}, #{})",
                    hit.title_hint,
                    hit.cheat.description.as_deref().unwrap_or("<unnamed>"),
                    hit.raw_filename,
                    hit.cheat.ordinal
                );
            }
        }
        return Ok(());
    }
    let title = title.ok_or("Please provide --title")?;
    if mode == "games" {
        let index = load_game_candidates(root, &platform)?;
        let hits = index.search_title(&title);
        let total = hits.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.game_candidates.v1",
                    "platform": platform,
                    "query": title,
                    "candidate_only": true,
                    "total_candidate_groups": total,
                    "candidates": hits.into_iter().take(limit).collect::<Vec<_>>()
                }))?
            );
        } else {
            println!("{total} candidate game groups for {title:?} on {platform}");
            println!("WARNING: filename grouping is not verified game/ROM identity.");
            for game in hits.into_iter().take(limit) {
                println!(
                    "- {} ({} sources, {} records, {} codes)",
                    game.title_hint,
                    game.source_ids.len(),
                    game.source_record_ids.len(),
                    game.code_fields
                );
            }
        }
    } else {
        let bundle = load_platform(root, &platform)?;
        let hits = bundle.search_title(&title);
        let total = hits.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.query.v1",
                    "platform": platform,
                    "query": title,
                    "candidate_only": true,
                    "total_source_records": total,
                    "records": hits.into_iter().take(limit).collect::<Vec<_>>()
                }))?
            );
        } else {
            println!("{total} candidate source records for {title:?} on {platform}");
            println!(
                "WARNING: title matches are suggestions; ROM/build compatibility is unverified."
            );
            for hit in hits.into_iter().take(limit) {
                let code_count = hit.codes.iter().filter(|x| x.is_code()).count();
                let memory_count = hit.codes.iter().filter(|x| x.is_memory_entry()).count();
                println!(
                    "- {} ({code_count} device codes, {memory_count} native memory entries; {})",
                    hit.raw_filename, hit.provenance.source_id
                );
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("cheatarium-query: {e}");
        std::process::exit(1);
    }
}
