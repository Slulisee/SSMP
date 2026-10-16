from pathlib import Path
import json
import tempfile
import unittest
from unittest.mock import Mock, patch

from engine import Options, build_args
from lyrics import LyricsFinder, clean_title, identity, valid_lyrics
from platform_support import config_directory, executable_name, nightly_asset, tool_path
from queue_model import MediaEntry, QueueRecord, QueueStore, normalize_preview
from services import catalog_provider, catalog_preview, choose_match, deezer, spotify


PLAIN = '\n'.join(['An original fixture line for testing lyric metadata only.'] * 5)
SYNCED = '\n'.join(f'[00:{i * 5:02d}.00] Original fixture line {i} for automated tests.' for i in range(5))
RECORD = dict(id=12, trackName='Fixture Song', artistName='Fixture Artist', duration=120, plainLyrics=PLAIN, syncedLyrics=SYNCED)


class FeatureTests(unittest.TestCase):
    def test_lyrics_quality_and_identity(self):
        self.assertEqual(valid_lyrics('test', '[00:00]test'), ('', ''))
        self.assertTrue(all(valid_lyrics(PLAIN, SYNCED)))
        self.assertTrue(LyricsFinder.accept(RECORD, 'Fixture Song', 'Fixture Artist', 122))
        self.assertFalse(LyricsFinder.accept(RECORD, 'Fixture Song', 'Different Artist', 120))
        self.assertFalse(LyricsFinder.accept(RECORD, 'Fixture Song', 'Fixture Artist', 180))
        self.assertEqual(identity({'title': 'Fixture Artist - Fixture Song (Official Video)', 'artist': 'Fixture Artist', 'uploader':'Fixture Artist'}), ('Fixture Song', 'Fixture Artist'))

    def test_lyrics_recovers_from_exact_and_by_id_failures(self):
        with patch('lyrics.Lrclib') as provider:
            provider.return_value.session.get.side_effect = [OSError('temporary failure'), Mock(json=lambda: [RECORD], raise_for_status=lambda: None)]
            provider.return_value.get_lrc_by_id.return_value = None
            result = LyricsFinder().find({'track': 'Fixture Song', 'artist': 'Fixture Artist', 'duration': 120})
            self.assertEqual(result.id, 12)
            self.assertEqual(result.synced, SYNCED)

    def test_new_extras_archive_and_subtitle_arguments(self):
        base = build_args(Options('https://example.com/a', '.'), Path('tools'))
        args = build_args(Options('https://example.com/a', '.', lyrics=True, subtitles=True, automatic_subtitles=True, embed_subtitles=True, subtitle_languages='en.*,de.*'), Path('tools'))
        for flag in ('--write-subs', '--write-auto-subs', '--embed-subs', '--convert-subs'):
            self.assertIn(flag, args)
        self.assertIn('|null)j', next(v for v in args if v.startswith('after_move:MEDIA:')))
        self.assertNotEqual(base[base.index('--download-archive')+1], args[args.index('--download-archive')+1])
        audio = build_args(Options('https://example.com/a', '.', preset='MP3 audio · 320 kbps', subtitles=True, embed_subtitles=True), Path('tools'))
        self.assertNotIn('--embed-subs', audio)
        with self.assertRaises(ValueError):
            build_args(Options('https://example.com', '.', subtitles=True, subtitle_languages='en\n--exec'), Path('tools'))

    def test_match_excludes_wrong_artist_live_and_duration(self):
        entry = MediaEntry(1, 'Fixture Song', 'Fixture Artist', 120, artist='Fixture Artist')
        candidates = [{'id':'abcdefghijk','title':'Fixture Artist - Fixture Song Official Audio','channel':'Fixture Artist - Topic','duration':121},
                      {'id':'12345678901','title':'Fixture Artist - Fixture Song live','channel':'Fixture Artist','duration':120},
                      {'id':'12345678902','title':'Different Artist - Fixture Song','channel':'Different Artist','duration':120}]
        self.assertIn('abcdefghijk', choose_match(entry, {'entries':candidates})[0])
        with self.assertRaises(ValueError):
            choose_match(entry, {'entries':candidates[1:]})
        collaboration = MediaEntry(1,'Fixture Song','First Artist, Second Artist',120,artist='First Artist, Second Artist')
        chosen = {'id':'abcdefghijk','title':'First Artist - Fixture Song feat. Second Artist','channel':'First Artist - Topic','duration':120}
        self.assertIn('abcdefghijk',choose_match(collaboration,{'entries':[chosen]})[0])

    def test_spotify_embed_catalog_without_session_token(self):
        entity = {'name':'Fixture Song','artists':[{'name':'Fixture Artist'}],'duration':120000,'id':'abc','uri':'spotify:track:abc'}
        html = '<script id="__NEXT_DATA__">'+json.dumps({'props':{'pageProps':{'state':{'data':{'entity':entity}}}}})+'</script>'
        with patch('services.get_data', return_value=html):
            info = catalog_preview('https://open.spotify.com/track/abc')
        source = normalize_preview('https://open.spotify.com/track/abc', info)
        self.assertEqual(source.kind, 'catalog')
        self.assertFalse(source.is_playlist)
        self.assertEqual(source.entries[0].duration, 120)
        self.assertEqual(source.provider, 'Spotify')
        with tempfile.TemporaryDirectory() as directory:
            store = QueueStore(Path(directory)/'queue.json')
            store.save([QueueRecord(source,[1])])
            payload = json.loads(store.path.read_text('utf-8'))
            # Previous releases construct MediaEntry directly from these fields.
            self.assertEqual(set(payload['items'][0]['source']['entries'][0]), {'index','title','channel','duration','thumbnail','available'})
            recovered = store.load()[0]
            self.assertEqual(recovered.source.entries[0].artist,'Fixture Artist')
            self.assertEqual(recovered.source.kind,'catalog')

    def test_deezer_album_pagination(self):
        track = {'id':1,'title':'One','artist':{'name':'Artist'},'duration':120}
        album = {'title':'Album','tracks':{'data':[track],'next':'https://api.deezer.com/album/1/tracks?index=1'}}
        with patch('services.get_data', side_effect=[album,{'data':[track | {'id':2,'title':'Two'}]}]):
            title, entries, is_playlist = deezer('https://www.deezer.com/album/1')
        self.assertEqual(len(entries),2)
        self.assertEqual(entries[1]['playlist_index'],2)
        self.assertTrue(is_playlist)

    def test_platform_paths_and_release_assets(self):
        home = Path('/home/example')
        self.assertEqual(config_directory('Linux',{},home),home/'.config/SSYTDLP')
        self.assertEqual(config_directory('Darwin',{},home),home/'Library/Application Support/SSYTDLP')
        self.assertEqual(config_directory('Windows',{'LOCALAPPDATA':'local'},home),Path('local/SSYTDLP'))
        self.assertEqual(nightly_asset('Darwin','arm64'),'yt-dlp_macos')
        self.assertEqual(nightly_asset('Linux','aarch64'),'yt-dlp_linux_aarch64')
        self.assertEqual(nightly_asset('Linux','x86_64'),'yt-dlp_linux')
        self.assertEqual(executable_name('deno','Linux'),'deno')
        self.assertEqual(executable_name('deno','Windows'),'deno.exe')
        with tempfile.TemporaryDirectory() as directory:
            with patch('platform_support.platform.system',return_value='Linux'):
                p=Path(directory)/'yt-dlp'
                p.write_text('fixture')
                self.assertEqual(tool_path(Path(directory),'yt-dlp'),p.resolve())

    def test_catalog_provider_host_validation(self):
        self.assertEqual(catalog_provider('https://open.spotify.com/track/abc'),'Spotify')
        self.assertEqual(catalog_provider('https://open.spotify.com.evil.example/track/abc'),'')


if __name__ == '__main__':
    unittest.main()
