"""Prepare isolated inputs and pinned playback runtime; never change the supplied app."""
from pathlib import Path
import hashlib, json, shutil, subprocess, urllib.request

ROOT = Path(__file__).resolve().parents[1]
ORIGINAL = Path(r'C:\Users\Anonym\Downloads\SSYTDLP\SSYTDLP-Portable\SSYTDLP')
WORK = ROOT / 'work'
WORK.mkdir(exist_ok=True)
(ROOT / 'tools').mkdir(exist_ok=True)
if not (ROOT / 'SSYTDLP').exists():
    shutil.copytree(ORIGINAL, ROOT / 'SSYTDLP')
baseline = WORK / 'baseline' / 'SSYTDLP'
if not baseline.exists():
    shutil.copytree(ORIGINAL / 'source', baseline / 'source')
    shutil.copy2(ORIGINAL / 'LICENSE.txt', baseline / 'LICENSE.txt')
    hashes = {str(p.relative_to(ORIGINAL)): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in ORIGINAL.rglob('*') if p.is_file()}
    (WORK / 'original-hashes.json').write_text(json.dumps(hashes, indent=2), encoding='utf-8')
print('Original SSYTDLP preserved; workspace copy ready.', flush=True)

def get(url):
    return urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent':'SSMP-build/0.1'}), timeout=90)

release = json.load(get('https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/tags/20261016'))
asset = next(a for a in release['assets'] if a['name'] == 'mpv-dev-x86_64-20261016-git-a2cd83c56d.7z')
archive = WORK / asset['name']
if not archive.exists():
    with get(asset['browser_download_url']) as response, archive.open('wb') as output:
        shutil.copyfileobj(response, output)
digest = hashlib.sha256(archive.read_bytes()).hexdigest()
expected = (asset.get('digest') or '').removeprefix('sha256:')
if expected and expected != digest:
    raise RuntimeError('mpv archive checksum mismatch')
runtime = ROOT / 'tools' / 'mpv'
runtime.mkdir(exist_ok=True)
subprocess.run(['7z', 'x', '-y', str(archive), '-o' + str(runtime)], check=True, stdout=subprocess.DEVNULL)
provenance = {'release':release['tag_name'], 'url':asset['browser_download_url'],
              'sha256':digest, 'upstream_sha256':expected,
              'mpv_source':'https://github.com/mpv-player/mpv/tree/a2cd83c56d',
              'build_source':'https://github.com/shinchiro/mpv-winbuild-cmake/tree/20261016'}
(ROOT / 'tools' / 'provenance.json').write_text(json.dumps(provenance, indent=2), encoding='utf-8')
for name, url in {'LICENSE':'https://www.gnu.org/licenses/gpl-3.0.txt',
                  'work/mpv-render.h':'https://raw.githubusercontent.com/mpv-player/mpv/a2cd83c56d/include/mpv/render.h',
                  'work/mpv-client.h':'https://raw.githubusercontent.com/mpv-player/mpv/a2cd83c56d/include/mpv/client.h'}.items():
    path=ROOT/name
    if not path.exists():
        path.write_bytes(get(url).read())
print('mpv pinned and verified:', digest, flush=True)
