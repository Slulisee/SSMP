#![cfg_attr(
    all(not(debug_assertions), feature = "desktop"),
    windows_subsystem = "windows"
)]
#[cfg(feature = "desktop")]
mod ui;
use anyhow::{Context, Result};
use ssmp::{library, store::Store};
use std::path::PathBuf;

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|s| s == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let root = value(&args, "--data")
        .map(PathBuf::from)
        .unwrap_or_else(ssmp::data_dir);
    let store = Store::open(&root)?;
    if args.iter().any(|a| a == "--version") {
        println!("SSMP {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help") {
        println!(
            "SSMP\n  --data DIRECTORY\n  --scan PATH\n  --serve [--bind 127.0.0.1:4534]\n  --add-user NAME --password-env VARIABLE [--admin]\n  --self-test MEDIA\n  --import MANIFEST\n  --backup DESTINATION\n  --smoke-path MEDIA [--smoke-page PAGE]"
        );
        return Ok(());
    }
    if let Some(name) = value(&args, "--add-user") {
        let variable = value(&args, "--password-env").context("Provide --password-env VARIABLE")?;
        let password = std::env::var(variable).context("Password environment variable is unset")?;
        store.add_user(&name, &password, args.iter().any(|s| s == "--admin"))?;
        println!("Created user: {name}");
        return Ok(());
    }
    if let Some(path) = value(&args, "--scan") {
        let report = library::scan(&store, &[PathBuf::from(path)])?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if let Some(path) = value(&args, "--import") {
        let report = library::import_manifest(&store, &PathBuf::from(path))?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    if let Some(path) = value(&args, "--backup") {
        store.backup(&PathBuf::from(path))?;
        println!("Backup saved");
        return Ok(());
    }
    #[cfg(feature = "desktop")]
    if let Some(path) = value(&args, "--self-test") {
        ssmp::player::self_test(&path)?;
        return Ok(());
    }
    #[cfg(feature = "desktop")]
    if let Some(url) = value(&args, "--self-test-url") {
        let variable = value(&args, "--token-env").context("Provide --token-env VARIABLE")?;
        let token = std::env::var(variable)?;
        ssmp::player::self_test_authenticated(&url, Some(&token))?;
        return Ok(());
    }
    if let Some(url) = value(&args, "--download") {
        let username = value(&args, "--username").context("Provide --username")?;
        let variable = value(&args, "--password-env").context("Provide --password-env VARIABLE")?;
        let id = value(&args, "--media-id").context("Provide --media-id")?;
        let remote = ssmp::server::Remote::login(&url, &username, &std::env::var(variable)?)?;
        let m = remote
            .library()?
            .into_iter()
            .find(|m| m.id == id)
            .context("Unknown media")?;
        let saved = remote.download(&store, &m)?;
        println!("OFFLINE_SAVED={}", saved.path);
        return Ok(());
    }
    if args.iter().any(|s| s == "--serve") {
        return ssmp::server::serve(
            root,
            &value(&args, "--bind").unwrap_or_else(|| "127.0.0.1:4534".into()),
        );
    }
    if let Some(path) = value(&args, "--smoke-path") {
        library::scan(&store, &[PathBuf::from(path)])?;
    }
    #[cfg(feature = "desktop")]
    {
        ui::launch(
            root,
            value(&args, "--smoke-path").is_some(),
            value(&args, "--smoke-page"),
        )
    }
    #[cfg(not(feature = "desktop"))]
    {
        anyhow::bail!("This is the headless host build. Use --serve or --help.")
    }
}
fn main() {
    if let Err(e) = run() {
        let message = format!("SSMP: {e:#}");
        eprintln!("{message}");
        let root = ssmp::data_dir();
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(root.join("last-error.txt"), message);
        std::process::exit(1)
    }
}
