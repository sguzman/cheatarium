//! Offline, read-only Cheatarium source and candidate-game search.
use cheatarium_client::{
    load_game_candidates, load_platform, load_repeated_codes, verify_platform_distribution,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut root = PathBuf::from("generated/v1");
    let mut platform = None;
    let mut title = None;
    let mut effect = None;
    let mut game_key = None;
    let mut varying_descriptions = false;
    let mut declared_format = None;
    let mut source_id = None;
    let mut json = false;
    let mut limit = 10usize;
    let mut args = env::args().skip(1);
    let mode = match args.next().as_deref() {
        Some("search") => "search",
        Some("games") => "games",
        Some("verify") => "verify",
        Some("effects") => "effects",
        Some("repeats") => "repeats",
        _ => {
            eprintln!("Usage: cheatarium-query <search|games|effects|repeats|verify> --db generated/v1 --platform snes [--title Mario] [--effect Infinite] [--game-key super-mario-world] [--varying-descriptions] [--declared-format game-genie] [--source-id libretro-database] [--limit 10] [--json]");
            return Err("Expected search, games, effects, or verify subcommand".into());
        }
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => root = PathBuf::from(args.next().ok_or("--db needs a path")?),
            "--platform" => platform = Some(args.next().ok_or("--platform needs a value")?),
            "--title" => title = Some(args.next().ok_or("--title needs a value")?),
            "--effect" => effect = Some(args.next().ok_or("--effect needs a value")?),
            "--game-key" => game_key = Some(args.next().ok_or("--game-key needs a value")?),
            "--varying-descriptions" => varying_descriptions = true,
            "--declared-format" => {
                declared_format = Some(args.next().ok_or("--declared-format needs a value")?)
            }
            "--source-id" => source_id = Some(args.next().ok_or("--source-id needs a value")?),
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
    if mode != "repeats" && varying_descriptions {
        return Err("--varying-descriptions applies only to repeats".into());
    }
    if mode != "repeats" && game_key.is_some() {
        return Err("--game-key applies only to repeats".into());
    }
    if mode != "effects" && (declared_format.is_some() || source_id.is_some()) {
        return Err("--declared-format and --source-id apply only to effects".into());
    }
    if declared_format
        .as_deref()
        .is_some_and(|s: &str| s.trim().is_empty())
        || source_id
            .as_deref()
            .is_some_and(|s: &str| s.trim().is_empty())
    {
        return Err("Effect source and device filters cannot be empty".into());
    }
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
    if mode == "repeats" {
        let game_key = game_key.ok_or("Please provide --game-key for repeats")?;
        if game_key.trim().is_empty() {
            return Err("--game-key cannot be empty".into());
        }
        let index = load_repeated_codes(root, &platform)?;
        let hits = index
            .by_candidate_game_key(&game_key)
            .into_iter()
            .filter(|g| !varying_descriptions || g.description_text_varies)
            .collect::<Vec<_>>();
        let total = hits.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.raw_repeats.v1",
                    "platform": platform,
                    "candidate_game_key": game_key,
                    "varying_descriptions_only": varying_descriptions,
                    "candidate_only": true,
                    "cheats_activated": false,
                    "equivalent_effect_verified": false,
                    "rom_compatibility_verified": false,
                    "total_groups": total,
                    "groups": hits.into_iter().take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!("{total} exact raw-code repetition groups for {game_key:?} on {platform}");
            println!("These repeat TEXT only: title, effects and ROM builds are unverified.");
            for hit in hits.into_iter().take(limit) {
                println!(
                    "- {} ({} source occurrences, {} distinct descriptions; region {:?}, format {:?}, revision {:?})",
                    hit.source_code,
                    hit.occurrences.len(),
                    hit.description_variants,
                    hit.region_hint,
                    hit.declared_format,
                    hit.revision_hint
                );
            }
        }
        return Ok(());
    }
    if mode == "effects" {
        let effect = effect.ok_or("Please provide --effect for effects search")?;
        if effect.trim().is_empty() {
            return Err("--effect cannot be empty".into());
        }
        let bundle = load_platform(root, &platform)?;
        let hits = bundle.search_effect_filtered(
            &effect,
            declared_format.as_deref(),
            source_id.as_deref(),
        );
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
                    "declared_format_filter": declared_format,
                    "source_id_filter": source_id,
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
