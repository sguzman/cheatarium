//! Offline, read-only graphical explorer for original Cheatarium source data.
//!
//! No ROM access, cheat activation, source edits, or network requests.
use cheatarium_client::{
    load_all_game_candidates, load_catalog, load_game_candidates, load_platform,
    verify_platform_distribution, CatalogEntry, GameIndex,
};
use eframe::egui::{self, Color32, RichText};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

const MAX_ROW_TITLE_CHARS: usize = 92;

#[derive(Debug)]
struct SourceView {
    id: String,
    filename: String,
    region: String,
    declared_format: String,
    upstream_repository: String,
    upstream_path: String,
    revision: String,
    git_blob_sha: String,
    license: String,
}

#[derive(Debug)]
struct EntryView {
    source_id: String,
    source_file: String,
    ordinal: usize,
    description: String,
    raw_code: Option<String>,
    role: String,
    enabled_upstream: bool,
    verification: String,
    native_fields: Vec<(String, String)>,
    composition_note: Option<String>,
}

fn matches_term(text: &str, needle: &str) -> bool {
    text.to_lowercase().contains(&needle.trim().to_lowercase())
}

fn abbreviated(text: &str, max: usize) -> String {
    let mut iter = text.chars();
    let result: String = iter.by_ref().take(max).collect();
    if iter.next().is_some() {
        format!("{result}…")
    } else {
        result
    }
}

struct LoadedGame {
    title: String,
    sources: Vec<SourceView>,
    entries: Vec<EntryView>,
}

struct GameRequest {
    generation: u64,
    platform: String,
    key: String,
}

struct GameResult {
    generation: u64,
    result: Result<LoadedGame, String>,
}

fn newest_request(mut initial: GameRequest, receiver: &Receiver<GameRequest>) -> GameRequest {
    // Only the most recently selected game is useful once the worker is free.
    while let Ok(replacement) = receiver.try_recv() {
        initial = replacement;
    }
    initial
}

fn load_verified_game(root: &Path, platform: &str, key: &str) -> Result<LoadedGame, String> {
    let result = (|| -> cheatarium_client::Result<_> {
        verify_platform_distribution(root, platform)?;
        let index = load_game_candidates(root, platform)?;
        let candidate = index.find_candidate(key)?;
        let bundle = load_platform(root, platform)?;
        let original_sources = bundle.sources_for_candidate(candidate)?;
        let sources = original_sources
            .iter()
            .map(|record| SourceView {
                id: record.id.clone(),
                filename: record.raw_filename.clone(),
                region: record
                    .region_hint
                    .clone()
                    .unwrap_or_else(|| "not specified".into()),
                declared_format: record
                    .format_hint
                    .clone()
                    .unwrap_or_else(|| "not declared".into()),
                upstream_repository: record.provenance.repository.clone(),
                upstream_path: record.provenance.upstream_path.clone(),
                revision: record.provenance.revision.clone(),
                git_blob_sha: record.provenance.git_blob_sha.clone(),
                license: record.provenance.license.clone(),
            })
            .collect();
        let entries = bundle
            .entries_for_candidate(candidate, None, None)?
            .into_iter()
            .map(|hit| EntryView {
                source_id: hit.source_record_id.to_owned(),
                source_file: hit.raw_filename.to_owned(),
                ordinal: hit.entry.ordinal,
                description: hit
                    .entry
                    .description
                    .clone()
                    .unwrap_or_else(|| "(no description)".into()),
                raw_code: hit.entry.code.clone(),
                role: hit
                    .entry
                    .role
                    .clone()
                    .unwrap_or_else(|| "unspecified".into()),
                enabled_upstream: hit.entry.source_enabled,
                verification: hit.entry.verification.clone(),
                native_fields: hit
                    .entry
                    .native_fields
                    .iter()
                    .map(|field| (field.name.clone(), field.value.clone()))
                    .collect(),
                composition_note: hit.entry.composition.as_ref().map(|c| c.relation.clone()),
            })
            .collect();
        Ok(LoadedGame {
            title: candidate.title_hint.clone(),
            sources,
            entries,
        })
    })();
    result.map_err(|error| error.to_string())
}

/// The selected game is a filename-derived browse group, never a ROM identity.
struct Explorer {
    data_root: PathBuf,
    platforms: Vec<CatalogEntry>,
    indexes: Vec<GameIndex>,
    platform_filter: Option<String>,
    game_search: String,
    games: Vec<(usize, usize)>,
    selected_game: Option<(String, String)>,
    game_title: String,
    sources: Vec<SourceView>,
    entries: Vec<EntryView>,
    source_filter: Option<String>,
    entry_search: String,
    entry_role: Option<String>,
    visible_entries: Vec<usize>,
    selected_entry: Option<usize>,
    error: Option<String>,
    loading_note: String,
    request_sender: Sender<GameRequest>,
    result_receiver: Receiver<GameResult>,
    request_generation: u64,
    pending_request: Option<u64>,
}

impl Explorer {
    fn open(data_root: PathBuf, cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(9.0, 7.0);
            style.spacing.button_padding = egui::vec2(10.0, 6.0);
        });
        let catalog = load_catalog(&data_root);
        let indexes = load_all_game_candidates(&data_root);
        let (platforms, indexes, error) = match (catalog, indexes) {
            (Ok(catalog), Ok(indexes)) => (catalog.bundles, indexes, None),
            (cat, idx) => (
                cat.map(|c| c.bundles).unwrap_or_default(),
                idx.unwrap_or_default(),
                Some("Cannot open verified local title catalogs. Confirm --db points to a complete Cheatarium generated/v1 snapshot.".to_owned()),
            ),
        };
        let (request_sender, pending_requests) = mpsc::channel::<GameRequest>();
        let (results, result_receiver) = mpsc::channel::<GameResult>();
        let worker_root = data_root.clone();
        std::thread::Builder::new()
            .name("cheatarium-source-loader".to_owned())
            .spawn(move || {
                while let Ok(first) = pending_requests.recv() {
                    let request = newest_request(first, &pending_requests);
                    let result = load_verified_game(&worker_root, &request.platform, &request.key);
                    if results
                        .send(GameResult {
                            generation: request.generation,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("Unable to initialize Cheatarium read-only source loader");
        let mut app = Self {
            data_root,
            platforms,
            indexes,
            platform_filter: None,
            game_search: String::new(),
            games: Vec::new(),
            selected_game: None,
            game_title: String::new(),
            sources: Vec::new(),
            entries: Vec::new(),
            source_filter: None,
            entry_search: String::new(),
            entry_role: Some("code".to_owned()),
            visible_entries: Vec::new(),
            selected_entry: None,
            error,
            loading_note: "Browse filename-derived game candidates; source codes are unverified."
                .into(),
            request_sender,
            result_receiver,
            request_generation: 0,
            pending_request: None,
        };
        app.rebuild_games();
        app
    }

    fn rebuild_games(&mut self) {
        let filter = self.game_search.trim().to_lowercase();
        self.games.clear();
        for (platform_index, index) in self.indexes.iter().enumerate() {
            if self
                .platform_filter
                .as_deref()
                .is_some_and(|selected| selected != index.platform)
            {
                continue;
            }
            for (game_index, game) in index.candidates.iter().enumerate() {
                if filter.is_empty()
                    || matches_term(&game.title_hint, &filter)
                    || game
                        .alternate_title_hints
                        .iter()
                        .any(|title| matches_term(title, &filter))
                {
                    self.games.push((platform_index, game_index));
                }
            }
        }
    }

    fn select_game(&mut self, platform: &str, key: &str) {
        if self
            .selected_game
            .as_ref()
            .is_some_and(|(p, k)| p == platform && k == key)
        {
            return;
        }
        self.request_generation = self.request_generation.wrapping_add(1);
        let generation = self.request_generation;
        self.selected_game = Some((platform.to_owned(), key.to_owned()));
        self.game_title = self
            .indexes
            .iter()
            .find(|index| index.platform == platform)
            .and_then(|index| index.find_candidate(key).ok())
            .map(|game| game.title_hint.clone())
            .unwrap_or_else(|| key.to_owned());
        self.sources.clear();
        self.entries.clear();
        self.visible_entries.clear();
        self.selected_entry = None;
        self.entry_search.clear();
        self.entry_role = Some("code".to_owned());
        self.source_filter = None;
        self.error = None;
        self.loading_note = "Verifying local source checksums and loading original entries…".into();
        self.pending_request = Some(generation);
        if self
            .request_sender
            .send(GameRequest {
                generation,
                platform: platform.to_owned(),
                key: key.to_owned(),
            })
            .is_err()
        {
            self.pending_request = None;
            self.error = Some("Cheatarium source loader is no longer available.".into());
        }
    }

    fn collect_finished_loads(&mut self) {
        while let Ok(reply) = self.result_receiver.try_recv() {
            // A result from an older selection is never allowed to replace the
            // game's currently selected original sources.
            if self.pending_request != Some(reply.generation) {
                continue;
            }
            self.pending_request = None;
            match reply.result {
                Ok(LoadedGame {
                    title,
                    sources,
                    entries,
                }) => {
                    self.game_title = title;
                    self.sources = sources;
                    self.entries = entries;
                    self.error = None;
                    self.rebuild_entries();
                    self.loading_note = format!(
                        "{} original sources · {} imported entries · checksums verified against local manifest",
                        self.sources.len(),
                        self.entries.len()
                    );
                }
                Err(error) => {
                    self.error = Some(format!("Unable to open original sources: {error}"));
                    self.loading_note = "Could not load original source records.".into();
                }
            }
        }
    }

    fn rebuild_entries(&mut self) {
        self.visible_entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                self.source_filter
                    .as_deref()
                    .is_none_or(|source| entry.source_id == source)
                    && self
                        .entry_role
                        .as_deref()
                        .is_none_or(|role| entry.role == role)
                    && (self.entry_search.trim().is_empty()
                        || matches_term(&entry.description, &self.entry_search))
            })
            .map(|(i, _)| i)
            .collect();
        if self
            .selected_entry
            .is_some_and(|selected| !self.visible_entries.contains(&selected))
        {
            self.selected_entry = None;
        }
    }

    fn show_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("masthead").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("CHEATARIUM").strong().color(Color32::from_rgb(235, 210, 152)));
                ui.separator();
                ui.label(RichText::new("Originals · Sources · Provenance").small().color(Color32::LIGHT_GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} indexed consoles", self.indexes.len()));
                });
            });
            ui.label(
                RichText::new("Imported cheats are untested. Original 'enabled' flags are archival data, not instructions.")
                    .small()
                    .color(Color32::from_rgb(225, 180, 122)),
            );
        });
    }

    fn show_games(&mut self, ctx: &egui::Context) {
        let mut pick: Option<(String, String)> = None;
        let mut filters_changed = false;
        egui::SidePanel::left("games_sidebar")
            .default_width(325.0)
            .min_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Games");
                ui.horizontal(|ui| {
                    ui.label("Find");
                    filters_changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut self.game_search)
                                .hint_text("Title or alternate title…"),
                        )
                        .changed();
                });
                egui::ComboBox::from_id_salt("platform_filter")
                    .selected_text(self.platform_filter.as_deref().unwrap_or("All consoles"))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        filters_changed |= ui
                            .selectable_value(&mut self.platform_filter, None, "All consoles")
                            .changed();
                        for platform in &self.platforms {
                            filters_changed |= ui
                                .selectable_value(
                                    &mut self.platform_filter,
                                    Some(platform.platform.clone()),
                                    &platform.platform,
                                )
                                .changed();
                        }
                    });
                ui.separator();
                ui.label(format!("{} matching filename candidates", self.games.len()));
                ui.add_space(5.0);
                egui::ScrollArea::vertical()
                    .id_salt("games_scroll")
                    .show_rows(ui, 28.0, self.games.len(), |ui, range| {
                        for row in range {
                            let (platform_idx, game_idx) = self.games[row];
                            let platform = &self.indexes[platform_idx].platform;
                            let game = &self.indexes[platform_idx].candidates[game_idx];
                            let selected = self
                                .selected_game
                                .as_ref()
                                .is_some_and(|(p, key)| p == platform && key == &game.key);
                            let name = abbreviated(&game.title_hint, 42);
                            let label = format!("{name}  ·  {platform}");
                            if ui.selectable_label(selected, label).clicked() {
                                pick = Some((platform.clone(), game.key.clone()));
                            }
                        }
                    });
            });
        if filters_changed {
            self.rebuild_games();
        }
        if let Some((platform, key)) = pick {
            self.select_game(&platform, &key);
        }
    }

    fn show_details(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("source_inspector")
            .default_width(335.0)
            .min_width(265.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Original entry");
                ui.separator();
                let Some(entry_id) = self.selected_entry else {
                    ui.label("Select an original source entry to inspect its code and provenance.");
                    return;
                };
                let Some(entry) = self.entries.get(entry_id) else { return };
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new(&entry.description).heading().strong());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Ordinal #{}", entry.ordinal)).monospace());
                        ui.label(RichText::new(&entry.role).small().color(Color32::GRAY));
                    });
                    ui.separator();
                    ui.label(RichText::new("ORIGINAL CODE").strong());
                    if let Some(raw) = &entry.raw_code {
                        ui.label(RichText::new(raw).monospace().color(Color32::from_rgb(200, 219, 243)));
                        if ui.button("Copy original text").clicked() {
                            ui.ctx().copy_text(raw.clone());
                        }
                    } else {
                        ui.label("No device-code string in this source entry");
                    }
                    if !entry.native_fields.is_empty() {
                        ui.separator();
                        ui.label(RichText::new("NATIVE FIELDS").strong());
                        for (name, value) in &entry.native_fields {
                            ui.monospace(format!("{name} = {value}"));
                        }
                    }
                    ui.separator();
                    ui.label(format!("Imported status: {}", entry.verification));
                    ui.label(format!("Marked enabled by upstream: {}", entry.enabled_upstream));
                    if let Some(note) = &entry.composition_note {
                        ui.label(RichText::new(format!("Composition: {note}")).color(Color32::from_rgb(239, 185, 121)));
                    }
                    ui.small("Imported metadata does not establish compatibility or verified gameplay effects.");
                    ui.separator();
                    ui.label(RichText::new("PROVENANCE").strong());
                    ui.label(format!("Original file: {}", entry.source_file));
                    ui.monospace(&entry.source_id);
                    if let Some(source) = self.sources.iter().find(|source| source.id == entry.source_id) {
                        ui.label(format!("Region hint: {}", source.region));
                        ui.label(format!("Declared format: {}", source.declared_format));
                        ui.label(format!("Original source collection: {}", source.upstream_repository));
                        ui.label(format!("Upstream path: {}", source.upstream_path));
                        ui.label(format!("License: {}", source.license));
                        ui.label(format!("Revision: {}", source.revision));
                        ui.label(RichText::new(format!("Git blob: {}", source.git_blob_sha)).monospace().small());
                        if ui.button("Copy source reference").clicked() {
                            ui.ctx().copy_text(format!("{} #{}", source.id, entry.ordinal));
                        }
                    }
                });
            });
    }

    fn show_entries(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.label(RichText::new(error).color(Color32::from_rgb(246, 132, 125)));
                ui.separator();
            }
            let Some((platform, key)) = &self.selected_game else {
                ui.heading("Explore original cheats");
                ui.add_space(10.0);
                ui.label("Choose a game from the left. Each candidate is grouped from filenames, not verified ROM identity.");
                ui.label("The detail view preserves every source's original ordinal and text; no cheats are executed.");
                ui.separator();
                ui.monospace(format!("Local data: {}", self.data_root.display()));
                return;
            };
            ui.heading(format!("{} · {}", self.game_title, platform));
            ui.label(RichText::new(format!("Candidate key: {key}")).small().color(Color32::GRAY));
            ui.label(&self.loading_note);
            if self.pending_request.is_some() {
                ui.add(egui::Spinner::new());
                return;
            }
            ui.separator();
            let mut filter_changed = false;
            ui.horizontal(|ui| {
                ui.label("Description");
                filter_changed |= ui
                    .add(egui::TextEdit::singleline(&mut self.entry_search).hint_text("Search original wording…"))
                    .changed();
            });
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("source_filter")
                    .selected_text(
                        self.source_filter.as_deref().and_then(|id| {
                            self.sources.iter().find(|s| s.id == id).map(|s| s.filename.as_str())
                        }).unwrap_or("All original files"),
                    )
                    .width(235.0)
                    .show_ui(ui, |ui| {
                        filter_changed |= ui.selectable_value(&mut self.source_filter, None, "All original files").changed();
                        for source in &self.sources {
                            filter_changed |= ui
                                .selectable_value(&mut self.source_filter, Some(source.id.clone()), abbreviated(&source.filename, 48))
                                .changed();
                        }
                    });
                egui::ComboBox::from_id_salt("role_filter")
                    .selected_text(self.entry_role.as_deref().unwrap_or("All entry types"))
                    .show_ui(ui, |ui| {
                        filter_changed |= ui.selectable_value(&mut self.entry_role, None, "All entry types").changed();
                        for role in ["code", "memory-entry", "section-heading"] {
                            filter_changed |= ui
                                .selectable_value(&mut self.entry_role, Some(role.to_owned()), role)
                                .changed();
                        }
                    });
            });
            if filter_changed {
                self.rebuild_entries();
            }
            ui.label(format!("{} matching original entries · no text merged", self.visible_entries.len()));
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("entries_scroll")
                .show_rows(ui, 28.0, self.visible_entries.len(), |ui, range| {
                    for row in range {
                        let original = self.visible_entries[row];
                        let entry = &self.entries[original];
                        let title = abbreviated(&entry.description, MAX_ROW_TITLE_CHARS);
                        let label = format!("#{}  {}  ·  {}", entry.ordinal, title, abbreviated(&entry.source_file, 22));
                        if ui.selectable_label(self.selected_entry == Some(original), label).clicked() {
                            self.selected_entry = Some(original);
                        }
                    }
                });
        });
    }
}

impl eframe::App for Explorer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.collect_finished_loads();
        if self.pending_request.is_some() {
            ctx.request_repaint_after(Duration::from_millis(60));
        }
        self.show_header(ctx);
        self.show_games(ctx);
        self.show_details(ctx);
        self.show_entries(ctx);
    }
}

fn data_root_from_args() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from("generated/v1");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => root = PathBuf::from(args.next().ok_or("--db needs a directory")?),
            _ => {
                return Err(format!(
                    "Unknown option {arg}. Usage: cheatarium-explorer [--db generated/v1]"
                ))
            }
        }
    }
    Ok(root)
}

fn main() -> eframe::Result {
    let root = match data_root_from_args() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("cheatarium-explorer: {error}");
            std::process::exit(2);
        }
    };
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1280.0, 790.0))
            .with_min_inner_size(egui::vec2(900.0, 580.0)),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "Cheatarium",
        native_options,
        Box::new(move |cc| Ok(Box::new(Explorer::open(root.clone(), cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_game_selections_discard_superseded_pending_requests() {
        let (sender, receiver) = mpsc::channel();
        for (generation, key) in [(1, "first"), (2, "second"), (3, "latest")] {
            sender
                .send(GameRequest {
                    generation,
                    platform: "snes".into(),
                    key: key.into(),
                })
                .unwrap();
        }
        let first = receiver.recv().unwrap();
        let newest = newest_request(first, &receiver);
        assert_eq!(newest.generation, 3);
        assert_eq!(newest.key, "latest");
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn title_matching_trims_query_but_never_rewrites_originals() {
        assert!(matches_term("Infinite Lives", " LiVeS "));
        assert!(matches_term("Mario World", "mARio"));
        assert!(!matches_term("Start on Level 2", "level 3"));
    }

    #[test]
    fn short_row_titles_are_unicode_safe() {
        assert_eq!(abbreviated("Ä ö 🎮", 4), "Ä ö …");
        assert_eq!(abbreviated("ABC", 4), "ABC");
    }

    #[test]
    fn distinct_original_ordinals_stay_distinct() {
        let rows = [
            EntryView {
                source_id: "x".into(),
                source_file: "Sample".into(),
                ordinal: 3,
                description: "Start on level 2".into(),
                raw_code: Some("ABCD".into()),
                role: "code".into(),
                enabled_upstream: false,
                verification: "unverified".into(),
                native_fields: Vec::new(),
                composition_note: None,
            },
            EntryView {
                source_id: "x".into(),
                source_file: "Sample".into(),
                ordinal: 101,
                description: "Start On Level 2".into(),
                raw_code: Some("ABCD".into()),
                role: "code".into(),
                enabled_upstream: false,
                verification: "unverified".into(),
                native_fields: Vec::new(),
                composition_note: None,
            },
        ];
        assert_eq!(rows.len(), 2);
        assert_ne!(rows[0].ordinal, rows[1].ordinal);
        assert_eq!(rows[0].raw_code, rows[1].raw_code);
    }
}
