# SSMP — product specification

Slulisee's Simple Music / Streaming Player. Agreed product direction, 2026-10-16.

## Product promise

Add your files and press play. SSMP is a local-first music and video player with optional personal hosting. Local playback, library management, playlists, lyrics and customization require no account, cloud service, subscription, advertisements or internet connection. Optional network features never become prerequisites for local use.

## Platform and architecture decisions

Desktop first: Windows, macOS and Linux. The headless host is a separate operating mode and must run on Linux without a graphical session. Android and iOS are subsequent clients; do not claim desktop compilation establishes mobile support.

Rust owns library persistence, importers, profiles, server APIs and the desktop application. The first desktop renderer is egui/eframe. libmpv owns decoding, audio output, seeking, gapless playlists and video rendering. Use the same library model for local and hosted collections. Future mobile clients use the documented API; the desktop renderer is not a claim of mobile readiness.

SQLite stores durable track identities, metadata overrides, playlists, play history, preferences and user data. Media files remain ordinary files. Scan and network work run off the UI thread. Runtime errors must appear in the UI. Save changes transactionally. Stable content identities and explicit relinking preserve user information across file moves. Never merge live and studio recordings merely because titles match.

## First desktop release requirements

### Library and imports

- Open files and recursively index selected folders. Read embedded tags and adjacent artwork, lyrics and metadata when present. Watch explicitly selected import folders; never silently index unrelated folders.
- Music, music videos, and general video including anime/series have appropriate separate views. Video initially has a simple file-oriented presentation with editable series, season and episode fields.
- Supply metadata corrections through an editor; preserve overrides on rescan. Import a season/series metadata JSON manifest with a preview/report. Later online metadata connectors must present matches before applying them.
- Built-in playlist imports: M3U/M3U8, PLS, XSPF and iTunes library XML. Preserve unresolved entries in an import report; do not acquire media silently. Importers have a documented extension boundary.
- Users choose between keeping folders as they are and managed-library organization. Managed moves/renames and tag write-back require separately configured permissions and a preview/undo record.

### Playback and presentation

- Persistent editable queue; shuffle, repeat, volume, seek, background audio, normal playback when the window loses focus, and restored queue without automatic playback on launch.
- Gapless album playback through a persistent playback engine; crossfade and loudness normalization are independent optional settings.
- Video rendering, embedded/external subtitles, subtitle delay, styling, audio/subtitle-track selection and optional two-language subtitles.
- Plain and time-synchronized lyrics, manual sidecar support, lyrics offset and optional online retrieval. Missing lyrics never prevent playback.
- Good keyboard navigation, shortcuts, search, sorting, configurable navigation, optional lyrics/queue panels, themes, accent color, text size and density. Changes are reversible and preferences exportable.

### Personal state

- Listening history, personal mixes, music queue/playlist resume and video resume are independent controls. Turning history off stops durable listening-event collection.
- Favorites, recently played and top tracks. Configurable history-based mixes and smart playlists.
- Suitable settings synchronize by default for hosted users with a toggle. Device paths, credentials and audio outputs remain device-specific. Resolve concurrent edits deliberately; no implied completion from a settings checkbox.

## Hosting and offline behavior

The same distribution provides `ssmp --serve`. The owner explicitly creates credentials, chooses shared media and chooses a bind address. Default bind is loopback. LAN/Tailscale setup is documented; router forwarding, VPN changes and firewall changes are not performed silently. HTTPS is delegated to a reverse proxy or private network for the first release.

Profiles have separate favorites, playlists, history and playback positions. Library metadata is shared; administrative file operations require owner privileges. APIs never accept arbitrary filesystem paths from a remote client. File streaming supports HTTP byte ranges. Credentials are hashed; sessions are revocable. The headless host survives closing a desktop client because it is a separate process.

Offline files include enough metadata for normal library browsing. Downloads are written to temporary files and only committed after successful transfer. Cache keys include host and media identity. Manual downloads and keeping a playlist offline precede automatic cache rules. An unavailable server must leave local/cached playback functional.

"Keep the next X unwatched episodes" initially means downloading existing server media onto a device. Acquiring missing content is a different opt-in subsystem.

## SSYTDLP integration

Preserve the existing SSYTDLP name and independent usability. Add an optional "Add completed downloads to SSMP" setting and configurable inbox destination. Emit versioned JSON manifests only for completed files, via atomic rename. Include source metadata where present. SSMP acknowledges/imports manifests idempotently; a failed handoff is reported without converting a successful media download into failure.

Copy the supplied downloader into this workspace before editing. Preserve original file hashes and rollback copies. Its existing MIT license already satisfies the final fully-open-source decision; retain third-party notices and regenerate packages/checksums when changed.

## Later modules

- Torrent acquisition through an established external client, using user-configured sources. A maintained setup guide links to relevant documentation/community discussions and explains configuration. No claim that popularity guarantees file safety.
- Initially acquire released episodes only; future-release monitoring comes afterward. Quality/language preferences, duplicates, disk quotas, upload/seeding limits and file moves must be coordinated with the download client. Explain peer exposure and VPN limitations on first enablement; any VPN must apply to the actual download host.
- Android/iOS applications, external music-server adapters, casting, advanced layout editing, additional metadata/lyrics connectors, and broader importer extensions.

## Licensing

Fully open source, without a no-resale clause. License SSMP's original project code under GPL-3.0-or-later, compatible with the chosen GPL media runtime; keep independently distributed SSYTDLP original source under its existing MIT license. Preserve all bundled dependency notices and provide exact runtime source/build pointers. Revisit distribution-specific compatibility before mobile/store releases.

## Validation and delivery gates

Every delivered milestone contains executable functionality and source. The release checklist distinguishes implemented, tested on Windows, source-prepared for Linux/macOS, and planned features. Do not call the full first-release scope complete while requirements remain open.

Tests cover real media probing/scanning, stable identity after rename, metadata overrides, playlist variants and unresolved entries, lyrics timestamps, durable profiles/playlists/history, authenticated streaming/ranges, offline files, import handoff and downloader rollback. Exercise actual libmpv playback/seek/video frames with generated media, and render the actual desktop UI for visual review. Preserve originals and test rollback on a disposable copy. Native OS media controls, audio hardware, Linux/macOS packaging and mobile behavior require their own evidence.

## First implementation milestone

Deliver a runnable Windows desktop application, portable package, shared library core and a working authenticated headless host; indexing and metadata editing, playlists/imports, queue and audio/video playback, lyrics, independent history/resume settings, host connection/offline copying, and a working SSYTDLP inbox handoff. Record all remaining first-release gates in RELEASE-CHECKLIST.md. This milestone is version 0.1, not a claim that every first-release or mobile feature is complete.
