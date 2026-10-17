"""Verify new extraction, bundled playback, source identity and isolated rollback."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,time,tomllib,zipfile
ROOT=Path(__file__).resolve().parents[1]
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
OUT=ROOT/f'qa/music-extracted-{int(time.time())}';OUT.mkdir()
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
records=[]
def run(*command,env=None):
    result=subprocess.run(list(map(str,command)),cwd=OUT,env=env,capture_output=True,text=True,encoding='utf-8',timeout=45,creationflags=subprocess.CREATE_NO_WINDOW)
    records.append({'command':subprocess.list2cmdline(list(map(str,command))),'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr})
    (ROOT/'qa/music-package-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
    assert result.returncode==0,records[-1]
    return result.stdout
for kind in ['windows-x64','source']:
    path=ROOT/f'dist/SSMP-{VERSION}-{kind}.zip'
    with zipfile.ZipFile(path)as z:
        assert z.testzip()is None
        for name in z.namelist():assert (OUT/name).resolve().is_relative_to(OUT.resolve()),name
        z.extractall(OUT)
app=OUT/f'SSMP-{VERSION}-windows-x64';source=OUT/f'SSMP-{VERSION}-source'
for name,expected in json.loads((app/'tools/runtime-hashes.json').read_text()).items():assert digest(app/name)==expected,name
assert (source/'SOURCE-TREE.txt').read_text()==(app/'SOURCE-TREE.txt').read_text()
env=dict(os.environ,SSMP_LIBMPV=str(app/'tools/mpv/libmpv-2.dll'))
assert "Slulisee's Simple Music Player"in run(app/'SSMP.exe','--data',OUT/'data','--version',env=env)
assert '"added": 5'in run(app/'SSMP.exe','--data',OUT/'data','--scan',ROOT/'qa/media',env=env)
assert 'GENUINE_ERROR_AND_RECOVERY=true'in run(app/'SSMP.exe','--data',OUT/'data','--playback-regression',ROOT/'qa/media/First Light/A Place to Begin.wav',env=env)
shot=OUT/'extracted-ui.png';smoke=dict(env,SSMP_SCREENSHOT=str(shot))
run(app/'SSMP.exe','--data',OUT/'ui','--smoke-path',ROOT/'qa/media','--smoke-page','playlists',env=smoke)
proof=json.loads(shot.with_suffix('.json').read_text());assert proof['engine_available']and not proof['player_error']and proof['position']>0,proof
before=digest(app/'SSMP.exe');target=OUT/'rollback-copy';shutil.copytree(source,target)
for name in ['SSMP.exe','libunwind.dll']:shutil.copy2(app/name,target/name)
shutil.copytree(app/'tools',target/'tools',dirs_exist_ok=True)
sentinel=target/'my-music.bin';sentinel.write_bytes(b'User media must survive rollback.');sentinel_hash=digest(sentinel)
bash=Path('C:/Program Files/Git/bin/bash.exe')
out=run(bash,app/'ROLLBACK-MUSIC.sh','--snapshot',app/'rollback/before','--target',target)
assert 'MUSIC_ROLLBACK_OK'in out
assert 'SSMP 0.1.0'in run(target/'SSMP.exe','--data',OUT/'rollback-data','--version',env=env)
assert 'PLAYBACK_ADVANCED=true'in run(target/'SSMP.exe','--data',OUT/'rollback-data','--self-test',ROOT/'qa/media/First Light/A Place to Begin.wav',env=env)
assert digest(sentinel)==sentinel_hash and digest(app/'SSMP.exe')==before
records.append({'check':'all packaged runtime hashes; matching source tree; native extracted UI; old source/executable rollback; preserved media sentinel; current executable unchanged','result':'PASS'})
(ROOT/'qa/music-package-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print('MUSIC_EXTRACTED_PACKAGES_OK: runtime hashes, native UI, seeking/error recovery, source identity, byte-for-byte rollback, restored 0.1 playback, preserved media and unchanged 0.2 executable.')
