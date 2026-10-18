"""Provision the pinned Windows media runtime for a clean release builder."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,tomllib,urllib.request,zipfile,sys
ROOT=Path(__file__).resolve().parents[1]
def digest(path):
    with path.open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def fetch(url,expected,target):
    if not target.exists():
        target.parent.mkdir(parents=True,exist_ok=True)
        with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'SSMP-build'}),timeout=120)as response,target.open('wb')as out:shutil.copyfileobj(response,out)
    if digest(target)!=expected:raise ValueError('Pinned runtime checksum mismatch: '+target.name)
def main():
    version=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version'];app=ROOT/f'dist/SSMP-{version}-windows-x64';app.mkdir(parents=True,exist_ok=True)
    manifest=json.loads((ROOT/'tools/provenance.json').read_text(encoding='utf-8'));cache=ROOT/'work/runtime-cache';cache.mkdir(parents=True,exist_ok=True)
    archive=cache/'mpv.7z';fetch(manifest['url'],manifest['sha256'],archive)
    seven=shutil.which('7z')or'C:/Program Files/7-Zip/7z.exe';unpacked=cache/'mpv'
    subprocess.run([seven,'x','-y',str(archive),'-o'+str(unpacked)],check=True,stdout=subprocess.DEVNULL)
    runtime=next(unpacked.rglob('libmpv-2.dll'));(app/'tools/mpv').mkdir(parents=True,exist_ok=True);shutil.copy2(runtime,app/'tools/mpv/libmpv-2.dll')
    ff=manifest['ffmpeg'];archive=cache/'ffmpeg.zip';fetch(ff['url'],ff['sha256'],archive)
    (ROOT/'SSYTDLP/tools').mkdir(parents=True,exist_ok=True);notice=app/'licenses/ffmpeg';notice.mkdir(parents=True,exist_ok=True)
    with zipfile.ZipFile(archive)as z:
        for name in ['ffprobe','ffmpeg']:
            member=next(n for n in z.namelist()if n.endswith('/bin/'+name+'.exe'));target=ROOT/'SSYTDLP/tools'/(name+'.exe');target.write_bytes(z.read(member))
            if name=='ffprobe':
                if digest(target)!=ff['ffprobe_sha256']:raise ValueError('ffprobe build differs from pin')
                shutil.copy2(target,app/'tools/ffprobe.exe')
        for member in z.namelist():
            if Path(member).name.lower()in['license','license.txt','readme.txt']:(notice/Path(member).name).write_bytes(z.read(member))
    binary=ROOT/'target/release/ssmp-desktop.exe'
    if binary.exists() and '--keep-binary' not in sys.argv:shutil.copy2(binary,app/'SSMP.exe')
    shutil.copy2(ROOT/'tools/provenance.json',app/'tools/provenance.json')
    for name in ['LICENSE','README.md','CHANGELOG.md','RELEASE-CHECKLIST.md','SPECIFICATION.md']:shutil.copy2(ROOT/name,app/name)
    for name in ['MUSIC-GUIDE.md','MUSIC-0.3-SPEC.md','UPDATES.md','HOSTING.md','IMPORTS.md','DOCKER.md','THIRD-PARTY.md']:
        if(ROOT/'docs'/name).exists():(app/'docs').mkdir(exist_ok=True);shutil.copy2(ROOT/'docs'/name,app/'docs'/name)
    (app/'licenses/mpv').mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/'LICENSE',app/'licenses/mpv/COPYING')
    registry=Path(os.environ.get('CARGO_HOME',str(Path.home()/'.cargo')))/'registry/src';packages=[]
    for package in tomllib.loads((ROOT/'Cargo.lock').read_text(encoding='utf-8'))['package']:
        if not package.get('source','').startswith('registry+'):continue
        folder=next(registry.glob(f"*/{package['name']}-{package['version']}"),None)
        if folder:
            metadata=tomllib.loads((folder/'Cargo.toml').read_text(encoding='utf-8'))['package'];files=[]
            for path in folder.rglob('*'):
                if path.is_file()and path.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE')):
                    dst=app/'licenses/rust'/folder.name/path.relative_to(folder);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,dst);files.append(path.relative_to(folder).as_posix())
            packages.append({'name':package['name'],'version':package['version'],'license':metadata.get('license'),'license_files':files})
    (app/'licenses/rust').mkdir(parents=True,exist_ok=True);(app/'licenses/rust/packages.json').write_text(json.dumps(packages,indent=2),encoding='utf-8')
    hashes={p.relative_to(app).as_posix():digest(p)for p in app.rglob('*')if p.is_file()and p.suffix in ['.exe','.dll']}
    (app/'tools/runtime-hashes.json').write_text(json.dumps(hashes,indent=2),encoding='utf-8');print('SSMP_PINNED_RUNTIME_READY')
if __name__=='__main__':main()
