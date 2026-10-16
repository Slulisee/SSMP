# Integration research and implementation

Research and live checks: 16 October 2026.

| Project / API | Finding and use |
| --- | --- |
| [SongSync](https://github.com/Lambada10/SongSync) | Android reference for timed lyric files and embedded lyrics; credits syncedlyrics. Its Android UI/code was not copied. |
| [syncedlyrics](https://github.com/moehmeni/syncedlyrics) | MIT Python project. SSYTDLP integrates its LRCLIB provider and HTTP session, with stricter identity/duration/content checks and fallback when the by-ID endpoint fails. Version 1.0.1 is pinned. |
| [LRCLIB](https://github.com/tranxuanthang/lrclib), [API docs](https://lrclib.net/docs) | Public lyric records with plain and timed text. Live lookup, timed LRC, provenance and embedded lyrics were tested. Short/bad community records are rejected. |
| [Mutagen](https://github.com/quodlibet/mutagen) | Metadata editing without re-encoding audio. Tested lyric embedding while preserving artist tags in MP3, M4A, Opus, FLAC and MP4. Version 1.48.1 is pinned. |
| [spotDL](https://github.com/spotDL/spotify-downloader) | Confirms the catalog-to-YouTube approach used by Spotify download tools. SSYTDLP implements public catalog imports and conservative YouTube matching in its existing queue instead of adding a separate spotDL application. |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp), [supported sites](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md) | Existing nightly remains the downloader. Direct public SoundCloud media and YouTube media were tested. Subtitle flags produce SRT sidecars and optional embedded video streams. |
| [Odesli / Songlink docs](https://github.com/songlink/docs/blob/master/api-v1-alpha.1.md) | Evaluated cross-service mapping. The tested API call returned HTTP 401, so the shipped implementation does not depend on it. |

## Service behavior and live evidence

| Service | Implemented route | Live result |
| --- | --- | --- |
| YouTube / YouTube Music | Direct yt-dlp nightly | YouTube MP3, timed lyrics, embedded USLT and real subtitles passed. Music URLs use the same extractor; account-gated sources were not tested. |
| SoundCloud | Direct available public streams | Forss — Flickermood public HLS stream downloaded and converted successfully. An account-only original was not requested. |
| Spotify | Public track/album/playlist embed metadata → YouTube audio match | Rick Astley track preview and matched MP3 passed; Global Warming album (18 tracks) and Today's Top Hits playlist (50 tracks) previewed successfully. |
| Deezer | Public catalog API, including album/playlist pagination → YouTube match | Daft Punk track preview and matched MP3 passed; Discovery album (14 tracks) previewed successfully. |
| Apple Music | Public iTunes catalog track/album lookup → YouTube match | Coldplay — Yellow catalog preview passed. The route shares the tested matcher; a separate Apple-origin download was not run. Apple Music playlists are not implemented. |
| Bandcamp and other yt-dlp sites | Direct yt-dlp support when the site exposes media | Existing engine capability; not every site was tested in this session. |
| Other subscription catalogs, including Tidal, Amazon Music and Qobuz | No dedicated catalog importer in this release | Not advertised as native full-track download support. Individual supported public pages still use yt-dlp's available extractors. |

Spotify's public embed can expose fewer tracks than the full source collection.
If an explicit total is present, the preview labels the publicly visible subset.
Private collections, authentication/DRM-restricted audio and paywall access are
not implemented. The personal-use disclaimer does not change those boundaries.

## Bugs found and fixed

- Missing yt-dlp metadata fields originally emitted bare `NA`, breaking JSON.
  The lyric handoff now emits JSON `null` for missing fields.
- The LRCLIB exact-duration and by-ID endpoints intermittently returned 503.
  Bounded retries, validated search fallback and reuse of validated response
  text keep optional enrichment from breaking a completed download.
- Short incorrect community lyrics were found in a real lookup. Content,
  identity and duration checks reject those records.
- A changed lyrics/subtitle option could otherwise hit the old download archive
  and skip enrichment. Extras profiles now have their own archive keys, while
  existing media files remain protected from overwrite.
- Windows-only executable names and folder opening were replaced with host
  helpers. macOS/Linux runtime verification remains an explicit release gate.
- Catalog details are stored beside the original queue-entry fields so the
  previous release can still parse the saved queue after a rollback.

Licenses for bundled Python dependencies are included under `licenses/`.
No SongSync or spotDL source code is incorporated; their workflows informed
the implementation and are credited here.
