#!/usr/bin/env python3
"""Dependency-free verification/packaging of the reviewed committed MP4.
This does not render, compile, install dependencies or certify a later binary.
"""
from __future__ import annotations
import argparse,hashlib,json,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
SOURCE=ROOT/'assets/render/showcase/aging-effects-20261023'
def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def package(output):
    output=output.resolve()
    if output.exists():raise ValueError('Package destination must not already exist')
    manifest=json.loads((SOURCE/'SHA256.json').read_text(encoding='utf-8'))
    records=manifest['files']
    if not 1<=len(records)<=200:raise ValueError('Unexpected showcase inventory')
    names=set()
    for item in records:
        relative=Path(item['path']);raw=SOURCE/relative;source=raw.resolve(strict=True)
        if relative.is_absolute() or '..' in relative.parts or not source.is_relative_to(SOURCE.resolve()) or raw.is_symlink() or not source.is_file():raise ValueError('Unsafe showcase path')
        if item['path'] in names:raise ValueError('Duplicate showcase path')
        names.add(item['path'])
        if source.stat().st_size!=item['bytes'] or digest(source)!=item['sha256']:raise ValueError('Committed showcase byte identity mismatch: '+item['path'])
    video=json.loads((SOURCE/'video-receipt.json').read_text(encoding='utf-8'))
    if video['status']!='PASS' or video['visual_review']!='ACCEPTED' or video['decode_validation']!='PASS':raise ValueError('Video has not passed execution and visual review')
    film=SOURCE/'SolidCraft-Aging-Effects-Showcase.mp4'
    if digest(film)!=video['sha256']:raise ValueError('Selected MP4 differs from accepted encoding')
    selected=sorted(names)+['SHA256.json']
    allowed={'.mp4','.png','.solidcraft','.json','.md'}
    if any(Path(name).suffix.lower() not in allowed for name in selected):raise ValueError('Unexpected binary or font in showcase inventory')
    output.mkdir(parents=True)
    for name in selected:
        source=SOURCE/name;target=output/name;target.parent.mkdir(parents=True,exist_ok=True)
        with source.open('rb') as stream,target.open('xb') as destination:shutil.copyfileobj(stream,destination,length=1024*1024)
        if digest(source)!=digest(target):raise ValueError('Copied package bytes changed')
    report={'status':'PASS','mp4_sha256':video['sha256'],'files':len(selected),'width':video['width'],'height':video['height'],'fps':video['fps'],'seconds':video['seconds'],
        'scope':'Byte-preserving delivery of an already reviewed native showcase; no new renderer or aggregate-CI claim; no fonts or tool binaries included'}
    (output/'PACKAGE.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8',newline='\n')
    return report
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--out',type=Path,required=True);args=parser.parse_args();print(json.dumps(package(args.out),indent=2))
