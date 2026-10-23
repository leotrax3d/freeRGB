#!/usr/bin/env python3
"""Linux end-to-end smoke: real daemon processes, IPC, installs, update and restart."""
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time

repo=Path(__file__).resolve().parent.parent
daemon=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else repo/'target/debug/freergbd'
if sys.platform != 'linux':
    raise SystemExit('This process smoke uses Linux sockets; cross-platform daemon integration tests are in cargo test --workspace.')
with tempfile.TemporaryDirectory(prefix='freergb-effects-') as tmp:
    root=Path(tmp)
    release=root/'release';release.mkdir()
    catalog=json.loads((repo/'effects/catalog.json').read_text())
    for entry in catalog['effects']:
        shutil.copyfile(repo/'effects/packages'/entry['artifact']['url'],release/entry['artifact']['url'])
    (release/'catalog.json').write_text(json.dumps(catalog))
    runtime=root/'run';runtime.mkdir(mode=0o700)
    environment=os.environ.copy()
    environment.update(XDG_CONFIG_HOME=str(root/'config'),XDG_DATA_HOME=str(root/'data'),XDG_RUNTIME_DIR=str(runtime))
    process=None
    log=(root/'daemon.log').open('w+')
    def call(method,params=None,allow_error=False):
        with socket.socket(socket.AF_UNIX) as s:
            s.settimeout(3)
            s.connect(str(runtime/'freergb.sock'))
            s.sendall((json.dumps(dict(jsonrpc='2.0',id=1,method=method,params=params or {}))+'\n').encode())
            reply=json.loads(s.makefile().readline())
            if allow_error: return reply
            assert 'error' not in reply,reply
            return reply['result']
    def start():
        global process
        process=subprocess.Popen([str(daemon),'--no-hardware','--virtual-device','--no-modules','--effect-dev-directory',str(release)],env=environment,stdout=log,stderr=subprocess.STDOUT)
        deadline=time.monotonic()+10
        while True:
            assert process.poll() is None,'daemon exited'
            try:
                call('daemon.version');return
            except (FileNotFoundError,ConnectionRefusedError):
                if time.monotonic()>deadline: raise
                time.sleep(.05)
    def stop():
        global process
        if process is not None:
            process.send_signal(signal.SIGINT)
            process.wait(timeout=10)
            assert process.returncode==0,process.returncode
            process=None
    def complete():
        deadline=time.monotonic()+10
        while True:
            status=call('marketplace.status')
            if status['packages']['job'] is None:
                assert status['packages']['error'] is None,status
                return status
            assert time.monotonic()<deadline,'job timed out'
            time.sleep(.02)
    try:
        start()
        call('marketplace.refresh');complete()
        call('marketplace.install',{'id':'aurora'});complete()
        selected=dict(type='marketplace',id='aurora',version='1.0.0',parameters=dict(speed=.7,brightness=.6))
        call('effects.set',selected)
        before=call('diagnostics.get')['ticks'];time.sleep(.2)
        assert call('diagnostics.get')['ticks']>before
        assert call('marketplace.status')['runtime_error'] is None
        assert call('devices.list')[0]['frames']>0
        assert 'error' in call('marketplace.uninstall',{'id':'aurora'},True)
        stop()
        # No GUI and no refresh are involved in restoring the selected program.
        start();assert call('effects.get')==selected
        assert call('marketplace.status')['runtime_error'] is None
        catalog['sequence']=2;catalog['effects'][0]['version']='1.1.0'
        (release/'catalog.json').write_text(json.dumps(catalog))
        call('marketplace.refresh');complete()
        call('marketplace.install',{'id':'aurora'});status=complete()
        assert status['packages']['installed'][0]['version']=='1.1.0'
        assert call('effects.get')==selected,'update replaced running effect'
        selected['version']='1.1.0';call('effects.set',selected)
        stop();start();assert call('effects.get')==selected
        call('effects.set',dict(type='solid',color=dict(r=0,g=0,b=0)))
        call('marketplace.uninstall',{'id':'aurora'})
        assert call('marketplace.status')['packages']['installed']==[]
        call('marketplace.install',{'id':'aurora'});complete();call('effects.set',selected)
        stop()
        package=root/'data/freergb/effect-marketplace/packages/aurora/1.1.0.wasm'
        package.write_bytes(b'corrupt')
        start()
        assert call('marketplace.status')['runtime_error'] is not None
        assert call('effects.get')['type']!='marketplace'
        call('color.set',{'color':'#123456'})
        stop()
        print('PASS: real daemon IPC, file download, installation, rendering, persisted restart, version-pinned update, uninstall and corruption recovery')
    except BaseException:
        log.flush();log.seek(0);print(log.read(),file=sys.stderr)
        raise
    finally:
        if process is not None:
            process.kill();process.wait(timeout=5)
        log.close()
