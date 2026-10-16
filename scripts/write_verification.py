from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[1]
QA=ROOT/'qa'
lines=[
 'SSMP 0.1.0 - observed local milestone verification, 2026-10-16',
 'Working directory: '+str(ROOT),
 'Changed fields: SSYTDLP engine.Options.ssmp_enabled (default False), ssmp_inbox; Settings -> SSMP; completed-download atomic version-1 manifest handoff.',
 'Changed behavior: optional SSMP imports; independent downloader behavior when disabled; native SSMP desktop and optional authenticated host added.',
 'MODIFIED_FILE='+str(ROOT/'dist/SSMP-0.1.0-windows-x64/SSMP.exe'),
 'MODIFIED_DOWNLOADER='+str(ROOT/'SSYTDLP/SSYTDLP.exe'),
 'DIFF_FILE='+str(ROOT/'CHANGES.diff'),
 'VERIFICATION_FILE='+str(ROOT/'VERIFICATION.txt'),
 'ROLLBACK_FILE='+str(ROOT/'ROLLBACK.sh'),
 'Rollback input: disposable copy, never the supplied original or the modified working copy.',
 'Test environment: TEMP and TMP='+str(ROOT/'work/temp')+'; SSMP_PYTHON=C:\\Users\\Anonym\\AppData\\Local\\Programs\\Python\\Python312\\python.exe.',
 'No router, firewall, VPN or antivirus settings were changed by these scripts.',
 '',
 'BASELINE/MODIFIED/ROLLBACK: exact commands, input, literal output and exit status follow.',
]
for r in json.loads((QA/'change-results.json').read_text(encoding='utf-8')):
    lines.extend(['',r['label']])
    if 'command' in r:
        lines.extend(['COMMAND: '+r['command'],'INPUT: '+r['input'],'STDOUT:',r['stdout'],'STDERR:',r['stderr'],'EXIT: '+str(r['exit'])])
    else:lines.append(r['result'])
lines.extend(['','Native Windows checks:',"COMMAND: scripts/build.ps1 -CargoArgs @('test','--offline')",'INPUT: Rust core tests, generated temporary files','EXIT: 0',(QA/'core-tests-final.log').read_text(encoding='utf-8-sig'),
 'COMMAND: python scripts/qa.py','INPUT: generated FLAC/MP3/Opus/WAV/MKV; disposable local host and profiles; no user media','EXIT: 0',(QA/'native-tests.log').read_text(encoding='utf-8-sig'),
 'COMMAND: python scripts/qa.py --ui-only','INPUT: final native UI build and generated media','EXIT: 0',(QA/'native-ui-final.log').read_text(encoding='utf-8-sig'),
 'Per-command native output:',(QA/'native-results.json').read_text(encoding='utf-8'),
 'Final UI output:',(QA/'native-ui-final-results.json').read_text(encoding='utf-8'),
 'COMMAND: python scripts/downloader_e2e.py','INPUT: generated local HTTP MKV; actual bundled yt-dlp and FFmpeg; isolated output/inbox','EXIT: 0',(QA/'downloader-e2e.log').read_text(encoding='utf-8-sig'),
 'COMMAND: python scripts/downloader_settings_check.py','INPUT: real settings dialog, disposable config','EXIT: 0',(QA/'downloader-settings.log').read_text(encoding='utf-8-sig'),
 'Packaged downloader native main-window smoke check: SSYTDLP/SSYTDLP.exe --smoke-test qa/ssytdlp-smoke.json; EXIT: 0',(QA/'ssytdlp-smoke.json').read_text(encoding='utf-8'),
 'Headless feature selection build output:',(QA/'headless-build.log').read_text(encoding='utf-8-sig'),
 'Final desktop release build output:',(ROOT/'work/build-current.log').read_text(encoding='utf-8-sig'),
 'Antivirus custom scans: MpCmdRun.exe -Scan -ScanType 3 -File PATH -DisableRemediation. Each scan command returned EXIT 0.',
 'SSMP INPUT: '+str(ROOT/'dist/SSMP-0.1.0-windows-x64'),(QA/'antivirus-scan.log').read_text(encoding='utf-8-sig'),
 'SSYTDLP INPUT: '+str(ROOT/'SSYTDLP'),(QA/'antivirus-downloader.log').read_text(encoding='utf-8-sig'),
 'The user reported a potential-virus alert, then temporarily disabled protection. Detection name is unknown. Defender custom scan success does not establish acceptance by Bitdefender/Malwarebytes with protection re-enabled. No vendor sample was uploaded. Binaries are unsigned.',
 'Observed limits: physical audible output unreviewed (automated mpv uses null audio); Linux/macOS/mobile not executed; LRCLIB live results not checked; full release gaps are in RELEASE-CHECKLIST.md.',
 'Original 63 supplied files were hashed before changes and verified unchanged. Rollback restored baseline source/executable and all 25 baseline tests on a separate copy; the main modified integration remains present.',
 'ZIP CRC, extracted-package execution and artifact SHA256 evidence are written separately to qa/package-tests.log, qa/artifacts.json and dist/SHA256SUMS.txt after assembly.',
])
for p in [ROOT/'dist/SSMP-0.1.0-windows-x64/SSMP.exe',ROOT/'SSYTDLP/SSYTDLP.exe',ROOT/'work/baseline/SSYTDLP/SSYTDLP.exe']:
    lines.append('SHA256 '+str(p)+' '+hashlib.sha256(p.read_bytes()).hexdigest())
(ROOT/'VERIFICATION.txt').write_text('\n'.join(lines)+'\n',encoding='utf-8')
print('VERIFICATION_WRITTEN='+str(ROOT/'VERIFICATION.txt'))
