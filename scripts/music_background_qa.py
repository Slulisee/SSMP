"""Measure an already-minimized, isolated QA window without injecting UI input.

Prepare with SSMP_PROFILE=1 and scripts/music_interactions.py, then minimize
the generated test window through Computer Use before running this check.
"""
from pathlib import Path
import hashlib,json,sqlite3,time

ROOT=Path(__file__).resolve().parents[1]
DATA=ROOT/'qa/music-interactive'
EXE=ROOT/'dist/SSMP-0.2.0-windows-x64/SSMP.exe'

def sample():
    with sqlite3.connect(DATA/'library.sqlite3') as db:
        state=json.loads(db.execute('SELECT data FROM states WHERE user_id=?',('local',)).fetchone()[0])
    return {
        'background':json.loads((DATA/'background.json').read_text()),
        'trace_at':json.loads((DATA/'interaction.json').read_text())['at'],
        'saved_listening_seconds':sum(state.get('listening_seconds',{}).values()),
    }

before=sample()
start=time.monotonic()
time.sleep(5)
after=sample()
result={
    'command':'python scripts/music_background_qa.py',
    'input':'Disposable generated WAV/profile; Computer Use confirmed minimization before this read-only check; null audio output.',
    'executable_sha256':hashlib.sha256(EXE.read_bytes()).hexdigest(),
    'elapsed':time.monotonic()-start,
    'before':before,'after':after,
}
result['pass']=(
    after['trace_at']==before['trace_at']
    and after['background']['paint_frames']==before['background']['paint_frames']
    and after['background']['ticks']-before['background']['ticks']>=12
    and after['background']['position']-before['background']['position']>3
    and after['saved_listening_seconds']-before['saved_listening_seconds']>3
)
(ROOT/'qa/music-minimized-results.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
assert result['pass'],result
print('MUSIC_MINIMIZED_OK: playback advances, measured listening time is saved, and painting stays stopped.')
