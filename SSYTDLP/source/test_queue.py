from pathlib import Path
from string import Formatter
import hashlib
import json
import tempfile
import unittest
from PIL import Image

from appearance import import_wallpaper, read_wallpaper
from engine import APP_NAME, Options, build_args, compact_indices
from i18n import LANGUAGES, STRINGS, tr
from queue_model import Batch, MediaEntry, MediaSource, QueueRecord, QueueStore, normalize_preview


def fixture():
    return normalize_preview('https://www.youtube.com/playlist?list=fixture', {
        '_type': 'playlist', 'title': 'Album', 'entries': [
            {'id': 'a', 'title': 'One', 'duration': 90, 'playlist_index': 1},
            {'id': 'b', 'title': '[Private video]', 'availability': 'private', 'playlist_index': 2},
            {'id': 'c', 'title': 'Three', 'playlist_index': 3},
        ]})


class QueueTests(unittest.TestCase):
    def test_preview_retains_unavailable_positions(self):
        source = fixture()
        self.assertTrue(source.is_playlist)
        self.assertEqual([entry.index for entry in source.entries], [1, 2, 3])
        self.assertFalse(source.entries[1].available)
        self.assertEqual(source.entries[0].duration, 90)
        single = normalize_preview('https://example.com/a', {'title': 'Single', 'duration': 9})
        self.assertFalse(single.is_playlist)
        self.assertEqual(single.entries[0].index, 1)

    def test_exact_playlist_subset_in_command(self):
        record = QueueRecord(fixture(), [3, 1])
        options = record.options(Options('https://example.com', '.'))
        args = build_args(options, Path('tools'))
        self.assertEqual(args[args.index('--playlist-items') + 1], '1,3')
        self.assertEqual(args[-2:], ['--', record.source.url])
        self.assertEqual(compact_indices((1, 2, 3, 5, 7, 8)), '1:3,5,7:8')
        for bad in ((0,), (-1,), (True,), ('1',)):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                compact_indices(bad)

    def test_empty_or_unavailable_selection_rejected(self):
        for indices in ([], [2], [99]):
            with self.subTest(indices=indices), self.assertRaises(ValueError):
                QueueRecord(fixture(), indices).validate()

    def test_queue_restart_and_backup_recovery(self):
        with tempfile.TemporaryDirectory() as directory:
            store = QueueStore(Path(directory) / 'queue.json')
            a = QueueRecord(fixture(), [1, 3], status='downloading')
            b = QueueRecord(fixture(), [3], status='completed')
            store.save([a, b])
            recovered = store.load()
            self.assertEqual([r.status for r in recovered], ['queued', 'completed'])
            self.assertEqual(recovered[0].selected, [1, 3])
            store.save(recovered)
            store.path.write_text('corrupt')
            recovered = store.load()
            self.assertEqual(len(recovered), 2)
            self.assertEqual(recovered[0].source.url, a.source.url)
            store.save(recovered)
            self.assertEqual(len(json.loads(store.path.with_suffix('.bak').read_text())['items']), 2)

    def test_batch_order_subset_and_continue_after_failure(self):
        called, events = [], []
        class FakeDownload:
            def __init__(self, tools, emit):
                self.emit = emit
            def run(self, options):
                called.append(options.playlist_items)
                self.emit('done', {'status': 'error' if len(called) == 1 else 'success', 'files': 0 if len(called) == 1 else 1})
        records = [QueueRecord(fixture(), [1]), QueueRecord(fixture(), [3])]
        Batch(Path('tools'), lambda *event: events.append(event), FakeDownload).run(records, Options('https://example.com', '.'))
        self.assertEqual(called, [(1,), (3,)])
        self.assertEqual(events[-1][1]['errors'], 1)
        self.assertEqual(events[-1][1]['completed'], 1)

    def test_batch_cancel_does_not_start_next_source(self):
        called, events = [], []
        class FakeDownload:
            def __init__(self, tools, emit):
                self.emit = emit
            def cancel(self):
                pass
            def run(self, options):
                called.append(options)
                batch.cancel()
                self.emit('done', {'status': 'cancelled', 'files': 0})
        batch = Batch(Path('tools'), lambda *event: events.append(event), FakeDownload)
        batch.run([QueueRecord(fixture(), [1]), QueueRecord(fixture(), [3])], Options('https://example.com', '.'))
        self.assertEqual(len(called), 1)
        self.assertTrue(events[-1][1]['cancelled'])

    def test_all_translations_have_the_same_placeholders(self):
        for key, texts in STRINGS.items():
            self.assertEqual(len(texts), len(LANGUAGES), key)
            fields = [{name for _, name, _, _ in Formatter().parse(text) if name} for text in texts]
            self.assertTrue(all(field == fields[0] for field in fields), key)
            for lang in LANGUAGES:
                self.assertTrue(tr(lang, key, **{name: '1' for name in fields[0]}), (lang, key))
        self.assertEqual(APP_NAME, 'SSYTDLP')
        self.assertEqual(tr('de', 'settings'), 'Einstellungen')

    def test_wallpaper_import_preserves_original_and_decodes_copy(self):
        with tempfile.TemporaryDirectory() as directory:
            original = Path(directory) / 'original.jpg'
            Image.new('RGB', (120, 80), 'navy').save(original)
            original_hash = hashlib.sha256(original.read_bytes()).hexdigest()
            target = Path(directory) / 'config/wallpaper.png'
            self.assertEqual(import_wallpaper(str(original), target), str(target))
            self.assertEqual(read_wallpaper(str(target)).size, (120, 80))
            self.assertEqual(hashlib.sha256(original.read_bytes()).hexdigest(), original_hash)


if __name__ == '__main__':
    unittest.main()
