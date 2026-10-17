# Changelog

## 0.2.0 — Music player release

- Restore the name Slulisee's Simple Music Player; keep music videos and remove general series/video UI.
- Fix seek dragging, previous/restart behavior, shuffle activation and asynchronous playback errors.
- Add artist/album navigation, favorite-artist shuffle weighting, full-screen synced/word-timed lyrics and video/audio switching.
- Add playlist pictures/descriptions, song/album creation, local-song selection, date/duration columns, sorting/custom order, shared context menus and measured playlist time.
- Add measured statistics, genre coverage, activity, export and privacy controls.
- Add English/German/Spanish/French, sixteen themes, vector icons and reduced motion.
- Add optional cached metadata enrichment, real output-device selection and desktop/headless folder watching.
- Resolve the reported Windows desktop slowdown with cached CPU window presentation; bound redraws, virtualize song rows, cache filtering and resolve only the selected queued track.


## Unreleased

### Interface

- Center playback around the Play/Pause button with consistent control sizes and spacing.
- Anchor volume at the right edge of the bottom player.
- Give seeking a separate row spanning almost the entire window width, with elapsed and total time at its ends.
- Center page headings, search controls, home content, album cards and form columns; make collection statistics equal in size.
- Adapt album grids and player actions to narrower windows. Keep server/settings navigation in a fixed sidebar footer and scroll the remaining navigation when necessary.
- Center the video frame, metadata and controls, preserving the frame's aspect ratio.

### Branding

- Expand SSMP to **Slulisee's Simple Media Player**, keeping the abbreviation and executable name.

## 0.1 development milestone

- Add the local music/video player, lyrics, playlists, themes, metadata editing, optional authenticated host, offline copying and SSYTDLP handoff.
- Document the implemented scope and outstanding release gates in RELEASE-CHECKLIST.md.
