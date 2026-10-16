# Repository and collaboration

Canonical repository: https://github.com/Slulisee/SSMP

SSMP's original source is GPL-3.0-or-later. The `SSYTDLP/` folder contains the tested optional handoff integration and retains its MIT license. The independently maintained downloader repository is https://github.com/Slulisee/SSYTDLP. This initial SSMP publication does not update that repository.

The repository tracks source, assets, specifications, maintenance documentation, dependency locks and CI. Build outputs, downloaded playback engines, generated media/libraries, private configuration and workstation verification reports stay out of Git. Native dependency source/license pointers are in THIRD-PARTY.md.

Windows execution was verified locally for milestone 0.1. The CI workflow checks a headless Linux host and desktop builds on Windows, Linux and macOS; check the actual run results before treating a platform as verified. Mobile clients and public binary release preparation remain roadmap work.

Public releases will need their own version tag, changelog, provenance/checksums and the outstanding distribution checks in RELEASE-CHECKLIST.md. Creating this source repository does not publish the local portable ZIPs as a release.
