//! Music navigation, sorting, listening accounting and shuffle rules shared by UI and tests.
use crate::model::{Media, UserState};
use rand::Rng;

pub const PREVIOUS_THRESHOLD: f64 = 3.;
pub fn album_key(m: &Media) -> (String, String) {
    (
        m.album.trim().to_lowercase(),
        if m.album_artist.trim().is_empty() {
            m.artist.trim()
        } else {
            m.album_artist.trim()
        }
        .to_lowercase(),
    )
}
#[derive(Default)]
pub struct ListeningClock {
    position: f64,
    pending: f64,
    discontinuity: u64,
}
impl ListeningClock {
    pub fn reset(&mut self, position: f64, discontinuity: u64) {
        self.position = position;
        self.pending = 0.;
        self.discontinuity = discontinuity;
    }
    pub fn sample(
        &mut self,
        dt: f64,
        position: f64,
        speed: f64,
        paused: bool,
        discontinuity: u64,
    ) -> f64 {
        if paused || !dt.is_finite() || dt <= 0. || dt > 1. || discontinuity != self.discontinuity {
            self.reset(position, discontinuity);
            return 0.;
        }
        self.pending = (self.pending + dt).min(1.);
        if (position - self.position).abs() < 0.0001 {
            return 0.;
        }
        let seconds = heard_seconds(self.pending, self.position, position, speed, false);
        self.position = position;
        self.pending = 0.;
        seconds
    }
}
pub fn previous_restarts(position: f64) -> bool {
    position > PREVIOUS_THRESHOLD
}
/// Enhanced LRC supplies real word timestamps; plain LRC remains line-synced.
pub fn lyric_words(text: &str) -> Vec<(Option<f64>, String)> {
    let mut result = Vec::new();
    let mut rest = text;
    let mut time = None;
    while let Some(start) = rest.find('<') {
        if start > 0 {
            result.push((time, rest[..start].to_string()));
        }
        let tag = &rest[start + 1..];
        let Some(end) = tag.find('>') else {
            result.push((time, rest[start..].into()));
            return result;
        };
        let parsed = tag[..end]
            .split_once(':')
            .and_then(|(m, s)| Some(m.parse::<f64>().ok()? * 60. + s.parse::<f64>().ok()?));
        if let Some(at) = parsed {
            time = Some(at);
        } else {
            result.push((time, format!("<{}>", &tag[..end])));
        }
        rest = &tag[end + 1..];
    }
    if !rest.is_empty() {
        result.push((time, rest.into()));
    }
    result
}

/// Do not count time while buffering, seeking, paused or suspended.
pub fn heard_seconds(dt: f64, before: f64, after: f64, speed: f64, paused: bool) -> f64 {
    let advance = after - before;
    if paused
        || !dt.is_finite()
        || dt <= 0.
        || dt > 1.
        || !advance.is_finite()
        || advance <= 0.
        || advance > dt * speed.clamp(0.25, 4.) + 0.4
    {
        return 0.;
    }
    dt.min(advance / speed.clamp(0.25, 4.))
}

pub fn shuffled_next<R: Rng + ?Sized>(
    state: &UserState,
    rows: &[Media],
    rng: &mut R,
) -> Option<usize> {
    let candidates: Vec<(usize, u32)> = state
        .queue
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != state.queue_index)
        .map(|(i, id)| {
            let favored = state.preferences.weight_favorite_artists
                && rows.iter().find(|m| &m.id == id).is_some_and(|m| {
                    state
                        .favorite_artists
                        .contains(&m.artist.trim().to_lowercase())
                });
            (i, if favored { 3 } else { 1 })
        })
        .collect();
    let sum: u32 = candidates.iter().map(|(_, w)| w).sum();
    if sum == 0 {
        return None;
    }
    let mut pick = rng.gen_range(0..sum);
    for (index, weight) in candidates {
        if pick < weight {
            return Some(index);
        }
        pick -= weight;
    }
    None
}

pub fn sort_tracks(rows: &mut [Media], key: &str, descending: bool) {
    rows.sort_by(|a, b| {
        let order = match key {
            "Title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            "Artist" => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
            "Album" => a
                .album
                .to_lowercase()
                .cmp(&b.album.to_lowercase())
                .then(a.track.cmp(&b.track)),
            "Recently added" => b.added.cmp(&a.added),
            "Release date" => b.release_date.cmp(&a.release_date),
            "Duration" => a.duration.total_cmp(&b.duration),
            _ => std::cmp::Ordering::Equal,
        };
        if descending { order.reverse() } else { order }
    });
}

pub fn date(at: u64) -> String {
    if at == 0 {
        return "—".into();
    }
    // Civil date from Unix days (Gregorian calendar), independent of locale.
    let z = at as i64 / 86400 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn album_artist_groups_compilations_and_featured_artists() {
        let a = Media {
            album: "A collection".into(),
            artist: "First artist".into(),
            album_artist: "Various Artists".into(),
            ..Default::default()
        };
        let b = Media {
            album: "a collection ".into(),
            artist: "Second artist".into(),
            album_artist: "Various Artists".into(),
            ..Default::default()
        };
        assert_eq!(album_key(&a), album_key(&b));
    }
    #[test]
    fn playlist_time_requires_playing_that_playlist() {
        let mut s = UserState::default();
        s.preferences.history = true;
        s.playlists.push(crate::model::Playlist {
            id: "p".into(),
            items: vec!["a".into()],
            ..Default::default()
        });
        s.record_seconds("a", 1., 100);
        assert_eq!(s.playlists[0].listening_seconds, 0.);
        s.queue_playlist = Some("p".into());
        s.record_seconds("a", 1., 101);
        assert_eq!(s.playlists[0].listening_seconds, 1.);
        s.record_seconds("b", 1., 102);
        assert_eq!(s.playlists[0].listening_seconds, 1.);
        s.clear_history();
        assert_eq!(s.playlists[0].listening_seconds, 0.);
    }
    #[test]
    fn enhanced_lyrics_use_real_timing_and_keep_text() {
        let words = lyric_words("<00:01.20>Hello <00:01.80>world");
        assert_eq!(
            words,
            vec![(Some(1.2), "Hello ".into()), (Some(1.8), "world".into())]
        );
        assert_eq!(
            lyric_words("plain <3 text"),
            vec![(None, "plain ".into()), (None, "<3 text".into())]
        );
    }
    #[test]
    fn quantized_audio_clock_does_not_undercount() {
        let mut clock = ListeningClock::default();
        let mut seconds = 0.;
        for frame in 1..=100 {
            let position = (frame / 4) as f64 * 0.064;
            seconds += clock.sample(0.016, position, 1., false, 0);
        }
        assert!((seconds - 1.6).abs() < 0.01, "{seconds}");
        assert_eq!(clock.sample(0.016, 60., 1., false, 1), 0.);
        assert_eq!(clock.sample(0.5, 60., 1., true, 1), 0.);
        assert_eq!(clock.sample(0.5, 60.5, 1., false, 1), 0.5);
    }
    use rand::{SeedableRng, rngs::StdRng};
    #[test]
    fn seek_and_pause_do_not_inflate_time() {
        assert_eq!(heard_seconds(0.5, 10., 90., 1., false), 0.);
        assert_eq!(heard_seconds(0.5, 90., 10., 1., false), 0.);
        assert_eq!(heard_seconds(0.5, 10., 10., 1., false), 0.);
        assert_eq!(heard_seconds(0.5, 10., 10.5, 1., true), 0.);
        assert_eq!(heard_seconds(5., 10., 15., 1., false), 0.);
        assert_eq!(heard_seconds(0.5, 10., 11., 2., false), 0.5);
    }
    #[test]
    fn previous_threshold_and_shuffle_rules() {
        assert!(!previous_restarts(2.99));
        assert!(previous_restarts(3.01));
        let mut s = UserState::default();
        s.queue = vec!["a".into(), "b".into(), "c".into()];
        s.favorite_artists.push("fav".into());
        let rows = vec![Media {
            id: "b".into(),
            artist: "Fav".into(),
            ..Default::default()
        }];
        let mut rng = StdRng::seed_from_u64(42);
        let mut counts = [0; 3];
        for _ in 0..4000 {
            counts[shuffled_next(&s, &rows, &mut rng).unwrap()] += 1;
        }
        assert_eq!(counts[0], 0);
        assert!(counts[1] > counts[2] * 2);
        s.preferences.weight_favorite_artists = false;
        counts = [0; 3];
        for _ in 0..4000 {
            counts[shuffled_next(&s, &rows, &mut rng).unwrap()] += 1;
        }
        assert!((counts[1] as i32 - counts[2] as i32).abs() < 300);
    }
    #[test]
    fn accounting_is_opt_in_and_clearable() {
        let mut s = UserState::default();
        s.record_seconds("a", 1., 100000);
        assert!(s.listening_seconds.is_empty());
        s.preferences.history = true;
        s.record_seconds("a", 0.5, 100000);
        s.record_seconds("a", f64::NAN, 100000);
        assert_eq!(s.listening_seconds["a"], 0.5);
        assert_eq!(s.listening_days[&1], 0.5);
        s.clear_history();
        assert!(s.listening_seconds.is_empty());
        assert_eq!(s.stats_since, 0);
    }
    #[test]
    fn old_state_and_playlist_migrate_without_loss() {
        let s: UserState = serde_json::from_str(
            r#"{"playlists":[{"id":"p","name":"old","items":["x"]}],"counts":{"x":42}}"#,
        )
        .unwrap();
        assert_eq!(s.counts["x"], 42);
        assert!(s.listening_seconds.is_empty());
        let mut p = s.playlists[0].clone();
        p.add("x", 100);
        p.add("y", 101);
        assert_eq!(p.items, vec!["x", "y"]);
        assert_eq!(p.added["y"], 101);
        assert_eq!(date(86400), "1970-01-02");
    }
}
