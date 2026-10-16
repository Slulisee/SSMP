"""Provision checksum-verified yt-dlp nightly and Deno for this host."""
from pathlib import Path
import hashlib
import io
import json
import os
import platform
import re
import zipfile

from engine import update_nightly
from platform_support import executable_name
from services import get_data
from urllib.request import Request, urlopen


def fetch(url):
    with urlopen(Request(url, headers={'User-Agent': 'SSYTDLP/3.0 setup'}), timeout=90) as response:
        return response.read()


def prepare(root=None):
    root = Path(root) if root else Path(__file__).resolve().parent.parent
    tools = root / 'tools'
    tools.mkdir(parents=True, exist_ok=True)
    events = []
    update_nightly(tools, lambda *event: events.append(event))
    if events[-1][0] != 'updated':
        raise RuntimeError(str(events[-1][1]))
    if not (tools / executable_name('deno')).exists():
        arch = 'aarch64' if platform.machine().lower() in ('arm64', 'aarch64') else 'x86_64'
        target = 'pc-windows-msvc' if os.name == 'nt' else 'apple-darwin' if platform.system() == 'Darwin' else 'unknown-linux-gnu'
        release = get_data('https://api.github.com/repos/denoland/deno/releases/latest')
        tag = release['tag_name']
        if not re.fullmatch(r'v\d+\.\d+\.\d+', tag):
            raise ValueError('Unexpected Deno release tag.')
        url = f'https://github.com/denoland/deno/releases/download/{tag}/deno-{arch}-{target}.zip'
        checksum = re.search(r'\b[0-9a-fA-F]{64}\b', fetch(url + '.sha256sum').decode())
        data = fetch(url)
        if not checksum or hashlib.sha256(data).hexdigest() != checksum[0].lower():
            raise ValueError('Deno checksum mismatch.')
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            name = executable_name('deno')
            output = tools / name
            output.write_bytes(archive.read(name))
            output.chmod(0o755)
        manifest_path = tools / 'engines.json'
        manifest = json.loads(manifest_path.read_text('utf-8'))
        manifest['deno'] = {'version': tag.lstrip('v'), 'sha256': hashlib.sha256(output.read_bytes()).hexdigest(), 'source': url}
        manifest_path.write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print('Nightly and Deno ready. FFmpeg and ffprobe must be installed separately on macOS/Linux.')


if __name__ == '__main__':
    prepare()
