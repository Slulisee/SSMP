from dataclasses import replace
from pathlib import Path
import json
import os
import tempfile
import unittest
from unittest.mock import patch

from engine import Options, build_args
from ssmp_bridge import publish, default_inbox


class SSMPTests(unittest.TestCase):
    def test_atomic_completed_file_manifest_and_metadata(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            media = base / 'song.flac'
            media.write_bytes(b'completed fixture')
            target = publish([media, media], str(base / 'inbox'), {str(media): {'track': 'Title', 'artist': 'Artist'}}, 'https://example.com/song')
            data = json.loads(target.read_text(encoding='utf-8'))
            self.assertEqual(data['version'], 1)
            self.assertEqual(len(data['items']), 1)
            self.assertEqual(data['items'][0]['title'], 'Title')
            self.assertEqual(data['items'][0]['artist'], 'Artist')
            self.assertEqual(data['items'][0]['path'], str(media.resolve()))
            self.assertFalse(list((base / 'inbox').glob('*.tmp')))

    def test_missing_and_partial_files_are_not_published(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            for name in ['missing.mp3', 'unfinished.mp3.part', 'program.exe']:
                file = base / name
                if name != 'missing.mp3':
                    file.write_bytes(b'fixture')
                with self.assertRaises((ValueError, FileNotFoundError)):
                    publish([file], str(base / 'inbox'))
            self.assertFalse((base / 'inbox').exists())

    def test_disabled_integration_does_not_change_engine_arguments(self):
        options = Options('https://example.com/media', 'output')
        args = build_args(options, Path('tools'))
        self.assertFalse(any(a.startswith('after_move:MEDIA:') for a in args))
        enabled = build_args(replace(options, ssmp_enabled=True), Path('tools'))
        self.assertTrue(any(a.startswith('after_move:MEDIA:') for a in enabled))
        self.assertEqual(enabled[-2:], ['--', options.url])

    def test_default_inbox_respects_explicit_data_directory(self):
        with patch.dict(os.environ, {'SSMP_DATA_DIR': str(Path('custom-library').absolute())}):
            self.assertEqual(default_inbox(), Path(os.environ['SSMP_DATA_DIR']) / 'imports')

    def test_empty_handoff_has_no_side_effects(self):
        with tempfile.TemporaryDirectory() as folder:
            self.assertIsNone(publish([], str(Path(folder) / 'inbox')))
            self.assertFalse((Path(folder) / 'inbox').exists())


if __name__ == '__main__':
    unittest.main()
