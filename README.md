# SSMP

**Slulisee's Simple Media Player (SSMP)** — your collection, your rules.

A native, local-first music and video player with an optional personal media host. No account is required for local playback. No advertisements, subscriptions, or mandatory online services.

This repository contains the **0.1 development milestone source**. Portable Windows packages have been built and tested locally; they are not yet published as GitHub release downloads. For a source checkout, follow **Build from source** below. Remaining release gates are recorded in [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md).

## Start on Windows

Open `SSMP.exe` from the extracted portable ZIP (or `dist/SSMP-0.1.0-windows-x64/SSMP.exe` in this workspace), then choose **Add a folder** or drop your music/videos onto the window. Keep `libunwind.dll` and the complete `tools` folder beside the executable: they contain required runtime libraries, the playback engine and metadata reader. The app stores its library in `%LOCALAPPDATA%\SSMP`; use `SSMP.exe --data PATH` for a separate library.

The portable ZIP is `dist/SSMP-0.1.0-windows-x64.zip`. The updated downloader is packaged separately as `dist/SSYTDLP-with-SSMP-windows-x64.zip`. Its existing installation is preserved.

## This milestone

- Local audio/video indexing, search, album views, favorites, queue, shuffle and repeat.
- libmpv audio/video playback, OpenGL video rendering, audio/subtitle-track selection and dual subtitles.
- Embedded/sidecar lyrics, synchronized LRC, manual edits/import, and optional on-demand LRCLIB lookup.
- M3U/M3U8, PLS, XSPF and iTunes XML imports with unresolved-entry reporting; M3U8 export.
- Editable metadata and previewed JSON season/series imports.
- Independent history, personal top-song view, music resume and video resume controls.
- Four themes, accent colors, interface scale, compact rows, optional lyrics/queue panels and preference export/import.
- Authenticated host, individual profiles, shared metadata ownership, range streaming, profile revision checks and offline copying.
- Optional completed-file SSYTDLP handoff through an atomic inbox manifest.

Version 0.1 is the first working desktop/host milestone. Read [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) for implementation gaps and verified platform boundaries. This is not the completed mobile or full first-release roadmap.

## Useful controls

**Space** plays/pauses; **Left/Right** seek five seconds when a text field is not focused. **Escape** leaves fullscreen. Use a track's **…** menu for play next, queue, metadata, playlists, and offline downloads. Audio continues while the window is inactive. Operating-system media keys, tray behavior and mobile background services are separate release gates.

## Hosting and imports

- [Hosting guide](docs/HOSTING.md): Windows, headless Linux build, users, LAN/Tailscale, backups and diagnostics.
- [Import guide](docs/IMPORTS.md): playlists, lyrics, season metadata and SSYTDLP integration.
- [Architecture and API](docs/ARCHITECTURE.md): source map, persistence, native media boundary and maintenance.
- [Product specification](SPECIFICATION.md): agreed direction, release scope and later modules.
- [Third-party components](docs/THIRD-PARTY.md): playback runtime provenance, licenses and source links.
- Local delivery packages contain `VERIFICATION.txt` and selected `qa/` evidence. Generated test libraries, machine-specific reports and binaries are excluded from Git.

## Build from source

Install a current stable Rust toolchain. `cargo build --release --locked` builds the desktop. Install libmpv and ffprobe for native Linux/macOS desktop use; the Windows package supplies them. `cargo build --release --locked --no-default-features` builds a host without desktop graphics dependencies. Run `cargo test --locked` for core tests. `scripts/build.ps1` is the Windows helper.

Windows packaging is performed by `scripts/package.py` after a verified native build. CI workflows are supplied for Linux/macOS/Windows; an unexecuted workflow is not a platform test result.

## License

SSMP's original code is **GPL-3.0-or-later**, without a resale restriction. See LICENSE. The independently maintained SSYTDLP source retains its existing **MIT** license; other bundled components retain their own terms and notices.

## Later modules

Torrent acquisition, a maintained user-source setup guide, future-release monitoring, Android/iOS clients, third-party server adapters, richer series pages, managed file organization, importer extensions, and advanced customization remain explicitly tracked roadmap work.
