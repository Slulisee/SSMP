# SSMP 0.2 release status

This release is a runnable Windows music-player update. The accepted scope is in [the specification](docs/MUSIC-0.2-SPEC.md), with operation in [the music guide](docs/MUSIC-GUIDE.md). Exact commands and exit results accompany the local delivery in MUSIC-VERIFICATION.txt and qa/ evidence.

## Implemented

- Music-focused navigation, artists/albums, music videos, favorites, local/offline libraries and personal hosting. General series/episode/subtitle screens have been removed.
- Preview-on-drag/seek-on-release, previous/restart threshold, independent shuffle mode, weighted favorite artists, repeat, error recovery, output selection and normalization.
- Four language catalogs and sixteen themes, accent/scale/density controls, vector icons, animated hover/playing/lyrics states and reduced motion.
- Cached CPU-rendered interface on Windows, with no GPU device/swapchain, a software-frame music-video path, visible-row rendering, cached filtering and adaptive repaint timers. Explicit GPU diagnostic alternatives remain; Linux/macOS retain their existing OpenGL path.
- Playlist creation from a song or album, image/description editing, local-song selection, number/album/date/duration columns, sort/search/genre filters, custom-order moves, export and deletion.
- Local synced/word-timed lyrics, full-screen focus mode, offset and optional LRCLIB search. Linked video can be hidden/restored while music continues.
- Opt-in measured listening time, playlist-specific time, top songs/artists/genres, daily activity, streaks, busiest UTC hour, missing-genre coverage, clearing and JSON export. Legacy play counters do not become invented hours.
- Optional cached, rate-spaced MusicBrainz/Cover Art Archive/LRCLIB enrichment and protected manual edits.
- Background folder watching in the desktop and headless host, plus the existing atomic downloader inbox.
- Existing authenticated host, profile isolation, Argon2id credentials, expiring/revocable sessions, byte ranges/HEAD, stale-revision rejection, owner metadata boundaries and verified offline copying.

## Verification boundary

Windows native regressions and generated-media UI/playback checks are the local evidence. Physical speaker/headphone output and switching between actual hardware devices need listening review on that hardware. Live metadata-service availability is recorded separately from deterministic matching/cache tests.

The source supports Linux/macOS and a headless build, but native execution on those operating systems is not established by a Windows build or a supplied CI file. Android/iOS clients, system media-key/tray support, crossfade/equalizer DSP, third-party server adapters, online playlist-provider imports and production deployment/load testing remain further work.

The existing portable packages are unsigned. Microsoft Defender results, if present in the delivery evidence, apply to the scanned executable only. Protected Bitdefender/Malwarebytes retesting and signing remain release gates. Runtime license texts and pinned provenance/source links accompany the package.
