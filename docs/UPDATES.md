# Install and update SSMP

For Windows, run `SSMP-Windows-AMD64-Setup.exe`. Setup installs for your current Windows user under `%LOCALAPPDATA%/Programs/SSMP`, adds a Start menu entry and registers an uninstaller. No Python or Rust installation is needed. Choose an existing SSMP program folder to upgrade it.

The library and settings remain in `%LOCALAPPDATA%/SSMP`, or the explicit `--data` location you chose. Media remains in its own folders. Setup/uninstall removes only its app-file list and empty folders; it does not recursively delete music, libraries or other files placed alongside the app.

## Automatic updates

Settings → App updates includes **Download updates and install after closing**. It is enabled by default, matching the selected update policy. Checks contact only this project's public stable GitHub releases, about once a day. The app downloads an installer in a worker and checks its published size and SHA-256 before staging it. Playback continues.

Close SSMP to apply a ready update silently. The installer waits until all SSMP windows close before replacing program files. It does not reopen the player automatically. Disabling the preference keeps the existing manual check/download controls and prevents an automatic handoff. A staged installer is verified again before launch. An incomplete, corrupt or wrong-repository download is never installed.

App updates do not enable song/artist metadata queries. Those have their own separate preference and explicit buttons. Offline playback works without an update connection. The current Windows build is unsigned.

Portable ZIPs remain available. Keep their full runtime folder together. Linux/macOS desktop and mobile package status is listed separately; Windows setup does not update those systems. Docker hosts update by replacing their container image, as explained in [DOCKER.md](DOCKER.md).

## Rollback

Keep the previous portable version and a library backup. The delivered rollback snapshot restores the previous source/executable in a separate selected copy and preserves library/media files. Use the exact commands in the verification record. Do not replace an installation while its player is running.

The downloader's current name is SSMD (formerly SSYTDLP). Its own setup and update channel stay in the existing [downloader repository](https://github.com/Slulisee/SSYTDLP); its queue, settings, engines and media are separate from SSMP.
