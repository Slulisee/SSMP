# SSMP music specification

Product: **Slulisee's Simple Music Player (SSMP)**. Updated direction agreed 2026-10-16.

The current accepted release specification is [Music 0.2](docs/MUSIC-0.2-SPEC.md). The [music guide](docs/MUSIC-GUIDE.md) explains the resulting workflows, and [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) distinguishes executed validation from later platforms.

The product is a simple local music player with optional self-hosting. Offline use, straightforward setup, useful customization, reliable playback and personal control are core requirements. Music videos accompany music; general video/series management has left the product direction. Existing files and library records remain recoverable.

Local use needs no account. Hosted users have independent preferences, playlists, favorites and listening metrics on a shared library. Linux remains important for a headless host. Windows desktop is the verified local delivery target; Linux/macOS desktop and Android/iOS require their own native validation and packaging.

Metadata and online lyrics are optional services. Automatic lookup is opt-in and manual edits take precedence. Statistics measure real listening, support an off switch and never estimate unknown past hours. English, German, Spanish and French are included. Themes, motion, density, output devices and source folders are configurable.

Original SSMP code uses GPL-3.0-or-later with no noncommercial/resale exception. SSYTDLP remains independently usable and retains its own license. The SSYTDLP integration is an optional completed-file handoff.
