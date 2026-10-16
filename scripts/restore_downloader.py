"""Standalone archive rollback. Run on a fresh disposable copy to verify."""
from pathlib import Path
import hashlib, shutil, sys
root=Path(__file__).resolve().parents[1]
target=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else root
baseline=root/'rollback/before-ssmp'
if not (target/'source/engine.py').is_file():raise SystemExit('Choose an extracted SSYTDLP folder.')
for p in (baseline/'source').rglob('*'):
    if not p.is_file() or '__pycache__' in p.parts:continue
    dst=target/'source'/p.relative_to(baseline/'source');dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dst)
for name in ['ssmp_bridge.py','test_ssmp.py','test_ssmp_endtoend.py']:(target/'source'/name).unlink(missing_ok=True)
for name in ['SSYTDLP.exe','LICENSE.txt']:
    shutil.copy2(baseline/name,target/name)
    assert hashlib.sha256((baseline/name).read_bytes()).digest()==hashlib.sha256((target/name).read_bytes()).digest()
print('ROLLBACK_OK: original downloader source and executable restored; SSMP integration removed; media and library data preserved.')
