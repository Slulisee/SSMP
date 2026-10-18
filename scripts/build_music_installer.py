"""Build a per-user Windows setup from the prepared SSMP runtime folder."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,tomllib
ROOT=Path(__file__).resolve().parents[1]
def compiler():
    for value in [os.environ.get('NSIS_MAKENSIS'),shutil.which('makensis'),str(Path(os.environ.get('ProgramFiles(x86)','C:/Program Files (x86)'))/'NSIS/makensis.exe'),str(Path(os.environ.get('LOCALAPPDATA',''))/'tauri/NSIS/makensis.exe')]:
        if value and Path(value).is_file():return str(Path(value).resolve())
    raise FileNotFoundError('Install NSIS 3.x or set NSIS_MAKENSIS.')
def quote(value):return str(value).replace('$','$$').replace('"','$\\"')
def main():
    version=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
    app=ROOT/f'dist/SSMP-{version}-windows-x64';assert(app/'SSMP.exe').is_file()
    hashes={p.relative_to(app).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()for p in app.rglob('*')if p.is_file()and p.suffix in ['.exe','.dll']and not set(p.relative_to(app).parts)&{'qa','rollback'}}
    (app/'tools/runtime-hashes.json').write_text(json.dumps(hashes,indent=2),encoding='utf-8')
    build=ROOT/'work/music-installer';build.mkdir(exist_ok=True)
    files=sorted(p for p in app.rglob('*') if p.is_file()and not set(p.relative_to(app).parts)&{'qa','rollback','scripts','__pycache__'} and not p.name.startswith(('ROLLBACK','MUSIC-VERIFICATION')))
    payload=[];current=None
    for p in files:
        parent=p.relative_to(app).parent
        if parent!=current:payload.append(f'SetOutPath "$INSTDIR\\{quote(parent)}"');current=parent
        payload.append(f'File "{quote(p)}"')
    (build/'payload.nsh').write_text('\n'.join(payload)+'\n',encoding='utf-8')
    uninstall=[f'Delete "$INSTDIR\\{quote(p.relative_to(app))}"' for p in files]
    directories={parent for p in files for parent in p.relative_to(app).parents if parent!=Path('.')}
    uninstall.extend(f'RMDir "$INSTDIR\\{quote(p)}"'for p in sorted(directories,key=lambda p:len(p.parts),reverse=True))
    (build/'uninstall.nsh').write_text('\n'.join(uninstall)+'\n',encoding='utf-8')
    target=ROOT/'dist/SSMP-Windows-AMD64-Setup.exe'
    subprocess.run([compiler(),'/V2','/DROOT='+str(ROOT),'/DVERSION='+version,'/DOUTPUT='+str(target),'/DPAYLOAD='+str(build/'payload.nsh'),'/DUNINSTALL_FILES='+str(build/'uninstall.nsh'),str(ROOT/'scripts/installer.nsi')],check=True)
    digest=hashlib.sha256(target.read_bytes()).hexdigest()
    target.with_suffix('.exe.sha256').write_text(digest+'  '+target.name+'\n',encoding='ascii')
    (build/'manifest.json').write_text(json.dumps({p.relative_to(app).as_posix():hashlib.sha256(p.read_bytes()).hexdigest()for p in files},indent=2),encoding='utf-8')
    print(f'SSMP_INSTALLER_BUILT={target.name} bytes={target.stat().st_size} SHA256={digest}')
if __name__=='__main__':main()
