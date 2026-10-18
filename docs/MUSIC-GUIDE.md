# SSMP 0.2 music guide

SSMP means **Slulisee's Simple Music Player** again. Add a folder or drop music into the window. Playback, playlists, lyrics, statistics and customization work locally. A server and online metadata are optional.

## Listening

The bottom bar keeps transport in the center, volume at the right, and seeking on its own wide rail. Dragging previews a position while music continues; releasing performs one seek. Previous restarts after three seconds. Near the beginning it selects the prior track; a quick second press is a convenient way to go back. Next advances. Shuffle is an on/off mode and does not start or change a song when toggled. Repeat cycles off, all and one; hover it to see its mode.

Click the volume icon to select an output device or mute. The same choices appear under Settings → Playback. System default follows the operating system's default output. Music sources are managed under Sources & server. SSMP does not record a microphone.

Space plays/pauses, Left/Right seek five seconds, L opens lyrics, F11 expands lyrics, and Escape leaves full-screen lyrics. Text entry keeps normal typing shortcuts.

The Windows interface uses cached software drawing by default, avoiding a GPU context for its window. Music keeps playing when minimized. Only visible song rows build their controls and cover images; filtered views are cached. Animations can be disabled in Settings. A generated-media performance check, including 10,001 synthetic library rows, is available as `scripts/music_performance_qa.py`; results describe the tested machine rather than every possible device.

## Artists, albums and playlists

Click an artist under a song to open their music. The artist page offers shuffle and a favorite button. When favorite-artist weighting is enabled, their tracks have three times the selection weight in shuffle; all other queued tracks remain eligible. Immediate repeats are excluded when the queue has alternatives.

Click an album name to browse it. Make a playlist copies that album's track selection into a new editable playlist. The original album remains a library grouping.

Right-click a song or use its three-dot button for the same actions. Add to playlist always starts with Create playlist, including an empty playlist library. New playlists created this way contain the selected song.

A playlist has a name, description, custom picture, track count, total duration and measured listening time **from playing that playlist**. Edit playlist changes its description and picture; clicking the picture opens image selection. Add songs searches music already in the library. More options offers export and deletion. Deleting a playlist keeps music files.

The table shows index, title/artist, album, date added and duration. Date columns collapse in narrow windows. Sort by title, artist, album, date added, release date or duration; the Custom order view preserves membership order. In custom order, a track's menu offers Move up and Move down. Compact view reduces row height. Search can target title, artist, album or genre, and a genre filter appears when tags exist. The animated bars identify the currently playing song, including during searches.

## Lyrics and music videos

Use the lyrics button in the player or click its artwork. A local `.lrc` file beside the audio is read automatically; plain text is also supported. Synced lines follow playback and clicking a timed line seeks there. Enhanced LRC `<mm:ss.xx>` word timestamps enable real word highlighting; ordinary LRC remains line-synced. The timing offset adjusts display synchronization.

Full screen expands the lyrics view; Escape restores the window. Keyboard shortcuts are L for lyrics and F11 for full-screen lyrics, when a text field is not focused. The background uses a blurred version of available cover art. Missing or untimed lyrics are identified clearly. Find lyrics online queries LRCLIB when requested.

Music videos have a dedicated view. The video button switches between picture and audio-only presentation without reloading the music. Edit information → Link music video can pair a local video with an audio track; its picture follows the audio timeline. A paired file should contain the same performance/timing. General series, season, episode and watched-status screens have left the interface. Existing library records and media files are preserved during migration.

## Statistics

Enable Remember listening history and measured statistics to start measuring. The dashboard reports actual listening hours and exact durations, qualified play counts, explored tracks, top songs/artists/genres, recent daily activity, active-day streak and busiest hour. It excludes pauses, buffering, seeks and suspended time. A play qualifies after half the track or thirty seconds, whichever comes first, with a minimum of one second.

The first genre tag supplies each song's genre. Genre coverage shows how much listening has usable tags. Old play counters remain available, but SSMP does not infer historical hours from them. The measurement start date is shown. Daily/hourly charts use UTC. Export my statistics creates JSON. Turning history off pauses measurement; clearing history also clears measured playlist time.

## Customization and metadata

Settings includes sixteen themes shared in spirit and palette with SSYTDLP, custom accent color, scale, compact rows and an animations switch. English, German, Spanish and French can be selected immediately.

Automatically find missing metadata online is off initially. It uses [MusicBrainz](https://musicbrainz.org/doc/MusicBrainz_API), [Cover Art Archive](https://coverartarchive.org/) and [LRCLIB](https://lrclib.net/docs). Queries contain song/artist/album names and duration, never audio uploads. Matching requires title, artist, confidence and compatible duration. Album art/release information is applied only to a matching album or a previously empty album. Requests run in a worker, MusicBrainz calls are spaced, and replies are cached. Missing results and connection failures leave local playback available.

Manual edits are stored in the library and protected during rescans and automatic enrichment. The editor groups track identity, album details, artwork, linked video and lyrics. It changes library information rather than rewriting original media tags.

## Automatic discovery and hosting

Enable Watch my music folders for changes after choosing folders. Scans run in the background about every five seconds. New/changed files appear without restarting playback. Remove source stops watching that folder; existing library items stay available.

A headless host can use `SSMP --data DATA --watch MUSIC_FOLDER --serve --bind 0.0.0.0:4534`. On Linux use the host executable built with `--no-default-features`. The configured folders belong to the host's local settings, separate from remote user profiles. See [HOSTING.md](HOSTING.md) for accounts, LAN/Tailscale and service setup. Linux/macOS/mobile platform status is recorded in [the release checklist](../RELEASE-CHECKLIST.md).


## 0.3: summaries, selection and direct lookups

Statistics has All time, Monthly wrap-up and Yearly wrap-up tabs. Choose the year/month, step through the five story cards and explore the measured rankings and listening calendar below. Make a replay playlist copies up to fifty available ranked songs. Export my statistics includes the selected period and preserves the original measurement fields for all-time exports (JSON schema 2). Periods use UTC. Older daily totals remain, but detailed period song rankings start with 0.3; coverage notes explain any gap.

Select songs enables table checkboxes. Ctrl-click toggles a song and Shift selects a range. Select all visible uses the current table/filter. The toolbar offers checked-field editing, playlist creation/addition, queue additions, online lookup and removal from the current playlist. Batch changes affect only explicitly checked fields; an empty checked field clears its value. Original files remain untouched.

Find information for all songs is in the library header. Individual song menus and Edit information have Find information online; a selection has the same action. These one-time requests work while recurring automatic metadata lookup is off. Progress and cancellation stay visible. Missing recording/artist genres are fetched from MusicBrainz and can be searched/sorted. Nonempty values and manual edits are kept. The owner edits shared server metadata; other profiles can keep their own local lyrics overrides.

Lyrics and Edit information both offer Add lyrics file and Find lyrics online. UTF-8/BOM and UTF-16 LRC/text are accepted. Online results validate title, artist and duration, with a fallback when exact album matching fails. Progress/errors stay on the lyrics page. The queue remains usable beside lyrics; compact windows stack the lyric identity and text. Repeat one shows a 1 in the icon. Interface size previews while dragging and applies after release; compact navigation and a separate transport row support larger scales.

See [UPDATES.md](UPDATES.md) for setup and the selected automatic-close update policy, and [DOCKER.md](DOCKER.md) for the headless host.
