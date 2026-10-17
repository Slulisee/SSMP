"""Native music UI checks with generated media, isolated profile, and explicit sizes/locales."""
from pathlib import Path
import json,os,subprocess,time,tomllib
ROOT=Path(__file__).resolve().parents[1]
VERSION=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
EXE=ROOT/f'dist/SSMP-{VERSION}-windows-x64/SSMP.exe'
OUT=ROOT/'qa/music-screenshots';OUT.mkdir(exist_ok=True)
DATA=ROOT/f'qa/music-ui-{int(time.time())}'
cases=[(page,'en','Ocean','1440x920') for page in ['home','music','artists','artist','albums','album','playlists','lyrics','stats','settings','server','video','edit']]
cases += [('playlists','de','Carbon','1000x720'),('settings','es','Frost','1000x720'),('lyrics','fr','Forest','1000x720'),('stats','de','Nord','1000x720')]
records=[]
try:
    for page,lang,theme,size in cases:
        name=f'{page}-{lang}-{theme.lower()}-{size}'
        env=dict(os.environ,SSMP_SMOKE_LANGUAGE=lang,SSMP_SMOKE_THEME=theme,SSMP_SMOKE_SIZE=size,SSMP_SCREENSHOT=str(OUT/(name+'.png')))
        command=[str(EXE),'--data',str(DATA),'--smoke-path',str(ROOT/'qa/media'),'--smoke-page',page]
        result=subprocess.run(command,env=env,cwd=ROOT,capture_output=True,text=True,encoding='utf-8',timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
        assert result.returncode==0,(name,result.stderr)
        proof=json.loads((OUT/(name+'.json')).read_text())
        assert proof['engine_available'] and not proof['engine_error'] and not proof['player_error'],proof
        assert proof['position']>0,proof
        if page=='video':assert proof['rendered_frames']>0,proof
        records.append({'case':name,'command':subprocess.list2cmdline(command),'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr,'proof':proof})
        print('MUSIC_NATIVE_UI_OK='+name,flush=True)
finally:
    (ROOT/'qa/music-ui-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
