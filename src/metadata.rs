//! Optional, conservative metadata enrichment. Original media files are never written.
use crate::{model::Media, store::Store};
use anyhow::{Context, Result};
use serde_json::Value;
use std::{path::Path, time::Duration};

fn normalized(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}
fn genres(value: &Value) -> String {
    let mut list = value["genres"].as_array().cloned().unwrap_or_default();
    list.retain(|v| v["count"].as_i64().unwrap_or(1) > 0);
    list.sort_by_key(|v| std::cmp::Reverse(v["count"].as_i64().unwrap_or(0)));
    list.iter()
        .take(3)
        .filter_map(|v| v["name"].as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn apply_recordings(m: &mut Media, value: &Value) -> Option<String> {
    let recordings = value["recordings"].as_array()?;
    for r in recordings {
        if r["score"].as_u64().unwrap_or(0) < 95
            || normalized(&text(&r["title"])) != normalized(&m.title)
        {
            continue;
        }
        let credits = r["artist-credit"].as_array().cloned().unwrap_or_default();
        let artists = credits
            .iter()
            .map(|a| text(&a["artist"]["name"]))
            .collect::<Vec<_>>()
            .join(" ");
        let artist_matches = normalized(&artists) == normalized(&m.artist)
            || credits
                .iter()
                .any(|a| normalized(&text(&a["artist"]["name"])) == normalized(&m.artist));
        if m.artist.trim().is_empty() || !artist_matches {
            continue;
        }
        let length = r["length"].as_f64().unwrap_or(0.) / 1000.;
        if length > 0. && m.duration > 0. && (length - m.duration).abs() > 5. {
            continue;
        }
        if m.musicbrainz_id.is_empty() {
            m.musicbrainz_id = text(&r["id"]);
        }
        if let Some(a) = credits.first() {
            if m.artist_id.is_empty() {
                m.artist_id = text(&a["artist"]["id"]);
            }
            if m.artist_country.is_empty() {
                m.artist_country = text(&a["artist"]["country"]);
            }
        }
        let releases = r["releases"].as_array().cloned().unwrap_or_default();
        let release = releases
            .iter()
            .find(|v| normalized(&text(&v["title"])) == normalized(&m.album))
            .or_else(|| {
                if m.album.is_empty() {
                    releases.first()
                } else {
                    None
                }
            });
        let mut release_id = None;
        if let Some(v) = release {
            if m.album.is_empty() {
                m.album = text(&v["title"]);
            }
            if m.release_date.is_empty() {
                m.release_date = text(&v["date"]);
            }
            release_id = v["id"].as_str().map(String::from);
        }
        if m.genre.is_empty() {
            m.genre = genres(r);
        }
        if m.album_artist.is_empty() {
            m.album_artist = m.artist.clone();
        }
        m.metadata_source = "MusicBrainz".into();
        return release_id;
    }
    None
}

fn read_json(
    client: &reqwest::blocking::Client,
    url: &str,
    query: &[(&str, String)],
    cache: &Path,
) -> Result<Value> {
    if let Ok(bytes) = std::fs::read(cache) {
        if let Ok(v) = serde_json::from_slice(&bytes) {
            return Ok(v);
        }
    }
    if url.starts_with("https://musicbrainz.org/") {
        static LAST: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(at) = *last {
            let elapsed = at.elapsed();
            if elapsed < Duration::from_millis(1100) {
                std::thread::sleep(Duration::from_millis(1100) - elapsed);
            }
        }
        *last = Some(std::time::Instant::now());
    }
    let response = client.get(url).query(query).send()?.error_for_status()?;
    let mut bytes = Vec::new();
    response.take(4 * 1024 * 1024).read_to_end(&mut bytes)?;
    let value: Value = serde_json::from_slice(&bytes)?;
    if let Some(parent) = cache.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cache, serde_json::to_vec(&value)?)?;
    Ok(value)
}
use std::io::Read;

pub fn enrich(store: &Store, media: &Media) -> Result<Media> {
    let mut m = media.clone();
    if m.locked {
        return Ok(m);
    }
    anyhow::ensure!(
        !m.artist.trim().is_empty(),
        "Add the artist in Edit information before searching online"
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("SSMP/0.3.0 (https://github.com/Slulisee/SSMP)")
        .build()?;
    let cache = store.root.join("metadata-cache");
    std::fs::create_dir_all(&cache)?;
    let escape = |s: &str| s.replace(['"', '\\'], " ");
    let query = format!(
        "recording:\"{}\" AND artist:\"{}\"",
        escape(&m.title),
        escape(&m.artist)
    );
    use sha2::{Digest, Sha256};
    let key = hex::encode(Sha256::digest(query.as_bytes()));
    let value = read_json(
        &client,
        "https://musicbrainz.org/ws/2/recording/",
        &[
            ("query", query),
            ("fmt", "json".into()),
            ("limit", "5".into()),
        ],
        &cache.join(format!("{key}.json")),
    )?;
    let mut release = apply_recordings(&mut m, &value);
    // Search folksonomy tags are not genres. Resolve MusicBrainz's curated
    // genre field on the matched recording; keep missing genres honest.
    if m.musicbrainz_id.len() == 36
        && m.musicbrainz_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        if let Ok(v) = read_json(
            &client,
            &format!(
                "https://musicbrainz.org/ws/2/recording/{}",
                m.musicbrainz_id
            ),
            &[("fmt", "json".into()), ("inc", "genres+releases".into())],
            &cache.join(format!("recording-genres-{}.json", m.musicbrainz_id)),
        ) {
            if m.genre.is_empty() {
                m.genre = genres(&v);
            }
            let mut candidates = v["releases"].as_array().cloned().unwrap_or_default();
            candidates.retain(|v| normalized(&text(&v["title"])) == normalized(&m.album));
            candidates.sort_by_key(|v| {
                (
                    text(&v["status"]) != "Official",
                    text(&v["date"]).is_empty(),
                    text(&v["date"]),
                )
            });
            if let Some(v) = candidates.first() {
                if release.is_none() {
                    release = v["id"].as_str().map(String::from);
                }
                if m.release_date.is_empty() {
                    m.release_date = text(&v["date"]);
                }
            }
        }
    }
    if m.artist_id.len() == 36
        && m.artist_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
        && (m.artist_country.is_empty() || m.genre.is_empty())
    {
        if let Ok(v) = read_json(
            &client,
            &format!("https://musicbrainz.org/ws/2/artist/{}", m.artist_id),
            &[("fmt", "json".into()), ("inc", "genres".into())],
            &cache.join(format!("artist-genres-{}.json", m.artist_id)),
        ) {
            m.artist_country = text(&v["country"]);
            if m.genre.is_empty() {
                m.genre = genres(&v);
            }
        }
    }
    if m.artwork.is_empty() {
        if let Some(id) = release
            .filter(|id| id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
        {
            let cached = store.root.join("artwork").join(format!("mb-{id}.jpg"));
            if cached.is_file() {
                m.artwork = cached.to_string_lossy().into_owned();
            } else if let Ok(r) = client
                .get(format!(
                    "https://coverartarchive.org/release/{id}/front-250"
                ))
                .send()
            {
                if r.status().is_success()
                    && r.headers()
                        .get("content-type")
                        .and_then(|h| h.to_str().ok())
                        .is_some_and(|s| s.starts_with("image/"))
                {
                    let mut bytes = Vec::new();
                    r.take(8 * 1024 * 1024).read_to_end(&mut bytes)?;
                    if !bytes.is_empty() && bytes.len() < 8 * 1024 * 1024 {
                        let path = store.root.join("artwork").join(format!("mb-{id}.jpg"));
                        std::fs::write(&path, bytes)?;
                        m.artwork = path.to_string_lossy().into_owned();
                    }
                }
            }
        }
    }
    if m.lyrics.is_empty() {
        let query = [
            ("track_name", m.title.clone()),
            ("artist_name", m.artist.clone()),
            ("album_name", m.album.clone()),
            ("duration", m.duration.round().to_string()),
        ];
        if let Ok(v) = read_json(
            &client,
            "https://lrclib.net/api/get",
            &query,
            &cache.join(format!("lyrics-{}.json", m.id)),
        ) {
            m.lyrics = v["syncedLyrics"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or_else(|| v["plainLyrics"].as_str())
                .unwrap_or_default()
                .to_string();
        }
    }
    m.metadata_checked = crate::now();
    Ok(m)
}
/// Rebase a delayed lookup on the current library record so a watcher/relink or
/// manual edit performed during the request is never rolled back by its reply.
pub fn merge_missing(current: &Media, online: &Media) -> Media {
    if current.locked {
        return current.clone();
    }
    let mut m = current.clone();
    for (dst, src) in [
        (&mut m.album, &online.album),
        (&mut m.genre, &online.genre),
        (&mut m.release_date, &online.release_date),
        (&mut m.album_artist, &online.album_artist),
        (&mut m.artwork, &online.artwork),
        (&mut m.lyrics, &online.lyrics),
        (&mut m.musicbrainz_id, &online.musicbrainz_id),
        (&mut m.artist_id, &online.artist_id),
        (&mut m.artist_country, &online.artist_country),
    ] {
        if dst.is_empty() {
            *dst = src.clone();
        }
    }
    m.metadata_checked = online.metadata_checked;
    m.metadata_source = online.metadata_source.clone();
    m
}

/// An explicit request may fill empty fields on a manually edited track, but
/// never replaces its nonempty values or clears its rescan protection.
pub fn merge_requested(current: &Media, online: &Media) -> Media {
    let mut base = current.clone();
    base.locked = false;
    let mut merged = merge_missing(&base, online);
    merged.locked = current.locked;
    merged
}

pub fn lyrics(track: &Media) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("SSMP/0.3.0 (https://github.com/Slulisee/SSMP)")
        .build()?;
    let response = client
        .get("https://lrclib.net/api/get")
        .query(&[
            ("track_name", track.title.clone()),
            ("artist_name", track.artist.clone()),
            ("album_name", track.album.clone()),
            ("duration", track.duration.round().to_string()),
        ])
        .send()?;
    if response.status().is_success() {
        let value: Value = response.json()?;
        if let Some(text) = lyrics_text(&value) {
            return Ok(text);
        }
    }
    // Local album names often differ from LRCLIB's release. Search without
    // that constraint, then validate identity and duration before accepting.
    let values: Value = client
        .get("https://lrclib.net/api/search")
        .query(&[("track_name", &track.title), ("artist_name", &track.artist)])
        .send()?
        .error_for_status()?
        .json()?;
    choose_lyrics(track, &values).context(
        "No matching lyrics found. Check the title and artist, or add a local lyrics file.",
    )
}

fn lyrics_text(value: &Value) -> Option<String> {
    ["syncedLyrics", "plainLyrics"]
        .into_iter()
        .filter_map(|k| value[k].as_str())
        .find(|s| !s.trim().is_empty())
        .map(String::from)
}
pub fn choose_lyrics(track: &Media, values: &Value) -> Option<String> {
    let mut candidates: Vec<_> = values
        .as_array()?
        .iter()
        .filter(|v| {
            normalized(&text(&v["trackName"])) == normalized(&track.title)
                && normalized(&text(&v["artistName"])) == normalized(&track.artist)
                && (track.duration <= 0.
                    || v["duration"]
                        .as_f64()
                        .is_some_and(|d| (d - track.duration).abs() <= 5.))
                && lyrics_text(v).is_some()
        })
        .collect();
    candidates.sort_by_key(|v| {
        (
            v["syncedLyrics"]
                .as_str()
                .is_none_or(|s| s.trim().is_empty()),
            normalized(&text(&v["albumName"])) != normalized(&track.album),
        )
    });
    candidates.first().and_then(|v| lyrics_text(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lyric_search_checks_identity_duration_and_prefers_synced() {
        let track = Media {
            title: "Track".into(),
            artist: "Artist".into(),
            duration: 180.,
            ..Default::default()
        };
        let values = serde_json::json!([
            {"trackName":"Track","artistName":"Wrong","duration":180.,"syncedLyrics":"wrong"},
            {"trackName":"Track","artistName":"Artist","duration":230.,"syncedLyrics":"wrong version"},
            {"trackName":"Track","artistName":"Artist","duration":180.,"plainLyrics":"plain"},
            {"trackName":"Track","artistName":"Artist","duration":181.,"syncedLyrics":"[00:01]right"}]);
        assert_eq!(
            choose_lyrics(&track, &values).as_deref(),
            Some("[00:01]right")
        );
        assert!(choose_lyrics(&track, &serde_json::json!([])).is_none());
    }
    #[test]
    fn manual_online_request_fills_only_missing_fields() {
        let current = Media {
            locked: true,
            album: "My album".into(),
            lyrics: "My lyrics".into(),
            ..Default::default()
        };
        let online = Media {
            genre: "pop".into(),
            album: "Other album".into(),
            lyrics: "Other lyrics".into(),
            ..Default::default()
        };
        let merged = merge_requested(&current, &online);
        assert!(merged.locked);
        assert_eq!(merged.genre, "pop");
        assert_eq!(merged.album, "My album");
        assert_eq!(merged.lyrics, "My lyrics");
    }
    #[test]
    fn decades_and_arbitrary_tags_are_not_genres() {
        assert_eq!(
            genres(&serde_json::json!({"tags":[{"name":"1980s","count":20}]})),
            ""
        );
        assert_eq!(
            genres(
                &serde_json::json!({"genres":[{"name":"rock","count":5},{"name":"pop","count":10},{"name":"noise","count":-1}]})
            ),
            "pop; rock"
        );
    }
    #[test]
    fn conservative_match_preserves_manual_values() {
        let value = serde_json::json!({"recordings":[{"score":100,"title":"Track","id":"rec","length":180000,"artist-credit":[{"artist":{"name":"Artist","id":"artist"}}],"releases":[{"id":"release","title":"Album","date":"2020-01-02"}],"tags":[{"name":"rock","count":10}]}]});
        let mut m = Media {
            title: "Track".into(),
            artist: "Artist".into(),
            album: "My album".into(),
            genre: "My genre".into(),
            duration: 180.,
            ..Default::default()
        };
        assert!(apply_recordings(&mut m, &value).is_none());
        assert_eq!(m.musicbrainz_id, "rec");
        assert_eq!(m.album, "My album");
        assert_eq!(m.genre, "My genre");
        assert!(m.release_date.is_empty());
        m.album.clear();
        assert!(apply_recordings(&mut m, &value).is_some());
        assert_eq!(m.release_date, "2020-01-02");
        let mut wrong = Media {
            title: "Track".into(),
            artist: "Someone else".into(),
            duration: 180.,
            ..Default::default()
        };
        assert!(apply_recordings(&mut wrong, &value).is_none());
        assert!(wrong.musicbrainz_id.is_empty());
        wrong.artist = "Artist".into();
        wrong.duration = 90.;
        assert!(apply_recordings(&mut wrong, &value).is_none());
    }
}
