"""【KDE虚拟机】【会话控制】通过 SSH 与 libvirt 操作指定测试虚拟机，输入不进入宿主桌面"""

import json
from pathlib import Path
import shlex
import subprocess
import time


class Session:
    """【KDE虚拟机】【测试会话】持有虚拟机身份、证据目录和独立配置目录"""

    def __init__(self, domain, host, root, size=(1228, 768), ssh_options=(), prefix="kde"):
        """【虚拟机测试】【会话创建】按 domain、host、root、size、ssh_options、prefix 创建隔离会话，不启动程序，返回值为空"""
        self.domain, self.host = domain, host
        self.ssh_options = list(ssh_options)
        self.clipboard_binary = "wl-paste"
        self.root, self.size = Path(root).resolve(), size
        self.root.mkdir(parents=True, exist_ok=True)
        self.remote = f"/tmp/mark-shot-{prefix}-{self.root.name}"
        self.screenshot("desktop-before")

    def guest(self, code, values=None):
        """【KDE虚拟机】【远程执行】按 code、values 执行客体 Python，返回标准输出；失败保留错误"""
        script = "import json,os,subprocess,time\nfrom pathlib import Path\n"
        script += "values=json.loads(" + repr(json.dumps(values)) + ")\n" + code
        result = subprocess.run(
            ["ssh", *self.ssh_options, "-o", "BatchMode=yes", "-o", "ConnectTimeout=5", self.host, "python -"],
            input=script, text=True, capture_output=True, timeout=30,
        )
        if result.returncode:
            raise RuntimeError(result.stderr.strip() or result.stdout.strip() or f"远程执行失败: {result.returncode}")
        return result.stdout.strip()

    def setup(self, config):
        """【KDE虚拟机】【配置准备】按 config 创建独立客体配置，返回值为空，不写用户配置"""
        self.guest("""
root=Path(values['root']);root.mkdir(parents=True,exist_ok=True)
(root/'config.toml').write_text(values['config'])
for name in ['config','data','state','cache','output']:(root/name).mkdir(exist_ok=True)
""", {"root": self.remote, "config": config})

    def launch(self, binary, args=(), name="capture", overrides=None, with_config=True):
        """【虚拟机测试】【程序启动】按 binary、args、name、overrides、with_config 启动测试进程，返回 PID；继承当前桌面环境"""
        return int(self.guest("""
root=Path(values['root'])
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
environment.update(XDG_CONFIG_HOME=str(root/'config'),XDG_DATA_HOME=str(root/'data'),XDG_STATE_HOME=str(root/'state'),XDG_CACHE_HOME=str(root/'cache'),MARK_SHOT_LOG='info')
environment.update(values['overrides'])
with (root/(values['name']+'.log')).open('w') as log:
 arguments=[values['binary'],*(['--config',str(root/'config.toml')] if values['with_config'] else []),*values['args']]
 process=subprocess.Popen(arguments,env=environment,stdin=subprocess.DEVNULL,stdout=log,stderr=log,start_new_session=True)
(root/(values['name']+'.pid')).write_text(str(process.pid))
print(process.pid)
""", {"root": self.remote, "binary": binary, "args": list(args), "name": name, "overrides": overrides or {}, "with_config": with_config}))

    def alive(self, pid):
        """【KDE虚拟机】【进程检查】按 pid 返回是否仍运行，僵尸进程视为已退出"""
        return self.guest("""
path=Path('/proc')/str(values)/'stat'
print('yes' if path.exists() and path.read_text().split()[2]!='Z' else 'no')
""", pid) == "yes"

    def command(self, binary, args, overrides=None):
        """【KDE虚拟机】【同步命令】按 binary、args、overrides 执行独立配置命令，返回输出；不创建后台测试进程"""
        return self.guest("""
root=Path(values['root']);environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
environment.update(XDG_CONFIG_HOME=str(root/'config'),XDG_DATA_HOME=str(root/'data'),XDG_STATE_HOME=str(root/'state'),XDG_CACHE_HOME=str(root/'cache'),MARK_SHOT_LOG='info')
environment.update(values['overrides'])
result=subprocess.run([values['binary'],'--config',str(root/'config.toml'),*values['args']],env=environment,text=True,capture_output=True,timeout=20,check=True)
print(result.stdout)
""", {"root": self.remote, "binary": binary, "args": args, "overrides": overrides or {}})

    def wait(self, predicate, description, timeout=10):
        """【KDE虚拟机】【状态等待】按 predicate 等待条件成立并返回值；timeout 秒后以 description 报告失败"""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = predicate()
            if value:
                return value
            time.sleep(0.15)
        raise AssertionError(description)

    def files(self, directory, pattern):
        """【KDE虚拟机】【产物列表】按 directory、pattern 返回本次测试目录的匹配路径，不扫描用户目录"""
        assert directory.startswith(self.remote + "/")
        return json.loads(self.guest("print(json.dumps(sorted(str(p) for p in Path(values[0]).glob(values[1]))))", [directory, pattern]))

    def clipboard(self, name="clipboard.png"):
        """【KDE虚拟机】【剪贴板读取】将当前 PNG 复制到本次客体证据目录，返回下载路径，不改变所有权"""
        remote = f"{self.remote}/{name}"
        self.guest("""
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
deadline=time.monotonic()+5
while True:
 try:
  result=subprocess.run([values['binary'],'--no-newline','--type','image/png'],env=environment,capture_output=True,timeout=5)
 except subprocess.TimeoutExpired as error:
  raise RuntimeError('PNG读取超时，已接收字节数='+str(len(error.stdout or b''))) from error
 if result.returncode==0:break
 if time.monotonic()>=deadline:raise RuntimeError(result.stderr.decode())
 time.sleep(.1)
Path(values['path']).write_bytes(result.stdout)
""", {"path": remote, "binary": self.clipboard_binary})
        return self.download(remote, name)

    def owned(self):
        """【KDE虚拟机】【子进程查询】无参数，返回环境中使用本次独立数据目录的进程，便于清理派生窗口"""
        return json.loads(self.guest("""
result=[];marker=('XDG_DATA_HOME='+values+'/data').encode()
for path in Path('/proc').iterdir():
 if not path.name.isdigit():continue
 try:
  if marker in (path/'environ').read_bytes().split(b'\\0'):result.append(int(path.name))
 except (OSError,PermissionError):pass
print(json.dumps(result))
""", self.remote))

    def cleanup(self):
        """【KDE虚拟机】【会话清理】无参数，仅终止 owned 找到的本次派生进程，不按程序名称终止"""
        for pid in self.owned():
            self.stop(pid)

    def kwin(self, code):
        """【KDE虚拟机】【窗口脚本】按 code 执行本次 KWin 脚本并返回 JSON 日志，完成后卸载，不保留桌面脚本"""
        return json.loads(self.guest("""
env=dict(os.environ,XDG_RUNTIME_DIR='/run/user/1000',DBUS_SESSION_BUS_ADDRESS='unix:path=/run/user/1000/bus')
root=Path(values['root']);name='markshot-audit-'+root.name
marker='MARKSHOT_KDE_'+str(time.time_ns())+' '
path=root/'windows.js';path.write_text("print("+json.dumps(marker)+"+JSON.stringify((function(){"+values['code']+"})()));")
command=['qdbus6','org.kde.KWin']
identifier=subprocess.check_output(command+['/Scripting','org.kde.kwin.Scripting.loadScript',str(path),name],env=env,text=True).strip()
try:
 subprocess.run(command+['/Scripting/Script'+identifier,'org.kde.kwin.Script.run'],env=env,capture_output=True,check=True)
 time.sleep(.08)
 logs=subprocess.check_output(['journalctl','--user','--since','30 seconds ago','-o','cat'],text=True)
 print(next(line.split(marker,1)[1] for line in logs.splitlines() if marker in line))
finally:subprocess.run(command+['/Scripting','org.kde.kwin.Scripting.unloadScript',name],env=env,capture_output=True,check=True)
""", {"root": self.remote, "code": code}))

    def windows(self, app=None):
        """【KDE虚拟机】【窗口清单】按 app 返回 KWin 真实窗口及状态，空值返回全部，不修改窗口"""
        windows = self.kwin("return workspace.windowList().map(w=>({id:w.internalId.toString(),app:w.resourceClass.toString(),caption:w.caption,x:w.frameGeometry.x,y:w.frameGeometry.y,width:w.frameGeometry.width,height:w.frameGeometry.height,minimized:w.minimized}));")
        return [window for window in windows if app is None or window["app"] == app]

    def restore(self, identifier):
        """【KDE虚拟机】【窗口恢复】按 identifier 恢复本次标注窗口并给予焦点，其他程序不能通过此入口操作"""
        assert any(window["id"] == identifier and window["app"].startswith("mark-shot") for window in self.windows())
        self.kwin("const w=workspace.windowList().find(w=>w.internalId.toString()===" + json.dumps(identifier) + ");w.minimized=false;workspace.activeWindow=w;return true;")

    def stop(self, pid):
        """【KDE虚拟机】【进程结束】按 pid 结束本次测试程序，返回值为空，不按名称终止其他程序"""
        self.guest("""
import signal
try:os.kill(values,signal.SIGTERM)
except ProcessLookupError:pass
""", pid)

    def log(self, name):
        """【KDE虚拟机】【日志领取】按 name 保存对应客体日志到证据目录，返回日志文本"""
        text = self.guest("path=Path(values);print(path.read_text() if path.exists() else '')", f"{self.remote}/{name}.log")
        (self.root / f"{name}.log").write_text(text)
        return text

    def monitor(self, command):
        """【KDE虚拟机】【QMP调用】按 command 调用指定虚拟机监控接口，返回结果，协议错误抛出异常"""
        result = subprocess.check_output(
            ["virsh", "-c", "qemu:///system", "qemu-monitor-command", self.domain, json.dumps(command)],
            text=True, timeout=10,
        )
        value = json.loads(result)
        if "error" in value:
            raise RuntimeError(value["error"])
        return value.get("return")

    def screenshot(self, name):
        """【KDE虚拟机】【画面保存】按 name 保存虚拟显卡画面，返回 PNG 路径，更新客体实际尺寸"""
        from PIL import Image
        path = self.root / f"{name}.png"
        subprocess.run(["virsh", "-c", "qemu:///system", "screenshot", self.domain, str(path)],
                       check=True, capture_output=True, timeout=10)
        with Image.open(path) as image:
            self.size = image.size
        return path

    def move(self, x, y):
        """【KDE虚拟机】【指针移动】按 x、y 客体像素坐标发送绝对鼠标事件，返回值为空"""
        events = [{"type": "abs", "data": {"axis": axis, "value": round(value * 32767 / (extent - 1))}}
                  for axis, value, extent in zip(("x", "y"), (x, y), self.size)]
        self.monitor({"execute": "input-send-event", "arguments": {"events": events}})
        time.sleep(0.08)

    def button(self, button="left", pressed=True):
        """【KDE虚拟机】【鼠标按钮】按 button、pressed 发送客体鼠标按键，返回值为空"""
        self.monitor({"execute": "input-send-event", "arguments": {"events": [
            {"type": "btn", "data": {"button": button, "down": pressed}}
        ]}})

    def click(self, x, y, button="left"):
        """【KDE虚拟机】【完整点击】按 x、y、button 移动并完成按下释放，返回值为空"""
        self.move(x, y)
        self.button(button, True)
        time.sleep(0.06)
        self.button(button, False)
        time.sleep(0.15)

    def drag(self, start, end):
        """【KDE虚拟机】【区域拖动】按 start、end 发送连续左键拖动，确保释放，返回值为空"""
        self.move(*start)
        self.button()
        try:
            for step in range(1, 13):
                self.move(*(start[index] + (end[index] - start[index]) * step / 12 for index in range(2)))
        finally:
            self.button(pressed=False)
        time.sleep(0.2)

    def key(self, chord):
        """【KDE虚拟机】【键盘输入】按 chord 的 QEMU 键名完成组合键，返回值为空"""
        keys = chord.split("+")
        for pressed, values in [(True, keys), (False, list(reversed(keys)))]:
            events = [{"type": "key", "data": {"down": pressed, "key": {"type": "qcode", "data": key}}} for key in values]
            self.monitor({"execute": "input-send-event", "arguments": {"events": events}})
            time.sleep(0.08)
        time.sleep(0.15)

    def download(self, remote, name):
        """【KDE虚拟机】【文件领取】按 remote 下载测试产物为 name，返回本地路径，不读取其他目录"""
        path = self.root / name
        subprocess.run(["scp", "-q", *self.ssh_options, "-o", "BatchMode=yes", f"{self.host}:{shlex.quote(remote)}", str(path)], check=True, timeout=30)
        return path

    def scroll(self, down=True, steps=1):
        """【KDE虚拟机】【滚轮输入】按 down、steps 发送客体真实滚轮刻度，返回值为空"""
        for _ in range(steps):
            self.button("wheel-down" if down else "wheel-up", True)
            self.button("wheel-down" if down else "wheel-up", False)
            time.sleep(0.12)
