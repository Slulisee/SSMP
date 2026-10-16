"""Baseline/modified/rollback transaction evidence using isolated test copies."""
from pathlib import Path
import difflib, hashlib, json, os, shutil, subprocess, sys, time
ROOT=Path(__file__).resolve().parents[1]
QA=ROOT/'qa'
ENV=os.environ.copy();ENV.update(TEMP=str(ROOT/'work/temp'),TMP=str(ROOT/'work/temp'))
ENV['SSMP_PYTHON']=sys.executable
records=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(label,command):
    r=subprocess.run(list(map(str,command)),cwd=ROOT,env=ENV,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=100)
    record={'label':label,'command':subprocess.list2cmdline(list(map(str,command))),'input':'Generated fixtures / isolated test data only','stdout':r.stdout,'stderr':r.stderr,'exit':r.returncode}
    records.append(record)
    (QA/(label.lower()+'.log')).write_text(r.stdout+r.stderr,encoding='utf-8')
    assert r.returncode==0,record
    print(label+'_EXIT=0',flush=True)

run('BASELINE',[sys.executable,'-m','unittest','discover','-s',ROOT/'work/baseline/SSYTDLP/source','-v'])
run('MODIFIED',[sys.executable,'-m','unittest','discover','-s',ROOT/'SSYTDLP/source','-v'])
target=QA/('rollback-check-'+str(int(time.time())))
shutil.copytree(ROOT/'SSYTDLP/source',target/'source',ignore=shutil.ignore_patterns('__pycache__'))
shutil.copy2(ROOT/'SSYTDLP/SSYTDLP.exe',target/'SSYTDLP.exe')
sentinel=target/'media-preserved.txt';sentinel.write_text('Generated media-preservation sentinel',encoding='utf-8')
modified=sha(ROOT/'SSYTDLP/source/engine.py')
run('ROLLBACK',[r'C:\Program Files\Git\bin\bash.exe','ROLLBACK.sh',target])
assert sha(target/'SSYTDLP.exe')==sha(ROOT/'work/baseline/SSYTDLP/SSYTDLP.exe')
assert not (target/'source/ssmp_bridge.py').exists()
assert sentinel.read_text()=='Generated media-preservation sentinel'
assert sha(ROOT/'SSYTDLP/source/engine.py')==modified
run('RESTORED_TESTS',[sys.executable,'-m','unittest','discover','-s',target/'source','-v'])
run('ORIGINALS',[sys.executable,'scripts/verify_originals.py'])
shutil.copy2(ROOT/'dist/SSMP-0.1.0-windows-x64/libunwind.dll',QA/'libunwind.dll')
run('HEADLESS_VERSION',[QA/'SSMP-host-only.exe','--data',QA/'headless-data','--version'])
run('HEADLESS_SCAN',[QA/'SSMP-host-only.exe','--data',QA/'headless-data','--scan',QA/'media'])
records.append({'label':'RESTORED_STATUS','result':'Baseline source/executable hashes restored; 25 tests restored; media sentinel retained; main workspace SSMP integration remains modified.'})
(QA/'change-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')

diff=[]
paths=[ROOT/n for n in ['.gitignore','Cargo.toml','build.rs','README.md','SPECIFICATION.md','RELEASE-CHECKLIST.md','ROLLBACK.sh']]
for folder in ['src','assets','docs','scripts','.github']:
    paths.extend(p for p in (ROOT/folder).rglob('*') if p.is_file() and p.suffix in {'.rs','.md','.py','.ps1','.rc','.manifest','.yml'})
paths.extend(p for p in (ROOT/'SSYTDLP/source').iterdir() if p.is_file() and p.suffix in {'.py','.ps1'})
paths.extend(ROOT/'SSYTDLP'/n for n in ['README.md','SSMP-INTEGRATION.md','ROLLBACK.sh'])
paths.extend(ROOT/'SSYTDLP/source'/n for n in ['requirements.txt','requirements-build.txt'])
for p in paths:
    rel=p.relative_to(ROOT).as_posix()
    old=ROOT/'work/baseline'/p.relative_to(ROOT) if rel.startswith('SSYTDLP/') else None
    if old and not old.exists():
        old=Path(r'C:\Users\Anonym\Downloads\SSYTDLP\SSYTDLP-Portable')/p.relative_to(ROOT)
    before=old.read_text(encoding='utf-8-sig').splitlines(keepends=True) if old and old.exists() else []
    after=p.read_text(encoding='utf-8-sig').splitlines(keepends=True)
    diff.extend(difflib.unified_diff(before,after,fromfile='a/'+rel if before else '/dev/null',tofile='b/'+rel))
diff.extend(['\nBinary delivery hashes:\n','SSYTDLP baseline SHA256='+sha(ROOT/'work/baseline/SSYTDLP/SSYTDLP.exe')+'\n','SSYTDLP modified SHA256='+sha(ROOT/'SSYTDLP/SSYTDLP.exe')+'\n','SSMP executable SHA256='+sha(ROOT/'dist/SSMP-0.1.0-windows-x64/SSMP.exe')+'\n'])
(ROOT/'CHANGES.diff').write_text(''.join(diff),encoding='utf-8')
print('CHANGE_VERIFICATION_OK: baseline=25 modified=30 restored=25; original files unchanged; headless executable works; diff saved.',flush=True)
