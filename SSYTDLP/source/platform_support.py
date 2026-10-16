"""Host-specific executable discovery, user folders and desktop opening."""
from pathlib import Path
import os
import platform
import shutil
import subprocess
import sys


def executable_name(name, system=None):
    return name + '.exe' if (system or platform.system()) == 'Windows' else name


def tool_path(tools, name, required=True):
    candidate = Path(tools) / executable_name(name)
    if candidate.is_file():
        return candidate.resolve()
    # Always provision our own nightly; never silently pick a system stable engine.
    found = shutil.which(name) if name != 'yt-dlp' else None
    if not found and name != 'yt-dlp' and platform.system() != 'Windows':
        found = next((str(Path(prefix) / name) for prefix in ('/opt/homebrew/bin', '/usr/local/bin', '/usr/bin') if (Path(prefix) / name).is_file()), None)
    if found:
        return Path(found).resolve()
    if required:
        raise FileNotFoundError(f'Missing {name}. Install it or place it in the tools folder. See CROSS-PLATFORM.md.')
    return None


def tool_args(tools):
    # Keep argument construction usable before dependencies have been provisioned.
    ffmpeg = tool_path(tools, 'ffmpeg', required=False)
    args = ['--ffmpeg-location', str(ffmpeg if ffmpeg else tools)]
    for runtime in ('deno', 'node'):
        path = tool_path(tools, runtime, required=False)
        if path:
            args += ['--js-runtimes', f'{runtime}:{path}']
            break
    return args


def config_directory(system=None, environ=None, home=None):
    system = system or platform.system()
    environ = os.environ if environ is None else environ
    home = Path.home() if home is None else Path(home)
    if system == 'Windows':
        return Path(environ.get('LOCALAPPDATA', home / 'AppData/Local')) / 'SSYTDLP'
    if system == 'Darwin':
        return home / 'Library/Application Support/SSYTDLP'
    return Path(environ.get('XDG_CONFIG_HOME', home / '.config')) / 'SSYTDLP'


def open_folder(path):
    path = Path(path).expanduser().resolve()
    path.mkdir(parents=True, exist_ok=True)
    if sys.platform == 'win32':
        os.startfile(str(path))
    else:
        subprocess.Popen(['open' if sys.platform == 'darwin' else 'xdg-open', str(path)], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def nightly_asset(system=None, machine=None):
    system, machine = system or platform.system(), (machine or platform.machine()).lower()
    if system == 'Windows':
        return 'yt-dlp_arm64.exe' if machine in ('arm64', 'aarch64') else 'yt-dlp.exe'
    if system == 'Darwin':
        return 'yt-dlp_macos'
    if machine in ('aarch64', 'arm64'):
        return 'yt-dlp_linux_aarch64'
    if machine in ('x86_64', 'amd64'):
        return 'yt-dlp_linux'
    raise ValueError('No prebuilt nightly engine for this CPU; install yt-dlp nightly with pip.')
