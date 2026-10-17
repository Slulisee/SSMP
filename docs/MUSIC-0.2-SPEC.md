# SSMP 0.2 — music player release

Product name: **Slulisee's Simple Music Player**. This release replaces the series/video-library interface with a music library. Existing files and database records remain intact; local music videos stay playable with an immediate picture/audio-only toggle.

## Accepted scope

- English, German, Spanish and French throughout navigation, settings, dialogs and player actions. Persistent language selection.
- Seeking previews during dragging and commits one accurate seek on release while playback continues. Previous restarts after three seconds; within three seconds it selects the previous track. Next advances normally. Shuffle is persistent, never starts playback by itself, avoids immediate repeats, and optionally weights favorite artists.
- Correct mpv asynchronous event handling: distinguish cancelled/unavailable startup operations from actual file failures; reset errors on successful load. Preserve useful, dismissible failure details.
- A compact music interface inspired by the supplied references, with the downloader's Ocean, Carbon, Forest, Nord, Frost and warm/light themes. Consistent vector icons, hover states, animated meters/transitions/lyrics, and a reduced-motion switch.
- Artist and album libraries, clickable artist/album names, artist favorites, shuffle artist, and album-to-playlist conversion.
- The song context menu and ellipsis share actions. Add to playlist always offers Create playlist. Playlist art, name and description are editable. Playlist detail includes duration, measured listening time, track count, add-installed-songs dialog, numbered/title/album/date-added/duration columns, searching, sorting and compact/list views. Custom order is persistent and can be rearranged.
- Lyrics focus mode with artwork-derived background, timed active lines and click-to-seek. Full-screen button and Escape exit. Support local LRC and optional LRCLIB lookup; word highlighting only when enhanced timing actually exists.
- Music video and audio-only presentation switch without changing the current position, audio or queue. General series/season screens and subtitle tools leave the UI.
- Local statistics show measured listening seconds/hours, play counts, tracks/artists/genres, daily activity, discovery, favorites and listening concentration. No invented historical hours. History off stops measurement; clear data removes metrics. Genre coverage and missing tags are explicit.
- Optional online enrichment using MusicBrainz, Cover Art Archive and LRCLIB. Disabled by default, runs off the UI thread, observes service limits, caches results and fills missing fields without replacing manual edits. Metadata queries contain title/artist/album, never audio files. Network failure leaves playback usable.
- Select local folders/server sources and an actual enumerated audio output device. No microphone or recording features.
- Optional folder watching discovers new files and file changes without blocking playback. Configurable roots, enable/disable, status and manual scan.
- Preserve local/offline playback, existing authenticated self-hosting, downloads, user isolation and SSYTDLP inbox integration.

## Validation and delivery

Back up the existing source and executable before changes. Run baseline tests, then meaningful regression tests covering seeking, error recovery, shuffle/previous, real listening accounting, metadata matching/non-overwrite, migration, playlist ordering and watched-folder import. Test actual playback and native UI at desktop/compact sizes, all four languages, light/dark themes, lyrics, artist, playlist, statistics and music-video views. Assemble a runnable Windows portable build, test a fresh extraction, keep a focused diff and rollback with hashes, and publish only the SSMP changes to the existing GitHub repository. Changes already present in SSYTDLP belong to another work stream and must be preserved.

Other operating systems remain source-supported; claim platform execution only when performed. Real speakers/headphones and device-switch behavior require interactive verification on that hardware.

## Research

- mpv command/property semantics: https://mpv.io/manual/master/
- MusicBrainz identification and one-request-per-second limit: https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting
- MusicBrainz search: https://musicbrainz.org/doc/MusicBrainz_API/Search
- LRCLIB API: https://lrclib.net/docs
- User feedback: https://www.reddit.com/r/musichoarder/comments/1qtyrx1/what_features_would_you_like_in_a_music_player/
- Reference design screenshots were supplied by the user. They guide layout; they are not redistributed as SSMP artwork.
