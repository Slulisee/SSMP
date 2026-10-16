"""Verify ZIPs, extracted native apps and the standalone downloader rollback."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,time,zipfile
ROOT=Path(__file__).resolve().parents[1]
QA=ROOT/'qa'
RUN=QA/('extracted-'+str(int(time.time())))
ENV=os.environ.copy();ENV.update(TEMP=str(ROOT/'work/temp'),TMP=str(ROOT/'work/temp'))
records=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(args):
    r=subprocess.run(list(map(str,args)),cwd=RUN,env=ENV,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=40,creationflags=subprocess.CREATE_NO_WINDOW)
    records.append({'command':subprocess.list2cmdline(list(map(str,args))),'stdout':r.stdout,'stderr':r.stderr,'exit':r.returncode})
    assert r.returncode==0,records[-1]
    return r.stdout

RUN.mkdir(parents=True)
for name in ['SSMP-0.1.0-windows-x64.zip','SSYTDLP-with-SSMP-windows-x64.zip']:
    with zipfile.ZipFile(ROOT/'dist'/name) as z:
        assert z.testzip() is None
        assert all(not Path(n).is_absolute() and '..' not in Path(n).parts for n in z.namelist())
        z.extractall(RUN)
app=RUN/'SSMP-0.1.0-windows-x64'
down=RUN/'SSYTDLP'
for name,digest in json.loads((app/'tools/runtime-hashes.json').read_text()).items():assert sha(app/name)==digest,name
for line in (down/'SHA256SUMS.txt').read_text().splitlines():
    digest,name=line.split('  ',1);assert sha(down/name)==digest,name
assert 'SSMP 0.1.0' in run([app/'SSMP.exe','--data',RUN/'library','--version'])
scan=json.loads(run([app/'SSMP.exe','--data',RUN/'library','--scan',QA/'media']))
assert not scan['errors'],scan
ENV['SSMP_LIBMPV']=str(app/'tools/mpv/libmpv-2.dll')
proof=run([app/'SSMP.exe','--data',RUN/'library','--self-test',QA/'media/Motion Studies/Motion.Study.S01E01.mkv'])
assert 'PLAYBACK_ADVANCED=true' in proof and 'NONZERO_PIXELS=true' in proof
run([down/'SSYTDLP.exe','--smoke-test',RUN/'downloader-smoke.json'])
assert json.loads((RUN/'downloader-smoke.json').read_text())['mapped']==1
target=RUN/'rollback-copy'
shutil.copytree(down/'source',target/'source',ignore=shutil.ignore_patterns('__pycache__'))
shutil.copy2(down/'SSYTDLP.exe',target/'SSYTDLP.exe')
sentinel=target/'media.txt';sentinel.write_text('retain generated sample')
run([r'C:\Program Files\Git\bin\bash.exe',down/'ROLLBACK.sh',target])
assert sha(target/'SSYTDLP.exe')==sha(down/'rollback/before-ssmp/SSYTDLP.exe')
for p in (down/'rollback/before-ssmp/source').rglob('*'):
    if p.is_file() and '__pycache__' not in p.parts:assert sha(p)==sha(target/'source'/p.relative_to(down/'rollback/before-ssmp/source'))
assert not (target/'source/ssmp_bridge.py').exists() and (down/'source/ssmp_bridge.py').exists()
assert sentinel.read_text()=='retain generated sample'
(QA/'package-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print('EXTRACTED_PACKAGES_OK: all manifest hashes; SSMP version, scan, video playback/seek/render; native SSYTDLP window; standalone rollback hashes and preserved media.',flush=True)
