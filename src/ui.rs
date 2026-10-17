use anyhow::{Context, Result};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use ssmp::{library, model::*, player::Player, server::Remote, store::Store};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

mod i18n;
mod icons;
mod music_ui;
mod software_window;
mod themes;

type TaskResult<T> = std::result::Result<T, String>;
enum Event {
    Imported(TaskResult<ImportReport>),
    Playlist(TaskResult<Playlist>),
    Connected(TaskResult<(Remote, Vec<Media>, UserState)>),
    Saved(TaskResult<UserState>),
    Downloaded(TaskResult<Media>),
    Message(TaskResult<String>),
    Lyrics(TaskResult<(String, String)>),
    Enriched(TaskResult<Vec<Media>>),
    Watched(TaskResult<(ImportReport, u64)>),
}
#[derive(Clone, Debug, PartialEq)]
enum Page {
    Home,
    Music,
    Albums,
    Videos,
    Artists,
    Artist,
    Album,
    Stats,
    Lyrics,
    Favorites,
    History,
    Playlists,
    Offline,
    NowPlaying,
    Server,
    Settings,
}
impl Page {
    fn title(&self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Music => "Music",
            Self::Albums => "Albums",
            Self::Videos => "Music videos",
            Self::Artists => "Artists",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::Stats => "Your listening",
            Self::Lyrics => "Lyrics",
            Self::Favorites => "Favorites",
            Self::History => "Listening history",
            Self::Playlists => "Playlists",
            Self::Offline => "Available offline",
            Self::NowPlaying => "Now playing",
            Self::Server => "Your server",
            Self::Settings => "Settings",
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
    let rendering = std::env::var("SSMP_RENDERER").unwrap_or_default();
    let viewport = egui::ViewportBuilder::default()
        .with_icon(icon)
        .with_title(format!("SSMP — {}", ssmp::APP_NAME))
        .with_inner_size(size)
        .with_min_inner_size([1000., 650.]);
    if rendering.eq_ignore_ascii_case("software") || cfg!(windows) && rendering.is_empty() {
        return software_window::run(root, smoke, smoke_page, viewport);
    }
    let directx = cfg!(windows)
        && !std::env::var("SSMP_RENDERER").is_ok_and(|v| v.eq_ignore_ascii_case("opengl"));
    let mut gpu_setup = eframe::egui_wgpu::WgpuSetupCreateNew::default();
    if directx {
        gpu_setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: if directx {
            eframe::Renderer::Wgpu
        } else {
            eframe::Renderer::Glow
        },
        // Windowed OpenGL swap synchronization can stall the Windows compositor
        // on mixed refresh-rate/VRR and virtual-display configurations.
        vsync: !cfg!(windows),
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            present_mode: eframe::wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: Some(1),
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(gpu_setup),
            ..Default::default()
        },
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
    primary_sub: i64,
    secondary_sub: i64,
    last_track: String,
    listen_clock: ssmp::music::ListeningClock,
    listen_recorded: bool,
    listen_seconds: f64,
    last_tick: Instant,
    thumbnails: HashMap<String, egui::TextureHandle>,
    smoke: bool,
    smoke_page: Option<String>,
    screenshot_sent: bool,
    show_welcome: bool,
    smoke_expected: String,
    qa_last_trace: Instant,
    linked_video_requested: bool,

    selected_artist: Option<String>,
    selected_album: Option<(String, String)>,
    sort_key: String,
    sort_desc: bool,
    search_field: String,
    genre_filter: String,
    playlist_create: bool,
    playlist_seed: Vec<String>,
    playlist_add: bool,
    playlist_edit: bool,
    playlist_delete: bool,
    playlist_query: String,
    seek_preview: Option<f64>,
    lyrics_fullscreen: bool,
    last_lyric: Option<usize>,
    lyric_manual_scroll: Instant,
    devices: Vec<(String, String)>,
    last_devices: Instant,
    last_watch: Instant,
    watch_signature: u64,
    watch_running: bool,
    last_metadata: Instant,
    metadata_running: bool,
    metadata_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    eof_handled: bool,
    queue_back: Vec<usize>,
    backdrop: Option<(String, egui::TextureHandle)>,
    video_frame: Option<egui::TextureHandle>,
    filtered_cache: std::cell::RefCell<Option<(String, std::sync::Arc<Vec<Media>>)>>,
    genre_cache: std::cell::RefCell<Option<std::sync::Arc<std::collections::BTreeSet<String>>>>,
}
impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        root: PathBuf,
        smoke: bool,
        smoke_page: Option<String>,
    ) -> Result<Self> {
        Self::new_context(
            &cc.egui_ctx,
            cc.gl.as_deref(),
            cc.wgpu_render_state.as_ref(),
            root,
            smoke,
            smoke_page,
        )
    }
    fn new_context(
        ctx: &egui::Context,
        gl: Option<&eframe::glow::Context>,
        gpu: Option<&eframe::egui_wgpu::RenderState>,
        root: PathBuf,
        smoke: bool,
        smoke_page: Option<String>,
    ) -> Result<Self> {
        let store = Store::open(&root)?;
        let mut state = store.state("local")?;
        if state.preferences.design_version < 2 {
            state.preferences.theme = "Ocean".into();
            state.preferences.accent = [0, 140, 255];
            state.preferences.design_version = 2;
        }
        let rows = store.media()?;
        if let Some(gpu) = gpu {
            let adapter = gpu.adapter.get_info();
            let graphics = serde_json::json!({
                "backend": format!("{:?}", adapter.backend),
                "renderer": adapter.name,
                "driver": adapter.driver,
                "driver_info": adapter.driver_info,
                "vsync": false,
                "video_renderer": "libmpv software frames; no OpenGL context",
            });
            let _ = std::fs::write(
                store.root.join("graphics.json"),
                serde_json::to_vec_pretty(&graphics).unwrap_or_default(),
            );
        }
        if let Some(gl) = gl {
            use eframe::glow::{self, HasContext};
            let graphics = unsafe {
                serde_json::json!({
                    "backend": "OpenGL",
                    "vendor": gl.get_parameter_string(glow::VENDOR),
                    "renderer": gl.get_parameter_string(glow::RENDERER),
                    "version": gl.get_parameter_string(glow::VERSION),
                    "vsync": !cfg!(windows),
                })
            };
            let _ = std::fs::write(
                store.root.join("graphics.json"),
                serde_json::to_vec_pretty(&graphics).unwrap_or_default(),
            );
        }
        let (tx, rx) = mpsc::channel();
        if gl.is_none() && gpu.is_none() {
            let graphics = serde_json::json!({"backend":"Software", "renderer":"cached CPU raster / native software window", "gpu_context":false, "video_renderer":"libmpv software frames"});
            let _ = std::fs::write(
                store.root.join("graphics.json"),
                serde_json::to_vec_pretty(&graphics).unwrap_or_default(),
            );
        }
        let player = Player::new(gl, smoke);
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
            status: String::new(),
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
            primary_sub: -1,
            secondary_sub: 0,
            last_track: String::new(),
            listen_clock: Default::default(),
            listen_recorded: false,
            listen_seconds: 0.,
            last_tick: Instant::now(),
            thumbnails: HashMap::new(),
            smoke,
            smoke_page,
            screenshot_sent: false,
            show_welcome,
            smoke_expected: String::new(),
            qa_last_trace: Instant::now(),
            linked_video_requested: false,
            selected_artist: None,
            selected_album: None,
            sort_key: "Custom order".into(),
            sort_desc: false,
            search_field: "All fields".into(),
            genre_filter: String::new(),
            playlist_create: false,
            playlist_seed: vec![],
            playlist_add: false,
            playlist_edit: false,
            playlist_delete: false,
            playlist_query: String::new(),
            seek_preview: None,
            lyrics_fullscreen: false,
            last_lyric: None,
            lyric_manual_scroll: Instant::now() - Duration::from_secs(10),
            devices: vec![],
            last_devices: Instant::now() - Duration::from_secs(30),
            last_watch: Instant::now(),
            watch_signature: 0,
            watch_running: false,
            last_metadata: Instant::now() - Duration::from_secs(300),
            metadata_running: false,
            metadata_cancel: Default::default(),
            eof_handled: false,
            queue_back: vec![],
            backdrop: None,
            video_frame: None,
            filtered_cache: Default::default(),
            genre_cache: Default::default(),
        };
        app.selected_playlist = app.state.playlists.first().map(|p| p.id.clone());
        app.apply_preferences(ctx);
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
                    ..Default::default()
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
                    "artists" => Page::Artists,
                    "artist" => Page::Artist,
                    "albums" => Page::Albums,
                    "album" => Page::Album,
                    "stats" => Page::Stats,
                    "lyrics" => Page::Lyrics,
                    "music" => Page::Music,
                    "edit" => Page::Music,
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
        if smoke {
            if let Ok(language) = std::env::var("SSMP_SMOKE_LANGUAGE") {
                app.state.preferences.language = language;
            }
            if let Ok(theme) = std::env::var("SSMP_SMOKE_THEME") {
                app.set_theme(&theme, ctx);
            }
            app.selected_artist = app
                .rows
                .iter()
                .find(|m| !m.artist.is_empty())
                .map(|m| m.artist.clone());
            app.selected_album = app
                .rows
                .iter()
                .find(|m| m.kind == "audio")
                .map(ssmp::music::album_key);
            if app.smoke_page.as_deref() == Some("edit") {
                app.edit = app.current();
            }
        }
        app.apply_audio_preferences();
        if !smoke && app.rows.iter().any(|m| m.tag_version < 2) {
            let paths = app
                .rows
                .iter()
                .filter(|m| m.tag_version < 2)
                .map(|m| PathBuf::from(&m.path))
                .filter(|p| p.is_file())
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                app.import_paths(paths);
            }
        }
        Ok(app)
    }
    fn notify(&mut self, message: impl Into<String>, error: bool) {
        self.status = i18n::translate(&self.state.preferences.language, &message.into());
        self.error = error;
    }
    fn changed(&mut self) {
        self.filtered_cache.get_mut().take();
        self.genre_cache.get_mut().take();
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
            .set_title(self.t("Choose a music folder"))
            .pick_folder()
        {
            self.import_paths(vec![path]);
        }
    }
    fn add_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title(self.t("Add music or music videos"))
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
        self.video_frame = None;
        self.eof_handled = false;
        self.seek_preview = None;
        self.error = false;
        self.status.clear();
        self.linked_video_requested = false;
        self.last_track.clear();
        let selected = self.state.queue.get(self.state.queue_index).cloned();
        let valid: std::collections::HashSet<_> =
            self.library().iter().map(|m| m.id.clone()).collect();
        self.state.queue.retain(|id| valid.contains(id));
        self.state.queue_index = selected
            .as_ref()
            .and_then(|id| self.state.queue.iter().position(|item| item == id))
            .unwrap_or(
                self.state
                    .queue_index
                    .min(self.state.queue.len().saturating_sub(1)),
            );
        let chosen = self
            .state
            .queue
            .get(self.state.queue_index)
            .and_then(|id| self.library().iter().find(|m| &m.id == id))
            .cloned();
        if let Some(media) = chosen {
            let cached = self
                .store
                .get(&media.id)
                .ok()
                .flatten()
                .filter(|m| std::path::Path::new(&m.path).is_file());
            let path = if let Some(media) = cached {
                media.path
            } else if let Some(remote) = &self.remote {
                remote.playback_url(&media.id)
            } else {
                media.path
            };
            let token = self.remote.as_ref().map(|r| r.connection.token.clone());
            if let Some(player) = self.player.as_mut() {
                player.load_selected(&path, self.state.queue_index, position, token.as_deref());
                player.set(
                    "vid",
                    if self.state.preferences.show_music_video {
                        "auto"
                    } else {
                        "no"
                    },
                );
            } else {
                self.notify(self.engine_error.clone(), true)
            }
            self.loaded_at = Instant::now();
            self.changed();
        }
    }
    fn play(&mut self, id: &str) {
        self.queue_back.clear();
        self.state.queue_playlist = if self.page == Page::Playlists {
            self.selected_playlist.clone()
        } else {
            None
        };
        let kind = self
            .library()
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.kind.clone())
            .unwrap_or_default();
        let selection: Vec<_> = self
            .filtered()
            .iter()
            .cloned()
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
        if is_video && self.state.preferences.show_music_video {
            self.page = Page::NowPlaying;
        }
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
        self.changed();
        self.notify("Added to your queue", false);
    }
    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            self.busy = self.busy.saturating_sub(1);
            if let Event::Watched(Ok((_, signature))) = &event {
                if *signature == self.watch_signature {
                    self.watch_running = false;
                    continue;
                }
            }
            self.filtered_cache.get_mut().take();
            self.genre_cache.get_mut().take();
            match event {
                Event::Watched(result) => {
                    self.watch_running = false;
                    match result {
                        Ok((r, signature)) => {
                            self.watch_signature = signature;
                            self.rows = self.store.media().unwrap_or_default();
                            if r.added > 0 {
                                self.notify(
                                    format!("{} · {}", self.t("New tracks"), r.added),
                                    false,
                                );
                            }
                        }
                        Err(e) => self.notify(e, true),
                    }
                }
                Event::Enriched(result) => {
                    self.metadata_running = false;
                    match result {
                        Ok(rows) => {
                            let mut n = 0;
                            for enriched in rows {
                                if let Ok(Some(current)) = self.store.get(&enriched.id) {
                                    if !current.locked
                                        && self.state.preferences.online_metadata
                                        && self.remote.is_none()
                                    {
                                        let merged =
                                            ssmp::metadata::merge_missing(&current, &enriched);
                                        let _ = self.store.put(&merged);
                                        n += 1;
                                    }
                                }
                            }
                            self.rows = self.store.media().unwrap_or_default();
                            if n > 0 {
                                self.notify(format!("{} · {n}", self.t("Metadata checked")), false);
                                self.last_metadata = Instant::now() - Duration::from_secs(300);
                            }
                        }
                        Err(e) => self.notify(
                            format!("{}: {e}", self.t("Metadata lookup unavailable")),
                            false,
                        ),
                    }
                }
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
                        self.queue_back.clear();
                        self.metadata_cancel
                            .store(true, std::sync::atomic::Ordering::Relaxed);
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
        if self.state.queue.is_empty() {
            self.state.queue_playlist = None;
            self.state.queue = self
                .library()
                .iter()
                .filter(|m| m.kind == "audio")
                .map(|m| m.id.clone())
                .collect();
            self.state.queue_index = 0;
        }
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
                if self.thumbnails.len() >= 256 {
                    if let Some(key) = self.thumbnails.keys().next().cloned() {
                        self.thumbnails.remove(&key);
                    }
                }
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
        let palette = themes::palette(&self.state.preferences.theme);
        ui.painter()
            .rect_filled(rect, 8., themes::mix(palette.panel, self.accent(), 0.15));
        icons::draw(
            ui.painter(),
            rect.shrink(size * 0.22),
            icons::Icon::Album,
            self.accent(),
        );
        response
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
                    .user_agent("SSMP/0.2 (https://github.com/Slulisee/SSMP)")
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
    fn save_metadata(&mut self, mut m: Media) {
        self.filtered_cache.get_mut().take();
        self.genre_cache.get_mut().take();
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
}
impl App {
    fn update_window(&mut self, ctx: &egui::Context, frame: Option<&mut eframe::Frame>) {
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
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.lyrics_fullscreen = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::L)) {
                self.page = Page::Lyrics;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
                self.page = Page::Lyrics;
                self.lyrics_fullscreen = !self.lyrics_fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.lyrics_fullscreen));
            }
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
            });
        }
        self.bottom(ctx);
        if !self.lyrics_fullscreen {
            self.sidebar(ctx);
        }
        if self.state.preferences.show_queue && self.page != Page::Lyrics {
            self.queue_panel(ctx);
        }
        self.central(ctx, frame);
        self.music_dialogs(ctx);
        if self.smoke {
            let keep = std::env::var("SSMP_SMOKE_KEEP_OPEN").is_ok_and(|v| v == "1");
            if keep && self.qa_last_trace.elapsed() > Duration::from_millis(100) {
                self.qa_last_trace = Instant::now();
                let proof = serde_json::json!({"at":ssmp::now(),"position":self.player.as_ref().map(|p|p.snapshot.position),"paused":self.player.as_ref().map(|p|p.snapshot.paused),"path":self.player.as_ref().map(|p|p.snapshot.path.clone()),"error":self.player.as_ref().map(|p|p.snapshot.error.clone()),"seek_commands":self.player.as_ref().map(|p|p.seek_commands),"shuffle":self.state.preferences.shuffle,"queue_index":self.state.queue_index,"playlist_count":self.state.playlists.len(),"page":self.page.title(),"video_visible":self.state.preferences.show_music_video,"preview":self.seek_preview});
                let _ = std::fs::write(
                    self.store.root.join("interaction.json"),
                    serde_json::to_vec_pretty(&proof).unwrap(),
                );
            }
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
                if !keep {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            if !keep && self.started.elapsed() > Duration::from_secs(15) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        let playing = self
            .player
            .as_ref()
            .is_some_and(|p| !p.snapshot.paused && !p.snapshot.path.is_empty());
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        let interval = if minimized {
            1000
        } else if playing {
            if self.page == Page::NowPlaying {
                33
            } else if self.state.preferences.animations {
                40
            } else {
                100
            }
        } else {
            500
        };
        ctx.request_repaint_after(Duration::from_millis(interval));
    }
    fn close_window(&mut self, gl: Option<&eframe::glow::Context>) {
        self.save();
        let _ = self.store.save_state(&self.profile_key, &self.state);
        if let Some(p) = self.player.as_mut() {
            p.close(gl);
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.update_window(ctx, Some(frame));
    }
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.close_window(gl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_edits_immediately_refresh_cached_search_results() {
        let directory = tempfile::tempdir().unwrap();
        let media = Media {
            id: "editable-track".into(),
            path: "fixture.wav".into(),
            title: "Before edit".into(),
            kind: "audio".into(),
            tag_version: 2,
            ..Default::default()
        };
        Store::open(directory.path()).unwrap().put(&media).unwrap();
        let ctx = egui::Context::default();
        let mut app =
            App::new_context(&ctx, None, None, directory.path().into(), false, None).unwrap();
        app.page = Page::Music;
        assert_eq!(app.filtered()[0].title, "Before edit");
        let mut edited = media;
        edited.title = "After edit".into();
        app.save_metadata(edited);
        assert_eq!(app.filtered()[0].title, "After edit");
        app.search = "before".into();
        assert!(app.filtered().is_empty());
        app.search = "after".into();
        assert_eq!(app.filtered().len(), 1);
        app.close_window(None);
    }
}
