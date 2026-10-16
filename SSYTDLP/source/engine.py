"""Small, shell-free yt-dlp nightly process layer. Python standard library only."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Callable
from urllib.parse import urlsplit
from urllib.request import Request, urlopen
import hashlib
import json
import os
import re
import subprocess
import threading

from platform_support import executable_name, nightly_asset, tool_args, tool_path


APP_NAME = 'SSYTDLP'
APP_FULL_NAME = "Slulisee's Simple yt-dlp Downloader"
NIGHTLY_REPO = 'https://api.github.com/repos/yt-dlp/yt-dlp-nightly-builds/releases/latest'
PRESETS = {
    '1080p video · MP4': ('video', '1080', 'mp4'),
    '720p video · MP4': ('video', '720', 'mp4'),
    'Best video · MKV': ('video', '', 'mkv'),
    'MP3 audio · 320 kbps': ('audio', '320K', 'mp3'),
    'M4A audio · 256 kbps': ('audio', '256K', 'm4a'),
    'Opus audio · best': ('audio', '0', 'opus'),
    'FLAC audio': ('audio', '0', 'flac'),
}


@dataclass(frozen=True)
class Options:
    url: str
    folder: str
    preset: str = '1080p video · MP4'
    playlist: bool = False
    metadata: bool = True
    cover: bool = True
    organize: bool = True
    skip_existing: bool = True
    playlist_items: tuple[int, ...] = ()
    lyrics: bool = False
    embed_lyrics: bool = True
    subtitles: bool = False
    subtitle_languages: str = 'en.*,de.*'
    automatic_subtitles: bool = False
    embed_subtitles: bool = False
    catalog_title: str = ''
    catalog_artist: str = ''
    catalog_album: str = ''
    catalog_number: int = 0
    ssmp_enabled: bool = False
    ssmp_inbox: str = ''


def validate_url(value: str) -> str:
    value = value.strip()
    if any(c.isspace() or ord(c) < 32 for c in value):
        raise ValueError('Paste one complete YouTube or media URL.')
    try:
        url = urlsplit(value)
        if url.scheme not in ('https', 'http') or not url.hostname:
            raise ValueError('Use a complete https:// or http:// URL.')
        if url.username is not None or url.password is not None:
            raise ValueError('URLs containing a username or password are not supported.')
        _ = url.port
    except (ValueError, UnicodeError) as exc:
        raise ValueError('Use a valid http:// or https:// URL without login details.') from exc
    return value


def process_kwargs() -> dict:
    return {'creationflags': subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == 'nt' else {'start_new_session': True}


def build_args(options: Options, tools: Path) -> list[str]:
    url = validate_url(options.url)
    if options.preset not in PRESETS:
        raise ValueError('Choose a download preset.')
    if not options.folder.strip():
        raise ValueError('Choose a download folder.')
    folder = Path(options.folder).expanduser().absolute()
    kind, quality, extension = PRESETS[options.preset]
    args = [
        '--ignore-config', '--no-plugin-dirs', '--newline', '--no-color', '--no-quiet',
        '--encoding', 'utf-8', '--windows-filenames', '--trim-filenames', '220',
        '--no-overwrites', '--no-post-overwrites', '--continue',
        '--no-write-playlist-metafiles', '--socket-timeout', '30', '--retries', '5',
        '--paths', str(folder), '--progress', '--progress-delta', '0.3',
        '--progress-template', 'download:PROGRESS:%(progress)j',
        '--print', 'before_dl:ITEM:%(title)j',
        '--print', 'after_move:FILE:%(filepath)j', '--no-simulate',
        '--yes-playlist' if options.playlist else '--no-playlist',
    ]
    args += tool_args(tools)
    if options.playlist_items:
        if not options.playlist:
            raise ValueError('Playlist selection requires a playlist URL.')
        args += ['--playlist-items', compact_indices(options.playlist_items)]
    # Preserve source music tags; fill gaps with uploader / playlist context.
    if options.metadata or options.organize or options.catalog_title:
        for rule in (
            '%(artist,creator,uploader|Unknown artist)s:%(artist)s',
            '%(album,playlist_title|Singles)s:%(album)s',
            '%(album_artist,artist|Unknown artist)s:%(album_artist)s',
            '%(track_number,playlist_index|)s:(?P<track_number>[0-9]*)',
        ):
            args += ['--parse-metadata', rule]
    if options.catalog_title:
        args += ['--parse-metadata', '%(track,title)s:%(track)s']
        for field, value in [('title', options.catalog_title), ('track', options.catalog_title), ('artist', options.catalog_artist), ('album_artist', options.catalog_artist), ('album', options.catalog_album)]:
            if value:
                args += ['--replace-in-metadata', field, '(?s)^.*$', value.replace('\\', '\\\\')]
        if options.catalog_number:
            args += ['--parse-metadata', f'{int(options.catalog_number)}:%(track_number)s']
    if options.organize:
        template = '%(artist).70B/%(album).70B/'
    elif options.playlist:
        template = '%(playlist_title|Playlist).100B/'
    else:
        template = ''
    if options.playlist:
        template += '%(playlist_index)03d - '
    elif options.catalog_number:
        template += '%(track_number)03d - '
    template += '%(title).140B [%(id)s].%(ext)s'
    args += ['--output', template]
    if kind == 'audio':
        args += ['--format', 'bestaudio/best', '--extract-audio', '--audio-format', extension, '--audio-quality', quality]
    elif quality:
        args += [
            '--format', f'bv*[height<={quality}][ext=mp4]+ba[ext=m4a]/b[height<=?{quality}][ext=mp4]/bv*[height<={quality}]+ba/b[height<=?{quality}]',
            '--format-sort', 'vcodec:h264', '--merge-output-format', 'mp4', '--remux-video', 'mp4',
        ]
    else:
        args += ['--format', 'bv*+ba/b', '--merge-output-format', 'mkv', '--remux-video', 'mkv']
    if options.metadata:
        args += ['--embed-metadata', '--no-embed-info-json']
    if options.cover:
        args += ['--embed-thumbnail', '--convert-thumbnails', 'jpg']
    if options.lyrics or options.ssmp_enabled:
        fields = ('filepath', 'title', 'track', 'artist', 'album', 'duration', 'uploader')
        args += ['--print', 'after_move:MEDIA:{' + ','.join('"' + field + '":%(' + field + '|null)j' for field in fields) + '}']
    if options.subtitles:
        if not re.fullmatch(r'[A-Za-z0-9_.*,\-]+', options.subtitle_languages):
            raise ValueError('Use subtitle language codes such as en.*,de.* or all,-live_chat.')
        args += ['--write-subs', '--sub-langs', options.subtitle_languages, '--sub-format', 'srt/vtt/best', '--convert-subs', 'srt']
        if options.automatic_subtitles:
            args += ['--write-auto-subs']
        if options.embed_subtitles and kind == 'video':
            args += ['--embed-subs']
    if options.skip_existing:
        key = ('video-' + (quality or 'best') + '-' + extension) if kind == 'video' else extension
        if options.lyrics or options.subtitles:
            # A new extras profile can enrich existing media without downloading it again.
            import hashlib
            profile = json.dumps([options.lyrics, options.embed_lyrics, options.subtitles, options.subtitle_languages, options.automatic_subtitles, options.embed_subtitles])
            key += '-extras-' + hashlib.sha256(profile.encode()).hexdigest()[:10]
        args += ['--download-archive', str(folder / '.yt-dlp-simple' / (key + '.txt'))]
    return args + ['--', url]


class Download:
    def __init__(self, tools: Path, emit: Callable[[str, object], None]):
        self.tools = tools
        self.emit = emit
        self.cancelled = threading.Event()
        self._process: subprocess.Popen | None = None
        self._lock = threading.Lock()

    def cancel(self) -> None:
        self.cancelled.set()
        with self._lock:
            process = self._process
            if process is not None and process.poll() is None:
                if os.name == 'nt':
                    subprocess.run(['taskkill.exe', '/PID', str(process.pid), '/T', '/F'], capture_output=True, **process_kwargs())
                else:
                    import signal
                    try:
                        os.killpg(process.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass

    def run(self, options: Options) -> None:
        completed = 0
        ssmp_paths = []
        ssmp_metadata = {}
        try:
            args = build_args(options, self.tools)
            for name in ('yt-dlp', 'ffmpeg', 'ffprobe'):
                tool_path(self.tools, name)
            folder = Path(options.folder).expanduser().absolute()
            folder.mkdir(parents=True, exist_ok=True)
            if options.skip_existing:
                (folder / '.yt-dlp-simple').mkdir(exist_ok=True)
            with self._lock:
                if self.cancelled.is_set():
                    self.emit('done', {'status': 'cancelled', 'files': 0})
                    return
                self._process = subprocess.Popen(
                    [str(tool_path(self.tools, 'yt-dlp')), *args], stdin=subprocess.DEVNULL,
                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                    encoding='utf-8', errors='replace', **process_kwargs(),
                )
                process = self._process
            assert process.stdout is not None
            for line in process.stdout:
                line = line.strip()
                if not line:
                    continue
                try:
                    if line.startswith('PROGRESS:'):
                        self.emit('progress', json.loads(line[9:]))
                        continue
                    if line.startswith('ITEM:'):
                        self.emit('item', json.loads(line[5:]))
                        continue
                    if line.startswith('FILE:'):
                        completed += 1
                        path = json.loads(line[5:])
                        ssmp_paths.append(path)
                        self.emit('file', path)
                        continue
                    if line.startswith('MEDIA:') and (options.lyrics or options.ssmp_enabled):
                        info = json.loads(line[6:])
                        ssmp_metadata[info['filepath']] = info
                        if options.lyrics and not self.cancelled.is_set():
                            from lyrics import add_lyrics
                            add_lyrics(info['filepath'], info, options.embed_lyrics, self.emit, self.cancelled)
                        continue
                except (ValueError, TypeError):
                    pass
                self.emit('log', line)
            process.stdout.close()
            code = process.wait()
            status = 'cancelled' if self.cancelled.is_set() else 'success' if code == 0 else 'error'
            if options.ssmp_enabled and ssmp_paths:
                try:
                    from ssmp_bridge import publish
                    manifest = publish(ssmp_paths, options.ssmp_inbox, ssmp_metadata, options.url)
                    if manifest:
                        self.emit('log', '[SSMP] Completed files added to inbox: ' + str(manifest))
                except Exception as exc:
                    self.emit('log', '[SSMP] Download completed, but library handoff failed: ' + str(exc))
            self.emit('done', {'status': status, 'files': completed, 'exit_code': code})
        except Exception as exc:
            self.emit('log', str(exc))
            self.emit('done', {'status': 'cancelled' if self.cancelled.is_set() else 'error', 'files': completed})


def _get(url: str) -> bytes:
    with urlopen(Request(url, headers={'User-Agent': 'SSYTDLP/2.0'}), timeout=45) as response:
        return response.read()


def update_nightly(tools: Path, emit: Callable[[str, object], None]) -> None:
    """Fixed upstream, pinned release URL, published SHA256, atomic replacement."""
    temporary = tools / 'yt-dlp.download'
    try:
        release = json.loads(_get(NIGHTLY_REPO))
        tag = release['tag_name']
        if not re.fullmatch(r'[0-9.]+', tag):
            raise ValueError('Unexpected nightly release tag.')
        prefix = f'https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/download/{tag}/'
        asset = nightly_asset()
        checksums = _get(prefix + 'SHA2-256SUMS').decode('utf-8')
        match = re.search(r'^([a-fA-F0-9]{64})\s+\*?' + re.escape(asset) + r'\s*$', checksums, re.M)
        if not match:
            raise ValueError('The release did not include a checksum for this platform.')
        expected = match[1].lower()
        tools.mkdir(parents=True, exist_ok=True)
        target = tools / executable_name('yt-dlp')
        if target.exists() and hashlib.sha256(target.read_bytes()).hexdigest() == expected:
            emit('updated', tag)
            return
        emit('log', f'Downloading official nightly {tag}...')
        data = _get(prefix + asset)
        if hashlib.sha256(data).hexdigest() != expected:
            raise ValueError('The nightly download failed checksum verification. Try updating again.')
        temporary.write_bytes(data)
        # Keep a single verified previous engine for manual recovery.
        if target.exists():
            import shutil
            shutil.copy2(target, tools / executable_name('yt-dlp.previous'))
        if os.name != 'nt':
            temporary.chmod(0o755)
        os.replace(temporary, target)
        manifest = tools / 'engines.json'
        entries = json.loads(manifest.read_text('utf-8')) if manifest.exists() else {}
        entries['yt-dlp'] = {'version': tag, 'channel': 'nightly', 'sha256': expected, 'source': prefix + asset}
        manifest.write_text(json.dumps(entries, indent=2), encoding='utf-8')
        emit('updated', tag)
    except Exception as exc:
        temporary.unlink(missing_ok=True)
        emit('update_error', str(exc))


def load_settings(path: Path) -> dict:
    try:
        value = json.loads(path.read_text('utf-8'))
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


def save_settings(path: Path, values: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(values, indent=2), encoding='utf-8')
    os.replace(temporary, path)


def compact_indices(indices: tuple[int, ...]) -> str:
    """Use original playlist positions, in compact yt-dlp inclusive ranges."""
    if any(type(i) is not int or i < 1 for i in indices):
        raise ValueError('Playlist positions must be positive integers.')
    numbers = sorted(set(indices))
    if not numbers:
        raise ValueError('Select at least one video.')
    runs = []
    start = previous = numbers[0]
    for number in numbers[1:] + [None]:
        if number is not None and number == previous + 1:
            previous = number
            continue
        runs.append(str(start) if start == previous else f'{start}:{previous}')
        start = previous = number
    return ','.join(runs)


class Preview(Download):
    """Read titles, durations and thumbnails without downloading any media."""
    def fetch(self, url: str) -> dict:
        from services import catalog_provider, catalog_preview
        if catalog_provider(url):
            if self.cancelled.is_set():
                raise InterruptedError('Preview cancelled.')
            return catalog_preview(url, self.cancelled)
        return self._extract(validate_url(url))

    def search(self, artist, title):
        return self._extract('ytsearch5:' + artist + ' ' + title + ' official audio')

    def _extract(self, query):
        args = [str(tool_path(self.tools, 'yt-dlp')), '--ignore-config', '--no-plugin-dirs',
                '--flat-playlist', '--dump-single-json', '--skip-download', '--yes-playlist',
                '--encoding', 'utf-8', '--no-color', '--socket-timeout', '25', '--retries', '1',
                *tool_args(self.tools), '--', query]
        with self._lock:
            if self.cancelled.is_set():
                raise InterruptedError('Preview cancelled.')
            self._process = subprocess.Popen(args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                             stderr=subprocess.PIPE, text=True, encoding='utf-8',
                                             errors='replace', **process_kwargs())
            process = self._process
        output, errors = process.communicate()
        if self.cancelled.is_set():
            raise InterruptedError('Preview cancelled.')
        if not output.strip():
            raise ValueError(errors.strip()[-3000:] or 'The source did not return media information.')
        try:
            result = json.loads(output)
        except ValueError as exc:
            raise ValueError(errors.strip()[-3000:] or 'Invalid media preview response.') from exc
        if not isinstance(result, dict):
            raise ValueError('Invalid media preview response.')
        return result
