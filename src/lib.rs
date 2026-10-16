pub mod library;
pub mod model;
#[cfg(feature = "desktop")]
pub mod player;
pub mod server;
pub mod store;

use std::path::PathBuf;
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("SSMP_DATA_DIR") {
        return p.into();
    }
    if cfg!(windows) {
        return PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
            .join("SSMP");
    }
    if cfg!(target_os = "macos") {
        return PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
            .join("Library/Application Support/SSMP");
    }
    PathBuf::from(std::env::var_os("XDG_DATA_HOME").unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
            .join(".local/share")
            .into_os_string()
    }))
    .join("ssmp")
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
pub fn quiet_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}
pub fn tool(name: &str) -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    for base in [
        exe.parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    ] {
        for p in [
            base.join("tools").join(format!("{name}{suffix}")),
            base.join("SSYTDLP/tools").join(format!("{name}{suffix}")),
        ] {
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from(format!("{name}{suffix}"))
}
