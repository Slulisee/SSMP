"""Exercise the real Windows installer in a fresh isolated target.

/NOINTEGRATION keeps this file/preservation check out of the user's Start menu
and installed-app registration. Actual product startup and uninstall still run.
"""
from pathlib import Path
import hashlib,json,os,subprocess,time
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/f'qa/music-install-{time.time_ns()}';OUT.mkdir(parents=True)
TARGET=OUT/'App with spaces';SETUP=ROOT/'dist/SSMP-Windows-AMD64-Setup.exe'
records=[]
def sha(path):
    with path.open('rb')as f:return hashlib.file_digest(f,'sha256').hexdigest()
def run(exe,args,expected=0,env=None):
    command=subprocess.list2cmdline([str(exe)])+' '+args
    r=subprocess.run(command,executable=str(exe),cwd=ROOT,env=env,capture_output=True,text=True,encoding='utf-8',timeout=180)
    records.append({'command':command,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr,'expected_exit':expected})
    (ROOT/'qa/music-installer-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
    assert r.returncode==expected,records[-1];return r.stdout
def smoke(name):
    shot=OUT/(name+'.png');env=dict(os.environ,SSMP_SCREENSHOT=str(shot))
    run(TARGET/'SSMP.exe',subprocess.list2cmdline(['--data',str(OUT/'profile'),'--smoke-path',str(ROOT/'qa/media'),'--smoke-page','playlists']),env=env)
    proof=json.loads(shot.with_suffix('.json').read_text(encoding='utf-8'));assert proof['engine_available']and not proof['player_error']and proof['position']>0,proof
manifest=json.loads((ROOT/'work/music-installer/manifest.json').read_text(encoding='utf-8'))
run(SETUP,'/S /NOINTEGRATION /D='+str(TARGET))
for name,expected in manifest.items():assert sha(TARGET/name)==expected,name
smoke('fresh')
sentinel=TARGET/'my-music.txt';sentinel.write_text('Preserve this user file.',encoding='utf-8')
before=sha(sentinel);library_before=sha(OUT/'profile/library.sqlite3')
(TARGET/'README.md').write_text('Old app file fixture',encoding='utf-8')
run(SETUP,'/S /NOINTEGRATION /D='+str(TARGET))
assert sha(TARGET/'README.md')==manifest['README.md'];assert sha(OUT/'profile/library.sqlite3')==library_before;assert sha(sentinel)==before
smoke('upgrade')
run(TARGET/'Uninstall.exe','/S _?='+str(TARGET))
assert not(TARGET/'SSMP.exe').exists();assert sha(sentinel)==before;assert(OUT/'profile/library.sqlite3').is_file()
records.append({'check':'fresh native install, every installed file hash, native playback/UI, upgrade retention, explicit-list uninstall, preserved library and user sentinel','result':'PASS','integration':'NOINTEGRATION used to preserve user system entries'})
(ROOT/'qa/music-installer-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print('SSMP_INSTALLER_OK: fresh install and playback, upgrade, explicit-list uninstall, preserved library and user files.')
