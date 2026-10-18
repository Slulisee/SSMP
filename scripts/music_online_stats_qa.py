"""Black-box online provider checks and calendar statistics using disposable records.

Only public track identifiers go online. Provider lyric text stays in the local
QA profile; the evidence file contains presence/hash facts, never lyric text.
"""
from pathlib import Path
from datetime import datetime, timezone
import hashlib,json,os,sqlite3,subprocess,time,tomllib
ROOT=Path(__file__).resolve().parents[1]
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
EXE=ROOT/f'dist/SSMP-{VERSION}-windows-x64/SSMP.exe'
DATA=ROOT/f'qa/v03-online-{time.time_ns()}'
records=[]
def run(*args):
    command=[str(EXE),'--data',str(DATA),*args]
    result=subprocess.run(command,capture_output=True,text=True,encoding='utf-8',timeout=90,creationflags=subprocess.CREATE_NO_WINDOW)
    records.append({'command':subprocess.list2cmdline(command),'exit':result.returncode})
    assert result.returncode==0,result.stderr
    return result.stdout
run('--version')
with sqlite3.connect(DATA/'library.sqlite3') as db:
    m={'id':'public-identifier','path':str(ROOT/'qa/media/First Light/A Place to Begin.wav'),'kind':'audio','title':'Take On Me','artist':'a-ha','album':'Deliberately unmatched QA album','duration':226.,'locked':True,'tag_version':2}
    db.execute('INSERT INTO media VALUES(?,?,?)',(m['id'],m['path'],json.dumps(m)))
online=json.loads(run('--enrich',m['id']))
assert online['locked'] and online['album']==m['album'] and online['musicbrainz_id'] and online['artist_id']
assert online['genre'].strip(), 'Expected curated genres for this widely indexed recording/artist'
lyric=json.loads(run('--find-lyrics',m['id']))
assert lyric['lyrics'].strip() and '[' in lyric['lyrics'], 'Expected synced search fallback despite mismatched local album'
facts={'metadata_source':online['metadata_source'],'recording_id_found':bool(online['musicbrainz_id']),'artist_id_found':bool(online['artist_id']),'genre':online['genre'],'manual_album_preserved':online['album']==m['album'],'manual_lock_preserved':online['locked'],'synced_lyrics_found':True,'lyrics_sha256':hashlib.sha256(lyric['lyrics'].encode()).hexdigest()}
day=lambda text:int(datetime.fromisoformat(text).replace(tzinfo=timezone.utc).timestamp())//86400
with sqlite3.connect(DATA/'library.sqlite3')as db:
    state={'stats_since':day('2025-01-01')*86400,'stats_detail_since':day('2026-09-01')*86400,'listening_seconds':{m['id']:1800.},'counts':{m['id']:6},'listening_days':{str(day('2026-09-30')):600.,str(day('2026-10-01')):1200.},'months':{'202609':{'seconds':{m['id']:600.},'plays':{m['id']:2}},'202610':{'seconds':{m['id']:1200.},'plays':{m['id']:4}}},'first_heard':{m['id']:day('2026-09-30')*86400}}
    db.execute('INSERT OR REPLACE INTO states VALUES(?,?)',('local',json.dumps(state)))
for options,seconds,plays in [(['--month','2026-09'],600.,2),(['--month','2026-10'],1200.,4),(['--year','2026'],1800.,6),([],1800.,6)]:
    report=json.loads(run('--statistics',*options))
    assert report['seconds']==seconds and report['plays']==plays and report['top_tracks'][0]['seconds']==seconds
facts.update(month_and_year_totals_verified=True,provider_text_published=False,profile=str(DATA),commands=records)
(ROOT/'qa/v03-online-stats-results.json').write_text(json.dumps(facts,indent=2),encoding='utf-8')
print('SSMP_LIVE_LOOKUPS_AND_PERIOD_STATS_OK: curated genres, protected manual fields, synced lyric search fallback, exact monthly/yearly totals.')
