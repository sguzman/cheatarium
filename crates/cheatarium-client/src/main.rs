//! Offline, read-only Cheatarium source and candidate-game search.
use cheatarium_client::{
    load_all_game_candidates, load_catalog, load_effect_tags, load_game_candidates, load_platform,
    load_repeated_codes, publications::load_snes_publications, reviews::load_effect_reviews,
    search_cross_platform_titles, verify_platform_distribution, GameEntryFilters,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut root = PathBuf::from("generated/v1");
    let mut platform = None;
    let mut title = None;
    let mut effect = None;
    let mut exact_code = None;
    let mut game_key = None;
    let mut entry_role = None;
    let mut exact_ordinal = None;
    let mut description_contains = None;
    let mut category = None;
    let mut composition_relation = None;
    let mut source_record_id = None;
    let mut varying_descriptions = false;
    let mut declared_format = None;
    let mut region_hint = None;
    let mut source_id = None;
    let mut json = false;
    let mut limit = 10usize;
    let mut offset = 0usize;
    let mut args = env::args().skip(1);
    let mode = match args.next().as_deref() {
        Some("search") => "search",
        Some("source") => "source",
        Some("games") => "games",
        Some("game") => "game",
        Some("entries") => "entries",
        Some("entry") => "entry",
        Some("platforms") => "platforms",
        Some("discover") => "discover",
        Some("verify") => "verify",
        Some("effects") => "effects",
        Some("codes") => "codes",
        Some("repeats") => "repeats",
        Some("tags") => "tags",
        Some("reviews") => "reviews",
        Some("publications") => "publications",
        Some("compositions") => "compositions",
        _ => {
            eprintln!("Usage: cheatarium-query <discover|platforms|search|source|games|game|entries|entry|effects|codes|repeats|tags|reviews|compositions|publications|verify> --db generated/v1 --platform snes [--title Mario] [--effect Infinite] [--code EXACT_RAW_CODE] [--game-key super-mario-world] [--role code|memory-entry|section-heading] [--description-contains TEXT] [--ordinal SOURCE_ORDINAL] [--category lives] [--source-record-id SOURCE] [--relation revision-alternatives] [--varying-descriptions] [--region-hint USA] [--declared-format game-genie] [--source-id libretro-database] [--limit 10] [--offset 0] [--json]");
            return Err("Expected search, games, effects, or verify subcommand".into());
        }
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => root = PathBuf::from(args.next().ok_or("--db needs a path")?),
            "--platform" => platform = Some(args.next().ok_or("--platform needs a value")?),
            "--title" => title = Some(args.next().ok_or("--title needs a value")?),
            "--effect" => effect = Some(args.next().ok_or("--effect needs a value")?),
            "--code" => exact_code = Some(args.next().ok_or("--code needs original source text")?),
            "--game-key" => game_key = Some(args.next().ok_or("--game-key needs a value")?),
            "--role" => entry_role = Some(args.next().ok_or("--role needs a value")?),
            "--ordinal" => {
                exact_ordinal = Some(
                    args.next()
                        .ok_or("--ordinal needs a number")?
                        .parse::<usize>()?,
                )
            }
            "--description-contains" => {
                description_contains = Some(args.next().ok_or("--description-contains needs text")?)
            }
            "--category" => category = Some(args.next().ok_or("--category needs an ID")?),
            "--relation" => {
                composition_relation = Some(args.next().ok_or("--relation needs a value")?)
            }
            "--source-record-id" => {
                source_record_id = Some(args.next().ok_or("--source-record-id needs an ID")?)
            }
            "--varying-descriptions" => varying_descriptions = true,
            "--region-hint" => {
                region_hint = Some(args.next().ok_or("--region-hint needs a value")?)
            }
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
            "--offset" => {
                offset = args.next().ok_or("--offset needs an integer")?.parse()?;
            }
            "--json" => json = true,
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    if mode == "discover" {
        if platform.is_some()
            || effect.is_some()
            || exact_code.is_some()
            || game_key.is_some()
            || entry_role.is_some()
            || exact_ordinal.is_some()
            || description_contains.is_some()
            || category.is_some()
            || composition_relation.is_some()
            || source_record_id.is_some()
            || varying_descriptions
            || region_hint.is_some()
            || declared_format.is_some()
            || source_id.is_some()
        {
            return Err("discover accepts --db, --title, --offset, --limit and --json only".into());
        }
        let query = title
            .as_deref()
            .ok_or("Please provide --title for cross-platform discovery")?;
        if query.trim().is_empty() {
            return Err("--title cannot be blank".into());
        }
        let indexes = load_all_game_candidates(&root)?;
        let hits = search_cross_platform_titles(&indexes, query);
        let total = hits.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.cross_platform_game_candidates.v1",
                    "query": query,
                    "indexed_platforms": indexes.len(),
                    "total_candidate_groups": total,
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "candidate_titles_only": true,
                    "rom_compatibility_verified": false,
                    "artifact_checksums_verified": true,
                    "manifest_authenticated": false,
                    "cheats_activated": false,
                    "matches": hits.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!(
                "{total} advisory game groups matching {query:?} across {} platforms",
                indexes.len()
            );
            println!("Title matches are not verified ROM identities or executable cheats.");
            for hit in hits.into_iter().skip(offset).take(limit) {
                println!(
                    "- {}:{} — {} ({} original sources)",
                    hit.platform,
                    hit.candidate.key,
                    hit.candidate.title_hint,
                    hit.candidate.source_record_ids.len()
                );
            }
        }
        return Ok(());
    }
    if mode == "platforms" {
        if platform.is_some()
            || title.is_some()
            || effect.is_some()
            || exact_code.is_some()
            || game_key.is_some()
            || category.is_some()
            || composition_relation.is_some()
            || source_record_id.is_some()
            || varying_descriptions
            || declared_format.is_some()
            || region_hint.is_some()
            || source_id.is_some()
            || entry_role.is_some()
            || description_contains.is_some()
            || exact_ordinal.is_some()
        {
            return Err("platforms accepts --db, --offset, --limit and --json only".into());
        }
        let catalog = load_catalog(&root)?;
        let total = catalog.bundles.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.platform_catalog.v1",
                    "source_format": catalog.format,
                    "matching_policy": catalog.matching_policy,
                    "total_platforms": total,
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "candidate_titles_only": true,
                    "rom_matching_verified": false,
                    "cheats_activated": false,
                    "platforms": catalog.bundles.iter().skip(offset).take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!("{total} indexed console/handheld platforms");
            println!("Filename candidates only; source cheats remain unverified.");
            for entry in catalog.bundles.iter().skip(offset).take(limit) {
                println!(
                    "- {}: {} original files, {} candidate groups, {} encoded codes, {} memory entries",
                    entry.platform,
                    entry.source_files,
                    entry.game_candidate_groups.unwrap_or(0),
                    entry.code_fields,
                    entry.native_memory_entries
                );
            }
        }
        return Ok(());
    }
    let platform = platform.ok_or("Please provide --platform")?;
    if offset != 0
        && mode != "compositions"
        && mode != "publications"
        && mode != "codes"
        && mode != "source"
        && mode != "game"
        && mode != "platforms"
        && mode != "games"
        && mode != "search"
        && mode != "entries"
    {
        return Err("--offset applies only to paginated source, game, publication, composition, code and search results".into());
    }
    if mode != "entry" && exact_ordinal.is_some() {
        return Err("--ordinal applies only to entry".into());
    }
    if mode != "entries" && description_contains.is_some() {
        return Err("--description-contains applies only to entries".into());
    }
    if mode != "entries" && entry_role.is_some() {
        return Err("--role applies only to entries".into());
    }
    if mode != "codes" && exact_code.is_some() {
        return Err("--code applies only to codes".into());
    }
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
        && mode != "codes"
        && mode != "source"
        && mode != "entries"
        && mode != "entry"
        && source_record_id.is_some()
    {
        return Err("--source-record-id applies only to reviews, publications, compositions, codes or source".into());
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
        && mode != "codes"
        && mode != "game"
        && mode != "entries"
        && game_key.is_some()
    {
        return Err(
            "--game-key applies only to repeats, tags, compositions, publications, codes and game"
                .into(),
        );
    }
    if mode != "entries" && region_hint.is_some() {
        return Err("--region-hint applies only to entries".into());
    }
    if mode != "effects" && mode != "entries" && declared_format.is_some() {
        return Err("--declared-format applies only to effects or entries".into());
    }
    if mode != "effects" && mode != "entries" && source_id.is_some() {
        return Err("--source-id applies only to effects or entries".into());
    }
    if region_hint
        .as_deref()
        .is_some_and(|s: &str| s.trim().is_empty())
    {
        return Err("Region source filter cannot be blank".into());
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
    if mode == "entry" {
        let selected = source_record_id
            .as_deref()
            .ok_or("Please provide --source-record-id")?;
        let ordinal = exact_ordinal.ok_or("Please provide --ordinal")?;
        verify_platform_distribution(&root, &platform)?;
        let bundle = load_platform(&root, &platform)?;
        let (source, entry) = bundle.find_original_entry(selected, ordinal)?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.original_entry.v1",
                    "platform": platform,
                    "source_record_id": source.id,
                    "source_ordinal": entry.ordinal,
                    "title_hint": source.title_hint,
                    "candidate_game_key": source.candidate_game_key,
                    "raw_filename": source.raw_filename,
                    "region_hint": source.region_hint,
                    "declared_format_hint": source.format_hint,
                    "provenance": source.provenance,
                    "entry": entry,
                    "original_source_enabled_is_not_activation": true,
                    "game_identity_verified": false,
                    "code_effect_or_compatibility_verified": false,
                    "cheats_activated": false,
                }))?
            );
        } else {
            println!(
                "{} #{}: {}",
                source.id,
                entry.ordinal,
                entry.description.as_deref().unwrap_or("<unnamed>")
            );
            println!("Original source entry; gameplay effects and ROM identity are unverified.");
            if let Some(raw) = entry.code.as_deref() {
                println!("Original code text: {raw}");
            }
        }
        return Ok(());
    }
    if mode == "entries" {
        let key = game_key.as_deref().ok_or("Please provide --game-key")?;
        verify_platform_distribution(&root, &platform)?;
        let index = load_game_candidates(&root, &platform)?;
        let candidate = index.find_candidate(key)?;
        let bundle = load_platform(&root, &platform)?;
        let hits = bundle.filter_entries_for_candidate(
            candidate,
            &GameEntryFilters {
                source_record_id: source_record_id.as_deref(),
                role: entry_role.as_deref(),
                description_contains: description_contains.as_deref(),
                region_hint: region_hint.as_deref(),
                declared_format_hint: declared_format.as_deref(),
                source_id: source_id.as_deref(),
            },
        )?;
        let total = hits.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.game_entries.v1",
                    "platform": platform,
                    "candidate_game_key": candidate.key,
                    "title_hint": candidate.title_hint,
                    "possible_title_collision": candidate.possible_title_collision,
                    "source_record_id_filter": source_record_id,
                    "role_filter": entry_role,
                    "description_contains_filter": description_contains,
                    "region_hint_filter": region_hint,
                    "declared_format_filter": declared_format,
                    "source_id_filter": source_id,
                    "source_facets_match_exact_original_labels": true,
                    "description_text_match_only": true,
                    "total_original_entries": total,
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "candidate_only": true,
                    "same_code_text_is_not_equivalence": true,
                    "original_source_enabled_is_not_activation": true,
                    "game_identity_verified": false,
                    "code_effects_verified": false,
                    "cheats_activated": false,
                    "entries": hits.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!(
                "{total} original entries for {} on {}",
                candidate.title_hint, platform
            );
            println!("Original source code text, not verified effects or ROM identity.");
            for hit in hits.into_iter().skip(offset).take(limit) {
                println!(
                    "- {} #{}: {} ({})",
                    hit.source_record_id,
                    hit.entry.ordinal,
                    hit.entry.description.as_deref().unwrap_or("<unnamed>"),
                    hit.entry.role.as_deref().unwrap_or("unspecified")
                );
            }
        }
        return Ok(());
    }
    if mode == "game" {
        let key = game_key.as_deref().ok_or("Please provide --game-key")?;
        verify_platform_distribution(&root, &platform)?;
        let index = load_game_candidates(&root, &platform)?;
        let candidate = index.find_candidate(key)?;
        let bundle = load_platform(&root, &platform)?;
        let sources = bundle.sources_for_candidate(candidate)?;
        let total = sources.len();
        if json {
            let page: Vec<_> = sources
                .iter()
                .skip(offset)
                .take(limit)
                .map(|record| {
                    serde_json::json!({
                        "source_record_id": record.id,
                        "title_hint": record.title_hint,
                        "raw_filename": record.raw_filename,
                        "region_hint": record.region_hint,
                        "declared_format_hint": record.format_hint,
                        "declared_cheats": record.declared_cheats,
                        "code_fields": record.codes.iter().filter(|code| code.role.as_deref() == Some("code")).count(),
                        "native_memory_entries": record.codes.iter().filter(|code| code.role.as_deref() == Some("memory-entry")).count(),
                        "section_headings": record.codes.iter().filter(|code| code.role.as_deref() == Some("section-heading")).count(),
                        "parse_warnings": record.parse_warnings,
                        "provenance": record.provenance,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.game_sources.v1",
                    "platform": platform,
                    "candidate_game_key": candidate.key,
                    "title_hint": candidate.title_hint,
                    "alternate_title_hints": candidate.alternate_title_hints,
                    "identity_confidence": candidate.identity_confidence,
                    "possible_title_collision": candidate.possible_title_collision,
                    "region_hints": candidate.region_hints,
                    "format_hints": candidate.format_hints,
                    "source_ids": candidate.source_ids,
                    "total_original_sources": total,
                    "total_code_fields": candidate.code_fields,
                    "total_native_memory_entries": candidate.native_memory_entries,
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "candidate_only": true,
                    "game_identity_verified": false,
                    "cheat_effects_verified": false,
                    "cheats_activated": false,
                    "sources": page,
                }))?
            );
        } else {
            println!(
                "{}: {} original sources on {}",
                candidate.title_hint, total, platform
            );
            println!("Advisory filename grouping only; no verified ROM identity or effects.");
            for record in sources.into_iter().skip(offset).take(limit) {
                println!(
                    "- {} ({}; {})",
                    record.id,
                    record.region_hint.as_deref().unwrap_or("unknown region"),
                    record.format_hint.as_deref().unwrap_or("unknown device"),
                );
            }
        }
        return Ok(());
    }
    if mode == "source" {
        let selected = source_record_id
            .as_deref()
            .ok_or("Please provide --source-record-id for original source inspection")?;
        if selected.is_empty() {
            return Err("--source-record-id cannot be empty".into());
        }
        let bundle = load_platform(&root, &platform)?;
        let record = bundle.find_source_record(selected)?;
        let total = record.codes.len();
        let code_fields = record
            .codes
            .iter()
            .filter(|entry| entry.role.as_deref() == Some("code"))
            .count();
        let memory_entries = record
            .codes
            .iter()
            .filter(|entry| entry.role.as_deref() == Some("memory-entry"))
            .count();
        let headings = record
            .codes
            .iter()
            .filter(|entry| entry.role.as_deref() == Some("section-heading"))
            .count();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.original_source.v1",
                    "platform": platform,
                    "source_record_id": record.id,
                    "title_hint": record.title_hint,
                    "candidate_game_key": record.candidate_game_key,
                    "region_hint": record.region_hint,
                    "declared_format_hint": record.format_hint,
                    "raw_filename": record.raw_filename,
                    "declared_cheats": record.declared_cheats,
                    "parse_warnings": record.parse_warnings,
                    "provenance": record.provenance,
                    "total_source_entries": total,
                    "code_fields": code_fields,
                    "native_memory_entries": memory_entries,
                    "section_headings": headings,
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "original_source_enabled_is_not_activation": true,
                    "game_identity_verified": false,
                    "code_effect_or_compatibility_verified": false,
                    "cheats_activated": false,
                    "entries": record.codes.iter().skip(offset).take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!("{}: {} original source entries", record.id, total);
            println!(
                "{} code fields; {} memory entries; {} headings",
                code_fields, memory_entries, headings
            );
            println!("Imported source text is not verified gameplay or ROM compatibility.");
            for entry in record.codes.iter().skip(offset).take(limit) {
                println!(
                    "- #{}: {} ({})",
                    entry.ordinal,
                    entry.description.as_deref().unwrap_or("<unnamed>"),
                    entry.role.as_deref().unwrap_or("unspecified")
                );
            }
        }
        return Ok(());
    }
    if mode == "codes" {
        let original = exact_code.ok_or("Please provide --code for exact source-code lookup")?;
        if original.is_empty() {
            return Err("--code cannot be empty".into());
        }
        let bundle = load_platform(&root, &platform)?;
        let matches = bundle.search_exact_code(&original);
        let hits: Vec<_> = matches
            .into_iter()
            .filter(|hit| {
                game_key
                    .as_deref()
                    .is_none_or(|key| key == hit.candidate_game_key)
            })
            .filter(|hit| {
                source_record_id
                    .as_deref()
                    .is_none_or(|id| id == hit.source_record_id)
            })
            .collect();
        let total = hits.len();
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema": "cheatarium.exact_codes.v1",
                    "platform": platform,
                    "raw_code": original,
                    "candidate_game_key_filter": game_key,
                    "source_record_id_filter": source_record_id,
                    "total_source_occurrences": total,
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "exact_original_text_only": true,
                    "equivalent_effect_verified": false,
                    "rom_compatibility_verified": false,
                    "cheats_activated": false,
                    "hits": hits.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!("{total} exact original-source text occurrences on {platform}");
            println!("Same text does not prove compatible ROMs or equivalent cheat effects.");
            for hit in hits.into_iter().skip(offset).take(limit) {
                println!(
                    "- {} #{}: {}",
                    hit.source_record_id,
                    hit.cheat.ordinal,
                    hit.cheat.description.as_deref().unwrap_or("<unnamed>")
                );
            }
        }
        return Ok(());
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
            return Err(
                "Historical multi-part publication witnesses currently support SNES only".into(),
            );
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
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "historical_publication_only": true,
                    "execution_observed": false,
                    "rom_match_verified": false,
                    "safe_to_auto_apply": false,
                    "publications": hits.into_iter().skip(offset).take(limit).collect::<Vec<_>>(),
                }))?
            );
        } else {
            println!("{total} historical SNES publication witnesses (not verified execution)");
            for item in hits.into_iter().skip(offset).take(limit) {
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
                    && source_record_id.as_deref().is_none_or(|id| id == record.id)
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
                .skip(offset)
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
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
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
            for (record, code, composition) in hits.into_iter().skip(offset).take(limit) {
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
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "candidates": hits.into_iter().skip(offset).take(limit).collect::<Vec<_>>()
                }))?
            );
        } else {
            println!("{total} candidate game groups for {title:?} on {platform}");
            println!("WARNING: filename grouping is not verified game/ROM identity.");
            for game in hits.into_iter().skip(offset).take(limit) {
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
                    "offset": offset,
                    "returned": total.saturating_sub(offset).min(limit),
                    "has_more": offset.saturating_add(limit) < total,
                    "records": hits.into_iter().skip(offset).take(limit).collect::<Vec<_>>()
                }))?
            );
        } else {
            println!("{total} candidate source records for {title:?} on {platform}");
            println!(
                "WARNING: title matches are suggestions; ROM/build compatibility is unverified."
            );
            for hit in hits.into_iter().skip(offset).take(limit) {
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
