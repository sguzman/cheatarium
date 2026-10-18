//! Strictly read-only, detailed human and JSON preview of the *same* source
//! bytes that the on-demand Eden installer would create. No mutation APIs.
use super::{eden_install, prepare_addons, AutomaticReport, Bundle, ErrBox};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

#[derive(Debug, Serialize)]
pub(super) struct ProposedFile {
    pub(super) title_id: String,
    pub(super) build_id: String,
    pub(super) cheat_name: String,
    pub(super) source_record_id: String,
    pub(super) source_archive_path: String,
    pub(super) destination: String,
    pub(super) byte_count: usize,
    pub(super) sha256: String,
    pub(super) exact_proposed_contents: String,
    pub(super) status: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct DisabledSettingsPreview {
    pub(super) config_path: String,
    pub(super) current_config_sha256: Option<String>,
    pub(super) proposed_config_sha256: Option<String>,
    pub(super) would_update_config: bool,
    pub(super) proposed_disabled_addon_names: BTreeMap<String, Vec<String>>,
    pub(super) proposed_disabled_settings_section: Option<String>,
    pub(super) issue: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct HistoricalBuildMatch {
    pub(super) title_id: String,
    pub(super) build_id: String,
    pub(super) archived_cheat_candidate_in_plan: bool,
    pub(super) current_installed_build_verified: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct MainBuildEvidence {
    pub(super) log_path: String,
    pub(super) log_found: bool,
    pub(super) historical_main_builds: Vec<HistoricalBuildMatch>,
    pub(super) all_games_current_build_verified: bool,
    pub(super) warning: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct DryRunReport<'a> {
    pub(super) format: &'static str,
    pub(super) read_only: bool,
    pub(super) emulator_files_written: usize,
    pub(super) state_files_written: usize,
    pub(super) user_confirmation_required_before_install: bool,
    pub(super) discovery: &'a super::eden_auto::Discovery,
    pub(super) candidate_build_scope: &'static str,
    pub(super) main_build_evidence: MainBuildEvidence,
    pub(super) source_plans: &'a [super::Preview],
    pub(super) skipped: &'a [super::SkippedAutomaticBuild],
    pub(super) files: Vec<ProposedFile>,
    pub(super) would_create_file_count: usize,
    pub(super) already_installed_file_count: usize,
    pub(super) collision_file_count: usize,
    pub(super) settings: DisabledSettingsPreview,
    pub(super) caveat: &'static str,
}

fn section(ini: &str) -> String {
    let mut in_disabled = false;
    let mut section_lines = Vec::new();
    for line in ini.lines() {
        if line.trim().starts_with('[') && line.trim().ends_with(']') {
            if in_disabled {
                break;
            }
            in_disabled = line.trim().eq_ignore_ascii_case("[DisabledAddOns]");
        }
        if in_disabled {
            section_lines.push(line.to_owned());
        }
    }
    section_lines.join("\n")
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn analyze<'a>(
    report: &'a AutomaticReport,
    bundle: &Bundle,
    config: &Path,
    mods_root: &Path,
    log_path: &Path,
    evidence: &super::eden_builds::LaunchEvidence,
) -> Result<DryRunReport<'a>, ErrBox> {
    let mut files = Vec::new();
    let mut to_disable = BTreeMap::<String, Vec<String>>::new();
    let mut already = 0usize;
    let mut conflicts = 0usize;
    let mut will_create = 0usize;
    for plan in &report.planned {
        // Use identical rendering to the installer, including exact source
        // checks and size/section restrictions, but never call install().
        let prepared = prepare_addons(bundle, plan, mods_root)?;
        let unchanged = if mods_root.exists() && config.exists() {
            eden_install::already_installed_unchanged(bundle, plan, mods_root, config)
                .unwrap_or(false)
        } else {
            false
        };
        for (entry, bytes) in prepared {
            let code = plan
                .proposed_addons
                .iter()
                .find(|code| {
                    code.source_record_id == entry.source_record_id
                        && code.source_ordinal == entry.source_ordinal
                })
                .ok_or("Planned cheat lost its source identity")?;
            let relative = Path::new(&entry.relative_path);
            let destination = mods_root.join(relative);
            let addon = relative
                .components()
                .nth(1)
                .ok_or("Missing planned addon directory")?
                .as_os_str()
                .to_string_lossy()
                .into_owned();
            let addon_path = mods_root.join(&plan.title_id).join(&addon);
            let status = if unchanged {
                already += 1;
                "already_installed_identical_and_disabled"
            } else {
                match fs::symlink_metadata(&addon_path) {
                    Ok(_) => {
                        conflicts += 1;
                        "blocked_existing_mod_directory"
                    }
                    Err(err) if err.kind() == ErrorKind::NotFound => {
                        will_create += 1;
                        to_disable
                            .entry(plan.title_id.clone())
                            .or_default()
                            .push(addon);
                        "would_create_disabled"
                    }
                    Err(_) => {
                        conflicts += 1;
                        "blocked_mod_directory_unreadable"
                    }
                }
            };
            files.push(ProposedFile {
                title_id: plan.title_id.clone(),
                build_id: plan.explicitly_selected_build_id.clone(),
                cheat_name: code.description.clone(),
                source_record_id: entry.source_record_id,
                source_archive_path: code.source_archive_path.clone(),
                destination: destination.to_string_lossy().into_owned(),
                byte_count: bytes.len(),
                sha256: sha(&bytes),
                exact_proposed_contents: String::from_utf8(bytes)?,
                status,
            });
        }
    }
    let mut settings = DisabledSettingsPreview {
        config_path: config.to_string_lossy().into_owned(),
        current_config_sha256: None,
        proposed_config_sha256: None,
        would_update_config: false,
        proposed_disabled_addon_names: to_disable.clone(),
        proposed_disabled_settings_section: None,
        issue: None,
    };
    match fs::symlink_metadata(config) {
        Ok(meta)
            if meta.is_file()
                && !meta.file_type().is_symlink()
                && meta.len() <= 8 * 1024 * 1024 =>
        {
            let current = fs::read_to_string(config)?;
            settings.current_config_sha256 = Some(sha(current.as_bytes()));
            let mut proposed = current.clone();
            for (title, names) in &to_disable {
                match eden_install::disabled_config(&proposed, title, names) {
                    Ok(next) => proposed = next,
                    Err(err) => {
                        settings.issue = Some(format!(
                            "Eden settings invalid; installation blocked: {err}"
                        ));
                        break;
                    }
                }
            }
            if settings.issue.is_none() {
                settings.would_update_config = proposed != current;
                settings.proposed_config_sha256 = Some(sha(proposed.as_bytes()));
                settings.proposed_disabled_settings_section = Some(section(&proposed));
            }
        }
        Ok(_) => {
            settings.issue = Some(
                "Eden settings are symlinked, oversized or nonregular; installation blocked".into(),
            )
        }
        Err(err) => {
            settings.issue = Some(format!(
                "Eden config unavailable; installation blocked: {err}"
            ))
        }
    }
    // Only launch-context-linked main NSO build observations are shown as
    // evidence. A historical launch is never current-install verification.
    let mut seen = BTreeSet::new();
    let mut historical_main_builds = Vec::new();
    for item in &evidence.main {
        let key = (item.title_id.clone(), item.build_id.clone());
        if !seen.insert(key) {
            continue;
        }
        historical_main_builds.push(HistoricalBuildMatch {
            archived_cheat_candidate_in_plan: report.planned.iter().any(|plan| {
                plan.title_id == item.title_id && plan.explicitly_selected_build_id == item.build_id
            }),
            title_id: item.title_id.clone(),
            build_id: item.build_id.clone(),
            current_installed_build_verified: false,
        });
    }
    Ok(DryRunReport {
        format: "cheatarium-eden-detailed-dry-run-v1",
        read_only: true,
        emulator_files_written: 0,
        state_files_written: 0,
        user_confirmation_required_before_install: true,
        discovery: &report.discovery,
        candidate_build_scope: report.candidate_build_scope,
        main_build_evidence: MainBuildEvidence {
            log_path: log_path.to_string_lossy().into_owned(),
            log_found: evidence.log_found,
            historical_main_builds,
            all_games_current_build_verified: false,
            warning: "Historical Eden main NSO logs are not proof of currently installed executable Build IDs; archived-only builds remain unverified.",
        },
        source_plans: &report.planned,
        skipped: &report.skipped,
        files,
        would_create_file_count: will_create,
        already_installed_file_count: already,
        collision_file_count: conflicts,
        settings,
        caveat: "An archived Build ID is not proof of the installed executable version. This report does not create folders, install mods, update Qt settings, enable cheats, or write any file.",
    })
}

pub(super) fn render_human(report: &DryRunReport<'_>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    writeln!(out, "CHEATARIUM / EDEN — READ-ONLY DRY RUN").unwrap();
    writeln!(out, "NO FILES WRITTEN. NO CHEATS ENABLED.").unwrap();
    writeln!(
        out,
        "CANDIDATE BUILD SCOPE: {}",
        report.candidate_build_scope
    )
    .unwrap();
    writeln!(out, "Discovered {} candidate games; inspected {} recognized game file names across {} directories.",
        report.discovery.games.len(),
        report.discovery.recognized_game_files,
        report.discovery.scanned_directories).unwrap();
    writeln!(
        out,
        "EDEN MAIN BUILD LOG: {} (found: {})",
        report.main_build_evidence.log_path, report.main_build_evidence.log_found
    )
    .unwrap();
    writeln!(out, "{}", report.main_build_evidence.warning).unwrap();
    for game in &report.discovery.games {
        writeln!(out, "\nGAME: {} [{}]", game.display_name, game.title_id).unwrap();
        writeln!(
            out,
            "  Name source: {}; discovery: {}",
            game.name_evidence,
            game.discovery_evidence.join(", ")
        )
        .unwrap();
        if let Some(path) = &game.sample_game_file {
            writeln!(out, "  ROM filename witness: {path}").unwrap();
        }
        let history: Vec<_> = report
            .main_build_evidence
            .historical_main_builds
            .iter()
            .filter(|item| item.title_id == game.title_id)
            .collect();
        if history.is_empty() {
            writeln!(
                out,
                "  HISTORICAL MAIN BUILD: none observed in current Eden log"
            )
            .unwrap();
        } else {
            for item in &history {
                writeln!(
                    out,
                    "  HISTORICAL MAIN BUILD: {} (archived cheats proposed: {}; NOT verified as currently installed)",
                    item.build_id, item.archived_cheat_candidate_in_plan
                ).unwrap();
            }
        }
        let plans: Vec<_> = report
            .source_plans
            .iter()
            .filter(|p| p.title_id == game.title_id)
            .collect();
        if plans.is_empty() {
            writeln!(out, "  Archived cheats: NONE eligible for installation").unwrap();
        }
        for plan in plans {
            writeln!(
                out,
                "  ARCHIVED BUILD: {} ({})",
                plan.explicitly_selected_build_id,
                if history
                    .iter()
                    .any(|item| item.build_id == plan.explicitly_selected_build_id)
                {
                    "historically observed main NSO; NOT verified current"
                } else {
                    "not observed as main in current log; NOT verified current"
                }
            )
            .unwrap();
            for file in report.files.iter().filter(|f| {
                f.title_id == game.title_id && f.build_id == plan.explicitly_selected_build_id
            }) {
                writeln!(out, "    CHEAT: {}", file.cheat_name).unwrap();
                writeln!(out, "    SOURCE: {}", file.source_archive_path).unwrap();
                writeln!(out, "    WOULD WRITE: {}", file.destination).unwrap();
                writeln!(
                    out,
                    "    STATUS: {} | bytes: {} | SHA256: {}",
                    file.status, file.byte_count, file.sha256
                )
                .unwrap();
                writeln!(
                    out,
                    "    EXACT CONTENTS:\n{}",
                    file.exact_proposed_contents.trim_end()
                )
                .unwrap();
            }
        }
    }
    for skipped in report.skipped {
        writeln!(
            out,
            "\nSKIPPED: {} / {} — {}",
            skipped.title_id,
            skipped.build_id.as_deref().unwrap_or("no archived build"),
            skipped.reason
        )
        .unwrap();
    }
    writeln!(out, "\nEDEN SETTINGS: {}", report.settings.config_path).unwrap();
    writeln!(
        out,
        "Would change config: {}",
        report.settings.would_update_config
    )
    .unwrap();
    if let Some(err) = &report.settings.issue {
        writeln!(out, "SETTINGS PROBLEM: {err}").unwrap();
    }
    if let Some(changes) = &report.settings.proposed_disabled_settings_section {
        writeln!(out, "PROPOSED [DisabledAddOns] SECTION:\n{changes}").unwrap();
    }
    writeln!(
        out,
        "\nTOTAL: {} files would be created, {} already installed, {} blocked by collisions",
        report.would_create_file_count,
        report.already_installed_file_count,
        report.collision_file_count
    )
    .unwrap();
    writeln!(out, "{}", report.caveat).unwrap();
    out
}
