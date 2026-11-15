"""【GNOME虚拟机】【剪贴板分块】独立服务大 PNG 和 UTF-8，核验 Wayland 消费者与原始字节"""

import base64
import hashlib
import io
import json
import re

from PIL import Image, ImageChops
from annotations import config
from scrolling import texture


def owner(session, binary, data, text=False):
    """【GNOME虚拟机】【来源服务】按 session、binary、data、text 创建独立剪贴板所有者，返回确认后的 PID"""
    encoded = base64.b64encode(data).decode()
    return int(session.guest("""
import base64
root=Path(values['root']);payload=root/'payload.bin';payload.write_bytes(base64.b64decode(values['data']))
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
environment.update(XDG_CONFIG_HOME=str(root/'config'),XDG_DATA_HOME=str(root/'data'),XDG_STATE_HOME=str(root/'state'),XDG_CACHE_HOME=str(root/'cache'),MARK_SHOT_LOG='info')
with payload.open('rb') as source,(root/'owner.log').open('w') as log:
 process=subprocess.Popen([values['binary'],'--clipboard-daemon',*(['text'] if values['text'] else []),'--config',str(root/'config.toml')],env=environment,stdin=source,stdout=subprocess.PIPE,stderr=log,start_new_session=True)
 import select
 if not select.select([process.stdout],[],[],5)[0]:raise RuntimeError('clipboard owner did not become ready')
 line=process.stdout.readline()
 if line!=b'\\n':raise RuntimeError(line.decode())
print(process.pid)
""", {"root": session.remote, "binary": binary, "data": encoded, "text": text}))


def run(session, binary, overrides):
    """【GNOME虚拟机】【分块流程】按 session、binary、overrides 服务大 PNG 并重复读取，返回大小与完整性，失败仍清理"""
    session.setup(config(session))
    if overrides.pop("MARK_SHOT_TEST_CLIPBOARD_KIND", "png") == "text":
        return text_run(session, binary)
    large = overrides.pop("MARK_SHOT_TEST_CLIPBOARD_SIZE", "normal") == "large"
    image = (Image.frombytes("RGB", (2048, 4096), hashlib.shake_256(b"mark-shot-gnome-incr").digest(2048 * 4096 * 3))
             if large else texture(320, 512))
    output = io.BytesIO()
    image.save(output, format="PNG")
    try:
        pid = owner(session, binary, output.getvalue())
        for index in range(2):
            with Image.open(session.clipboard(f"large-{index}.png")) as received:
                assert received.size == image.size and not ImageChops.difference(received.convert("RGB"), image).getbbox(), "GNOME 大 PNG 分块传输改变像素"
            assert session.alive(pid), "读取后错误结束了剪贴板所有者"
        log = session.log("owner")
        match = re.search(r"属性载荷=(\d+)字节", log)
        limit = int(match.group(1)) if match else None
        if large:
            assert limit and len(output.getvalue()) > limit, "大图没有覆盖服务器上限后的增量传输"
        return {"png_bytes": len(output.getvalue()), "size": list(image.size), "server_payload_limit": limit,
                "incr_transfer": bool(limit and len(output.getvalue()) > limit), "repeated_read": True, "exact_pixels": True}
    finally:
        session.log("owner")
        session.cleanup()


def text_run(session, binary):
    """【GNOME虚拟机】【文字分块】按 session、binary 提供大 UTF-8 来源并核对两次读取的摘要，返回字节数和一致性"""
    data = ("GNOME 中文剪贴板分块\n" * 700_000).encode()
    expected = hashlib.sha256(data).hexdigest()
    try:
        pid = owner(session, binary, data, text=True)
        for _ in range(2):
            result = json.loads(session.guest("""
import hashlib
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
result=subprocess.run([values,'--no-newline','--type','text/plain;charset=utf-8'],env=environment,capture_output=True,timeout=8,check=True)
print(json.dumps({'bytes':len(result.stdout),'sha256':hashlib.sha256(result.stdout).hexdigest()}))
""", session.clipboard_binary))
            assert result == {"bytes": len(data), "sha256": expected}, "GNOME UTF-8 传输内容改变"
            assert session.alive(pid), "读取文字后错误结束所有者"
        limit = int(re.search(r"属性载荷=(\d+)字节", session.log("owner")).group(1))
        assert len(data) > limit, "UTF-8 来源没有覆盖增量传输"
        return {"kind": "text", "utf8_bytes": len(data), "server_payload_limit": limit,
                "incr_transfer": True, "repeated_read": True, "exact_bytes": True}
    finally:
        session.log("owner")
        session.cleanup()
