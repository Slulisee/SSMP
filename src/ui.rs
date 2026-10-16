use anyhow::{Context, Result};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use ssmp::{library, model::*, player::Player, server::Remote, store::Store};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

type TaskResult<T> = std::result::Result<T, String>;
enum Event {
    Imported(TaskResult<ImportReport>),
    Playlist(TaskResult<Playlist>),
    Connected(TaskResult<(Remote, Vec<Media>, UserState)>),
    Saved(TaskResult<UserState>),
    Downloaded(TaskResult<Media>),
    Message(TaskResult<String>),
    Lyrics(TaskResult<(String, String)>),
}
#[derive(Clone, PartialEq)]
enum Page {
    Home,
    Music,
    Albums,
    Videos,
    Favorites,
    History,
    Playlists,
    Offline,
    NowPlaying,
    Server,
    Settings,
}
impl Page {
    fn title(&self) -> &str {
        match self {
            Self::Home => "Made for your collection",
            Self::Music => "Music",
            Self::Albums => "Albums",
            Self::Videos => "Videos & series",
            Self::Favorites => "Favorites",
            Self::History => "Listening history",
            Self::Playlists => "Playlists",
            Self::Offline => "Available offline",
            Self::NowPlaying => "Now playing",
            Self::Server => "Your server",
            Self::Settings => "Make it yours",
        }
    }
}

fn centered_column<R>(
    ui: &mut egui::Ui,
    max_width: f32,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let available = ui.available_rect_before_wrap();
    let width = available.width().min(max_width);
    let rect = egui::Rect::from_min_size(
        egui::pos2(available.center().x - width / 2., available.top()),
        Vec2::new(width, available.height()),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.set_width(width);
            contents(ui)
        },
    )
}
fn centered_row<R>(
    ui: &mut egui::Ui,
    max_width: f32,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.horizontal(|ui| {
        let width = ui.available_width().min(max_width);
        ui.add_space(((ui.available_width() - width) / 2.).max(0.));
        ui.allocate_ui_with_layout(
            Vec2::new(width, 0.),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_width(width);
                contents(ui)
            },
        )
        .inner
    })
}

pub fn launch(root: PathBuf, smoke: bool, smoke_page: Option<String>) -> Result<()> {
    let icon = image::load_from_memory(include_bytes!("../assets/SSMP.png"))?.to_rgba8();
    let icon = egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_raw(),
    };
    let size = if smoke {
        std::env::var("SSMP_SMOKE_SIZE")
            .ok()
            .and_then(|value| {
                let (width, height) = value.split_once('x')?;
                Some([
                    width.parse::<f32>().ok()?.clamp(1000., 3200.),
                    height.parse::<f32>().ok()?.clamp(650., 2160.),
                ])
            })
            .unwrap_or([1440., 920.])
    } else {
        [1440., 920.]
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon)
            .with_title(format!("SSMP — {}", ssmp::APP_NAME))
            .with_inner_size(size)
            .with_min_inner_size([1000., 650.]),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        "SSMP",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(cc, root, smoke, smoke_page)?))
        }),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}
struct App {
    store: Store,
    rows: Vec<Media>,
    state: UserState,
    profile_key: String,
    page: Page,
    search: String,
    player: Option<Player>,
    engine_error: String,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    busy: usize,
    dirty: bool,
    last_save: Instant,
    last_inbox: Instant,
    last_position: Instant,
    started: Instant,
    loaded_at: Instant,
    saving: bool,
    remote: Option<Remote>,
    remote_preferences: Preferences,
    remote_rows: Vec<Media>,
    status: String,
    error: bool,
    new_playlist: String,
    selected_playlist: Option<String>,
    edit: Option<Media>,
    host_url: String,
    username: String,
    password: String,
    host_bind: String,
    host_user: String,
    host_password: String,
    host_child: Option<std::process::Child>,
    repeat: u8,
    lyrics_offset: f64,
    sub_offset: f64,
    sub_scale: f64,
    primary_sub: i64,
    secondary_sub: i64,
    audio_track: i64,
    last_track: String,
    listen_start: f64,
    listen_recorded: bool,
    listen_seconds: f64,
    last_tick: Instant,
    thumbnails: HashMap<String, egui::TextureHandle>,
    smoke: bool,
    smoke_page: Option<String>,
    screenshot_sent: bool,
    show_welcome: bool,
    manifest: Option<(PathBuf, ImportManifest)>,
    smoke_expected: String,
}
impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        root: PathBuf,
        smoke: bool,
        smoke_page: Option<String>,
    ) -> Result<Self> {
        let store = Store::open(&root)?;
        let state = store.state("local")?;
        let rows = store.media()?;
        let (tx, rx) = mpsc::channel();
        let player = Player::new(cc.gl.as_deref(), smoke);
        let engine_error = player
            .as_ref()
            .err()
            .map(|e| format!("{e:#}"))
            .unwrap_or_default();
        let show_welcome = !state.onboarding && !smoke;
        let mut app = Self {
            store,
            rows,
            state,
            profile_key: "local".into(),
            page: Page::Home,
            search: String::new(),
            player: player.ok(),
            engine_error,
            tx,
            rx,
            busy: 0,
            dirty: false,
            last_save: Instant::now(),
            last_inbox: Instant::now(),
            last_position: Instant::now(),
            started: Instant::now(),
            loaded_at: Instant::now(),
            saving: false,
            remote: None,
            remote_preferences: Preferences::default(),
            remote_rows: Vec::new(),
            status: "Ready when you are. Add a folder or drop your files here.".into(),
            error: false,
            new_playlist: String::new(),
            selected_playlist: None,
            edit: None,
            host_url: "http://127.0.0.1:4534".into(),
            username: String::new(),
            password: String::new(),
            host_bind: "127.0.0.1:4534".into(),
            host_user: "owner".into(),
            host_password: String::new(),
            host_child: None,
            repeat: 0,
            lyrics_offset: 0.,
            sub_offset: 0.,
            sub_scale: 1.,
            primary_sub: -1,
            secondary_sub: 0,
            audio_track: -1,
            last_track: String::new(),
            listen_start: 0.,
            listen_recorded: false,
            listen_seconds: 0.,
            last_tick: Instant::now(),
            thumbnails: HashMap::new(),
            smoke,
            smoke_page,
            screenshot_sent: false,
            show_welcome,
            manifest: None,
            smoke_expected: String::new(),
        };
        app.selected_playlist = app.state.playlists.first().map(|p| p.id.clone());
        app.apply_preferences(&cc.egui_ctx);
        if let Some(player) = app.player.as_mut() {
            player.set("volume", &app.state.preferences.volume.to_string());
            player.set("speed", &app.state.preferences.speed.to_string());
        }
        if smoke {
            app.state.onboarding = true;
            app.state.preferences.history = true;
            app.state.preferences.recommendations = true;
            if app.state.playlists.is_empty() && !app.rows.is_empty() {
                app.state.playlists.push(Playlist {
                    id: "smoke-favorites".into(),
                    name: "Sunday, slowed down".into(),
                    items: app
                        .rows
                        .iter()
                        .filter(|m| m.kind == "audio")
                        .map(|m| m.id.clone())
                        .collect(),
                    unresolved: vec![],
                });
                app.selected_playlist = Some("smoke-favorites".into());
            }
            if let Some(m) = app.rows.iter().find(|m| m.kind == "audio").cloned() {
                app.smoke_expected = m.path.clone();
                app.state.favorites = vec![m.id.clone()];
                app.play(&m.id);
            }
            if let Some(page) = app.smoke_page.as_deref() {
                app.page = match page {
                    "video" => Page::NowPlaying,
                    "settings" => Page::Settings,
                    "server" => Page::Server,
                    "playlists" => Page::Playlists,
                    _ => Page::Home,
                };
            }
            if app.smoke_page.as_deref() == Some("video") {
                if let Some(m) = app.rows.iter().find(|m| m.kind == "video").cloned() {
                    app.smoke_expected = m.path.clone();
                    app.play(&m.id);
                    app.page = Page::NowPlaying;
                    app.primary_sub = 1;
                    app.secondary_sub = 2;
                    if let Some(player) = app.player.as_mut() {
                        player.command(&["set", "sid", "1"]);
                        player.command(&["set", "secondary-sid", "2"]);
                    }
                }
            }
        }
        Ok(app)
    }
    fn notify(&mut self, message: impl Into<String>, error: bool) {
        self.status = message.into();
        self.error = error;
    }
    fn changed(&mut self) {
        self.dirty = true;
    }
    fn library(&self) -> &[Media] {
        if self.remote.is_some() {
            &self.remote_rows
        } else {
            &self.rows
        }
    }
    fn current(&self) -> Option<Media> {
        self.state
            .queue
            .get(self.state.queue_index)
            .and_then(|id| self.library().iter().find(|m| &m.id == id))
            .cloned()
    }
    fn apply_preferences(&self, ctx: &egui::Context) {
        let p = &self.state.preferences;
        let mut v = if p.theme == "Daylight" {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        if p.theme != "Daylight" {
            v.panel_fill = if p.theme == "OLED" {
                Color32::BLACK
            } else if p.theme == "Ocean" {
                Color32::from_rgb(12, 25, 35)
            } else {
                Color32::from_rgb(18, 18, 24)
            };
            v.window_fill = if p.theme == "OLED" {
                Color32::from_rgb(8, 8, 10)
            } else {
                Color32::from_rgb(25, 25, 34)
            };
            v.extreme_bg_color = Color32::from_rgb(13, 13, 18);
            v.faint_bg_color = Color32::from_rgb(29, 29, 38);
            v.widgets.inactive.bg_fill = Color32::from_rgb(39, 39, 50);
            v.widgets.hovered.bg_fill = Color32::from_rgb(54, 51, 72);
            v.widgets.active.bg_fill = Color32::from_rgb(67, 58, 94);
        }
        v.selection.bg_fill = Color32::from_rgb(p.accent[0] / 3, p.accent[1] / 3, p.accent[2] / 3);
        v.selection.stroke = Stroke::new(1_f32, self.accent());
        ctx.set_visuals(v);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10., 10.);
        style.spacing.button_padding = Vec2::new(12., 8.);
        style.spacing.slider_width = 220.;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.));
        ctx.set_style(style);
        ctx.set_zoom_factor(p.scale);
    }
    fn accent(&self) -> Color32 {
        Color32::from_rgb(
            self.state.preferences.accent[0],
            self.state.preferences.accent[1],
            self.state.preferences.accent[2],
        )
    }
    fn import_paths(&mut self, paths: Vec<PathBuf>) {
        if self.remote.is_some() {
            self.notify("Switch to your local library to import files. Server owners can scan folders on the host.",true);
            return;
        }
        for p in &paths {
            if p.is_dir() {
                let s = p.to_string_lossy().into_owned();
                if !self.state.roots.contains(&s) {
                    self.state.roots.push(s);
                    self.changed();
                }
            }
        }
        let root = self.store.root.clone();
        let tx = self.tx.clone();
        self.busy += 1;
        std::thread::spawn(move || {
            let r = (|| -> Result<_> { library::scan(&Store::open(&root)?, &paths) })();
            let _ = tx.send(Event::Imported(r.map_err(|e| format!("{e:#}"))));
        });
        self.notify("Reading your files and metadata…", false);
    }
    fn add_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Choose a music or video folder")
            .pick_folder()
        {
            self.import_paths(vec![path]);
        }
    }
    fn add_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Add music or videos")
            .add_filter(
                "Media",
                &[
                    "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "mp4", "mkv", "webm", "avi",
                    "mov",
                ],
            )
            .pick_files()
        {
            self.import_paths(paths);
        }
    }
    fn playlist_import(&mut self) {
        if self.remote.is_some() {
            self.notify("Import a playlist in the local library first. Hosted playlist import will follow the same matching workflow.",true);
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Playlists", &["m3u", "m3u8", "pls", "xspf", "xml"])
            .pick_file()
        {
            let root = self.store.root.clone();
            let tx = self.tx.clone();
            self.busy += 1;
            std::thread::spawn(move || {
                let r =
                    (|| -> Result<_> { library::import_playlist(&Store::open(&root)?, &path) })();
                let _ = tx.send(Event::Playlist(r.map_err(|e| format!("{e:#}"))));
            });
        }
    }
    fn load_queue(&mut self, position: f64) {
        let rows = self.library().to_vec();
        let mut ids = Vec::new();
        let mut paths = Vec::new();
        for id in &self.state.queue {
            if let Some(m) = rows.iter().find(|m| &m.id == id) {
                let cached = self
                    .store
                    .get(id)
                    .ok()
                    .flatten()
                    .filter(|m| std::path::Path::new(&m.path).is_file());
                let path = if let Some(m) = cached {
                    m.path
                } else if let Some(remote) = &self.remote {
                    remote.playback_url(id)
                } else {
                    m.path.clone()
                };
                ids.push(id.clone());
                paths.push(path);
            }
        }
        self.state.queue = ids;
        self.state.queue_index = self.state.queue_index.min(paths.len().saturating_sub(1));
        if !paths.is_empty() {
            let token = self.remote.as_ref().map(|r| r.connection.token.clone());
            if let Some(player) = self.player.as_mut() {
                player.load(&paths, self.state.queue_index, position, token.as_deref());
            } else {
                self.notify(self.engine_error.clone(), true)
            }
            self.loaded_at = Instant::now();
            self.changed();
        }
    }
    fn play(&mut self, id: &str) {
        let kind = self
            .library()
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.kind.clone())
            .unwrap_or_default();
        let selection: Vec<_> = self
            .filtered()
            .into_iter()
            .filter(|m| m.kind == kind)
            .collect();
        self.state.queue = selection.iter().map(|m| m.id.clone()).collect();
        if !self.state.queue.iter().any(|s| s == id) {
            self.state.queue = vec![id.into()]
        }
        self.state.queue_index = self.state.queue.iter().position(|s| s == id).unwrap_or(0);
        let is_video = self
            .library()
            .iter()
            .find(|m| m.id == id)
            .is_some_and(|m| m.kind == "video");
        let resume = if is_video {
            self.state.preferences.resume_video
        } else {
            self.state.preferences.resume_music
        };
        let position = if resume {
            *self.state.positions.get(id).unwrap_or(&0.)
        } else {
            0.
        };
        self.load_queue(position);
        if is_video {
            self.page = Page::NowPlaying
        }
    }
    fn play_album(&mut self, media: &Media) {
        if media.kind != "audio" || media.album.is_empty() {
            self.play(&media.id);
            return;
        }
        let mut tracks: Vec<_> = self
            .library()
            .iter()
            .filter(|m| m.kind == "audio" && m.album == media.album && m.artist == media.artist)
            .cloned()
            .collect();
        tracks.sort_by(|a, b| a.track.cmp(&b.track).then(a.title.cmp(&b.title)));
        self.state.queue = tracks.iter().map(|m| m.id.clone()).collect();
        self.state.queue_index = 0;
        self.resume_queue();
    }
    fn enqueue(&mut self, id: &str, next: bool) {
        if self.state.queue.is_empty() {
            self.state.queue.push(id.into());
            self.state.queue_index = 0;
            self.changed();
            return;
        }
        let index = if next {
            (self.state.queue_index + 1).min(self.state.queue.len())
        } else {
            self.state.queue.len()
        };
        self.state.queue.insert(index, id.into());
        let m = self.library().iter().find(|m| m.id == id).cloned();
        if let Some(m) = m {
            let path = if let Some(r) = &self.remote {
                r.playback_url(id)
            } else {
                m.path
            };
            if let Some(p) = self.player.as_mut() {
                p.command(&["loadfile", &path, "append"]);
                if next {
                    p.command(&[
                        "playlist-move",
                        &(self.state.queue.len() - 1).to_string(),
                        &index.to_string(),
                    ]);
                }
            }
        }
        self.changed();
        self.notify("Added to your queue", false);
    }
    fn step(&mut self, delta: i32) {
        if self.state.queue.is_empty() {
            return;
        }
        let n = self.state.queue.len() as i32;
        self.state.queue_index = (self.state.queue_index as i32 + delta).rem_euclid(n) as usize;
        if let Some(p) = self.player.as_mut() {
            p.set("playlist-pos", &self.state.queue_index.to_string());
            p.set("pause", "no");
        }
        self.changed();
    }
    fn filtered(&self) -> Vec<Media> {
        let mut rows: Vec<_> = self
            .library()
            .iter()
            .filter(|m| match self.page {
                Page::Music | Page::Albums => m.kind == "audio",
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
                self.search.is_empty()
                    || format!("{} {} {} {}", m.title, m.artist, m.album, m.series)
                        .to_lowercase()
                        .contains(&self.search.to_lowercase())
            })
            .cloned()
            .collect();
        if self.page == Page::History {
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
        }
        if self.page == Page::Playlists {
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
        }
        rows
    }
    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            self.busy = self.busy.saturating_sub(1);
            match event {
                Event::Imported(r) => match r {
                    Ok(r) => {
                        self.rows = self.store.media().unwrap_or_default();
                        self.notify(
                            format!(
                                "{} added · {} refreshed{}",
                                r.added,
                                r.updated,
                                if r.errors.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · {} issue(s): {}", r.errors.len(), r.errors[0])
                                }
                            ),
                            !r.errors.is_empty(),
                        )
                    }
                    Err(e) => self.notify(e, true),
                },
                Event::Playlist(r) => match r {
                    Ok(p) => {
                        self.rows = self.store.media().unwrap_or_default();
                        self.selected_playlist = Some(p.id.clone());
                        let n = p.items.len();
                        let missing = p.unresolved.len();
                        self.state.playlists.push(p);
                        self.page = Page::Playlists;
                        self.changed();
                        self.notify(
                            format!("Playlist imported: {n} tracks, {missing} unresolved entries"),
                            false,
                        )
                    }
                    Err(e) => self.notify(e, true),
                },
                Event::Connected(r) => match r {
                    Ok((remote, rows, mut state)) => {
                        let _ = self.store.save_state(&self.profile_key, &self.state);
                        self.profile_key =
                            format!("{}:{}", remote.connection.url, remote.connection.username);
                        let server_preferences = state.preferences.clone();
                        if let Ok(local) = self.store.state(&self.profile_key) {
                            if !local.preferences.sync_settings {
                                state.preferences = local.preferences;
                            }
                        }
                        self.remote_preferences = server_preferences;
                        self.remote = Some(remote);
                        self.remote_rows = rows;
                        self.state = state;
                        self.dirty = false;
                        self.apply_preferences(ctx);
                        self.notify("Connected. Your personal library state is loaded.", false);
                    }
                    Err(e) => self.notify(e, true),
                },
                Event::Saved(r) => {
                    self.saving = false;
                    match r {
                        Ok(s) => {
                            self.state.revision = s.revision;
                            let _ = self.store.save_state(&self.profile_key, &self.state);
                        }
                        Err(e) => self.notify(e, true),
                    }
                }
                Event::Downloaded(r) => match r {
                    Ok(m) => {
                        self.rows = self.store.media().unwrap_or_default();
                        self.notify(format!("Available offline: {}", m.title), false)
                    }
                    Err(e) => self.notify(e, true),
                },
                Event::Message(r) => match r {
                    Ok(s) => {
                        self.rows = self.store.media().unwrap_or_default();
                        self.notify(s, false)
                    }
                    Err(e) => self.notify(e, true),
                },
                Event::Lyrics(r) => match r {
                    Ok((id, text)) => {
                        if let Ok(Some(mut m)) = self.store.get(&id) {
                            m.lyrics = text;
                            m.locked = true;
                            let _ = self.store.put(&m);
                            self.rows = self.store.media().unwrap_or_default();
                            self.notify("Lyrics saved in your library", false)
                        }
                    }
                    Err(e) => self.notify(e, true),
                },
            }
        }
    }
    fn save(&mut self) {
        if !self.dirty || self.saving {
            return;
        }
        self.dirty = false;
        self.last_save = Instant::now();
        if let Err(e) = self.store.save_state(&self.profile_key, &self.state) {
            self.notify(e.to_string(), true);
            self.dirty = true;
            return;
        }
        if let Some(remote) = self.remote.clone() {
            let mut state = self.state.clone();
            if !state.preferences.sync_settings {
                state.preferences = self.remote_preferences.clone();
            }
            let tx = self.tx.clone();
            self.saving = true;
            self.busy += 1;
            std::thread::spawn(move || {
                let r = remote.save(&state).map_err(|e| format!("{e:#}"));
                let _ = tx.send(Event::Saved(r));
            });
        }
    }
    fn download(&mut self, id: &str) {
        if let (Some(remote), Some(m)) = (
            self.remote.clone(),
            self.library().iter().find(|m| m.id == id).cloned(),
        ) {
            let root = self.store.root.clone();
            let tx = self.tx.clone();
            self.busy += 1;
            self.notify(
                format!("Downloading {} for offline playback…", m.title),
                false,
            );
            std::thread::spawn(move || {
                let r = (|| -> Result<_> { remote.download(&Store::open(&root)?, &m) })();
                let _ = tx.send(Event::Downloaded(r.map_err(|e| format!("{e:#}"))));
            });
        } else {
            self.notify("This file is already available locally", false)
        }
    }
    fn download_playlist(&mut self, ids: &[String]) {
        if let Some(remote) = self.remote.clone() {
            let items: Vec<_> = ids
                .iter()
                .filter_map(|id| self.library().iter().find(|m| &m.id == id).cloned())
                .collect();
            let root = self.store.root.clone();
            let tx = self.tx.clone();
            self.busy += 1;
            self.notify("Downloading playlist one file at a time…", false);
            std::thread::spawn(move || {
                let result = (|| -> Result<_> {
                    let store = Store::open(&root)?;
                    let mut count = 0;
                    for item in items {
                        remote.download(&store, &item)?;
                        count += 1;
                    }
                    Ok(format!("{count} playlist items are available offline"))
                })();
                let _ = tx.send(Event::Message(
                    result.map_err(|e| format!("Playlist download stopped: {e}")),
                ));
            });
        }
    }
    fn resume_queue(&mut self) {
        let position = self
            .current()
            .filter(|m| {
                if m.kind == "video" {
                    self.state.preferences.resume_video
                } else {
                    self.state.preferences.resume_music
                }
            })
            .and_then(|m| self.state.positions.get(&m.id).copied())
            .unwrap_or(0.);
        self.load_queue(position);
    }

    fn cover(&mut self, ui: &mut egui::Ui, m: &Media, size: f32) -> egui::Response {
        if !m.artwork.is_empty() {
            if !self.thumbnails.contains_key(&m.artwork) {
                if let Ok(image) = image::open(&m.artwork) {
                    let image = image.thumbnail(320, 320).to_rgba8();
                    let tex = ui.ctx().load_texture(
                        &m.artwork,
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.as_raw(),
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    self.thumbnails.insert(m.artwork.clone(), tex);
                }
            }
            if let Some(tex) = self.thumbnails.get(&m.artwork) {
                return ui.add(egui::Image::new((tex.id(), Vec2::splat(size))).corner_radius(8.));
            }
        }
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        let colors = [
            [60, 49, 88],
            [39, 76, 82],
            [91, 56, 68],
            [56, 65, 100],
            [85, 70, 45],
        ];
        let color = colors[m.id.bytes().fold(0usize, |a, b| a + b as usize) % colors.len()];
        ui.painter()
            .rect_filled(rect, 8., Color32::from_rgb(color[0], color[1], color[2]));
        let center = rect.center();
        let radius = size * 0.3;
        ui.painter().circle_stroke(
            center,
            radius,
            Stroke::new(size * 0.008, Color32::from_white_alpha(35)),
        );
        ui.painter().circle_stroke(
            center,
            radius * 0.68,
            Stroke::new(size * 0.008, Color32::from_white_alpha(25)),
        );
        let letter = m
            .album
            .chars()
            .next()
            .or_else(|| m.title.chars().next())
            .unwrap_or('S')
            .to_uppercase()
            .to_string();
        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            letter,
            egui::FontId::proportional(size * 0.27),
            Color32::from_white_alpha(225),
        );
        response
    }
    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(210.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(20.),
            )
            .show(ctx, |ui| {
                let area = ui.available_rect_before_wrap();
                let footer = egui::Rect::from_min_max(
                    egui::pos2(area.left(), area.bottom() - 108.),
                    area.max,
                );
                let navigation = egui::Rect::from_min_max(
                    area.min,
                    egui::pos2(area.right(), footer.top() - 12.),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("nav-scroll")
                        .max_rect(navigation),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("navigation-scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 6.;
                                ui.vertical_centered(|ui| {
                                    ui.add_space(4.);
                                    ui.label(
                                        RichText::new("SSMP")
                                            .size(32.)
                                            .strong()
                                            .color(self.accent()),
                                    )
                                    .on_hover_text(ssmp::APP_NAME);
                                    ui.label(
                                        RichText::new("YOUR COLLECTION. YOUR RULES.")
                                            .size(9.)
                                            .weak(),
                                    );
                                });
                                ui.add_space(18.);
                                for (symbol, label, page) in [
                                    ("H", "Home", Page::Home),
                                    ("M", "Music", Page::Music),
                                    ("A", "Albums", Page::Albums),
                                    ("V", "Videos & series", Page::Videos),
                                ] {
                                    self.nav(ui, symbol, label, page);
                                }
                                ui.add_space(12.);
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new("YOUR LIBRARY").size(10.).weak());
                                });
                                ui.add_space(4.);
                                for (symbol, label, page) in [
                                    ("+", "Favorites", Page::Favorites),
                                    ("P", "Playlists", Page::Playlists),
                                    ("R", "History", Page::History),
                                    ("D", "Offline", Page::Offline),
                                ] {
                                    self.nav(ui, symbol, label, page);
                                }
                                ui.add_space(16.);
                                if ui
                                    .add_sized(
                                        [168., 36.],
                                        egui::Button::new(
                                            RichText::new("+  Add a folder").color(self.accent()),
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.add_folder();
                                }
                                ui.add_space(12.);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(if self.remote.is_some() {
                                            "CONNECTED LIBRARY"
                                        } else {
                                            "LOCAL LIBRARY"
                                        })
                                        .size(10.)
                                        .weak(),
                                    );
                                    if let Some(remote) = &self.remote {
                                        ui.label(
                                            RichText::new(&remote.connection.username).small(),
                                        );
                                        if ui.small_button("Back to local library").clicked() {
                                            self.save();
                                            self.remote = None;
                                            self.profile_key = "local".into();
                                            self.state =
                                                self.store.state("local").unwrap_or_default();
                                            self.remote_rows.clear();
                                            self.dirty = false;
                                            self.apply_preferences(ctx);
                                        }
                                    } else {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} items · no account needed",
                                                self.rows.len()
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                    }
                                });
                            });
                    },
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("nav-footer")
                        .max_rect(footer)
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 8.;
                        self.nav(ui, "N", "Your server", Page::Server);
                        self.nav(ui, "S", "Settings", Page::Settings);
                        ui.label(RichText::new("SSMP 0.1 · Open source").size(10.).weak());
                    },
                );
            });
    }
    fn nav(&mut self, ui: &mut egui::Ui, symbol: &str, label: &str, page: Page) {
        let active = self.page == page;
        let text = RichText::new(format!("{symbol}     {label}")).color(if active {
            self.accent()
        } else {
            ui.visuals().text_color()
        });
        if ui
            .add_sized(
                [
                    168.,
                    if ui.ctx().content_rect().height() < 800. {
                        32.
                    } else {
                        37.
                    },
                ],
                egui::Button::new(text)
                    .fill(if active {
                        ui.visuals().selection.bg_fill
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(Stroke::NONE)
                    .corner_radius(7.),
            )
            .clicked()
        {
            self.page = page;
            self.search.clear();
        }
    }
    fn bottom(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("transport")
            .exact_height(132.)
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.window_fill)
                    .inner_margin(egui::Margin::symmetric(24, 12)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().button_padding = Vec2::new(8., 8.);
                let current = self.current();
                let snapshot = self
                    .player
                    .as_ref()
                    .map(|p| p.snapshot.clone())
                    .unwrap_or_default();
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 108.),
                    egui::Sense::hover(),
                );
                let compact = rect.width() < 1100.;
                let narrow = rect.width() < 850.;
                let widths = if compact {
                    [46., 34., 72., 34., 46.]
                } else {
                    [96., 38., 76., 38., 96.]
                };
                let controls_width: f32 = widths.iter().sum::<f32>() + 32.;
                let controls = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 28.),
                    Vec2::new(controls_width, 56.),
                );
                let volume_width = if compact { 148. } else { 196. };
                let volume = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - volume_width, rect.top()),
                    Vec2::new(volume_width, 56.),
                );
                let extras_width = if narrow {
                    36.
                } else if compact {
                    78.
                } else {
                    160.
                };
                let extras = egui::Rect::from_min_size(
                    egui::pos2(volume.left() - 12. - extras_width, rect.top()),
                    Vec2::new(extras_width, 56.),
                );
                let info = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2((controls.left() - 18.).max(rect.left()), rect.top() + 56.),
                );

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("track-info")
                        .max_rect(info)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.set_clip_rect(info.intersect(ui.clip_rect()));
                        if let Some(m) = &current {
                            self.cover(ui, m, 48.);
                            let width = ui.available_width();
                            ui.allocate_ui_with_layout(
                                Vec2::new(width, 40.),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(width);
                                    ui.spacing_mut().item_spacing.y = 3.;
                                    if ui
                                        .add(
                                            egui::Label::new(RichText::new(&m.title).strong())
                                                .truncate()
                                                .sense(egui::Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        self.page = Page::NowPlaying;
                                    }
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(m.description()).small().weak(),
                                        )
                                        .truncate(),
                                    );
                                },
                            );
                        } else {
                            ui.add(
                                egui::Label::new(
                                    RichText::new("Choose something to play").strong(),
                                )
                                .truncate(),
                            );
                        }
                    },
                );

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("playback-controls")
                        .max_rect(controls)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.;
                        if ui
                            .add_sized(
                                [widths[0], 36.],
                                egui::Button::new(if compact { "Mix" } else { "Shuffle" })
                                    .corner_radius(8.),
                            )
                            .on_hover_text("Shuffle the queue")
                            .clicked()
                        {
                            use rand::seq::SliceRandom;
                            self.state.queue.shuffle(&mut rand::thread_rng());
                            self.state.queue_index = 0;
                            self.load_queue(0.);
                        }
                        if ui
                            .add_sized([widths[1], 36.], egui::Button::new("|<").corner_radius(8.))
                            .on_hover_text("Previous track")
                            .clicked()
                        {
                            self.step(-1);
                        }
                        let text = if snapshot.paused || snapshot.path.is_empty() {
                            "Play"
                        } else {
                            "Pause"
                        };
                        if ui
                            .add_sized(
                                [widths[2], 40.],
                                egui::Button::new(RichText::new(text).strong())
                                    .fill(self.accent())
                                    .stroke(Stroke::NONE)
                                    .corner_radius(10.),
                            )
                            .on_hover_text("Play / pause · Space")
                            .clicked()
                        {
                            if snapshot.path.is_empty() {
                                self.resume_queue();
                            } else if let Some(p) = self.player.as_mut() {
                                p.toggle();
                            }
                        }
                        if ui
                            .add_sized([widths[3], 36.], egui::Button::new(">|").corner_radius(8.))
                            .on_hover_text("Next track")
                            .clicked()
                        {
                            self.step(1);
                        }
                        let repeat = match (self.repeat, compact) {
                            (0, true) => "Off",
                            (1, true) => "All",
                            (_, true) => "One",
                            (0, false) => "Repeat off",
                            (1, false) => "Repeat all",
                            _ => "Repeat one",
                        };
                        let mut button = egui::Button::new(repeat).corner_radius(8.);
                        if self.repeat > 0 {
                            button = button.fill(ui.visuals().selection.bg_fill);
                        }
                        if ui
                            .add_sized([widths[4], 36.], button)
                            .on_hover_text("Repeat: off / all / one")
                            .clicked()
                        {
                            self.repeat = (self.repeat + 1) % 3;
                            if let Some(p) = self.player.as_mut() {
                                p.set("loop-playlist", if self.repeat == 1 { "inf" } else { "no" });
                                p.set("loop-file", if self.repeat == 2 { "inf" } else { "no" });
                            }
                        }
                    },
                );

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("player-actions")
                        .max_rect(extras)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.;
                        let favorite = current
                            .as_ref()
                            .is_some_and(|m| self.state.favorites.contains(&m.id));
                        if narrow {
                            ui.menu_button("…", |ui| {
                                if let Some(m) = &current {
                                    if ui
                                        .button(if favorite {
                                            "Remove favorite"
                                        } else {
                                            "Favorite"
                                        })
                                        .clicked()
                                    {
                                        self.state.favorite(&m.id);
                                        self.changed();
                                        ui.close();
                                    }
                                }
                                if ui.button("Show / hide queue").clicked() {
                                    self.state.preferences.show_queue =
                                        !self.state.preferences.show_queue;
                                    self.changed();
                                    ui.close();
                                }
                            });
                        } else {
                            let text = if compact {
                                if favorite { "♥" } else { "+" }
                            } else if favorite {
                                "Favorited"
                            } else {
                                "Favorite"
                            };
                            if ui
                                .add_enabled(
                                    current.is_some(),
                                    egui::Button::new(text)
                                        .min_size(Vec2::new(if compact { 34. } else { 88. }, 36.))
                                        .corner_radius(8.),
                                )
                                .on_hover_text("Toggle favorite")
                                .clicked()
                            {
                                if let Some(m) = &current {
                                    self.state.favorite(&m.id);
                                    self.changed();
                                }
                            }
                            let mut button = egui::Button::new(if compact { "Q" } else { "Queue" })
                                .min_size(Vec2::new(if compact { 36. } else { 64. }, 36.))
                                .corner_radius(8.);
                            if self.state.preferences.show_queue {
                                button = button.fill(ui.visuals().selection.bg_fill);
                            }
                            if ui.add(button).on_hover_text("Show / hide queue").clicked() {
                                self.state.preferences.show_queue =
                                    !self.state.preferences.show_queue;
                                self.changed();
                            }
                        }
                    },
                );

                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("volume")
                        .max_rect(volume)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.;
                        let rail_width = if compact { 108. } else { 148. };
                        ui.add_sized(
                            [volume_width - rail_width - 8., 24.],
                            egui::Label::new(
                                RichText::new(if compact { "Vol" } else { "Volume" })
                                    .size(11.)
                                    .weak(),
                            ),
                        );
                        ui.spacing_mut().slider_width = rail_width;
                        if ui
                            .add_sized(
                                [rail_width, 24.],
                                egui::Slider::new(&mut self.state.preferences.volume, 0.0..=100.)
                                    .show_value(false)
                                    .trailing_fill(true),
                            )
                            .on_hover_text(format!("Volume: {:.0}%", self.state.preferences.volume))
                            .changed()
                        {
                            if let Some(p) = self.player.as_mut() {
                                p.set("volume", &self.state.preferences.volume.to_string());
                            }
                            self.changed();
                        }
                    },
                );

                let seek = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), rect.top() + 64.),
                    Vec2::new(rect.width(), 24.),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("seek")
                        .max_rect(seek)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 10.;
                        ui.add_sized(
                            [52., 24.],
                            egui::Label::new(
                                RichText::new(duration(snapshot.position))
                                    .size(12.)
                                    .monospace()
                                    .weak(),
                            ),
                        );
                        let width = (seek.width() - 124.).max(1.);
                        ui.spacing_mut().slider_width = width;
                        let mut position = snapshot.position;
                        if ui
                            .add_sized(
                                [width, 24.],
                                egui::Slider::new(&mut position, 0.0..=snapshot.duration.max(1.))
                                    .show_value(false)
                                    .trailing_fill(true),
                            )
                            .on_hover_text("Seek · Left / Right arrows")
                            .changed()
                        {
                            if let Some(p) = self.player.as_mut() {
                                p.seek(position);
                            }
                        }
                        ui.add_sized(
                            [52., 24.],
                            egui::Label::new(
                                RichText::new(duration(snapshot.duration))
                                    .size(12.)
                                    .monospace()
                                    .weak(),
                            ),
                        );
                    },
                );

                let status = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), rect.top() + 94.),
                    Vec2::new(rect.width(), 14.),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .id_salt("player-status")
                        .max_rect(status)
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                    |ui| {
                        let text = if self.busy > 0 {
                            format!("Working · {}", self.status)
                        } else {
                            self.status.clone()
                        };
                        ui.add(
                            egui::Label::new(RichText::new(text).size(10.).color(if self.error {
                                Color32::from_rgb(245, 150, 145)
                            } else {
                                ui.visuals().weak_text_color()
                            }))
                            .truncate(),
                        );
                    },
                );
            });
    }
    fn side_panel(&mut self, ctx: &egui::Context) {
        if !self.state.preferences.show_queue && !self.state.preferences.show_lyrics {
            return;
        }
        egui::SidePanel::right("details").default_width(288.).width_range(230.0..=400.).frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill).inner_margin(20.)).show(ctx,|ui|{
            ui.add_space(8.);centered_row(ui,196.,|ui|{let q=self.state.preferences.show_queue;if ui.selectable_label(!q,"Lyrics").clicked(){self.state.preferences.show_queue=false;self.state.preferences.show_lyrics=true;self.changed();}if ui.selectable_label(q,"Queue").clicked(){self.state.preferences.show_queue=true;self.changed();}if ui.small_button("Hide").clicked(){self.state.preferences.show_queue=false;self.state.preferences.show_lyrics=false;self.changed();}});ui.add_space(20.);
            if self.state.preferences.show_queue{
                ui.label(RichText::new(format!("{} in your queue",self.state.queue.len())).strong());ui.add_space(10.);
                let items=self.state.queue.clone();egui::ScrollArea::vertical().show(ui,|ui|{for (i,id) in items.iter().enumerate(){if let Some(m)=self.library().iter().find(|m|&m.id==id).cloned(){
                    ui.push_id(i,|ui|{let active=i==self.state.queue_index;egui::Frame::new().fill(if active{ui.visuals().selection.bg_fill}else{Color32::TRANSPARENT}).inner_margin(8.).corner_radius(7.).show(ui,|ui|{
                        ui.horizontal(|ui|{if ui.selectable_label(active,RichText::new(&m.title).color(if active{self.accent()}else{ui.visuals().text_color()})).clicked(){self.state.queue_index=i;if let Some(p)=self.player.as_mut(){p.set("playlist-pos",&i.to_string());p.set("pause","no");}self.changed();}});
                        ui.label(RichText::new(m.description()).small().weak());ui.horizontal(|ui|{if i>0&&ui.small_button("Up").clicked(){self.state.queue.swap(i,i-1);if self.state.queue_index==i{self.state.queue_index-=1}else if self.state.queue_index==i-1{self.state.queue_index+=1}if let Some(p)=self.player.as_mut(){p.command(&["playlist-move",&i.to_string(),&(i-1).to_string()]);}self.changed();}
                            if ui.small_button("Remove").clicked(){self.state.queue.remove(i);if i<self.state.queue_index{self.state.queue_index-=1}self.state.queue_index=self.state.queue_index.min(self.state.queue.len().saturating_sub(1));if let Some(p)=self.player.as_mut(){p.command(&["playlist-remove",&i.to_string()]);}self.changed();}
                        });
                    });});
                }}});
            }else if let Some(m)=self.current(){
                ui.vertical_centered(|ui|{
                ui.label(RichText::new(&m.title).size(18.).strong());ui.label(RichText::new(m.description()).small().weak());ui.add_space(18.);
                if m.lyrics.trim().is_empty(){
                    ui.label(RichText::new("Every song has a story.").size(19.).strong());ui.label("Add a .lrc or .txt file beside your music, import lyrics, or search when you are online.");
                    if ui.button("Import lyrics file").clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("Lyrics",&["lrc","txt"]).pick_file(){match std::fs::read_to_string(path){Ok(text)=>{let mut edit=m.clone();edit.lyrics=text;edit.locked=true;self.save_metadata(edit)},Err(e)=>self.notify(e.to_string(),true)}}}
                    if self.remote.is_none()&&ui.button("Search lyrics online").clicked(){self.lookup_lyrics(&m);}
                }else{
                    let rows=library::lyrics(&m.lyrics);let position=self.player.as_ref().map(|p|p.snapshot.position).unwrap_or(0.)+self.lyrics_offset;
                    let current=rows.iter().rposition(|(t,_)|*t>=0.&&*t<=position);
                    egui::ScrollArea::vertical().max_height(ui.available_height()-70.).show(ui,|ui|{for (i,(time,text)) in rows.iter().enumerate(){let active=current==Some(i);let response=ui.add(egui::Label::new(RichText::new(text).size(if active{22.}else{18.}).color(if active{self.accent()}else{ui.visuals().weak_text_color()})).sense(egui::Sense::click()));if response.clicked()&&*time>=0.{if let Some(p)=self.player.as_mut(){p.seek((*time-self.lyrics_offset).max(0.))}}if active&&!self.player.as_ref().is_some_and(|p|p.snapshot.paused){response.scroll_to_me(Some(egui::Align::Center));}ui.add_space(10.);}});
                    ui.add_space(12.);ui.horizontal(|ui|{ui.label(RichText::new("Timing offset").small().weak());ui.add(egui::DragValue::new(&mut self.lyrics_offset).speed(0.1).suffix(" s"));});
                }
                });
            }else{ui.label(RichText::new("Lyrics live here.").size(21.).strong());ui.label("Play something to see its lyrics. Plain text and synchronized LRC are supported.");}
        });
    }
    fn lookup_lyrics(&mut self, m: &Media) {
        if m.artist.is_empty() {
            self.notify(
                "Add the artist in Edit information before searching lyrics",
                true,
            );
            return;
        }
        let m = m.clone();
        let tx = self.tx.clone();
        self.busy += 1;
        self.notify("Searching LRCLIB for matching lyrics…", false);
        std::thread::spawn(move || {
            let result = (|| -> Result<_> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(15))
                    .user_agent("SSMP/0.1 (Slulisee's Simple Media Player)")
                    .build()?;
                let v: serde_json::Value = client
                    .get("https://lrclib.net/api/get")
                    .query(&[
                        ("artist_name", m.artist.as_str()),
                        ("track_name", m.title.as_str()),
                        ("album_name", m.album.as_str()),
                        ("duration", &m.duration.round().to_string()),
                    ])
                    .send()?
                    .error_for_status()?
                    .json()?;
                let text = v["syncedLyrics"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| v["plainLyrics"].as_str())
                    .context("No matching lyrics are available")?;
                Ok((m.id, text.to_string()))
            })();
            let _ = tx.send(Event::Lyrics(
                result.map_err(|e| format!("Lyrics lookup: {e}")),
            ));
        });
    }
    fn hero(&mut self, ui: &mut egui::Ui) {
        let accent = self.accent();
        egui::Frame::new()
            .fill(ui.visuals().selection.bg_fill)
            .inner_margin(20.)
            .corner_radius(15.)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.;
                    ui.label(
                        RichText::new("A LITTLE LESS NOISE. A LOT MORE YOU.")
                            .size(10.)
                            .color(accent),
                    );
                    ui.add_space(4.);
                    ui.label(
                        RichText::new("Your collection.\nYour rules.")
                            .size(32.)
                            .strong(),
                    );
                    ui.add_space(4.);
                    ui.label(
                        "Music, videos, and all the things you love. Right where you left them.",
                    );
                    ui.add_space(8.);
                    centered_row(ui, 310., |ui| {
                        if ui
                            .add_sized(
                                [150., 36.],
                                egui::Button::new(RichText::new("+  Add your music").strong())
                                    .fill(accent),
                            )
                            .clicked()
                        {
                            self.add_folder();
                        }
                        if ui
                            .add_sized([150., 36.], egui::Button::new("Import a playlist"))
                            .clicked()
                        {
                            self.playlist_import();
                        }
                    });
                });
            });
        ui.add_space(16.);
        let audio = self.library().iter().filter(|m| m.kind == "audio").count();
        let video = self.library().len() - audio;
        let stats = [
            (audio.to_string(), "TRACKS"),
            (video.to_string(), "VIDEOS"),
            (self.state.playlists.len().to_string(), "PLAYLISTS"),
            (self.state.favorites.len().to_string(), "FAVORITES"),
        ];
        let gap = 12.;
        let width = (ui.available_width() - gap * 3.) / 4.;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (value, label) in stats {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 66.), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 10., ui.visuals().faint_bg_color);
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.top() + 23.),
                    egui::Align2::CENTER_CENTER,
                    value,
                    egui::FontId::proportional(24.),
                    ui.visuals().text_color(),
                );
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.top() + 47.),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(10.),
                    ui.visuals().weak_text_color(),
                );
            }
        });
        ui.add_space(18.);
    }
    fn cards(&mut self, ui: &mut egui::Ui, rows: &[Media]) {
        let mut groups = Vec::new();
        for m in rows {
            let key = if m.kind == "video" && !m.series.is_empty() {
                m.series.clone()
            } else if m.album.is_empty() {
                m.title.clone()
            } else {
                format!("{} / {}", m.artist, m.album)
            };
            if !groups.iter().any(|(k, _): &(String, Media)| k == &key) {
                groups.push((key, m.clone()));
            }
        }
        let count = if self.page == Page::Home {
            4
        } else {
            groups.len()
        };
        let columns = (((ui.available_width() + 16.) / 180.).floor() as usize)
            .clamp(1, 4)
            .min(groups.len().max(1));
        let max_size = if self.page == Page::Home { 180. } else { 210. };
        let width =
            ((ui.available_width() - 16. * (columns - 1) as f32) / columns as f32).min(max_size);
        let visible: Vec<_> = groups.iter().take(count).collect();
        for row in visible.chunks(columns) {
            let row_width = width * row.len() as f32 + 16. * row.len().saturating_sub(1) as f32;
            centered_row(ui, row_width, |ui| {
                ui.spacing_mut().item_spacing.x = 16.;
                for group in row {
                    let m = &group.1;
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, width + 48.),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.set_width(width);
                            let response = self.cover(ui, m, width);
                            if response.interact(egui::Sense::click()).clicked() {
                                self.play_album(m);
                            }
                            let title = if m.kind == "video" && !m.series.is_empty() {
                                &m.series
                            } else if m.album.is_empty() {
                                &m.title
                            } else {
                                &m.album
                            };
                            if ui
                                .add(
                                    egui::Label::new(RichText::new(title).strong())
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                self.play_album(m);
                            }
                            ui.add(
                                egui::Label::new(RichText::new(m.description()).small().weak())
                                    .truncate(),
                            );
                        },
                    );
                }
            });
            ui.add_space(10.);
        }
    }
    fn track_list(&mut self, ui: &mut egui::Ui, rows: &[Media], limit: Option<usize>) {
        if rows.is_empty() {
            ui.add_space(35.);
            ui.label(
                RichText::new(match self.page {
                    Page::Favorites => "Your favorites will feel at home here.",
                    Page::History => "Your listening story starts here.",
                    Page::Offline => "Your local files and completed downloads appear here.",
                    _ => "A fresh start for your collection.",
                })
                .size(24.)
                .strong(),
            );
            ui.add_space(10.);
            ui.label("Add files or a folder, or connect to your own SSMP server.");
            return;
        }
        ui.horizontal(|ui| {
            ui.add_space(45.);
            ui.label(RichText::new("TITLE").size(10.).weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(70.);
                ui.label(RichText::new("DURATION").size(10.).weak());
            });
        });
        ui.separator();
        let count = limit.unwrap_or(rows.len()).min(rows.len());
        let height = if self.state.preferences.compact {
            49.
        } else {
            65.
        };
        let selected = self.current().map(|m| m.id).unwrap_or_default();
        for (i, m) in rows.iter().take(count).enumerate() {
            ui.push_id((i, &m.id), |ui| {
                let active = selected == m.id;
                let fill = if active {
                    ui.visuals().selection.bg_fill
                } else if i % 2 == 0 {
                    ui.visuals().faint_bg_color
                } else {
                    Color32::TRANSPARENT
                };
                let response = egui::Frame::new()
                    .fill(fill)
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .corner_radius(7.)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(height - 10.);
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [28., 28.],
                                    egui::Button::new(if active { "||" } else { ">" }).frame(false),
                                )
                                .clicked()
                            {
                                if active {
                                    if let Some(p) = self.player.as_mut() {
                                        p.toggle()
                                    }
                                } else {
                                    self.play(&m.id)
                                }
                            }
                            self.cover(
                                ui,
                                m,
                                if self.state.preferences.compact {
                                    33.
                                } else {
                                    44.
                                },
                            );
                            let width = (ui.available_width() - 145.).max(140.);
                            ui.allocate_ui_with_layout(
                                Vec2::new(width, height - 10.),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_min_width(width);
                                    let title = RichText::new(&m.title).strong().color(if active {
                                        self.accent()
                                    } else {
                                        ui.visuals().text_color()
                                    });
                                    let r = ui.add(
                                        egui::Label::new(title)
                                            .truncate()
                                            .sense(egui::Sense::click()),
                                    );
                                    if r.double_clicked() {
                                        self.play(&m.id);
                                    }
                                    r.on_hover_text(&m.path);
                                    ui.label(
                                        RichText::new(format!(
                                            "{}{}",
                                            m.description(),
                                            if m.missing {
                                                " · file unavailable"
                                            } else {
                                                ""
                                            }
                                        ))
                                        .small()
                                        .weak(),
                                    );
                                },
                            );
                            ui.label(
                                RichText::new(duration(m.duration))
                                    .small()
                                    .monospace()
                                    .weak(),
                            );
                            if ui
                                .small_button(if self.state.favorites.contains(&m.id) {
                                    "♥"
                                } else {
                                    "+"
                                })
                                .on_hover_text("Toggle favorite")
                                .clicked()
                            {
                                self.state.favorite(&m.id);
                                self.changed();
                            }
                            ui.menu_button("…", |ui| {
                                self.media_menu(ui, m);
                            });
                        });
                    })
                    .response;
                if response.double_clicked() {
                    self.play(&m.id);
                }
                response.context_menu(|ui| self.media_menu(ui, m));
            });
        }
    }
    fn media_menu(&mut self, ui: &mut egui::Ui, m: &Media) {
        if ui.button("Play now").clicked() {
            self.play(&m.id);
            ui.close();
        }
        if ui.button("Play next").clicked() {
            self.enqueue(&m.id, true);
            ui.close();
        }
        if ui.button("Add to queue").clicked() {
            self.enqueue(&m.id, false);
            ui.close();
        }
        if ui.button("Edit information").clicked() {
            self.edit = Some(m.clone());
            ui.close();
        }
        ui.menu_button("Add to playlist", |ui| {
            let mut selected = None;
            for p in &self.state.playlists {
                if ui.button(&p.name).clicked() {
                    selected = Some(p.id.clone());
                }
            }
            if let Some(id) = selected {
                if let Some(p) = self.state.playlists.iter_mut().find(|p| p.id == id) {
                    p.items.push(m.id.clone());
                    self.changed();
                }
                ui.close();
            }
        });
        if self.remote.is_some() && ui.button("Keep offline").clicked() {
            self.download(&m.id);
            ui.close();
        }
        if self.page == Page::Playlists && ui.button("Remove from this playlist").clicked() {
            if let Some(p) = self
                .selected_playlist
                .as_ref()
                .and_then(|id| self.state.playlists.iter_mut().find(|p| &p.id == id))
            {
                p.items.retain(|id| id != &m.id);
                self.changed();
            }
            ui.close();
        }
        if m.kind == "video"
            && ui
                .button(if self.state.watched.contains(&m.id) {
                    "Mark unwatched"
                } else {
                    "Mark watched"
                })
                .clicked()
        {
            if self.state.watched.contains(&m.id) {
                self.state.watched.retain(|id| id != &m.id)
            } else {
                self.state.watched.push(m.id.clone())
            }
            self.changed();
            ui.close();
        }
    }
    fn now_playing(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let Some(m) = self.current() {
            if m.kind == "video" {
                if let Some(player) = self.player.as_mut() {
                    if player.texture_id.is_none() {
                        if let Some(tex) = player.texture {
                            player.texture_id = Some(frame.register_native_glow_texture(tex));
                        }
                    }
                    if let Some(gl) = frame.gl() {
                        player.render_gl(gl);
                    }
                    if let Some(id) = player.texture_id {
                        let height = (ui.available_width() * 9. / 16.)
                            .min((ui.clip_rect().height() - 205.).max(180.));
                        let width = height * 16. / 9.;
                        centered_row(ui, width, |ui| {
                            ui.add(
                                egui::Image::new((id, Vec2::new(width, height)))
                                    .fit_to_exact_size(Vec2::new(width, height)),
                            );
                        });
                    }
                }
                ui.add_space(12.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(&m.title).size(23.).strong());
                    ui.label(m.description());
                });
                ui.add_space(10.);
                let tracks = self
                    .player
                    .as_ref()
                    .map(|p| p.snapshot.tracks.clone())
                    .unwrap_or_default();
                centered_column(ui, 570., |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let old = self.audio_track;
                        egui::ComboBox::from_id_salt("audio-track")
                            .selected_text(if self.audio_track < 0 {
                                "Audio: automatic".into()
                            } else {
                                format!("Audio {}", self.audio_track)
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.audio_track, -1, "Automatic");
                                for t in tracks.iter().filter(|t| t.kind == "audio") {
                                    ui.selectable_value(
                                        &mut self.audio_track,
                                        t.id,
                                        format!("{} {} {}", t.id, t.language, t.title),
                                    );
                                }
                            });
                        if self.audio_track != old {
                            if let Some(p) = self.player.as_mut() {
                                let value = if self.audio_track < 0 {
                                    "auto".into()
                                } else {
                                    self.audio_track.to_string()
                                };
                                p.set("aid", &value);
                            }
                        }
                        let old = self.primary_sub;
                        egui::ComboBox::from_id_salt("primary-sub")
                            .selected_text(if self.primary_sub < 0 {
                                "Subtitles: automatic".into()
                            } else if self.primary_sub == 0 {
                                "Subtitles: off".into()
                            } else {
                                format!("Subtitles {}", self.primary_sub)
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.primary_sub, -1, "Automatic");
                                ui.selectable_value(&mut self.primary_sub, 0, "Off");
                                for t in tracks.iter().filter(|t| t.kind == "sub") {
                                    ui.selectable_value(
                                        &mut self.primary_sub,
                                        t.id,
                                        format!("{} {} {}", t.id, t.language, t.title),
                                    );
                                }
                            });
                        if old != self.primary_sub {
                            if let Some(p) = self.player.as_mut() {
                                let value = if self.primary_sub < 0 {
                                    "auto".into()
                                } else if self.primary_sub == 0 {
                                    "no".into()
                                } else {
                                    self.primary_sub.to_string()
                                };
                                p.set("sid", &value);
                            }
                        }
                        let old = self.secondary_sub;
                        egui::ComboBox::from_id_salt("secondary-sub")
                            .selected_text(if self.secondary_sub == 0 {
                                "Second language: off".into()
                            } else {
                                format!("Second language {}", self.secondary_sub)
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.secondary_sub, 0, "Off");
                                for t in tracks.iter().filter(|t| t.kind == "sub") {
                                    ui.selectable_value(
                                        &mut self.secondary_sub,
                                        t.id,
                                        format!("{} {} {}", t.id, t.language, t.title),
                                    );
                                }
                            });
                        if old != self.secondary_sub {
                            if let Some(p) = self.player.as_mut() {
                                let value = if self.secondary_sub == 0 {
                                    "no".into()
                                } else {
                                    self.secondary_sub.to_string()
                                };
                                p.set("secondary-sid", &value);
                            }
                        }
                        if ui.button("Add subtitle file").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Subtitles", &["srt", "ass", "ssa", "vtt", "sub"])
                                .pick_file()
                            {
                                if let Some(p) = self.player.as_mut() {
                                    p.command(&["sub-add", &path.to_string_lossy(), "select"]);
                                }
                            }
                        }
                    });
                });
                centered_column(ui, 410., |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Subtitle delay");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.sub_offset)
                                    .speed(0.1)
                                    .suffix(" s"),
                            )
                            .changed()
                        {
                            if let Some(p) = self.player.as_mut() {
                                p.set("sub-delay", &self.sub_offset.to_string());
                            }
                        }
                        ui.label("Subtitle size");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.sub_scale)
                                    .speed(0.05)
                                    .range(0.5..=3.),
                            )
                            .changed()
                        {
                            if let Some(p) = self.player.as_mut() {
                                p.set("sub-scale", &self.sub_scale.to_string());
                            }
                        }
                        if ui.button("Fullscreen").clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                                    !ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false)),
                                ));
                        }
                    });
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(25.);
                    self.cover(ui, &m, ui.available_width().min(340.));
                    ui.add_space(22.);
                    ui.label(RichText::new(&m.title).size(30.).strong());
                    ui.label(RichText::new(m.description()).size(18.).weak());
                    ui.add_space(12.);
                    ui.label(RichText::new(&m.album).weak());
                    ui.add_space(15.);
                    if ui.button("Show lyrics").clicked() {
                        self.state.preferences.show_lyrics = true;
                        self.state.preferences.show_queue = false;
                        self.changed();
                    }
                });
            }
        } else {
            ui.label("Choose something from your library to start playing.");
        }
    }
    fn playlist_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_playlist)
                    .hint_text("Name your new playlist")
                    .desired_width(220.),
            );
            if ui.button("Create playlist").clicked() && !self.new_playlist.trim().is_empty() {
                let id = ssmp::token();
                self.state.playlists.push(Playlist {
                    id: id.clone(),
                    name: self.new_playlist.trim().into(),
                    ..Default::default()
                });
                self.new_playlist.clear();
                self.selected_playlist = Some(id);
                self.changed();
            }
            if ui.button("Import playlist…").clicked() {
                self.playlist_import();
            }
        });
        ui.add_space(15.);
        let lists = self.state.playlists.clone();
        ui.horizontal_wrapped(|ui| {
            for p in &lists {
                if ui
                    .selectable_label(
                        self.selected_playlist.as_deref() == Some(&p.id),
                        format!("{}  ({})", p.name, p.items.len()),
                    )
                    .clicked()
                {
                    self.selected_playlist = Some(p.id.clone());
                }
            }
        });
        ui.add_space(18.);
        if let Some(p) = self
            .selected_playlist
            .as_ref()
            .and_then(|id| self.state.playlists.iter().find(|p| &p.id == id))
            .cloned()
        {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&p.name).size(25.).strong());
                if ui.button("Play playlist").clicked() {
                    if !self.state.preferences.resume_music || self.state.queue != p.items {
                        self.state.queue_index = 0;
                    }
                    self.state.queue = p.items.clone();
                    self.resume_queue();
                }
                if self.remote.is_none() && ui.button("Export M3U8").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(format!("{}.m3u8", p.name))
                        .save_file()
                    {
                        match library::export_playlist(&self.store, &p, &path) {
                            Ok(()) => self.notify("Playlist exported", false),
                            Err(e) => self.notify(e.to_string(), true),
                        }
                    }
                }
                if self.remote.is_some() && ui.button("Download playlist").clicked() {
                    self.download_playlist(&p.items);
                }
                if ui.button("Delete playlist").clicked() {
                    self.state.playlists.retain(|s| s.id != p.id);
                    self.selected_playlist = None;
                    self.changed();
                }
            });
            if !p.unresolved.is_empty() {
                egui::CollapsingHeader::new(format!(
                    "{} unresolved import entries",
                    p.unresolved.len()
                ))
                .show(ui, |ui| {
                    for entry in &p.unresolved {
                        ui.label(entry);
                    }
                });
            }
            ui.add_space(14.);
            let rows = self.filtered();
            self.track_list(ui, &rows, None);
        } else {
            ui.add_space(30.);
            ui.label(
                RichText::new("A playlist for every version of you.")
                    .size(24.)
                    .strong(),
            );
            ui.label("Create a playlist, then use the menu beside a track to add it.");
        }
    }
    fn server_page(&mut self, ui: &mut egui::Ui) {
        ui.label("Connect to your own collection, or share this computer with your other devices.");
        ui.add_space(20.);
        egui::Frame::group(ui.style())
            .inner_margin(20.)
            .corner_radius(12.)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new("Connect to an SSMP host").size(21.).strong());
                ui.add_space(10.);
                ui.label("Server address");
                ui.add_sized(
                    [ui.available_width().min(420.), 36.],
                    egui::TextEdit::singleline(&mut self.host_url),
                );
                ui.label("Username");
                ui.add_sized(
                    [ui.available_width().min(420.), 36.],
                    egui::TextEdit::singleline(&mut self.username),
                );
                ui.label("Password");
                ui.add_sized(
                    [ui.available_width().min(420.), 36.],
                    egui::TextEdit::singleline(&mut self.password).password(true),
                );
                ui.add_space(10.);
                if ui.button("Connect & load library").clicked() {
                    let url = self.host_url.clone();
                    let user = self.username.clone();
                    let password = std::mem::take(&mut self.password);
                    let tx = self.tx.clone();
                    self.busy += 1;
                    std::thread::spawn(move || {
                        let r = (|| -> Result<_> {
                            let remote = Remote::login(&url, &user, &password)?;
                            let library = remote.library()?;
                            let state = remote.state()?;
                            Ok((remote, library, state))
                        })();
                        let _ = tx.send(Event::Connected(r.map_err(|e| format!("{e:#}"))));
                    });
                }
                if let Some(remote) = self.remote.clone() {
                    ui.label(
                        RichText::new(format!(
                            "Connected as {}{}",
                            remote.connection.username,
                            if remote.connection.admin {
                                " · owner"
                            } else {
                                ""
                            }
                        ))
                        .color(self.accent()),
                    );
                    if ui.button("Refresh from server").clicked() {
                        let tx = self.tx.clone();
                        self.busy += 1;
                        std::thread::spawn(move || {
                            let r = (|| -> Result<_> {
                                let rows = remote.library()?;
                                let state = remote.state()?;
                                Ok((remote, rows, state))
                            })();
                            let _ = tx.send(Event::Connected(r.map_err(|e| format!("{e:#}"))));
                        });
                    }
                }
            });
        ui.add_space(22.);
        egui::Frame::group(ui.style()).inner_margin(20.).corner_radius(12.).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Host this library").size(21.).strong());ui.label("The host runs as a separate process. Your media stays on this computer.");ui.add_space(12.);
            ui.label("Owner username");ui.add_sized([ui.available_width().min(420.),36.],egui::TextEdit::singleline(&mut self.host_user));
            ui.label("Owner password");ui.add_sized([ui.available_width().min(420.),36.],egui::TextEdit::singleline(&mut self.host_password).password(true).hint_text("At least 10 characters"));
            if ui.button("Create owner account").clicked(){let root=self.store.root.clone();let name=self.host_user.clone();let password=std::mem::take(&mut self.host_password);let tx=self.tx.clone();self.busy+=1;std::thread::spawn(move||{let r=(||->Result<String>{Store::open(&root)?.add_user(&name,&password,true)?;Ok(format!("Owner account '{name}' created"))})();let _=tx.send(Event::Message(r.map_err(|e|e.to_string())));});}
            ui.add_space(14.);ui.horizontal(|ui|{ui.label("Listen address");ui.text_edit_singleline(&mut self.host_bind);});ui.label(RichText::new("127.0.0.1:4534 is local only. Use 0.0.0.0:4534 to allow your LAN/Tailscale devices.").small().weak());
            ui.horizontal(|ui|{if ui.button("Start host").clicked(){let r=(||->Result<_>{let exe=std::env::current_exe()?;let log=std::fs::File::create(self.store.root.join("host.log"))?;let child=ssmp::quiet_command(exe).args(["--serve","--data"]).arg(&self.store.root).args(["--bind",&self.host_bind]).stdin(std::process::Stdio::null()).stdout(log.try_clone()?).stderr(log).spawn()?;Ok(child)})();match r{Ok(child)=>{self.host_child=Some(child);self.notify("Host process started. Connect above to verify it is reachable.",false)},Err(e)=>self.notify(format!("{e:#}"),true)}}
                if self.host_child.is_some()&&ui.button("Stop this host").clicked(){if let Some(mut child)=self.host_child.take(){let _=child.kill();let _=child.wait();}self.notify("Host stopped",false);}
            });
            if let Some(child)=self.host_child.as_mut(){if let Ok(Some(status))=child.try_wait(){self.notify(format!("Host exited ({status}); inspect host.log in the data folder"),true);self.host_child=None;}}
            ui.add_space(12.);ui.label(RichText::new("See docs/HOSTING.md for Linux, additional users, Tailscale, backups, and reverse-proxy setup.").small().weak());
        });
    }
    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let before = serde_json::to_string(&self.state.preferences).unwrap_or_default();
        ui.label("Useful defaults. Room to make everything feel like you.");
        ui.add_space(20.);
        ui.label(RichText::new("Appearance").size(21.).strong());
        ui.add_space(8.);
        ui.horizontal_wrapped(|ui| {
            ui.label("Theme");
            egui::ComboBox::from_id_salt("theme")
                .selected_text(&self.state.preferences.theme)
                .show_ui(ui, |ui| {
                    for name in ["Midnight", "Ocean", "OLED", "Daylight"] {
                        ui.selectable_value(&mut self.state.preferences.theme, name.into(), name);
                    }
                });
            ui.label("Accent");
            ui.color_edit_button_srgb(&mut self.state.preferences.accent);
        });
        ui.add(
            egui::Slider::new(&mut self.state.preferences.scale, 0.8..=1.4).text("Interface size"),
        );
        ui.checkbox(&mut self.state.preferences.compact, "Compact library rows");
        ui.checkbox(
            &mut self.state.preferences.show_lyrics,
            "Show the lyrics panel",
        );
        ui.checkbox(
            &mut self.state.preferences.show_queue,
            "Show the queue panel",
        );
        ui.add_space(20.);
        ui.label(
            RichText::new("Your listening, your choice")
                .size(21.)
                .strong(),
        );
        ui.add_space(8.);
        ui.checkbox(
            &mut self.state.preferences.history,
            "Remember listening history and play counts",
        );
        ui.checkbox(
            &mut self.state.preferences.recommendations,
            "Show personal top-song mixes from my listening history",
        );
        ui.checkbox(
            &mut self.state.preferences.resume_music,
            "Resume music and playlists from their saved position",
        );
        ui.checkbox(
            &mut self.state.preferences.resume_video,
            "Resume videos from their saved position",
        );
        ui.label(RichText::new("History, mixes, and resume are independent. Playback never starts automatically on launch.").small().weak());
        if ui.button("Clear listening history and counts").clicked() {
            self.state.clear_history();
            self.changed();
            self.notify("Listening history and counts cleared", false);
        }
        ui.add_space(20.);
        ui.label(RichText::new("Playback & devices").size(21.).strong());
        ui.add_space(8.);
        ui.checkbox(
            &mut self.state.preferences.normalize,
            "Use ReplayGain loudness information when available",
        );
        ui.add(
            egui::Slider::new(&mut self.state.preferences.speed, 0.5..=2.)
                .step_by(0.05)
                .text("Playback speed"),
        );
        ui.checkbox(
            &mut self.state.preferences.sync_settings,
            "Synchronize suitable preferences with my SSMP profile",
        );
        ui.label(RichText::new("File locations and credentials stay on this device. Server profile edits use conflict detection.").small().weak());
        ui.add_space(20.);
        ui.label(RichText::new("Library & SSYTDLP").size(21.).strong());
        ui.add_space(8.);
        ui.label("SSYTDLP inbox (use this path in its SSMP integration setting):");
        let mut inbox = self
            .store
            .root
            .join("imports")
            .to_string_lossy()
            .into_owned();
        ui.add(
            egui::TextEdit::singleline(&mut inbox)
                .desired_width(f32::INFINITY)
                .interactive(false),
        );
        ui.label(RichText::new("Only completed handoffs are imported. Failed manifests remain in imports/failed for inspection.").small().weak());
        ui.horizontal_wrapped(|ui| {
            if ui.button("Rescan saved folders").clicked() {
                let paths = self.state.roots.iter().map(PathBuf::from).collect();
                self.import_paths(paths);
            }
            if ui.button("Import series/season metadata…").clicked() {
                self.choose_manifest();
            }
        });
        for path in &self.state.roots {
            ui.label(RichText::new(path).small().weak());
        }
        ui.add_space(20.);
        ui.label(
            RichText::new("Portable preferences & backups")
                .size(21.)
                .strong(),
        );
        ui.add_space(8.);
        ui.horizontal_wrapped(|ui|{
            if ui.button("Export preferences").clicked(){if let Some(path)=rfd::FileDialog::new().set_file_name("SSMP-preferences.json").save_file(){match std::fs::write(path,serde_json::to_vec_pretty(&self.state.preferences).unwrap_or_default()){Ok(())=>self.notify("Preferences exported",false),Err(e)=>self.notify(e.to_string(),true)}}}
            if ui.button("Import preferences").clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("Preferences",&["json"]).pick_file(){match std::fs::read(path).map_err(anyhow::Error::from).and_then(|b|Ok(serde_json::from_slice::<Preferences>(&b)?)){Ok(mut p)=>{p.scale=p.scale.clamp(0.8,1.4);p.volume=p.volume.clamp(0.,100.);p.speed=p.speed.clamp(0.5,2.);self.state.preferences=p;self.changed();},Err(e)=>self.notify(e.to_string(),true)}}}
            if ui.button("Back up library database").clicked(){if let Some(path)=rfd::FileDialog::new().set_file_name("SSMP-backup.sqlite3").save_file(){match self.store.backup(&path){Ok(())=>self.notify("Library database backup saved; media files stay in their original folders",false),Err(e)=>self.notify(e.to_string(),true)}}}
            if ui.button("Reset preferences").clicked(){self.state.preferences=Preferences::default();self.changed();}
        });
        if serde_json::to_string(&self.state.preferences).unwrap_or_default() != before {
            self.apply_preferences(ui.ctx());
            if let Some(p) = self.player.as_mut() {
                p.set("speed", &self.state.preferences.speed.to_string());
                p.set(
                    "replaygain",
                    if self.state.preferences.normalize {
                        "track"
                    } else {
                        "no"
                    },
                );
            }
            self.changed();
        }
    }
    fn choose_manifest(&mut self) {
        if self.remote.is_some() {
            self.notify(
                "Import metadata on the host, or use Edit information as the owner",
                true,
            );
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SSMP series/season metadata", &["json"])
            .pick_file()
        {
            match std::fs::read(&path)
                .map_err(anyhow::Error::from)
                .and_then(|s| Ok(serde_json::from_slice::<ImportManifest>(&s)?))
            {
                Ok(manifest) => self.manifest = Some((path, manifest)),
                Err(e) => self.notify(e.to_string(), true),
            }
        }
    }
    fn save_metadata(&mut self, mut m: Media) {
        m.locked = true;
        if let Some(remote) = self.remote.clone() {
            if !remote.connection.admin {
                self.notify("Only the owner can edit shared metadata", true);
                return;
            }
            if let Some(old) = self.remote_rows.iter_mut().find(|old| old.id == m.id) {
                *old = m.clone();
            }
            let tx = self.tx.clone();
            self.busy += 1;
            std::thread::spawn(move || {
                let result = (|| -> Result<String> {
                    let r = reqwest::blocking::Client::builder()
                        .timeout(Duration::from_secs(15))
                        .build()?
                        .put(format!(
                            "{}/api/v1/metadata/{}",
                            remote.connection.url, m.id
                        ))
                        .bearer_auth(&remote.connection.token)
                        .json(&m)
                        .send()?;
                    r.error_for_status()?;
                    Ok("Shared metadata updated".into())
                })();
                let _ = tx.send(Event::Message(result.map_err(|e| e.to_string())));
            });
        } else {
            match self.store.put(&m) {
                Ok(()) => {
                    self.rows = self.store.media().unwrap_or_default();
                    self.notify(
                        "Information saved. Your edits are preserved on rescan.",
                        false,
                    )
                }
                Err(e) => self.notify(e.to_string(), true),
            }
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.show_welcome {
            egui::Window::new("Welcome to SSMP").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER,[0.,0.]).default_width(460.).show(ctx,|ui|{
                ui.label(RichText::new(ssmp::APP_NAME).size(22.).strong());ui.add_space(12.);ui.label(RichText::new("Make yourself at home.").size(30.).strong());ui.add_space(12.);ui.label("Your music and videos work locally. Connect a personal server whenever you want. There is no account to create for local playback.");ui.add_space(18.);
                ui.checkbox(&mut self.state.preferences.history,"Remember listening history on this device");ui.checkbox(&mut self.state.preferences.recommendations,"Build my personal top-song mixes");ui.label(RichText::new("You can change these independently in Settings. Nothing is sent to a cloud account.").small().weak());ui.add_space(20.);
                ui.horizontal(|ui|{if ui.button("Add my first folder").clicked(){self.show_welcome=false;self.state.onboarding=true;self.changed();self.add_folder();}if ui.button("Explore first").clicked(){self.show_welcome=false;self.state.onboarding=true;self.changed();}});
            });
        }
        if let Some(mut edit) = self.edit.take() {
            let mut open = true;
            let mut save = false;
            egui::Window::new("Edit information").open(&mut open).default_width(520.).collapsible(false).show(ctx,|ui|{
                egui::Grid::new("metadata-editor").spacing([16.,10.]).show(ui,|ui|{
                    for (label,value) in [("Title",&mut edit.title),("Artist",&mut edit.artist),("Album",&mut edit.album),("Series",&mut edit.series)]{ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(340.));ui.end_row();}
                    ui.label("Season");ui.add(egui::DragValue::new(&mut edit.season));ui.end_row();ui.label("Episode");ui.add(egui::DragValue::new(&mut edit.episode));ui.end_row();
                });ui.add_space(10.);ui.label("Lyrics (plain text or LRC)");ui.add(egui::TextEdit::multiline(&mut edit.lyrics).desired_rows(7).desired_width(f32::INFINITY));ui.add_space(10.);
                if self.remote.is_none(){ui.label(RichText::new(&edit.path).small().weak());if ui.button("Relink to another file…").clicked(){if let Some(path)=rfd::FileDialog::new().pick_file(){match library::probe(&path){Ok(fresh)=>{edit.path=fresh.path;edit.duration=fresh.duration;edit.size=fresh.size;edit.modified=fresh.modified;edit.fingerprint=fresh.fingerprint;edit.missing=false;},Err(e)=>self.notify(e.to_string(),true)}}}}
                ui.label(RichText::new("Edits are stored in the library database. Original file tags are preserved.").small().weak());if ui.button("Save information").clicked(){save=true;}
            });
            if save {
                self.save_metadata(edit)
            } else if open {
                self.edit = Some(edit)
            }
        }
        if let Some((path, manifest)) = self.manifest.take() {
            let mut open = true;
            let mut apply = false;
            egui::Window::new("Preview metadata import").open(&mut open).default_width(620.).show(ctx,|ui|{
                ui.label(RichText::new(format!("{} · season {}",manifest.series,manifest.season)).size(23.).strong());ui.label(format!("{} entries · manifest version {}",manifest.items.len(),manifest.version));
                egui::ScrollArea::vertical().max_height(350.).show(ui,|ui|{for m in &manifest.items{ui.label(format!("E{:02}  {}",m.episode,m.title));ui.label(RichText::new(&m.path).small().weak());ui.separator();}});
                ui.label("Only supplied fields are applied. Unavailable files are listed in the result. Your original files are preserved.");
                if ui.add_enabled(manifest.version==1,egui::Button::new("Apply this metadata")).clicked(){apply=true;}
            });
            if apply {
                let root = self.store.root.clone();
                let tx = self.tx.clone();
                self.busy += 1;
                std::thread::spawn(move || {
                    let r = (|| -> Result<_> {
                        library::import_manifest(&Store::open(&root)?, &path)
                    })();
                    let _ = tx.send(Event::Imported(r.map_err(|e| e.to_string())));
                });
            } else if open {
                self.manifest = Some((path, manifest));
            }
        }
    }
    fn central(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill).inner_margin(24.)).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(self.page.title()).size(27.).strong());
                ui.label(RichText::new(if self.remote.is_some() { "Your server. Your personal space." } else { "Stored by you. Played by you." }).small().weak());
            });
            ui.add_space(8.);
            centered_row(ui, 460., |ui| {
                let width = (ui.available_width() - 98.).max(80.);
                ui.add_sized([width, 36.], egui::TextEdit::singleline(&mut self.search).hint_text("Search your collection"));
                if ui.add_sized([88., 36.], egui::Button::new("+ Files")).clicked() { self.add_files(); }
            });
            ui.add_space(18.);
            if !self.engine_error.is_empty() {
                egui::Frame::group(ui.style()).show(ui, |ui| { ui.colored_label(Color32::from_rgb(240,150,145), &self.engine_error); });
                ui.add_space(10.);
            }
            egui::ScrollArea::vertical().id_salt("main-scroll").auto_shrink([false,false]).show(ui, |ui| {
                let width = match self.page { Page::Settings | Page::Server => 680., _ => 1200. };
                centered_column(ui, width, |ui| {
                match self.page{
                    Page::Home=>{self.hero(ui);let mut rows=self.filtered();rows.sort_by_key(|m|std::cmp::Reverse(m.added));if !rows.is_empty(){ui.vertical_centered(|ui|{ui.label(RichText::new("A little inspiration").size(21.).strong());});ui.add_space(12.);self.cards(ui,&rows);ui.add_space(25.);}
                        if self.state.preferences.recommendations&&!self.state.counts.is_empty(){ui.vertical_centered(|ui|{ui.label(RichText::new("Your top songs · updated from your listening").size(21.).strong());});let mut top=rows.clone();top.retain(|m|self.state.counts.contains_key(&m.id));top.sort_by_key(|m|std::cmp::Reverse(self.state.counts.get(&m.id).copied().unwrap_or(0)));self.track_list(ui,&top,Some(5));ui.add_space(20.);}
                        ui.vertical_centered(|ui|{ui.label(RichText::new("Recently added").size(21.).strong());});ui.add_space(10.);self.track_list(ui,&rows,Some(12));}
                    Page::Albums=>{let rows=self.filtered();self.cards(ui,&rows);}
                    Page::Playlists=>self.playlist_page(ui),Page::NowPlaying=>self.now_playing(ui,frame),Page::Server=>self.server_page(ui),Page::Settings=>self.settings_page(ui),
                    Page::Videos=>{ui.horizontal(|ui|{if ui.button("Import series / season information").clicked(){self.choose_manifest();}ui.label(RichText::new("Use each item's menu to edit information or watched status.").small().weak());});ui.add_space(16.);let rows=self.filtered();self.track_list(ui,&rows,None);}
                    Page::History=>{if !self.state.preferences.history{ui.label("Listening history is switched off. You can enable it in Settings.");ui.add_space(12.);}let rows=self.filtered();self.track_list(ui,&rows,None);}
                    _=>{let rows=self.filtered();self.track_list(ui,&rows,None);}
                }
                });
            });
        });
    }
    fn tick(&mut self) {
        let dt = self.last_tick.elapsed().as_secs_f64().min(0.5);
        self.last_tick = Instant::now();
        if let Some(player) = self.player.as_mut() {
            player.tick();
            let snapshot = player.snapshot.clone();
            if !snapshot.error.is_empty() {
                self.status = snapshot.error.clone();
                self.error = true;
            }
            if !snapshot.path.is_empty() && self.loaded_at.elapsed() > Duration::from_millis(300) {
                if snapshot.index < self.state.queue.len()
                    && snapshot.index != self.state.queue_index
                {
                    self.state.queue_index = snapshot.index;
                    self.dirty = true;
                }
                if let Some(id) = self.state.queue.get(self.state.queue_index).cloned() {
                    if id != self.last_track {
                        self.last_track = id.clone();
                        self.listen_seconds = 0.;
                        self.listen_start = snapshot.position;
                        self.listen_recorded = false;
                    }
                    if !snapshot.paused
                        && snapshot.position > self.listen_start
                        && snapshot.position - self.listen_start < 2.
                    {
                        self.listen_seconds += dt;
                    }
                    self.listen_start = snapshot.position;
                    if !self.listen_recorded
                        && self.listen_seconds >= (snapshot.duration * 0.5).clamp(1., 30.)
                    {
                        self.state.listen(&id);
                        self.listen_recorded = true;
                        self.dirty = true;
                    }
                    if self.last_position.elapsed() > Duration::from_secs(3) {
                        let is_video = self
                            .library()
                            .iter()
                            .find(|m| m.id == id)
                            .is_some_and(|m| m.kind == "video");
                        let remember = if is_video {
                            self.state.preferences.resume_video
                        } else {
                            self.state.preferences.resume_music
                        };
                        if remember {
                            self.state.positions.insert(id.clone(), snapshot.position);
                            self.dirty = true;
                        }
                        if is_video
                            && snapshot.duration > 0.
                            && snapshot.position / snapshot.duration > 0.95
                            && !self.state.watched.contains(&id)
                        {
                            self.state.watched.push(id);
                            self.dirty = true;
                        }
                        self.last_position = Instant::now();
                    }
                }
            }
        }
        if self.last_save.elapsed() > Duration::from_secs(1) {
            self.save();
        }
        if self.last_inbox.elapsed() > Duration::from_secs(3)
            && self.remote.is_none()
            && self.busy < 2
        {
            self.last_inbox = Instant::now();
            let has_files =
                std::fs::read_dir(self.store.root.join("imports")).is_ok_and(|entries| {
                    entries
                        .flatten()
                        .any(|e| e.path().extension().is_some_and(|s| s == "json"))
                });
            if has_files {
                let root = self.store.root.clone();
                let tx = self.tx.clone();
                self.busy += 1;
                std::thread::spawn(move || {
                    let result =
                        (|| -> Result<_> { library::consume_inbox(&Store::open(&root)?) })();
                    let _ = tx.send(Event::Imported(result.map_err(|e| e.to_string())));
                });
            }
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.process_events(ctx);
        self.tick();
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() {
            self.import_paths(dropped);
        }
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::Space) {
                    if let Some(p) = self.player.as_mut() {
                        if p.snapshot.path.is_empty() {
                            self.resume_queue();
                        } else {
                            p.toggle();
                        }
                    }
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    if let Some(p) = self.player.as_mut() {
                        p.seek(p.snapshot.position + 5.);
                    }
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    if let Some(p) = self.player.as_mut() {
                        p.seek(p.snapshot.position - 5.);
                    }
                }
                if i.key_pressed(egui::Key::Escape) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                }
            });
        }
        self.bottom(ctx);
        self.sidebar(ctx);
        self.side_panel(ctx);
        self.central(ctx, frame);
        self.dialogs(ctx);
        if self.smoke {
            if self.started.elapsed() > Duration::from_secs(3) && !self.screenshot_sent {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.screenshot_sent = true;
            }
            let shots = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| {
                        if let egui::Event::Screenshot { image, .. } = e {
                            Some(image.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            });
            for shot in shots {
                let path = std::env::var_os("SSMP_SCREENSHOT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.store.root.join("screenshot.png"));
                let bytes: Vec<u8> = shot.pixels.iter().flat_map(|p| p.to_array()).collect();
                let _ = image::save_buffer(
                    &path,
                    &bytes,
                    shot.size[0] as u32,
                    shot.size[1] as u32,
                    image::ColorType::Rgba8,
                );
                let proof = serde_json::json!({"page":self.smoke_page,"items":self.rows.len(),"engine_available":self.player.is_some(),"engine_error":self.engine_error,"position":self.player.as_ref().map(|p|p.snapshot.position),"duration":self.player.as_ref().map(|p|p.snapshot.duration),"rendered_frames":self.player.as_ref().map(|p|p.frames),"player_error":self.player.as_ref().map(|p|p.snapshot.error.clone()),"expected_path":self.smoke_expected,"actual_path":self.player.as_ref().map(|p|p.snapshot.path.clone()),"secondary_subtitle":self.player.as_ref().map(|p|p.get("secondary-sid"))});
                let _ = std::fs::write(
                    path.with_extension("json"),
                    serde_json::to_vec_pretty(&proof).unwrap_or_default(),
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if self.started.elapsed() > Duration::from_secs(15) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        ctx.request_repaint_after(Duration::from_millis(
            if self
                .player
                .as_ref()
                .is_some_and(|p| !p.snapshot.paused && !p.snapshot.path.is_empty())
            {
                16
            } else {
                150
            },
        ));
    }
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.save();
        let _ = self.store.save_state(&self.profile_key, &self.state);
        if let Some(p) = self.player.as_mut() {
            p.close(gl);
        }
    }
}
