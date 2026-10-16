from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[1]
original=Path(r'C:\Users\Anonym\Downloads\SSYTDLP\SSYTDLP-Portable\SSYTDLP')
hashes=json.loads((ROOT/'work/original-hashes.json').read_text())
changed=[name for name,digest in hashes.items() if not (original/name).is_file() or hashlib.sha256((original/name).read_bytes()).hexdigest()!=digest]
if changed:
    raise SystemExit('Original files changed: '+', '.join(changed))
print(f'ORIGINALS_UNCHANGED={len(hashes)} files verified with SHA-256')
