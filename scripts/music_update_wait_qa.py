"""Verify real setup waits for application shutdown before overwriting files."""
from pathlib import Path
import ctypes,hashlib,json,subprocess,time
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/f'qa/v03-update-wait-{time.time_ns()}';OUT.mkdir()
target=OUT/'App with spaces';target.mkdir();marker=target/'README.md';marker.write_text('Old app file fixture',encoding='utf-8')
sentinel=target/'user-song.txt';sentinel.write_text('Keep this user data',encoding='utf-8');expected=hashlib.sha256(sentinel.read_bytes()).hexdigest()
kernel=ctypes.WinDLL('kernel32',use_last_error=True);kernel.CreateMutexW.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_wchar_p];kernel.CreateMutexW.restype=ctypes.c_void_p;kernel.CloseHandle.argtypes=[ctypes.c_void_p]
handle=kernel.CreateMutexW(None,False,'Local\\Slulisee.SSMP.Application');assert handle
setup=ROOT/'dist/SSMP-Windows-AMD64-Setup.exe';command=subprocess.list2cmdline([str(setup)])+' /S /NOINTEGRATION /D='+str(target)
records=[]
try:
    result=subprocess.run(command,executable=str(setup),capture_output=True,timeout=25)
    assert result.returncode==2 and marker.read_text(encoding='utf-8')=='Old app file fixture'
    records.append({'command':command,'exit':result.returncode,'result':'Running-app guard leaves app files unchanged'})
    automatic=subprocess.Popen(command.replace(' /S ',' /S /AUTO '),executable=str(setup),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    time.sleep(2)
    assert automatic.poll()is None and marker.read_text(encoding='utf-8')=='Old app file fixture'
    kernel.CloseHandle(handle);handle=None
    code=automatic.wait(timeout=120);assert code==0
    assert(target/'SSMP.exe').is_file()and marker.read_text(encoding='utf-8')!='Old app file fixture'
    assert hashlib.sha256(sentinel.read_bytes()).hexdigest()==expected
    records.append({'command':command.replace(' /S ',' /S /AUTO '),'exit':code,'result':'Waited while app mutex existed, installed after shutdown, preserved user file'})
    command=subprocess.list2cmdline([str(target/'Uninstall.exe')])+' /S _?='+str(target)
    result=subprocess.run(command,executable=str(target/'Uninstall.exe'),timeout=90);assert result.returncode==0
    assert sentinel.is_file()and not(target/'SSMP.exe').exists()
    records.append({'command':command,'exit':result.returncode,'result':'Explicit-list uninstall preserved generated user file'})
finally:
    if handle:kernel.CloseHandle(handle)
    if 'automatic'in globals() and automatic.poll()is None:automatic.terminate();automatic.wait(timeout=10)
    (ROOT/'qa/v03-update-wait-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print('SSMP_AUTOMATIC_INSTALL_WAIT_OK: app files untouched while running; installation succeeds after shutdown; user data preserved.')
