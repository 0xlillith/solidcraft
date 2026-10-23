"""Original native CAD material study: a compact instrument head on a display plinth.
All geometry is authored through public SolidCraft commands. No imported model,
font, source mesh or geometry replacement; this is a visual study, not a certified
functional assembly. Rendering/aging/lighting are separate saved-shot intent.
"""
from __future__ import annotations
import math,json
from pathlib import Path

def commands()->list[dict]:
    output=[]
    def add(command,**params):output.append({'command':command,'params':params})
    def body(command,name,**params):add(command,operation='new',body_name=name,**params)
    def look(name,color,metallic,roughness):add('appearance.assign',bodies=[name],color=color,opacity=1,pbr={'metallic':metallic,'roughness':roughness,'dielectric_ior':1.5})
    body('solid.cylinder','Display plinth',radius=60,height=5,base=[0,0,0]);look('Display plinth',[36,43,53],0.4,0.32)
    body('solid.torus','Plinth rim',major=58.5,minor=1.5,center=[0,0,4.2]);look('Plinth rim',[74,84,95],0.9,0.24)
    body('solid.cylinder','Steel flange',radius=36,height=7,base=[0,0,5]);look('Steel flange',[150,161,175],1.0,0.24)
    body('solid.cylinder','Painted housing',radius=28.5,height=36,base=[0,0,12]);look('Painted housing',[24,75,96],0.0,0.28)
    body('solid.torus','Lower seal',major=28.0,minor=1.5,center=[0,0,14]);look('Lower seal',[22,26,28],0.0,0.8)
    body('solid.torus','Upper seal',major=27.7,minor=1.3,center=[0,0,46]);look('Upper seal',[22,26,28],0.0,0.8)
    body('solid.cylinder','Steel collar',radius=30.4,height=5,base=[0,0,46.5]);look('Steel collar',[170,181,194],1.0,0.18)
    body('solid.torus','Collar highlight',major=29.6,minor=1.0,center=[0,0,51]);look('Collar highlight',[180,191,203],1.0,0.16)
    body('solid.sphere','Copper dome',radius=21,center=[0,0,52]);look('Copper dome',[193,112,62],1.0,0.22)
    for index in range(8):
        angle=2*math.pi*index/8
        x,y=32*math.cos(angle),32*math.sin(angle)
        body('solid.torus',f'Fastener washer {index+1}',major=2.4,minor=.6,center=[x,y,12.4]);look(f'Fastener washer {index+1}',[142,153,169],1.0,.28)
        body('solid.cylinder',f'Fastener head {index+1}',radius=2.6,height=2.4,base=[x,y,12.5]);look(f'Fastener head {index+1}',[172,184,199],1.0,.2)
        body('solid.box',f'Fastener slot {index+1}',length=3.5,width=.55,height=.08,center=[x,y,14.93]);look(f'Fastener slot {index+1}',[14,17,22],0.0,.8)
    for side in [-1,1]:
        base=[-43,0,28] if side==-1 else [26,0,28]
        body('solid.cylinder',f'Port ferrule {side}',radius=7.0,height=17,axis='X',base=base);look(f'Port ferrule {side}',[180,138,68],1.0,.3)
        end=-43.05 if side==-1 else 43.0
        body('solid.cylinder',f'Port recess {side}',radius=4.8,height=.08,axis='X',base=[end,0,28]);look(f'Port recess {side}',[12,17,19],0.0,.75)
    for index in range(6):
        angle=-math.pi/2+(index-2.5)*.18
        x,y=28.4*math.cos(angle),28.4*math.sin(angle)
        body('solid.cylinder',f'Front indicator {index+1}',radius=.65,height=.2,axis=[math.cos(angle),math.sin(angle),0],base=[x,y,31])
        look(f'Front indicator {index+1}',[215,164,63] if index<3 else [44,163,178],0.0,.23)
    add('view.apply',camera={'target':[0,0,35],'yaw':-.72,'pitch':.42,'roll':0,'distance':325,'fov':.48,'up':'z'})
    return output

def write(destination:Path)->None:
    if destination.exists():raise FileExistsError(destination)
    destination.write_text(json.dumps({'commands':commands()},indent=2,allow_nan=False)+'\n',encoding='utf-8')

if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--out',type=Path,required=True);args=parser.parse_args();write(args.out)
