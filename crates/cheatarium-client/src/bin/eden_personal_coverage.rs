//! Join the user's recorded static NSP Build IDs to Eden's *advisory* local
//! inventory. Never opens game files or imports/installs third-party cheat bodies.
//! The curation manifest is public metadata, not a runtime version witness.
use super::{is_title_id, CachedTitle, ErrBox, InventoryGame, Report};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const MAX_MANIFEST_BYTES: u64 = 2_000_000;
const MAX_GAMES: usize = 1024;
const MAX_EXTERNAL_SOURCES_PER_GAME: usize = 32;

#[derive(Debug, Deserialize)]
struct Manifest {
    format: String,
    games: Vec<CuratedGame>,
}

#[derive(Debug, Deserialize)]
struct CuratedGame {
    title_id: String,
    title: String,
    static_nsp_observation: StaticObservation,
    #[serde(default)]
    exact_build_external_sources: Vec<ExternalSource>,
}

#[derive(Debug, Deserialize)]
struct StaticObservation {
    status: String,
    cheat_build_id: String,
    pinned_native_cheat_sections_matched: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ExternalSource {
    source_id: String,
    title_id: String,
    build_id: String,
    game_version_claim: String,
    author_credit: String,
    source_url: String,
    relationship: String,
    rights_status: String,
    source_build_claim_verified: bool,
    body_imported: bool,
    gameplay_verified: bool,
}

#[derive(Debug, Serialize)]
struct Match {
    title_id: String,
    title: String,
    in_eden_cache: bool,
    filename_labeled_base_files: usize,
    static_nsp_build_id: String,
    static_nsp_extraction_status: String,
    pinned_native_exact_build_sections: usize,
    exact_build_external_sources: Vec<ExternalSource>,
    coverage_status: &'static str,
    current_eden_build_verified: bool,
    cheats_gameplay_verified: bool,
    installation_allowed: bool,
}

#[derive(Debug, Serialize)]
struct UncuratedGame {
    title_id: String,
    cached_title_name: Option<String>,
    in_eden_cache: bool,
    filename_labeled_base_files: usize,
    registered_update_files: usize,
    archived_cheat_build_candidates: usize,
    external_link_only_references: usize,
    next_action: &'static str,
    current_eden_build_verified: bool,
    cheats_gameplay_verified: bool,
}

#[derive(Debug, Default, Serialize)]
struct ScanEvidence {
    scanned_game_files: usize,
    game_files_without_filename_title_id: usize,
    directories_not_descended_without_recursive_scan: usize,
    registered_roots_not_scanned: usize,
    traversal_limit_reached: bool,
    scan_warnings: usize,
    may_have_unexamined_games: bool,
}

fn scan_evidence(report: &Report) -> ScanEvidence {
    let unavailable = report.game_roots.iter()
        .filter(|root| root.status != "scanned")
        .count();
    let scan_warnings = report.warnings.iter()
        .filter(|message| message.starts_with("directory_scan_incomplete:"))
        .count();
    ScanEvidence {
        scanned_game_files: report.game_files.len(),
        game_files_without_filename_title_id: report.unresolved_game_files,
        directories_not_descended_without_recursive_scan:
            report.scan_statistics.directories_not_descended_scan_disabled,
        registered_roots_not_scanned: unavailable,
        traversal_limit_reached: report.scan_statistics.traversal_limit_reached,
        scan_warnings,
        may_have_unexamined_games: report.unresolved_game_files > 0
            || report.scan_statistics.directories_not_descended_scan_disabled > 0
            || report.scan_statistics.traversal_limit_reached
            || unavailable > 0
            || scan_warnings > 0,
    }
}

#[derive(Debug, Serialize)]
pub(super) struct Coverage {
    schema_version: u32,
    format: &'static str,
    read_only: bool,
    scan_evidence: ScanEvidence,
    matched_eden_candidate_games: usize,
    uncurated_eden_candidate_games: usize,
    uncurated_games: Vec<UncuratedGame>,
    curated_games_not_found_in_eden_candidates: Vec<String>,
    games: Vec<Match>,
    limitations: Vec<&'static str>,
}

fn valid_build(id: &str) -> bool {
    id.len() == 16 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn match_catalog(
    manifest: Manifest,
    inventory: &[InventoryGame],
    cached_titles: &[CachedTitle],
) -> Result<Coverage, ErrBox> {
    if manifest.format != "cheatarium-user-switch-cheat-source-curation-v1" {
        return Err("Unexpected Switch personal curation format".into());
    }
    if manifest.games.len() > MAX_GAMES {
        return Err("Personal curation has too many games".into());
    }
    let mut catalog = BTreeMap::new();
    for game in manifest.games {
        if !is_title_id(&game.title_id)
            || !valid_build(&game.static_nsp_observation.cheat_build_id)
            || game.static_nsp_observation.status != "main_nso_extracted_from_local_nsp"
        {
            return Err(format!(
                "Invalid or unverified static NSP identity: {}",
                game.title_id
            )
            .into());
        }
        if game.exact_build_external_sources.len() > MAX_EXTERNAL_SOURCES_PER_GAME {
            return Err(format!("Too many external source links: {}", game.title_id).into());
        }
        let mut source_ids = BTreeSet::new();
        for source in &game.exact_build_external_sources {
            if source.title_id != game.title_id
                || source.build_id != game.static_nsp_observation.cheat_build_id
                || !source.source_build_claim_verified
                || source.body_imported
                || source.gameplay_verified
                || !source.source_url.starts_with("https://")
                || source.source_url.len() > 2048
                || source.source_id.is_empty()
                || !source_ids.insert(source.source_id.as_str())
            {
                return Err(format!(
                    "Invalid unverified external source claim: {}",
                    game.title_id
                )
                .into());
            }
        }
        let title_id = game.title_id.clone();
        if catalog.insert(title_id.clone(), game).is_some() {
            return Err(format!("Duplicate personal curation Title ID: {title_id}").into());
        }
    }

    let mut matched = Vec::new();
    let mut present = BTreeSet::new();
    for item in inventory {
        if let Some(game) = catalog.get(&item.title_id) {
            if !present.insert(item.title_id.as_str()) {
                continue;
            }
            let sections = game
                .static_nsp_observation
                .pinned_native_cheat_sections_matched;
            let external = &game.exact_build_external_sources;
            matched.push(Match {
                title_id: item.title_id.clone(),
                title: game.title.clone(),
                in_eden_cache: item.in_eden_cache,
                filename_labeled_base_files: item.filename_labeled_base_files
                    + item.ticket_filename_labeled_base_files,
                static_nsp_build_id: game.static_nsp_observation.cheat_build_id.clone(),
                static_nsp_extraction_status: game.static_nsp_observation.status.clone(),
                pinned_native_exact_build_sections: sections,
                exact_build_external_sources: external.clone(),
                coverage_status: if sections > 0 {
                    "pinned_source_sections_for_static_nsp_build"
                } else if !external.is_empty() {
                    "external_exact_build_claims_link_only_not_imported"
                } else {
                    "no_exact_build_source_found"
                },
                current_eden_build_verified: false,
                cheats_gameplay_verified: false,
                installation_allowed: false,
            });
        }
    }
    // This is the user-library expansion queue. Cache-only labels can be stale
    // and must never be presented as confirmed installed games.
    let mut uncurated = Vec::new();
    let mut uncurated_ids = BTreeSet::new();
    for item in inventory {
        if catalog.contains_key(&item.title_id) || !uncurated_ids.insert(item.title_id.as_str()) {
            continue;
        }
        let name = cached_titles
            .iter()
            .find(|name| name.title_id == item.title_id)
            .and_then(|name| name.title_name.clone());
        uncurated.push(UncuratedGame {
            title_id: item.title_id.clone(),
            cached_title_name: name,
            in_eden_cache: item.in_eden_cache,
            filename_labeled_base_files: item.filename_labeled_base_files
                + item.ticket_filename_labeled_base_files,
            registered_update_files: item.update_files_in_registered_folders
                + item.ticket_filename_labeled_update_files,
            archived_cheat_build_candidates: item.archived_build_candidate_count,
            external_link_only_references: item.external_reference_count,
            next_action: if item.filename_labeled_base_files
                + item.ticket_filename_labeled_base_files
                > 0
            {
                "extract_static_main_build_id_from_owned_game_file"
            } else if item.in_eden_cache {
                "confirm_cached_game_still_registered_and_locate_owned_game_file"
            } else {
                "confirm_game_identity_from_registered_update_or_filename"
            },
            current_eden_build_verified: false,
            cheats_gameplay_verified: false,
        });
    }
    // Prioritize candidate games with native cheat data rather than chasing
    // metadata-only links; all entries remain advisory until Build ID extraction.
    uncurated.sort_by(|a, b| {
        b.archived_cheat_build_candidates
            .cmp(&a.archived_cheat_build_candidates)
            .then_with(|| {
                b.filename_labeled_base_files
                    .cmp(&a.filename_labeled_base_files)
            })
            .then_with(|| a.title_id.cmp(&b.title_id))
    });
    let absent = catalog
        .keys()
        .filter(|id| !present.contains(id.as_str()))
        .cloned()
        .collect();
    Ok(Coverage {
        schema_version: 1,
        format: "cheatarium-eden-personal-switch-coverage-v1",
        read_only: true,
        scan_evidence: ScanEvidence::default(),
        matched_eden_candidate_games: matched.len(),
        uncurated_eden_candidate_games: uncurated.len(),
        uncurated_games: uncurated,
        curated_games_not_found_in_eden_candidates: absent,
        games: matched,
        limitations: vec![
            "Eden filenames and game-list caches are advisory, not installed-game proof.",
            "The recorded Build IDs were extracted from local NSP snapshots; they are not proof of what Eden currently launches.",
            "Pinned source sections are lexical matches to those snapshots, not gameplay-tested cheats.",
            "External source links claim exact Title ID and Build ID; their third-party code bodies have not been imported.",
            "Uncurated games are candidate Title IDs discovered from Eden labels, not confirmed currently installed games.",
            "Archive candidate counts alone never prove a compatible Build ID; uncurated games require static or runtime executable identification.",
            "No cheats, ROMs, secrets, saved games, or Eden settings are read or changed by this comparison.",
        ],
    })
}

pub(super) fn read_coverage(
    path: &Path,
    inventory: &[InventoryGame],
    cached_titles: &[CachedTitle],
    report: &Report,
) -> Result<Coverage, ErrBox> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_MANIFEST_BYTES {
        return Err("Personal curation must be a bounded regular JSON file".into());
    }
    let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
    let mut coverage = match_catalog(manifest, inventory, cached_titles)?;
    coverage.scan_evidence = scan_evidence(report);
    Ok(coverage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_game() -> Vec<InventoryGame> {
        vec![InventoryGame {
            title_id: "01006A800016E000".into(),
            filename_labeled_base_files: 1,
            ticket_filename_labeled_base_files: 0,
            ticket_filename_labeled_update_files: 0,
            in_eden_cache: true,
            archived_build_candidate_count: 0,
            external_reference_count: 0,
            update_files_in_registered_folders: 0,
            installed_base_game_verified: false,
            installed_update_verified: false,
            installed_build_id_verified: false,
        }]
    }

    fn example_json() -> String {
        r#"{"format":"cheatarium-user-switch-cheat-source-curation-v1","games":[{"title_id":"01006A800016E000","title":"Smash","static_nsp_observation":{"status":"main_nso_extracted_from_local_nsp","cheat_build_id":"CBD5A9B56EA859BA","pinned_native_cheat_sections_matched":0},"exact_build_external_sources":[{"source_id":"sample","title_id":"01006A800016E000","build_id":"CBD5A9B56EA859BA","game_version_claim":"13.0.2","author_credit":"Author","source_url":"https://example.com/source","relationship":"original","rights_status":"link_only","source_build_claim_verified":true,"body_imported":false,"gameplay_verified":false}]}]}"#.to_owned()
    }

    #[test]
    fn exact_external_claim_is_not_an_installed_or_gameplay_verified_cheat() {
        let manifest = serde_json::from_str(&example_json()).unwrap();
        let result = match_catalog(manifest, &one_game(), &[]).unwrap();
        assert_eq!(result.matched_eden_candidate_games, 1);
        assert!(result.curated_games_not_found_in_eden_candidates.is_empty());
        assert_eq!(
            result.games[0].coverage_status,
            "external_exact_build_claims_link_only_not_imported"
        );
        assert_eq!(result.games[0].exact_build_external_sources.len(), 1);
        assert!(!result.games[0].installation_allowed);
        assert!(!result.games[0].current_eden_build_verified);
        assert!(!result.games[0].cheats_gameplay_verified);
    }

    #[test]
    fn public_six_game_manifest_reports_smash_as_link_only_gap() {
        let content = include_str!("../../../../research/switch-personal-curation-v1.json");
        let manifest = serde_json::from_str(content).unwrap();
        let result = match_catalog(manifest, &one_game(), &[]).unwrap();
        assert_eq!(result.matched_eden_candidate_games, 1);
        assert_eq!(result.curated_games_not_found_in_eden_candidates.len(), 5);
        assert_eq!(result.games[0].title, "Super Smash Bros. Ultimate");
        assert_eq!(result.games[0].pinned_native_exact_build_sections, 0);
        assert_eq!(result.games[0].static_nsp_build_id, "CBD5A9B56EA859BA");
        assert_eq!(result.games[0].exact_build_external_sources.len(), 2);
        assert!(!result.games[0].installation_allowed);
    }

    #[test]
    fn refuses_mismatched_claimed_build_id() {
        let changed = example_json().replace(
            r#""build_id":"CBD5A9B56EA859BA""#,
            r#""build_id":"B9B166DF1DB90BAF""#,
        );
        let manifest = serde_json::from_str(&changed).unwrap();
        assert!(match_catalog(manifest, &one_game(), &[]).is_err());
    }

    #[test]
    fn expansion_queue_reports_uncurated_eden_game_without_guessing_build() {
        let mut inventory = one_game();
        inventory.push(InventoryGame {
            title_id: "0100000000010000".into(),
            filename_labeled_base_files: 1,
            ticket_filename_labeled_base_files: 0,
            ticket_filename_labeled_update_files: 0,
            in_eden_cache: true,
            archived_build_candidate_count: 3,
            external_reference_count: 2,
            update_files_in_registered_folders: 0,
            installed_base_game_verified: false,
            installed_update_verified: false,
            installed_build_id_verified: false,
        });
        let names = vec![CachedTitle {
            title_id: "0100000000010000".into(),
            title_name: Some("Super Mario Odyssey".into()),
            evidence: "eden_appname_cache",
            installed_game_verified: false,
        }];
        let manifest = serde_json::from_str(&example_json()).unwrap();
        let result = match_catalog(manifest, &inventory, &names).unwrap();
        assert_eq!(result.matched_eden_candidate_games, 1);
        assert_eq!(result.uncurated_eden_candidate_games, 1);
        let uncurated = &result.uncurated_games[0];
        assert_eq!(
            uncurated.cached_title_name.as_deref(),
            Some("Super Mario Odyssey")
        );
        assert_eq!(uncurated.archived_cheat_build_candidates, 3);
        assert_eq!(
            uncurated.next_action,
            "extract_static_main_build_id_from_owned_game_file"
        );
        assert!(!uncurated.current_eden_build_verified);
    }

    #[test]
    fn cache_only_candidate_is_not_claimed_as_installed() {
        let mut item = one_game().remove(0);
        item.title_id = "0100000000010000".into();
        item.filename_labeled_base_files = 0;
        let manifest = serde_json::from_str(&example_json()).unwrap();
        let result = match_catalog(manifest, &[item], &[]).unwrap();
        assert_eq!(result.uncurated_eden_candidate_games, 1);
        assert_eq!(
            result.uncurated_games[0].next_action,
            "confirm_cached_game_still_registered_and_locate_owned_game_file"
        );
        assert_eq!(result.matched_eden_candidate_games, 0);
    }

    #[test]
    fn unregistered_curated_game_is_not_invented_in_eden() {
        let manifest = serde_json::from_str(&example_json()).unwrap();
        let result = match_catalog(manifest, &[], &[]).unwrap();
        assert_eq!(result.matched_eden_candidate_games, 0);
        assert_eq!(
            result.curated_games_not_found_in_eden_candidates,
            vec!["01006A800016E000"]
        );
    }
}
