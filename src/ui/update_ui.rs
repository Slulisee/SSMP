use super::*;
impl App {
    pub(super) fn check_app_update(&mut self, manual: bool) {
        if self.update_busy || !cfg!(windows) {
            return;
        }
        self.update_busy = true;
        self.update_manual = manual;
        self.update_status = self.t("Checking for app updates…").into();
        let root = self.store.root.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(root.join("updates"));
            let _ = std::fs::write(root.join("updates/last-check.txt"), ssmp::now().to_string());
            let result = ssmp::updates::check().map_err(|e| e.to_string());
            let _ = tx.send(Event::UpdateChecked(result));
        });
    }
    pub(super) fn download_app_update(&mut self) {
        let Some(release) = self.update_offer.clone() else {
            return;
        };
        if self.update_busy {
            return;
        }
        self.update_busy = true;
        self.update_status = self.t("Downloading app update…").into();
        self.update_progress = None;
        let root = self.store.root.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = ssmp::updates::download(&root, release, |done, total| {
                let _ = tx.send(Event::UpdateProgress(done, total));
            })
            .map_err(|e| e.to_string());
            let _ = tx.send(Event::UpdateDownloaded(result));
        });
    }
    pub(super) fn update_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(22.);
        ui.heading(self.t("App updates"));
        ui.label(format!("SSMP {}", env!("CARGO_PKG_VERSION")));
        let text = self.t("Download updates and install after closing");
        if ui
            .checkbox(&mut self.state.preferences.automatic_updates, text)
            .changed()
        {
            self.changed();
        }
        ui.label(RichText::new(self.t("Checks this project's stable releases once a day. Library and media files stay in place.")).small().weak());
        ui.label(&self.update_status);
        if let Some((done, total)) = self.update_progress {
            ui.add(egui::ProgressBar::new(done as f32 / total.max(1) as f32).show_percentage());
        }
        if let Some(version) = ssmp::updates::pending_version(&self.store.root) {
            ui.label(format!(
                "{}: {version}",
                self.t("Ready to install after closing")
            ));
            if !self.state.preferences.automatic_updates
                && ui
                    .button(self.t("Install after closing this time"))
                    .clicked()
            {
                self.update_apply_once = true;
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.update_busy && cfg!(windows),
                    egui::Button::new(self.t("Check for app updates")),
                )
                .clicked()
            {
                self.check_app_update(true);
            }
            if self.update_offer.is_some()
                && ui
                    .add_enabled(
                        !self.update_busy,
                        egui::Button::new(self.t("Download update")),
                    )
                    .clicked()
            {
                self.download_app_update();
            }
            if ui.button(self.t("Release notes")).clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(
                    "https://github.com/Slulisee/SSMP/releases",
                ));
            }
        });
    }
    pub(super) fn update_tick(&mut self) {
        if !self.smoke
            && !self.update_checked
            && self.started.elapsed() > Duration::from_secs(8)
            && self.state.preferences.automatic_updates
            && cfg!(windows)
        {
            self.update_checked = true;
            let last = std::fs::read_to_string(self.store.root.join("updates/last-check.txt"))
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            if ssmp::now().saturating_sub(last) > 86400 {
                self.check_app_update(false);
            }
        }
    }
}
