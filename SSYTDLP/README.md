# SSYTDLP

**Slulisee's Simple yt-dlp Downloader** · made by Slulisee

This build adds optional **Settings → SSMP** completed-download handoff. See [SSMP-INTEGRATION.md](SSMP-INTEGRATION.md). Keep the new `_internal` folder with the EXE; this update uses folder packaging.

A small Windows x64 GUI using bundled **yt-dlp nightly**, FFmpeg and Deno.
Extract the whole ZIP, then double-click **SSYTDLP.exe**. No installation or
Python runtime is required. Keep the `tools` and `_internal` folders beside the executable.

macOS/Linux source, setup and native build scripts are included in the separate
cross-platform source ZIP. See [CROSS-PLATFORM.md](CROSS-PLATFORM.md) for dependencies
and the explicit native testing limits.

## Lyrics, subtitles and music services

The two new main-window toggles enable optional lyrics and subtitles.

- **Download lyrics · LRC + tags** searches LRCLIB via the community `syncedlyrics`
  provider. It saves timed `.lrc` when available, plain `.lyrics.txt`, and a
  `.lyrics.json` record-source note. Plain lyrics can also be embedded in MP3,
  M4A, Opus, FLAC and MP4 tags. MKV keeps the lyric sidecars. Existing lyric
  sidecars/tags are preserved. No confident match leaves the media intact.
- **Download subtitles** writes SRT files beside the media. Open Settings →
  Lyrics & subtitles to choose languages, include automatic captions, or also
  embed subtitles in MP4/MKV videos. Audio presets keep SRT files separately.
- Lyrics/subtitles default off. Switching them on uses a separate extras archive
  profile, so existing media can be enriched without overwriting its audio.
  To retry a provider failure, edit the completed queue selection and clear
  **Skip previously downloaded items** for that attempt.
- **SoundCloud, Bandcamp, YouTube / YouTube Music** and supported public media
  sites use yt-dlp directly.
- **Spotify, Deezer and Apple Music** links import public catalog metadata and
  match a recording on YouTube when downloading. This is not direct Spotify,
  Deezer or Apple Music audio. The preview identifies the catalog provider and
  the log shows the actual audio source. Track/album import is supported; public
  Spotify/Deezer playlists also use the existing selection UI. Apple Music
  playlist import is not included. Public Spotify embeds may expose a subset.

See [RESEARCH.md](RESEARCH.md) for evaluated repositories, a service capability
table, real examples and bug fixes. The translated Settings → Services & personal
use tab and [DISCLAIMER.md](DISCLAIMER.md) contain the requested personal-use
notice. A private-use label does not grant copying rights or restricted access.

## Preview, select, queue, download

1. Paste a video or playlist link and click **Preview / Add** (or press Enter).
   This reads the source information without downloading its video or audio.
2. In the preview, check the videos you want. Click the checkbox, double-click
   a row, or select rows and press Space. **Select all**, **Select none** and
   **Invert** work on available entries. Unavailable entries stay visible.
   Selecting a row shows its title, channel, duration and thumbnail when supplied.
3. Click **Add to queue**. Add more links in the same way. Nothing downloads yet.
4. Choose the preset, folder, metadata, cover and sorting options for this batch.
   Click **Download queue** when ready.

The queue runs one source at a time. **Edit selection** (or double-click a queue
row) reopens its cached preview. **Remove** deletes selected links from the queue;
**Clear completed** removes finished rows. After an error the next source still
runs. Start the queue again to retry failed/cancelled items; completed sources
are skipped unless you edit their selection to queue them again.

Playlist choices use the original playlist positions and preserve their numbering
in the downloaded filenames. If the source playlist has changed or been reordered
since previewing it, remove and add the link again to refresh its contents.

Queued links, selected positions and statuses are saved after changes. Interrupted
sources reopen queued. **Cancel** stops the active download and retains partial
files and pending sources. Start again with the same folder and preset to resume.

## Presets and tags

- 1080p MP4, 720p MP4, or best-quality MKV video.
- MP3 320 kbps, M4A 256 kbps, Opus, or FLAC audio.
- Optional embedded metadata and cover / thumbnail.
- Optional Artist / Album folder sorting and duplicate skipping per preset.

Resolution is capped at the selected quality; a smaller source stays smaller.
Audio conversions depend on source quality. FLAC cannot restore lost detail.
Existing output files are preserved. Duplicate records use `.yt-dlp-simple` in
the destination, retaining compatibility with the first version.

Source tags supply title, artist, album and track number. Missing artist falls
back to channel/uploader, and missing album falls back to playlist title or
`Singles`. These values also fill missing embedded tags when metadata is enabled.
The source thumbnail supplies cover art when dedicated album artwork is absent.
There is no external music database lookup.

## Settings

Click **⚙ Settings** for:

- **English, Deutsch, Español, Français**: app labels, buttons, dialogs and status
  messages are translated. Video titles and raw yt-dlp diagnostics stay as supplied.
- **Light / Dark**: changes the controls, tables, text fields and dialog styling.
- **Custom wallpaper**: choose PNG, JPEG, WebP or BMP. The image is copied into
  SSYTDLP's settings folder; the original is preserved. It appears behind the
  header and around the solid download panel, with tinting for legibility.
  Wallpaper can be removed independently of the theme.

Settings apply immediately. On shorter displays, the main panel scrolls so the
download controls remain accessible. Queue lists also have their own scrollbar.

Settings, wallpaper and queue live in `%LOCALAPPDATA%\SSYTDLP`. On first launch,
the app reads the previous version's folder/preset preferences if present, while
leaving its files intact. Queue URLs are saved so your staged batch survives a
restart. No background service or scheduled task is installed.

## Nightly engine

The package includes **yt-dlp nightly 2026.10.10.233141**, FFmpeg / ffprobe 9.0.2
essentials and Deno 2.9.7. **Update nightly** uses the official nightly repository,
checks the published SHA256 and retains one previous engine after replacement.
Downloads and previews always use the bundled engine, independent of any stable
version installed on the PC. Engine provenance is in `tools/engines.json`.

## Source, checks and rollback

The maintainable Python/Tkinter source is in `source`. Pillow handles wallpaper
and preview images. With Python 3.12, run `source\Build.ps1` to rebuild.

`VERIFICATION.txt` records actual checks and their limits. The build is unsigned
and was tested on Windows x64. Private/account-gated sources and every supported
website were not tested.

`ROLLBACK.sh` (Git Bash, with Python installed) restores the source and executable from before SSMP integration using the included `rollback/before-ssmp` snapshot. It leaves tools, queued links, settings and media intact.
For example: `bash ROLLBACK.sh .` from the extracted SSYTDLP folder.

The checkbox update gives checked options a blue fill and white mark, with
empty outlined boxes for unchecked options in both themes. To update an
existing copy, close SSYTDLP and extract this package into a fresh folder, keeping `SSYTDLP.exe`, `_internal` and `tools` together. The queue, settings and wallpaper remain in their per-user data directory.

Upstream: [yt-dlp](https://github.com/yt-dlp/yt-dlp),
[nightly builds](https://github.com/yt-dlp/yt-dlp-nightly-builds/releases),
[FFmpeg source/build information](https://www.gyan.dev/ffmpeg/builds/),
[Deno](https://github.com/denoland/deno/releases/tag/v2.9.7).
