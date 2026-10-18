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
        println!("SSMP {} — {}", env!("CARGO_PKG_VERSION"), ssmp::APP_NAME);
        return Ok(());
    }
    if args.iter().any(|a| a == "--help") {
        println!(
            "SSMP — {}\n  --data DIRECTORY\n  --scan PATH\n  --watch DIRECTORY --serve [--bind 127.0.0.1:4534]\n  --serve [--bind 127.0.0.1:4534]\n  --add-user NAME --password-env VARIABLE [--admin]\n  --self-test MEDIA\n  --import MANIFEST\n  --enrich MEDIA_ID\n  --backup DESTINATION\n  --smoke-path MEDIA [--smoke-page PAGE]",
            ssmp::APP_NAME
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
    if let Some(path) = value(&args, "--watch") {
        let path = std::fs::canonicalize(path).context("Watch folder is unavailable")?;
        if !path.is_dir() {
            anyhow::bail!("Choose a watch folder");
        }
        let mut state = store.state("local")?;
        let path = path.to_string_lossy().into_owned();
        if !state.roots.contains(&path) {
            state.roots.push(path);
        }
        state.preferences.watch_folders = true;
        store.save_state("local", &state)?;
    }
    #[cfg(feature = "desktop")]
    if let Some(path) = value(&args, "--playback-regression") {
        ssmp::player::interaction_test(&path)?;
        return Ok(());
    }
    #[cfg(feature = "desktop")]
    if let Some(path) = value(&args, "--music-video-test") {
        ssmp::player::music_video_test(
            &path,
            &value(&args, "--video").context("Provide --video FILE")?,
        )?;
        return Ok(());
    }
    if let Some(id) = value(&args, "--enrich") {
        let media = store.get(&id)?.context("Unknown media")?;
        let mut lookup = media.clone();
        lookup.locked = false;
        let online = ssmp::metadata::enrich(&store, &lookup)?;
        let merged = ssmp::metadata::merge_requested(&media, &online);
        store.put(&merged)?;
        println!("{}", serde_json::to_string_pretty(&merged)?);
        return Ok(());
    }
    if let Some(id) = value(&args, "--find-lyrics") {
        let mut media = store.get(&id)?.context("Unknown media")?;
        media.lyrics = ssmp::metadata::lyrics(&media)?;
        media.locked = true;
        store.put(&media)?;
        println!("{}", serde_json::to_string_pretty(&media)?);
        return Ok(());
    }
    if args.iter().any(|a| a == "--statistics") {
        let period = if let Some(month) = value(&args, "--month") {
            let (y, m) = month.split_once('-').context("Use YYYY-MM")?;
            let (y, m) = (y.parse::<u32>()?, m.parse::<u32>()?);
            anyhow::ensure!(
                (1970..=9998).contains(&y) && (1..=12).contains(&m),
                "Invalid calendar month"
            );
            ssmp::stats::Period::Month(y, m)
        } else if let Some(y) = value(&args, "--year") {
            let y = y.parse::<u32>()?;
            anyhow::ensure!((1970..=9998).contains(&y), "Invalid year");
            ssmp::stats::Period::Year(y)
        } else {
            ssmp::stats::Period::AllTime
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&ssmp::stats::report(
                &store.state("local")?,
                &store.media()?,
                period
            ))?
        );
        return Ok(());
    }
    if args.iter().any(|a| a == "--check-app-updates") {
        println!(
            "{}",
            serde_json::to_string_pretty(&ssmp::updates::check()?)?
        );
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
        #[cfg(windows)]
        let _host_guard = ssmp::updates::AppMutex::hold();
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
        let args: Vec<_> = std::env::args().collect();
        let root = value(&args, "--data")
            .map(PathBuf::from)
            .unwrap_or_else(ssmp::data_dir);
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(root.join("last-error.txt"), message);
        std::process::exit(1)
    }
}
