"""Assemble reviewable local milestone artifacts and verify every ZIP CRC."""
from pathlib import Path
import hashlib, json, shutil, stat, tomllib, zipfile

ROOT=Path(__file__).resolve().parents[1]
DIST=ROOT/'dist'
APP=DIST/'SSMP-0.1.0-windows-x64'
DOWN=ROOT/'SSYTDLP'
BASE=ROOT/'work/baseline/SSYTDLP'

def copy(src,dst):
    dst.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(src,dst)

def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda:f.read(1048576),b''):h.update(block)
    return h.hexdigest()

def files(folder):
    return [p for p in folder.rglob('*') if p.is_file() and '__pycache__' not in p.parts and p.suffix!='.pyc']

def archive(output,entries):
    with zipfile.ZipFile(output,'w',zipfile.ZIP_DEFLATED,compresslevel=5) as z:
        for p,name in entries:
            info=zipfile.ZipInfo.from_file(p,str(name).replace('\\','/'),strict_timestamps=False)
            info.compress_type=zipfile.ZIP_DEFLATED
            if p.suffix=='.sh':
                info.create_system=3;info.external_attr=(stat.S_IFREG|0o755)<<16
            with p.open('rb') as src,z.open(info,'w',force_zip64=True) as dst:shutil.copyfileobj(src,dst)
    with zipfile.ZipFile(output) as z:assert z.testzip() is None,output
    print(f'ZIP_CRC_OK={output.name}',flush=True)

assert (APP/'SSMP.exe').is_file()
for name in ['README.md','SPECIFICATION.md','RELEASE-CHECKLIST.md','LICENSE','VERIFICATION.txt']:
    copy(ROOT/name,APP/name)
shutil.copytree(ROOT/'docs',APP/'docs',dirs_exist_ok=True)
evidence=['native-results.json','native-ui-final-results.json','native-tests.log','native-ui-final.log','core-tests-final.log','change-results.json','downloader-e2e.json','downloader-settings.json','antivirus-scan.log','antivirus-downloader.log']
for name in evidence:copy(ROOT/'qa'/name,APP/'qa'/name)
shutil.copytree(ROOT/'qa/screenshots',APP/'qa/screenshots',dirs_exist_ok=True)
copy(ROOT/'tools/provenance.json',APP/'tools/provenance.json')
shutil.copytree(DOWN/'licenses/ffmpeg',APP/'licenses/ffmpeg',dirs_exist_ok=True)
copy(ROOT/'LICENSE',APP/'licenses/mpv/GPL-3.0.txt')
copy(ROOT/'tools/runtime-licenses/libunwind.txt',APP/'licenses/llvm-minGW/LICENSE.TXT')

locked=tomllib.loads((ROOT/'Cargo.lock').read_text(encoding='utf-8'))['package']
registry=ROOT/'work/cargo/registry/src'
notices=[]
for pkg in locked:
    if not pkg.get('source','').startswith('registry+'):continue
    folder=next(registry.glob(f"*/{pkg['name']}-{pkg['version']}"),None)
    if folder is None:
        notices.append({'name':pkg['name'],'version':pkg['version'],'source':pkg['source'],'status':'not fetched on this target'})
        continue
    meta=tomllib.loads((folder/'Cargo.toml').read_text(encoding='utf-8'))['package']
    license_files=[]
    for p in folder.rglob('*'):
        if p.is_file() and p.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE')):
            rel=p.relative_to(folder)
            copy(p,APP/'licenses/rust'/folder.name/rel);license_files.append(str(rel))
    notices.append({'name':pkg['name'],'version':pkg['version'],'license':meta.get('license'),'repository':meta.get('repository'),'license_files':license_files})
(APP/'licenses/rust/packages.json').write_text(json.dumps(notices,indent=2),encoding='utf-8')
runtime={str(p.relative_to(APP)):digest(p) for p in files(APP) if p.suffix.lower() in {'.exe','.dll'}}
(APP/'tools/runtime-hashes.json').write_text(json.dumps(runtime,indent=2),encoding='utf-8')

snapshot=DOWN/'rollback/before-ssmp'
shutil.copytree(BASE/'source',snapshot/'source',dirs_exist_ok=True,ignore=shutil.ignore_patterns('__pycache__','*.pyc'))
for name in ['SSYTDLP.exe','LICENSE.txt']:copy(BASE/name,snapshot/name)
copy(ROOT/'scripts/restore_downloader.py',DOWN/'rollback/restore.py')
copy(ROOT/'VERIFICATION.txt',DOWN/'VERIFICATION.txt')
copy(ROOT/'CHANGES.diff',DOWN/'CHANGES.diff')
sum_files=[]
for p in files(DOWN):
    rel=p.relative_to(DOWN)
    if any(x in rel.parts for x in ['build','dist','native-artifacts']) or p.name=='SHA256SUMS.txt':continue
    if rel.parts[0]=='rollback' and (len(rel.parts)<2 or rel.parts[1] not in {'before-ssmp','restore.py'}):continue
    sum_files.append(p)
(DOWN/'SHA256SUMS.txt').write_text(''.join(f'{digest(p)}  {p.relative_to(DOWN).as_posix()}\n' for p in sum_files),encoding='utf-8')
archive(DIST/'SSMP-0.1.0-windows-x64.zip',[(p,Path(APP.name)/p.relative_to(APP)) for p in files(APP)])
down_entries=[]
for p in sum_files+[DOWN/'SHA256SUMS.txt']:
    rel=p.relative_to(DOWN)
    if rel.parts[0]=='rollback' and (len(rel.parts)<2 or rel.parts[1] not in {'before-ssmp','restore.py'}):continue
    down_entries.append((p,Path('SSYTDLP')/rel))
archive(DIST/'SSYTDLP-with-SSMP-windows-x64.zip',down_entries)

source=[]
for name in ['Cargo.toml','Cargo.lock','build.rs','README.md','SPECIFICATION.md','RELEASE-CHECKLIST.md','LICENSE','ROLLBACK.sh','CHANGES.diff','VERIFICATION.txt','.gitignore']:
    source.append((ROOT/name,Path('SSMP-source')/name))
for folder in ['src','assets','docs','scripts','.github','SSYTDLP/source','work/baseline/SSYTDLP']:
    source.extend((p,Path('SSMP-source')/p.relative_to(ROOT)) for p in files(ROOT/folder))
for name in ['README.md','SSMP-INTEGRATION.md','LICENSE.txt']:
    source.append((DOWN/name,Path('SSMP-source/SSYTDLP')/name))
source.append((ROOT/'tools/provenance.json',Path('SSMP-source/tools/provenance.json')))
source.append((ROOT/'tools/runtime-licenses/libunwind.txt',Path('SSMP-source/tools/runtime-licenses/libunwind.txt')))
for name in evidence:source.append((ROOT/'qa'/name,Path('SSMP-source/qa')/name))
archive(DIST/'SSMP-0.1.0-source.zip',source)
artifacts={p.name:{'bytes':p.stat().st_size,'sha256':digest(p)} for p in DIST.glob('*.zip')}
(DIST/'SHA256SUMS.txt').write_text(''.join(f"{v['sha256']}  {k}\n" for k,v in artifacts.items()),encoding='utf-8')
(ROOT/'qa/artifacts.json').write_text(json.dumps(artifacts,indent=2),encoding='utf-8')
print('PACKAGES_VERIFIED='+str(len(artifacts)),flush=True)
