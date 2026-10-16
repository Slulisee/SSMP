use crate::{model::*, store::Store};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub fn media_kind(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "mp3" | "flac" | "wav" | "ogg" | "opus" | "m4a" | "aac" | "wma" | "aiff" | "alac"
        | "ape" => Some("audio"),
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "m4v" | "ts" | "mpg" | "mpeg" | "flv" => {
            Some("video")
        }
        _ => None,
    }
}
pub fn identity(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hash = Sha256::new();
    hash.update(size.to_le_bytes());
    let mut buffer = vec![0u8; 128 * 1024];
    let n = file.read(&mut buffer)?;
    hash.update(&buffer[..n]);
    if size > buffer.len() as u64 {
        file.seek(SeekFrom::End(-(buffer.len() as i64)))?;
        let n = file.read(&mut buffer)?;
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}
pub fn episode(name: &str) -> (String, u32, u32) {
    let upper = name.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    for i in 0..bytes.len().saturating_sub(5) {
        if bytes[i] != b'S' {
            continue;
        }
        let rest = &upper[i + 1..];
        if let Some(e) = rest.find('E') {
            if e > 0 && e <= 3 {
                let s = rest[..e].parse::<u32>();
                let digits: String = rest[e + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let (Ok(s), Ok(ep)) = (s, digits.parse::<u32>()) {
                    return (
                        name[..i]
                            .trim_matches([' ', '.', '_', '-'])
                            .replace('.', " "),
                        s,
                        ep,
                    );
                }
            }
        }
    }
    (String::new(), 0, 0)
}
pub fn probe(path: &Path) -> Result<Media> {
    let path = fs::canonicalize(path).context("Media file is unavailable")?;
    let metadata = path.metadata()?;
    if !metadata.is_file() {
        bail!("Choose a media file")
    }
    let kind = media_kind(&path).context("Unsupported media extension")?;
    let title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let (series, season, ep) = episode(&title);
    let mut m = Media {
        id: identity(&path)?,
        path: path.to_string_lossy().into_owned(),
        title,
        kind: kind.into(),
        size: metadata.len(),
        added: crate::now(),
        modified: metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0),
        series,
        season,
        episode: ep,
        ..Default::default()
    };
    if let Ok(result) = crate::quiet_command(crate::tool("ffprobe"))
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(&path)
        .output()
    {
        if result.status.success() {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&result.stdout) {
                m.duration = v["format"]["duration"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.);
                if let Some(tags) = v["format"]["tags"].as_object() {
                    for (key, value) in tags {
                        let value = value.as_str().unwrap_or_default().to_string();
                        match key.to_ascii_lowercase().as_str() {
                            "title" => m.title = value,
                            "artist" | "album_artist" if m.artist.is_empty() => m.artist = value,
                            "album" => m.album = value,
                            "lyrics" | "unsyncedlyrics" => m.lyrics = value,
                            _ => {}
                        }
                    }
                }
                let mut tags = std::collections::BTreeMap::new();
                if let Some(format) = v["format"]["tags"].as_object() {
                    for (k, value) in format {
                        tags.insert(
                            k.to_ascii_lowercase(),
                            value.as_str().unwrap_or_default().to_string(),
                        );
                    }
                }
                if let Some(streams) = v["streams"].as_array() {
                    for stream in streams.iter().filter(|s| s["codec_type"] == "audio") {
                        if let Some(values) = stream["tags"].as_object() {
                            for (k, value) in values {
                                tags.entry(k.to_ascii_lowercase()).or_insert_with(|| {
                                    value.as_str().unwrap_or_default().to_string()
                                });
                            }
                        }
                    }
                }
                if let Some(title) = tags.get("title") {
                    m.title = title.clone()
                }
                if let Some(artist) = tags.get("artist").or_else(|| tags.get("album_artist")) {
                    m.artist = artist.clone()
                }
                if let Some(album) = tags.get("album") {
                    m.album = album.clone()
                }
                if let Some(lyrics) = tags.get("lyrics").or_else(|| tags.get("unsyncedlyrics")) {
                    m.lyrics = lyrics.clone()
                }
                m.track = tags
                    .get("track")
                    .or_else(|| tags.get("tracknumber"))
                    .and_then(|s| s.split('/').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                if kind == "video"
                    && v["streams"].as_array().is_some_and(|s| {
                        !s.iter().any(|s| {
                            s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1
                        })
                    })
                {
                    m.kind = "audio".into();
                }
            }
        }
    }
    m.fingerprint = m.id.clone();
    for ext in ["lrc", "txt"] {
        let sidecar = path.with_extension(ext);
        if sidecar.is_file() {
            m.lyrics = fs::read_to_string(sidecar).unwrap_or_default();
            break;
        }
    }
    for p in [
        path.with_extension("jpg"),
        path.with_extension("png"),
        path.parent().unwrap().join("cover.jpg"),
        path.parent().unwrap().join("folder.jpg"),
        path.parent().unwrap().join("cover.png"),
    ] {
        if p.is_file() {
            m.artwork = p.to_string_lossy().into_owned();
            break;
        }
    }
    Ok(m)
}
pub fn scan(store: &Store, paths: &[PathBuf]) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    for input in paths {
        let files: Vec<_> = if input.is_dir() {
            walkdir::WalkDir::new(input)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.into_path())
                .filter(|p| media_kind(p).is_some())
                .collect()
        } else {
            vec![input.clone()]
        };
        for path in files {
            let result = (|| -> Result<bool> {
                let p = fs::canonicalize(&path)?;
                let metadata = p.metadata()?;
                let key = p.to_string_lossy();
                let modified = metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                if let Some(mut old) = store.by_path(&key)? {
                    if old.size == metadata.len() && old.modified == modified {
                        if old.missing {
                            old.missing = false;
                            store.put(&old)?
                        }
                        return Ok(false);
                    }
                    let fresh = probe(&p)?;
                    old.duration = fresh.duration;
                    old.size = fresh.size;
                    old.modified = fresh.modified;
                    old.fingerprint = fresh.fingerprint;
                    old.missing = false;
                    if !old.locked {
                        old.title = fresh.title;
                        old.artist = fresh.artist;
                        old.album = fresh.album;
                        old.lyrics = fresh.lyrics;
                        old.artwork = fresh.artwork;
                    }
                    store.put(&old)?;
                    return Ok(false);
                }
                let mut m = probe(&p)?;
                if let Some(mut old) = store.get(&m.id)?.or(store.by_fingerprint(&m.fingerprint)?) {
                    old.path = m.path;
                    old.modified = m.modified;
                    old.missing = false;
                    store.put(&old)?;
                    return Ok(false);
                }
                m.missing = false;
                store.put(&m)?;
                Ok(true)
            })();
            match result {
                Ok(true) => report.added += 1,
                Ok(false) => report.updated += 1,
                Err(e) => report.errors.push(format!("{}: {e}", path.display())),
            }
        }
    }
    for mut m in store.media()? {
        if !Path::new(&m.path).is_file() && !m.missing {
            m.missing = true;
            store.put(&m)?;
        }
    }
    Ok(report)
}
pub fn import_manifest(store: &Store, path: &Path) -> Result<ImportReport> {
    let bytes = fs::read(path)?;
    if bytes.len() > 8 * 1024 * 1024 {
        bail!("Manifest is too large")
    }
    let manifest: ImportManifest = serde_json::from_slice(&bytes)?;
    if manifest.version != 1 {
        bail!("Unsupported manifest version")
    }
    let mut report = ImportReport::default();
    for item in manifest.items {
        let raw = PathBuf::from(&item.path);
        let p = if raw.is_absolute() {
            raw
        } else {
            path.parent().unwrap_or(Path::new(".")).join(raw)
        };
        if !p.is_file() {
            report.errors.push(format!("Missing: {}", p.display()));
            continue;
        }
        let p = fs::canonicalize(p)?;
        let mut m = if let Some(m) = store.by_path(&p.to_string_lossy())? {
            report.updated += 1;
            m
        } else {
            let m = probe(&p)?;
            if let Some(mut old) = store.get(&m.id)? {
                report.updated += 1;
                old.path = m.path;
                old
            } else {
                report.added += 1;
                m
            }
        };
        if !item.title.is_empty() {
            m.title = item.title;
            m.locked = true
        }
        if !item.artist.is_empty() {
            m.artist = item.artist;
            m.locked = true
        }
        if !item.album.is_empty() {
            m.album = item.album;
            m.locked = true
        }
        if !item.source.is_empty() {
            m.source = item.source
        }
        let series = if item.series.is_empty() {
            &manifest.series
        } else {
            &item.series
        };
        if !series.is_empty() {
            m.series = series.clone();
            m.locked = true
        }
        if item.season > 0 {
            m.season = item.season
        } else if manifest.season > 0 {
            m.season = manifest.season
        }
        if item.episode > 0 {
            m.episode = item.episode
        }
        store.put(&m)?;
    }
    Ok(report)
}
pub fn consume_inbox(store: &Store) -> Result<ImportReport> {
    let mut total = ImportReport::default();
    for item in fs::read_dir(store.root.join("imports"))?.flatten() {
        let path = item.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let claimed = path.with_extension("processing");
        if fs::rename(&path, &claimed).is_err() {
            continue;
        }
        match import_manifest(store, &claimed) {
            Ok(r) => {
                let dest = if r.errors.is_empty() {
                    "processed"
                } else {
                    "failed"
                };
                total.added += r.added;
                total.updated += r.updated;
                total.errors.extend(r.errors);
                let mut target = store.root.join("imports").join(dest).join(item.file_name());
                if target.exists() {
                    target = target.with_file_name(format!(
                        "{}-{}",
                        crate::token(),
                        item.file_name().to_string_lossy()
                    ));
                }
                fs::rename(&claimed, target)?;
            }
            Err(e) => {
                total.errors.push(format!("{}: {e}", path.display()));
                let target = store.root.join("imports/failed").join(format!(
                    "{}-{}",
                    crate::token(),
                    item.file_name().to_string_lossy()
                ));
                fs::rename(&claimed, target)?;
            }
        }
    }
    Ok(total)
}

pub fn playlist_locations(path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path)?
        .trim_start_matches('\u{feff}')
        .to_string();
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    match ext.as_str() {
        "m3u" | "m3u8" => Ok(text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .map(str::to_string)
            .collect()),
        "pls" => {
            let mut entries = Vec::new();
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    if let Some(n) = k.to_ascii_lowercase().strip_prefix("file") {
                        if let Ok(n) = n.parse::<usize>() {
                            entries.push((n, v.trim().to_string()))
                        }
                    }
                }
            }
            entries.sort_by_key(|p| p.0);
            Ok(entries.into_iter().map(|p| p.1).collect())
        }
        "xspf" | "xml" => {
            use quick_xml::{Reader, events::Event};
            let mut reader = Reader::from_str(&text);
            reader.config_mut().trim_text(true);
            let mut result = Vec::new();
            let mut current = String::new();
            let mut key_location = false;
            loop {
                match reader.read_event() {
                    Ok(Event::Start(e)) => {
                        current = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                    }
                    Ok(Event::Text(e)) => {
                        let raw = e.decode()?.into_owned();
                        let value = quick_xml::escape::unescape(&raw)?.into_owned();
                        if current == "location" || (current == "string" && key_location) {
                            result.push(value.clone());
                            key_location = false
                        }
                        if current == "key" {
                            key_location = value == "Location";
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => bail!("Invalid playlist XML: {e}"),
                    _ => {}
                }
            }
            Ok(result)
        }
        _ => bail!("Choose M3U, M3U8, PLS, XSPF, or iTunes XML"),
    }
}
pub fn import_playlist(store: &Store, path: &Path) -> Result<Playlist> {
    let mut playlist = Playlist {
        id: crate::token(),
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        ..Default::default()
    };
    for location in playlist_locations(path)? {
        let p = if location.starts_with("file:") {
            url::Url::parse(&location)
                .ok()
                .and_then(|u| u.to_file_path().ok())
        } else if location.contains("://") {
            None
        } else {
            Some(PathBuf::from(&location))
        };
        if let Some(p) = p {
            let p = if p.is_absolute() {
                p
            } else {
                path.parent().unwrap_or(Path::new(".")).join(p)
            };
            if p.is_file() && media_kind(&p).is_some() {
                let report = scan(store, &[p.clone()])?;
                if report.errors.is_empty() {
                    if let Some(m) = store.by_path(&fs::canonicalize(p)?.to_string_lossy())? {
                        playlist.items.push(m.id);
                        continue;
                    }
                }
            }
        }
        playlist.unresolved.push(location);
    }
    Ok(playlist)
}
pub fn export_playlist(store: &Store, p: &Playlist, path: &Path) -> Result<()> {
    let mut text = String::from("#EXTM3U\n");
    for id in &p.items {
        if let Some(m) = store.get(id)? {
            text.push_str(&format!(
                "#EXTINF:{},{} - {}\n{}\n",
                m.duration as u64,
                m.artist.replace(['\r', '\n'], " "),
                m.title.replace(['\r', '\n'], " "),
                m.path
            ));
        }
    }
    fs::write(path, text)?;
    Ok(())
}
pub fn lyrics(text: &str) -> Vec<(f64, String)> {
    let mut rows = Vec::new();
    let mut offset = 0.;
    for line in text.lines() {
        let mut rest = line.trim();
        let mut times = Vec::new();
        if let Some(value) = rest
            .strip_prefix("[offset:")
            .and_then(|s| s.strip_suffix(']'))
        {
            offset = value.parse::<f64>().unwrap_or(0.) / 1000.;
            continue;
        }
        while rest.starts_with('[') {
            let Some(end) = rest.find(']') else { break };
            let stamp = &rest[1..end];
            let Some((min, sec)) = stamp.split_once(':') else {
                break;
            };
            if let (Ok(m), Ok(s)) = (min.parse::<u32>(), sec.parse::<f64>()) {
                times.push(m as f64 * 60. + s);
                rest = &rest[end + 1..]
            } else {
                break;
            }
        }
        if times.is_empty() {
            if !rest.starts_with('[') && !rest.is_empty() {
                rows.push((-1., rest.into()));
            }
        } else {
            for t in times {
                rows.push((t, rest.into()));
            }
        }
    }
    for (t, _) in &mut rows {
        if *t >= 0. {
            *t += offset
        }
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wav(path: &Path) {
        let mut b = vec![0u8; 44 + 2000];
        b[..4].copy_from_slice(b"RIFF");
        fs::write(path, b).unwrap();
    }
    #[test]
    fn file_move_keeps_metadata_and_identity() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::open(&d.path().join("data")).unwrap();
        let a = d.path().join("song.wav");
        wav(&a);
        scan(&s, &[a.clone()]).unwrap();
        let mut m = s.media().unwrap().remove(0);
        m.title = "My title".into();
        m.locked = true;
        s.put(&m).unwrap();
        let b = d.path().join("renamed.wav");
        fs::rename(a, &b).unwrap();
        scan(&s, &[b]).unwrap();
        let rows = s.media().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, m.id);
        assert_eq!(rows[0].title, "My title");
        assert!(!rows[0].missing);
    }
    #[test]
    fn playlist_formats_and_unresolved_items() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::open(&d.path().join("data")).unwrap();
        wav(&d.path().join("a.wav"));
        for (ext, text) in [
            ("m3u8", "#EXTM3U\na.wav\nmissing.mp3"),
            ("pls", "[playlist]\nFile2=missing.mp3\nFile1=a.wav"),
            (
                "xspf",
                "<playlist><trackList><track><location>a.wav</location></track><track><location>missing.mp3</location></track></trackList></playlist>",
            ),
            (
                "xml",
                "<plist><dict><key>Location</key><string>a.wav</string><key>Location</key><string>missing.mp3</string></dict></plist>",
            ),
        ] {
            let p = d.path().join(format!("list.{ext}"));
            fs::write(&p, text).unwrap();
            let list = import_playlist(&s, &p).unwrap();
            assert_eq!(list.items.len(), 1, "{ext}");
            assert_eq!(list.unresolved, vec!["missing.mp3"]);
        }
    }
    #[test]
    fn manifests_are_idempotent_and_apply_season_metadata() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::open(d.path()).unwrap();
        let p = d.path().join("A S01E02.wav");
        wav(&p);
        let manifest = d.path().join("imports/one.json");
        let v = ImportManifest {
            version: 1,
            series: "A".into(),
            season: 2,
            items: vec![ImportItem {
                path: p.to_string_lossy().into(),
                title: "Episode".into(),
                episode: 3,
                ..Default::default()
            }],
        };
        fs::write(&manifest, serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(import_manifest(&s, &manifest).unwrap().added, 1);
        assert_eq!(import_manifest(&s, &manifest).unwrap().updated, 1);
        let m = s.media().unwrap().remove(0);
        assert_eq!((m.series.as_str(), m.season, m.episode), ("A", 2, 3));
    }
    #[test]
    fn lyrics_support_multiple_timestamps_and_offset() {
        let rows = lyrics("[offset:250]\n[00:01.50][00:03.00]Hello\n[00:05.00]World");
        assert_eq!(
            rows,
            vec![
                (1.75, "Hello".into()),
                (3.25, "Hello".into()),
                (5.25, "World".into())
            ]
        );
    }
    #[test]
    fn episode_detection_preserves_show_name() {
        assert_eq!(
            episode("Some.Show.S02E003.1080p"),
            ("Some Show".into(), 2, 3)
        );
    }
}
