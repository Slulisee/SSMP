"""Black-box tests of the packaged native app; only generated media and disposable data."""
from pathlib import Path
import json, os, socket, sqlite3, subprocess, sys, time, urllib.request, urllib.error, shutil, tomllib

ROOT=Path(__file__).resolve().parents[1]
QA=ROOT/'qa'
RUN=QA/('run-'+str(int(time.time())))
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
EXE=Path(os.environ.get('SSMP_QA_EXE',str(ROOT/f'dist/SSMP-{VERSION}-windows-x64/SSMP.exe')))
ENV=os.environ.copy();ENV.update(TEMP=str(ROOT/'work/temp'),TMP=str(ROOT/'work/temp'),SSMP_DATA_DIR=str(QA/'diagnostics'))
ENV['SSMP_QA_PASSWORD']='generated-test-passphrase-2026'
records=[]
def run(*args,env=None,timeout=40):
    command=[str(EXE),*map(str,args)]
    r=subprocess.run(command,cwd=ROOT,env=env or ENV,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=timeout,creationflags=subprocess.CREATE_NO_WINDOW)
    records.append({'command':subprocess.list2cmdline(command),'stdout':r.stdout,'stderr':r.stderr,'exit':r.returncode})
    assert r.returncode==0,records[-1]
    return r.stdout

def http(url,method='GET',data=None,token=None,headers=None):
    h=dict(headers or {})
    if token:h['Authorization']='Bearer '+token
    if data is not None:h['Content-Type']='application/json'
    req=urllib.request.Request(url,json.dumps(data).encode() if data is not None else None,h,method=method)
    try:r=urllib.request.urlopen(req,timeout=12)
    except urllib.error.HTTPError as e:r=e
    with r:return r.status,dict(r.headers),r.read()

def main():
    assert EXE.is_file()
    core=RUN/'core-data'
    assert f'SSMP {VERSION}' in run('--data',core,'--version')
    scanned=json.loads(run('--data',core,'--scan',QA/'media'))
    assert not scanned['errors'],scanned
    run('--data',core,'--import',QA/'season.json')
    with sqlite3.connect(core/'library.sqlite3') as db:
        media=[json.loads(row[0]) for row in db.execute('select data from media')]
    assert len(media)==5,len(media)
    assert any(m['title']=='Stillwater' and m['artist']=='SSMP Studio' for m in media)
    assert any(m['series']=='Motion Studies' and m['episode']==1 for m in media)
    for m in media:
        out=run('--data',QA/'playback-data','--self-test',m['path'])
        assert 'PLAYBACK_ADVANCED=true' in out,out
        if m['kind']=='video':assert 'NONZERO_PIXELS=true' in out,out
    run('--data',core,'--add-user','owner','--password-env','SSMP_QA_PASSWORD','--admin')
    run('--data',core,'--add-user','listener','--password-env','SSMP_QA_PASSWORD')
    sock=socket.socket();sock.bind(('127.0.0.1',0));port=sock.getsockname()[1];sock.close()
    url=f'http://127.0.0.1:{port}'
    log=(QA/'host.log').open('w',encoding='utf-8')
    watch=RUN/'watch';watch.mkdir(parents=True,exist_ok=True)
    host=subprocess.Popen([str(EXE),'--data',str(core),'--watch',str(watch),'--serve','--bind',f'127.0.0.1:{port}'],cwd=ROOT,env=ENV,stdout=log,stderr=subprocess.STDOUT,creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        deadline=time.time()+12
        while True:
            try:
                if http(url+'/api/v1/health')[0]==200:break
            except OSError:pass
            assert time.time()<deadline,'Host failed to start'
            time.sleep(.15)
        assert http(url+'/api/v1/library')[0]==401
        tokens={}
        for user in ['owner','listener']:
            status,_,body=http(url+'/api/v1/login','POST',{'username':user,'password':ENV['SSMP_QA_PASSWORD']})
            assert status==200,(status,body)
            tokens[user]=json.loads(body)['token']
        owner=tokens['owner'];listener=tokens['listener']
        status,_,body=http(url+'/api/v1/library',token=owner);assert status==200
        catalog=json.loads(body)
        assert len(catalog)==5
        assert all(not Path(m['path']).is_absolute() and not m['artwork'] for m in catalog)
        chosen=next(m for m in catalog if m['kind']=='audio')
        source=next(m for m in media if m['id']==chosen['id'])
        status,headers,body=http(url+'/api/v1/media/'+chosen['id'],token=owner,headers={'Range':'bytes=4-17'})
        assert status==206 and body==Path(source['path']).read_bytes()[4:18]
        assert headers['Content-Range']==f"bytes 4-17/{chosen['size']}"
        status,headers,body=http(url+'/api/v1/media/'+chosen['id'],'HEAD',token=owner)
        assert status==200 and not body and int(headers['Content-Length'])==chosen['size']
        assert http(url+'/api/v1/media/'+chosen['id'],token=owner,headers={'Range':'bytes=999999999999-'})[0]==416
        status,_,body=http(url+'/api/v1/state',token=owner);state=json.loads(body);state['favorites']=[chosen['id']]
        assert http(url+'/api/v1/state','PUT',state,owner)[0]==200
        assert http(url+'/api/v1/state','PUT',state,owner)[0]==409
        assert json.loads(http(url+'/api/v1/state',token=listener)[2])['favorites']==[]
        edit=dict(chosen,title='Edited by owner',path='C:/not-a-real-server-file',id='replacement-id')
        assert http(url+'/api/v1/metadata/'+chosen['id'],'PUT',edit,listener)[0]==403
        assert http(url+'/api/v1/metadata/'+chosen['id'],'PUT',edit,owner)[0]==200
        with sqlite3.connect(core/'library.sqlite3') as db:stored=json.loads(db.execute('select data from media where id=?',(chosen['id'],)).fetchone()[0])
        assert stored['path']==source['path'] and stored['id']==source['id']
        env=ENV.copy();env['SSMP_QA_TOKEN']=owner
        out=run('--data',QA/'stream-test','--self-test-url',url+'/api/v1/media/'+chosen['id'],'--token-env','SSMP_QA_TOKEN',env=env)
        assert 'PLAYBACK_ADVANCED=true' in out
        out=run('--data',RUN/'offline-client','--download',url,'--username','owner','--password-env','SSMP_QA_PASSWORD','--media-id',chosen['id'])
        offline=Path(out.strip().split('OFFLINE_SAVED=',1)[1]);assert offline.is_file()
        watched=watch/'Watched.wav'
        import wave, struct
        with wave.open(str(watched),'wb')as file:file.setparams((1,2,8000,0,'NONE','not compressed'));file.writeframes(struct.pack('<h',0)*8000)
        deadline=time.time()+12
        while len(json.loads(http(url+'/api/v1/library',token=owner)[2]))<6:
            assert time.time()<deadline,'Host watch folder was not scanned'
            time.sleep(.3)
        records.append({'check':'headless host folder watching','result':'PASS'})
        before_inbox=len(json.loads(http(url+'/api/v1/library',token=owner)[2]))
        # A clean runner has no earlier downloader QA output. Exercise the same
        # completed-file inbox protocol with a fresh media path outside watch.
        handoff=RUN/'handoff';handoff.mkdir()
        incoming=handoff/'Inbox.wav'
        with wave.open(str(incoming),'wb')as file:
            file.setparams((1,2,11025,0,'NONE','not compressed'))
            file.writeframes(struct.pack('<h',37)*11025)
        manifest=core/'imports/generated-handoff.tmp'
        manifest.write_text(json.dumps({'version':1,'producer':'SSMD','items':[{'path':str(incoming.resolve()),'title':'Generated inbox handoff','artist':'SSMP Studio'}]}),encoding='utf-8')
        manifest.replace(manifest.with_suffix('.json'))
        deadline=time.time()+8
        while len(json.loads(http(url+'/api/v1/library',token=owner)[2]))<before_inbox+1:
            assert time.time()<deadline,'Host inbox handoff was not consumed'
            time.sleep(.3)
        assert http(url+'/api/v1/logout','POST',{},owner)[0]==200
        assert http(url+'/api/v1/library',token=owner)[0]==401
        records.append({'check':'authenticated host, isolation, stale revisions, owner metadata boundaries, byte ranges, HEAD, revocation, remote playback, inbox watching','result':'PASS'})
    finally:
        host.terminate();host.wait(timeout=8);log.close()
    assert 'PLAYBACK_ADVANCED=true' in run('--data',RUN/'offline-client','--self-test',offline)
    records.append({'check':'offline playback with host stopped','result':'PASS'})
    ui_checks()
    print('SSMP_QA_OK: 5 media playback cases; authenticated host, ranges, profiles, revision conflicts, owner boundaries, remote playback, inbox handoff, offline playback; 5 native UI captures.')

def ui_checks():
    shots=QA/'screenshots';shots.mkdir(exist_ok=True)
    for page in ['home','video','settings','server','playlists']:
        env=ENV.copy();env['SSMP_SCREENSHOT']=str(shots/(page+'.png'))
        run('--data',RUN/('gui-'+page),'--smoke-path',QA/'media','--smoke-page',page,env=env,timeout=30)
        proof=json.loads((shots/(page+'.json')).read_text())
        assert proof['engine_available'] and not proof['engine_error'],proof
        assert not proof['player_error'],proof
        assert proof['position']>0,proof
        assert os.path.samefile(proof['actual_path'],proof['expected_path']),proof
        if page=='video':
            assert proof['rendered_frames']>0,proof
            assert proof['secondary_subtitle']=='2',proof
        records.append({'check':'native UI '+page,'result':'PASS','proof':proof})
    print('NATIVE_UI_OK: 5 pages, correct requested media, advancing playback, rendered video, dual subtitles.')

try:
    if '--ui-only' in sys.argv:ui_checks()
    else:main()
finally:
    name='native-ui-final-results.json' if '--ui-only' in sys.argv else 'native-results.json'
    (QA/name).write_text(json.dumps(records,indent=2),encoding='utf-8')
