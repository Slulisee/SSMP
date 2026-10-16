# Imports, lyrics and SSYTDLP

## Files and folders

Use **Add a folder**, **+ Files**, or drop paths on the player. Add a folder to remember it for **Settings → Rescan saved folders**. Initial indexing reads embedded tags using ffprobe and falls back to filenames. The UI stays responsive during the scan. Album art comes from adjacent `cover.jpg`, `folder.jpg`, `cover.png`, or an image with the same basename as the media. Extracting embedded artwork is still a release gate.

The initial build leaves media where it is and stores edits in SQLite. Automatic managed-folder moves and tag write-back will use preview/undo controls in a later milestone. No nonfunctional file-management switches are exposed.

## Playlists

Import M3U/M3U8, PLS, XSPF or iTunes library XML from **Playlists**. Relative paths resolve against the playlist directory. `file:` URIs are decoded. Network URLs and unavailable references remain in **Unresolved import entries**; they do not trigger a download. UTF-8 playlists are supported; legacy non-UTF-8 encodings need explicit conversion in milestone 0.1. Playlist duplicates retain their order in the saved playlist; the initial library-based playlist view still needs dedicated reorder controls.

Playlist exporters use M3U8. Extensible importer packages and automatic source-service matching are future modules. Do not install arbitrary code to open an unsupported playlist.

## Lyrics

Place `.lrc` or `.txt` beside a track, with the same basename. Embedded lyrics are read from format tags where ffprobe exposes them. Use the lyrics panel to import text or LRC into the library. Synchronized LRC supports repeated timestamps and the offset tag; clicking a timed line seeks the player. The panel has a temporary timing-offset control.

**Search lyrics online** sends the current title, artist, album and duration to LRCLIB only when clicked. Results are cached in the library; existing lyrics display offline. There is no network requirement for local lyrics. The online provider can be unavailable or lack a matching version; manual import remains available. Live LRCLIB service availability has not been established by the offline test suite.

## Series and season metadata

Choose **Videos & series → Import series / season information**. Review the manifest before applying it. Example:

```json
{
  "version": 1,
  "series": "My Series",
  "season": 2,
  "items": [
    {"path": "episodes/S02E01.mkv", "title": "A New Beginning", "episode": 1},
    {"path": "episodes/S02E02.mkv", "title": "The Journey", "episode": 2}
  ]
}
```

Relative media paths resolve against the manifest location. Item fields override season defaults. Individual items can also supply `artist`, `album`, `series`, `season`, `episode` and `source`. Use the item menu to edit information or watched status. Online series search and richly grouped season pages remain later release gates.

## SSYTDLP

The updated copy is in the workspace's `SSYTDLP` folder. In that app, open **Settings → SSMP** and enable **Add completed downloads to SSMP**. Leave the inbox empty for the normal local SSMP library, or copy the actual inbox path from **SSMP → Settings → Library & SSYTDLP**.

SSYTDLP emits atomic, versioned manifests after completed files and optional lyric processing. SSMP imports them while open. Closed-player handoffs wait on disk until SSMP opens. Processed manifests move to `imports/processed`; incomplete/problematic manifests move to `imports/failed` and retain their source paths. Importing the same completed file again preserves the existing library identity.

This milestone's integration is local/inbox-based. A remote transfer/upload workflow is not implemented yet. For a host, the inbox and referenced media must be accessible to the host's account; the Windows client does not make a local path accessible to another computer automatically.

The supplied SSYTDLP installation outside this workspace is preserved. The updated executable and source in this workspace are a separate copy. Its source keeps the existing MIT license and all independent runtime licenses.
