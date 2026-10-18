use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct MetadataBatch {
    pub rows: Vec<Media>,
    pub errors: Vec<String>,
    pub explicit: bool,
}
#[derive(Default)]
pub(super) struct BulkEdit {
    ids: Vec<String>,
    values: BTreeMap<&'static str, String>,
    enabled: BTreeSet<&'static str>,
}

impl App {
    pub(super) fn compact_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(64.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(themes::palette(&self.state.preferences.theme).panel)
                    .inner_margin(10),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("SSMP").size(13.).strong());
                ui.add_space(12.);
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 92.).max(100.))
                    .show(ui, |ui| {
                        for (icon, label, page) in [
                            (icons::Icon::Home, "Home", Page::Home),
                            (icons::Icon::Music, "Songs", Page::Music),
                            (icons::Icon::Artist, "Artists", Page::Artists),
                            (icons::Icon::Album, "Albums", Page::Albums),
                            (icons::Icon::Video, "Music videos", Page::Videos),
                            (icons::Icon::Heart, "Favorites", Page::Favorites),
                            (icons::Icon::Queue, "Playlists", Page::Playlists),
                            (icons::Icon::Stats, "Statistics", Page::Stats),
                            (icons::Icon::Clock, "History", Page::History),
                            (icons::Icon::Download, "Offline", Page::Offline),
                        ] {
                            if icons::button(ui, icon, self.t(label), self.page == page, 38.)
                                .clicked()
                            {
                                self.page = page;
                                self.search.clear();
                                self.genre_filter.clear();
                                self.sort_key = "Custom order".into();
                            }
                        }
                    });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    if icons::button(
                        ui,
                        icons::Icon::Settings,
                        self.t("Settings"),
                        self.page == Page::Settings,
                        38.,
                    )
                    .clicked()
                    {
                        self.page = Page::Settings;
                    }
                    if icons::button(
                        ui,
                        icons::Icon::Folder,
                        self.t("Sources & server"),
                        self.page == Page::Server,
                        38.,
                    )
                    .clicked()
                    {
                        self.page = Page::Server;
                    }
                });
            });
    }
    pub(super) fn start_metadata(&mut self, ids: Vec<String>, explicit: bool) {
        if self.metadata_running {
            return;
        }
        if self.remote.as_ref().is_some_and(|r| !r.connection.admin) {
            self.notify("Only the owner can edit shared metadata", true);
            return;
        }
        let wanted: BTreeSet<_> = ids.into_iter().collect();
        let rows: Vec<_> = self
            .library()
            .iter()
            .filter(|m| wanted.contains(&m.id) && m.series.is_empty())
            .cloned()
            .collect();
        self.start_metadata_values(rows, explicit);
    }
    pub(super) fn start_metadata_values(&mut self, rows: Vec<Media>, explicit: bool) {
        if self.metadata_running {
            return;
        }
        if self.remote.as_ref().is_some_and(|r| !r.connection.admin) {
            self.notify("Only the owner can edit shared metadata", true);
            return;
        }
        if rows.is_empty() {
            return;
        }
        self.metadata_running = true;
        self.metadata_explicit = explicit;
        self.busy += 1;
        self.metadata_cancel = Default::default();
        let cancel = self.metadata_cancel.clone();
        self.metadata_progress = Some((0, rows.len(), String::new()));
        self.notify("Finding missing information online…", false);
        let root = self.store.root.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<MetadataBatch> {
                let store = Store::open(&root)?;
                let total = rows.len();
                let mut batch = MetadataBatch {
                    rows: vec![],
                    errors: vec![],
                    explicit,
                };
                for (index, mut m) in rows.into_iter().enumerate() {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    let _ = tx.send(Event::MetadataProgress(index, total, m.title.clone()));
                    if explicit {
                        m.locked = false;
                    }
                    match ssmp::metadata::enrich(&store, &m) {
                        Ok(row) => batch.rows.push(row),
                        Err(e) => batch.errors.push(format!("{}: {e}", m.title)),
                    }
                    let _ = tx.send(Event::MetadataProgress(index + 1, total, m.title));
                }
                Ok(batch)
            })();
            let _ = tx.send(Event::Enriched(result.map_err(|e| e.to_string())));
        });
    }
    pub(super) fn accept_metadata(&mut self, batch: MetadataBatch) {
        let mut updated = Vec::new();
        let mut changed = 0;
        if !batch.explicit && !self.state.preferences.online_metadata {
            return;
        }
        for online in batch.rows {
            if let Some(current) = self.library().iter().find(|m| m.id == online.id).cloned() {
                let merged = if batch.explicit {
                    ssmp::metadata::merge_requested(&current, &online)
                } else {
                    ssmp::metadata::merge_missing(&current, &online)
                };
                if [
                    (&merged.genre, &current.genre),
                    (&merged.album, &current.album),
                    (&merged.lyrics, &current.lyrics),
                    (&merged.artwork, &current.artwork),
                    (&merged.release_date, &current.release_date),
                    (&merged.artist_country, &current.artist_country),
                ]
                .iter()
                .any(|(a, b)| a != b)
                {
                    changed += 1;
                }
                if let Some(edit) = self.edit.as_mut().filter(|m| m.id == merged.id) {
                    *edit = ssmp::metadata::merge_requested(edit, &merged);
                }
                updated.push(merged);
            }
        }
        let checked = updated.len();
        if self.remote.is_some() {
            for m in updated {
                self.save_metadata(m);
            }
        } else if let Err(e) = self.store.put_many(&updated) {
            self.notify(e.to_string(), true);
            return;
        } else {
            self.rows = self.store.media().unwrap_or_default();
        }
        self.filtered_cache.get_mut().take();
        self.genre_cache.get_mut().take();
        let status = format!(
            "{}: {checked} · {}: {changed} · {}: {}",
            self.t("Metadata checked"),
            self.t("Updated"),
            self.t("Unavailable"),
            batch.errors.len()
        );
        if let Some(error) = batch.errors.first() {
            self.notify(format!("{status}\n{error}"), true);
        } else {
            self.notify(status, false);
        }
        self.last_metadata = Instant::now();
    }
    pub(super) fn lookup_status(&mut self, ui: &mut egui::Ui) {
        if let Some((done, total, title)) = self
            .metadata_progress
            .clone()
            .filter(|_| self.metadata_running)
        {
            ui.horizontal_wrapped(|ui| {
                ui.spinner();
                ui.label(format!("{done}/{total} · {title}"));
                if ui.small_button(self.t("Cancel")).clicked() {
                    self.metadata_cancel
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            });
        }
        if self.lyrics_pending.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(self.t("Searching lyrics…"));
            });
        }
        if !self.status.is_empty() {
            ui.label(RichText::new(&self.status).color(if self.error {
                Color32::LIGHT_RED
            } else {
                ui.visuals().weak_text_color()
            }));
        }
    }
    pub(super) fn choose_lyrics_file(&mut self) -> Option<String> {
        let path = rfd::FileDialog::new()
            .add_filter("Lyrics", &["lrc", "txt"])
            .pick_file()?;
        match ssmp::library::read_lyrics_file(&path) {
            Ok(text) => Some(text),
            Err(e) => {
                self.notify(e.to_string(), true);
                None
            }
        }
    }
    pub(super) fn apply_lyrics(&mut self, id: &str, text: String) {
        if let Some(edit) = self.edit.as_mut().filter(|m| m.id == id) {
            edit.lyrics = text;
            self.notify("Lyrics found. Save changes to keep them.", false);
            return;
        }
        if self.remote.is_some() {
            self.state.lyrics_overrides.insert(id.into(), text);
            self.changed();
            self.notify("Lyrics saved in your library", false);
        } else if let Some(mut m) = self.rows.iter().find(|m| m.id == id).cloned() {
            m.lyrics = text;
            self.save_metadata(m);
        }
    }
    pub(super) fn select_song(&mut self, id: &str, rows: &[Media], range: bool) {
        self.select_mode = true;
        if range {
            if let Some(anchor) = self
                .selection_anchor
                .as_ref()
                .and_then(|id| rows.iter().position(|m| &m.id == id))
            {
                if let Some(to) = rows.iter().position(|m| m.id == id) {
                    for m in &rows[anchor.min(to)..=anchor.max(to)] {
                        self.selected_songs.insert(m.id.clone());
                    }
                    return;
                }
            }
        }
        if !self.selected_songs.insert(id.into()) {
            self.selected_songs.remove(id);
        }
        self.selection_anchor = Some(id.into());
    }
    pub(super) fn begin_bulk_edit(&mut self) {
        let ids: Vec<_> = self.selected_songs.iter().cloned().collect();
        let mut edit = BulkEdit {
            ids,
            ..Default::default()
        };
        for key in ["Artist", "Album", "Album artist", "Genre", "Release date"] {
            let value = self
                .library()
                .iter()
                .filter(|m| self.selected_songs.contains(&m.id))
                .map(|m| match key {
                    "Artist" => &m.artist,
                    "Album" => &m.album,
                    "Album artist" => &m.album_artist,
                    "Genre" => &m.genre,
                    _ => &m.release_date,
                })
                .next()
                .cloned()
                .unwrap_or_default();
            edit.values.insert(key, value);
        }
        self.bulk_edit = Some(edit);
    }
    pub(super) fn selection_toolbar(&mut self, ui: &mut egui::Ui, rows: &[Media]) {
        ui.horizontal_wrapped(|ui| {
            let label = self.t("Select songs");
            if ui.selectable_label(self.select_mode, label).clicked() {
                self.select_mode = !self.select_mode;
                if !self.select_mode {
                    self.selected_songs.clear();
                }
            }
            if self.select_mode {
                if ui.button(self.t("Select all visible")).clicked() {
                    self.selected_songs
                        .extend(rows.iter().map(|m| m.id.clone()));
                }
                if ui.button(self.t("Clear selection")).clicked() {
                    self.selected_songs.clear();
                }
                ui.label(format!(
                    "{} {}",
                    self.selected_songs.len(),
                    self.t("selected")
                ));
                if !self.selected_songs.is_empty() {
                    if ui.button(self.t("Edit selected")).clicked() {
                        self.begin_bulk_edit();
                    }
                    if ui
                        .add_enabled(
                            !self.metadata_running,
                            egui::Button::new(self.t("Find information online")),
                        )
                        .clicked()
                    {
                        self.start_metadata(self.selected_songs.iter().cloned().collect(), true);
                    }
                    ui.menu_button(self.t("Add to playlist"), |ui| {
                        if ui.button(self.t("Create playlist")).clicked() {
                            self.playlist_seed = self.selected_songs.iter().cloned().collect();
                            self.playlist_create = true;
                            ui.close();
                        }
                        for p in self.state.playlists.clone() {
                            if ui.button(&p.name).clicked() {
                                if let Some(p) =
                                    self.state.playlists.iter_mut().find(|v| v.id == p.id)
                                {
                                    for id in &self.selected_songs {
                                        p.add(id, ssmp::now());
                                    }
                                }
                                self.changed();
                                ui.close();
                            }
                        }
                    });
                    if ui.button(self.t("Add to queue")).clicked() {
                        self.state.queue.extend(self.selected_songs.iter().cloned());
                        self.changed();
                    }
                    if self.page == Page::Playlists
                        && ui.button(self.t("Remove selected from playlist")).clicked()
                    {
                        if let Some(p) = self
                            .state
                            .playlists
                            .iter_mut()
                            .find(|p| Some(&p.id) == self.selected_playlist.as_ref())
                        {
                            p.items.retain(|id| !self.selected_songs.contains(id));
                            p.added.retain(|id, _| !self.selected_songs.contains(id));
                        }
                        self.changed();
                    }
                }
            }
        });
    }
    pub(super) fn bulk_edit_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.bulk_edit.take() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new(RichText::new(self.t("Edit selected")).size(16.).strong())
            .id(egui::Id::new("bulk-editor"))
            .open(&mut open)
            .resizable(false)
            .default_width(520.)
            .default_height(260.)
            .vscroll(true)
            .max_height((ctx.content_rect().height() - 40.).max(160.))
            .show(ctx, |ui| {
                ui.label(format!("{} {}", edit.ids.len(), self.t("selected")));
                ui.label(self.t(
                    "Only checked fields will change. An empty checked field clears that value.",
                ));
                ui.add_space(10.);
                egui::Grid::new("bulk-fields")
                    .spacing([12., 9.])
                    .show(ui, |ui| {
                        for (key, value) in &mut edit.values {
                            let mut enabled = edit.enabled.contains(key);
                            if ui.checkbox(&mut enabled, self.t(key)).changed() {
                                if enabled {
                                    edit.enabled.insert(key);
                                } else {
                                    edit.enabled.remove(key);
                                }
                            }
                            ui.scope(|ui| {
                                ui.set_width(300.);
                                ui.add_enabled(
                                    enabled,
                                    egui::TextEdit::singleline(value).desired_width(300.),
                                );
                            });
                            ui.end_row();
                        }
                    });
                ui.add_space(14.);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !edit.enabled.is_empty(),
                            egui::Button::new(self.t("Save changes")),
                        )
                        .clicked()
                    {
                        save = true;
                    }
                    if ui.button(self.t("Cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        if save {
            let wanted: BTreeSet<_> = edit.ids.into_iter().collect();
            let mut rows: Vec<_> = self
                .library()
                .iter()
                .filter(|m| wanted.contains(&m.id))
                .cloned()
                .collect();
            for m in &mut rows {
                for (key, value) in &edit.values {
                    if edit.enabled.contains(key) {
                        match *key {
                            "Artist" => m.artist = value.clone(),
                            "Album" => m.album = value.clone(),
                            "Album artist" => m.album_artist = value.clone(),
                            "Genre" => m.genre = value.clone(),
                            _ => m.release_date = value.clone(),
                        }
                    }
                }
                m.locked = true;
            }
            if self.remote.is_some() {
                for m in rows {
                    self.save_metadata(m);
                }
            } else {
                match self.store.put_many(&rows) {
                    Ok(()) => {
                        self.rows = self.store.media().unwrap_or_default();
                        self.changed();
                        self.notify("Selected songs updated", false);
                    }
                    Err(e) => self.notify(e.to_string(), true),
                }
            }
        } else if open && !cancel {
            self.bulk_edit = Some(edit);
        }
    }
}
