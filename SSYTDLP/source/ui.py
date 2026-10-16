"""SSYTDLP: preview, stage links, then download the batch."""
from __future__ import annotations
from pathlib import Path
import json
import os
import queue
import sys
import threading
import tkinter as tk
from tkinter import filedialog, messagebox, ttk
from PIL import Image, ImageOps, ImageTk

from appearance import PALETTES, read_wallpaper
from dialogs import SelectionDialog, SettingsDialog
from engine import APP_NAME, APP_FULL_NAME, PRESETS, Options, Preview, build_args, load_settings, save_settings, update_nightly, validate_url
from i18n import LANGUAGES, tr
from queue_model import Batch, QueueRecord, QueueStore, normalize_preview
from platform_support import config_directory, open_folder


def app_directory():
    return Path(sys.executable).resolve().parent if getattr(sys, 'frozen', False) else Path(__file__).resolve().parent.parent


class App:
    def __init__(self, root, settings_path=None):
        self.root = root
        self.home = app_directory()
        self.tools = self.home / 'tools'
        data = config_directory().parent
        self.settings_path = settings_path or config_directory() / 'settings.json'
        settings = load_settings(self.settings_path)
        if settings_path is None and not self.settings_path.exists():
            settings = load_settings(data / 'yt-dlp Simple/settings.json')
        self.language = settings.get('language', 'en')
        if self.language not in LANGUAGES:
            self.language = 'en'
        self.theme = settings.get('theme', 'light')
        if self.theme not in PALETTES:
            self.theme = 'light'
        self.wallpaper_path = str(settings.get('wallpaper', ''))
        self.wallpaper_image = None
        self.background_photo = None
        self.resize_after = None
        self.events = queue.Queue()
        self.store = QueueStore(self.settings_path.with_name('queue.json'))
        self.records = self.store.load()
        self.operation = 'idle'
        self.preview_job = None
        self.batch = None
        self.selection_dialog = None
        self.close_pending = False
        self.logs = []
        self.url = tk.StringVar()
        self.folder = tk.StringVar(value=settings.get('folder', str(Path.home() / 'Downloads')))
        self.preset = settings.get('preset', next(iter(PRESETS)))
        if self.preset not in PRESETS:
            self.preset = next(iter(PRESETS))
        self.preset_display = tk.StringVar()
        self.metadata = tk.BooleanVar(value=bool(settings.get('metadata', True)))
        self.cover = tk.BooleanVar(value=bool(settings.get('cover', True)))
        self.organize = tk.BooleanVar(value=bool(settings.get('organize', True)))
        self.skip_existing = tk.BooleanVar(value=bool(settings.get('skip_existing', True)))
        self.lyrics = tk.BooleanVar(value=bool(settings.get('lyrics', False)))
        self.subtitles = tk.BooleanVar(value=bool(settings.get('subtitles', False)))
        self.embed_lyrics = bool(settings.get('embed_lyrics', True))
        self.subtitle_languages = str(settings.get('subtitle_languages', 'en.*,de.*'))
        self.automatic_subtitles = bool(settings.get('automatic_subtitles', False))
        self.embed_subtitles = bool(settings.get('embed_subtitles', False))
        self.ssmp_enabled = bool(settings.get('ssmp_enabled', False))
        self.ssmp_inbox = str(settings.get('ssmp_inbox', ''))
        self.current = tk.StringVar(value='')
        self.status = tk.StringVar()
        self.status_key, self.status_values = 'ready', {}
        self.hint = tk.StringVar()
        self.queue_count = tk.StringVar()
        self.version = tk.StringVar(value=self.read_version())
        root.title(APP_NAME + ' · ' + APP_FULL_NAME)
        root.geometry('1020x900')
        root.minsize(900, 740)
        try:
            root.iconbitmap(str(self.home / 'source/app.ico'))
        except tk.TclError:
            pass
        self.canvas = tk.Canvas(root, highlightthickness=0)
        self.canvas.pack(fill='both', expand=True)
        self.canvas.bind('<Configure>', self.resize)
        root.bind_all('<MouseWheel>', self.scroll_panel, add='+')
        root.bind_all('<Button-4>', self.scroll_panel, add='+')
        root.bind_all('<Button-5>', self.scroll_panel, add='+')
        self.main = None
        self.apply_appearance(self.language, self.theme, self.wallpaper_path, persist=False)
        root.protocol('WM_DELETE_WINDOW', self.close)
        self.poll_id = root.after(80, self.poll)

    def t(self, key, **values):
        return tr(self.language, key, **values)

    def read_version(self):
        try:
            return 'yt-dlp nightly ' + json.loads((self.tools / 'engines.json').read_text('utf-8'))['yt-dlp']['version']
        except (OSError, ValueError, KeyError):
            return 'yt-dlp nightly'

    def set_status(self, key, **values):
        self.status_key, self.status_values = key, values
        self.status.set(self.t(key, **values))

    def apply_appearance(self, language, theme, wallpaper, persist=True):
        self.language, self.theme, self.wallpaper_path = language, theme, wallpaper
        self.colors = PALETTES[theme]
        try:
            self.wallpaper_image = read_wallpaper(wallpaper)
        except Exception:
            self.wallpaper_image = None
            self.wallpaper_path = ''
        c = self.colors
        style = ttk.Style(self.root)
        style.theme_use('clam')
        style.configure('.', font=('Segoe UI', 10), background=c['panel'], foreground=c['text'])
        style.configure('TFrame', background=c['panel'])
        style.configure('TLabel', background=c['panel'])
        style.configure('Section.TLabel', font=('Segoe UI', 10, 'bold'))
        style.configure('Muted.TLabel', foreground=c['muted'], font=('Segoe UI', 9))
        style.configure('TButton', padding=(10, 6), background=c['button'], foreground=c['text'], borderwidth=0)
        style.map('TButton', background=[('active', c['active']), ('disabled', c['panel'])], foreground=[('disabled', c['muted'])])
        style.configure('Primary.TButton', padding=(16, 8), background=c['accent'], foreground='white', font=('Segoe UI', 10, 'bold'))
        style.map('Primary.TButton', background=[('disabled', c['button']), ('active', '#286ac0')], foreground=[('disabled', c['muted'])])
        style.configure('TEntry', padding=7, fieldbackground=c['field'], foreground=c['text'], bordercolor=c['border'], insertcolor=c['text'])
        style.configure('TCombobox', padding=6, fieldbackground=c['field'], foreground=c['text'], arrowcolor=c['text'], bordercolor=c['border'])
        style.map('TCombobox', fieldbackground=[('readonly', c['field'])], selectbackground=[('readonly', c['field'])], selectforeground=[('readonly', c['text'])])
        style.configure('TCheckbutton', padding=(0, 3), background=c['panel'], foreground=c['text'], indicatorsize=16,
                        indicatorbackground=c['field'], indicatorforeground=c['text'],
                        upperbordercolor=c['muted'], lowerbordercolor=c['muted'])
        style.map('TCheckbutton', background=[('active', c['panel'])], foreground=[('disabled', c['muted'])],
                  indicatorbackground=[('disabled', 'selected', c['button']), ('disabled', c['field']),
                                       ('selected', c['accent']), ('active', c['button']), ('!selected', c['field'])],
                  indicatorforeground=[('disabled', c['muted']), ('selected', '#ffffff'), ('!selected', c['text'])])
        style.configure('Treeview', background=c['field'], fieldbackground=c['field'], foreground=c['text'], rowheight=29, borderwidth=0)
        style.map('Treeview', background=[('selected', c['selected'])], foreground=[('selected', c['text'])])
        style.configure('Treeview.Heading', background=c['button'], foreground=c['text'], padding=7, font=('Segoe UI', 9, 'bold'))
        style.configure('TSeparator', background=c['border'])
        style.configure('TNotebook', background=c['panel'], borderwidth=0)
        style.configure('TNotebook.Tab', background=c['button'], foreground=c['text'], padding=(10, 7))
        style.map('TNotebook.Tab', background=[('selected', c['selected']), ('active', c['active'])], foreground=[('selected', c['text']), ('active', c['text'])])
        style.configure('Horizontal.TProgressbar', background=c['accent'], troughcolor=c['border'], borderwidth=0, thickness=7)
        self.root.option_add('*TCombobox*Listbox.background', c['field'])
        self.root.option_add('*TCombobox*Listbox.foreground', c['text'])
        self.root.option_add('*TCombobox*Listbox.selectBackground', c['selected'])
        if self.main is not None:
            self.main.destroy()
            self.scroller.destroy()
            self.panel_scrollbar.destroy()
            self.settings_button.destroy()
        self.canvas.delete('all')
        self.canvas.configure(bg=c['bg'])
        self.build()
        self.set_status(self.status_key, **self.status_values)
        self.paint()
        if persist:
            self.persist()

    def build(self):
        self.canvas.create_text(24, 21, text=APP_NAME, anchor='nw', fill=self.colors['text'], font=('Segoe UI', 24, 'bold'), tags='header')
        self.canvas.create_text(26, 60, text=APP_FULL_NAME, anchor='nw', fill=self.colors['muted'], font=('Segoe UI', 10), tags='header')
        self.canvas.create_text(24, 0, text='made by Slulisee', anchor='w', fill=self.colors['muted'], font=('Segoe UI', 9), tags='watermark')
        self.settings_button = ttk.Button(self.canvas, text='⚙ ' + self.t('settings'), command=lambda: SettingsDialog(self))
        self.settings_window = self.canvas.create_window(0, 30, window=self.settings_button, anchor='ne')
        self.scroller = tk.Canvas(self.canvas, bg=self.colors['panel'], highlightthickness=0)
        self.panel_window = self.canvas.create_window(20, 91, window=self.scroller, anchor='nw')
        self.panel_scrollbar = ttk.Scrollbar(self.canvas, command=self.scroller.yview)
        self.scrollbar_window = self.canvas.create_window(0, 91, window=self.panel_scrollbar, anchor='nw', state='hidden')
        self.scroller.configure(yscrollcommand=self.panel_scrollbar.set)
        main = self.main = ttk.Frame(self.scroller, padding=(18, 14))
        self.content_window = self.scroller.create_window(0, 0, window=main, anchor='nw')
        main.columnconfigure(0, weight=1)
        main.rowconfigure(3, weight=1, minsize=105)
        ttk.Label(main, text=self.t('link'), style='Section.TLabel').grid(row=0, column=0, sticky='w')
        link = ttk.Frame(main)
        link.grid(row=1, column=0, sticky='ew', pady=(5, 12))
        link.columnconfigure(0, weight=1)
        self.url_entry = ttk.Entry(link, textvariable=self.url)
        self.url_entry.grid(row=0, column=0, sticky='ew')
        self.url_entry.bind('<Return>', lambda _: self.preview_link())
        self.paste_button = ttk.Button(link, text=self.t('paste'), command=self.paste)
        self.paste_button.grid(row=0, column=1, padx=7)
        self.add_button = ttk.Button(link, text=self.t('preview_add'), style='Primary.TButton', command=self.preview_link)
        self.add_button.grid(row=0, column=2)
        heading = ttk.Frame(main)
        heading.grid(row=2, column=0, sticky='ew', pady=(0, 7))
        ttk.Label(heading, text=self.t('queue'), style='Section.TLabel').pack(side='left')
        ttk.Label(heading, textvariable=self.queue_count, style='Muted.TLabel').pack(side='right')
        table = ttk.Frame(main)
        table.grid(row=3, column=0, sticky='nsew')
        table.columnconfigure(0, weight=1)
        table.rowconfigure(0, weight=1)
        self.tree = ttk.Treeview(table, columns=('title', 'selected', 'status'), show='headings', selectmode='extended', height=4)
        for name, key, width, stretch in [('title', 'title', 500, True), ('selected', 'selected', 110, False), ('status', 'status', 135, False)]:
            self.tree.heading(name, text=self.t(key))
            self.tree.column(name, width=width, minwidth=80, stretch=stretch)
        self.tree.grid(row=0, column=0, sticky='nsew')
        scrollbar = ttk.Scrollbar(table, command=self.tree.yview)
        scrollbar.grid(row=0, column=1, sticky='ns')
        self.tree.configure(yscrollcommand=scrollbar.set)
        self.tree.bind('<Double-1>', lambda _: self.edit_selection())
        self.tree.bind('<Delete>', lambda _: self.remove_selected())
        queue_actions = ttk.Frame(main)
        queue_actions.grid(row=4, column=0, sticky='ew', pady=(7, 0))
        self.edit_button = ttk.Button(queue_actions, text=self.t('edit'), command=self.edit_selection)
        self.edit_button.pack(side='left')
        self.remove_button = ttk.Button(queue_actions, text=self.t('remove'), command=self.remove_selected)
        self.remove_button.pack(side='left', padx=6)
        self.clear_button = ttk.Button(queue_actions, text=self.t('clear_done'), command=self.clear_completed)
        self.clear_button.pack(side='left')
        ttk.Label(main, text=self.t('queue_hint'), style='Muted.TLabel', wraplength=900).grid(row=5, column=0, sticky='w', pady=(6, 10))
        ttk.Separator(main).grid(row=6, column=0, sticky='ew')
        ttk.Label(main, text=self.t('options'), style='Section.TLabel').grid(row=7, column=0, sticky='w', pady=(10, 5))
        self.preset_display.set(self.t('preset_' + str(list(PRESETS).index(self.preset))))
        self.preset_box = ttk.Combobox(main, textvariable=self.preset_display, values=[self.t(f'preset_{i}') for i in range(len(PRESETS))], state='readonly')
        self.preset_box.grid(row=8, column=0, sticky='ew')
        self.preset_box.bind('<<ComboboxSelected>>', self.preset_changed)
        ttk.Label(main, textvariable=self.hint, style='Muted.TLabel').grid(row=9, column=0, sticky='w', pady=(3, 6))
        checks = ttk.Frame(main)
        checks.grid(row=10, column=0, sticky='ew')
        checks.columnconfigure(0, weight=1)
        checks.columnconfigure(1, weight=1)
        self.checks = []
        for i, (key, variable) in enumerate([('metadata', self.metadata), ('cover', self.cover), ('organize', self.organize), ('skip', self.skip_existing), ('lyrics', self.lyrics), ('subtitles', self.subtitles)]):
            check = ttk.Checkbutton(checks, text=self.t(key), variable=variable)
            check.grid(row=i // 2, column=i % 2, sticky='w')
            self.checks.append(check)
        ttk.Label(main, text=self.t('fallback'), style='Muted.TLabel').grid(row=11, column=0, sticky='w', pady=(1, 9))
        destination = ttk.Frame(main)
        destination.grid(row=12, column=0, sticky='ew', pady=(0, 10))
        destination.columnconfigure(1, weight=1)
        ttk.Label(destination, text=self.t('save_to'), style='Section.TLabel').grid(row=0, column=0, padx=(0, 10))
        self.folder_entry = ttk.Entry(destination, textvariable=self.folder)
        self.folder_entry.grid(row=0, column=1, sticky='ew')
        self.browse_button = ttk.Button(destination, text=self.t('browse'), command=self.browse)
        self.browse_button.grid(row=0, column=2, padx=(7, 0))
        actions = ttk.Frame(main)
        actions.grid(row=13, column=0, sticky='ew')
        self.download_button = ttk.Button(actions, text=self.t('download'), style='Primary.TButton', command=self.start)
        self.download_button.pack(side='left')
        self.cancel_button = ttk.Button(actions, text=self.t('cancel'), command=self.cancel)
        self.cancel_button.pack(side='left', padx=7)
        ttk.Button(actions, text=self.t('open_folder'), command=self.open_folder).pack(side='right')
        self.progress = ttk.Progressbar(main, mode='determinate', maximum=100)
        self.progress.grid(row=14, column=0, sticky='ew', pady=(11, 7))
        ttk.Label(main, textvariable=self.current, style='Section.TLabel').grid(row=15, column=0, sticky='w')
        ttk.Label(main, textvariable=self.status, style='Muted.TLabel').grid(row=16, column=0, sticky='w', pady=(2, 6))
        self.log = tk.Text(main, height=3, wrap='word', bg=self.colors['field'], fg=self.colors['muted'], relief='flat', font=('Consolas', 9), padx=8, pady=5)
        self.log.grid(row=17, column=0, sticky='ew')
        self.log.insert('end', '\n'.join(self.logs))
        self.log.configure(state='disabled')
        footer = ttk.Frame(main)
        footer.grid(row=18, column=0, sticky='ew', pady=(8, 0))
        ttk.Label(footer, textvariable=self.version, style='Muted.TLabel').pack(side='left')
        self.update_button = ttk.Button(footer, text=self.t('update'), command=self.update, padding=(8, 3))
        self.update_button.pack(side='right')
        self.editable = [self.settings_button, self.url_entry, self.paste_button, self.add_button, self.edit_button, self.remove_button, self.clear_button, self.folder_entry, self.browse_button, self.update_button, self.download_button, *self.checks]
        self.refresh_queue()
        self.preset_changed()
        self.set_operation(self.operation)

    def resize(self, _=None):
        if self.resize_after is not None:
            self.root.after_cancel(self.resize_after)
        self.resize_after = self.root.after(60, self.paint)

    def paint(self):
        self.resize_after = None
        width, height = max(900, self.canvas.winfo_width()), max(740, self.canvas.winfo_height())
        viewport = height - 123
        minimum = self.main.winfo_reqheight()
        content_height = max(viewport, minimum)
        scroll_needed = content_height > viewport
        panel_width = width - 40 - (15 if scroll_needed else 0)
        self.canvas.itemconfigure(self.panel_window, width=panel_width, height=viewport)
        self.canvas.itemconfigure(self.scrollbar_window, height=viewport, state='normal' if scroll_needed else 'hidden')
        self.canvas.coords(self.scrollbar_window, width - 35, 91)
        self.scroller.itemconfigure(self.content_window, width=panel_width, height=content_height)
        self.scroller.configure(scrollregion=(0, 0, panel_width, content_height))
        if not scroll_needed:
            self.scroller.yview_moveto(0)
        self.canvas.coords(self.settings_window, width - 24, 29)
        self.canvas.coords('watermark', 24, height - 16)
        self.canvas.delete('wallpaper')
        if self.wallpaper_image is not None:
            image = ImageOps.fit(self.wallpaper_image, (width, height), Image.Resampling.LANCZOS)
            image = Image.blend(image, Image.new('RGB', image.size, self.colors['bg']), 0.63 if self.theme == 'light' else 0.48)
            self.background_photo = ImageTk.PhotoImage(image)
            self.canvas.create_image(0, 0, anchor='nw', image=self.background_photo, tags='wallpaper')
            self.canvas.tag_lower('wallpaper')
        else:
            self.background_photo = None

    def scroll_panel(self, event):
        if event.widget.winfo_toplevel() is self.root and not isinstance(event.widget, (ttk.Treeview, tk.Text)):
            if self.scroller.yview() != (0.0, 1.0):
                step = -1 if getattr(event, 'num', None) == 4 else 1 if getattr(event, 'num', None) == 5 else -int(event.delta) if sys.platform == 'darwin' else -int(event.delta / 120)
                self.scroller.yview_scroll(step, 'units')

    def preset_changed(self, _=None):
        labels = [self.t(f'preset_{i}') for i in range(len(PRESETS))]
        if self.preset_display.get() in labels:
            self.preset = list(PRESETS)[labels.index(self.preset_display.get())]
        kind, _, ext = PRESETS[self.preset]
        self.hint.set(self.t('hint_video' if kind == 'video' else 'hint_flac' if ext == 'flac' else 'hint_audio'))

    def get_options(self):
        return Options('https://example.com', self.folder.get(), self.preset, False, self.metadata.get(), self.cover.get(), self.organize.get(), self.skip_existing.get(),
                       lyrics=self.lyrics.get(), embed_lyrics=self.embed_lyrics, subtitles=self.subtitles.get(), subtitle_languages=self.subtitle_languages,
                       automatic_subtitles=self.automatic_subtitles, embed_subtitles=self.embed_subtitles,
                       ssmp_enabled=self.ssmp_enabled, ssmp_inbox=self.ssmp_inbox)

    def persist(self):
        values = self.get_options().__dict__.copy()
        for key in ('url', 'playlist', 'playlist_items', 'catalog_title', 'catalog_artist', 'catalog_album', 'catalog_number'):
            values.pop(key, None)
        values.update(language=self.language, theme=self.theme, wallpaper=self.wallpaper_path)
        try:
            save_settings(self.settings_path, values)
            self.store.save(self.records)
        except OSError as exc:
            self.append_log(str(exc))
            self.set_status('save_error')

    def refresh_queue(self):
        selected = self.tree.selection()
        self.tree.delete(*self.tree.get_children())
        for record in self.records:
            self.tree.insert('', 'end', iid=record.id, values=(record.source.title, f'{len(record.selected)} / {len(record.source.entries)}', self.t(record.status)))
        for key in selected:
            if self.tree.exists(key):
                self.tree.selection_add(key)
        self.queue_count.set(self.t('queue_count', links=len(self.records), videos=sum(len(r.selected) for r in self.records)))

    def set_operation(self, operation):
        self.operation = operation
        for widget in self.editable:
            widget.configure(state='normal' if operation == 'idle' else 'disabled')
        self.preset_box.configure(state='readonly' if operation == 'idle' else 'disabled')
        self.cancel_button.configure(state='normal' if operation in ('download', 'preview') else 'disabled')

    def paste(self):
        try:
            self.url.set(self.root.clipboard_get().strip())
        except tk.TclError:
            self.set_status('empty_clipboard')

    def browse(self):
        folder = filedialog.askdirectory(title=self.t('save_to'), initialdir=self.folder.get() if Path(self.folder.get()).is_dir() else str(Path.home()))
        if folder:
            self.folder.set(folder)

    def open_folder(self):
        try:
            folder = Path(self.folder.get()).expanduser().absolute()
            folder.mkdir(parents=True, exist_ok=True)
            open_folder(folder)
        except OSError as exc:
            messagebox.showerror(self.t('open_folder'), str(exc), parent=self.root)

    def preview_link(self):
        if self.operation != 'idle':
            return
        try:
            url = validate_url(self.url.get())
        except ValueError:
            self.set_status('invalid_link')
            return
        self.set_operation('preview')
        self.set_status('preview_loading')
        self.progress.start(15)
        job = self.preview_job = Preview(self.tools, lambda *_: None)
        def fetch():
            try:
                source = normalize_preview(url, job.fetch(url))
                self.events.put(('preview_ok', source))
            except Exception as exc:
                self.events.put(('preview_error', str(exc)))
        threading.Thread(target=fetch, daemon=True).start()

    def show_selection(self, source, record=None):
        selected = record.selected if record else [entry.index for entry in source.entries if entry.available]
        def accept(indices):
            if record is not None:
                record.selected = indices
                record.status = 'queued'
            else:
                if any(r.source.url == source.url and r.selected == indices and r.status != 'completed' for r in self.records):
                    self.set_status('duplicate')
                    return
                self.records.append(QueueRecord(source, indices))
                self.url.set('')
            self.refresh_queue()
            self.persist()
            self.set_status('added')
        self.selection_dialog = SelectionDialog(self, source, selected, accept, record is not None)

    def edit_selection(self):
        if self.operation == 'idle' and self.tree.selection():
            record = next((r for r in self.records if r.id == self.tree.selection()[0]), None)
            if record:
                self.show_selection(record.source, record)

    def remove_selected(self):
        if self.operation == 'idle':
            selected = set(self.tree.selection())
            self.records = [r for r in self.records if r.id not in selected]
            self.refresh_queue()
            self.persist()

    def clear_completed(self):
        if self.operation == 'idle':
            self.records = [r for r in self.records if r.status != 'completed']
            self.refresh_queue()
            self.persist()

    def start(self):
        if self.operation != 'idle':
            return
        pending = [r for r in self.records if r.status != 'completed']
        if not pending:
            self.set_status('nothing')
            return
        defaults = self.get_options()
        try:
            for record in pending:
                build_args(record.options(defaults), self.tools)
        except ValueError as exc:
            self.append_log(str(exc))
            self.set_status('nothing')
            return
        self.persist()
        self.set_operation('download')
        self.set_status('connecting')
        self.progress.configure(value=0)
        self.progress.start(15)
        self.batch = Batch(self.tools, lambda kind, value: self.events.put((kind, value)))
        threading.Thread(target=self.batch.run, args=(pending, defaults), daemon=True).start()

    def cancel(self):
        job = self.batch if self.operation == 'download' else self.preview_job
        if job is not None:
            self.set_status('cancelling')
            self.cancel_button.configure(state='disabled')
            threading.Thread(target=job.cancel, daemon=True).start()

    def update(self):
        if self.operation == 'idle':
            self.set_operation('update')
            self.set_status('update_check')
            threading.Thread(target=update_nightly, args=(self.tools, lambda kind, value: self.events.put((kind, value))), daemon=True).start()

    def append_log(self, value):
        self.logs.append(str(value))
        self.logs = self.logs[-350:]
        self.log.configure(state='normal')
        self.log.delete('1.0', 'end')
        self.log.insert('end', '\n'.join(self.logs))
        self.log.see('end')
        self.log.configure(state='disabled')

    def poll(self):
        try:
            for _ in range(150):
                kind, value = self.events.get_nowait()
                if kind == 'log':
                    self.append_log(value)
                    if any(word in str(value) for word in ('[ExtractAudio]', '[Merger]', '[Metadata]', '[EmbedThumbnail]', '[VideoRemuxer]')):
                        self.set_status('finishing')
                elif kind == 'item':
                    self.current.set(str(value)[:115])
                    self.progress.stop()
                    self.progress.configure(value=0)
                elif kind == 'file':
                    self.append_log(self.t('saved', path=value))
                elif kind == 'progress' and isinstance(value, dict):
                    self.progress.stop()
                    total = value.get('total_bytes') or value.get('total_bytes_estimate') or 0
                    downloaded = value.get('downloaded_bytes') or 0
                    percent = min(100, downloaded * 100 / total) if total else 0
                    self.progress.configure(value=percent)
                    speed = value.get('speed') or 0
                    text = self.t('speed', percent=f'{percent:.0f}', speed=f'{speed / 1048576:.1f}')
                    if isinstance(value.get('eta'), (int, float)):
                        text += ' · ' + self.t('remaining', seconds=int(value['eta']))
                    self.status.set(text)
                elif kind == 'queue_status':
                    for record in self.records:
                        if record.id == value['id']:
                            record.status = value['status']
                    self.refresh_queue()
                    self.persist()
                elif kind == 'batch_done':
                    self.progress.stop()
                    self.progress.configure(value=100 if not value['cancelled'] else 0)
                    self.batch = None
                    self.set_operation('idle')
                    self.set_status('batch_cancelled' if value['cancelled'] else 'batch_done', **value)
                    self.persist()
                elif kind in ('preview_ok', 'preview_error'):
                    cancelled = self.preview_job is not None and self.preview_job.cancelled.is_set()
                    self.preview_job = None
                    self.progress.stop()
                    self.progress.configure(value=0)
                    self.set_operation('idle')
                    if cancelled:
                        self.set_status('preview_stopped')
                    elif kind == 'preview_ok' and not self.close_pending:
                        self.show_selection(value)
                        self.set_status('ready')
                    else:
                        self.append_log(value)
                        self.set_status('preview_failed')
                elif kind == 'thumbnail':
                    dialog = self.selection_dialog
                    if dialog is not None and value[0] == dialog.id:
                        dialog.set_thumbnail(value[1], value[2])
                elif kind in ('updated', 'update_error'):
                    self.set_operation('idle')
                    if kind == 'updated':
                        self.version.set('yt-dlp nightly ' + str(value))
                        self.set_status('update_ok')
                    else:
                        self.append_log(value)
                        self.set_status('update_error')
                if self.close_pending and self.operation == 'idle':
                    self.persist()
                    self.root.destroy()
                    return
        except queue.Empty:
            pass
        self.poll_id = self.root.after(80, self.poll)

    def close(self):
        if self.operation != 'idle':
            if messagebox.askyesno(APP_NAME, self.t('close_update' if self.operation == 'update' else 'close_busy'), parent=self.root):
                self.close_pending = True
                if self.operation != 'update':
                    self.cancel()
            return
        self.persist()
        self.root.destroy()


def main():
    if os.name == 'nt':
        import ctypes
        try:
            ctypes.windll.shcore.SetProcessDpiAwareness(1)
        except (AttributeError, OSError):
            pass
    root = tk.Tk()
    smoke = len(sys.argv) > 2 and sys.argv[1] == '--smoke-test'
    app = App(root, Path(sys.argv[2]).with_suffix('.settings.json') if smoke else None)
    if smoke:
        def verify():
            import mutagen
            from lyrics import LyricsFinder
            from services import catalog_provider
            provider = LyricsFinder()
            values = {'title': root.title(), 'presets': list(PRESETS), 'nightly': app.version.get(), 'mapped': root.winfo_ismapped(), 'buttons': [app.download_button.cget('text'), app.add_button.cget('text'), app.settings_button.cget('text')], 'languages': LANGUAGES, 'theme': app.theme, 'watermark': app.canvas.itemcget('watermark', 'text')}
            values['extras'] = [control.cget('text') for control in app.checks]
            values['lyric_provider'] = type(provider.provider).__name__
            values['spotify_import'] = catalog_provider('https://open.spotify.com/track/example')
            Path(sys.argv[2]).write_text(json.dumps(values, indent=2), encoding='utf-8')
            root.destroy()
        root.after(1200, verify)
    root.mainloop()
