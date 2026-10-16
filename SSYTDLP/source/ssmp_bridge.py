"""Versioned, atomic completed-file handoff; no dependency on a running SSMP UI."""
from pathlib import Path
import json
import os
import sys
import uuid

EXTENSIONS = {'.mp3', '.flac', '.wav', '.ogg', '.opus', '.m4a', '.aac', '.wma', '.aiff',
              '.alac', '.ape', '.mp4', '.mkv', '.webm', '.avi', '.mov', '.m4v', '.ts', '.mpg', '.mpeg', '.flv'}


def default_inbox() -> Path:
    if os.environ.get('SSMP_DATA_DIR'):
        return Path(os.environ['SSMP_DATA_DIR']) / 'imports'
    if sys.platform == 'win32':
        return Path(os.environ.get('LOCALAPPDATA', str(Path.home() / 'AppData/Local'))) / 'SSMP/imports'
    if sys.platform == 'darwin':
        return Path.home() / 'Library/Application Support/SSMP/imports'
    return Path(os.environ.get('XDG_DATA_HOME', str(Path.home() / '.local/share'))) / 'ssmp/imports'


def publish(paths, destination='', metadata=None, source='') -> Path | None:
    items = []
    metadata = metadata or {}
    for raw in dict.fromkeys(str(p) for p in paths):
        path = Path(raw).expanduser().resolve(strict=True)
        if not path.is_file() or path.suffix.lower() not in EXTENSIONS:
            raise ValueError('SSMP only imports completed media files: ' + str(path))
        info = metadata.get(raw, metadata.get(str(path), {}))
        items.append({'path': str(path), 'title': str(info.get('track') or info.get('title') or ''),
                      'artist': str(info.get('artist') or ''), 'album': str(info.get('album') or ''),
                      'source': source})
    if not items:
        return None
    inbox = Path(destination).expanduser() if destination.strip() else default_inbox()
    inbox.mkdir(parents=True, exist_ok=True)
    target = inbox / ('ssytdlp-' + uuid.uuid4().hex + '.json')
    temporary = target.with_suffix('.tmp')
    try:
        with temporary.open('x', encoding='utf-8') as output:
            json.dump({'version': 1, 'producer': 'SSYTDLP', 'items': items}, output, ensure_ascii=False, indent=2)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)
    return target
