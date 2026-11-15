"""【GNOME虚拟机】【录屏回归】通过原生录屏工具与门户授权核验 MP4 和 GIF 成品"""

import io
import json
import subprocess
import time

from PIL import Image, ImageChops, ImageStat
from annotations import config
from checks import visible_selection
from gnome_capture import prime
from recording import phase, status


def parameters(session):
    """【GNOME虚拟机】【小工具栏参数】按 session 用真实滚轮调整并恢复帧率，返回空值，核对字段和选区呈现"""
    session.move(1100, 100)
    before = session.screenshot("fps-before")
    session.move(280, 661)
    session.scroll(down=False)
    session.move(1100, 100)
    increased = session.screenshot("fps-increased")
    session.move(280, 661)
    session.scroll(down=True)
    session.move(1100, 100)
    restored = session.screenshot("fps-restored")
    with Image.open(before) as a, Image.open(increased) as b, Image.open(restored) as c:
        area = (240, 644, 340, 682)
        original, changed, reset = [image.convert("RGB").crop(area) for image in (a, b, c)]
        assert ImageChops.difference(original, changed).getbbox(), "小工具栏帧率没有响应滚轮"
        assert not ImageChops.difference(original, reset).getbbox(), "反向滚轮没有恢复帧率"
    visible_selection(restored)


def run(session, binary, overrides):
    """【GNOME虚拟机】【录制流程】按 session、binary、overrides 授权、暂停恢复停止并解码媒体，返回结果"""
    format = overrides.pop("MARK_SHOT_TEST_FORMAT", "mp4")
    assert format in ("mp4", "gif")
    environment = {"MARK_SHOT_MEDIA_LIBRARY": "/tmp/libmarkshot-media.so", **overrides}
    prime(session, binary, environment)
    session.setup(config(session) + '[recording]\nfps=12\ncountdown=0\naudio="off"\nencoder="software"\nformat=' + json.dumps(format) + '\ndirectory=' + json.dumps(session.remote + "/output") + '\n')
    source = session.screenshot("recording-source")
    session.launch(binary, ["--speed"], overrides=environment)
    try:
        session.wait(lambda: session.windows("mark-shot"), "录屏选择界面没有显示")
        time.sleep(0.3)
        session.drag((150, 100), (790, 580))
        session.key("ctrl+shift+g" if format == "gif" else "ctrl+shift+r")
        visible_selection(session.screenshot("recording-tool"))
        parameters(session)
        assert status(session, binary)["phase"] == "idle", "选择工具直接启动了采集"
        session.key("ret")
        session.wait(lambda: session.windows("xdg-desktop-portal-gnome") or status(session, binary)["phase"] == "recording", "没有录屏授权或采集状态")
        if session.windows("xdg-desktop-portal-gnome"):
            session.screenshot("recording-permission")
            assert session.authorize(), "录屏授权未完成"
        started = phase(session, binary, "recording")
        time.sleep(1.2)
        session.command(binary, ["--pause-recording"])
        phase(session, binary, "paused")
        session.screenshot("paused")
        time.sleep(0.35)
        session.command(binary, ["--resume-recording"])
        phase(session, binary, "recording")
        time.sleep(1.2)
        session.command(binary, ["--stop-recording"])
        completed = phase(session, binary, "finished")
        video = session.download(completed["output"], "recording." + format)
        probe = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", str(video)], timeout=10))
        stream = next(s for s in probe["streams"] if s["codec_type"] == "video")
        assert (stream["width"], stream["height"]) == (640, 480), stream
        assert float(probe["format"]["duration"]) > 1.5
        subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(video), "-f", "null", "-"], capture_output=True, check=True, timeout=20)
        frame = subprocess.check_output(["ffmpeg", "-v", "error", "-i", str(video), "-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"], timeout=10)
        (session.root / "recorded-frame.png").write_bytes(frame)
        with Image.open(source) as desktop, Image.open(io.BytesIO(frame)) as result:
            difference = ImageChops.difference(result.convert("RGB"), desktop.convert("RGB").crop((150, 100, 790, 580)))
            mean = sum(ImageStat.Stat(difference).mean) / 3
            assert mean < 12, f"录屏内容与实际桌面不匹配: {mean}"
        return {"format": format, "tool_parameters_adjusted": True, "tool_did_not_start_early": True, "pause_resume_stop": True,
                "capture": started["detail"], "size": [640, 480], "duration": probe["format"]["duration"], "decoded": True, "desktop_mean_error": mean}
    finally:
        session.log("capture")
        session.cleanup()
