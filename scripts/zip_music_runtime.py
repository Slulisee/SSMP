"""Archive a prepared portable runtime without private libraries or credentials."""
from pathlib import Path
import hashlib,tomllib,zipfile
ROOT=Path(__file__).resolve().parents[1]
version=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
app=ROOT/f'dist/SSMP-{version}-windows-x64';target=app.parent/(app.name+'.zip')
with zipfile.ZipFile(target,'w',zipfile.ZIP_DEFLATED,compresslevel=5,strict_timestamps=False)as z:
    for p in app.rglob('*'):
        if not p.is_file()or'__pycache__'in p.parts:continue
        assert p.suffix.lower()not in ['.sqlite3','.sqlite3-wal','.sqlite3-shm'],p
        z.write(p,app.name+'/'+p.relative_to(app).as_posix())
with zipfile.ZipFile(target)as z:assert z.testzip()is None
with target.open('rb')as f:digest=hashlib.file_digest(f,'sha256').hexdigest()
target.with_suffix('.zip.sha256').write_text(digest+'  '+target.name+'\n',encoding='ascii')
print('SSMP_PORTABLE_CRC_OK='+target.name)
