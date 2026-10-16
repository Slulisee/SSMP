from __future__ import annotations
from io import BytesIO
from pathlib import Path
import threading
import re
import tkinter as tk
from tkinter import filedialog, messagebox, ttk
from urllib.request import Request, urlopen
import uuid
from PIL import Image, ImageTk

from appearance import import_wallpaper
from engine import validate_url
from i18n import LANGUAGES


def duration_text(seconds):
    if not isinstance(seconds, (int, float)) or seconds < 0:
        return '—'
    seconds = int(seconds)
    hours, remainder = divmod(seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    return f'{hours}:{minutes:02d}:{seconds:02d}' if hours else f'{minutes}:{seconds:02d}'


class SelectionDialog:
    def __init__(self, app, source, selected, callback, editing=False):
        self.app, self.source, self.callback = app, source, callback
        self.id = uuid.uuid4().hex
        self.window = tk.Toplevel(app.root)
        self.window.title(app.t('preview'))
        self.window.geometry('950x560')
        self.window.minsize(820, 480)
        self.window.transient(app.root)
        self.window.configure(bg=app.colors['panel'])
        self.window.protocol('WM_DELETE_WINDOW', self.close)
        self.window.bind('<Escape>', lambda _: self.close())
        self.selected = set(selected)
        self.entries = {str(e.index): e for e in source.entries}
        self.thumbnail_token = ''
        self.photo = None
        main = ttk.Frame(self.window, padding=18)
        main.pack(fill='both', expand=True)
        main.columnconfigure(0, weight=1)
        main.rowconfigure(3, weight=1)
        ttk.Label(main, text=source.title, style='Section.TLabel', wraplength=880).grid(row=0, column=0, columnspan=2, sticky='w')
        hint = app.t('source_catalog', provider=source.provider) if source.kind == 'catalog' else app.t('select_hint')
        ttk.Label(main, text=hint, style='Muted.TLabel', wraplength=880).grid(row=1, column=0, columnspan=2, sticky='w', pady=(4, 12))
        actions = ttk.Frame(main)
        actions.grid(row=2, column=0, columnspan=2, sticky='ew', pady=(0, 8))
        for key, mode in [('select_all', 'all'), ('select_none', 'none'), ('invert', 'invert')]:
            ttk.Button(actions, text=app.t(key), command=lambda m=mode: self.change_all(m)).pack(side='left', padx=(0, 6))
        self.counter = tk.StringVar()
        ttk.Label(actions, textvariable=self.counter, style='Muted.TLabel').pack(side='right')
        table = ttk.Frame(main)
        table.grid(row=3, column=0, sticky='nsew')
        table.rowconfigure(0, weight=1)
        table.columnconfigure(0, weight=1)
        self.tree = ttk.Treeview(table, columns=('use', 'number', 'title', 'channel', 'duration'), show='headings', selectmode='extended')
        for name, label, width, stretch in [('use', '✓', 36, False), ('number', '#', 42, False), ('title', app.t('title'), 260, True), ('channel', app.t('channel'), 115, True), ('duration', app.t('duration'), 70, False)]:
            self.tree.heading(name, text=label)
            self.tree.column(name, width=width, minwidth=width if not stretch else 70, stretch=stretch, anchor='center' if name in ('use', 'number', 'duration') else 'w')
        self.tree.tag_configure('unavailable', foreground=app.colors['muted'])
        self.tree.grid(row=0, column=0, sticky='nsew')
        scroll = ttk.Scrollbar(table, command=self.tree.yview)
        scroll.grid(row=0, column=1, sticky='ns')
        self.tree.configure(yscrollcommand=scroll.set)
        detail = ttk.Frame(main, padding=(16, 0, 0, 0), width=220)
        detail.grid(row=3, column=1, sticky='ns')
        self.thumbnail = ttk.Label(detail, text=app.t('no_thumbnail'), anchor='center', width=28, wraplength=200)
        self.thumbnail.pack(fill='x', pady=(5, 12))
        self.detail_title = ttk.Label(detail, text='', style='Section.TLabel', wraplength=200)
        self.detail_title.pack(anchor='w')
        self.detail_channel = ttk.Label(detail, text='', style='Muted.TLabel', wraplength=200)
        self.detail_channel.pack(anchor='w', pady=5)
        footer = ttk.Frame(main)
        footer.grid(row=4, column=0, columnspan=2, sticky='ew', pady=(14, 0))
        ttk.Button(footer, text=app.t('cancel'), command=self.close).pack(side='right')
        self.accept_button = ttk.Button(footer, text=app.t('save_selection' if editing else 'add_queue'), style='Primary.TButton', command=self.accept)
        self.accept_button.pack(side='right', padx=(0, 8))
        self.tree.bind('<Button-1>', self.click)
        self.tree.bind('<Double-1>', self.double_click)
        self.tree.bind('<space>', self.space)
        self.tree.bind('<<TreeviewSelect>>', self.show_details)
        for key in self.entries:
            self.insert(key)
        self.refresh_count()
        first = next(iter(self.entries))
        self.tree.selection_set(first)
        self.tree.focus(first)
        self.window.grab_set()

    def insert(self, key):
        entry = self.entries[key]
        values = ('☑' if entry.index in self.selected else '☐' if entry.available else '—', entry.index, entry.title, entry.channel if entry.available else self.app.t('unavailable'), duration_text(entry.duration))
        if self.tree.exists(key):
            self.tree.item(key, values=values)
        else:
            self.tree.insert('', 'end', iid=key, values=values, tags=() if entry.available else ('unavailable',))

    def refresh_count(self):
        self.counter.set(self.app.t('selection_count', selected=len(self.selected), total=len(self.entries)))
        self.accept_button.configure(state='normal' if self.selected else 'disabled')

    def toggle(self, keys):
        for key in keys:
            entry = self.entries[key]
            if entry.available:
                if entry.index in self.selected:
                    self.selected.remove(entry.index)
                else:
                    self.selected.add(entry.index)
                self.insert(key)
        self.refresh_count()

    def click(self, event):
        key = self.tree.identify_row(event.y)
        if key and self.tree.identify_column(event.x) == '#1':
            self.tree.selection_set(key)
            self.tree.focus(key)
            self.toggle([key])
            return 'break'

    def double_click(self, event):
        key = self.tree.identify_row(event.y)
        if key and self.tree.identify_column(event.x) != '#1':
            self.toggle([key])
        return 'break'

    def space(self, _):
        self.toggle(self.tree.selection())
        return 'break'

    def change_all(self, mode):
        available = {entry.index for entry in self.entries.values() if entry.available}
        self.selected = available if mode == 'all' else set() if mode == 'none' else available - self.selected
        for key in self.entries:
            self.insert(key)
        self.refresh_count()

    def show_details(self, _=None):
        keys = self.tree.selection()
        if not keys:
            return
        entry = self.entries[keys[-1]]
        self.detail_title.configure(text=entry.title)
        self.detail_channel.configure(text=entry.channel + (' · ' + duration_text(entry.duration) if entry.duration else ''))
        self.photo = None
        self.thumbnail.configure(image='', text=self.app.t('no_thumbnail'))
        token = self.thumbnail_token = uuid.uuid4().hex
        if entry.thumbnail:
            def fetch():
                try:
                    url = validate_url(entry.thumbnail)
                    with urlopen(Request(url, headers={'User-Agent': 'SSYTDLP/2.0'}), timeout=12) as response:
                        data = response.read(4_000_001)
                    if len(data) > 4_000_000:
                        return
                    self.app.events.put(('thumbnail', (self.id, token, data)))
                except Exception:
                    pass
            threading.Thread(target=fetch, daemon=True).start()

    def set_thumbnail(self, token, data):
        if token != self.thumbnail_token:
            return
        try:
            with Image.open(BytesIO(data)) as raw:
                if raw.width * raw.height > 20_000_000:
                    return
                image = raw.convert('RGB')
                image.thumbnail((210, 155), Image.Resampling.LANCZOS)
            self.photo = ImageTk.PhotoImage(image)
            self.thumbnail.configure(image=self.photo, text='')
        except Exception:
            pass

    def accept(self):
        if self.selected:
            self.callback(sorted(self.selected))
            self.close()

    def close(self):
        if self.app.selection_dialog is self:
            self.app.selection_dialog = None
        self.window.destroy()


class SettingsDialog:
    def __init__(self, app):
        self.app = app
        self.window = tk.Toplevel(app.root)
        self.window.title(app.t('settings'))
        self.window.geometry('560x660')
        self.window.resizable(False, False)
        self.window.transient(app.root)
        self.window.configure(bg=app.colors['panel'])
        self.window.bind('<Escape>', lambda _: self.window.destroy())
        self.language = tk.StringVar(value=LANGUAGES[app.language])
        self.theme = tk.StringVar(value=app.t(app.theme))
        self.wallpaper = app.wallpaper_path
        self.subtitle_languages = tk.StringVar(value=app.subtitle_languages)
        self.embed_lyrics = tk.BooleanVar(value=app.embed_lyrics)
        self.automatic_subtitles = tk.BooleanVar(value=app.automatic_subtitles)
        self.embed_subtitles = tk.BooleanVar(value=app.embed_subtitles)
        self.ssmp_enabled = tk.BooleanVar(value=app.ssmp_enabled)
        self.ssmp_inbox = tk.StringVar(value=app.ssmp_inbox)
        outer = ttk.Frame(self.window, padding=14)
        outer.pack(fill='both', expand=True)
        tabs = ttk.Notebook(outer)
        tabs.pack(fill='both', expand=True)
        main = ttk.Frame(tabs, padding=18)
        media = ttk.Frame(tabs, padding=18)
        services = ttk.Frame(tabs, padding=18)
        tabs.add(main, text=app.t('appearance'))
        tabs.add(media, text=app.t('media'))
        tabs.add(services, text=app.t('services'))
        integration = ttk.Frame(tabs, padding=18)
        tabs.add(integration, text='SSMP')
        ttk.Label(integration, text='SSMP', style='Section.TLabel').pack(anchor='w')
        ttk.Checkbutton(integration, text=app.t('ssmp_enable'), variable=self.ssmp_enabled).pack(anchor='w', pady=(12, 16))
        ttk.Label(integration, text=app.t('ssmp_inbox')).pack(anchor='w')
        ttk.Entry(integration, textvariable=self.ssmp_inbox).pack(fill='x', pady=8)
        def choose_inbox():
            folder = filedialog.askdirectory(parent=self.window)
            if folder:
                self.ssmp_inbox.set(folder)
        ttk.Button(integration, text=app.t('browse'), command=choose_inbox).pack(anchor='w')
        ttk.Label(integration, text=app.t('ssmp_help'), wraplength=450, style='Muted.TLabel', justify='left').pack(anchor='w', pady=18)
        main.columnconfigure(0, weight=1)
        for row, key in [(0, 'language'), (2, 'theme'), (4, 'wallpaper')]:
            ttk.Label(main, text=app.t(key), style='Section.TLabel').grid(row=row, column=0, sticky='w', pady=(0 if row == 0 else 16, 6))
        ttk.Combobox(main, textvariable=self.language, values=list(LANGUAGES.values()), state='readonly').grid(row=1, column=0, sticky='ew')
        ttk.Combobox(main, textvariable=self.theme, values=[app.t('light'), app.t('dark')], state='readonly').grid(row=3, column=0, sticky='ew')
        self.wallpaper_label = ttk.Label(main, text=Path(self.wallpaper).name if self.wallpaper else app.t('no_image'), style='Muted.TLabel')
        self.wallpaper_label.grid(row=5, column=0, sticky='w')
        buttons = ttk.Frame(main)
        buttons.grid(row=6, column=0, sticky='w', pady=8)
        ttk.Button(buttons, text=app.t('choose_image'), command=self.choose).pack(side='left', padx=(0, 6))
        ttk.Button(buttons, text=app.t('remove_image'), command=self.remove).pack(side='left')
        ttk.Label(main, text=app.t('wallpaper_hint'), style='Muted.TLabel', wraplength=430).grid(row=7, column=0, sticky='w')
        ttk.Label(media, text=app.t('lyrics'), style='Section.TLabel').pack(anchor='w')
        ttk.Label(media, text=app.t('lyrics_help'), style='Muted.TLabel', wraplength=475).pack(anchor='w', pady=(5, 10))
        ttk.Checkbutton(media, text=app.t('embed_lyrics'), variable=self.embed_lyrics).pack(anchor='w')
        ttk.Separator(media).pack(fill='x', pady=18)
        ttk.Label(media, text=app.t('sub_languages'), style='Section.TLabel').pack(anchor='w')
        ttk.Entry(media, textvariable=self.subtitle_languages).pack(fill='x', pady=(7, 12))
        ttk.Checkbutton(media, text=app.t('auto_subtitles'), variable=self.automatic_subtitles).pack(anchor='w')
        ttk.Checkbutton(media, text=app.t('embed_subtitles'), variable=self.embed_subtitles).pack(anchor='w', pady=5)
        ttk.Label(media, text=app.t('sub_help'), style='Muted.TLabel', wraplength=475).pack(anchor='w', pady=(10, 0))
        ttk.Label(services, text=app.t('services'), style='Section.TLabel').pack(anchor='w')
        ttk.Label(services, text=app.t('service_help'), wraplength=475, justify='left').pack(anchor='w', pady=12)
        ttk.Separator(services).pack(fill='x', pady=14)
        ttk.Label(services, text=app.t('disclaimer'), style='Muted.TLabel', wraplength=475, justify='left').pack(anchor='w')
        footer = ttk.Frame(outer)
        footer.pack(fill='x', pady=(12, 0))
        ttk.Button(footer, text=app.t('cancel'), command=self.window.destroy).pack(side='right')
        ttk.Button(footer, text=app.t('apply'), style='Primary.TButton', command=self.apply).pack(side='right', padx=(0, 8))
        self.window.grab_set()

    def choose(self):
        path = filedialog.askopenfilename(parent=self.window, title=self.app.t('choose_image'), filetypes=[('Images', '*.png *.jpg *.jpeg *.webp *.bmp'), ('All files', '*.*')])
        if path:
            self.wallpaper = path
            self.wallpaper_label.configure(text=Path(path).name)

    def remove(self):
        self.wallpaper = ''
        self.wallpaper_label.configure(text=self.app.t('no_image'))

    def apply(self):
        if not re.fullmatch(r'[A-Za-z0-9_.*,\-]+', self.subtitle_languages.get().strip()):
            messagebox.showerror(self.app.t('settings'), self.app.t('sub_languages'), parent=self.window)
            return
        try:
            wallpaper = self.wallpaper
            if wallpaper and wallpaper != self.app.wallpaper_path:
                wallpaper = import_wallpaper(wallpaper, self.app.settings_path.parent / 'wallpaper.png')
        except Exception:
            messagebox.showerror(self.app.t('settings'), self.app.t('image_error'), parent=self.window)
            return
        language = next(key for key, value in LANGUAGES.items() if value == self.language.get())
        theme = 'dark' if self.theme.get() == self.app.t('dark') else 'light'
        self.app.subtitle_languages = self.subtitle_languages.get().strip()
        self.app.embed_lyrics = self.embed_lyrics.get()
        self.app.automatic_subtitles = self.automatic_subtitles.get()
        self.app.embed_subtitles = self.embed_subtitles.get()
        self.app.ssmp_enabled = self.ssmp_enabled.get()
        self.app.ssmp_inbox = self.ssmp_inbox.get().strip()
        self.window.destroy()
        self.app.apply_appearance(language, theme, wallpaper)
