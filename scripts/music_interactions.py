"""Prepare an isolated long-running native window for human/Computer Use interaction QA.

This script launches the test process only. UI inputs use the Computer Use API.
"""
from pathlib import Path
import json,math,os,sqlite3,struct,subprocess,sys,time,wave
ROOT=Path(__file__).resolve().parents[1]
DATA=ROOT/'qa/music-interactive'
MEDIA=ROOT/'qa/music-interactive-media'
EXE=Path(os.environ.get('SSMP_QA_EXE',str(ROOT/'dist/SSMP-0.2.0-windows-x64/SSMP.exe')))
MEDIA.mkdir(exist_ok=True);DATA.mkdir(exist_ok=True)
def wav(path,frequency):
    if path.exists():return
    with wave.open(str(path),'wb')as f:
        f.setparams((1,2,16000,0,'NONE','not compressed'))
        second=b''.join(struct.pack('<h',int(120*math.sin(2*math.pi*frequency*n/16000)))for n in range(16000))
        for _ in range(180):f.writeframesraw(second)
if sys.argv[1:] == ['prepare']:
    for n in range(2):wav(MEDIA/f'Track {n+1}.wav',220+n*110)
    subprocess.run([str(EXE),'--data',str(DATA),'--scan',str(MEDIA)],check=True,capture_output=True,creationflags=subprocess.CREATE_NO_WINDOW)
    with sqlite3.connect(DATA/'library.sqlite3')as db:
        for index,(id,raw)in enumerate(db.execute('SELECT id,data FROM media').fetchall()):
            m=json.loads(raw);m.update(artist='SSMP Studio',album='Playback study',genre='Test tone',lyrics='[00:00.00]Generated test audio\n[00:30.00]Playback keeps going\n[01:00.00]A fresh start',title=f'Test track {index+1}')
            db.execute('UPDATE media SET data=? WHERE id=?',(json.dumps(m),id))
        # This test-only state enables the watcher and measurements in a disposable profile.
        state={'onboarding':True,'roots':[str(MEDIA)],'preferences':{'history':True,'watch_folders':True,'online_metadata':False,'design_version':2,'theme':'Ocean'}}
        db.execute('INSERT INTO states VALUES(?,?) ON CONFLICT(user_id) DO UPDATE SET data=excluded.data',('local',json.dumps(state)))
    env=dict(os.environ,SSMP_SMOKE_KEEP_OPEN='1',SSMP_SCREENSHOT=str(ROOT/'qa/music-interactive-start.png'))
    process=subprocess.Popen([str(EXE),'--data',str(DATA),'--smoke-path',str(MEDIA),'--smoke-page','music'],cwd=ROOT,env=env,creationflags=subprocess.CREATE_NO_WINDOW)
    (DATA/'pid.txt').write_text(str(process.pid))
    print('INTERACTIVE_QA_PID='+str(process.pid),flush=True)
elif sys.argv[1:] == ['status']:
    print((DATA/'interaction.json').read_text())
elif sys.argv[1:] == ['add-track']:
    wav(MEDIA/'Watched track.wav',660);print('WATCH_FIXTURE_ADDED')
else:raise SystemExit('Use prepare, status or add-track')
