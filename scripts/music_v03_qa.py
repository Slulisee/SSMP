"""Native UI regression checks for the 0.3 feedback release."""
from pathlib import Path
import json,os,subprocess,time,sys
ROOT=Path(__file__).resolve().parents[1]
EXE=ROOT/'dist/SSMP-0.3.0-windows-x64/SSMP.exe'
PREFIX=os.environ.get('SSMP_QA_PREFIX','v03')
OUT=ROOT/f'qa/{PREFIX}-screenshots';OUT.mkdir(exist_ok=True)
DATA=ROOT/f'qa/v03-ui-{time.time_ns()}'
cases=[(p,'en','Ocean','1440x920')for p in ['music','playlists','lyrics','lyrics-queue','edit','bulk','stats','stats-month','stats-year','settings','video']]
cases += [('stats-month','de','Nord','1000x720'),('bulk','es','Frost','1000x720'),('edit','fr','Carbon','1000x720'),('lyrics-queue','de','Forest','1000x720')]
cases=[(*case,1.0)for case in cases]
cases += [('music','de','Ocean','1000x720',1.5),('lyrics-queue','de','Forest','1000x720',1.5),('edit','fr','Carbon','1000x720',1.5)]
if sys.argv[1:]:cases=[case for case in cases if case[0]in sys.argv[1:]]
records=[]
try:
    for page,lang,theme,size,scale in cases:
        name=f'{page}-{lang}-{size}'+(f'-scale{scale}'if scale!=1.0 else'');shot=OUT/(name+'.png')
        env=dict(os.environ,SSMP_SMOKE_LANGUAGE=lang,SSMP_SMOKE_THEME=theme,SSMP_SMOKE_SIZE=size,SSMP_SMOKE_SCALE=str(scale),SSMP_SCREENSHOT=str(shot))
        command=[str(EXE),'--data',str(DATA),'--smoke-path',str(ROOT/'qa/media'),'--smoke-page',page]
        result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True,encoding='utf-8',timeout=35,creationflags=subprocess.CREATE_NO_WINDOW)
        assert result.returncode==0,(name,result.stderr)
        proof=json.loads(shot.with_suffix('.json').read_text(encoding='utf-8'))
        assert proof['engine_available'] and not proof['engine_error'] and not proof['player_error'] and proof['position']>0,proof
        if page=='video':assert proof['rendered_frames']>0
        records.append({'case':name,'command':subprocess.list2cmdline(command),'exit':result.returncode,'proof':proof})
        print('V03_NATIVE_UI_OK='+name,flush=True)
finally:(ROOT/f'qa/{PREFIX}-ui-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
