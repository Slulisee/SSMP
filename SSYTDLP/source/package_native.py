from pathlib import Path
import platform
import tarfile
import zipfile

root = Path(__file__).resolve().parent.parent
dist = root / 'dist'
name = 'SSYTDLP-' + platform.system() + '-' + platform.machine()
output = root / 'native-artifacts'
output.mkdir(exist_ok=True)
if platform.system() == 'Windows':
    target = output / (name + '.zip')
    with zipfile.ZipFile(target, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in dist.rglob('*'):
            if path.is_file():
                archive.write(path, path.relative_to(dist))
else:
    target = output / (name + '.tar.gz')
    with tarfile.open(target, 'w:gz') as archive:
        for path in dist.iterdir():
            if platform.system() == 'Darwin' and path.name == 'SSYTDLP':
                continue
            archive.add(path, arcname=path.name)
print(target)
