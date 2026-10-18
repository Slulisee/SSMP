use super::*;
use ssmp::stats::{Period, Rank};

impl App {
    pub(super) fn wrap_stats_page(&mut self, ui: &mut egui::Ui) {
        let (year, month, _) = ssmp::stats::calendar(ssmp::now());
        ui.horizontal_wrapped(|ui| {
            for (kind, label) in [
                (0, "All time"),
                (1, "Monthly wrap-up"),
                (2, "Yearly wrap-up"),
            ] {
                if ui
                    .selectable_label(self.report_kind == kind, self.t(label))
                    .clicked()
                {
                    self.report_kind = kind;
                    self.wrap_slide = 0;
                }
            }
            if self.report_kind > 0 {
                egui::ComboBox::from_id_salt("report-year")
                    .selected_text(self.report_year.to_string())
                    .show_ui(ui, |ui| {
                        let first = if self.state.stats_since > 0 {
                            ssmp::stats::calendar(self.state.stats_since).0
                        } else {
                            year
                        };
                        for y in (first.min(year)..=year).rev() {
                            ui.selectable_value(&mut self.report_year, y, y.to_string());
                        }
                    });
                if self.report_kind == 1 {
                    egui::ComboBox::from_id_salt("report-month")
                        .selected_text(format!("{:02}", self.report_month))
                        .show_ui(ui, |ui| {
                            for m in 1..=12 {
                                ui.selectable_value(&mut self.report_month, m, format!("{m:02}"));
                            }
                        });
                }
            }
        });
        let period = match self.report_kind {
            1 => Period::Month(self.report_year, self.report_month),
            2 => Period::Year(self.report_year),
            _ => Period::AllTime,
        };
        let now = ssmp::now();
        if self
            .report_cache
            .as_ref()
            .is_none_or(|(p, t, _)| *p != period || *t != now)
        {
            self.report_cache = Some((
                period,
                now,
                std::sync::Arc::new(ssmp::stats::report(&self.state, self.library(), period)),
            ));
        }
        let r = self.report_cache.as_ref().unwrap().2.clone();
        if !self.state.preferences.history {
            ui.label(self.t("Statistics are paused. Enable listening history to start measuring."));
            if ui.button(self.t("Enable listening history")).clicked() {
                self.state.preferences.history = true;
                self.changed();
            }
        }
        ui.add_space(16.);
        if self.report_kind > 0 {
            egui::Frame::new()
                .fill(themes::mix(ui.visuals().panel_fill, self.accent(), 0.16))
                .inner_margin(22)
                .corner_radius(12.)
                .show(ui, |ui| {
                    ui.set_width((ui.available_width() - 4.).max(180.));
                    ui.label(
                        RichText::new(format!("{} · {}", self.t("Your music story"), r.title))
                            .size(17.)
                            .strong(),
                    );
                    ui.add_space(12.);
                    match self.wrap_slide {
                        0 => {
                            ui.label(
                                RichText::new(if r.seconds >= 3600. {
                                    format!("{:.1} h", r.seconds / 3600.)
                                } else if r.seconds >= 60. {
                                    format!("{:.1} min", r.seconds / 60.)
                                } else {
                                    format!("{:.0} s", r.seconds)
                                })
                                .size(48.)
                                .strong()
                                .color(self.accent()),
                            );
                            ui.label(format!(
                                "{} · {} {}",
                                duration(r.seconds),
                                r.active_days,
                                self.t("active days")
                            ));
                        }
                        1 => {
                            ui.label(self.t("Your top song"));
                            if let Some(t) = r.top_tracks.first() {
                                ui.label(RichText::new(&t.name).size(26.).strong());
                                ui.label(format!(
                                    "{} · {} {}",
                                    duration(t.seconds),
                                    t.plays,
                                    self.t("plays")
                                ));
                                if ui.button(self.t("Play now")).clicked() {
                                    self.play(&t.id);
                                }
                            } else {
                                ui.label(self.t("No measured listening in this period"));
                            }
                        }
                        2 => {
                            ui.label(self.t("Your top artists"));
                            if let Some(t) = r.top_artists.first() {
                                ui.label(RichText::new(&t.name).size(30.).strong());
                                ui.label(format!(
                                    "{} · {} {}",
                                    duration(t.seconds),
                                    r.artists,
                                    self.t("Artists")
                                ));
                                if ui.button(self.t("Go to artist")).clicked() {
                                    self.open_artist(&t.name);
                                }
                            }
                        }
                        3 => {
                            ui.label(self.t("Your top genres"));
                            if let Some(t) = r.top_genres.first() {
                                ui.label(RichText::new(&t.name).size(30.).strong());
                                ui.label(duration(t.seconds));
                            } else {
                                ui.label(self.t("No genre information yet"));
                            }
                            ui.label(format!(
                                "{}: {:.1}%",
                                self.t("Genre coverage of listening time"),
                                r.genre_coverage
                            ));
                        }
                        _ => {
                            ui.label(
                                RichText::new(format!(
                                    "{} {}",
                                    r.new_tracks,
                                    self.t("New discoveries")
                                ))
                                .size(28.)
                                .strong(),
                            );
                            ui.label(format!(
                                "{} {} · {} {}",
                                r.longest_streak,
                                self.t("day streak"),
                                r.tracks,
                                self.t("Songs explored")
                            ));
                            if let Some((day, s)) = r.busiest_day {
                                ui.label(format!(
                                    "{}: {} · {}",
                                    self.t("Your biggest day"),
                                    ssmp::music::date(day * 86400),
                                    duration(s)
                                ));
                            }
                        }
                    }
                    ui.add_space(16.);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(self.wrap_slide > 0, egui::Button::new(self.t("Previous")))
                            .clicked()
                        {
                            self.wrap_slide -= 1;
                        }
                        for n in 0..5 {
                            if ui
                                .selectable_label(self.wrap_slide == n, format!("{}", n + 1))
                                .clicked()
                            {
                                self.wrap_slide = n;
                            }
                        }
                        if ui
                            .add_enabled(self.wrap_slide < 4, egui::Button::new(self.t("Next")))
                            .clicked()
                        {
                            self.wrap_slide += 1;
                        }
                    });
                });
            ui.add_space(18.);
        }
        let cards = [
            (
                "Listening time",
                format!("{:.2} h", r.seconds / 3600.),
                duration(r.seconds),
            ),
            (
                "Songs explored",
                r.tracks.to_string(),
                format!("{} {}", r.plays, self.t("qualified plays")),
            ),
            (
                "Active days",
                r.active_days.to_string(),
                format!("{} {}", r.longest_streak, self.t("day streak")),
            ),
            (
                "Artists",
                r.artists.to_string(),
                format!("{} {}", r.albums, self.t("Albums")),
            ),
            (
                "Daily average",
                duration(r.average_active_day),
                self.t("Per active listening day").into(),
            ),
            (
                "New discoveries",
                r.new_tracks.to_string(),
                self.t("First measured in SSMP").into(),
            ),
        ];
        let columns = if ui.available_width() > 720. { 3. } else { 2. };
        let width = (ui.available_width() - (columns - 1.) * 8.) / columns;
        egui::Grid::new("listening-metric-cards")
            .num_columns(columns as usize)
            .spacing([8., 8.])
            .show(ui, |ui| {
                for (index, (label, value, note)) in cards.into_iter().enumerate() {
                    egui::Frame::new()
                        .fill(ui.visuals().faint_bg_color)
                        .inner_margin(14)
                        .corner_radius(8.)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width((width - 28.).max(100.));
                                ui.label(RichText::new(self.t(label)).small().weak());
                                ui.label(
                                    RichText::new(value).size(26.).strong().color(self.accent()),
                                );
                                ui.label(RichText::new(note).small().weak());
                            });
                        });
                    if (index + 1) % (columns as usize) == 0 {
                        ui.end_row();
                    }
                }
            });
        if let Some(change) = r.change_percent {
            ui.label(format!(
                "{change:+.1}% · {}",
                self.t("Compared with the previous calendar period")
            ));
        }
        ui.add_space(18.);
        ui.heading(self.t("Your listening calendar"));
        ui.label(
            RichText::new(
                self.t("Daily activity and hours use UTC. Hover a square for exact time."),
            )
            .small()
            .weak(),
        );
        let (start, end) = period.bounds();
        let today = ssmp::now() / 86400;
        let first = if period == Period::AllTime {
            today.saturating_sub(89)
        } else {
            start / 86400
        };
        let last = today.min(end / 86400 - 1);
        let maximum = r.days.values().copied().fold(1., f64::max);
        ui.horizontal_wrapped(|ui| {
            for day in first..=last {
                let s = r.days.get(&day).copied().unwrap_or(0.);
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::splat(14.), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    2.,
                    themes::mix(
                        ui.visuals().faint_bg_color,
                        self.accent(),
                        if s > 0. {
                            (0.2 + 0.8 * s / maximum) as f32
                        } else {
                            0.
                        },
                    ),
                );
                response.on_hover_text(format!(
                    "{} · {}",
                    ssmp::music::date(day * 86400),
                    duration(s)
                ));
            }
        });
        if let Some((day, s)) = r.busiest_day {
            ui.label(format!(
                "{}: {} · {}",
                self.t("Your biggest day"),
                ssmp::music::date(day * 86400),
                duration(s)
            ));
        }
        if let Some((hour, s)) = r
            .hours
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .filter(|(_, s)| **s > 0.)
        {
            ui.label(format!(
                "{}: {hour:02}:00 UTC · {}",
                self.t("Your busiest hour"),
                duration(*s)
            ));
        }
        ui.add_space(15.);
        ui.collapsing(self.t("Hours and weekdays"), |ui| {
            let maximum = r.hours.iter().copied().fold(1., f64::max);
            let width = (ui.available_width() - 23. * 4.) / 24.;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.;
                for (h, s) in r.hours.iter().enumerate() {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width.max(3.), 60.), egui::Sense::hover());
                    let bar = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - (s / maximum * 60.) as f32),
                        rect.right_bottom(),
                    );
                    ui.painter().rect_filled(bar, 2., self.accent());
                    response.on_hover_text(format!("{h:02}:00 UTC · {}", duration(*s)));
                }
            });
            for (label, s) in [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ]
            .iter()
            .zip(r.weekdays)
            {
                ui.label(format!("{} · {}", self.t(label), duration(s)));
            }
        });
        self.report_ranking(ui, "Your top songs", &r.top_tracks, r.seconds, 0);
        self.report_ranking(ui, "Your top artists", &r.top_artists, r.seconds, 1);
        self.report_ranking(ui, "Your top albums", &r.top_albums, r.seconds, 2);
        self.report_ranking(ui, "Your top genres", &r.top_genres, r.seconds, 2);
        self.report_ranking(ui, "Your top playlists", &r.top_playlists, r.seconds, 2);
        self.report_ranking(ui, "Your top decades", &r.top_decades, r.seconds, 2);
        ui.label(format!(
            "{}: {:.1}%",
            self.t("Genre coverage of listening time"),
            r.genre_coverage
        ));
        ui.label(format!(
            "{}: {:.1}%",
            self.t("Your top song's share"),
            r.top_track_share
        ));
        if r.detail_since > 0 {
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    self.t("Detailed period rankings measured since"),
                    ssmp::music::date(r.detail_since)
                ))
                .small()
                .weak(),
            );
        }
        if r.detailed_seconds + 1. < r.seconds {
            ui.label(self.t("Older daily totals are included. Missing historical song rankings are not estimated."));
        }
        ui.add_space(16.);
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.t("Export my statistics")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("ssmp-statistics-{}.json", r.title))
                    .save_file()
                {
                    match serde_json::to_vec_pretty(&ssmp::stats::export(&self.state, &r))
                        .map_err(anyhow::Error::from)
                        .and_then(|b| std::fs::write(path, b).map_err(anyhow::Error::from))
                    {
                        Ok(()) => self.notify("Statistics exported", false),
                        Err(e) => self.notify(e.to_string(), true),
                    }
                }
            }
            if ui
                .add_enabled(
                    !r.top_tracks.is_empty(),
                    egui::Button::new(self.t("Make a replay playlist")),
                )
                .clicked()
            {
                self.playlist_seed = r
                    .top_tracks
                    .iter()
                    .take(50)
                    .map(|t| t.id.clone())
                    .filter(|id| self.library().iter().any(|m| &m.id == id))
                    .collect();
                self.new_playlist = format!("SSMP Replay {}", r.title);
                self.playlist_create = true;
            }
        });
        if self.report_year == 0 {
            self.report_year = year;
            self.report_month = month;
        }
    }
    fn report_ranking(
        &mut self,
        ui: &mut egui::Ui,
        heading: &'static str,
        rows: &[Rank],
        total: f64,
        action: u8,
    ) {
        if rows.is_empty() {
            return;
        }
        ui.add_space(20.);
        ui.heading(self.t(heading));
        for (n, row) in rows.iter().take(10).enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:02}", n + 1)).weak());
                let response = ui.add(
                    egui::Label::new(&row.name)
                        .truncate()
                        .sense(egui::Sense::click()),
                );
                if response.clicked() {
                    match action {
                        0 => self.play(&row.id),
                        1 => self.open_artist(&row.name),
                        _ => (),
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "{} · {:.1}%",
                        duration(row.seconds),
                        row.seconds / total.max(1.) * 100.
                    ));
                });
            });
            ui.add(egui::ProgressBar::new((row.seconds / total.max(1.)) as f32).desired_height(4.));
        }
    }
}
