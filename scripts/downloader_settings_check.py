"""Exercise the real settings dialog with an isolated configuration file."""
from pathlib import Path
import json, sys, tkinter as tk, time
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'SSYTDLP/source'))
from ui import App
from dialogs import SettingsDialog
settings=ROOT/'qa'/f'dialog-{int(time.time())}.json'
root=tk.Tk()
try:
    app=App(root,settings);root.update()
    assert app.ssmp_enabled is False
    dialog=SettingsDialog(app);root.update()
    dialog.ssmp_enabled.set(True)
    dialog.ssmp_inbox.set(str(ROOT/'qa/dialog-inbox'))
    dialog.apply();root.update()
    saved=json.loads(settings.read_text(encoding='utf-8'))
    assert saved['ssmp_enabled'] and saved['ssmp_inbox']==str(ROOT/'qa/dialog-inbox')
    assert app.get_options().ssmp_enabled
    (ROOT/'qa/downloader-settings.json').write_text(json.dumps({'default_disabled':True,'dialog_applied':True,'saved':True,'engine_options_connected':True},indent=2),encoding='utf-8')
    print('DOWNLOADER_SETTINGS_OK: default off; real dialog applies and persists the SSMP handoff settings; engine receives options.')
finally:
    root.destroy()
