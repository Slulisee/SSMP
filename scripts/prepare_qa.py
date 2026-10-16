from pathlib import Path
import json, subprocess
from PIL import Image, ImageDraw, ImageFont

ROOT=Path(__file__).resolve().parents[1]
media=ROOT/'qa/media'
media.mkdir(parents=True,exist_ok=True)
ffmpeg=ROOT/'SSYTDLP/tools/ffmpeg.exe'
tracks=[('Stillwater','First Light',(59,89,103),'flac'),('Slow Mornings','Sunday, Slowed Down',(107,70,84),'mp3'),('Out of Orbit','Night Drive',(70,62,119),'opus'),('A Place to Begin','First Light',(59,89,103),'wav')]
for n,(title,album,color,ext) in enumerate(tracks):
    folder=media/album;folder.mkdir(exist_ok=True)
    target=folder/(title+'.'+ext)
    if not target.exists():
        subprocess.run([str(ffmpeg),'-hide_banner','-loglevel','error','-y','-f','lavfi','-i',f'sine=frequency={220+n*55}:duration=8','-af','volume=0.05','-metadata',f'title={title}','-metadata','artist=SSMP Studio','-metadata',f'album={album}',str(target)],check=True)
    image=Image.new('RGB',(600,600),color);draw=ImageDraw.Draw(image)
    for i in range(600):
        c=tuple(int(v*(0.65+0.55*i/600)) for v in color)
        draw.line((0,i,600,i),fill=c)
    for radius in range(70,240,28):draw.ellipse((300-radius,270-radius,300+radius,270+radius),outline=tuple(min(255,v+50) for v in color),width=2)
    font=ImageFont.truetype('C:/Windows/Fonts/segoeui.ttf',42);small=ImageFont.truetype('C:/Windows/Fonts/segoeui.ttf',17)
    draw.text((40,470),album,fill='white',font=font);draw.text((43,535),'SSMP STUDIO / GENERATED TEST MEDIA',fill=(200,200,210),font=small)
    image.save(folder/'cover.jpg',quality=92)
    target.with_suffix('.lrc').write_text('[00:00.00]A little less noise\n[00:02.00]A little more room to breathe\n[00:04.00]Your collection, your rules\n[00:06.00]A place to begin\n',encoding='utf-8')
video=media/'Motion Studies';video.mkdir(exist_ok=True)
sub1=video/'en.srt';sub2=video/'de.srt'
sub1.write_text('1\n00:00:00,000 --> 00:00:07,900\nYour collection. Your rules.\n',encoding='utf-8')
sub2.write_text('1\n00:00:00,000 --> 00:00:07,900\nDeine Sammlung. Deine Regeln.\n',encoding='utf-8')
target=video/'Motion.Study.S01E01.mkv'
if not target.exists():
    subprocess.run([str(ffmpeg),'-hide_banner','-loglevel','error','-y','-f','lavfi','-i','testsrc2=size=640x360:rate=24:duration=8','-f','lavfi','-i','sine=frequency=300:duration=8','-i',str(sub1),'-i',str(sub2),'-map','0:v','-map','1:a','-map','2','-map','3','-c:v','libx264','-preset','ultrafast','-crf','28','-c:a','aac','-c:s','srt','-metadata','title=Motion Study — Episode 1','-metadata:s:s:0','language=eng','-metadata:s:s:1','language=deu',str(target)],check=True)
(ROOT/'qa/season.json').write_text(json.dumps({'version':1,'series':'Motion Studies','season':1,'items':[{'path':str(target),'title':'A Place to Begin','episode':1}]},indent=2),encoding='utf-8')
(ROOT/'qa/example.m3u8').write_text('#EXTM3U\n'+str(media/'First Light/Stillwater.flac')+'\n'+str(media/'Sunday, Slowed Down/Slow Mornings.mp3')+'\nmissing-track.mp3\n',encoding='utf-8')
print('Generated 4 audio fixtures, a video with 2 subtitle languages, artwork, lyrics and import manifests.')
