//! Read-only evidence gate for privately staged Switch cheats.
//! No NSP decryption, secrets, cheat installation or Eden settings changes.
use cheatarium_client::load_game_candidates;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "eden_builds.rs"]
#[allow(dead_code)]
mod eden_builds;

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_MANIFEST_BYTES: u64 = 2_000_000;
const MAX_CHEAT_BYTES: u64 = 1_000_000;

#[derive(Deserialize)]
struct Catalog {
    format: String,
    games: Vec<Game>,
}
#[derive(Deserialize)]
struct Game {
    title_id: String,
    title: String,
    hamlet_builds: Vec<Build>,
}
#[derive(Deserialize)]
struct Build {
    build_id: String,
    #[serde(default)]
    strict_native_hex_sections: usize,
}
#[derive(Serialize)]
struct GameStatus {
    title: String,
    title_id: String,
    historical_main_build_ids: Vec<String>,
    staged_source_build_ids: Vec<String>,
    exact_matching_build_id: Option<String>,
    native_sections_in_matching_source: usize,
    status: &'static str,
}
#[derive(Serialize)]
struct Report {
    format: &'static str,
    read_only: bool,
    installed_cheats: usize,
    qt_config_modified: bool,
    main_log_found: bool,
    saved_history_found: bool,
    historical_evidence_is_not_current_version_proof: bool,
    games: Vec<GameStatus>,
    warning: &'static str,
}
fn valid_build(id: &str) -> bool {
    id.len() == 16
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_lowercase())
}
fn valid_title(id: &str) -> bool {
    valid_build(id) && id.starts_with("01")
}

fn staged(root: &Path, game: &Game) -> Result<BTreeMap<String, usize>, ErrBox> {
    let mut known = BTreeMap::new();
    for build in &game.hamlet_builds {
        if !valid_build(&build.build_id) {
            return Err("Curated source contains malformed Build ID".into());
        }
        let file = root
            .join(&game.title_id)
            .join(format!("{}.txt", build.build_id));
        match fs::symlink_metadata(&file) {
            Ok(meta) => {
                if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_CHEAT_BYTES
                {
                    return Err(format!(
                        "Private cheat cache contains invalid file: {}",
                        file.display()
                    )
                    .into());
                }
                let body = fs::read_to_string(&file)?;
                // Private curator stages original bracketed native-hex sections
                // with no preamble. Never treat a malformed partial file as ready.
                let mut count = 0;
                let mut active = false;
                for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
                    if line.starts_with('[') && line.ends_with(']') {
                        count += 1;
                        active = true;
                    } else if !active || {
                        let words = line.split_ascii_whitespace().collect::<Vec<_>>();
                        words.is_empty()
                            || words.len() > 5
                            || words.iter().any(|word| {
                                word.len() != 8 || !word.bytes().all(|b| b.is_ascii_hexdigit())
                            })
                    } {
                        return Err(
                            format!("Invalid staged cheat syntax: {}", file.display()).into()
                        );
                    }
                }
                if count == 0 || count != build.strict_native_hex_sections {
                    return Err(format!(
                        "Staged source section count mismatched manifest: {}",
                        file.display()
                    )
                    .into());
                }
                known.insert(build.build_id.clone(), count);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        };
    }
    Ok(known)
}
fn evaluate(
    game: &Game,
    observations: &BTreeSet<String>,
    private: &BTreeMap<String, usize>,
) -> GameStatus {
    let match_id = if observations.len() == 1 {
        let id = observations.iter().next().expect("one observation");
        private.get(id).map(|_| id.clone())
    } else {
        None
    };
    let status = if observations.len() > 1 {
        "blocked_conflicting_historical_builds"
    } else if observations.is_empty() {
        "blocked_no_historical_main_build_evidence"
    } else if match_id.is_some() {
        "source_build_match_historical_only_not_gameplay_verified"
    } else {
        "blocked_no_private_source_for_observed_build"
    };
    GameStatus {
        title: game.title.clone(),
        title_id: game.title_id.clone(),
        historical_main_build_ids: observations.iter().cloned().collect(),
        staged_source_build_ids: private.keys().cloned().collect(),
        native_sections_in_matching_source: match_id
            .as_ref()
            .and_then(|id| private.get(id))
            .copied()
            .unwrap_or(0),
        exact_matching_build_id: match_id,
        status,
    }
}
fn default_private_root() -> PathBuf {
    // Match the existing private Python acquisition tool's actual destination,
    // even on hosts where XDG_DATA_HOME points somewhere else.
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".local/share/cheatarium/private-switch-curation")
}
fn run() -> Result<(), ErrBox> {
    let mut manifest = PathBuf::from("research/switch-personal-curation-v1.json");
    let mut db = PathBuf::from("generated/v1");
    let mut root = default_private_root();
    let mut log = eden_builds::log_default();
    let mut history = eden_builds::eden_history::default_path();
    let mut only_title = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => {
                manifest = PathBuf::from(args.next().ok_or("--manifest requires a path")?)
            }
            "--db" => db = PathBuf::from(args.next().ok_or("--db requires a path")?),
            "--private-root" => {
                root = PathBuf::from(args.next().ok_or("--private-root requires a path")?)
            }
            "--log" => log = PathBuf::from(args.next().ok_or("--log requires a path")?),
            "--history-file" => {
                history = PathBuf::from(args.next().ok_or("--history-file requires a path")?)
            }
            "--title-id" => {
                only_title = Some(
                    args.next()
                        .ok_or("--title-id requires a Title ID")?
                        .to_ascii_uppercase(),
                )
            }
            "--json" => {}
            "--help" | "-h" => {
                println!("Read-only source/evidence readiness: --db PATH --manifest PATH --private-root PATH --log PATH --history-file PATH --title-id ID --json");
                return Ok(());
            }
            _ => {
                return Err(format!(
                    "Unknown argument: {arg}. Installation is not supported by this read-only tool."
                )
                .into())
            }
        }
    }
    let meta = fs::symlink_metadata(&manifest)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_MANIFEST_BYTES {
        return Err("Curated manifest must be a bounded regular file".into());
    }
    let catalog: Catalog = serde_json::from_slice(&fs::read(manifest)?)?;
    if catalog.format != "cheatarium-user-switch-cheat-source-curation-v1" {
        return Err("Unrecognized curated manifest format".into());
    }
    let index = load_game_candidates(&db, "switch")?;
    let live = eden_builds::read_launch_evidence(&log, &index)?;
    let (saved, found_history) = eden_builds::eden_history::load(&history)?;
    let mut by_title = BTreeMap::<String, BTreeSet<String>>::new();
    for entry in &live.main {
        by_title
            .entry(entry.title_id.clone())
            .or_default()
            .insert(entry.build_id.clone());
    }
    for entry in &saved.entries {
        if entry.evidence_kind == "main_nso" {
            by_title
                .entry(entry.title_id.clone())
                .or_default()
                .insert(entry.build_id.clone());
        }
    }
    let mut rows = Vec::new();
    for game in &catalog.games {
        if !valid_title(&game.title_id) {
            return Err("Curated manifest contains malformed Title ID".into());
        }
        if only_title
            .as_ref()
            .is_some_and(|title| title != &game.title_id)
        {
            continue;
        }
        let local = staged(&root, game)?;
        let seen = by_title.get(&game.title_id).cloned().unwrap_or_default();
        rows.push(evaluate(game, &seen, &local));
    }
    if rows.is_empty() {
        return Err("No curated title matched the requested --title-id".into());
    }
    let report = Report {
        format: "cheatarium-eden-private-cheat-readiness-v1",
        read_only: true,
        installed_cheats: 0,
        qt_config_modified: false,
        main_log_found: live.log_found,
        saved_history_found: found_history,
        historical_evidence_is_not_current_version_proof: true,
        games: rows,
        warning: "Build ID match is historical-only. Individual codes may conflict, fail on Eden, or alter saves. Nothing was installed, tested in-game, or modified.",
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("cheatarium-eden-curated-readiness: {e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Game {
        Game {
            title_id: "01002DA013484000".into(),
            title: "Skyward Sword HD".into(),
            hamlet_builds: vec![Build {
                build_id: "5441AF3C89F97E1D".into(),
                strict_native_hex_sections: 57,
            }],
        }
    }
    #[test]
    fn fails_closed_without_observed_build() {
        let game = source();
        let private = BTreeMap::from([("5441AF3C89F97E1D".into(), 57)]);
        assert_eq!(
            evaluate(&game, &BTreeSet::new(), &private).status,
            "blocked_no_historical_main_build_evidence"
        );
    }
    #[test]
    fn matches_only_single_exact_archived_build() {
        let game = source();
        let private = BTreeMap::from([("5441AF3C89F97E1D".into(), 57)]);
        let seen = BTreeSet::from(["5441AF3C89F97E1D".into()]);
        let result = evaluate(&game, &seen, &private);
        assert_eq!(result.native_sections_in_matching_source, 57);
        assert_eq!(
            result.exact_matching_build_id.as_deref(),
            Some("5441AF3C89F97E1D")
        );
        let different = BTreeSet::from(["D824D4B1BBD45D03".into()]);
        assert_eq!(
            evaluate(&game, &different, &private).status,
            "blocked_no_private_source_for_observed_build"
        );
        let conflicting = BTreeSet::from(["5441AF3C89F97E1D".into(), "D824D4B1BBD45D03".into()]);
        assert_eq!(
            evaluate(&game, &conflicting, &private).status,
            "blocked_conflicting_historical_builds"
        );
    }
}
