//! Offline, read-only Cheatarium source and candidate-game search.
use cheatarium_client::{
    load_effect_tags, load_game_candidates, load_platform, load_repeated_codes,
    publications::load_snes_publications, reviews::load_effect_reviews,
    verify_platform_distribution,
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
    let mut category = None;
    let mut composition_relation = None;
    let mut source_record_id = None;
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
        Some("tags") => "tags",
        Some("reviews") => "reviews",
        Some("publications") => "publications",
        Some("compositions") => "compositions",
        _ => {
            eprintln!("Usage: cheatarium-query <search|games|effects|repeats|tags|reviews|compositions|publications|verify> --db generated/v1 --platform snes [--title Mario] [--effect Infinite] [--game-key super-mario-world] [--category lives] [--source-record-id SOURCE] [--relation revision-alternatives] [--varying-descriptions] [--declared-format game-genie] [--source-id libretro-database] [--limit 10] [--json]");
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
            "--category" => category = Some(args.next().ok_or("--category needs an ID")?),
            "--relation" => {
                composition_relation = Some(args.next().ok_or("--relation needs a value")?)
            }
            "--source-record-id" => {
                source_record_id = Some(args.next().ok_or("--source-record-id needs an ID")?)
            }
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
    if mode != "compositions" && composition_relation.is_some() {
        return Err("--relation applies only to compositions".into());
    }
    if composition_relation
        .as_deref()
        .is_some_and(|v| !matches!(v, "revision-alternatives" | "unresolved"))
    {
        return Err("--relation must be revision-alternatives or unresolved".into());
    }
    if mode != "reviews"
        && mode != "publications"
        && mode != "compositions"
        && source_record_id.is_some()
    {
        return Err("--source-record-id applies only to reviews, publications or compositions".into());
    }
    if mode != "repeats" && varying_descriptions {
        return Err("--varying-descriptions applies only to repeats".into());
    }
    if mode != "tags" && mode != "reviews" && category.is_some() {
        return Err("--category applies only to tags or reviews".into());
    }
    if mode != "repeats"
        && mode != "tags"
        && mode != "compositions"
        && mode != "publications"
        && game_key.is_some()
    {
        return Err("--game-key applies only to repeats, tags, compositions and publications".into());
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
    if mode == "publications" {
        if platform != "snes" {
            return Err("Historical multi-part publication witnesses currently support SNES only".into());
        }
        // Reject altered local publication/source artifacts before resolving claims.
        // SHA-256 validates this checkout, not the authenticity of its manifest.
        verify_platform_distribution(&root, &platform)?;
        let registry = load_snes_publications(&root)?;
        let hits: Vec<_> = registry
            .records
            .iter()
            .filter(|item| {
                game_key
                    .as_deref()
                    .is_none_or(|key| key == item.candidate_game_key)
            })
            .filter(|item| {
                source_record_id
                    .as_deref()
                    .is_none_or(|id| id == item.source_record_id)
            })
            .collect();
        let total = hits.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.snes_publications.v1",
                    "platform": "snes",
                    "game_key_filter": game_key,
                    "source_record_id_filter": source_record_id,
                    "total_witnesses": total,
                    "historical_publication_only": true,
                    "execution_observed": false,
                    "rom_match_verified": false,
                    "safe_to_auto_apply": false,
                    "publications": hits.into_iter().take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!("{total} historical SNES publication witnesses (not verified execution)");
            for item in hits.into_iter().take(limit) {
                println!(
                    "- {} #{}: {} ({})",
                    item.source_record_id,
                    item.source_ordinal,
                    item.raw_code,
                    item.publication.reference
                );
            }
            println!("No evidence of cartridge compatibility or safe code activation.");
        }
        return Ok(());
    }
    if mode == "compositions" {
        if platform != "snes" {
            return Err("Source composition evidence currently supports SNES only".into());
        }
        let bundle = load_platform(&root, &platform)?;
        let hits: Vec<_> = bundle
            .records
            .iter()
            .filter(|record| {
                game_key
                    .as_deref()
                    .is_none_or(|key| key == record.candidate_game_key)
                    && source_record_id
                        .as_deref()
                        .is_none_or(|id| id == record.id)
            })
            .flat_map(|record| {
                record.codes.iter().filter_map(move |code| {
                    code.composition
                        .as_ref()
                        .map(|composition| (record, code, composition))
                })
            })
            .filter(|(_, _, group)| {
                composition_relation
                    .as_deref()
                    .is_none_or(|relation| relation == group.relation)
            })
            .collect();
        let total = hits.len();
        let resolved = hits
            .iter()
            .filter(|(_, _, composition)| composition.relation == "revision-alternatives")
            .count();
        if json {
            let entries: Vec<_> = hits
                .into_iter()
                .take(limit)
                .map(|(record, code, composition)| {
                    serde_json::json!({
                        "source_record_id": record.id,
                        "candidate_game_key": record.candidate_game_key,
                        "source_ordinal": code.ordinal,
                        "raw_source_code": code.code,
                        "source_provenance": record.provenance,
                        "composition": composition,
                        "decoded_combined_writes": code.snes_decode,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.source_compositions.v1",
                    "platform": platform,
                    "candidate_game_key_filter": game_key,
                    "relation_filter": composition_relation,
                    "source_record_id_filter": source_record_id,
                    "total_groups": total,
                    "evidenced_revision_alternatives": resolved,
                    "unresolved_groups": total - resolved,
                    "rom_match_verified": false,
                    "safe_to_combine_or_auto_apply": false,
                    "entries": entries,
                }))?
            );
        } else {
            println!("{total} plus-joined SNES source records, {resolved} with revision-alternative evidence");
            println!("No output authorizes choosing a revision or combining/activating codes.");
            for (record, code, composition) in hits.into_iter().take(limit) {
                println!(
                    "- {} #{}: {} ({})",
                    record.id,
                    code.ordinal,
                    code.code.as_deref().unwrap_or(""),
                    composition.relation
                );
            }
        }
        return Ok(());
    }
    if mode == "reviews" {
        let index = load_effect_reviews(&root, &platform)?;
        let claims = index
            .claims
            .iter()
            .filter(|claim| {
                category
                    .as_deref()
                    .is_none_or(|id| claim.effect_category == id)
            })
            .filter(|claim| {
                source_record_id
                    .as_deref()
                    .is_none_or(|id| claim.source_record_id == id)
            })
            .collect::<Vec<_>>();
        let total = claims.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.effect_reviews.v1",
                    "platform": platform,
                    "category_filter": category,
                    "source_record_id_filter": source_record_id,
                    "claims": claims.into_iter().take(limit).collect::<Vec<_>>(),
                    "total_claims": total,
                    "reviews_are_evidence_not_execution_permission": true,
                    "unverified_imports_promoted": false,
                    "cheats_activated": false,
                }))?
            );
        } else {
            println!("{total} separately evidenced effect reviews on {platform}");
            println!("These are source-bound reports/observations, not universal compatibility.");
            for claim in claims.into_iter().take(limit) {
                println!(
                    "- {}: {} ({}, original {} #{})",
                    claim.id,
                    claim.effect_category,
                    claim.assessment,
                    claim.source_record_id,
                    claim.source_ordinal
                );
            }
        }
        return Ok(());
    }
    if mode == "tags" {
        let index = load_effect_tags(&root, &platform)?;
        if let Some(ref selected) = category {
            if selected.trim().is_empty() {
                return Err("--category cannot be empty".into());
            }
            let group = index
                .by_category(selected)
                .ok_or("No signal matches for that category")?;
            let hits: Vec<_> = group
                .matches
                .iter()
                .filter(|hit| {
                    game_key
                        .as_deref()
                        .is_none_or(|key| hit.candidate_game_key == key)
                })
                .collect();
            let total = hits.len();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema": "cheatarium.lexical_tags.v1",
                        "platform": platform,
                        "taxonomy_id": index.taxonomy_id,
                        "category": selected,
                        "candidate_game_key_filter": game_key,
                        "interpretation": index.interpretation,
                        "source_text_only": true,
                        "verified_effect": false,
                        "verified_game_identity": false,
                        "cheats_activated": false,
                        "total_matches": total,
                        "hits": hits.into_iter().take(limit).collect::<Vec<_>>(),
                    }))?
                );
            } else {
                println!(
                    "{total} textual {selected:?} cues on {platform}; no verified gameplay effect."
                );
                for hit in hits.into_iter().take(limit) {
                    println!(
                        "- {} #{} ({}; phrase {:?})",
                        hit.source_record_id,
                        hit.ordinal,
                        hit.candidate_game_key,
                        hit.matched_phrase
                    );
                }
            }
        } else {
            if game_key.is_some() {
                return Err("--game-key needs --category in tags mode".into());
            }
            let counts: Vec<_> = index
                .categories
                .iter()
                .map(|group| {
                    serde_json::json!({
                        "category": group.id,
                        "matches": group.matches.len()
                    })
                })
                .collect();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema": "cheatarium.lexical_tag_categories.v1",
                        "platform": platform,
                        "taxonomy_id": index.taxonomy_id,
                        "source_text_only": true,
                        "verified_effect": false,
                        "categories": counts,
                    }))?
                );
            } else {
                println!("Lexical description cues on {platform} (not verified effects):");
                for group in &index.categories {
                    println!("- {}: {} source occurrences", group.id, group.matches.len());
                }
            }
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
