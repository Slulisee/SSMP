//! Stable app updates from this repository. Files are verified before handoff.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
const REPO: &str = "https://github.com/Slulisee/SSMP/releases";
const API: &str = "https://api.github.com/repos/Slulisee/SSMP/releases/latest";
const ASSET: &str = "SSMP-Windows-AMD64-Setup.exe";
const MAX: u64 = 512 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    pub checksum_url: Option<String>,
}
fn version(value: &str) -> Result<[u32; 3]> {
    let parts: Vec<_> = value
        .strip_prefix('v')
        .unwrap_or(value)
        .split('.')
        .collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|s| s.is_empty() || s.len() > 5 || !s.chars().all(|c| c.is_ascii_digit()))
    {
        bail!("Invalid stable app version");
    }
    Ok([parts[0].parse()?, parts[1].parse()?, parts[2].parse()?])
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}
pub fn parse_release(value: &Value, current: &str) -> Result<Option<Release>> {
    let tag = value["tag_name"]
        .as_str()
        .context("Missing release version")?;
    if value["draft"].as_bool().unwrap_or(true) || value["prerelease"].as_bool().unwrap_or(true) {
        bail!("Only stable published app updates are accepted");
    }
    let next = version(tag)?;
    if value["html_url"].as_str() != Some(format!("{REPO}/tag/{tag}").as_str()) {
        bail!("Unexpected release repository");
    }
    if next <= version(current)? {
        return Ok(None);
    }
    let assets = value["assets"].as_array().context("No update assets")?;
    let asset = assets
        .iter()
        .find(|a| a["name"] == ASSET)
        .context("The release installer is not available yet")?;
    let url = format!("{REPO}/download/{tag}/{ASSET}");
    if asset["browser_download_url"].as_str() != Some(&url) {
        bail!("Unexpected installer URL");
    }
    let bytes = asset["size"]
        .as_u64()
        .filter(|s| *s > 0 && *s <= MAX)
        .context("Invalid installer size")?;
    let sha = asset["digest"]
        .as_str()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| valid_hash(s))
        .unwrap_or("")
        .to_lowercase();
    let checksum = format!("{url}.sha256");
    let checksum_url = assets
        .iter()
        .any(|a| a["name"] == format!("{ASSET}.sha256") && a["browser_download_url"] == checksum)
        .then_some(checksum);
    if sha.is_empty() && checksum_url.is_none() {
        bail!("The installer checksum is not available yet");
    }
    Ok(Some(Release {
        version: tag.trim_start_matches('v').into(),
        url,
        bytes,
        sha256: sha,
        checksum_url,
    }))
}
fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .user_agent(concat!("SSMP/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() != "https" {
                attempt.stop()
            } else if attempt.previous().len() > 6 {
                attempt.error("Too many update redirects")
            } else {
                attempt.follow()
            }
        }))
        .build()?)
}
pub fn check() -> Result<Option<Release>> {
    let response = client()?
        .get(API)
        .header("Accept", "application/vnd.github+json")
        .send()?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    response
        .error_for_status()?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 2 * 1024 * 1024 {
        bail!("Update response is too large");
    }
    parse_release(&serde_json::from_slice(&bytes)?, env!("CARGO_PKG_VERSION"))
}
fn validate_release(r: &Release) -> Result<()> {
    version(&r.version)?;
    let expected = format!("{REPO}/download/v{}/{ASSET}", r.version);
    // Our publisher consistently uses v-prefixed stable tags.
    if r.url != expected || r.bytes == 0 || r.bytes > MAX || !valid_hash(&r.sha256) {
        bail!("Invalid staged update identity");
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
struct Pending {
    release: Release,
}
pub fn download(
    root: &Path,
    mut release: Release,
    mut progress: impl FnMut(u64, u64),
) -> Result<()> {
    let client = client()?;
    if release.sha256.is_empty() {
        let url = release
            .checksum_url
            .as_ref()
            .context("No installer checksum")?;
        if url != &format!("{}.sha256", release.url) {
            bail!("Unexpected checksum URL");
        }
        let mut bytes = Vec::new();
        client
            .get(url)
            .send()?
            .error_for_status()?
            .take(4097)
            .read_to_end(&mut bytes)?;
        let text = String::from_utf8(bytes)?;
        let fields: Vec<_> = text.split_whitespace().collect();
        if fields.len() != 2 || !valid_hash(fields[0]) || fields[1].trim_start_matches('*') != ASSET
        {
            bail!("Invalid installer checksum");
        }
        release.sha256 = fields[0].to_lowercase();
    }
    validate_release(&release)?;
    let folder = root.join("updates").join(&release.version);
    std::fs::create_dir_all(&folder)?;
    let temporary = folder.join(format!("{ASSET}.part"));
    let target = folder.join(ASSET);
    let result = (|| -> Result<()> {
        let mut response = client.get(&release.url).send()?.error_for_status()?;
        let mut output = std::fs::File::create(&temporary)?;
        let mut digest = Sha256::new();
        let mut count = 0;
        let mut buffer = [0; 256 * 1024];
        loop {
            let n = response.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > release.bytes {
                bail!("Installer exceeds its published size");
            }
            output.write_all(&buffer[..n])?;
            digest.update(&buffer[..n]);
            progress(count, release.bytes);
        }
        output.sync_all()?;
        drop(output);
        if count != release.bytes || hex::encode(digest.finalize()) != release.sha256 {
            bail!("Installer checksum or size mismatch");
        }
        std::fs::rename(&temporary, &target)?;
        let pending = root.join("updates/pending.json");
        let part = pending.with_extension("part");
        std::fs::write(&part, serde_json::to_vec_pretty(&Pending { release })?)?;
        std::fs::rename(part, pending)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
pub fn pending_version(root: &Path) -> Option<String> {
    let p: Pending =
        serde_json::from_slice(&std::fs::read(root.join("updates/pending.json")).ok()?).ok()?;
    (version(&p.release.version).ok()? > version(env!("CARGO_PKG_VERSION")).ok()?)
        .then_some(p.release.version)
}
pub fn verified_pending(root: &Path) -> Result<PathBuf> {
    let pending: Pending =
        serde_json::from_slice(&std::fs::read(root.join("updates/pending.json"))?)?;
    validate_release(&pending.release)?;
    let path = root
        .join("updates")
        .join(&pending.release.version)
        .join(ASSET);
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root.join("updates").canonicalize()?) {
        bail!("Invalid update cache path");
    }
    let mut file = std::fs::File::open(&canonical)?;
    if file.metadata()?.len() != pending.release.bytes {
        bail!("Installer size changed after download");
    }
    let mut h = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    if hex::encode(h.finalize()) != pending.release.sha256 {
        bail!("Installer checksum changed after download");
    }
    Ok(canonical)
}
pub fn launch_pending(root: &Path, home: &Path) -> Result<()> {
    if pending_version(root).is_none() {
        return Ok(());
    }
    let path = verified_pending(root)?;
    if !home.join("SSMP.exe").is_file() {
        bail!("The SSMP installation folder is missing");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // NSIS /D must be last and unquoted, even for a folder containing spaces.
        std::process::Command::new(path)
            .raw_arg(format!(
                "/S /AUTO /D={}",
                home.canonicalize()?
                    .display()
                    .to_string()
                    .trim_start_matches(r"\\?\")
            ))
            .creation_flags(0x08000000)
            .spawn()?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        bail!("Automatic installation currently requires Windows");
    }
}

#[cfg(windows)]
pub struct AppMutex(*mut std::ffi::c_void);
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(a: *mut std::ffi::c_void, b: i32, c: *const u16) -> *mut std::ffi::c_void;
    fn CloseHandle(h: *mut std::ffi::c_void) -> i32;
}
#[cfg(windows)]
impl AppMutex {
    pub fn hold() -> Self {
        let name: Vec<_> = "Local\\Slulisee.SSMP.Application"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        Self(unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) })
    }
}
#[cfg(windows)]
impl Drop for AppMutex {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        serde_json::json!({"tag_name":"v0.4.0","draft":false,"prerelease":false,"html_url":format!("{REPO}/tag/v0.4.0"),"assets":[{"name":ASSET,"browser_download_url":format!("{REPO}/download/v0.4.0/{ASSET}"),"size":12,"digest":format!("sha256:{}","a".repeat(64))}]})
    }
    #[test]
    fn stable_updates_require_correct_repository_size_and_digest() {
        let mut v = fixture();
        assert!(parse_release(&v, "0.3.0").unwrap().is_some());
        assert!(parse_release(&v, "0.4.0").unwrap().is_none());
        v["assets"][0]["browser_download_url"] = "https://example.com/installer.exe".into();
        assert!(parse_release(&v, "0.3.0").is_err());
        v = fixture();
        v["prerelease"] = true.into();
        assert!(parse_release(&v, "0.3.0").is_err());
        v = fixture();
        v["assets"][0]["digest"] = "".into();
        assert!(parse_release(&v, "0.3.0").is_err());
    }
    #[test]
    fn corrupted_pending_installer_is_never_executed() {
        let temp = tempfile::tempdir().unwrap();
        let mut r = parse_release(&fixture(), "0.3.0").unwrap().unwrap();
        let bytes = b"test fixture";
        r.bytes = bytes.len() as u64;
        r.sha256 = hex::encode(Sha256::digest(bytes));
        let folder = temp.path().join("updates/0.4.0");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(ASSET), bytes).unwrap();
        std::fs::write(
            temp.path().join("updates/pending.json"),
            serde_json::to_vec(&Pending { release: r }).unwrap(),
        )
        .unwrap();
        assert!(verified_pending(temp.path()).is_ok());
        std::fs::write(folder.join(ASSET), b"bad replacement").unwrap();
        assert!(verified_pending(temp.path()).is_err());
    }
}
