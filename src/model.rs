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
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "Midnight".into(),
            accent: [171, 145, 255],
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
    pub revision: u64,
    pub roots: Vec<String>,
    pub onboarding: bool,
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
