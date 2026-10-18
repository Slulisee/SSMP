# SSMP 0.3 feedback release

This update addresses the October 18 report and continues the desktop/full-version foundation. The previous working CPU-rendered Windows interface remains the default.

## Accepted behavior

- Statistics supports all-time, calendar-month and calendar-year views. Monthly/yearly views include a five-part wrap-up, measured time, qualified plays, explored tracks/artists/albums, active days, longest streak, new tracks, daily averages, top song share, genre coverage, busiest day/hour, weekday/hour patterns, and track/artist/album/genre/playlist/decade rankings. A replay playlist can be created from the ranking.
- Every number comes from recorded listening. Older daily totals remain available. Detailed period rankings start with 0.3; missing historical breakdowns are identified rather than reconstructed from play counts. Calendar boundaries and hourly activity use UTC consistently with existing statistics.
- JSON export keeps the original all-time measurement fields and adds a versioned period report.
- Repeat one has a visible 1 inside its repeat symbol. Off/all/one remain separate states.
- The queue can stay open beside lyrics, including at compact sizes. Lyrics actions and feedback stay visible on that page.
- Add lyrics file is available on the lyrics page and editor. UTF-8/BOM and UTF-16 LRC/text are supported. Existing synced and enhanced-LRC behavior remains.
- Online lyric lookup reports progress, success and failure. Exact lookup falls back to a title/artist search validated against identity and duration. Late replies preserve manual edits.
- Missing information can be searched explicitly for all songs, a selection, or one song, independently of the recurring metadata preference. Work remains paced, cached and cancellable, and failures do not stop the rest of the batch. Curated MusicBrainz recording/artist genres remain searchable and sortable.
- Multi-selection supports checkboxes, modifier/range selection, quick editing of explicitly checked fields, playlist creation/addition, queue additions, metadata lookup and playlist removal. Original files are not rewritten.
- Interface size previews while dragging and applies after release. The search controls use a common row height and vertical alignment. Editor and other dialog titles use compact typography.
- Windows has a per-user installer, explicit file-list uninstall and SHA-256-verified stable updates. The selected default checks in the background and installs after closing, with an off switch. The downloader receives the same automatic-close policy while keeping its existing installer and manual updater.
- Docker/Compose provides the headless SSMP host with a persistent data volume, read-only music folder, non-root runtime, health check and documented container update/backup steps.

## Validation and remaining platform work

Use the accompanying verification record for actual commands, native checks, API results, installer behavior, package identity and rollback. A Windows build does not establish native macOS/Linux desktop or mobile behavior. Docker build/health execution is reported separately. Advanced DSP/crossfade, native media-key/tray integration, additional server adapters and mobile clients remain on the full-version roadmap.
