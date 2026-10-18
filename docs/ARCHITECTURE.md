# Maintaining SSMP

Read SPECIFICATION.md for product decisions and RELEASE-CHECKLIST.md for tested scope.

| Component | Responsibility |
|---|---|
| `src/main.rs` | Desktop/host command dispatch and developer verification commands |
| `src/model.rs` | Versioned persisted media, preferences, playlist and profile structures |
| `src/store.rs` | SQLite transactions, credentials, sessions, user state and backups |
| `src/library.rs` | Scanning, media identity, ffprobe, playlists, metadata manifests and lyrics |
| `src/player.rs` | libmpv FFI, decoding, asynchronous events and software/OpenGL music-video frames |
| `src/server.rs` | HTTP API, authentication, ranges, remote client and verified offline copies |
| `src/ui.rs` | Desktop shell, persistent state, jobs and native capture harness |
| `src/ui/software_window.rs` | Cached CPU rasterization, native software presentation, bounded redraw scheduling and minimized playback ticks |
| `src/ui/music_ui.rs` | Music pages, playlists, statistics, lyrics and player layout |
| `src/ui/icons.rs`, `themes.rs`, `i18n.rs` | Vector icons, sixteen palettes and locale lookup |
| `assets/languages.json` | English/German/Spanish/French catalogs |
| `src/music.rs` | Listening clock, shuffle, previous threshold, sorting and enhanced lyrics |
| `src/metadata.rs` | Optional matching, cached online requests and non-overwriting merge |
| `SSYTDLP/source/ssmp_bridge.py` | Atomic completed-file handoffs, independently usable by the downloader |

The desktop uses a native Rust UI. Windows defaults to cached CPU rasterization and a softbuffer/GDI surface, creating no GPU device or swapchain. This replaced OpenGL and DirectX defaults after the user's desktop slowdown persisted across both GPU paths. `SSMP_RENDERER=dx12` and `opengl` retain explicit diagnostic alternatives; Linux/macOS retain the previous OpenGL path. `graphics.json` records the actual backend. `SSMP_PROFILE=1` enables local paint-time diagnostics. UI input/hover redraws are bounded to 60 fps; automatic music redraws to 25 fps and video to about 30 fps. Pauses reduce redraw frequency; minimized playback/accounting uses an independent 250 ms timer without painting. This stays below the listening clock gap limit and does not depend on suppressed redraw events.

libmpv supplies changed 1280Ã—720 software video frames for the CPU/DirectX window, becoming ordinary egui textures. CPU video frames receive new texture identities so cached rasterized primitives refresh correctly. Music-only playback uploads no video frames. The queue resolves only its selected media path, using a linear membership check instead of querying every queued file. Filtered rows and genre options are cached until inputs or library data change. Song lists allocate their full scroll extent but build widgets and covers only for visible rows. Unchanged folder-watcher signatures avoid reloading the library. All mpv handles and graphics objects stay on the creating thread; network requests and indexing use background jobs. Thumbnail decoding for newly visible covers and very large artist/album grids remain further profiling targets.

Media IDs are durable library keys initially derived from file size plus first/last 128 KiB SHA-256 sampling. This is a fast change/move fingerprint, not an audio fingerprint, whole-file cryptographic verification or a deduplication guarantee. A separate current fingerprint allows a known file to change while retaining its ID. Manual relinking retains user data. Acoustic matching, tag-independent identity and multiple physical locations per recording are future work.

The bottom player anchors its transport controls to the window center, volume to the right edge, and seeking to a separate full-width row. Set egui's `slider_width` on the slider's own UI scope: `add_sized` alone does not stretch the rail. Centered column/row helpers align page content; card rows allocate an explicit size for each card. Keep the available height intact in centered columns so video and scrolling can measure their viewport correctly. For visual checks, `SSMP_SMOKE_SIZE=1000x720` selects a smaller native window only when using the existing `--smoke-path` capture mode.

SQLite uses WAL and busy timeouts. Profile updates use optimistic revisions and an immediate transaction. A stale remote save returns 409. Never add silent last-writer-wins behavior without a designed merge policy. Offline download identity currently checks size and the same sampled fingerprint; transport uses HTTPS when an HTTPS host is configured. Downloads commit only after verification; interrupted temporary files are removed.

## API v1

All authenticated routes use `Authorization: Bearer TOKEN`. No token is accepted through a URL query string. Clients should use HTTPS or a trusted private network.

| Method | Path | Behavior |
|---|---|---|
| GET | `/api/v1/health` | Public name/version/health |
| POST | `/api/v1/login` | JSON username/password â†’ expiring session |
| POST | `/api/v1/logout` | Revoke current session |
| GET | `/api/v1/library` | Shared library with server directories/artwork paths removed |
| GET | `/api/v1/state` | Current user's state and revision |
| PUT | `/api/v1/state` | Save current user's state; stale revisions return 409 |
| GET/HEAD | `/api/v1/media/ID` | Authenticated file streaming with single HTTP ranges |
| PUT | `/api/v1/metadata/ID` | Owner-only changes to the allowed metadata fields |

Users are created locally using the CLI/host UI. There is no public account-creation endpoint. Server routes take media IDs, not remote filesystem paths. Owner metadata updates ignore supplied path/identity fields. The host has a request concurrency cap and per-IP login throttling. Production hardening, proxy deployment tests and broader load testing remain release gates.

## Build

Standard Rust: `cargo test --locked`, `cargo build --release --locked`. On Linux use `--no-default-features` for a host without desktop dependencies. Native desktop builds require platform development libraries and an installed libmpv at runtime. Windows `scripts/build.ps1` supports the existing local LLVM-MinGW toolchain where available and otherwise a normal Cargo installation. It puts caches and temporary files inside the project.

`scripts/bootstrap.py` is this workstation's reproducible initial preparation helper: it copies the existing downloader and fetches the pinned Windows mpv runtime. Other contributors should use a normal Rust toolchain and the runtime described in docs/THIRD-PARTY.md; they do not need the original user's filesystem paths.

Tests and `scripts/qa.py` use generated audio/video and disposable data directories, never the user's library. `scripts/verify_originals.py` checks SHA-256 for every file in the supplied downloader. `ROLLBACK.sh` restores only the workspace downloader integration and is verified on a disposable copy.

## Music 0.2 state and playback

SSMP owns the queue and loads the selected file directly. It no longer races asynchronous loadfile operations with playlist-pos changes. Async command replies carry identifiers; obsolete replies are discarded and file failures carry mpv error text. The drag preview is separate from the actual position and commits a single absolute exact seek on release. Cover images are drawn by the UI instead of auto-loading external images as video tracks.

ListeningClock accumulates wall time across quantized audio clock updates. Seek/restart discontinuities, pauses, buffering and long/suspended frames reset that interval. UserState records measured seconds only with history enabled. Playlist time is recorded only for the active playlist and its own member tracks. Clearing history clears those measurements. Serde defaults migrate earlier stored JSON without inventing hours.

Folder signatures and scans run on workers. The server reads host-owned local roots separately from authenticated profile state and scans on a background worker. `--watch PATH --serve` configures headless discovery. Automatic metadata is disabled by default and runs serially, with MusicBrainz spacing and cached provider replies. Delayed results merge onto the latest unlocked library record.

Run `scripts/music_playback_qa.py` for real seek/error/link-video checks and `scripts/music_ui_qa.py` for music screens, languages and sizes. `scripts/music_interactions.py` prepares disposable long tracks for native pointer/keyboard checks. `scripts/package_music.py` packages the reviewed Git index, so unrelated workspace edits cannot silently enter a source archive.


## Music 0.3

`stats.rs` aggregates calendar-month track seconds, qualified plays, hours and playlist time into period reports. The old all-time/daily fields remain available. `stats_detail_since` marks the reliable breakdown boundary; unavailable historical rankings are not fabricated. Exports use schema 2 with the original all-time keys. Large authenticated profile saves have a 16 MiB bound, while other request bodies retain 2 MiB limits. The revision transaction preserves period data when a 0.2 client saves ordinary changes; an explicit history reset still clears it.

`ui/enhancements.rs` owns selection, atomic local batch edits and paced explicit/automatic metadata jobs. Results rebase on current values. `ui/report_ui.rs` renders the wrap-up and rankings, caching reports. `ui/update_ui.rs` and `updates.rs` check fixed stable-release URLs and verified installer identities in workers; Windows setup waits for running instances/hosts. Installer payload/uninstall lists contain only app-owned files. User media is never recursively removed. `Dockerfile` builds the existing headless binary with no desktop/media-renderer dependency.
