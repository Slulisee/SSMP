# SSMP 0.1 release status

This is a working Windows development milestone. The complete product remains in development. The specification records the intended first release and later modules; this file records the delivered boundary.

## Implemented and exercised on Windows x64

- Native desktop, persistent SQLite collection, recursive manual scans, search, separate music/video views, album playback, favorites and playlists.
- Metadata probing with ffprobe, embedded and adjacent metadata, persistent manual overrides, stable identity after an unchanged file moves, previewed JSON season imports.
- libmpv audio, seek, OpenGL video, subtitle/audio selection and two simultaneous subtitle languages. Generated FLAC, MP3, Opus, WAV and MKV fixtures play and advance. Audible output through a physical device still needs human review: automated playback uses a null audio output.
- M3U/M3U8, PLS, XSPF and iTunes XML parsing, unresolved entry preservation and M3U8 export. Imports match available local paths; they do not automatically acquire absent songs.
- Adjacent/embedded lyrics, timestamped LRC and offset, manual import/editing. LRCLIB lookup is implemented and optional; live provider availability/results have not been validated.
- Saved queue, shuffle/repeat, configurable music/video resume, independent history and top-song display. No automatic playback on launch.
- Themes, accents, density, scale, lyrics/queue panels and preference export/import. Five actual native windows were captured and visually reviewed.
- Owner-created host accounts, Argon2id credentials, expiring/revocable sessions, independent profiles, owner metadata editing, authenticated range/HEAD streaming, stale revision rejection and verified offline file copying.
- SSYTDLP opt-in completed-download handoff, atomic inbox claiming, host and GUI inbox polling, failed-manifest retention. Actual bundled yt-dlp/FFmpeg downloaded a generated local media fixture and published a manifest.
- A headless feature selection, library backup command, unchanged original downloader hashes and rollback on a disposable copy.

## Antivirus and distribution

The user reported a potential-virus detection and temporarily disabled their protection. The first build disappeared after compilation. Its detection name has not been supplied. The new SSMP binary has ordinary PE version information, manifest and icon. SSYTDLP now uses a visible folder runtime instead of a self-extracting PyInstaller onefile executable; no obfuscator or packer was added.

A Microsoft Defender custom scan of the assembled SSMP package reported no threats. See `qa/antivirus-scan.log`. This does not resolve the reported Bitdefender/Malwarebytes detection. Retesting both packages with the user's normal protection enabled, identifying any remaining detection and vendor review if necessary are open distribution gates. Packages are unsigned. Code signing requires a publisher certificate; it is not a guarantee against detections.

## Required before a broad first release

- Physical audio devices, gapless transitions, variable frame rates/HDR, long playback, high DPI/accessibility and broader format/subtitle validation.
- OS media keys/session integration, close/minimize/tray options and sleep/wake behavior.
- Large-library profiling, bounded thumbnail cache and virtualized list layout; indexing and HTTP operations run off the UI thread, but some local artwork/database work still needs optimization.
- Live folder watching, embedded-cover extraction, multiple locations per recording and stronger duplicate matching. Sampled file identity is explicitly documented; it is not an acoustic fingerprint.
- Managed folders, tag write-back, reversible file operations, richer layouts/navigation, sorting, smart playlists and fuller personal mixes. Current recommendations are a top-song view, not an automatic recommendation engine.
- Offline playlist membership/metadata reconciliation, automatic cache rules and next-X-unwatched copying. Current playlist download is an explicit sequential batch; local/offline browsing works independently.
- Better remote conflict resolution, proxy/TLS deployment, LAN/Tailscale installation and load testing. The current server shares one collection with all authenticated users and delegates HTTPS to a proxy/private network.
- Native Linux/macOS builds, packages and service installation. Source and CI are prepared; neither OS was executed here. Android/iOS and third-party server adapters come later.
- Reproducible public release automation, complete corresponding-source audit for every bundled native dependency, signing and protected antivirus retesting. License texts, Cargo.lock and pinned runtime source/build links accompany this milestone; public distribution review remains open.

## Later modules

Torrent-client integration, a maintained user-configured-source guide, future-episode monitoring, mobile background services, casting, online series metadata matching and additional import/server adapters remain later modules. No torrent acquisition is included in 0.1.
