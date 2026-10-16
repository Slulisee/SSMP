# Maintaining SSMP

Read SPECIFICATION.md for product decisions and RELEASE-CHECKLIST.md for tested scope.

| Component | Responsibility |
|---|---|
| `src/main.rs` | Desktop/host command dispatch and developer verification commands |
| `src/model.rs` | Versioned persisted media, preferences, playlist and profile structures |
| `src/store.rs` | SQLite transactions, credentials, sessions, user state and backups |
| `src/library.rs` | Scanning, media identity, ffprobe, playlists, metadata manifests and lyrics |
| `src/player.rs` | libmpv FFI, decoding, queue commands and OpenGL video rendering |
| `src/server.rs` | HTTP API, authentication, ranges, remote client and verified offline copies |
| `src/ui.rs` | egui desktop interface, background jobs and playback-state integration |
| `SSYTDLP/source/ssmp_bridge.py` | Atomic completed-file handoffs, independently usable by the downloader |

The desktop uses a native Rust UI. libmpv renders to an OpenGL framebuffer texture; egui displays that texture in its own layout. Software rendering is used only by the headless playback-test command, not as the desktop video path. All mpv handles and GL objects stay on the creating thread. Network requests and indexing use background jobs. Avoid blocking I/O in draw callbacks; thumbnail decoding and very large collection layout still need profiling and optimization.

Media IDs are durable library keys initially derived from file size plus first/last 128 KiB SHA-256 sampling. This is a fast change/move fingerprint, not an audio fingerprint, whole-file cryptographic verification or a deduplication guarantee. A separate current fingerprint allows a known file to change while retaining its ID. Manual relinking retains user data. Acoustic matching, tag-independent identity and multiple physical locations per recording are future work.

SQLite uses WAL and busy timeouts. Profile updates use optimistic revisions and an immediate transaction. A stale remote save returns 409. Never add silent last-writer-wins behavior without a designed merge policy. Offline download identity currently checks size and the same sampled fingerprint; transport uses HTTPS when an HTTPS host is configured. Downloads commit only after verification; interrupted temporary files are removed.

## API v1

All authenticated routes use `Authorization: Bearer TOKEN`. No token is accepted through a URL query string. Clients should use HTTPS or a trusted private network.

| Method | Path | Behavior |
|---|---|---|
| GET | `/api/v1/health` | Public name/version/health |
| POST | `/api/v1/login` | JSON username/password → expiring session |
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
