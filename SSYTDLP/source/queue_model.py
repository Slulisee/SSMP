"""Playlist selection, persistent queue and sequential batch execution."""
from __future__ import annotations
from dataclasses import asdict, dataclass, field, replace
from pathlib import Path
from typing import Callable
import json
import os
import shutil
import threading
import uuid

from engine import Download, Options, Preview, validate_url


@dataclass
class MediaEntry:
    index: int
    title: str
    channel: str = ''
    duration: float | None = None
    thumbnail: str = ''
    available: bool = True
    url: str = ''
    artist: str = ''
    album: str = ''


@dataclass
class MediaSource:
    url: str
    title: str
    is_playlist: bool
    entries: list[MediaEntry]
    kind: str = 'direct'
    provider: str = ''


def normalize_preview(url: str, info: dict) -> MediaSource:
    catalog = bool(info.get('_ssy_catalog'))
    playlist = bool(info.get('is_catalog_playlist')) if catalog else info.get('_type') in ('playlist', 'multi_video') or isinstance(info.get('entries'), list)
    raw_entries = info.get('entries') or [] if playlist or catalog else [info]
    entries = []
    used = set()
    for position, raw in enumerate(raw_entries, 1):
        raw = raw if isinstance(raw, dict) else {}
        index = raw.get('playlist_index')
        index = index if type(index) is int and index > 0 else position
        if index in used:
            raise ValueError('The playlist contains duplicate positions. Reload the preview.')
        used.add(index)
        title = str(raw.get('title') or raw.get('id') or f'Video {index}')
        available = bool(raw) and raw.get('availability') not in ('private', 'needs_auth', 'premium_only', 'subscriber_only') and title.lower() not in ('[private video]', '[deleted video]') and raw.get('_type') != 'playlist'
        thumbnail = raw.get('thumbnail') or next((t.get('url') for t in reversed(raw.get('thumbnails') or []) if isinstance(t, dict) and t.get('url')), '')
        duration = raw.get('duration')
        entries.append(MediaEntry(index, title, str(raw.get('artist') or raw.get('channel') or raw.get('uploader') or ''), duration if isinstance(duration, (int, float)) else None, str(thumbnail), available, str(raw.get('url') or ''), str(raw.get('artist') or ''), str(raw.get('album') or '')))
    if not entries or not any(entry.available for entry in entries):
        raise ValueError('No available videos were found in this source.')
    return MediaSource(validate_url(url), str(info.get('title') or entries[0].title), playlist, entries, 'catalog' if catalog else 'direct', str(info.get('provider') or ''))


@dataclass
class QueueRecord:
    source: MediaSource
    selected: list[int]
    id: str = field(default_factory=lambda: uuid.uuid4().hex)
    status: str = 'queued'

    def validate(self) -> None:
        allowed = {entry.index for entry in self.source.entries if entry.available}
        if not self.selected or any(type(i) is not int or i not in allowed for i in self.selected):
            raise ValueError('Select at least one available video.')
        self.selected = sorted(set(self.selected))

    def options(self, defaults: Options) -> Options:
        self.validate()
        return replace(defaults, url=self.source.url, playlist=self.source.is_playlist,
                       playlist_items=tuple(self.selected) if self.source.is_playlist else ())


class QueueStore:
    def __init__(self, path: Path):
        self.path = path

    def save(self, records: list[QueueRecord]) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        items = [asdict(record) for record in records]
        # Keep the original entry shape readable by prior SSYTDLP builds.
        # New catalog details live alongside it and survive normal v3 reloads.
        for item in items:
            source = item['source']
            details = {}
            for entry in source['entries']:
                details[str(entry['index'])] = {key: entry.pop(key) for key in ('url', 'artist', 'album')}
            source['entry_details'] = details
        data = json.dumps({'version': 1, 'items': items}, ensure_ascii=False, indent=2)
        temporary = self.path.with_suffix('.tmp')
        temporary.write_text(data, encoding='utf-8')
        if self.path.exists():
            try:
                previous = json.loads(self.path.read_text('utf-8'))
                if previous.get('version') == 1 and isinstance(previous.get('items'), list):
                    shutil.copy2(self.path, self.path.with_suffix('.bak'))
            except (ValueError, AttributeError):
                pass
        os.replace(temporary, self.path)

    def load(self) -> list[QueueRecord]:
        for path in (self.path, self.path.with_suffix('.bak')):
            try:
                data = json.loads(path.read_text('utf-8'))
                if data.get('version') != 1 or not isinstance(data.get('items'), list):
                    continue
                records = []
                ids = set()
                for raw in data['items']:
                    source = raw['source']
                    details = source.get('entry_details') or {}
                    entries = [MediaEntry(**(e | details.get(str(e['index']), {}))) for e in source['entries']]
                    provider = source.get('provider', '')
                    if not provider:
                        from services import catalog_provider
                        provider = catalog_provider(source['url'])
                    entry = MediaSource(validate_url(source['url']), str(source['title']), bool(source['is_playlist']), entries, source.get('kind', 'catalog' if provider else 'direct'), provider)
                    status = raw.get('status', 'queued')
                    status = status if status in ('queued', 'completed', 'error', 'cancelled') else 'queued'
                    record = QueueRecord(entry, raw['selected'], str(raw['id']), status)
                    record.validate()
                    if record.id in ids:
                        raise ValueError('Duplicate queue ID')
                    ids.add(record.id)
                    records.append(record)
                return records
            except (OSError, ValueError, TypeError, KeyError, AttributeError):
                continue
        return []


class Batch:
    def __init__(self, tools: Path, emit: Callable[[str, object], None], download_factory=Download):
        self.tools, self.emit = tools, emit
        self.download_factory = download_factory
        self.cancelled = threading.Event()
        self.active = None
        self._lock = threading.Lock()

    def cancel(self) -> None:
        self.cancelled.set()
        with self._lock:
            if self.active is not None:
                self.active.cancel()

    def run(self, records: list[QueueRecord], defaults: Options) -> None:
        successes = errors = files = 0
        try:
            for record in records:
                if self.cancelled.is_set():
                    break
                self.emit('queue_status', {'id': record.id, 'status': 'downloading'})
                final = {}
                def receive(kind, value):
                    if kind == 'done':
                        final.update(value)
                    else:
                        self.emit(kind, value)
                try:
                    if record.source.kind == 'catalog':
                        final.update(self.run_catalog(record, defaults))
                    else:
                        options = record.options(defaults)
                        with self._lock:
                            if self.cancelled.is_set():
                                self.emit('queue_status', {'id': record.id, 'status': 'cancelled'})
                                break
                            self.active = self.download_factory(self.tools, receive)
                        self.active.run(options)
                except Exception as exc:
                    self.emit('log', str(exc))
                    final = {'status': 'error', 'files': 0}
                finally:
                    with self._lock:
                        self.active = None
                status = final.get('status', 'error')
                status = 'completed' if status == 'success' else status
                self.emit('queue_status', {'id': record.id, 'status': status})
                successes += status == 'completed'
                errors += status == 'error'
                files += final.get('files', 0)
                if status == 'cancelled':
                    break
        finally:
            self.emit('batch_done', {'completed': successes, 'errors': errors, 'files': files, 'cancelled': self.cancelled.is_set()})

    def run_catalog(self, record, defaults):
        from services import choose_match
        files = errors = 0
        for entry in record.source.entries:
            if entry.index not in record.selected:
                continue
            if self.cancelled.is_set():
                return {'status': 'cancelled', 'files': files}
            try:
                with self._lock:
                    if self.cancelled.is_set():
                        return {'status': 'cancelled', 'files': files}
                    self.active = Preview(self.tools, self.emit)
                info = self.active.search(entry.artist or entry.channel, entry.title)
                url, title = choose_match(entry, info)
                self.emit('log', f'[Catalog] {record.source.provider}: {entry.title} -> YouTube audio: {title} ({url})')
                options = replace(defaults, url=url, playlist=False, playlist_items=(), catalog_title=entry.title,
                                  catalog_artist=entry.artist or entry.channel, catalog_album=entry.album,
                                  catalog_number=entry.index if record.source.is_playlist else 0)
                result = {}
                def receive(kind, value):
                    if kind == 'done':
                        result.update(value)
                    else:
                        self.emit(kind, value)
                with self._lock:
                    if self.cancelled.is_set():
                        return {'status': 'cancelled', 'files': files}
                    self.active = self.download_factory(self.tools, receive)
                self.active.run(options)
                files += result.get('files', 0)
                if result.get('status') == 'cancelled':
                    return {'status': 'cancelled', 'files': files}
                errors += result.get('status') != 'success'
            except Exception as exc:
                if self.cancelled.is_set():
                    return {'status': 'cancelled', 'files': files}
                errors += 1
                self.emit('log', '[Catalog] ' + str(exc))
        return {'status': 'error' if errors else 'success', 'files': files}
