"""【X11虚拟机】【会话控制】复用客体输入，使用原生 X11 窗口状态与选择区读取"""

import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "kde_vm"))
from session import Session as BaseSession


class X11Session(BaseSession):
    """【X11虚拟机】【隔离会话】只操作指定 Xfce 虚拟机和本次测试进程"""

    def __init__(self, root, host=None):
        """【X11虚拟机】【初始化】按 root、host 创建测试会话，校验原生 X11，返回值为空"""
        setup = Path.home() / "VirtualMachines/arch-x11-test/setup"
        if host is None:
            interfaces = subprocess.check_output(["virsh", "-c", "qemu:///system", "domifaddr", "arch-x11-test", "--source", "agent"], text=True)
            address = next(line.split()[-1].split("/")[0] for line in interfaces.splitlines()
                           if "ipv4" in line)
            host = "tester@" + address
        super().__init__("arch-x11-test", host, root, ssh_options=(
            "-i", str(setup / "id_ed25519"), "-o", "StrictHostKeyChecking=accept-new",
            "-o", f"UserKnownHostsFile={setup / 'known_hosts'}"), prefix="x11")
        environment = json.loads(self.guest(r"""
for path in Path('/proc').iterdir():
 if not path.name.isdigit():continue
 try:
  if (path/'comm').read_text().strip()!='xfce4-session':continue
  environment=dict(item.decode().split('=',1) for item in (path/'environ').read_bytes().split(b'\0') if b'=' in item)
  assert environment['XDG_SESSION_TYPE']=='x11',environment
  assert not environment.get('WAYLAND_DISPLAY'),environment
  wanted=['DISPLAY','XAUTHORITY','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS','XDG_SESSION_TYPE','XDG_CURRENT_DESKTOP']
  result={key:environment[key] for key in wanted if key in environment}
  subprocess.run(['systemctl','--user','import-environment',*result],env={**os.environ,**result},check=True)
  print(json.dumps(result));break
 except (FileNotFoundError,PermissionError):continue
else:raise RuntimeError('没有原生 X11 的 Xfce 会话')
"""))
        (self.root / "environment.json").write_text(json.dumps(environment, indent=2) + "\n")
        self.clipboard_binary = "/tmp/mark-shot-x11-xclip"
        self.guest(r"""
path=Path(values)
path.write_text('#!/bin/sh\ncase "$*" in\n *image/png*) exec xclip -selection clipboard -out -target image/png ;;\n *) exec xclip -selection clipboard -out -target UTF8_STRING ;;\nesac\n')
path.chmod(0o700)
""", self.clipboard_binary)

    def windows(self, app=None):
        """【X11虚拟机】【窗口清单】按 app 返回客户区、最小化和焦点状态，空值返回全部，不修改窗口"""
        result = json.loads(self.guest(r"""
import re
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
active=subprocess.check_output(['xprop','-root','_NET_ACTIVE_WINDOW'],env=environment,text=True).strip().split()[-1]
focus=subprocess.check_output(['xdotool','getwindowfocus','-f'],env=environment,text=True).strip()
clients={}
for line in subprocess.check_output(['wmctrl','-lpGx'],env=environment,text=True).splitlines():
 fields=line.split(None,9)
 if len(fields)<9:continue
 clients[int(fields[0],16)]=fields
extra=subprocess.run(['xdotool','search','--onlyvisible','--classname','^mark-shot$'],env=environment,text=True,capture_output=True)
for identifier in extra.stdout.splitlines():clients.setdefault(int(identifier),None)
result=[]
for number,fields in clients.items():
 identifier=f'0x{number:08x}'
 try:
  info=subprocess.check_output(['xwininfo','-id',identifier],env=environment,text=True,stderr=subprocess.DEVNULL)
  prop=subprocess.check_output(['xprop','-id',identifier,'WM_STATE','WM_CLASS','_NET_WM_STATE','_NET_WM_PID','_NET_WM_NAME'],env=environment,text=True,stderr=subprocess.DEVNULL)
 except subprocess.CalledProcessError:continue
 names=re.search(r'WM_CLASS.*?= "(.*?)", "(.*?)"',prop)
 resource=names.group(1) if names else fields[7].split('.')[0] if fields else ''
 geometry=[int(re.search(label+r':\s*(-?\d+)',info).group(1)) for label in ['Absolute upper-left X','Absolute upper-left Y','Width','Height']]
 name=re.search(r'_NET_WM_NAME.*?= "(.*?)"',prop)
 pid=re.search(r'_NET_WM_PID.*?= (\d+)',prop)
 result.append(dict(id=identifier,app=resource,caption=name.group(1) if name else fields[9] if fields and len(fields)>9 else '',pid=int(pid.group(1)) if pid else int(fields[2]) if fields else 0,x=geometry[0],y=geometry[1],width=geometry[2],height=geometry[3],minimized='Iconic' in prop,focused=number==int(focus),active=number==int(active,16),maximized='_NET_WM_STATE_MAXIMIZED_HORZ' in prop))
print(json.dumps(result))
"""))
        # 1. 【X11虚拟机】【窗口身份】原生窗口共用 WM_CLASS，按实际标题识别各个测试界面
        titles = {"Region detection target": "mark-shot-region-detection-target", "Scroll test page": "mark-shot-scroll-test-page",
                  "Pinned Mark Shot": "mark-shot-pin", "Mark Shot — Settings": "mark-shot-settings",
                  "Mark Shot - Edit": "mark-shot-editor", "Mark Shot — Color": "mark-shot-color",
                  "Mark Shot / OCR": "mark-shot-ocr-model-guide"}
        for value in result:
            if value["app"] == "mark-shot":
                value["app"] = titles.get(value["caption"], value["app"])
        return [value for value in result if app is None or value["app"] == app]

    def text(self, value):
        """【X11虚拟机】【字符输入】按 value 发送回归所需 ASCII 字符，使用真实 QMP 键盘，返回值为空"""
        special = {" ": "spc", ".": "dot", "=": "equal", "_": "shift+minus", "/": "slash", ":": "shift+semicolon",
                   "-": "minus", "#": "shift+3", ",": "comma"}
        for character in value:
            self.key(special.get(character, character))

    def restore(self, identifier):
        """【X11虚拟机】【窗口恢复】按 identifier 恢复本次测试窗口并聚焦，拒绝其他应用，返回值为空"""
        assert any(w["id"] == identifier and w["app"].startswith("mark-shot") for w in self.windows())
        self.window_command(identifier, ["wmctrl", "-ia"])

    def window_command(self, identifier, command):
        """【X11虚拟机】【窗口操作】按 identifier、command 发送原生窗口请求，返回值为空，仅允许当前清单中的窗口"""
        assert any(w["id"] == identifier for w in self.windows())
        self.guest("""
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
subprocess.run([*values['command'],values['id']],env=environment,check=True)
""", {"id": identifier, "command": command})
