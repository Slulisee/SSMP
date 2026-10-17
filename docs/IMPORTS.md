# Importing music into SSMP

## Files and folders

Choose Add folder, Add files, or drop files into the window. Common audio formats are read by ffprobe/libmpv. Music video files have their own view. Watch my music folders for changes enables background discovery; Sources & server lists configured roots and offers manual scanning.

Embedded title, artist, album, track, genre, release date and lyrics tags are indexed when available. Adjacent `.lrc`/`.txt` lyrics and common cover filenames are recognized. Edit information changes the library record and preserves manual overrides on rescans. Optional online enrichment fills missing information; see [MUSIC-GUIDE.md](MUSIC-GUIDE.md).

## Playlists

Import playlist accepts M3U/M3U8, PLS, XSPF and iTunes XML. It matches local files and keeps unresolved entries in the result. Relative paths resolve against the playlist file. Export playlist creates M3U8. Importing a playlist does not acquire missing music from a streaming service.

To start from an album, open it and choose Make a playlist. To start from one song, use its context menu → Add to playlist → Create playlist. Add songs on a playlist searches files already in the library. Custom order can be changed with Move up/down from the song menu.

## Lyrics

Place `Song.lrc` beside `Song.flac` (or another audio format). `[mm:ss.xx]` timestamps enable line synchronization; enhanced `<mm:ss.xx>` timestamps within a line enable word highlighting. Plain lyrics remain readable. Full-screen lyrics and timing offset are available from the player. Online LRCLIB lookup is optional.

## SSYTDLP handoff

SSYTDLP can write an atomic completed-file manifest to SSMP's `imports/` folder when its optional integration is enabled. Desktop and host consume that inbox without requiring the downloader to remain open. Supplied song/artist/album fields are applied to the indexed files. Processed/failed manifests remain available for diagnosis. The downloader and player remain independently usable.

The 0.1 CLI manifest parser remains readable for existing backups. General series/season management is no longer part of the music interface.
