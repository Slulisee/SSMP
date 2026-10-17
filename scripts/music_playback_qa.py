"""Wait for GUI-subsystem CLI checks and preserve their real exit codes/output."""
from pathlib import Path
import json,subprocess,tomllib,os,time
ROOT=Path(__file__).resolve().parents[1]
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
EXE=Path(os.environ.get('SSMP_QA_EXE',str(ROOT/f'dist/SSMP-{VERSION}-windows-x64/SSMP.exe')))
AUDIO=ROOT/'qa/media/First Light/A Place to Begin.wav'
VIDEO=ROOT/'qa/media/Motion Studies/Motion.Study.S01E01.mkv'
records=[]
for args,expected in [(['--playback-regression',str(AUDIO)],'GENUINE_ERROR_AND_RECOVERY=true'),(['--music-video-test',str(AUDIO),'--video',str(VIDEO)],'AUDIO_CONTINUES_WHEN_HIDDEN=true')]:
    command=[str(EXE),'--data',str(ROOT/'qa/music-regression'),*args]
    result=subprocess.run(command,cwd=ROOT,capture_output=True,text=True,encoding='utf-8',timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
    records.append({'command':subprocess.list2cmdline(command),'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr})
    (ROOT/'qa/music-playback-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
    print(result.stdout,end='',flush=True)
    assert result.returncode==0 and expected in result.stdout,records[-1]
isolated=ROOT/f'qa/music-error-{time.time_ns()}'
command=[str(EXE),'--data',str(isolated),'--watch',str(isolated/'not-a-folder'),'--serve']
result=subprocess.run(command,cwd=ROOT,capture_output=True,text=True,encoding='utf-8',timeout=10,creationflags=subprocess.CREATE_NO_WINDOW)
assert result.returncode!=0 and (isolated/'last-error.txt').is_file()
records.append({'command':subprocess.list2cmdline(command),'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr,'expected_failure':True,'isolated_error_log':str(isolated/'last-error.txt')})
(ROOT/'qa/music-playback-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print('EXPLICIT_DATA_DIAGNOSTICS_ISOLATED=true',flush=True)
print('MUSIC_PLAYBACK_REGRESSIONS_OK',flush=True)
