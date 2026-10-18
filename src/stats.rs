//! Calendar summaries use measured time. Legacy totals never become guessed ranks.
use crate::model::{Media, UserState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub fn calendar(at: u64) -> (u32, u32, u32) {
    let z = (at / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    ((y + i64::from(month <= 2)) as u32, month as u32, day as u32)
}
pub fn timestamp(year: u32, month: u32) -> u64 {
    if !(1970..=9998).contains(&year) || !(1..=12).contains(&month) {
        return 0;
    }
    let y = year as i64 - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = month as i64 + if month > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5;
    ((era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468).max(0) as u64) * 86400
}
pub fn month_key(at: u64) -> u32 {
    let (y, m, _) = calendar(at);
    y * 100 + m
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Period {
    #[default]
    AllTime,
    Month(u32, u32),
    Year(u32),
}
impl Period {
    pub fn title(self) -> String {
        match self {
            Self::AllTime => "All time".into(),
            Self::Month(y, m) => format!("{y}-{m:02}"),
            Self::Year(y) => y.to_string(),
        }
    }
    pub fn bounds(self) -> (u64, u64) {
        match self {
            Self::AllTime => (0, u64::MAX),
            Self::Month(y, m) => (
                timestamp(y, m),
                if m == 12 {
                    timestamp(y + 1, 1)
                } else {
                    timestamp(y, m + 1)
                },
            ),
            Self::Year(y) => (timestamp(y, 1), timestamp(y + 1, 1)),
        }
    }
    pub fn previous(self) -> Option<Self> {
        match self {
            Self::AllTime => None,
            Self::Year(y) => Some(Self::Year(y.saturating_sub(1))),
            Self::Month(y, 1) => Some(Self::Month(y.saturating_sub(1), 12)),
            Self::Month(y, m) => Some(Self::Month(y, m.saturating_sub(1))),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MonthStats {
    pub seconds: BTreeMap<String, f64>,
    pub plays: BTreeMap<String, u64>,
    pub hours: [f64; 24],
    pub playlists: BTreeMap<String, f64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Rank {
    pub id: String,
    pub name: String,
    pub seconds: f64,
    pub plays: u64,
}
fn ranks(map: BTreeMap<String, (String, f64, u64)>) -> Vec<Rank> {
    let mut result: Vec<_> = map
        .into_iter()
        .map(|(id, (name, seconds, plays))| Rank {
            id,
            name,
            seconds,
            plays,
        })
        .collect();
    result.sort_by(|a, b| {
        b.seconds
            .total_cmp(&a.seconds)
            .then_with(|| a.id.cmp(&b.id))
    });
    result
}
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub period: Period,
    pub title: String,
    pub timezone: &'static str,
    pub seconds: f64,
    pub detailed_seconds: f64,
    pub plays: u64,
    pub tracks: usize,
    pub artists: usize,
    pub albums: usize,
    pub genres: usize,
    pub active_days: usize,
    pub longest_streak: usize,
    pub new_tracks: usize,
    pub average_active_day: f64,
    pub average_track: f64,
    pub top_track_share: f64,
    pub genre_coverage: f64,
    pub change_percent: Option<f64>,
    pub busiest_day: Option<(u64, f64)>,
    pub hours: [f64; 24],
    pub weekdays: [f64; 7],
    pub days: BTreeMap<u64, f64>,
    pub months: BTreeMap<u32, f64>,
    pub top_tracks: Vec<Rank>,
    pub top_artists: Vec<Rank>,
    pub top_albums: Vec<Rank>,
    pub top_genres: Vec<Rank>,
    pub top_playlists: Vec<Rank>,
    pub top_decades: Vec<Rank>,
    pub detail_since: u64,
}
pub fn report(state: &UserState, rows: &[Media], period: Period) -> Report {
    let (start, end) = period.bounds();
    let days: BTreeMap<_, _> = state
        .listening_days
        .iter()
        .filter(|(d, s)| **d >= start / 86400 && **d < end / 86400 && **s > 0.)
        .map(|(d, s)| (*d, *s))
        .collect();
    let mut seconds: BTreeMap<String, f64> = BTreeMap::new();
    let mut plays: BTreeMap<String, u64> = BTreeMap::new();
    let mut hours = [0.; 24];
    let mut playlists = BTreeMap::<String, f64>::new();
    for (key, data) in &state.months {
        let at = timestamp(key / 100, key % 100);
        if at < start || at >= end {
            continue;
        }
        for (id, s) in &data.seconds {
            *seconds.entry(id.clone()).or_default() += s;
        }
        for (id, n) in &data.plays {
            *plays.entry(id.clone()).or_default() += n;
        }
        for (h, s) in data.hours.iter().enumerate() {
            hours[h] += s;
        }
        for (id, s) in &data.playlists {
            *playlists.entry(id.clone()).or_default() += s;
        }
    }
    if period == Period::AllTime {
        seconds = state.listening_seconds.clone();
        plays = state.counts.clone();
        for (h, s) in &state.listening_hours {
            if *h < 24 {
                hours[*h as usize] = *s;
            }
        }
        playlists = state
            .playlists
            .iter()
            .map(|p| (p.id.clone(), p.listening_seconds))
            .collect();
    }
    let detail: f64 = seconds.values().sum();
    let total = if period == Period::AllTime {
        detail
    } else {
        days.values().sum()
    };
    let mut weekdays = [0.; 7];
    let mut months = BTreeMap::<u32, f64>::new();
    let mut streak = 0;
    let mut longest = 0;
    let mut previous = None;
    for (day, s) in &days {
        weekdays[((day + 3) % 7) as usize] += s;
        *months.entry(month_key(day * 86400)).or_default() += s;
        streak = if previous.is_some_and(|d| d + 1 == *day) {
            streak + 1
        } else {
            1
        };
        longest = longest.max(streak);
        previous = Some(*day);
    }
    let lookup: BTreeMap<_, _> = rows.iter().map(|m| (m.id.as_str(), m)).collect();
    let mut songs = BTreeMap::new();
    let mut artists = BTreeMap::new();
    let mut albums = BTreeMap::new();
    let mut genres = BTreeMap::new();
    let mut decades = BTreeMap::new();
    let mut tagged = 0.;
    for (id, s) in &seconds {
        if *s <= 0. {
            continue;
        }
        let count = plays.get(id).copied().unwrap_or(0);
        let m = lookup.get(id.as_str());
        let title = m
            .map(|m| format!("{} · {}", m.title, m.artist))
            .unwrap_or_else(|| "Removed from library".into());
        songs.insert(id.clone(), (title, *s, count));
        if let Some(m) = m {
            for (map, key, name) in [
                (&mut artists, m.artist.to_lowercase(), m.artist.clone()),
                (
                    &mut albums,
                    format!("{}\0{}", m.album.to_lowercase(), m.artist.to_lowercase()),
                    m.album.clone(),
                ),
            ] {
                if !name.trim().is_empty() {
                    let value = map.entry(key).or_insert((name, 0., 0));
                    value.1 += s;
                    value.2 += count;
                }
            }
            let genre = m.genre.split(';').next().unwrap_or("").trim();
            if !genre.is_empty() {
                tagged += s;
                let v = genres
                    .entry(genre.to_lowercase())
                    .or_insert((genre.to_string(), 0., 0));
                v.1 += s;
                v.2 += count;
            }
            if let Some(year) = m
                .release_date
                .get(..4)
                .and_then(|y| y.parse::<u32>().ok())
                .filter(|y| (1800..=2200).contains(y))
            {
                let name = format!("{}s", year / 10 * 10);
                let v = decades.entry(name.clone()).or_insert((name, 0., 0));
                v.1 += s;
                v.2 += count;
            }
        }
    }
    let top_tracks = ranks(songs);
    let top_artists = ranks(artists);
    let top_albums = ranks(albums);
    let top_genres = ranks(genres);
    let playlist_names: BTreeMap<_, _> = state.playlists.iter().map(|p| (&p.id, &p.name)).collect();
    let top_playlists = ranks(
        playlists
            .into_iter()
            .filter(|(_, s)| *s > 0.)
            .map(|(id, s)| {
                let name = playlist_names
                    .get(&id)
                    .map(|s| (*s).clone())
                    .unwrap_or_else(|| "Removed playlist".into());
                (id, (name, s, 0))
            })
            .collect(),
    );
    let change = period
        .previous()
        .map(|p| {
            let (a, b) = p.bounds();
            state
                .listening_days
                .iter()
                .filter(|(d, _)| **d >= a / 86400 && **d < b / 86400)
                .map(|(_, s)| s)
                .sum::<f64>()
        })
        .filter(|s| *s > 0.)
        .map(|old| (total - old) / old * 100.);
    Report {
        period,
        title: period.title(),
        timezone: "UTC",
        seconds: total,
        detailed_seconds: detail,
        plays: plays.values().sum(),
        tracks: top_tracks.len(),
        artists: top_artists.len(),
        albums: top_albums.len(),
        genres: top_genres.len(),
        active_days: days.len(),
        longest_streak: longest,
        new_tracks: state
            .first_heard
            .values()
            .filter(|at| **at >= start && **at < end)
            .count(),
        average_active_day: total / days.len().max(1) as f64,
        average_track: detail / top_tracks.len().max(1) as f64,
        top_track_share: top_tracks
            .first()
            .map_or(0., |t| t.seconds / total.max(1.) * 100.),
        genre_coverage: tagged / total.max(1.) * 100.,
        change_percent: change,
        busiest_day: days
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(d, s)| (*d, *s)),
        hours,
        weekdays,
        days,
        months,
        top_tracks,
        top_artists,
        top_albums,
        top_genres,
        top_playlists,
        top_decades: ranks(decades),
        detail_since: state.stats_detail_since,
    }
}

pub fn export(state: &UserState, report: &Report) -> serde_json::Value {
    let seconds = if report.period == Period::AllTime {
        state.listening_seconds.clone()
    } else {
        report
            .top_tracks
            .iter()
            .map(|r| (r.id.clone(), r.seconds))
            .collect()
    };
    let plays = if report.period == Period::AllTime {
        state.counts.clone()
    } else {
        report
            .top_tracks
            .iter()
            .map(|r| (r.id.clone(), r.plays))
            .collect()
    };
    let hours: BTreeMap<_, _> = report
        .hours
        .iter()
        .enumerate()
        .map(|(h, s)| (h, *s))
        .collect();
    serde_json::json!({"schema_version":2,"period":report.period,"report":report,"measured_since":state.stats_since,"listening_seconds":seconds,"qualified_plays":plays,"days_utc":report.days,"hours_utc":hours})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leap_year_and_period_boundaries() {
        assert_eq!(calendar(0), (1970, 1, 1));
        assert_eq!((timestamp(2024, 3) - timestamp(2024, 2)) / 86400, 29);
        for y in [1970, 2000, 2024, 2026, 2100] {
            for m in 1..=12 {
                assert_eq!(calendar(timestamp(y, m)), (y, m, 1));
            }
        }
        assert_eq!(
            Period::Month(2026, 1).previous(),
            Some(Period::Month(2025, 12))
        );
    }
    #[test]
    fn summaries_keep_periods_and_legacy_measurements_honest() {
        let mut s = UserState::default();
        s.preferences.history = true;
        let jan = timestamp(2026, 1);
        let feb = timestamp(2026, 2);
        for _ in 0..10 {
            s.record_seconds("a", 1., jan + 3600);
            s.record_seconds("b", 2., feb + 7200);
        }
        let rows = vec![
            Media {
                id: "a".into(),
                title: "A".into(),
                artist: "Artist".into(),
                genre: "pop".into(),
                ..Default::default()
            },
            Media {
                id: "b".into(),
                title: "B".into(),
                artist: "Other".into(),
                ..Default::default()
            },
        ];
        let r = report(&s, &rows, Period::Month(2026, 1));
        assert_eq!(r.seconds, 10.);
        assert_eq!(r.top_tracks[0].id, "a");
        assert_eq!(r.genre_coverage, 100.);
        let r = report(&s, &rows, Period::Year(2026));
        assert_eq!(r.seconds, 30.);
        assert_eq!(r.tracks, 2);
        assert_eq!(r.top_tracks[0].id, "b");
        assert_eq!(r.active_days, 2);
        s.listening_days.insert((jan / 86400) + 2, 50.);
        let r = report(&s, &rows, Period::Month(2026, 1));
        assert_eq!(r.seconds, 60.);
        assert_eq!(r.detailed_seconds, 10.);
        s.clear_history();
        assert!(s.months.is_empty());
        assert!(s.first_heard.is_empty());
    }
}
