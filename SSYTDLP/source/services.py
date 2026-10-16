"""Public catalog metadata imports; audio matching is explicitly from YouTube."""
from difflib import SequenceMatcher
from html.parser import HTMLParser
from urllib.parse import parse_qs, urlencode, urlsplit
from urllib.request import Request, urlopen
import json
import re

from lyrics import clean_title, normalized


def catalog_provider(url):
    host = (urlsplit(url).hostname or '').lower()
    if host == 'open.spotify.com':
        return 'Spotify'
    if host in ('deezer.com', 'www.deezer.com'):
        return 'Deezer'
    if host in ('music.apple.com', 'itunes.apple.com'):
        return 'Apple Music'
    return ''


def get_data(url, as_json=True):
    with urlopen(Request(url, headers={'User-Agent': 'Mozilla/5.0 SSYTDLP/3.0 (public catalog preview)'}), timeout=20) as response:
        data = response.read(8_000_001)
    if len(data) > 8_000_000:
        raise ValueError('Catalog response exceeded the preview size limit.')
    text = data.decode('utf-8-sig')
    return json.loads(text) if as_json else text


class ScriptData(HTMLParser):
    def __init__(self):
        super().__init__()
        self.inside = False
        self.parts = []
    def handle_starttag(self, tag, attrs):
        if tag == 'script' and dict(attrs).get('id') == '__NEXT_DATA__':
            self.inside = True
    def handle_data(self, data):
        if self.inside:
            self.parts.append(data)
    def handle_endtag(self, tag):
        if tag == 'script':
            self.inside = False


def catalog_entry(index, title, artist, album, duration, url, thumbnail=''):
    return {'id': str(index), 'playlist_index': index, 'title': str(title), 'track': str(title),
            'artist': str(artist), 'album': str(album or ''), 'duration': duration,
            'url': url, 'thumbnail': thumbnail, '_ssy_catalog': True}


def catalog_preview(url, cancelled=None):
    provider = catalog_provider(url)
    if provider == 'Spotify':
        title, entries, is_playlist = spotify(url, cancelled)
    elif provider == 'Deezer':
        title, entries, is_playlist = deezer(url, cancelled)
    elif provider == 'Apple Music':
        title, entries, is_playlist = apple(url)
    else:
        raise ValueError('Unknown catalog service.')
    if not entries:
        raise ValueError('This public catalog did not expose any track metadata.')
    if cancelled is not None and cancelled.is_set():
        raise InterruptedError('Preview cancelled.')
    return {'_type': 'playlist' if is_playlist else 'video', 'title': title,
            'entries': entries, '_ssy_catalog': True, 'provider': provider,
            'is_catalog_playlist': is_playlist}


def spotify(url, cancelled=None):
    match = re.search(r'/(track|album|playlist)/([A-Za-z0-9]+)', urlsplit(url).path)
    if not match:
        raise ValueError('Use a public Spotify track, album or playlist link.')
    kind, item_id = match.groups()
    parser = ScriptData()
    parser.feed(get_data(f'https://open.spotify.com/embed/{kind}/{item_id}', as_json=False))
    if not parser.parts:
        raise ValueError('Spotify did not expose public embed metadata for this link.')
    data = json.loads(''.join(parser.parts))
    entity = data.get('props', {}).get('pageProps', {}).get('state', {}).get('data', {}).get('entity', {})
    raw = [entity] if kind == 'track' else entity.get('trackList') or []
    title = entity.get('name') or entity.get('title') or 'Spotify'
    entries = []
    for index, item in enumerate(raw, 1):
        if cancelled is not None and cancelled.is_set():
            raise InterruptedError('Preview cancelled.')
        artists = item.get('artists') or []
        artist = ', '.join(a.get('name', '') for a in artists if isinstance(a, dict)) or item.get('subtitle') or ''
        name = item.get('title') or item.get('name')
        if not name or not artist:
            raise ValueError('Spotify returned a track without title/artist metadata.')
        uri = item.get('uri') or f'spotify:track:{item_id}'
        track_id = str(uri).rsplit(':', 1)[-1]
        duration = item.get('duration') or item.get('durationMs')
        duration = duration / 1000 if isinstance(duration, (float, int)) else None
        images = (item.get('visualIdentity') or entity.get('visualIdentity') or {}).get('image') or []
        thumbnail = images[-1].get('url', '') if images else ''
        album = title if kind == 'album' else item.get('albumName') or ''
        entries.append(catalog_entry(index, name, artist, album, duration, 'https://open.spotify.com/track/' + track_id, thumbnail))
    count = entity.get('trackCount') or entity.get('totalTracks')
    if isinstance(count, int) and count > len(entries):
        title += f' ({len(entries)} of {count} publicly visible tracks)'
    return title, entries, kind != 'track'


def deezer(url, cancelled=None):
    match = re.search(r'/(track|album|playlist)/(\d+)', urlsplit(url).path)
    if not match:
        raise ValueError('Use a public Deezer track, album or playlist link.')
    kind, item_id = match.groups()
    data = get_data(f'https://api.deezer.com/{kind}/{item_id}')
    if data.get('error'):
        raise ValueError('Deezer catalog: ' + str(data['error'].get('message', 'not available')))
    title = data.get('title') or 'Deezer'
    if kind == 'track':
        raw = [data]
    else:
        page = data.get('tracks') or {}
        raw = list(page.get('data') or [])
        seen = set()
        while page.get('next'):
            if cancelled is not None and cancelled.is_set():
                raise InterruptedError('Preview cancelled.')
            next_url = page['next']
            if urlsplit(next_url).hostname != 'api.deezer.com' or next_url in seen:
                raise ValueError('Unexpected Deezer pagination response.')
            seen.add(next_url)
            page = get_data(next_url.replace('http://', 'https://', 1))
            raw.extend(page.get('data') or [])
    entries = []
    for i, item in enumerate(raw, 1):
        album = title if kind == 'album' else (item.get('album') or {}).get('title', 'Singles')
        art = item.get('album') or (data if kind == 'album' else {})
        entries.append(catalog_entry(i, item.get('title_short') or item.get('title', ''), (item.get('artist') or {}).get('name', ''), album, item.get('duration'), f'https://www.deezer.com/track/{item["id"]}', art.get('cover_big') or art.get('cover', '')))
    return title, entries, kind != 'track'


def apple(url):
    parts = urlsplit(url)
    item_id = (parse_qs(parts.query).get('i') or [''])[0]
    track = bool(item_id)
    if not item_id:
        match = re.search(r'/(\d+)$', parts.path)
        if not match:
            raise ValueError('Use a public Apple Music song or album link.')
        item_id = match[1]
        track = '/song/' in parts.path
    country = parts.path.strip('/').split('/')[0]
    country = country if len(country) == 2 else 'us'
    data = get_data('https://itunes.apple.com/lookup?' + urlencode({'id': item_id, 'entity': 'song', 'country': country}))
    songs = [item for item in data.get('results', []) if item.get('wrapperType') == 'track' and item.get('kind') == 'song']
    entries = [catalog_entry(i, item.get('trackName', ''), item.get('artistName', ''), item.get('collectionName', ''), item.get('trackTimeMillis', 0) / 1000, item.get('trackViewUrl', url), item.get('artworkUrl100', '').replace('100x100', '600x600')) for i, item in enumerate(songs, 1)]
    title = songs[0].get('trackName' if track else 'collectionName', 'Apple Music') if songs else 'Apple Music'
    return title, entries, not track


def choose_match(entry, info):
    raw_artist = entry.artist or entry.channel
    title, artist = normalized(clean_title(entry.title)), normalized(raw_artist)
    title_words = set(title.split())
    artist_words = set(artist.split()) - {'feat', 'ft', 'featuring', 'and'}
    artist_parts = re.split(r'\s*(?:,|&|\bfeat\.?|\bft\.?)\s*', raw_artist, flags=re.I)
    primary_artist = normalized(artist_parts[0])
    ranked = []
    for raw in info.get('entries') or []:
        if not isinstance(raw, dict):
            continue
        candidate = normalized(raw.get('title'))
        channel = normalized(raw.get('channel') or raw.get('uploader'))
        candidate_words = set(candidate.split())
        coverage = len(title_words & candidate_words) / max(1, len(title_words))
        artist_coverage = len(artist_words & set((candidate + ' ' + channel).split())) / max(1, len(artist_words))
        collaboration_ok = len(artist_parts) > 1 and primary_artist and (primary_artist in candidate or primary_artist in channel) and artist_coverage >= .8
        artist_ok = bool(artist) and (artist in candidate or artist in channel or collaboration_ok or SequenceMatcher(None, artist, channel.replace(' topic', '')).ratio() >= .8)
        if coverage < .8 or not artist_ok:
            continue
        if any(word in candidate_words - title_words - artist_words for word in ('cover', 'karaoke', 'remix', 'live')):
            continue
        duration = raw.get('duration')
        distance = abs(duration - entry.duration) if isinstance(duration, (int, float)) and entry.duration else 0
        if entry.duration and distance > max(15, entry.duration * .08):
            continue
        link = raw.get('url') or raw.get('webpage_url') or ''
        if not link.startswith('https://'):
            video_id = raw.get('id') or link
            link = 'https://www.youtube.com/watch?v=' + video_id if re.fullmatch(r'[A-Za-z0-9_-]{11}', video_id) else ''
        if urlsplit(link).hostname not in ('www.youtube.com', 'youtube.com', 'music.youtube.com', 'youtu.be'):
            continue
        score = coverage * 100 - distance + (10 if 'audio' in candidate_words or 'topic' in channel.split() else 0)
        ranked.append((score, link, raw.get('title', '')))
    if not ranked:
        raise ValueError(f'No confident YouTube audio match for {entry.artist or entry.channel} — {entry.title}.')
    _, url, matched_title = max(ranked)
    return url, matched_title
