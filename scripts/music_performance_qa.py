"""Windows software-window workload check, using synthetic metadata and one generated WAV.

The large case measures browsing 10,001 rows, not decoding 10,000 media files.
Results are machine-specific; all profiles are disposable and original music is untouched.
"""
from pathlib import Path
import ctypes,json,os,sqlite3,subprocess,time,tomllib
from ctypes import wintypes
ROOT=Path(__file__).resolve().parents[1]
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
EXE=ROOT/f'dist/SSMP-{VERSION}-windows-x64/SSMP.exe'
OUT=ROOT/f'qa/music-performance-{time.time_ns()}';OUT.mkdir()
audio=ROOT/'qa/music-interactive-media/Track 1.wav'
assert audio.is_file(),'Prepare the generated interactive WAV first.'
kernel=ctypes.WinDLL('kernel32',use_last_error=True)
kernel.GetProcessTimes.argtypes=[wintypes.HANDLE,*([ctypes.POINTER(wintypes.FILETIME)]*4)]
kernel.GetProcessTimes.restype=wintypes.BOOL
def cpu(process):
    fields=[wintypes.FILETIME()for _ in range(4)]
    assert kernel.GetProcessTimes(wintypes.HANDLE(int(process._handle)),*[ctypes.byref(f)for f in fields])
    return sum((f.dwHighDateTime<<32)|f.dwLowDateTime for f in fields[2:])/10_000_000
def proof(data):return json.loads((data/'performance.json').read_text())
records=[]
for count in [3,10001]:
    data=OUT/str(count)
    init=subprocess.run([str(EXE),'--data',str(data),'--version'],capture_output=True,timeout=15,creationflags=subprocess.CREATE_NO_WINDOW)
    assert init.returncode==0
    with sqlite3.connect(data/'library.sqlite3')as db:
        for index in range(count):
            item={'id':f'performance-{index:05}', 'path':str(audio)if index==0 else str(OUT/f'metadata-only/{index}.mp3'),
              'kind':'audio','title':f'Track {index:05}', 'artist':'000 SSMP Studio'if index==0 else f'Artist {index//30:04}',
              'album':f'Album {index//10:04}', 'duration':180.,'genre':f'Genre {index%12}', 'tag_version':2,'locked':True,
              'lyrics':'Synthetic benchmark metadata. '*32,'missing':index!=0}
            db.execute('INSERT OR REPLACE INTO media VALUES(?,?,?)',(item['id'],item['path'],json.dumps(item)))
        state={'onboarding':True,'preferences':{'design_version':2,'watch_folders':False,'online_metadata':False,'theme':'Ocean'}}
        db.execute('INSERT OR REPLACE INTO states VALUES(?,?)',('local',json.dumps(state)))
    command=[str(EXE),'--data',str(data),'--smoke-path',str(ROOT/'qa/media'),'--smoke-page','music']
    process=subprocess.Popen(command,cwd=ROOT,env=dict(os.environ,SSMP_PROFILE='1',SSMP_SMOKE_KEEP_OPEN='1',SSMP_SCREENSHOT=str(data/'window.png')),creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        deadline=time.monotonic()+15
        while not(data/'performance.json').exists():
            assert process.poll()is None,'Player exited during performance check'
            assert time.monotonic()<deadline,'No software-window profiling result'
            time.sleep(.1)
        time.sleep(2)
        before=proof(data);start_cpu=cpu(process);start=time.monotonic();time.sleep(5)
        elapsed=time.monotonic()-start;cost=cpu(process)-start_cpu;after=proof(data)
        interaction=json.loads((data/'interaction.json').read_text())
        frames=after['frames']-before['frames'];work=after['paint_work_seconds']-before['paint_work_seconds']
        row={'metadata_rows':count,'command':subprocess.list2cmdline(command),'backend':after['backend'],
          'elapsed':elapsed,'cpu_seconds':cost,'percent_one_core':100*cost/elapsed,
          'logical_processors':os.cpu_count(),'percent_machine':100*cost/elapsed/os.cpu_count(),
          'paint_frames':frames,'paint_work_seconds':work,'warm_mean_paint_ms':1000*work/max(1,frames),
          'position':interaction['position'],'player_error':interaction['error'],'snapshot':str(data/'window.png')}
        records.append(row);(ROOT/'qa/music-performance-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
        print(json.dumps(row),flush=True)
        assert after['backend']=='Software' and frames>0 and not interaction['error'] and interaction['position']>0
        assert row['warm_mean_paint_ms']<25 and row['percent_one_core']<40,row
    finally:
        if process.poll()is None:process.terminate();process.wait(timeout=8)
print('MUSIC_SOFTWARE_PERFORMANCE_OK: 3 and 10001 synthetic library rows; advancing playback; cached native CPU painting within measured budgets.',flush=True)
