"""【X11虚拟机】【剪贴板交付】核对源端增量服务与 Xfce 管理器接管后的重复读取"""

import hashlib
import io
import json
import re

from PIL import Image, ImageChops
from annotations import config
from gnome_clipboard import owner
from scrolling import texture


def manager(session):
    """【X11虚拟机】【选择区身份】按 session 读取当前 CLIPBOARD 所有者的 PID 和程序名称，返回身份，不改变所有权"""
    return json.loads(session.guest(r"""
import ctypes as c,re
os.environ.update(DISPLAY=':0',XAUTHORITY=str(Path.home()/'.Xauthority'))
lib=c.CDLL('libX11.so.6');lib.XOpenDisplay.restype=c.c_void_p
lib.XInternAtom.argtypes=[c.c_void_p,c.c_char_p,c.c_int];lib.XInternAtom.restype=c.c_ulong
lib.XGetSelectionOwner.argtypes=[c.c_void_p,c.c_ulong];lib.XGetSelectionOwner.restype=c.c_ulong
lib.XCloseDisplay.argtypes=[c.c_void_p]
display=lib.XOpenDisplay(None)
if not display:raise RuntimeError('无法查询 X11 选择区身份')
try:identifier=lib.XGetSelectionOwner(display,lib.XInternAtom(display,b'CLIPBOARD',0))
finally:lib.XCloseDisplay(display)
pid=None;name=None
if identifier:
 prop=subprocess.check_output(['xprop','-id',str(identifier),'_NET_WM_PID'],text=True)
 match=re.search(r'= (\d+)',prop)
 if match:
  pid=int(match.group(1));name=(Path('/proc')/str(pid)/'comm').read_text().strip()
print(json.dumps({'window':hex(identifier),'pid':pid,'name':name}))
"""))


def run(session, binary, overrides):
    """【X11虚拟机】【完整性流程】按 session、binary、overrides 重复读取 PNG 或 UTF-8，返回内容核对和管理器接管结果"""
    session.setup(config(session))
    text = overrides.pop("MARK_SHOT_TEST_CLIPBOARD_KIND", "png") == "text"
    large = overrides.pop("MARK_SHOT_TEST_CLIPBOARD_SIZE", "normal") == "large"
    image = None
    if text:
        data = ("X11 中文剪贴板分块\n" * 900_000).encode()
    else:
        image = (Image.frombytes("RGB", (2048, 4096), hashlib.shake_256(b"mark-shot-x11-incr").digest(2048 * 4096 * 3))
                 if large else texture(320, 512))
        payload = io.BytesIO()
        image.save(payload, format="PNG")
        data = payload.getvalue()
    try:
        pid = owner(session, binary, data, text=text)
        for index in range(2):
            if text:
                actual = json.loads(session.guest("""
import hashlib
env={**os.environ,'DISPLAY':':0','XAUTHORITY':str(Path.home()/'.Xauthority')}
result=subprocess.run(['xclip','-selection','clipboard','-out','-target','UTF8_STRING'],env=env,capture_output=True,check=True,timeout=8)
print(json.dumps({'bytes':len(result.stdout),'sha256':hashlib.sha256(result.stdout).hexdigest()}))
"""))
                assert actual == {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}, "UTF-8 交付字节不一致"
            else:
                with Image.open(session.clipboard(f"copy-{index}.png")) as actual:
                    assert actual.size == image.size and not ImageChops.difference(actual.convert("RGB"), image).getbbox(), "PNG 交付像素不一致"
        identity = manager(session)
        source_alive = session.alive(pid)
        assert source_alive or identity["name"] == "xfsettingsd", identity
        limit = int(re.search(r"属性载荷=(\d+)字节", session.log("owner")).group(1))
        if text or large:
            assert len(data) > limit, "测试载荷没有覆盖真正的增量传输"
        return {"kind": "text" if text else "png", "bytes": len(data), "size": list(image.size) if image else None,
                "server_payload_limit": limit, "source_incr_transfer": len(data) > limit, "repeated_read": True,
                "exact_bytes" if text else "exact_pixels": True, "source_alive": source_alive, "clipboard_owner": identity}
    finally:
        session.log("owner")
        session.cleanup()
