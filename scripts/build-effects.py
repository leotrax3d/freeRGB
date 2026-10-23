#!/usr/bin/env python3
"""Build portable Rust effects and the official GitHub release catalog."""
import argparse
import re
import hashlib
import json
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parent.parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--version',default='1.0.0')
parser.add_argument('--sequence',type=int,default=1)
parser.add_argument('--out-dir',type=Path,default=root/'effects')
args=parser.parse_args()
if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+',args.version) or args.sequence<1:
    parser.error('Use a numeric major.minor.patch version and a positive catalog sequence')
out=args.out_dir/'packages'
out.mkdir(parents=True,exist_ok=True)
entries=[]
examples=[('aurora','Aurora','Flowing teal and violet curtains.','Ambient',['#10a37f','#8b5cf6']),('comet','Comet','A warm comet crossing the shared device canvas.','Motion',['#ffb454','#3344ff']),('prism','Prism','A vivid two-dimensional field of shifting colour.','Colour',['#f43f5e','#3b82f6'])]
for id,name,description,category,preview in examples:
    asset=f'{id}-{args.version}.wasm'
    subprocess.run(['rustc',str(root/'effects/examples/effect.rs'),'--crate-name',f'effect_{id}','--edition','2021','--crate-type','cdylib','--target','wasm32-unknown-unknown','-C','opt-level=3','-C','panic=abort','-C','strip=symbols','--cfg',f'effect="{id}"','-o',str(out/asset)],check=True)
    (out/asset).chmod(0o644)
    data=(out/asset).read_bytes()
    entries.append(dict(id=id,version=args.version,api=1,publisher=dict(id='leotrax3d',name='freeRGB Team'),name=name,description=description,category=category,preview=preview,permissions=[],parameters=[dict(key='speed',label='Speed',min=0.0,max=3.0,default=1.0,step=0.05),dict(key='brightness',label='Intensity',min=0.0,max=1.0,default=1.0,step=0.01)],artifact=dict(target='wasm32-unknown-unknown',url=asset,size=len(data),sha256=hashlib.sha256(data).hexdigest())))
(args.out_dir/'catalog.json').write_text(json.dumps(dict(schema=1,sequence=args.sequence,effects=entries),indent=2)+'\n')
print('Built',len(entries),'Rust WebAssembly effects')
