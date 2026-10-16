use crate::{model::*, store::Store};
use anyhow::{Context, Result, bail};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tiny_http::{Header, Request, Response, StatusCode};

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).unwrap()
}
fn json(request: Request, status: u16, value: impl serde::Serialize) {
    let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec());
    let _ = request.respond(
        Response::from_data(body)
            .with_status_code(StatusCode(status))
            .with_header(header("Content-Type", "application/json; charset=utf-8"))
            .with_header(header("Cache-Control", "no-store")),
    );
}
fn body(request: &mut Request) -> Result<Vec<u8>> {
    if request.body_length().is_some_and(|n| n > 2 * 1024 * 1024) {
        bail!("Request body too large")
    }
    let mut bytes = Vec::new();
    request
        .as_reader()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 2 * 1024 * 1024 {
        bail!("Request body too large")
    }
    Ok(bytes)
}
pub fn byte_range(text: Option<&str>, size: u64) -> Result<(u64, u64, bool)> {
    if size == 0 {
        bail!("Empty file")
    }
    let Some(text) = text else {
        return Ok((0, size - 1, false));
    };
    let text = text.strip_prefix("bytes=").context("Invalid range")?;
    if text.contains(',') {
        bail!("Multiple ranges are unsupported")
    }
    let (start, end) = text.split_once('-').context("Invalid range")?;
    let (start, end) = if start.is_empty() {
        let n = end.parse::<u64>()?;
        if n == 0 {
            bail!("Invalid range")
        }
        (size.saturating_sub(n), size - 1)
    } else {
        (
            start.parse::<u64>()?,
            if end.is_empty() {
                size - 1
            } else {
                end.parse::<u64>()?.min(size - 1)
            },
        )
    };
    if start >= size || end < start {
        bail!("Range outside file")
    }
    Ok((start, end, true))
}
type Failures = Arc<Mutex<HashMap<String, (u64, u32)>>>;
pub fn serve(root: PathBuf, bind: &str) -> Result<()> {
    let _ = Store::open(&root)?;
    let server = tiny_http::Server::http(bind).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    println!("SSMP host listening at http://{}", server.server_addr());
    let failures: Failures = Default::default();
    let count = Arc::new(AtomicUsize::new(0));
    let mut last_inbox = std::time::Instant::now();
    loop {
        if last_inbox.elapsed() > std::time::Duration::from_secs(2) {
            last_inbox = std::time::Instant::now();
            if let Ok(store) = Store::open(&root) {
                if let Ok(report) = crate::library::consume_inbox(&store) {
                    if report.added + report.updated + report.errors.len() > 0 {
                        println!(
                            "Inbox: {} added, {} updated, {} issues",
                            report.added,
                            report.updated,
                            report.errors.len()
                        );
                    }
                }
            }
        }
        let request = match server.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(Some(request)) => request,
            Ok(None) => continue,
            Err(e) => return Err(e.into()),
        };
        if count.load(Ordering::SeqCst) >= 32 {
            json(request, 503, serde_json::json!({"error":"Host is busy"}));
            continue;
        }
        count.fetch_add(1, Ordering::SeqCst);
        let root = root.clone();
        let failures = failures.clone();
        let count = count.clone();
        std::thread::spawn(move || {
            struct Slot(Arc<AtomicUsize>);
            impl Drop for Slot {
                fn drop(&mut self) {
                    self.0.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _slot = Slot(count);
            handle(request, &root, failures);
        });
    }
}
fn handle(mut request: Request, root: &Path, failures: Failures) {
    let method = request.method().as_str().to_string();
    let path = request.url().split('?').next().unwrap_or("").to_string();
    if path == "/api/v1/health" && method == "GET" {
        json(
            request,
            200,
            serde_json::json!({"name":"SSMP","version":env!("CARGO_PKG_VERSION"),"api":1}),
        );
        return;
    }
    let mut store = match Store::open(root) {
        Ok(s) => s,
        Err(_) => {
            json(
                request,
                500,
                serde_json::json!({"error":"Library is unavailable"}),
            );
            return;
        }
    };
    if path == "/api/v1/login" && method == "POST" {
        let peer = request
            .remote_addr()
            .map(|a| a.ip().to_string())
            .unwrap_or_default();
        {
            let mut f = failures.lock().unwrap();
            f.retain(|_, (at, _)| crate::now().saturating_sub(*at) < 60);
            if f.get(&peer).is_some_and(|(_, n)| *n >= 6) {
                json(
                    request,
                    429,
                    serde_json::json!({"error":"Too many login attempts; retry in one minute"}),
                );
                return;
            }
        }
        let result = (|| -> Result<Session> {
            let login: Login = serde_json::from_slice(&body(&mut request)?)?;
            store.login(&login)
        })();
        match result {
            Ok(session) => {
                failures.lock().unwrap().remove(&peer);
                json(request, 200, session)
            }
            Err(_) => {
                let mut f = failures.lock().unwrap();
                let entry = f.entry(peer).or_insert((crate::now(), 0));
                entry.1 += 1;
                drop(f);
                json(
                    request,
                    401,
                    serde_json::json!({"error":"Invalid credentials"}),
                )
            }
        }
        return;
    }
    let token = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))
        .and_then(|h| h.value.as_str().strip_prefix("Bearer "))
        .unwrap_or("")
        .to_string();
    let (user, admin) = match store.authenticate(&token) {
        Ok(Some(u)) => u,
        _ => {
            json(
                request,
                401,
                serde_json::json!({"error":"Sign in to this SSMP host"}),
            );
            return;
        }
    };
    match (method.as_str(), path.as_str()) {
        ("POST", "/api/v1/logout") => {
            let _ = store.revoke(&token);
            json(request, 200, serde_json::json!({"ok":true}));
        }
        ("GET", "/api/v1/library") => match store.media() {
            Ok(mut rows) => {
                for row in &mut rows {
                    row.path = Path::new(&row.path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    row.artwork.clear();
                }
                json(request, 200, rows)
            }
            Err(_) => json(
                request,
                500,
                serde_json::json!({"error":"Could not read library"}),
            ),
        },
        ("GET", "/api/v1/state") => match store.state(&user) {
            Ok(mut s) => {
                s.roots.clear();
                json(request, 200, s)
            }
            Err(_) => json(
                request,
                500,
                serde_json::json!({"error":"Could not read profile"}),
            ),
        },
        ("PUT", "/api/v1/state") => {
            let result = (|| -> Result<UserState> {
                let mut state: UserState = serde_json::from_slice(&body(&mut request)?)?;
                state.roots.clear();
                store.compare_save(&user, &mut state)?;
                Ok(state)
            })();
            match result {
                Ok(state) => json(request, 200, state),
                Err(e) => json(request, 409, serde_json::json!({"error":e.to_string()})),
            }
        }
        ("PUT", p) if p.starts_with("/api/v1/metadata/") => {
            if !admin {
                json(
                    request,
                    403,
                    serde_json::json!({"error":"Only the owner can edit shared metadata"}),
                );
                return;
            }
            let result = (|| -> Result<Media> {
                let id = p.trim_start_matches("/api/v1/metadata/");
                let mut old = store.get(id)?.context("Unknown media")?;
                let edit: Media = serde_json::from_slice(&body(&mut request)?)?;
                old.title = edit.title;
                old.artist = edit.artist;
                old.album = edit.album;
                old.series = edit.series;
                old.season = edit.season;
                old.episode = edit.episode;
                old.lyrics = edit.lyrics;
                old.locked = true;
                store.put(&old)?;
                old.path.clear();
                old.artwork.clear();
                Ok(old)
            })();
            match result {
                Ok(m) => json(request, 200, m),
                Err(e) => json(request, 400, serde_json::json!({"error":e.to_string()})),
            }
        }
        ("GET" | "HEAD", p) if p.starts_with("/api/v1/media/") => {
            let id = p.trim_start_matches("/api/v1/media/");
            let m = match store.get(id) {
                Ok(Some(m)) => m,
                _ => {
                    json(request, 404, serde_json::json!({"error":"Unknown media"}));
                    return;
                }
            };
            let mut file = match std::fs::File::open(&m.path) {
                Ok(f) => f,
                Err(_) => {
                    json(
                        request,
                        404,
                        serde_json::json!({"error":"Media file is unavailable"}),
                    );
                    return;
                }
            };
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            let range = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Range"))
                .map(|h| h.value.as_str());
            let (start, end, partial) = match byte_range(range, size) {
                Ok(r) => r,
                Err(_) => {
                    let _ = request.respond(
                        Response::empty(416)
                            .with_header(header("Content-Range", &format!("bytes */{size}"))),
                    );
                    return;
                }
            };
            let length = end - start + 1;
            let _ = file.seek(SeekFrom::Start(start));
            let mut headers = vec![
                header("Accept-Ranges", "bytes"),
                header("Content-Type", mime(&m.path)),
                header("Cache-Control", "private, no-store"),
                header("X-Content-Type-Options", "nosniff"),
            ];
            if partial {
                headers.push(header(
                    "Content-Range",
                    &format!("bytes {start}-{end}/{size}"),
                ));
            }
            // tiny_http suppresses the body for HEAD while retaining the declared length.
            let response = Response::new(
                StatusCode(if partial { 206 } else { 200 }),
                headers,
                file.take(length),
                Some(length as usize),
                None,
            )
            .with_chunked_threshold(usize::MAX);
            let _ = request.respond(response);
        }
        _ => json(
            request,
            404,
            serde_json::json!({"error":"Unknown API route"}),
        ),
    }
}
fn mime(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "ogg" | "opus" => "audio/ogg",
        "m4a" => "audio/mp4",
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

#[derive(Clone)]
pub struct Remote {
    pub connection: Connection,
    client: reqwest::blocking::Client,
}
impl Remote {
    pub fn new(connection: Connection) -> Result<Self> {
        let url = url::Url::parse(&connection.url)?;
        if !["http", "https"].contains(&url.scheme())
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("Use an HTTP(S) SSMP host address without credentials or query parameters")
        }
        Ok(Self {
            connection,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
    pub fn login(url: &str, username: &str, password: &str) -> Result<Self> {
        let mut remote = Self::new(Connection {
            url: url.trim_end_matches('/').into(),
            token: String::new(),
            username: username.into(),
            admin: false,
        })?;
        let r = remote
            .client
            .post(format!("{}/api/v1/login", remote.connection.url))
            .json(&Login {
                username: username.into(),
                password: password.into(),
            })
            .send()?;
        if !r.status().is_success() {
            bail!("Sign-in failed (HTTP {})", r.status())
        }
        let session: Session = r.json()?;
        remote.connection.token = session.token;
        remote.connection.admin = session.admin;
        Ok(remote)
    }
    pub fn library(&self) -> Result<Vec<Media>> {
        Ok(self
            .client
            .get(format!("{}/api/v1/library", self.connection.url))
            .bearer_auth(&self.connection.token)
            .send()?
            .error_for_status()?
            .json()?)
    }
    pub fn state(&self) -> Result<UserState> {
        Ok(self
            .client
            .get(format!("{}/api/v1/state", self.connection.url))
            .bearer_auth(&self.connection.token)
            .send()?
            .error_for_status()?
            .json()?)
    }
    pub fn save(&self, state: &UserState) -> Result<UserState> {
        let r = self
            .client
            .put(format!("{}/api/v1/state", self.connection.url))
            .bearer_auth(&self.connection.token)
            .json(state)
            .send()?;
        if r.status().as_u16() == 409 {
            bail!("Profile changed on another device. Refresh the server before saving again.")
        }
        Ok(r.error_for_status()?.json()?)
    }
    pub fn playback_url(&self, id: &str) -> String {
        format!("{}/api/v1/media/{id}", self.connection.url)
    }
    pub fn download(&self, store: &Store, media: &Media) -> Result<Media> {
        use sha2::{Digest, Sha256};
        let host = hex::encode(Sha256::digest(self.connection.url.as_bytes()));
        let dir = store.root.join("offline").join(&host[..16]).join(&media.id);
        std::fs::create_dir_all(&dir)?;
        let ext = Path::new(&media.path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        let path = dir.join(format!("media.{ext}"));
        let part = path.with_extension(format!("{ext}.part"));
        let result = (|| -> Result<()> {
            let mut response = self
                .client
                .get(self.playback_url(&media.id))
                .timeout(std::time::Duration::from_secs(1800))
                .bearer_auth(&self.connection.token)
                .send()?
                .error_for_status()?;
            let mut out = std::fs::File::create(&part)?;
            let size = response.copy_to(&mut out)?;
            out.sync_all()?;
            if size != media.size {
                bail!("Download size mismatch")
            }
            let fingerprint = if media.fingerprint.is_empty() {
                &media.id
            } else {
                &media.fingerprint
            };
            if crate::library::identity(&part)? != *fingerprint {
                bail!("Download identity mismatch")
            }
            std::fs::rename(&part, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&part);
        }
        result?;
        let mut local = media.clone();
        local.path = path.to_string_lossy().into_owned();
        local.source = format!("SSMP host: {}", self.connection.url);
        local.artwork.clear();
        store.put(&local)?;
        Ok(local)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_and_invalid_byte_ranges() {
        assert_eq!(
            byte_range(Some("bytes=10-19"), 100).unwrap(),
            (10, 19, true)
        );
        assert_eq!(byte_range(Some("bytes=-8"), 100).unwrap(), (92, 99, true));
        assert_eq!(byte_range(Some("bytes=98-"), 100).unwrap(), (98, 99, true));
        assert!(byte_range(Some("bytes=100-"), 100).is_err());
        assert!(byte_range(Some("bytes=3-1"), 100).is_err());
        assert!(byte_range(Some("bytes=0-1,3-4"), 100).is_err());
    }
    #[test]
    fn rejects_non_http_and_embedded_credentials() {
        for url in [
            "file:///etc/passwd",
            "http://name:pass@localhost:4534",
            "https://example.com/?token=secret",
        ] {
            assert!(
                Remote::new(Connection {
                    url: url.into(),
                    token: String::new(),
                    username: String::new(),
                    admin: false
                })
                .is_err()
            );
        }
    }
}
