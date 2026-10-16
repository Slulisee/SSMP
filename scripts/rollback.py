"""Restore only the workspace downloader integration; never touch user media/data."""
from pathlib import Path
import hashlib, shutil, sys

ROOT=Path(__file__).resolve().parents[1]
target=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else ROOT/'SSYTDLP'
if not target.is_relative_to(ROOT):
    raise SystemExit('Rollback target must be inside the SSMP workspace.')
baseline=ROOT/'work/baseline/SSYTDLP'
if not (baseline/'source/engine.py').is_file():
    raise SystemExit('Original snapshot is missing.')
target.mkdir(parents=True,exist_ok=True)
for file in (baseline/'source').rglob('*'):
    if not file.is_file() or '__pycache__' in file.parts:continue
    out=target/'source'/file.relative_to(baseline/'source')
    out.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(file,out)
for name in ['ssmp_bridge.py','test_ssmp.py','test_ssmp_endtoend.py']:
    (target/'source'/name).unlink(missing_ok=True)
for name in ['SSYTDLP.exe','LICENSE.txt']:
    if (baseline/name).exists():
        shutil.copy2(baseline/name,target/name)
for file in (baseline/'source').rglob('*'):
    if not file.is_file() or '__pycache__' in file.parts:continue
    assert hashlib.sha256(file.read_bytes()).digest()==hashlib.sha256((target/'source'/file.relative_to(baseline/'source')).read_bytes()).digest()
print('ROLLBACK_OK: original downloader source and executable restored; SSMP integration removed; media and library data preserved.')
