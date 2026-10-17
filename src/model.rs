use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Media {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub kind: String,
    pub series: String,
    pub season: u32,
    pub episode: u32,
    pub duration: f64,
    pub size: u64,
    pub added: u64,
    pub modified: u64,
    pub lyrics: String,
    pub artwork: String,
    pub source: String,
    pub missing: bool,
    pub locked: bool,
    pub fingerprint: String,
    pub track: u32,
    pub genre: String,
    pub release_date: String,
    pub album_artist: String,
    pub musicbrainz_id: String,
    pub artist_id: String,
    pub artist_country: String,
    pub metadata_checked: u64,
    pub metadata_source: String,
    pub music_video: String,
    pub tag_version: u32,
}
impl Media {
    pub fn description(&self) -> String {
        if !self.series.is_empty() {
            format!("{} · S{:02} E{:02}", self.series, self.season, self.episode)
        } else if !self.artist.is_empty() {
            self.artist.clone()
        } else if self.kind == "video" {
            "Video".into()
        } else {
            "Unknown artist".into()
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: String,
    pub accent: [u8; 3],
    pub scale: f32,
    pub compact: bool,
    pub history: bool,
    pub recommendations: bool,
    pub resume_music: bool,
    pub resume_video: bool,
    pub sync_settings: bool,
    pub show_lyrics: bool,
    pub show_queue: bool,
    pub volume: f64,
    pub normalize: bool,
    pub speed: f64,
    pub language: String,
    pub animations: bool,
    pub online_metadata: bool,
    pub watch_folders: bool,
    pub audio_device: String,
    pub show_music_video: bool,
    pub shuffle: bool,
    pub weight_favorite_artists: bool,
    pub design_version: u32,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "Ocean".into(),
            accent: [0, 140, 255],
            scale: 1.,
            compact: false,
            history: false,
            recommendations: false,
            resume_music: false,
            resume_video: true,
            sync_settings: true,
            show_lyrics: true,
            show_queue: false,
            volume: 75.,
            normalize: false,
            speed: 1.,
            language: "en".into(),
            animations: true,
            online_metadata: false,
            watch_folders: false,
            audio_device: "auto".into(),
            show_music_video: true,
            shuffle: false,
            weight_favorite_artists: true,
            design_version: 0,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub items: Vec<String>,
    pub unresolved: Vec<String>,
    pub description: String,
    pub artwork: String,
    pub added: BTreeMap<String, u64>,
    pub listening_seconds: f64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Listen {
    pub id: String,
    pub at: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UserState {
    pub preferences: Preferences,
    pub favorites: Vec<String>,
    pub playlists: Vec<Playlist>,
    pub history: Vec<Listen>,
    pub counts: BTreeMap<String, u64>,
    pub positions: BTreeMap<String, f64>,
    pub watched: Vec<String>,
    pub queue: Vec<String>,
    pub queue_index: usize,
    pub queue_playlist: Option<String>,
    pub revision: u64,
    pub roots: Vec<String>,
    pub onboarding: bool,
    pub favorite_artists: Vec<String>,
    pub listening_seconds: BTreeMap<String, f64>,
    pub listening_days: BTreeMap<u64, f64>,
    pub listening_hours: BTreeMap<u64, f64>,
    pub stats_since: u64,
}
impl UserState {
    pub fn favorite(&mut self, id: &str) {
        if self.favorites.iter().any(|s| s == id) {
            self.favorites.retain(|s| s != id)
        } else {
            self.favorites.push(id.into())
        }
    }
    pub fn listen(&mut self, id: &str) {
        if self.preferences.history {
            self.history.push(Listen {
                id: id.into(),
                at: crate::now(),
            });
            if self.history.len() > 10000 {
                self.history.drain(..self.history.len() - 10000);
            }
            *self.counts.entry(id.into()).or_default() += 1;
        }
    }
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.counts.clear();
        self.listening_seconds.clear();
        self.listening_days.clear();
        self.listening_hours.clear();
        self.stats_since = 0;
        for playlist in &mut self.playlists {
            playlist.listening_seconds = 0.;
        }
    }
    /// Wall-clock listening time, never track position or seek distance.
    pub fn record_seconds(&mut self, id: &str, seconds: f64, at: u64) {
        if !self.preferences.history || !seconds.is_finite() || seconds <= 0. || seconds > 2. {
            return;
        }
        if self.stats_since == 0 {
            self.stats_since = at;
        }
        *self.listening_seconds.entry(id.into()).or_default() += seconds;
        *self.listening_days.entry(at / 86400).or_default() += seconds;
        *self.listening_hours.entry(at / 3600 % 24).or_default() += seconds;
        if let Some(playlist_id) = &self.queue_playlist {
            if let Some(p) = self
                .playlists
                .iter_mut()
                .find(|p| &p.id == playlist_id && p.items.iter().any(|v| v == id))
            {
                p.listening_seconds += seconds;
            }
        }
    }
    pub fn favorite_artist(&mut self, artist: &str) {
        let key = artist.trim().to_lowercase();
        if key.is_empty() {
            return;
        }
        if self.favorite_artists.contains(&key) {
            self.favorite_artists.retain(|a| a != &key);
        } else {
            self.favorite_artists.push(key);
        }
    }
}
impl Playlist {
    pub fn add(&mut self, id: &str, at: u64) {
        if !self.items.iter().any(|v| v == id) {
            self.items.push(id.into());
        }
        self.added.entry(id.into()).or_insert(at);
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Login {
    pub username: String,
    pub password: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub username: String,
    pub admin: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connection {
    pub url: String,
    pub token: String,
    pub username: String,
    pub admin: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportItem {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub series: String,
    pub season: u32,
    pub episode: u32,
    pub source: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportManifest {
    pub version: u32,
    #[serde(default)]
    pub series: String,
    #[serde(default)]
    pub season: u32,
    pub items: Vec<ImportItem>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImportReport {
    pub added: usize,
    pub updated: usize,
    pub errors: Vec<String>,
}
pub fn duration(seconds: f64) -> String {
    let s = seconds.max(0.) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}
