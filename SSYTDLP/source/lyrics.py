"""Conservative LRCLIB matching using syncedlyrics' community provider and Mutagen."""
from dataclasses import dataclass
from difflib import SequenceMatcher
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import unicodedata

from syncedlyrics.providers.lrclib import Lrclib


def normalized(text):
    return ' '.join(re.sub(r'[^\w\s]', ' ', unicodedata.normalize('NFKC', str(text or '')).casefold()).split())


def clean_title(text):
    text = re.sub(r'\s*[\[(](?:official[^\])]*|lyrics?[^\])]*|[48]k[^\])]*|hd|hq|audio|visuali[sz]er)[\])]', '', str(text), flags=re.I)
    return text.strip()


def identity(info):
    title = info.get('track') or info.get('title') or ''
    artist = info.get('artist') or ''
    if title in ('NA', 'None'):
        title = info.get('title') or ''
    if ' - ' in title and (not artist or normalized(artist) in ('unknown artist', 'na') or normalized(artist) == normalized(info.get('uploader'))):
        artist, title = title.split(' - ', 1)
    artist = re.sub(r'(?:\s*-\s*Topic|VEVO)$', '', str(artist), flags=re.I).strip()
    return clean_title(title), artist


def valid_lyrics(plain, synced):
    plain = str(plain or '').strip()
    synced = str(synced or '').strip()
    plain_ok = len(plain) >= 80 and len(plain.splitlines()) >= 4
    timed = re.findall(r'\[(\d{1,3}):(\d{2})(?:[.:]\d+)?\]', synced)
    synced_ok = len(synced) >= 100 and len(timed) >= 4 and any(int(m) * 60 + int(s) >= 10 for m, s in timed)
    return plain if plain_ok else '', synced if synced_ok else ''


@dataclass
class LyricsResult:
    id: int
    plain: str
    synced: str
    source: str


class LyricsFinder:
    def __init__(self):
        self.provider = Lrclib()
        self.provider.session.headers.update({'User-Agent': 'SSYTDLP/3.0 (personal desktop lyrics client)'})
        from requests.adapters import HTTPAdapter
        from urllib3.util.retry import Retry
        self.provider.session.mount('https://lrclib.net/', HTTPAdapter(max_retries=Retry(total=2, backoff_factor=.5, status_forcelist=[429, 500, 502, 503, 504], respect_retry_after_header=False)))

    def find(self, info, cancelled=None):
        title, artist = identity(info)
        if not title or not artist or normalized(artist) in ('unknown artist', 'na', 'none'):
            return None
        duration = info.get('duration')
        duration = duration if isinstance(duration, (int, float)) and duration > 0 else None
        params = {'track_name': title, 'artist_name': artist}
        if duration:
            params['duration'] = duration
        candidates = []
        try:
            response = self.provider.session.get('https://lrclib.net/api/get', params=params)
            if response.ok and isinstance(response.json(), dict):
                candidates.append(response.json())
        except Exception:
            # The signature endpoint may fail for music videos whose duration
            # includes an intro. Search still validates duration before accepting.
            pass
        if cancelled is not None and cancelled.is_set():
            return None
        # A short/incorrect community record must not block better search results.
        good = [item for item in candidates if self.accept(item, title, artist, duration)]
        if not good:
            response = self.provider.session.get('https://lrclib.net/api/search', params={'track_name': title, 'artist_name': artist})
            response.raise_for_status()
            data = response.json()
            candidates = data if isinstance(data, list) else []
            good = [item for item in candidates if self.accept(item, title, artist, duration)]
        if not good:
            return None
        best = min(good, key=lambda item: (0 if item.get('syncedLyrics') else 1, abs(float(item.get('duration') or 0) - (duration or 0))))
        if cancelled is not None and cancelled.is_set():
            return None
        # Integrate syncedlyrics' LRCLIB adapter for the selected, validated record.
        try:
            result = self.provider.get_lrc_by_id(str(best['id']))
        except Exception:
            result = None
        # The signature/search response already contains validated lyrics. Keep
        # them when the community adapter's optional by-ID endpoint is unavailable.
        plain, synced = valid_lyrics(result.unsynced if result else best.get('plainLyrics'), result.synced if result else best.get('syncedLyrics'))
        if not plain and synced:
            plain = re.sub(r'\[[^\]]+\]', '', synced).strip()
        return LyricsResult(int(best['id']), plain, synced, f'https://lrclib.net/api/get/{best["id"]}') if plain or synced else None

    @staticmethod
    def accept(item, title, artist, duration):
        if not isinstance(item, dict) or item.get('instrumental'):
            return False
        if SequenceMatcher(None, normalized(title), normalized(item.get('trackName'))).ratio() < .90:
            return False
        if SequenceMatcher(None, normalized(artist), normalized(item.get('artistName'))).ratio() < .85:
            return False
        if duration and (not isinstance(item.get('duration'), (int, float)) or abs(item['duration'] - duration) > max(8, duration * .04)):
            return False
        return any(valid_lyrics(item.get('plainLyrics'), item.get('syncedLyrics')))


def write_new(path, text):
    if path.exists():
        return
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(text, encoding='utf-8')
    os.replace(temporary, path)


def embed_lyrics(path, result):
    import mutagen
    from mutagen.id3 import ID3, USLT
    from mutagen.mp4 import MP4
    temporary = path.with_name(path.stem + '.ssytdlp-tags' + path.suffix)
    try:
        shutil.copy2(path, temporary)
        media = mutagen.File(temporary)
        if media is None:
            return False
        if media.tags is None:
            media.add_tags()
        if isinstance(media.tags, ID3):
            if media.tags.getall('USLT'):
                return True
            media.tags.add(USLT(encoding=3, lang='und', desc='', text=result.plain))
        elif isinstance(media, MP4):
            if media.tags.get('\xa9lyr'):
                return True
            media.tags['\xa9lyr'] = [result.plain]
        else:
            if media.tags.get('lyrics'):
                return True
            media.tags['lyrics'] = [result.plain]
            if result.synced:
                media.tags['syncedlyrics'] = [result.synced]
        media.save()
        os.replace(temporary, path)
        return True
    finally:
        temporary.unlink(missing_ok=True)


def add_lyrics(path, info, embed=True, emit=lambda *_: None, cancelled=None):
    path = Path(path)
    try:
        result = LyricsFinder().find(info, cancelled)
        if cancelled is not None and cancelled.is_set():
            return
        if result is None:
            emit('log', '[Lyrics] No confident lyrics match found; media download is complete.')
            return
        if result.synced:
            write_new(path.with_suffix('.lrc'), result.synced + '\n')
        if result.plain:
            write_new(path.with_suffix('.lyrics.txt'), result.plain + '\n')
        write_new(path.with_suffix('.lyrics.json'), json.dumps({'provider': 'LRCLIB via syncedlyrics', 'record_id': result.id, 'source': result.source, 'synced': bool(result.synced), 'plain_sha256': hashlib.sha256(result.plain.encode()).hexdigest()}, indent=2))
        embedded = embed_lyrics(path, result) if embed else False
        emit('log', f'[Lyrics] Saved lyrics for {path.name}; synced={bool(result.synced)}; embedded={embedded}; LRCLIB id={result.id}')
    except Exception as exc:
        emit('log', f'[Lyrics] Lyrics enrichment did not finish: {exc}. The media file was retained.')
