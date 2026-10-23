"""Edit an original 1080p showcase from verified native captures/rendered stills.
No generated geometry/images or camera-performance claims. Rasterized local
system-font captions are included; no font file or media-tool binary is bundled.
"""
from __future__ import annotations
import argparse,hashlib,json,math,subprocess,wave
from pathlib import Path
import numpy as np
from PIL import Image,ImageDraw,ImageFont

W,H,FPS=1920,1080,30
BG=(12,17,25);FG=(242,246,250);MUTED=(162,181,196);ACCENT=(91,208,216)
FONTS=Path('C:/Windows/Fonts')
def font(size:int,bold:bool=False):
    candidates=['segoeuib.ttf','arialbd.ttf'] if bold else ['segoeui.ttf','arial.ttf']
    for name in candidates:
        p=FONTS/name
        if p.is_file():return ImageFont.truetype(str(p),size)
    candidate=Path('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf' if bold else '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf')
    if candidate.is_file():return ImageFont.truetype(str(candidate),size)
    raise FileNotFoundError('Use a local system sans-serif font; no font binaries are bundled')
def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def smooth(t):t=max(0.,min(1.,t));return t*t*(3.-2.*t)
def fit(image,size):
    image=image.convert('RGB');image.thumbnail(size,Image.Resampling.LANCZOS);return image

def soundtrack(path:Path,duration:float):
    rate=48000;count=round(duration*rate);audio=np.zeros((count,2),dtype=np.float64)
    # Original low-level ambient bed: mathematical oscillators only, no samples.
    sequence=[(73.416,110.0,146.832,174.614),(65.406,98.0,130.813,164.814),(55.0,82.407,110.0,146.832),(61.735,92.499,123.471,164.814)]
    block=8.0
    for start in np.arange(0,duration,block):
        n=min(round((block+1.5)*rate),count-round(start*rate));t=np.arange(n)/rate
        envelope=np.minimum(t/1.2,1)*np.minimum(((block+1.5)-t)/1.5,1);envelope=np.clip(envelope,0,1)
        frequencies=sequence[int(start/block)%len(sequence)];pad=np.zeros(n)
        for i,f in enumerate(frequencies):pad+=(np.sin(2*np.pi*f*t)+.18*np.sin(2*np.pi*(f*2.002)*t))/(len(frequencies)*(1+i*.3))
        pulse=.78+.22*np.sin(2*np.pi*.25*t)**2;pad*=envelope*pulse*.11
        offset=round(start*rate);audio[offset:offset+n,0]+=pad;audio[offset:offset+n,1]+=pad*.96
    fade=np.minimum(np.arange(count)/rate/2.0,1)*np.minimum((count-np.arange(count))/rate/3.0,1);audio*=fade[:,None]
    peak=np.max(np.abs(audio));assert peak<1.0
    with wave.open(str(path),'wb') as out:out.setnchannels(2);out.setsampwidth(2);out.setframerate(rate);out.writeframes((np.clip(audio,-1,1)*32767).astype('<i2').tobytes())

def compose(manifest_path:Path,ffmpeg:Path,destination:Path):
    manifest=json.loads(manifest_path.read_text(encoding='utf-8'));root=manifest_path.parent
    assert not destination.exists(),'Never overwrite a delivered showcase'
    scenes=manifest['scenes'];assert 6<=len(scenes)<=20
    assets={};inputs=[]
    for scene in scenes:
        assert 3<=scene['seconds']<=18
        for key in ['image','compare']:
            if key in scene:
                path=(root/scene[key]).resolve(strict=True);assert path.is_relative_to(root.resolve())
                digest=sha(path);assert digest==manifest['sources'][scene[key]]['sha256']
                assets[scene[key]]=Image.open(path).convert('RGB');inputs.append({'file':scene[key],'sha256':digest,'pixels':assets[scene[key]].size})
    for source in manifest['sources'].values():assert source['classification'] in ['native_render','native_ui_capture','native_regrade','previous_verified_native_render']
    lengths=[round(s['seconds']*FPS) for s in scenes];frames=sum(lengths);duration=frames/FPS
    local=destination.parent;local.mkdir(parents=True,exist_ok=True);sound=local/'original-ambient.wav';assert not sound.exists();soundtrack(sound,duration)
    title_font=font(54,True);body_font=font(27);small_font=font(20);brand_font=font(21,True)
    log=local/'video-encode.log';receipt=local/'video-receipt.json'
    command=[str(ffmpeg),'-hide_banner','-loglevel','warning','-f','rawvideo','-pixel_format','rgb24','-video_size',f'{W}x{H}','-framerate',str(FPS),'-i','pipe:0','-i',str(sound),'-map','0:v:0','-map','1:a:0','-c:v','libx264','-preset','medium','-threads','3','-crf','18','-pix_fmt','yuv420p','-vf','scale=in_range=pc:out_range=tv:out_color_matrix=bt709','-color_primaries','bt709','-color_trc','bt709','-colorspace','bt709','-c:a','aac','-b:a','160k','-ar','48000','-movflags','+faststart','-shortest','-metadata','title=SolidCraft Render Studio — Aging & Effects','-metadata','comment=Edited showcase of verified native renderer output and application captures. Original synthetic soundtrack. Not real-time rendering.','-n',str(destination)]
    prepared=[]
    for scene in scenes:
        image=assets.get(scene.get('image'))
        if scene.get('kind')=='ui':base=fit(image.copy(),(1824,826));top=126
        else:base=fit(image.copy(),(1920,864));top=106
        prepared.append((base,top))
    base_cache={}
    def frame_of(index,local_frame):
        key=(index,None if 'compare' not in scenes[index] else local_frame)
        if key in base_cache:return base_cache[key].copy()
        scene=scenes[index];base,top=prepared[index];progress=local_frame/max(1,lengths[index]-1)
        canvas=Image.new('RGB',(W,H),BG)
        x=(W-base.width)//2;y=top+(864-base.height)//2 if scene.get('kind')!='ui' else top+(826-base.height)//2
        canvas.paste(base,(x,y))
        if 'compare' in scene:
            comparison=fit(assets[scene['compare']].copy(),(1920,864));assert comparison.size==base.size
            fraction=smooth((progress-.14)/.72);cut=round(base.width*fraction)
            if cut>0:canvas.paste(comparison.crop((0,0,cut,comparison.height)),(x,y))
            if 0<cut<base.width:
                draw=ImageDraw.Draw(canvas);draw.line((x+cut,y,x+cut,y+base.height),fill=ACCENT,width=3)
        draw=ImageDraw.Draw(canvas)
        draw.rectangle((0,0,W,104),fill=BG)
        draw.text((54,23),'SOLIDCRAFT  /  RENDER STUDIO',font=brand_font,fill=FG)
        draw.text((54,57),scene.get('eyebrow','NATIVE RENDERING / DEVELOPMENT SHOWCASE').upper(),font=small_font,fill=ACCENT)
        badge=f'{index+1:02} / {len(scenes):02}';bbox=draw.textbbox((0,0),badge,font=brand_font);draw.text((W-55-(bbox[2]-bbox[0]),31),badge,font=brand_font,fill=MUTED)
        draw.rectangle((0,938,W,H),fill=BG)
        draw.text((54,950),scene['title'],font=title_font,fill=FG)
        subtitle_font=body_font
        for point_size in range(27,19,-1):
            subtitle_font=font(point_size)
            if draw.textlength(scene['subtitle'],font=subtitle_font)<=1806:break
        if draw.textlength(scene['subtitle'],font=subtitle_font)>1806:raise ValueError('Subtitle would clip')
        draw.text((57,1017),scene['subtitle'],font=subtitle_font,fill=MUTED)
        if scene.get('kind')=='ui':draw.text((W-360,77),'ACTUAL APPLICATION CAPTURE',font=font(16),fill=MUTED)
        else:draw.text((W-360,77),scene.get('badge','ACTUAL NATIVE RENDER'),font=font(16),fill=MUTED)
        if 'compare' in scene:
            for label,px in [(scene.get('left_label','AFTER'),75),(scene.get('right_label','BEFORE'),W-240)]:
                draw.rounded_rectangle((px-12,126,px+160,166),radius=7,fill=BG);draw.text((px,135),label,font=small_font,fill=FG)
        # All camera movement/transitions are editorial; source pixels are not repainted.
        if 'compare' not in scene:base_cache[key]=canvas.copy()
        return canvas
    try:
        with log.open('wb') as logs:
            process=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=logs)
            checkpoints=[];absolute=0
            for index,scene in enumerate(scenes):
                previous=frame_of(index-1,lengths[index-1]-1) if index else Image.new('RGB',(W,H),BG)
                for n in range(lengths[index]):
                    image=frame_of(index,n)
                    if n<18:
                        # Crossfade the imagery only. Captions/heading switch
                        # cleanly instead of overlapping two unreadable titles.
                        box=(0,104,W,938)
                        transition=Image.blend(previous.crop(box),image.crop(box),smooth(n/18))
                        image.paste(transition,(0,104))
                    if index==len(scenes)-1 and n>lengths[index]-31:image=Image.blend(image,Image.new('RGB',(W,H),BG),smooth((n-(lengths[index]-31))/30))
                    process.stdin.write(image.tobytes())
                    if n==lengths[index]//2:
                        preview=local/f'chapter-{index+1:02}.jpg';image.save(preview,quality=90);checkpoints.append({'chapter':index+1,'second':(absolute+n)/FPS,'file':preview.name})
                absolute+=lengths[index];print('ENCODED_CHAPTER',index+1,scene['title'],flush=True)
            process.stdin.close();code=process.wait(timeout=600)
        if code:raise RuntimeError(f'Video encoder failed with {code}: '+log.read_text(errors='replace')[-5000:])
        validation=subprocess.run([str(ffmpeg),'-v','error','-i',str(destination),'-f','null','-'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=300)
        assert validation.returncode==0,validation.stderr.decode(errors='replace')
        metadata=subprocess.run([str(ffmpeg),'-hide_banner','-i',str(destination)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,timeout=30).stderr
        (local/'video-metadata.txt').write_text(metadata,encoding='utf-8')
        result={'status':'PASS','file':destination.name,'bytes':destination.stat().st_size,'sha256':sha(destination),'width':W,'height':H,'fps':FPS,'frames':frames,'seconds':duration,'decode_validation':'PASS','visual_review':'PENDING','chapters':checkpoints,'sources':inputs,'soundtrack':'Original mathematical oscillator bed; no samples or third-party music','editing_scope':'Native rendered stills and native UI screenshots with captions, comparison wipes and crossfades; no claim of real-time rendering, physical animation or generated imagery','source_manifest_sha256':sha(manifest_path),'encoder_sha256':sha(ffmpeg)}
        receipt.write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');print(json.dumps(result),flush=True)
        return result
    except BaseException as error:
        receipt.write_text(json.dumps({'status':'FAIL','error':str(error),'file':destination.name},indent=2)+'\n',encoding='utf-8');raise

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--manifest',type=Path,required=True);parser.add_argument('--ffmpeg',type=Path,required=True);parser.add_argument('--out',type=Path,required=True);args=parser.parse_args();compose(args.manifest,args.ffmpeg,args.out)
