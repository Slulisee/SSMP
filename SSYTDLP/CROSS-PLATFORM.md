# Windows, macOS and Linux

The GUI and download layer now choose executable names, process handling,
folder opening and configuration paths for the host OS. The same source can
be installed on Windows, macOS and Linux. The supplied Windows executable is
the native build tested on this Windows machine. Native macOS and Linux GUI
execution and packaging have not been tested here; the source, setup scripts
and build workflow are provided for verification on those systems.

## Windows

Extract the portable ZIP into a fresh folder and launch `SSYTDLP.exe`. Keep its
`_internal` and `tools` folders beside the executable. All Python dependencies, FFmpeg,
ffprobe, Deno and yt-dlp nightly are included in the Windows package.

Settings: `%LOCALAPPDATA%\SSYTDLP`.

## macOS

Use the cross-platform source ZIP, not the Windows package.

1. Install Python 3.12 with Tkinter. The python.org installer includes Tkinter;
   Homebrew Python users should also install the matching `python-tk` package.
2. Install FFmpeg (`brew install ffmpeg` if using Homebrew).
3. Extract the source archive. Run `bash Run.command` from that folder.

The launcher creates a local `.venv`, installs the pinned Python requirements,
and provisions the official yt-dlp nightly plus Deno, verifying their published
checksums. FFmpeg is discovered in PATH, `/opt/homebrew/bin`, `/usr/local/bin` or
`/usr/bin`. No system stable yt-dlp is silently substituted.

Settings: `~/Library/Application Support/SSYTDLP`.

To build a native `.app` on a Mac:

```sh
source .venv/bin/activate
python -m pip install -r source/requirements-build.txt
python source/build.py
```

The result is `dist/SSYTDLP.app`. FFmpeg remains an external dependency on
macOS/Linux. The app is unsigned; distribution signing/notarization is not
configured. Build it on the target CPU architecture.

## Linux

Use Python 3.12, Tkinter, venv support and FFmpeg. On Debian/Ubuntu the required
system packages are `python3`, `python3-venv`, `python3-tk` and `ffmpeg` (use
your distribution's package manager).

```sh
bash Run.sh
```

The launcher provisions the verified nightly and Deno as on macOS. An X11 or
Wayland desktop session with Tk support is required. `xdg-open` opens the output
folder. Settings use `$XDG_CONFIG_HOME/SSYTDLP`, or `~/.config/SSYTDLP`.

For a native Linux executable, install the build requirements in the venv and
run `python source/build.py`. Build on the oldest Linux distribution you intend
to support because PyInstaller executables depend on the host's libc baseline.

## Native build workflow

The source ZIP includes `.github/workflows/build.yml` for Windows, macOS and
Linux builds, plus `source/package_native.py` to preserve executable permissions
in the build archives. The workflow is prepared, not run in this session.
Native builds must be produced on their target OS; a Windows executable is
not a macOS/Linux binary. The Windows package's old rollback executables are
not included in the cross-platform source archive.

CI builds do not provision FFmpeg. Install it on the target system or stage
the appropriate FFmpeg/ffprobe binaries before packaging. The downloadable
Windows portable ZIP from this release already includes those binaries.

## Maintenance

- `platform_support.py`: executable paths, host configuration paths, URL-free
  native folder opening and nightly asset selection.
- `prepare_tools.py`: official nightly/Deno provisioning and SHA256 checks.
- `engine.py`: shell-free processes, subtitles, extras archive profiles.
- `lyrics.py`: community provider integration, matching and metadata embedding.
- `services.py`: public Spotify/Deezer/Apple catalogs and YouTube match scoring.
- `queue_model.py`: source types, selection persistence and batch execution.
- `i18n.py`: four-language UI, settings text and disclaimer.
