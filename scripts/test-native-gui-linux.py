#!/usr/bin/env python3
"""Linux/Xvfb startup smoke for both GUIs against a private virtual daemon.
Requires Xvfb, Pillow and python-xlib. No device passthrough or real hardware.
GUI screenshots require visual review; this is not an automated native click test.
Usage: python3 scripts/test-native-gui-linux.py [directory-containing-binaries]
"""
import os, time, tempfile, subprocess, socket, json, sys, signal
from pathlib import Path
from PIL import Image
from Xlib import display,X
repo=Path(__file__).resolve().parent.parent
bin=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else repo/'target/debug'
artifacts=repo/'target/quality-audit';artifacts.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='freergb-native-smoke-') as d:
 root=Path(d);runtime=root/'run';runtime.mkdir(mode=0o700)
 env=dict(os.environ,DISPLAY=':93',XDG_RUNTIME_DIR=str(runtime),XDG_CONFIG_HOME=str(root/'config'),XDG_DATA_HOME=str(root/'data'),LIBGL_ALWAYS_SOFTWARE='1',WEBKIT_DISABLE_DMABUF_RENDERER='1',WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS='1')
 processes=[]
 def spawn(args,name):
  log=open(artifacts/f'{name}.log','w');p=subprocess.Popen(args,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);processes.append((p,log));return p
 def rpc(method,params=None):
  with socket.socket(socket.AF_UNIX) as s:
   s.settimeout(3);s.connect(str(runtime/'freergb.sock'));s.sendall((json.dumps(dict(jsonrpc='2.0',id=1,method=method,params=params or {}))+'\n').encode());r=json.loads(s.makefile().readline());assert 'error' not in r,r;return r['result']
 try:
  xvfb=spawn(['Xvfb',':93','-screen','0','1440x1000x24'],'xvfb');time.sleep(.5)
  daemon=spawn([str(bin/'freergbd'),'--no-hardware','--no-state','--virtual-device','--no-modules'],'daemon')
  for _ in range(100):
   if (runtime/'freergb.sock').exists():break
   assert daemon.poll() is None;time.sleep(.1)
  assert rpc('daemon.version')['api_version']==1
  for name in ['freergb-gui','freergb-desktop']:
   gui=spawn([str(bin/name)],name);time.sleep(8)
   assert gui.poll() is None,(name,gui.returncode)
   assert rpc('devices.list')[0]['frames']>0
   rpc('color.set',{'color':'#00ff00'})
   assert rpc('effects.get')['color']=={'r':0,'g':255,'b':0}
   time.sleep(2)
   screen=display.Display(':93');window=screen.screen().root
   image=window.get_image(0,0,1440,1000,X.ZPixmap,0xffffffff)
   Image.frombytes('RGB',(1440,1000),image.data,'raw','BGRX').save(artifacts/f'{name}.png');screen.close()
   os.killpg(gui.pid,signal.SIGTERM);gui.wait(timeout=10)
   print('PASS:',name,'startup with virtual daemon; API effect round trip; screenshot saved',flush=True)
  rpc('daemon.shutdown');daemon.wait(timeout=10);assert daemon.returncode==0
 finally:
  for process,log in reversed(processes):
   try:os.killpg(process.pid,signal.SIGTERM)
   except ProcessLookupError:pass
   if process.poll() is None:
    process.terminate()
    try:process.wait(timeout=5)
    except subprocess.TimeoutExpired:process.kill();process.wait()
   log.close()
