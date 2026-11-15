"""【GNOME虚拟机】【会话控制】复用 libvirt 输入，以临时 Shell 观察状态验证窗口和门户"""

import json
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "kde_vm"))
from session import Session as BaseSession


class GnomeSession(BaseSession):
    """【GNOME虚拟机】【隔离测试】持有 SSH 身份、独立文件目录和本次 Shell 观察状态"""

    def __init__(self, root, host):
        """【GNOME虚拟机】【初始化】按 root、host 创建指定测试虚拟机会话，不改变桌面，返回值为空"""
        setup = Path.home() / "VirtualMachines/arch-gnome-test/setup"
        super().__init__("arch-gnome-test", host, root, ssh_options=(
            "-i", str(setup / "id_ed25519"), "-o", "StrictHostKeyChecking=accept-new",
            "-o", f"UserKnownHostsFile={setup / 'known_hosts'}"), prefix="gnome")
        self.clipboard_binary = "/tmp/mark-shot-gnome-wl-paste"

    def evaluate(self, code):
        """【GNOME虚拟机】【状态观察】按 code 在测试 Shell 中求值，返回 JSON；权限关闭或异常立即失败"""
        return json.loads(self.guest("""
import dbus
bus=dbus.SessionBus();shell=bus.get_object('org.gnome.Shell','/org/gnome/Shell')
success,result=shell.Eval(values,dbus_interface='org.gnome.Shell')
if not success:raise RuntimeError('GNOME observation rejected: '+str(result))
print(result if result else 'null')
""", code))

    def observe(self):
        """【GNOME虚拟机】【观察准备】无参数，通过本次虚拟机键盘打开 Looking Glass 观察状态，返回原观察权限标志"""
        self.guest("subprocess.run(['sudo','-n','loginctl','unlock-session','1'],check=True)")
        self.key("esc")
        try:
            self.evaluate("1+1")
            return False
        except Exception:
            self.key("alt+f2")
            self.text("lg")
            self.key("ret")
            time.sleep(0.4)
            self.text("global.context.unsafe_mode = true")
            self.key("ret")
            self.key("esc")
            self.wait(lambda: self.evaluate("1+1") == 2, "无法启用本次 GNOME 窗口观察")
            return True

    def text(self, value):
        """【GNOME虚拟机】【字符输入】按 value 发送真实 QMP 键盘输入，返回空值，只接受回归所需 ASCII 字符"""
        special = {" ": "spc", ".": "dot", "=": "equal", "_": "shift+minus", "/": "slash", ":": "shift+semicolon",
                   "-": "minus", "#": "shift+3", ",": "comma"}
        for character in value:
            self.key(special.get(character, character))

    def move(self, x, y):
        """【GNOME虚拟机】【指针坐标】按 x、y 映射 Mutter 的完整输出跨度，返回空值，避免尺寸减一造成分数偏移"""
        events = [{"type": "abs", "data": {"axis": axis, "value": round(value * 32767 / extent)}}
                  for axis, value, extent in zip(("x", "y"), (x, y), self.size)]
        self.monitor({"execute": "input-send-event", "arguments": {"events": events}})
        time.sleep(0.08)

    def windows(self, app=None):
        """【GNOME虚拟机】【窗口清单】按 app 返回实际 Meta 窗口几何和状态，空值返回全部，不改变窗口"""
        values = self.evaluate("global.get_window_actors().map(a=>{const w=a.meta_window,r=w.get_frame_rect();return {id:w.get_stable_sequence().toString(),app:w.get_wm_class(),caption:w.get_title(),pid:w.get_pid(),x:r.x,y:r.y,width:r.width,height:r.height,minimized:w.minimized,focused:w.has_focus()}})")
        return [value for value in values if app is None or value["app"] == app]

    def restore(self, identifier):
        """【GNOME虚拟机】【测试窗口恢复】按 identifier 恢复本次 mark-shot 窗口并聚焦，返回空值，拒绝其他程序"""
        assert any(w["id"] == identifier and (w["app"] or "").startswith("mark-shot") for w in self.windows())
        self.evaluate("(()=>{const w=global.get_window_actors().map(a=>a.meta_window).find(w=>w.get_stable_sequence().toString()===" + json.dumps(identifier) + ");w.unminimize();w.activate(global.get_current_time());return true})()")

    def authorize(self):
        """【GNOME虚拟机】【屏幕授权】无参数，使用系统分享按钮的真实键盘助记键确认来源，返回是否发出操作"""
        windows = [w for w in self.windows() if w["app"] == "xdg-desktop-portal-gnome" and "屏幕" in (w["caption"] or "")]
        if not windows:
            return False
        time.sleep(0.3)
        self.key("alt+s")
        return True

    def shortcut_permission(self):
        """【GNOME虚拟机】【快捷键授权】无参数，仅在系统快捷键授权窗口显示时按添加助记键，返回是否发出操作"""
        if any("Keyboard Shortcuts" in (w["caption"] or "") or "键盘快捷键" in (w["caption"] or "")
               for w in self.windows()):
            self.key("alt+a")
            return True
        return False
