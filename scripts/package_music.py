"""Package the reviewed SSMP Git index, runtime, evidence and verified rollback snapshot."""
from pathlib import Path
import hashlib,json,os,shutil,stat,subprocess,tomllib,zipfile
ROOT=Path(__file__).resolve().parents[1]
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
APP=ROOT/f'dist/SSMP-{VERSION}-windows-x64'
INDEX=ROOT/'work/music-release.index'
BASE=ROOT/'work/music-v02-before'
def digest(path):
    h=hashlib.sha256()
    with path.open('rb')as f:
        for b in iter(lambda:f.read(1048576),b''):h.update(b)
    return h.hexdigest()
def copy(src,dst):dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst)
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT,env=dict(os.environ,GIT_INDEX_FILE=str(INDEX)))
def archive(path,entries):
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED,compresslevel=5)as z:
        for source,name in entries:
            info=zipfile.ZipInfo.from_file(source,str(name).replace('\\','/'),strict_timestamps=False);info.compress_type=zipfile.ZIP_DEFLATED
            if source.suffix=='.sh':info.create_system=3;info.external_attr=(stat.S_IFREG|0o755)<<16
            with source.open('rb')as src,z.open(info,'w',force_zip64=True)as dst:shutil.copyfileobj(src,dst)
    with zipfile.ZipFile(path)as z:assert z.testzip()is None
    print('MUSIC_ZIP_CRC_OK='+path.name,flush=True)
def main():
    assert (APP/'SSMP.exe').is_file()and INDEX.is_file()
    source_root=ROOT/f'work/SSMP-{VERSION}-source';source_root.mkdir(exist_ok=True)
    # Only files from the isolated reviewed index enter source and documentation.
    index=[]
    for line in git('ls-files','-s').decode().splitlines():
        metadata,name=line.split('\t',1);mode,sha,stage=metadata.split()
        if stage!='0':raise RuntimeError('Unresolved index')
        path=source_root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(git('cat-file','blob',sha));index.append(name)
    for name in ['README.md','CHANGELOG.md','SPECIFICATION.md','RELEASE-CHECKLIST.md','LICENSE','ROLLBACK-MUSIC.sh']:
        copy(source_root/name,APP/name)
    copy(source_root/'ROLLBACK-MUSIC.sh',APP/'ROLLBACK.sh')
    for folder in ['docs','scripts']:
        for path in (source_root/folder).rglob('*'):
            if path.is_file():copy(path,APP/path.relative_to(source_root))
    old=ROOT/'dist/SSMP-0.1.0-windows-x64'
    shutil.copytree(old/'licenses',APP/'licenses',dirs_exist_ok=True)
    # Refresh notices for the locked desktop and playback dependencies.
    registry=ROOT/'work/cargo/registry/src';notices=[]
    for package in tomllib.loads((source_root/'Cargo.lock').read_text(encoding='utf-8'))['package']:
        if not package.get('source','').startswith('registry+'):continue
        folder=next(registry.glob(f"*/{package['name']}-{package['version']}"),None)
        if folder is None:
            notices.append({'name':package['name'],'version':package['version'],'source':package['source'],'status':'not fetched on this target'});continue
        metadata=tomllib.loads((folder/'Cargo.toml').read_text(encoding='utf-8'))['package'];license_files=[]
        for path in folder.rglob('*'):
            if path.is_file()and path.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE')):
                relative=path.relative_to(folder);copy(path,APP/'licenses/rust'/folder.name/relative);license_files.append(relative.as_posix())
        notices.append({'name':package['name'],'version':package['version'],'license':metadata.get('license'),'repository':metadata.get('repository'),'license_files':license_files})
    (APP/'licenses/rust/packages.json').write_text(json.dumps(notices,indent=2),encoding='utf-8')
    snapshot=APP/'rollback/before';snapshot.mkdir(parents=True,exist_ok=True)
    baseline={}
    for path in (BASE/'src').rglob('*'):
        if path.is_file():baseline[path.relative_to(BASE).as_posix()]=path
    for name in ['Cargo.toml','Cargo.lock','README.md','CHANGELOG.md','SSMP.exe']:baseline[name]=BASE/name
    for name,source in [('SPECIFICATION.md','docs/SPECIFICATION.md'),('RELEASE-CHECKLIST.md','docs/RELEASE-CHECKLIST.md'),('docs/ARCHITECTURE.md','docs/ARCHITECTURE.md'),('docs/IMPORTS.md','docs/IMPORTS.md'),('docs/HOSTING.md','docs/HOSTING.md')]:baseline[name]=BASE/source
    # Unmodified-at-start resources and QA/build helpers are recovered from the saved base commit.
    base_commit=(ROOT/'work/music-base-commit.txt').read_text().strip()
    for name in ['assets/SSMP.rc','assets/SSMP.manifest','assets/SSMP.png','assets/SSMP.ico','build.rs','scripts/qa.py','docs/THIRD-PARTY.md','LICENSE']:
        path=BASE/'extra'/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(subprocess.check_output(['git','show',f'{base_commit}:{name}'],cwd=ROOT));baseline[name]=path
    before_hashes={}
    for name,path in baseline.items():copy(path,snapshot/name);before_hashes[name]=digest(path)
    # Rollback removes only files introduced by this release, after checking their exact hashes.
    added_names=json.loads((ROOT/'work/music-added-files.json').read_text())
    additions={name:hashlib.sha256(git('show',':'+name)).hexdigest()for name in added_names}
    modified={name:hashlib.sha256(git('show',':'+name)).hexdigest()for name in baseline if name in index}
    modified['SSMP.exe']=digest(APP/'SSMP.exe')
    (snapshot/'snapshot.json').write_text(json.dumps({'files':before_hashes,'added':additions,'modified':modified},indent=2),encoding='utf-8')
    tree=git('write-tree').decode().strip()
    (APP/'SOURCE-TREE.txt').write_text(tree+'\n',encoding='utf-8')
    (source_root/'SOURCE-TREE.txt').write_text(tree+'\n',encoding='utf-8')
    evidence=['MUSIC-VERIFICATION.txt','music-ui-results.json','music-ui.log','music-playback-results.json','music-playback.log','music-interaction-results.json','music-tests.log','music-core-results.json','music-core.log','music-performance-results.json','music-performance.log','music-renderer-results.json','music-software-idle.json','music-minimized-results.json','music-minimized.log']
    for name in evidence:
        if (ROOT/'qa'/name).exists():copy(ROOT/'qa'/name,APP/'qa'/name)
    if (ROOT/'qa/MUSIC-VERIFICATION.txt').exists():copy(ROOT/'qa/MUSIC-VERIFICATION.txt',APP/'MUSIC-VERIFICATION.txt')
    if (ROOT/'qa/music-screenshots').exists():shutil.copytree(ROOT/'qa/music-screenshots',APP/'qa/music-screenshots',dirs_exist_ok=True)
    for name in ['provenance.json','runtime-licenses/libunwind.txt']:
        if (ROOT/'tools'/name).exists():copy(ROOT/'tools'/name,APP/'tools'/name)
    hashes={p.relative_to(APP).as_posix():digest(p)for p in APP.rglob('*')if p.is_file()and p.suffix.lower()in{'.exe','.dll'}}
    (APP/'tools/runtime-hashes.json').write_text(json.dumps(hashes,indent=2),encoding='utf-8')
    paths=[(p,Path(APP.name)/p.relative_to(APP))for p in APP.rglob('*')if p.is_file()and '__pycache__'not in p.parts]
    archive(ROOT/f'dist/SSMP-{VERSION}-windows-x64.zip',paths)
    paths=[(source_root/name,Path(source_root.name)/name)for name in index]
    paths.append((source_root/'SOURCE-TREE.txt',Path(source_root.name)/'SOURCE-TREE.txt'))
    archive(ROOT/f'dist/SSMP-{VERSION}-source.zip',paths)
    artifacts={p.name:{'bytes':p.stat().st_size,'sha256':digest(p)}for p in [ROOT/f'dist/SSMP-{VERSION}-windows-x64.zip',ROOT/f'dist/SSMP-{VERSION}-source.zip']}
    (ROOT/f'dist/SHA256SUMS-MUSIC-{VERSION}.txt').write_text(''.join(f'{v["sha256"]}  {name}\n'for name,v in artifacts.items()),encoding='utf-8')
    (ROOT/'qa/music-artifacts.json').write_text(json.dumps({'tree':tree,'artifacts':artifacts},indent=2),encoding='utf-8')
    print('MUSIC_PACKAGES_OK=2',flush=True)
if __name__=='__main__':main()
