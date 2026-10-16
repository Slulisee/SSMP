from pathlib import Path
import shutil

ROOT=Path(__file__).resolve().parents[1]
package=ROOT/'dist/SSMP-0.1.0-windows-x64'
(package/'tools/mpv').mkdir(parents=True,exist_ok=True)
shutil.copy2(ROOT/'tools/mpv/libmpv-2.dll',package/'tools/mpv/libmpv-2.dll')
shutil.copy2(ROOT/'SSYTDLP/tools/ffprobe.exe',package/'tools/ffprobe.exe')
runtime=Path(r'C:\Users\Anonym\Documents\Codex\2026-10-12\cod\work\toolchain\llvm-mingw-20261006-ucrt-x86_64\bin\libunwind.dll')
shutil.copy2(runtime,package/'libunwind.dll')
(ROOT/'tools/runtime-licenses').mkdir(parents=True,exist_ok=True)
shutil.copy2(runtime.parents[1]/'LICENSE.TXT',ROOT/'tools/runtime-licenses/libunwind.txt')
if (ROOT/'work/ssytdlp-folder/SSYTDLP/SSYTDLP.exe').stat().st_size<1000000:
    raise RuntimeError('SSYTDLP rebuild is incomplete')
shutil.copy2(ROOT/'work/ssytdlp-folder/SSYTDLP/SSYTDLP.exe',ROOT/'SSYTDLP/SSYTDLP.exe')
shutil.copytree(ROOT/'work/ssytdlp-folder/SSYTDLP/_internal',ROOT/'SSYTDLP/_internal',dirs_exist_ok=True)
print('Prepared SSMP playback runtime and updated workspace downloader executable.')
