# Components and source provenance

SSMP original code is GPL-3.0-or-later. SSYTDLP original code remains MIT. Both permit commercial redistribution subject to their license terms; there is no no-resale clause.

| Component | Included runtime / source |
|---|---|
| SSMP Rust dependencies | Exact versions in Cargo.lock; copied license/notice texts and package repository/license metadata in `licenses/rust/` in the Windows package. Rebuild with Cargo and a stable Rust compiler. |
| Windows graphics | Cached CPU painting through egui_software_backend 0.0.2 and softbuffer/GDI is the default. Optional wgpu 27 / DirectX 12 and OpenGL paths remain. These crates, constify, strength_reduce, naga, gpu-allocator and Windows bindings are included in the locked dependency notices. |
| libmpv | shinchiro Windows x86_64 dev build `20261016`, mpv commit `a2cd83c56d`. Archive SHA-256 `0142058a8d20930f1c086d0c16a593f9e27a8b3fe9d1f4bb3b2ee4a2d5ec994b`, matched to the upstream release digest. |
| ffprobe | Existing Gyan FFmpeg 9.0.2 essentials build, commit `946fcce07b`. Its supplied GPL license and build/source README are retained in `licenses/ffmpeg/`. |
| libunwind.dll | LLVM-MinGW `20261006-ucrt-x86_64` runtime, with its supplied Apache-2.0/LLVM-exception license. |
| SSYTDLP package | Python/Tkinter, PyInstaller, Pillow, mutagen, syncedlyrics and existing nightly yt-dlp, FFmpeg/ffprobe and Deno. Original engine provenance and third-party notices are retained in its `tools/engines.json` and `licenses/`. |

Playback runtime: [mpv source at the selected commit](https://github.com/mpv-player/mpv/tree/a2cd83c56d), [Windows build recipes at the release tag](https://github.com/shinchiro/mpv-winbuild-cmake/tree/20261016), [download and digest](https://github.com/shinchiro/mpv-winbuild-cmake/releases/tag/20261016). The mpv GPL license text is copied with the package.

Metadata runtime: [FFmpeg source at the supplied commit](https://github.com/FFmpeg/FFmpeg/commit/946fcce07b), [Gyan build information](https://www.gyan.dev/ffmpeg/builds/).

Compiler runtime: [LLVM-MinGW release](https://github.com/mstorsjo/llvm-mingw/releases/tag/20261006), [build/source repository](https://github.com/mstorsjo/llvm-mingw).

`scripts/package_music.py` refreshes license files from the exact local dependency sources and records runtime hashes. The source archive matches the reviewed Git tree, including source/assets/docs/scripts and Cargo.lock. The Windows package separately carries the original source/executable snapshot and rollback script. Cargo dependencies are fetched at the locked versions when rebuilding. Portable binaries are local development deliveries; a complete corresponding-source audit covering statically included media-library dependencies and protected antivirus retesting/signing remain public-binary-release work.
