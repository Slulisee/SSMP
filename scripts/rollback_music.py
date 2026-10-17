"""Restore an explicitly selected source/portable copy from a verified snapshot."""
from pathlib import Path
import argparse,hashlib,json,shutil
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--snapshot',type=Path,required=True);parser.add_argument('--target',type=Path,required=True);args=parser.parse_args()
    snapshot=args.snapshot.resolve(strict=True);target=args.target.resolve(strict=True)
    if snapshot==target or snapshot.is_relative_to(target) or target.is_relative_to(snapshot):raise SystemExit('Choose a separate target copy, outside the snapshot.')
    manifest=json.loads((snapshot/'snapshot.json').read_text(encoding='utf-8'))
    for name,expected in manifest.get('modified',{}).items():
        destination=(target/name).resolve()
        if not destination.is_relative_to(target):raise SystemExit('Invalid modified path')
        if destination.is_file()and digest(destination)not in {expected,manifest['files'].get(name)}:raise SystemExit('Target was edited; preserve it before rollback: '+name)
    for name,expected in manifest['files'].items():
        source=(snapshot/name).resolve(strict=True);destination=(target/name).resolve()
        if not source.is_relative_to(snapshot)or not destination.is_relative_to(target):raise SystemExit('Invalid snapshot path')
        if digest(source)!=expected:raise SystemExit('Snapshot checksum mismatch: '+name)
    for name,expected in manifest.get('added',{}).items():
        destination=(target/name).resolve()
        if not destination.is_relative_to(target):raise SystemExit('Invalid addition path')
        if destination.is_file()and digest(destination)!=expected:raise SystemExit('Target addition was edited; preserve it before rollback: '+name)
    for name in manifest.get('added',{}):
        destination=target/name
        if destination.is_file():destination.unlink()
    for name in manifest['files']:
        destination=target/name;destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(snapshot/name,destination)
    for name,expected in manifest['files'].items():assert digest(target/name)==expected,name
    print(f'MUSIC_ROLLBACK_OK: {len(manifest["files"])} baseline files restored byte-for-byte; added music-release files removed; library/media data preserved.')
if __name__=='__main__':main()
