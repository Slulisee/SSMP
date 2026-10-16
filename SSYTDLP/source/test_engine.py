from pathlib import Path
import hashlib
import json
import tempfile
import unittest
from unittest.mock import patch

import engine
from engine import Options, PRESETS, build_args, load_settings, save_settings, update_nightly, validate_url
from platform_support import executable_name, nightly_asset


class EngineTests(unittest.TestCase):
    def test_urls_are_single_http_arguments(self):
        for value in ('file:///C:/secret', 'javascript:test()', '--exec=whoami', 'https://name:pass@example.com/a', 'https://example.com/a\n--exec=x', 'https://example.com:bad/a', ''):
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_url(value)
        url = 'https://www.youtube.com/watch?v=x&list=y'
        args = build_args(Options(url, 'C:/Media & music'), Path('C:/tools'))
        self.assertEqual(args[-2:], ['--', url])
        self.assertEqual(args[args.index('--paths') + 1], str(Path('C:/Media & music').absolute()))
        self.assertIn('--ignore-config', args)
        self.assertIn('--no-plugin-dirs', args)

    def test_every_preset_has_its_actual_format(self):
        for name, (kind, quality, extension) in PRESETS.items():
            args = build_args(Options('https://example.com/video', 'C:/media', preset=name), Path('tools'))
            if kind == 'audio':
                self.assertEqual(args[args.index('--audio-format') + 1], extension)
                self.assertEqual(args[args.index('--audio-quality') + 1], quality)
            elif quality:
                self.assertIn('height<=' + quality, args[args.index('--format') + 1])
                self.assertEqual(args[args.index('--remux-video') + 1], 'mp4')
            else:
                self.assertEqual(args[args.index('--remux-video') + 1], 'mkv')

    def test_metadata_and_cover_are_independent(self):
        args = build_args(Options('https://example.com/a', '.', metadata=False, cover=True, organize=False), Path('tools'))
        self.assertNotIn('--embed-metadata', args)
        self.assertNotIn('--parse-metadata', args)
        self.assertIn('--embed-thumbnail', args)
        args = build_args(Options('https://example.com/a', '.', metadata=True, cover=False), Path('tools'))
        self.assertIn('--embed-metadata', args)
        self.assertNotIn('--embed-thumbnail', args)

    def test_playlist_order_and_album_fallback(self):
        args = build_args(Options('https://example.com/a', '.', playlist=True), Path('tools'))
        self.assertIn('--yes-playlist', args)
        template = args[args.index('--output') + 1]
        self.assertIn('%(artist)', template)
        self.assertIn('%(album)', template)
        self.assertIn('%(playlist_index)03d', template)
        self.assertIn('%(album,playlist_title|Singles)s:%(album)s', args)
        args = build_args(Options('https://example.com/a', '.', playlist=True, organize=False), Path('tools'))
        self.assertTrue(args[args.index('--output') + 1].startswith('%(playlist_title|Playlist)'))

    def test_archive_is_separate_per_preset(self):
        archives = set()
        for preset in PRESETS:
            args = build_args(Options('https://example.com/a', '.', preset=preset), Path('tools'))
            archives.add(args[args.index('--download-archive') + 1])
        self.assertEqual(len(archives), len(PRESETS))
        args = build_args(Options('https://example.com/a', '.', skip_existing=False), Path('tools'))
        self.assertNotIn('--download-archive', args)

    def test_invalid_destination_or_preset(self):
        for options in (Options('https://example.com', ''), Options('https://example.com', '.', preset='Unknown')):
            with self.assertRaises(ValueError):
                build_args(options, Path('tools'))

    def test_settings_round_trip_and_bad_file(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'data' / 'settings.json'
            self.assertEqual(load_settings(path), {})
            save_settings(path, {'folder': 'C:/Müsik', 'playlist': True})
            self.assertEqual(load_settings(path)['folder'], 'C:/Müsik')
            self.assertFalse(path.with_suffix('.tmp').exists())
            path.write_text('broken')
            self.assertEqual(load_settings(path), {})

    def test_nightly_update_verified_and_keeps_previous(self):
        data = b'verified fixture engine'
        digest = hashlib.sha256(data).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            tools = Path(directory)
            (tools / executable_name('yt-dlp')).write_bytes(b'previous')
            events = []
            with patch.object(engine, '_get', side_effect=[b'{"tag_name":"2026.10.10.233141"}', f'{digest}  {nightly_asset()}\n'.encode(), data]):
                update_nightly(tools, lambda *event: events.append(event))
            self.assertEqual((tools / executable_name('yt-dlp')).read_bytes(), data)
            self.assertEqual((tools / executable_name('yt-dlp.previous')).read_bytes(), b'previous')
            self.assertEqual(json.loads((tools / 'engines.json').read_text())['yt-dlp']['channel'], 'nightly')
            self.assertEqual(events[-1], ('updated', '2026.10.10.233141'))

    def test_bad_checksum_leaves_engine_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            tools = Path(directory)
            (tools / executable_name('yt-dlp')).write_bytes(b'previous')
            events = []
            with patch.object(engine, '_get', side_effect=[b'{"tag_name":"2026.10.10.233141"}', ('0' * 64 + '  ' + nightly_asset() + '\n').encode(), b'bad']):
                update_nightly(tools, lambda *event: events.append(event))
            self.assertEqual((tools / executable_name('yt-dlp')).read_bytes(), b'previous')
            self.assertEqual(events[-1][0], 'update_error')
            self.assertFalse((tools / 'yt-dlp.download').exists())


if __name__ == '__main__':
    unittest.main()
