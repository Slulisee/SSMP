"""Exercise the actual bundled yt-dlp/FFmpeg pipeline against generated local media."""
from pathlib import Path
import functools, http.server, json, os, sys, threading

ROOT=Path(__file__).resolve().parents[1]
os.environ['TEMP']=os.environ['TMP']=str(ROOT/'work/temp')
sys.path.insert(0,str(ROOT/'SSYTDLP/source'))
from engine import Download,Options

class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self,*args): pass

server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(Handler,directory=str(ROOT/'qa/media/Motion Studies')))
threading.Thread(target=server.serve_forever,daemon=True).start()
output=ROOT/'qa/downloader-output';output.mkdir(exist_ok=True)
inbox=ROOT/'qa/e2e-data/imports';inbox.mkdir(parents=True,exist_ok=True)
events=[]
url=f'http://127.0.0.1:{server.server_port}/Motion.Study.S01E01.mkv'
options=Options(url,str(output),preset='MP3 audio · 320 kbps',metadata=True,cover=False,organize=False,skip_existing=False,ssmp_enabled=True,ssmp_inbox=str(inbox))
try:
    Download(ROOT/'SSYTDLP/tools',lambda kind,value:events.append((kind,value))).run(options)
finally:
    server.shutdown();server.server_close()
done=[v for k,v in events if k=='done'][-1]
assert done['status']=='success',events[-20:]
files=[v for k,v in events if k=='file']
assert files and all(Path(p).is_file() for p in files)
manifests=list(inbox.glob('*.json'))
assert manifests
payload=json.loads(manifests[-1].read_text(encoding='utf-8'))
assert payload['items'][0]['path']==str(Path(files[0]).resolve())
proof={'status':done,'completed_files':files,'manifest':str(manifests[-1]),'payload':payload,'log':[v for k,v in events if k=='log'][-8:]}
(ROOT/'qa/downloader-e2e.json').write_text(json.dumps(proof,indent=2),encoding='utf-8')
print('SSYTDLP_E2E_OK: actual yt-dlp + FFmpeg completed a local media download and published its SSMP manifest.')
