use super::icons::{self, Icon};
use super::*;
use ssmp::music;
use std::collections::{BTreeMap, BTreeSet};

impl App {
    pub(super) fn t(&self, key: &'static str) -> &'static str {
        i18n::tr(&self.state.preferences.language, key)
    }
    pub(super) fn accent(&self) -> Color32 {
        Color32::from_rgb(
            self.state.preferences.accent[0],
            self.state.preferences.accent[1],
            self.state.preferences.accent[2],
        )
    }
    pub(super) fn set_theme(&mut self, name: &str, ctx: &egui::Context) {
        let p = themes::palette(name);
        self.state.preferences.theme = name.into();
        self.state.preferences.accent = [p.accent.r(), p.accent.g(), p.accent.b()];
        self.apply_preferences(ctx);
        self.changed();
    }
    pub(super) fn apply_preferences(&self, ctx: &egui::Context) {
        let p = themes::palette(&self.state.preferences.theme);
        let mut v = if p.light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        v.panel_fill = p.bg;
        v.window_fill = p.panel;
        v.extreme_bg_color = themes::mix(p.bg, p.panel, 0.35);
        v.faint_bg_color = themes::mix(p.bg, p.panel, 0.65);
        v.override_text_color = Some(p.text);
        v.weak_text_color = Some(p.muted);
        v.window_stroke = Stroke::new(1_f32, p.border);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1_f32, p.border);
        v.widgets.inactive.bg_fill = themes::mix(p.panel, p.text, 0.04);
        v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        v.widgets.inactive.bg_stroke = Stroke::new(1_f32, p.border);
        v.widgets.hovered.bg_fill = themes::mix(p.panel, self.accent(), 0.16);
        v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
        v.widgets.hovered.bg_stroke = Stroke::new(1_f32, self.accent());
        v.widgets.active.bg_fill = themes::mix(p.panel, self.accent(), 0.25);
        v.widgets.active.bg_stroke = Stroke::new(1_f32, self.accent());
        v.selection.bg_fill = themes::mix(p.panel, self.accent(), 0.18);
        v.selection.stroke = Stroke::new(1_f32, self.accent());
        ctx.set_visuals(v);
        let mut style = (*ctx.style()).clone();
        style.animation_time = if self.state.preferences.animations {
            0.18
        } else {
            0.
        };
        style.spacing.item_spacing = Vec2::new(8., 8.);
        style.spacing.button_padding = Vec2::new(13., 9.);
        style.spacing.slider_width = 160.;
        for (key, size) in [
            (egui::TextStyle::Body, 14.),
            (egui::TextStyle::Button, 14.),
            (egui::TextStyle::Small, 11.),
            (egui::TextStyle::Heading, 28.),
        ] {
            style
                .text_styles
                .insert(key, egui::FontId::proportional(size));
        }
        ctx.set_style(style);
        ctx.set_zoom_factor(self.state.preferences.scale);
    }
    pub(super) fn apply_audio_preferences(&mut self) {
        if let Some(p) = self.player.as_mut() {
            p.set("volume", &self.state.preferences.volume.to_string());
            p.set("speed", &self.state.preferences.speed.to_string());
            if !self.smoke {
                p.set("audio-device", &self.state.preferences.audio_device);
            }
            p.set(
                "af",
                if self.state.preferences.normalize {
                    "lavfi=[loudnorm=I=-16:TP=-1.5:LRA=11]"
                } else {
                    ""
                },
            );
        }
    }
    pub(super) fn step(&mut self, delta: i32) {
        if self.state.queue.is_empty() {
            return;
        }
        if delta < 0
            && self
                .player
                .as_ref()
                .is_some_and(|p| music::previous_restarts(p.snapshot.position))
        {
            if let Some(p) = self.player.as_mut() {
                p.seek(0.);
                p.set("pause", "no");
            }
            return;
        }
        let old = self.state.queue_index;
        if delta > 0
            && !self.state.preferences.shuffle
            && self.repeat == 0
            && old + 1 >= self.state.queue.len()
        {
            if let Some(p) = self.player.as_mut() {
                p.seek(p.snapshot.duration);
                p.set("pause", "yes");
            }
            return;
        }
        let next = if delta < 0 {
            self.queue_back
                .pop()
                .unwrap_or_else(|| old.saturating_sub(1))
        } else if self.state.preferences.shuffle {
            music::shuffled_next(&self.state, self.library(), &mut rand::thread_rng())
                .unwrap_or(old)
        } else {
            (old + 1) % self.state.queue.len()
        };
        if delta > 0 {
            self.queue_back.push(old);
            if self.queue_back.len() > 1000 {
                self.queue_back.remove(0);
            }
        }
        self.state.queue_index = next;
        self.load_queue(0.);
        self.changed();
    }
    pub(super) fn filtered(&self) -> std::sync::Arc<Vec<Media>> {
        let key = format!(
            "{:?}",
            (
                &self.page,
                &self.search,
                &self.search_field,
                &self.genre_filter,
                &self.sort_key,
                self.sort_desc,
                &self.selected_artist,
                &self.selected_album,
                &self.selected_playlist,
                if self.page == Page::Offline {
                    ssmp::now() / 5
                } else {
                    0
                }
            )
        );
        if let Some((cached_key, rows)) = self.filtered_cache.borrow().as_ref() {
            if cached_key == &key {
                return rows.clone();
            }
        }
        let query = self.search.trim().to_lowercase();
        let mut rows: Vec<_> = self
            .library()
            .iter()
            .filter(|m| m.series.is_empty())
            .filter(|m| match self.page {
                Page::Music | Page::Albums | Page::Artists | Page::Artist | Page::Album => {
                    m.kind == "audio"
                }
                Page::Videos => m.kind == "video",
                Page::Favorites => self.state.favorites.contains(&m.id),
                Page::History => self.state.counts.contains_key(&m.id),
                Page::Offline => std::path::Path::new(&m.path).is_file(),
                Page::Playlists => self
                    .selected_playlist
                    .as_ref()
                    .and_then(|id| self.state.playlists.iter().find(|p| &p.id == id))
                    .is_some_and(|p| p.items.contains(&m.id)),
                _ => true,
            })
            .filter(|m| {
                self.page != Page::Artist
                    || self
                        .selected_artist
                        .as_ref()
                        .is_some_and(|a| a.eq_ignore_ascii_case(&m.artist))
            })
            .filter(|m| {
                self.page != Page::Album
                    || self
                        .selected_album
                        .as_ref()
                        .is_some_and(|key| key == &music::album_key(m))
            })
            .filter(|m| {
                self.genre_filter.is_empty()
                    || m.genre
                        .to_lowercase()
                        .contains(&self.genre_filter.to_lowercase())
            })
            .filter(|m| {
                let haystack = match self.search_field.as_str() {
                    "Title" => m.title.clone(),
                    "Artist" => m.artist.clone(),
                    "Album" => m.album.clone(),
                    "Genre" => m.genre.clone(),
                    _ => format!("{} {} {} {}", m.title, m.artist, m.album, m.genre),
                };
                query.is_empty() || haystack.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
        if self.page == Page::Playlists && self.sort_key == "Custom order" {
            if let Some(p) = self
                .selected_playlist
                .as_ref()
                .and_then(|id| self.state.playlists.iter().find(|p| &p.id == id))
            {
                rows = p
                    .items
                    .iter()
                    .filter_map(|id| rows.iter().find(|m| &m.id == id).cloned())
                    .collect();
            }
        } else if self.page == Page::Playlists && self.sort_key == "Recently added" {
            if let Some(p) = self
                .selected_playlist
                .as_ref()
                .and_then(|id| self.state.playlists.iter().find(|p| &p.id == id))
            {
                rows.sort_by_key(|m| std::cmp::Reverse(p.added.get(&m.id).copied().unwrap_or(0)));
                if self.sort_desc {
                    rows.reverse();
                }
            }
        } else if self.page == Page::History {
            rows.sort_by_key(|m| {
                std::cmp::Reverse(
                    self.state
                        .history
                        .iter()
                        .rfind(|h| h.id == m.id)
                        .map(|h| h.at)
                        .unwrap_or(0),
                )
            });
        } else {
            music::sort_tracks(&mut rows, &self.sort_key, self.sort_desc);
        }
        let rows = std::sync::Arc::new(rows);
        *self.filtered_cache.borrow_mut() = Some((key, rows.clone()));
        rows
    }
    pub(super) fn open_artist(&mut self, artist: &str) {
        self.selected_artist = Some(artist.into());
        self.page = Page::Artist;
        self.search.clear();
        self.genre_filter.clear();
    }
    fn open_album(&mut self, m: &Media) {
        self.selected_album = Some(music::album_key(m));
        self.page = Page::Album;
        self.search.clear();
        self.genre_filter.clear();
    }
    fn play_rows(&mut self, rows: &[Media], shuffle: bool) {
        if rows.is_empty() {
            return;
        }
        self.state.queue_playlist = if self.page == Page::Playlists {
            self.selected_playlist.clone()
        } else {
            None
        };
        self.state.queue = rows.iter().map(|m| m.id.clone()).collect();
        self.state.queue_index = 0;
        self.queue_back.clear();
        self.state.preferences.shuffle = shuffle;
        if shuffle {
            use rand::Rng;
            self.state.queue_index = rand::thread_rng().gen_range(0..rows.len());
        }
        self.load_queue(0.);
    }
    pub(super) fn sidebar(&mut self, ctx: &egui::Context) {
        if ctx.content_rect().width() < 850. {
            self.compact_sidebar(ctx);
            return;
        }
        egui::SidePanel::left("navigation")
            .exact_width(208.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(themes::palette(&self.state.preferences.theme).panel)
                    .inner_margin(16),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.;
                ui.add_space(9.);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(30.), egui::Sense::hover());
                    icons::draw(ui.painter(), r, Icon::Music, self.accent());
                    ui.label(RichText::new("SSMP").size(24.).strong());
                });
                ui.label(RichText::new(self.t("Simple Music Player")).small().weak());
                ui.add_space(22.);
                let height = (ui.available_height() - 100.).max(200.);
                egui::ScrollArea::vertical()
                    .id_salt("nav-scroll")
                    .max_height(height)
                    .show(ui, |ui| {
                        for (icon, label, page) in [
                            (Icon::Home, "Home", Page::Home),
                            (Icon::Music, "Songs", Page::Music),
                            (Icon::Artist, "Artists", Page::Artists),
                            (Icon::Album, "Albums", Page::Albums),
                            (Icon::Video, "Music videos", Page::Videos),
                            (Icon::Heart, "Favorites", Page::Favorites),
                            (Icon::Queue, "Playlists", Page::Playlists),
                            (Icon::Stats, "Statistics", Page::Stats),
                            (Icon::Clock, "History", Page::History),
                            (Icon::Download, "Offline", Page::Offline),
                        ] {
                            self.navigation(ui, icon, label, page);
                        }
                        ui.add_space(22.);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(self.t("YOUR LIBRARY")).small().weak());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if icons::button(
                                        ui,
                                        Icon::Plus,
                                        self.t("Create playlist"),
                                        false,
                                        25.,
                                    )
                                    .clicked()
                                    {
                                        self.playlist_create = true;
                                        self.playlist_seed.clear();
                                    }
                                },
                            );
                        });

                        for p in self.state.playlists.clone() {
                            let selected = self.selected_playlist.as_ref() == Some(&p.id)
                                && self.page == Page::Playlists;
                            if ui
                                .add_sized(
                                    [ui.available_width(), 30.],
                                    egui::Button::new(RichText::new(&p.name).small())
                                        .selected(selected)
                                        .frame(selected),
                                )
                                .clicked()
                            {
                                self.selected_playlist = Some(p.id);
                                self.page = Page::Playlists;
                                self.search.clear();
                            }
                        }
                    });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.label(RichText::new("SSMP 0.3 · GPL-3.0").small().weak());
                    self.navigation(ui, Icon::Settings, "Settings", Page::Settings);
                    self.navigation(ui, Icon::Folder, "Sources & server", Page::Server);
                });
            });
    }
    fn navigation(&mut self, ui: &mut egui::Ui, icon: Icon, label: &'static str, page: Page) {
        let active = self.page == page;
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(
                ui.available_width(),
                if ui.ctx().content_rect().height() < 780. {
                    32.
                } else {
                    38.
                },
            ),
            egui::Sense::click(),
        );
        let h = ui
            .ctx()
            .animate_bool(response.id.with("nav"), response.hovered());
        if active || h > 0.001 {
            ui.painter().rect_filled(
                rect,
                6.,
                if active {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().selection.bg_fill.gamma_multiply(h * 0.5)
                },
            );
        }
        icons::draw(
            ui.painter(),
            egui::Rect::from_center_size(
                egui::pos2(rect.left() + 18., rect.center().y),
                Vec2::splat(28.),
            ),
            icon,
            if active {
                self.accent()
            } else {
                ui.visuals().weak_text_color()
            },
        );
        ui.painter().text(
            egui::pos2(rect.left() + 40., rect.center().y),
            egui::Align2::LEFT_CENTER,
            self.t(label),
            egui::FontId::proportional(14.),
            if active {
                self.accent()
            } else {
                ui.visuals().text_color()
            },
        );
        if response.clicked() {
            self.page = page;
            self.search.clear();
            self.genre_filter.clear();
            self.sort_key = "Custom order".into();
        }
    }
    pub(super) fn bottom(&mut self, ctx: &egui::Context) {
        let narrow = ctx.content_rect().width() < 900.;
        let height = if narrow { 154. } else { 110. };
        egui::TopBottomPanel::bottom("transport")
            .exact_height(height)
            .frame(
                egui::Frame::new()
                    .fill(themes::palette(&self.state.preferences.theme).panel)
                    .inner_margin(egui::Margin::symmetric(22, 10)),
            )
            .show(ctx, |ui| {
                let all = ui.available_rect_before_wrap();
                let current = self.current();
                let snapshot = self
                    .player
                    .as_ref()
                    .map(|p| p.snapshot.clone())
                    .unwrap_or_default();
                let info = egui::Rect::from_min_size(
                    all.min,
                    Vec2::new((all.width() * 0.27).min(310.), 52.),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(info), |ui| {
                    ui.horizontal(|ui| {
                        if let Some(m) = &current {
                            if self
                                .cover(ui, m, 48.)
                                .interact(egui::Sense::click())
                                .clicked()
                            {
                                self.page = Page::Lyrics;
                            }
                            ui.vertical(|ui| {
                                ui.set_max_width((info.width() - 65.).max(90.));
                                ui.add(
                                    egui::Label::new(RichText::new(&m.title).strong()).truncate(),
                                );
                                if ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(if m.artist.is_empty() {
                                                self.t("Unknown artist")
                                            } else {
                                                &m.artist
                                            })
                                            .small()
                                            .weak(),
                                        )
                                        .sense(egui::Sense::click())
                                        .truncate(),
                                    )
                                    .clicked()
                                {
                                    self.open_artist(&m.artist);
                                }
                            });
                        } else {
                            ui.label(RichText::new(self.t("Choose a song to begin")).weak());
                        }
                    });
                });
                let controls = egui::Rect::from_center_size(
                    egui::pos2(all.center().x, all.top() + if narrow { 80. } else { 24. }),
                    Vec2::new(202., 46.),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(controls)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 7.;
                        if icons::button(
                            ui,
                            Icon::Shuffle,
                            self.t("Shuffle"),
                            self.state.preferences.shuffle,
                            30.,
                        )
                        .clicked()
                        {
                            self.state.preferences.shuffle = !self.state.preferences.shuffle;
                            self.changed();
                        }
                        if icons::button(
                            ui,
                            Icon::Previous,
                            self.t("Previous / restart"),
                            false,
                            30.,
                        )
                        .clicked()
                        {
                            self.step(-1);
                        }
                        if icons::button(
                            ui,
                            if snapshot.paused || snapshot.path.is_empty() {
                                Icon::Play
                            } else {
                                Icon::Pause
                            },
                            self.t(if snapshot.paused { "Play" } else { "Pause" }),
                            true,
                            42.,
                        )
                        .clicked()
                        {
                            if snapshot.path.is_empty() {
                                self.resume_queue();
                            } else if let Some(p) = self.player.as_mut() {
                                p.toggle();
                            }
                        }
                        if icons::button(ui, Icon::Next, self.t("Next"), false, 30.).clicked() {
                            self.step(1);
                        }
                        if icons::button(
                            ui,
                            if self.repeat == 2 {
                                Icon::RepeatOne
                            } else {
                                Icon::Repeat
                            },
                            self.t(match self.repeat {
                                0 => "Repeat off",
                                1 => "Repeat all",
                                _ => "Repeat one",
                            }),
                            self.repeat > 0,
                            30.,
                        )
                        .clicked()
                        {
                            self.repeat = (self.repeat + 1) % 3;
                        }
                    },
                );
                let right = egui::Rect::from_min_max(
                    egui::pos2(all.right() - 290., all.top() + 5.),
                    egui::pos2(all.right(), all.top() + 45.),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(right)
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().slider_width = 120.;
                        let old = self.state.preferences.volume;
                        let r = ui
                            .add(
                                egui::Slider::new(&mut self.state.preferences.volume, 0.0..=100.0)
                                    .show_value(false),
                            )
                            .on_hover_text(format!("{}%", self.state.preferences.volume.round()));
                        if r.changed() {
                            if let Some(p) = self.player.as_mut() {
                                p.set("volume", &self.state.preferences.volume.to_string());
                            }
                            self.changed();
                        }
                        let volume = icons::button(
                            ui,
                            Icon::Volume,
                            self.t("Output device"),
                            old == 0.,
                            27.,
                        );
                        egui::Popup::menu(&volume).show(|ui| {
                            ui.label(RichText::new(self.t("Output device")).strong());
                            if ui.button(self.t("Mute / unmute")).clicked() {
                                self.state.preferences.volume = if old == 0. { 75. } else { 0. };
                                self.apply_audio_preferences();
                                self.changed();
                            }
                            ui.separator();
                            if ui
                                .selectable_label(
                                    self.state.preferences.audio_device == "auto",
                                    self.t("System default"),
                                )
                                .clicked()
                            {
                                self.state.preferences.audio_device = "auto".into();
                                self.apply_audio_preferences();
                                self.changed();
                                ui.close();
                            }
                            for (id, name) in self.devices.clone() {
                                if ui
                                    .selectable_label(
                                        self.state.preferences.audio_device == id,
                                        name,
                                    )
                                    .clicked()
                                {
                                    self.state.preferences.audio_device = id;
                                    self.apply_audio_preferences();
                                    self.changed();
                                    ui.close();
                                }
                            }
                            if ui.button(self.t("Refresh devices")).clicked() {
                                self.refresh_devices();
                            }
                        });
                        if icons::button(
                            ui,
                            Icon::Queue,
                            self.t("Queue"),
                            self.state.preferences.show_queue,
                            28.,
                        )
                        .clicked()
                        {
                            self.state.preferences.show_queue = !self.state.preferences.show_queue;
                            self.changed();
                        }
                        if icons::button(
                            ui,
                            Icon::Lyrics,
                            self.t("Lyrics"),
                            self.page == Page::Lyrics,
                            28.,
                        )
                        .clicked()
                        {
                            self.page = if self.page == Page::Lyrics {
                                Page::Music
                            } else {
                                Page::Lyrics
                            };
                        }
                        if current
                            .as_ref()
                            .is_some_and(|m| m.kind == "video" || !m.music_video.is_empty())
                            && icons::button(
                                ui,
                                Icon::Video,
                                self.t("Show / hide music video"),
                                self.state.preferences.show_music_video,
                                28.,
                            )
                            .clicked()
                        {
                            self.toggle_video();
                        }
                    },
                );
                let timeline = egui::Rect::from_min_size(
                    egui::pos2(all.left(), all.top() + if narrow { 112. } else { 62. }),
                    Vec2::new(all.width(), 20.),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(timeline)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        let mut position = self.seek_preview.unwrap_or(snapshot.position);
                        ui.add_sized(
                            [44., 20.],
                            egui::Label::new(
                                RichText::new(duration(position)).small().monospace().weak(),
                            ),
                        );
                        ui.spacing_mut().slider_width = (timeline.width() - 108.).max(50.);
                        let r = ui.add_enabled(
                            snapshot.duration > 0.,
                            egui::Slider::new(&mut position, 0.0..=snapshot.duration.max(0.01))
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        if r.changed() {
                            self.seek_preview = Some(position);
                        }
                        if self.seek_preview.is_some()
                            && (r.drag_stopped()
                                || (r.changed() && !r.dragged())
                                || (!ctx.input(|i| i.pointer.primary_down()) && !r.dragged()))
                        {
                            if let Some(p) = self.player.as_mut() {
                                p.seek(self.seek_preview.take().unwrap());
                            }
                        }
                        ui.add_sized(
                            [44., 20.],
                            egui::Label::new(
                                RichText::new(duration(snapshot.duration))
                                    .small()
                                    .monospace()
                                    .weak(),
                            ),
                        );
                    },
                );
            });
    }
    fn toggle_video(&mut self) {
        self.state.preferences.show_music_video = !self.state.preferences.show_music_video;
        if let Some(p) = self.player.as_mut() {
            p.set(
                "vid",
                if self.state.preferences.show_music_video {
                    "auto"
                } else {
                    "no"
                },
            );
        }
        self.page = if self.state.preferences.show_music_video {
            Page::NowPlaying
        } else {
            Page::Lyrics
        };
        self.changed();
    }
    pub(super) fn queue_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("queue")
            .exact_width(245.)
            .show(ctx, |ui| {
                ui.label(RichText::new(self.t("Queue")).size(18.).strong());
                ui.add_space(12.);
                for (i, id) in self.state.queue.clone().iter().enumerate() {
                    if let Some(m) = self.library().iter().find(|m| &m.id == id).cloned() {
                        if ui
                            .selectable_label(
                                i == self.state.queue_index,
                                format!("{}  {}", i + 1, m.title),
                            )
                            .clicked()
                        {
                            self.state.queue_index = i;
                            self.load_queue(0.);
                        }
                    }
                }
            });
    }
    fn search_controls(&mut self, ui: &mut egui::Ui) {
        let narrow = ui.available_width() < 1000.;
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 38.),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().interact_size.y = 34.;
                ui.spacing_mut().button_padding.y = 7.;
                let (r, _) = ui.allocate_exact_size(Vec2::splat(24.), egui::Sense::hover());
                icons::draw(
                    ui.painter(),
                    r,
                    Icon::Search,
                    ui.visuals().weak_text_color(),
                );
                let hint = self.t("Search your music");
                ui.add_sized(
                    [(ui.available_width() - 120.).clamp(80., 500.), 34.],
                    egui::TextEdit::singleline(&mut self.search).hint_text(hint),
                );
                egui::ComboBox::from_id_salt("search-field")
                    .selected_text(self.t_key(&self.search_field))
                    .width(100.)
                    .show_ui(ui, |ui| {
                        for key in ["All fields", "Title", "Artist", "Album", "Genre"] {
                            if ui
                                .selectable_label(self.search_field == key, self.t(key))
                                .clicked()
                            {
                                self.search_field = key.into();
                            }
                        }
                    });
                if !narrow {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(self.t("Add files")).clicked() {
                            self.add_files();
                        }
                        if ui.button(self.t("Add folder")).clicked() {
                            self.add_folder();
                        }
                    });
                }
            },
        );
        ui.horizontal_wrapped(|ui| {
            let genres = {
                let mut cache = self.genre_cache.borrow_mut();
                cache
                    .get_or_insert_with(|| {
                        std::sync::Arc::new(
                            self.library()
                                .iter()
                                .flat_map(|m| m.genre.split(';'))
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(String::from)
                                .collect(),
                        )
                    })
                    .clone()
            };
            if !genres.is_empty() {
                let all = self.t("All genres");
                egui::ComboBox::from_id_salt("genre-filter")
                    .selected_text(if self.genre_filter.is_empty() {
                        all
                    } else {
                        &self.genre_filter
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.genre_filter, String::new(), all);
                        for g in genres.iter() {
                            ui.selectable_value(&mut self.genre_filter, g.clone(), g);
                        }
                    });
            }
            ui.menu_button(self.t_key(&self.sort_key), |ui| {
                ui.label(RichText::new(self.t("Sort by")).strong());
                for key in [
                    "Custom order",
                    "Title",
                    "Artist",
                    "Album",
                    "Recently added",
                    "Release date",
                    "Duration",
                    "Genre",
                ] {
                    if ui
                        .selectable_label(self.sort_key == key, self.t(key))
                        .clicked()
                    {
                        self.sort_key = key.into();
                        ui.close();
                    }
                }
                ui.separator();
                let text = self.t("Descending");
                ui.checkbox(&mut self.sort_desc, text);
                let text = self.t("Compact view");
                if ui
                    .checkbox(&mut self.state.preferences.compact, text)
                    .changed()
                {
                    self.changed();
                }
            });
            if narrow {
                if ui.button(self.t("Add folder")).clicked() {
                    self.add_folder();
                }
                if ui.button(self.t("Add files")).clicked() {
                    self.add_files();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.metadata_running,
                    egui::Button::new(self.t("Find information for all songs")),
                )
                .clicked()
            {
                self.start_metadata(
                    self.library()
                        .iter()
                        .filter(|m| m.series.is_empty())
                        .map(|m| m.id.clone())
                        .collect(),
                    true,
                );
            }
            if self.metadata_running {
                self.lookup_status(ui);
            }
        });
        ui.add_space(12.);
    }
    fn t_key(&self, key: &str) -> String {
        match key {
            "All fields" => self.t("All fields"),
            "Title" => self.t("Title"),
            "Artist" => self.t("Artist"),
            "Album" => self.t("Album"),
            "Genre" => self.t("Genre"),
            "Custom order" => self.t("Custom order"),
            "Recently added" => self.t("Recently added"),
            "Release date" => self.t("Release date"),
            "Duration" => self.t("Duration"),
            _ => key,
        }
        .to_string()
    }
    pub(super) fn central(&mut self, ctx: &egui::Context, frame: Option<&mut eframe::Frame>) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(themes::palette(&self.state.preferences.theme).bg)
                    .inner_margin(26),
            )
            .show(ctx, |ui| {
                if self.page == Page::Lyrics {
                    self.lyrics_page(ui, ctx);
                    return;
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.t(self.page.title())).size(29.).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.busy > 0 {
                            ui.spinner();
                        }
                        ui.label(
                            RichText::new(if self.remote.is_some() {
                                self.t("Personal server")
                            } else {
                                self.t("Local library")
                            })
                            .small()
                            .weak(),
                        );
                    });
                });
                ui.add_space(15.);
                if !matches!(
                    self.page,
                    Page::Settings | Page::Server | Page::Stats | Page::NowPlaying
                ) {
                    self.search_controls(ui);
                }
                if !self.engine_error.is_empty() {
                    ui.colored_label(Color32::LIGHT_RED, &self.engine_error);
                }
                if self.error {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(Color32::LIGHT_RED, self.status.clone());
                        if icons::button(ui, Icon::Close, self.t("Dismiss"), false, 26.).clicked() {
                            self.error = false;
                            if let Some(p) = self.player.as_mut() {
                                p.snapshot.error.clear();
                            }
                        }
                    });
                }
                egui::ScrollArea::vertical()
                    .id_salt(("music-main", self.page.title()))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.page.clone() {
                            Page::Home => self.home_page(ui),
                            Page::Artists => self.artists_page(ui),
                            Page::Artist => self.artist_page(ui),
                            Page::Albums => self.albums_page(ui),
                            Page::Album => self.album_page(ui),
                            Page::Playlists => self.playlists_page(ui),
                            Page::Stats => self.stats_page(ui),
                            Page::Settings => self.settings_page(ui),
                            Page::Server => self.sources_page(ui),
                            Page::NowPlaying => self.music_video_page(ui, frame),
                            _ => {
                                let rows = self.filtered();
                                self.track_list(ui, &rows, None);
                            }
                        }
                        if !self.status.is_empty() && !self.error {
                            ui.add_space(22.);
                            ui.label(RichText::new(&self.status).small().weak());
                        }
                    });
            });
    }
    pub(super) fn track_list(&mut self, ui: &mut egui::Ui, rows: &[Media], limit: Option<usize>) {
        if rows.is_empty() {
            ui.add_space(24.);
            ui.heading(self.t("Your music belongs here"));
            ui.label(self.t("Add a folder, change your search, or connect to your server."));
            return;
        }
        self.selection_toolbar(ui, &rows[..rows.len().min(limit.unwrap_or(rows.len()))]);
        let width = ui.available_width();
        let album = width > 540.;
        let date = width > 780.;
        let album_w = if album { (width * 0.19).min(220.) } else { 0. };
        let date_w = if date { 110. } else { 0. };
        let title_w = (width - 32. - 48. - album_w - date_w - 150.).max(100.);
        let header = ui
            .allocate_exact_size(Vec2::new(width, 26.), egui::Sense::hover())
            .0;
        for (x, text) in [
            (10., "#"),
            (85., self.t("Title")),
            (85. + title_w, self.t("Album")),
            (85. + title_w + album_w, self.t("Date added")),
        ] {
            if text == self.t("Album") && !album || text == self.t("Date added") && !date {
                continue;
            }
            ui.painter().text(
                egui::pos2(header.left() + x, header.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(11.),
                ui.visuals().weak_text_color(),
            );
        }
        icons::draw(
            ui.painter(),
            egui::Rect::from_center_size(
                egui::pos2(header.right() - 100., header.center().y),
                Vec2::splat(19.),
            ),
            Icon::Clock,
            ui.visuals().weak_text_color(),
        );
        ui.separator();
        let current = self.current().map(|m| m.id).unwrap_or_default();
        let playing = self
            .player
            .as_ref()
            .is_some_and(|p| !p.snapshot.paused && !p.snapshot.eof && !p.snapshot.path.is_empty());
        let height = if self.state.preferences.compact {
            49.
        } else {
            64.
        };
        let count = rows.len().min(limit.unwrap_or(rows.len()));
        let stride = height + 8.;
        let list = ui
            .allocate_exact_size(
                Vec2::new(width, stride * count as f32),
                egui::Sense::hover(),
            )
            .0;
        let clip = ui.clip_rect();
        let first = (((clip.top() - list.top()).max(0.) / stride).floor() as usize).min(count);
        let end = (((clip.bottom() - list.top()).max(0.) / stride).ceil() as usize).min(count);
        for index in first..end {
            let m = &rows[index];
            let row = egui::Rect::from_min_size(
                list.min + egui::vec2(0., index as f32 * stride),
                egui::vec2(width, height),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
                ui.push_id((&m.id, index), |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
                    // Offscreen rows keep their scroll space without decoding art,
                    // building widgets or generating text/animation meshes.
                    if !ui.is_rect_visible(rect) {
                        return;
                    }
                    let active = current == m.id;
                    let hover = ui
                        .ctx()
                        .animate_bool(response.id.with("row"), response.hovered());
                    if active || self.selected_songs.contains(&m.id) || hover > 0.001 {
                        ui.painter().rect_filled(
                            rect,
                            5.,
                            if active || self.selected_songs.contains(&m.id) {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().selection.bg_fill.gamma_multiply(hover * 0.5)
                            },
                        );
                    }
                    let left = rect.left();
                    let center = rect.center().y;
                    if self.select_mode {
                        let rect = egui::Rect::from_center_size(
                            egui::pos2(left + 17., center),
                            Vec2::splat(26.),
                        );
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            let mut checked = self.selected_songs.contains(&m.id);
                            if ui.checkbox(&mut checked, "").clicked() {
                                self.select_song(&m.id, rows, ui.input(|i| i.modifiers.shift));
                            }
                        });
                    } else if active && playing {
                        self.playing_indicator(
                            ui,
                            egui::Rect::from_center_size(
                                egui::pos2(left + 17., center),
                                Vec2::splat(20.),
                            ),
                        );
                    } else {
                        ui.painter().text(
                            egui::pos2(left + 17., center),
                            egui::Align2::CENTER_CENTER,
                            (index + 1).to_string(),
                            egui::FontId::proportional(13.),
                            ui.visuals().weak_text_color(),
                        );
                    }
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(left + 35., center - 20.),
                            Vec2::splat(40.),
                        )),
                        |ui| {
                            if self
                                .cover(ui, m, 40.)
                                .interact(egui::Sense::click())
                                .double_clicked()
                            {
                                self.play(&m.id);
                            }
                        },
                    );
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(left + 85., center - 21.),
                            Vec2::new(title_w - 8., 45.),
                        )),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 3.;
                            let r = ui.add(
                                egui::Label::new(
                                    RichText::new(&m.title)
                                        .color(if active {
                                            self.accent()
                                        } else {
                                            ui.visuals().text_color()
                                        })
                                        .strong(),
                                )
                                .truncate()
                                .sense(egui::Sense::click()),
                            );
                            if r.clicked()
                                && (self.select_mode
                                    || ui.input(|i| {
                                        i.modifiers.ctrl || i.modifiers.command || i.modifiers.shift
                                    }))
                            {
                                self.select_song(&m.id, rows, ui.input(|i| i.modifiers.shift));
                            }
                            if r.double_clicked() && !self.select_mode {
                                self.play(&m.id);
                            }
                            r.context_menu(|ui| self.media_menu(ui, m));
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(if m.artist.is_empty() {
                                            self.t("Unknown artist")
                                        } else {
                                            &m.artist
                                        })
                                        .small()
                                        .weak(),
                                    )
                                    .truncate()
                                    .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                self.open_artist(&m.artist);
                            }
                        },
                    );
                    let mut x = left + 85. + title_w;
                    if album {
                        ui.scope_builder(
                            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                egui::pos2(x, center - 10.),
                                Vec2::new(album_w - 12., 24.),
                            )),
                            |ui| {
                                if ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(if m.album.is_empty() {
                                                "—"
                                            } else {
                                                &m.album
                                            })
                                            .small()
                                            .weak(),
                                        )
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                {
                                    self.open_album(m);
                                }
                            },
                        );
                        x += album_w;
                    }
                    if date {
                        let at = if self.page == Page::Playlists {
                            self.selected_playlist
                                .as_ref()
                                .and_then(|id| self.state.playlists.iter().find(|p| &p.id == id))
                                .and_then(|p| p.added.get(&m.id))
                                .copied()
                                .unwrap_or(0)
                        } else {
                            m.added
                        };
                        ui.painter().text(
                            egui::pos2(x, center),
                            egui::Align2::LEFT_CENTER,
                            music::date(at),
                            egui::FontId::proportional(12.),
                            ui.visuals().weak_text_color(),
                        );
                    }
                    ui.painter().text(
                        egui::pos2(rect.right() - 100., center),
                        egui::Align2::CENTER_CENTER,
                        duration(m.duration),
                        egui::FontId::proportional(12.),
                        ui.visuals().weak_text_color(),
                    );
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(egui::Rect::from_min_size(
                                egui::pos2(rect.right() - 74., center - 16.),
                                Vec2::new(74., 32.),
                            ))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 0.;
                            if icons::button(
                                ui,
                                Icon::Heart,
                                self.t("Favorite"),
                                self.state.favorites.contains(&m.id),
                                25.,
                            )
                            .clicked()
                            {
                                self.state.favorite(&m.id);
                                self.changed();
                            }
                            let more =
                                icons::button(ui, Icon::More, self.t("More options"), false, 30.);
                            egui::Popup::menu(&more).show(|ui| self.media_menu(ui, m));
                        },
                    );
                    if response.clicked()
                        && (self.select_mode
                            || ui.input(|i| {
                                i.modifiers.ctrl || i.modifiers.command || i.modifiers.shift
                            }))
                    {
                        self.select_song(&m.id, rows, ui.input(|i| i.modifiers.shift));
                    }
                    if response.double_clicked() && !self.select_mode {
                        self.play(&m.id);
                    }
                    response.context_menu(|ui| self.media_menu(ui, m));
                });
            });
        }
    }
    fn playing_indicator(&self, ui: &egui::Ui, rect: egui::Rect) {
        let time = if self.state.preferences.animations {
            ui.input(|i| i.time) as f32
        } else {
            0.4
        };
        for n in 0..3 {
            let h = if self.state.preferences.animations {
                0.25 + ((time * 5. + n as f32 * 1.7).sin() + 1.) * 0.32
            } else {
                [0.55, 0.95, 0.7][n]
            };
            let x = rect.left() + n as f32 * 6. + 2.;
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - h * rect.height()),
                    Vec2::new(3., h * rect.height()),
                ),
                1.,
                self.accent(),
            );
        }
    }
    pub(super) fn media_menu(&mut self, ui: &mut egui::Ui, m: &Media) {
        if ui.button(self.t("Select song")).clicked() {
            let rows = self.filtered();
            self.select_song(&m.id, &rows, false);
            ui.close();
        }
        if ui
            .add_enabled(
                !self.metadata_running,
                egui::Button::new(self.t("Find information online")),
            )
            .clicked()
        {
            self.start_metadata_values(vec![m.clone()], true);
            ui.close();
        }

        for (label, action) in [
            ("Play now", 0),
            ("Play next", 1),
            ("Add to queue", 2),
            ("Go to artist", 3),
            ("Go to album", 4),
            ("Edit information", 5),
        ] {
            if ui.button(self.t(label)).clicked() {
                match action {
                    0 => self.play(&m.id),
                    1 => self.enqueue(&m.id, true),
                    2 => self.enqueue(&m.id, false),
                    3 => self.open_artist(&m.artist),
                    4 => self.open_album(m),
                    _ => self.edit = Some(m.clone()),
                }
                ui.close();
            }
        }
        ui.separator();
        ui.menu_button(self.t("Add to playlist"), |ui| {
            if ui.button(self.t("Create playlist")).clicked() {
                self.playlist_seed = vec![m.id.clone()];
                self.playlist_create = true;
                ui.close();
            }
            if !self.state.playlists.is_empty() {
                ui.separator();
            }
            for p in self.state.playlists.clone() {
                if ui.button(&p.name).clicked() {
                    if let Some(p) = self.state.playlists.iter_mut().find(|v| v.id == p.id) {
                        p.add(&m.id, ssmp::now());
                    }
                    self.changed();
                    ui.close();
                }
            }
        });
        if self.page == Page::Playlists {
            if self.sort_key == "Custom order" {
                ui.horizontal(|ui| {
                    for (label, delta) in [("Move up", -1), ("Move down", 1)] {
                        if ui.button(self.t(label)).clicked() {
                            if let Some(p) = self.selected_playlist.as_ref().and_then(|id| {
                                self.state.playlists.iter_mut().find(|p| &p.id == id)
                            }) {
                                if let Some(i) = p.items.iter().position(|id| id == &m.id) {
                                    let j = (i as i32 + delta).clamp(0, p.items.len() as i32 - 1)
                                        as usize;
                                    p.items.swap(i, j);
                                    self.changed();
                                }
                            }
                            ui.close();
                        }
                    }
                });
            }
            if ui.button(self.t("Remove from playlist")).clicked() {
                if let Some(p) = self
                    .selected_playlist
                    .as_ref()
                    .and_then(|id| self.state.playlists.iter_mut().find(|p| &p.id == id))
                {
                    p.items.retain(|id| id != &m.id);
                    p.added.remove(&m.id);
                }
                self.changed();
                ui.close();
            }
        }
        if self.remote.is_some() && ui.button(self.t("Keep offline")).clicked() {
            self.download(&m.id);
            ui.close();
        }
    }
    fn album_cards(&mut self, ui: &mut egui::Ui, rows: &[Media]) {
        let width = ui.available_width();
        let columns = ((width + 18.) / 178.).floor().max(1.) as usize;
        let card_w = ((width - 18. * (columns - 1) as f32) / columns as f32).min(210.);
        for chunk in rows.chunks(columns) {
            ui.horizontal_top(|ui| {
                for m in chunk {
                    ui.allocate_ui_with_layout(
                        Vec2::new(card_w, card_w + 58.),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let response = self.cover(ui, m, card_w).interact(egui::Sense::click());
                            if response.clicked() {
                                self.open_album(m);
                            }
                            response.context_menu(|ui| self.media_menu(ui, m));
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(if m.album.is_empty() {
                                            self.t("Singles")
                                        } else {
                                            &m.album
                                        })
                                        .strong(),
                                    )
                                    .truncate()
                                    .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                self.open_album(m);
                            }
                            if ui
                                .add(
                                    egui::Label::new(RichText::new(&m.artist).small().weak())
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                self.open_artist(&m.artist);
                            }
                        },
                    );
                    ui.add_space(10.);
                }
            });
            ui.add_space(10.);
        }
    }
    fn unique_albums(&self) -> Vec<Media> {
        let mut seen = BTreeSet::new();
        self.filtered()
            .iter()
            .cloned()
            .filter(|m| m.kind == "audio" && seen.insert(music::album_key(m)))
            .collect()
    }
    fn home_page(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(self.t("Your collection. On your terms."))
                .size(20.)
                .strong(),
        );
        ui.label(RichText::new(self.t("No ads. No account. Just your music.")).weak());
        ui.add_space(18.);
        if self.library().is_empty() {
            if ui.button(self.t("Add your first folder")).clicked() {
                self.add_folder();
            }
            return;
        }
        if !self.state.playlists.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for p in self
                    .state
                    .playlists
                    .iter()
                    .take(6)
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    if ui
                        .add_sized([210., 46.], egui::Button::new(&p.name))
                        .clicked()
                    {
                        self.selected_playlist = Some(p.id);
                        self.page = Page::Playlists;
                    }
                }
            });
            ui.add_space(24.);
        }
        ui.heading(self.t("Recently added"));
        ui.add_space(12.);
        let mut albums = self.unique_albums();
        albums.sort_by_key(|m| std::cmp::Reverse(m.added));
        self.album_cards(ui, &albums.into_iter().take(8).collect::<Vec<_>>());
        if self.state.preferences.recommendations && !self.state.listening_seconds.is_empty() {
            ui.add_space(20.);
            ui.heading(self.t("On repeat"));
            ui.label(
                RichText::new(self.t("Ranked by your actual listening time"))
                    .small()
                    .weak(),
            );
            let mut rows = self.filtered().as_ref().clone();
            rows.retain(|m| {
                self.state
                    .listening_seconds
                    .get(&m.id)
                    .copied()
                    .unwrap_or(0.)
                    > 0.
            });
            rows.sort_by(|a, b| {
                self.state.listening_seconds[&b.id].total_cmp(&self.state.listening_seconds[&a.id])
            });
            self.track_list(ui, &rows, Some(6));
        }
    }
    fn albums_page(&mut self, ui: &mut egui::Ui) {
        let rows = self.unique_albums();
        self.album_cards(ui, &rows);
    }
    fn artists_page(&mut self, ui: &mut egui::Ui) {
        let mut artists: BTreeMap<String, Vec<Media>> = BTreeMap::new();
        for m in self.filtered().iter().cloned() {
            artists.entry(m.artist.clone()).or_default().push(m);
        }
        for (artist, rows) in artists {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(14)
                .corner_radius(8.)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        self.cover(ui, &rows[0], 64.);
                        ui.vertical(|ui| {
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(if artist.is_empty() {
                                            self.t("Unknown artist")
                                        } else {
                                            &artist
                                        })
                                        .size(20.)
                                        .strong(),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                self.open_artist(&artist);
                            }
                            ui.label(format!(
                                "{} {} · {} {}",
                                rows.len(),
                                self.t("songs"),
                                rows.iter().map(|m| &m.album).collect::<BTreeSet<_>>().len(),
                                self.t("albums")
                            ));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::button(
                                ui,
                                Icon::Shuffle,
                                self.t("Shuffle artist"),
                                false,
                                36.,
                            )
                            .clicked()
                            {
                                self.play_rows(&rows, true);
                            }
                            if icons::button(
                                ui,
                                Icon::Heart,
                                self.t("Favorite artist"),
                                self.state.favorite_artists.contains(&artist.to_lowercase()),
                                36.,
                            )
                            .clicked()
                            {
                                self.state.favorite_artist(&artist);
                                self.changed();
                            }
                        });
                    });
                });
            ui.add_space(8.);
        }
    }
    fn artist_page(&mut self, ui: &mut egui::Ui) {
        let artist = self.selected_artist.clone().unwrap_or_default();
        let rows = self.filtered();
        ui.heading(if artist.is_empty() {
            self.t("Unknown artist")
        } else {
            &artist
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{} {} · {}",
                rows.len(),
                self.t("songs"),
                duration(rows.iter().map(|m| m.duration).sum())
            ));
            if ui.button(self.t("Shuffle artist")).clicked() {
                self.play_rows(&rows, true);
            }
            if ui
                .selectable_label(
                    self.state.favorite_artists.contains(&artist.to_lowercase()),
                    self.t("Favorite artist"),
                )
                .clicked()
            {
                self.state.favorite_artist(&artist);
                self.changed();
            }
        });
        if let Some(m) = rows
            .iter()
            .find(|m| !m.genre.is_empty() || !m.artist_country.is_empty())
        {
            ui.label(format!("{}  {}", m.genre, m.artist_country));
        }
        ui.add_space(20.);
        let mut seen = BTreeSet::new();
        let albums: Vec<_> = rows
            .iter()
            .filter(|m| seen.insert(m.album.clone()))
            .cloned()
            .collect();
        self.album_cards(ui, &albums);
        ui.add_space(16.);
        self.track_list(ui, &rows, None);
    }
    fn album_page(&mut self, ui: &mut egui::Ui) {
        let rows = self.filtered();
        if let Some(m) = rows.first().cloned() {
            ui.horizontal(|ui| {
                self.cover(ui, &m, 140.);
                ui.vertical(|ui| {
                    ui.heading(if m.album.is_empty() {
                        self.t("Singles")
                    } else {
                        &m.album
                    });
                    if ui.link(&m.artist).clicked() {
                        self.open_artist(&m.artist);
                    }
                    ui.label(format!(
                        "{} {} · {} · {}",
                        rows.len(),
                        self.t("songs"),
                        duration(rows.iter().map(|m| m.duration).sum()),
                        m.release_date
                    ));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(self.t("Play album")).clicked() {
                            self.play_rows(&rows, false);
                        }
                        if ui.button(self.t("Shuffle")).clicked() {
                            self.play_rows(&rows, true);
                        }
                        if ui.button(self.t("Make a playlist")).clicked() {
                            self.playlist_seed = rows.iter().map(|m| m.id.clone()).collect();
                            self.new_playlist = m.album.clone();
                            self.playlist_create = true;
                        }
                    });
                });
            });
            ui.add_space(24.);
        }
        self.track_list(ui, &rows, None);
    }
    fn playlists_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.t("Create playlist")).clicked() {
                self.playlist_create = true;
                self.playlist_seed.clear();
            }
            if ui.button(self.t("Import playlist")).clicked() {
                self.playlist_import();
            }
            for p in self.state.playlists.clone() {
                if ui
                    .selectable_label(self.selected_playlist.as_ref() == Some(&p.id), &p.name)
                    .clicked()
                {
                    self.selected_playlist = Some(p.id);
                    self.search.clear();
                }
            }
        });
        ui.add_space(18.);
        let Some(p) = self
            .selected_playlist
            .as_ref()
            .and_then(|id| self.state.playlists.iter().find(|p| &p.id == id))
            .cloned()
        else {
            ui.label(self.t("Create a playlist and make it yours."));
            return;
        };
        let all: Vec<_> = p
            .items
            .iter()
            .filter_map(|id| self.library().iter().find(|m| &m.id == id).cloned())
            .collect();
        let cover = Media {
            id: p.id.clone(),
            title: p.name.clone(),
            album: p.name.clone(),
            artwork: p.artwork.clone(),
            ..Default::default()
        };
        ui.horizontal_top(|ui| {
            if self
                .cover(ui, &cover, 148.)
                .interact(egui::Sense::click())
                .on_hover_text(self.t("Choose photo"))
                .clicked()
            {
                self.choose_playlist_art(&p.id);
            }
            ui.vertical(|ui| {
                ui.label(RichText::new(self.t("PLAYLIST")).small().weak());
                ui.label(RichText::new(&p.name).size(31.).strong());
                if !p.description.is_empty() {
                    ui.add(egui::Label::new(&p.description).wrap());
                }
                let heard = p.listening_seconds;
                ui.label(format!(
                    "{} {} · {} {} · {} {}",
                    p.items.len(),
                    self.t("songs"),
                    duration(all.iter().map(|m| m.duration).sum()),
                    self.t("total"),
                    duration(heard),
                    self.t("listened")
                ));
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.t("Play")).clicked() {
                        self.play_rows(&all, false);
                    }
                    if ui.button(self.t("Shuffle")).clicked() {
                        self.play_rows(&all, true);
                    }
                    if ui.button(self.t("Add songs")).clicked() {
                        self.playlist_add = true;
                        self.playlist_query.clear();
                    }
                    if ui.button(self.t("Edit playlist")).clicked() {
                        self.playlist_edit = true;
                    }
                    ui.menu_button(self.t("More options"), |ui| {
                        if self.remote.is_none() && ui.button(self.t("Export playlist")).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(format!("{}.m3u8", p.name))
                                .save_file()
                            {
                                match library::export_playlist(&self.store, &p, &path) {
                                    Ok(_) => self.notify(self.t("Playlist exported"), false),
                                    Err(e) => self.notify(e.to_string(), true),
                                }
                            }
                            ui.close();
                        }
                        if ui.button(self.t("Delete playlist")).clicked() {
                            self.playlist_delete = true;
                            ui.close();
                        }
                    });
                    if self.remote.is_some() && ui.button(self.t("Keep offline")).clicked() {
                        self.download_playlist(&p.items);
                    }
                });
            });
        });
        ui.add_space(22.);
        if !p.unresolved.is_empty() {
            ui.label(format!(
                "{}: {}",
                self.t("Unmatched playlist entries"),
                p.unresolved.len()
            ));
        }
        let rows = self.filtered();
        self.track_list(ui, &rows, None);
    }
    fn choose_playlist_art(&mut self, id: &str) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(self.t("Images"), &["png", "jpg", "jpeg"])
            .pick_file()
        {
            if let Ok(image) = image::open(&path) {
                let destination = self
                    .store
                    .root
                    .join("artwork")
                    .join(format!("playlist-{id}-{}.png", ssmp::now()));
                if image.thumbnail(800, 800).save(&destination).is_ok() {
                    if let Some(p) = self.state.playlists.iter_mut().find(|p| p.id == id) {
                        p.artwork = destination.to_string_lossy().into_owned();
                    }
                    self.changed();
                }
            }
        }
    }
    fn stats_page(&mut self, ui: &mut egui::Ui) {
        self.wrap_stats_page(ui);
    }
    fn lyrics_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(m) = self.current() else {
            ui.heading(self.t("Choose a song to begin"));
            return;
        };
        let area = ui.available_rect_before_wrap();
        if !m.artwork.is_empty() {
            if self
                .backdrop
                .as_ref()
                .is_none_or(|(key, _)| key != &m.artwork)
            {
                if let Ok(image) = image::open(&m.artwork) {
                    let image = image
                        .resize_exact(160, 160, image::imageops::FilterType::Triangle)
                        .blur(18.)
                        .to_rgba8();
                    let texture = ctx.load_texture(
                        "lyrics-backdrop",
                        egui::ColorImage::from_rgba_unmultiplied([160, 160], image.as_raw()),
                        egui::TextureOptions::LINEAR,
                    );
                    self.backdrop = Some((m.artwork.clone(), texture));
                }
            }
            if let Some((_, texture)) = &self.backdrop {
                ui.painter().image(
                    texture.id(),
                    area,
                    egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
                    Color32::from_white_alpha(
                        if themes::palette(&self.state.preferences.theme).light {
                            35
                        } else {
                            75
                        },
                    ),
                );
            }
        }
        ui.horizontal(|ui| {
            if icons::button(ui, Icon::Back, self.t("Back to music"), false, 32.).clicked() {
                self.page = Page::Music;
                if self.lyrics_fullscreen {
                    self.lyrics_fullscreen = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                }
            }
            ui.label(RichText::new(self.t("Lyrics")).size(21.).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::button(
                    ui,
                    Icon::Fullscreen,
                    self.t("Full screen"),
                    self.lyrics_fullscreen,
                    32.,
                )
                .clicked()
                {
                    self.lyrics_fullscreen = !self.lyrics_fullscreen;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                        self.lyrics_fullscreen,
                    ));
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.t("Add lyrics file")).clicked() {
                if let Some(text) = self.choose_lyrics_file() {
                    self.apply_lyrics(&m.id, text);
                }
            }
            if ui
                .add_enabled(
                    self.lyrics_pending.is_none(),
                    egui::Button::new(self.t("Find lyrics online")),
                )
                .clicked()
            {
                self.lookup_lyrics(&m);
            }
        });
        self.lookup_status(ui);
        ui.add_space(18.);
        let available_height = ui.available_height();
        centered_column(ui, 1320., |ui| {
            ui.add_space(((available_height - 630.) / 2.).clamp(0., 240.));
            let width = ui.available_width();
            let font_size = (width / 34.).clamp(20., 34.);
            let stacked = width < 580.;
            let left = (width * 0.36).clamp(170., 420.);
            if stacked {
                ui.horizontal(|ui| {
                    self.cover(ui, &m, 58.);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&m.title).strong());
                        if ui.link(&m.artist).clicked() {
                            self.open_artist(&m.artist);
                        }
                    });
                });
                ui.add_space(10.);
            }
            ui.horizontal_top(|ui| {
                if !stacked {
                    ui.allocate_ui_with_layout(
                        Vec2::new(left, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.add_space(20.);
                            self.cover(ui, &m, (left - 20.).min(360.));
                            ui.add_space(17.);
                            ui.add(
                                egui::Label::new(RichText::new(&m.title).size(21.).strong()).wrap(),
                            );
                            if ui.link(&m.artist).clicked() {
                                self.open_artist(&m.artist);
                            }
                            ui.label(RichText::new(&m.album).weak());
                            ui.add_space(15.);
                            if m.kind == "video" || !m.music_video.is_empty() {
                                if ui.button(self.t("Show music video")).clicked() {
                                    self.state.preferences.show_music_video = true;
                                    if let Some(p) = self.player.as_mut() {
                                        p.set("vid", "auto");
                                    }
                                    self.page = Page::NowPlaying;
                                    self.changed();
                                }
                            }
                            ui.add_space(18.);
                            ui.label(RichText::new(self.t("Timing offset")).small().weak());
                            ui.add(
                                egui::DragValue::new(&mut self.lyrics_offset)
                                    .speed(0.1)
                                    .suffix(" s"),
                            );
                        },
                    );
                    ui.add_space(20.);
                }
                ui.allocate_ui_with_layout(
                    Vec2::new(
                        if stacked {
                            width
                        } else {
                            (width - left - 35.).max(180.)
                        },
                        ui.available_height(),
                    ),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let lines = library::lyrics(&m.lyrics);
                        let position = self
                            .player
                            .as_ref()
                            .map(|p| p.snapshot.position)
                            .unwrap_or(0.)
                            + self.lyrics_offset;
                        let current = lines
                            .iter()
                            .rposition(|(at, _)| *at >= 0. && *at <= position);
                        if lines.is_empty() {
                            ui.add_space(50.);
                            ui.heading(self.t("No lyrics yet"));
                            ui.label(
                                self.t("Add a local LRC file, edit the lyrics, or search online."),
                            );
                            if ui.button(self.t("Edit information")).clicked() {
                                self.edit = Some(m.clone());
                            }
                        }
                        let synced = lines.iter().any(|(at, _)| *at >= 0.);
                        if !synced && !lines.is_empty() {
                            ui.label(
                                RichText::new(self.t("These lyrics have no timestamps"))
                                    .small()
                                    .weak(),
                            );
                        }
                        if ui.input(|i| i.raw_scroll_delta.y.abs() > 0.1) {
                            self.lyric_manual_scroll = Instant::now();
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("lyrics-focus")
                            .max_height(ui.available_height() - 20.)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add_space(35.);
                                for (index, (at, text)) in lines.iter().enumerate() {
                                    let active = current == Some(index);
                                    let amount = if self.state.preferences.animations {
                                        ctx.animate_bool(
                                            egui::Id::new(("lyric", &m.id, index)),
                                            active,
                                        )
                                    } else {
                                        if active { 1. } else { 0. }
                                    };
                                    let color = themes::mix(
                                        ui.visuals().weak_text_color(),
                                        ui.visuals().text_color(),
                                        amount,
                                    );
                                    let words = music::lyric_words(text);
                                    let mut job = egui::text::LayoutJob::default();
                                    let timed =
                                        active && words.iter().any(|(time, _)| time.is_some());
                                    for (time, word) in words {
                                        job.append(
                                            &word,
                                            0.,
                                            egui::TextFormat {
                                                font_id: egui::FontId::proportional(
                                                    font_size + amount * 6.,
                                                ),
                                                color: if timed
                                                    && time.is_some_and(|t| t <= position)
                                                {
                                                    self.accent()
                                                } else {
                                                    color
                                                },
                                                ..Default::default()
                                            },
                                        );
                                    }
                                    let response = ui.add(
                                        egui::Label::new(job).wrap().sense(egui::Sense::click()),
                                    );
                                    if response.clicked() && *at >= 0. {
                                        if let Some(p) = self.player.as_mut() {
                                            p.seek((*at - self.lyrics_offset).max(0.));
                                        }
                                    }
                                    if active
                                        && self.last_lyric != current
                                        && self.lyric_manual_scroll.elapsed()
                                            > Duration::from_secs(3)
                                    {
                                        response.scroll_to_me(Some(egui::Align::Center));
                                    }
                                    ui.add_space(20.);
                                }
                                ui.add_space(100.);
                            });
                        self.last_lyric = current;
                    },
                );
            });
        });
    }
    fn music_video_page(&mut self, ui: &mut egui::Ui, frame: Option<&mut eframe::Frame>) {
        let Some(m) = self.current() else {
            ui.label(self.t("Choose a song to begin"));
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.heading(&m.title);
            if ui.button(self.t("Audio only")).clicked() {
                self.state.preferences.show_music_video = false;
                if let Some(p) = self.player.as_mut() {
                    p.set("vid", "no");
                }
                self.page = Page::Lyrics;
                self.changed();
            }
            if ui.button(self.t("Lyrics")).clicked() {
                self.page = Page::Lyrics;
            }
        });
        if m.kind != "video" && m.music_video.is_empty() {
            ui.label(self.t("No music video linked to this song"));
            return;
        }
        if let Some(p) = self.player.as_mut() {
            let cpu_window = frame.is_none();
            let use_gl = frame.as_ref().is_some_and(|frame| frame.gl().is_some());
            if let Some(frame) = frame.filter(|frame| frame.gl().is_some()) {
                if p.texture_id.is_none() {
                    if let Some(texture) = p.texture {
                        p.texture_id = Some(frame.register_native_glow_texture(texture));
                    }
                }
                if let Some(gl) = frame.gl() {
                    p.render_gl(gl);
                }
            } else {
                match p.video_frame_software() {
                    Ok(Some(mut pixels)) => {
                        for rgba in pixels.chunks_exact_mut(4) {
                            rgba[3] = 255;
                        }
                        let image = egui::ColorImage::from_rgba_unmultiplied([1280, 720], &pixels);
                        if !cpu_window && let Some(texture) = &mut self.video_frame {
                            texture.set(image, egui::TextureOptions::LINEAR);
                        } else {
                            self.video_frame = Some(ui.ctx().load_texture(
                                "music-video-frame",
                                image,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => p.snapshot.error = error.to_string(),
                }
            }
            let texture = if use_gl {
                p.texture_id
            } else {
                self.video_frame.as_ref().map(|t| t.id())
            };
            if let Some(id) = texture {
                let width = ui
                    .available_width()
                    .min((ui.available_height() - 75.).max(150.) * 16. / 9.);
                let size = Vec2::new(width, width * 9. / 16.);
                centered_row(ui, width, |ui| {
                    ui.add(egui::Image::new((id, size)).fit_to_exact_size(size));
                });
            }
        }
        ui.label(RichText::new(&m.artist).weak());
    }
    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let mut appearance = false;
        let mut audio = false;
        let mut changed = false;
        ui.heading(self.t("Appearance"));
        ui.label(RichText::new(self.t("Choose a theme. Make it feel like yours.")).weak());
        ui.add_space(12.);
        let available = ui.available_width();
        let columns = ((available + 8.) / 142.).floor().max(1.) as usize;
        let card_width = ((available - 8. * (columns - 1) as f32) / columns as f32).min(180.);
        for chunk in themes::NAMES.chunks(columns) {
            ui.horizontal_top(|ui| {
                for name in chunk {
                    ui.allocate_ui_with_layout(
                        Vec2::new(card_width, 68.),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let palette = themes::palette(name);
                            let active = self.state.preferences.theme == *name;
                            let response = egui::Frame::new()
                                .fill(palette.panel)
                                .stroke(Stroke::new(
                                    if active { 2_f32 } else { 1_f32 },
                                    if active {
                                        self.accent()
                                    } else {
                                        palette.border
                                    },
                                ))
                                .inner_margin(12)
                                .corner_radius(8.)
                                .show(ui, |ui| {
                                    ui.set_width(card_width - 24.);
                                    ui.set_height(40.);
                                    ui.vertical_centered(|ui| {
                                        let (r, _) = ui.allocate_exact_size(
                                            Vec2::new(card_width - 24., 18.),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(r.center(), 7., palette.accent);
                                        ui.label(RichText::new(*name).color(palette.text));
                                    });
                                })
                                .response
                                .interact(egui::Sense::click());
                            if response.clicked() {
                                self.set_theme(name, ui.ctx());
                            }
                        },
                    );
                }
            });
            ui.add_space(3.);
        }
        ui.add_space(15.);
        ui.horizontal_wrapped(|ui| {
            ui.label(self.t("Accent color"));
            let mut accent = self.state.preferences.accent;
            if ui.color_edit_button_srgb(&mut accent).changed() {
                self.state.preferences.accent = accent;
                appearance = true;
            }
            ui.label(self.t("Interface size"));
            let mut scale = self.scale_preview.unwrap_or(self.state.preferences.scale);
            let response = ui.add(
                egui::Slider::new(&mut scale, 0.8..=1.5)
                    .step_by(0.05)
                    .show_value(false),
            );
            ui.label(format!("{:.0}%", scale * 100.));
            if response.changed() {
                self.scale_preview = Some(scale);
            }
            if self.scale_preview.is_some() && !ui.ctx().input(|i| i.pointer.primary_down()) {
                self.state.preferences.scale = self.scale_preview.take().unwrap();
                appearance = true;
            }
            if ui.small_button("100%").clicked() {
                self.scale_preview = None;
                self.state.preferences.scale = 1.;
                appearance = true;
            }
            ui.label(
                RichText::new(self.t("Applied after release"))
                    .small()
                    .weak(),
            );
        });
        let text = self.t("Animations and smooth transitions");
        appearance |= ui
            .checkbox(&mut self.state.preferences.animations, text)
            .changed();
        let text = self.t("Compact view");
        changed |= ui
            .checkbox(&mut self.state.preferences.compact, text)
            .changed();
        ui.add_space(24.);
        ui.heading(self.t("Language"));
        egui::ComboBox::from_id_salt("language")
            .selected_text(match self.state.preferences.language.as_str() {
                "de" => "Deutsch",
                "es" => "Español",
                "fr" => "Français",
                _ => "English",
            })
            .show_ui(ui, |ui| {
                for (key, label) in [
                    ("en", "English"),
                    ("de", "Deutsch"),
                    ("es", "Español"),
                    ("fr", "Français"),
                ] {
                    if ui
                        .selectable_label(self.state.preferences.language == key, label)
                        .clicked()
                    {
                        self.state.preferences.language = key.into();
                        changed = true;
                    }
                }
            });
        ui.add_space(24.);
        ui.heading(self.t("Playback"));
        ui.label(self.t("Output device"));
        let selected = self
            .devices
            .iter()
            .find(|(id, _)| id == &self.state.preferences.audio_device)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| self.t("System default").into());
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("output-device")
                .width(320.)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            self.state.preferences.audio_device == "auto",
                            self.t("System default"),
                        )
                        .clicked()
                    {
                        self.state.preferences.audio_device = "auto".into();
                        audio = true;
                    }
                    for (id, label) in self.devices.clone() {
                        if ui
                            .selectable_label(self.state.preferences.audio_device == id, label)
                            .clicked()
                        {
                            self.state.preferences.audio_device = id;
                            audio = true;
                        }
                    }
                });
            if ui.button(self.t("Refresh devices")).clicked() {
                self.refresh_devices();
            }
        });
        let text = self.t("Normalize loudness");
        audio |= ui
            .checkbox(&mut self.state.preferences.normalize, text)
            .changed();
        ui.horizontal(|ui| {
            ui.label(self.t("Playback speed"));
            audio |= ui
                .add(
                    egui::Slider::new(&mut self.state.preferences.speed, 0.5..=2.)
                        .step_by(0.05)
                        .suffix("×"),
                )
                .changed();
        });
        let text = self.t("Remember song position");
        changed |= ui
            .checkbox(&mut self.state.preferences.resume_music, text)
            .changed();
        let text = self.t("Prefer favorite artists in shuffle");
        changed |= ui
            .checkbox(&mut self.state.preferences.weight_favorite_artists, text)
            .changed();
        ui.label(RichText::new(self.t("Favorite artists receive three times the shuffle weight. Other artists remain eligible.")).small().weak());
        ui.add_space(24.);
        ui.heading(self.t("Library & metadata"));
        let text = self.t("Watch my music folders for changes");
        changed |= ui
            .checkbox(&mut self.state.preferences.watch_folders, text)
            .changed();
        ui.label(
            RichText::new(
                self.t("Checks configured folders every five seconds. Playback keeps running."),
            )
            .small()
            .weak(),
        );
        let text = self.t("Automatically find missing metadata online");
        if ui
            .checkbox(&mut self.state.preferences.online_metadata, text)
            .changed()
        {
            changed = true;
            self.metadata_cancel.store(
                !self.metadata_explicit && !self.state.preferences.online_metadata,
                std::sync::atomic::Ordering::Relaxed,
            );
            self.last_metadata = Instant::now() - Duration::from_secs(300);
        }
        ui.label(RichText::new(self.t("MusicBrainz, Cover Art Archive and LRCLIB. Sends song, artist and album names only. Fills missing fields; keeps your edits. Everything else works offline.")).small().weak());
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.metadata_running,
                    egui::Button::new(self.t("Check metadata now")),
                )
                .clicked()
            {
                self.start_metadata(self.library().iter().map(|m| m.id.clone()).collect(), true);
            }
            if ui.button(self.t("Manage music sources")).clicked() {
                self.page = Page::Server;
            }
        });
        ui.add_space(24.);
        ui.heading(self.t("Privacy & statistics"));
        let text = self.t("Remember listening history and measured statistics");
        changed |= ui
            .checkbox(&mut self.state.preferences.history, text)
            .changed();
        let text = self.t("Personal top-song recommendations");
        changed |= ui
            .checkbox(&mut self.state.preferences.recommendations, text)
            .changed();
        let text = self.t("Sync preferences with my server");
        changed |= ui
            .checkbox(&mut self.state.preferences.sync_settings, text)
            .changed();
        if ui
            .button(self.t("Clear listening history and statistics"))
            .clicked()
        {
            self.state.clear_history();
            self.listen_seconds = 0.;
            self.listen_recorded = false;
            changed = true;
        }
        ui.label(
            RichText::new(
                self.t(
                    "Listening data stays in your local library or your personal server profile.",
                ),
            )
            .small()
            .weak(),
        );
        self.update_settings(ui);
        if appearance {
            self.apply_preferences(ui.ctx());
        }
        if audio {
            self.apply_audio_preferences();
        }
        if appearance || audio || changed {
            self.changed();
        }
    }
    fn refresh_devices(&mut self) {
        self.last_devices = Instant::now();
        if let Some(p) = self.player.as_ref() {
            self.devices = p
                .property_json("audio-device-list")
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            Some((
                                v["name"].as_str()?.to_string(),
                                v["description"]
                                    .as_str()
                                    .unwrap_or("Audio output")
                                    .to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
        }
    }
    fn sources_page(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t("Music sources"));
        ui.label(self.t("Your folders are the library. Files stay where you put them."));
        ui.add_space(12.);
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.t("Add folder")).clicked() {
                self.add_folder();
            }
            if ui.button(self.t("Add files")).clicked() {
                self.add_files();
            }
            if ui
                .add_enabled(
                    self.busy == 0,
                    egui::Button::new(self.t("Scan folders now")),
                )
                .clicked()
            {
                self.import_paths(self.state.roots.iter().map(PathBuf::from).collect());
            }
        });
        for path in self.state.roots.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.label(&path);
                if ui.small_button(self.t("Remove source")).clicked() {
                    self.state.roots.retain(|p| p != &path);
                    self.changed();
                }
            });
        }
        let text = self.t("Watch my music folders for changes");
        if ui
            .checkbox(&mut self.state.preferences.watch_folders, text)
            .changed()
        {
            self.changed();
        }
        ui.add_space(26.);
        ui.heading(self.t("Connect to a personal server"));
        ui.label(
            self.t("Use a LAN address or your Tailscale address. Local playback needs no server."),
        );
        if let Some(remote) = self.remote.clone() {
            ui.label(format!(
                "{} · {}",
                remote.connection.url, remote.connection.username
            ));
            if ui.button(self.t("Switch to local library")).clicked() {
                let _ = self.store.save_state(&self.profile_key, &self.state);
                if let Some(p) = self.player.as_mut() {
                    p.command(&["stop"]);
                }
                self.remote = None;
                self.remote_rows.clear();
                self.profile_key = "local".into();
                self.state = self.store.state("local").unwrap_or_default();
                self.selected_playlist = self.state.playlists.first().map(|p| p.id.clone());
                self.apply_preferences(ui.ctx());
                self.apply_audio_preferences();
                self.changed();
            }
        } else {
            ui.label(self.t("Server address"));
            ui.add(egui::TextEdit::singleline(&mut self.host_url).desired_width(440.));
            ui.label(self.t("Username"));
            ui.add(egui::TextEdit::singleline(&mut self.username).desired_width(300.));
            ui.label(self.t("Password"));
            ui.add(
                egui::TextEdit::singleline(&mut self.password)
                    .password(true)
                    .desired_width(300.),
            );
            if ui
                .add_enabled(self.busy == 0, egui::Button::new(self.t("Connect")))
                .clicked()
            {
                let url = self.host_url.clone();
                let username = self.username.clone();
                let password = std::mem::take(&mut self.password);
                let tx = self.tx.clone();
                self.busy += 1;
                std::thread::spawn(move || {
                    let r = (|| -> Result<_> {
                        let remote = Remote::login(&url, &username, &password)?;
                        let rows = remote.library()?;
                        let state = remote.state()?;
                        Ok((remote, rows, state))
                    })();
                    let _ = tx.send(Event::Connected(r.map_err(|e| e.to_string())));
                });
            }
        }
        ui.add_space(26.);
        ui.heading(self.t("Host this library"));
        ui.label(self.t("Create an owner, then start your server. Other devices use its address and their own profile."));
        ui.label(self.t("Bind address"));
        ui.add(egui::TextEdit::singleline(&mut self.host_bind).desired_width(300.));
        ui.label(
            RichText::new(
                self.t("127.0.0.1 is this computer. 0.0.0.0 listens on all network interfaces."),
            )
            .small()
            .weak(),
        );
        ui.label(self.t("Owner username"));
        ui.add(egui::TextEdit::singleline(&mut self.host_user).desired_width(300.));
        ui.label(self.t("Owner password"));
        ui.add(
            egui::TextEdit::singleline(&mut self.host_password)
                .password(true)
                .desired_width(300.),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.t("Create owner")).clicked() {
                match self
                    .store
                    .add_user(&self.host_user, &self.host_password, true)
                {
                    Ok(_) => {
                        self.host_password.clear();
                        self.notify(self.t("Owner created"), false)
                    }
                    Err(e) => self.notify(e.to_string(), true),
                }
            }
            if self.host_child.is_none() {
                if ui.button(self.t("Start server")).clicked() {
                    match std::env::current_exe()
                        .map(|exe| ssmp::quiet_command(exe))
                        .and_then(|mut cmd| {
                            let log = std::fs::File::create(self.store.root.join("host.log"))?;
                            cmd.stdout(std::process::Stdio::from(log.try_clone()?));
                            cmd.stderr(std::process::Stdio::from(log));
                            cmd.args([
                                "--data",
                                &self.store.root.to_string_lossy(),
                                "--serve",
                                "--bind",
                                &self.host_bind,
                            ])
                            .spawn()
                        }) {
                        Ok(child) => {
                            self.host_child = Some(child);
                            self.notify(self.t("Server started"), false)
                        }
                        Err(e) => self.notify(e.to_string(), true),
                    }
                }
            } else if ui.button(self.t("Stop server")).clicked() {
                if let Some(mut child) = self.host_child.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        });
    }
    pub(super) fn music_dialogs(&mut self, ctx: &egui::Context) {
        if self.playlist_delete {
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new(RichText::new(self.t("Delete playlist")).size(16.).strong())
                .default_height(120.)
                .vscroll(true)
                .max_height((ctx.content_rect().height() - 40.).max(160.))
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(self.t("Remove this playlist? Your music files are kept."));
                    ui.horizontal(|ui| {
                        if ui.button(self.t("Delete playlist")).clicked() {
                            confirm = true;
                        }
                        if ui.button(self.t("Cancel")).clicked() {
                            cancel = true;
                        }
                    });
                });
            if confirm {
                if let Some(id) = self.selected_playlist.take() {
                    self.state.playlists.retain(|p| p.id != id);
                }
                self.selected_playlist = self.state.playlists.first().map(|p| p.id.clone());
                self.changed();
                self.playlist_delete = false;
            } else if cancel {
                self.playlist_delete = false;
            }
        }
        if self.show_welcome {
            egui::Window::new(RichText::new(self.t("Welcome to SSMP")).size(16.).strong())
                .default_height(420.)
                .vscroll(true)
                .max_height((ctx.content_rect().height() - 40.).max(160.))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .default_width(470.)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(self.t("Your music. Your way."))
                            .size(28.)
                            .strong(),
                    );
                    ui.add_space(12.);
                    ui.label(
                        self.t("Start with a folder. No account or internet connection needed."),
                    );
                    ui.add_space(18.);
                    egui::ComboBox::from_id_salt("welcome-language")
                        .selected_text(self.state.preferences.language.clone())
                        .show_ui(ui, |ui| {
                            for (id, label) in [
                                ("en", "English"),
                                ("de", "Deutsch"),
                                ("es", "Español"),
                                ("fr", "Français"),
                            ] {
                                ui.selectable_value(
                                    &mut self.state.preferences.language,
                                    id.into(),
                                    label,
                                );
                            }
                        });
                    let text = self.t("Remember listening history and measured statistics");
                    ui.checkbox(&mut self.state.preferences.history, text);
                    ui.add_space(12.);
                    ui.horizontal(|ui| {
                        if ui.button(self.t("Add your first folder")).clicked() {
                            self.show_welcome = false;
                            self.state.onboarding = true;
                            self.changed();
                            self.add_folder();
                        }
                        if ui.button(self.t("Explore first")).clicked() {
                            self.show_welcome = false;
                            self.state.onboarding = true;
                            self.changed();
                        }
                    });
                });
        }
        if self.playlist_create {
            let mut open = true;
            let mut create = false;
            egui::Window::new(RichText::new(self.t("Create playlist")).size(16.).strong())
                .default_height(180.)
                .vscroll(true)
                .max_height((ctx.content_rect().height() - 40.).max(160.))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .default_width(390.)
                .show(ctx, |ui| {
                    ui.label(self.t("Playlist name"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_playlist)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(12.);
                    if ui
                        .add_enabled(
                            !self.new_playlist.trim().is_empty(),
                            egui::Button::new(self.t("Create playlist")),
                        )
                        .clicked()
                    {
                        create = true;
                    }
                });
            if create {
                let id = ssmp::token();
                let mut p = Playlist {
                    id: id.clone(),
                    name: self.new_playlist.trim().into(),
                    ..Default::default()
                };
                for track in self.playlist_seed.drain(..) {
                    p.add(&track, ssmp::now());
                }
                self.state.playlists.push(p);
                self.selected_playlist = Some(id);
                self.new_playlist.clear();
                self.playlist_create = false;
                self.page = Page::Playlists;
                self.changed();
            } else if !open {
                self.playlist_create = false;
            }
        }
        if self.playlist_edit {
            if let Some(mut p) = self
                .selected_playlist
                .as_ref()
                .and_then(|id| self.state.playlists.iter().find(|p| &p.id == id))
                .cloned()
            {
                let mut open = true;
                let mut save = false;
                egui::Window::new(RichText::new(self.t("Edit playlist")).size(16.).strong())
                    .default_height(350.)
                    .vscroll(true)
                    .max_height((ctx.content_rect().height() - 40.).max(160.))
                    .open(&mut open)
                    .collapsible(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                    .default_width(480.)
                    .show(ctx, |ui| {
                        ui.label(self.t("Playlist name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut p.name).desired_width(f32::INFINITY),
                        );
                        ui.label(self.t("Description"));
                        ui.add(
                            egui::TextEdit::multiline(&mut p.description)
                                .desired_rows(4)
                                .desired_width(f32::INFINITY),
                        );
                        if ui.button(self.t("Choose photo")).clicked() {
                            self.choose_playlist_art(&p.id);
                            p.artwork = self
                                .state
                                .playlists
                                .iter()
                                .find(|v| v.id == p.id)
                                .map(|v| v.artwork.clone())
                                .unwrap_or_default();
                        }
                        ui.add_space(12.);
                        if ui
                            .add_enabled(
                                !p.name.trim().is_empty(),
                                egui::Button::new(self.t("Save changes")),
                            )
                            .clicked()
                        {
                            save = true;
                        }
                    });
                if save {
                    if let Some(old) = self.state.playlists.iter_mut().find(|v| v.id == p.id) {
                        *old = p;
                    }
                    self.playlist_edit = false;
                    self.changed();
                } else if !open {
                    self.playlist_edit = false;
                }
            }
        }
        if self.playlist_add {
            let mut open = true;
            egui::Window::new(RichText::new(self.t("Add songs")).size(16.).strong())
                .default_height(480.)
                .vscroll(true)
                .max_height((ctx.content_rect().height() - 40.).max(160.))
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .open(&mut open)
                .default_width(570.)
                .default_height(440.)
                .collapsible(false)
                .show(ctx, |ui| {
                    let hint = self.t("Search installed songs");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.playlist_query)
                            .hint_text(hint)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(12.);
                    let rows: Vec<_> = self
                        .library()
                        .iter()
                        .filter(|m| {
                            m.kind == "audio"
                                && format!("{} {} {}", m.title, m.artist, m.album)
                                    .to_lowercase()
                                    .contains(&self.playlist_query.to_lowercase())
                        })
                        .cloned()
                        .collect();
                    egui::ScrollArea::vertical()
                        .max_height(350.)
                        .show(ui, |ui| {
                            for m in rows {
                                let added = self
                                    .selected_playlist
                                    .as_ref()
                                    .and_then(|id| {
                                        self.state.playlists.iter().find(|p| &p.id == id)
                                    })
                                    .is_some_and(|p| p.items.contains(&m.id));
                                ui.horizontal(|ui| {
                                    ui.label(format!("{} · {}", m.title, m.artist));
                                    if ui
                                        .add_enabled(
                                            !added,
                                            egui::Button::new(self.t(if added {
                                                "Added"
                                            } else {
                                                "Add"
                                            })),
                                        )
                                        .clicked()
                                    {
                                        if let Some(p) =
                                            self.selected_playlist.as_ref().and_then(|id| {
                                                self.state
                                                    .playlists
                                                    .iter_mut()
                                                    .find(|p| &p.id == id)
                                            })
                                        {
                                            p.add(&m.id, ssmp::now());
                                        }
                                        self.changed();
                                    }
                                });
                            }
                        });
                });
            if !open {
                self.playlist_add = false;
            }
        }
        if let Some(mut m) = self.edit.take() {
            let mut open = true;
            let mut save = false;
            let mut cancel = false;
            egui::Window::new(RichText::new(self.t("Edit information")).size(16.).strong())
                .default_height(570.)
                .vscroll(true)
                .max_height((ctx.content_rect().height() - 40.).max(160.))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
                .default_width(630.)
                .show(ctx, |ui| {
                    ui.horizontal_top(|ui| {
                        self.cover(ui, &m, 56.);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&m.title).size(19.).strong());
                            ui.label(RichText::new(&m.artist).weak());
                            ui.label(
                                RichText::new(self.t(
                                    "Your edits stay in the library. Original files are untouched.",
                                ))
                                .small()
                                .weak(),
                            );
                        });
                    });
                    ui.add_space(18.);
                    egui::Grid::new("music-metadata")
                        .num_columns(2)
                        .spacing([16., 8.])
                        .show(ui, |ui| {
                            for (label, value) in [
                                ("Title", &mut m.title),
                                ("Artist", &mut m.artist),
                                ("Album", &mut m.album),
                                ("Album artist", &mut m.album_artist),
                                ("Genre", &mut m.genre),
                                ("Release date", &mut m.release_date),
                            ] {
                                ui.label(self.t(label));
                                ui.add(egui::TextEdit::singleline(value).desired_width(390.));
                                ui.end_row();
                            }
                            ui.label(self.t("Track number"));
                            ui.add(egui::DragValue::new(&mut m.track));
                            ui.end_row();
                        });
                    ui.add_space(12.);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(self.t("Choose cover")).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(self.t("Images"), &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                m.artwork = path.to_string_lossy().into_owned();
                            }
                        }
                        if ui.button(self.t("Link music video")).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(self.t("Music videos"), &["mp4", "mkv", "webm", "mov"])
                                .pick_file()
                            {
                                m.music_video = path.to_string_lossy().into_owned();
                            }
                        }
                    });
                    ui.add_space(12.);
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                !self.metadata_running,
                                egui::Button::new(self.t("Find information online")),
                            )
                            .clicked()
                        {
                            self.start_metadata_values(vec![m.clone()], true);
                        }
                        if ui.button(self.t("Add lyrics file")).clicked() {
                            if let Some(text) = self.choose_lyrics_file() {
                                m.lyrics = text;
                            }
                        }
                        if ui
                            .add_enabled(
                                self.lyrics_pending.is_none(),
                                egui::Button::new(self.t("Find lyrics online")),
                            )
                            .clicked()
                        {
                            self.lookup_lyrics(&m);
                        }
                    });
                    self.lookup_status(ui);
                    ui.collapsing(self.t("Lyrics"), |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut m.lyrics)
                                .desired_width(f32::INFINITY)
                                .desired_rows(8),
                        );
                        ui.label(
                            RichText::new(
                                self.t(
                                    "Plain text or LRC timestamps. Synced lyrics follow playback.",
                                ),
                            )
                            .small()
                            .weak(),
                        );
                    });
                    if self.remote.is_none() {
                        ui.collapsing(self.t("File details"), |ui| {
                            ui.label(RichText::new(&m.path).small().weak());
                            if ui.button(self.t("Relink file")).clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    match library::probe(&path) {
                                        Ok(fresh) => {
                                            m.path = fresh.path;
                                            m.kind = fresh.kind;
                                            m.duration = fresh.duration;
                                            m.size = fresh.size;
                                            m.modified = fresh.modified;
                                            m.fingerprint = fresh.fingerprint;
                                            m.missing = false;
                                        }
                                        Err(e) => self.notify(e.to_string(), true),
                                    }
                                }
                            }
                        });
                    }
                    if !m.metadata_source.is_empty() {
                        ui.label(
                            RichText::new(format!("{} · {}", m.metadata_source, m.musicbrainz_id))
                                .small()
                                .weak(),
                        );
                    }
                    ui.add_space(15.);
                    ui.horizontal(|ui| {
                        if ui.button(self.t("Save changes")).clicked() {
                            save = true;
                        }
                        if ui.button(self.t("Cancel")).clicked() {
                            cancel = true;
                        }
                    });
                });
            if save {
                self.save_metadata(m);
            } else if open && !cancel {
                self.edit = Some(m);
            }
        }
    }
    pub(super) fn tick(&mut self) {
        self.update_tick();
        if self
            .host_child
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some())
        {
            self.host_child = None;
            self.notify(self.t("Server stopped. See host.log for details."), true);
        }
        let dt = self.last_tick.elapsed().as_secs_f64();
        self.last_tick = Instant::now();
        let mut finished = false;
        let linked = self
            .current()
            .filter(|m| m.kind == "audio" && !m.music_video.is_empty())
            .map(|m| m.music_video);
        if let Some(p) = self.player.as_mut() {
            p.tick();
            let snapshot = p.snapshot.clone();
            if !self.linked_video_requested
                && self.state.preferences.show_music_video
                && snapshot.duration > 0.
            {
                if let Some(path) = linked {
                    self.linked_video_requested = true;
                    p.command(&["video-add", &path, "select"]);
                }
            }
            if !snapshot.error.is_empty() {
                self.status = snapshot.error;
                self.error = true;
            }
            if !snapshot.path.is_empty() && self.loaded_at.elapsed() > Duration::from_millis(350) {
                if let Some(id) = self.state.queue.get(self.state.queue_index).cloned() {
                    if self.last_track != id {
                        self.last_track = id.clone();
                        self.listen_seconds = 0.;
                        self.listen_clock
                            .reset(snapshot.position, snapshot.discontinuity);
                        self.listen_recorded = false;
                        self.last_lyric = None;
                    }
                    let seconds = self.listen_clock.sample(
                        dt,
                        snapshot.position,
                        self.state.preferences.speed,
                        snapshot.paused || snapshot.eof,
                        snapshot.discontinuity,
                    );
                    self.listen_seconds += seconds;
                    if seconds > 0. && self.state.preferences.history {
                        self.state.record_seconds(&id, seconds, ssmp::now());
                        self.dirty = true;
                    }
                    if !self.listen_recorded
                        && self.listen_seconds >= (snapshot.duration * 0.5).clamp(1., 30.)
                    {
                        self.filtered_cache.get_mut().take();
                        self.state.listen(&id);
                        self.listen_recorded = true;
                        self.dirty = true;
                    }
                    if self.last_position.elapsed() > Duration::from_secs(3) {
                        if self.state.preferences.resume_music {
                            self.state.positions.insert(id, snapshot.position);
                            self.dirty = true;
                        }
                        self.last_position = Instant::now();
                    }
                }
                if snapshot.eof && !self.eof_handled {
                    self.eof_handled = true;
                    finished = true;
                }
            }
        }
        if finished {
            if self.repeat == 2 {
                self.load_queue(0.);
            } else if self.state.preferences.shuffle
                || self.state.queue_index + 1 < self.state.queue.len()
                || self.repeat == 1
            {
                self.step(1);
            }
        }
        if self.last_save.elapsed() > Duration::from_secs(2) {
            self.save();
        }
        if self.last_devices.elapsed() > Duration::from_secs(30) {
            self.refresh_devices();
        }
        if self.remote.is_none()
            && self.last_inbox.elapsed() > Duration::from_secs(3)
            && self.busy < 2
        {
            self.last_inbox = Instant::now();
            let has = std::fs::read_dir(self.store.root.join("imports")).is_ok_and(|e| {
                e.flatten()
                    .any(|e| e.path().extension().is_some_and(|s| s == "json"))
            });
            if has {
                let root = self.store.root.clone();
                let tx = self.tx.clone();
                self.busy += 1;
                std::thread::spawn(move || {
                    let r = (|| -> Result<_> { library::consume_inbox(&Store::open(&root)?) })();
                    let _ = tx.send(Event::Imported(r.map_err(|e| e.to_string())));
                });
            }
        }
        if self.remote.is_none()
            && self.state.preferences.watch_folders
            && !self.watch_running
            && self.last_watch.elapsed() > Duration::from_secs(5)
            && !self.state.roots.is_empty()
        {
            self.last_watch = Instant::now();
            self.watch_running = true;
            self.busy += 1;
            let roots = self
                .state
                .roots
                .iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            let previous = self.watch_signature;
            let root = self.store.root.clone();
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let r = (|| -> Result<_> {
                    let signature = ssmp::library::folder_signature(&roots);
                    let report = if signature != previous {
                        library::scan(&Store::open(&root)?, &roots)?
                    } else {
                        ImportReport::default()
                    };
                    Ok((report, signature))
                })();
                let _ = tx.send(Event::Watched(r.map_err(|e| e.to_string())));
            });
        }
        if self.remote.is_none()
            && self.state.preferences.online_metadata
            && !self.metadata_running
            && self.last_metadata.elapsed() > Duration::from_secs(300)
        {
            self.last_metadata = Instant::now();
            let rows: Vec<_> = self
                .rows
                .iter()
                .filter(|m| {
                    !m.locked
                        && !m.artist.is_empty()
                        && m.metadata_checked == 0
                        && m.series.is_empty()
                })
                .take(12)
                .cloned()
                .collect();
            if !rows.is_empty() {
                self.start_metadata(rows.iter().map(|m| m.id.clone()).collect(), false);
            }
        }
    }
}
