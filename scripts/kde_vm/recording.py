"""【KDE虚拟机】【录屏验证】通过工具入口、KDE 授权与控制命令产生可独立解码的视频"""

import json
import io
import subprocess
import time

from PIL import Image

from annotations import config
from checks import visible_selection


def status(session, binary):
    """【KDE虚拟机】【录制状态】按 session、binary 查询当前独立配置的后台状态并返回 JSON"""
    return json.loads(session.command(binary, ["--recording-status"]))


def phase(session, binary, wanted):
    """【KDE虚拟机】【录制等待】按 wanted 等待 session、binary 状态变化，失败报告实际状态，返回状态快照"""
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        value = status(session, binary)
        if value["phase"] == wanted:
            return value
        assert value["phase"] not in ("failed", "cancelled"), value
        time.sleep(0.15)
    raise AssertionError(f"录制未进入 {wanted}: {value}")


def run(session, binary, overrides):
    """【KDE虚拟机】【录制流程】按 session、binary、overrides 调整参数、授权、暂停恢复停止，返回媒体结果"""
    format = overrides.get("MARK_SHOT_TEST_FORMAT", "mp4")
    assert format in ("mp4", "gif"), format
    session.setup(config(session) + '[recording]\nfps=12\ncountdown=0\naudio="off"\nencoder="software"\nformat=' + json.dumps(format) + '\ndirectory=' + json.dumps(session.remote + "/output") + '\n')
    environment = {"MARK_SHOT_MEDIA_LIBRARY": "/usr/lib/mark-shot/media/libmarkshot-media.so", **overrides}
    pid = session.launch(binary, ["--speed"], overrides=environment)
    try:
        time.sleep(0.8)
        session.screenshot("initial")
        session.drag((150, 100), (790, 580))
        session.key("ctrl+shift+g" if format == "gif" else "ctrl+shift+r")
        visible_selection(session.screenshot("recording-tool"))
        # 1. 【KDE虚拟机】【本次参数】帧率向上滚动一次再向下恢复，参数操作不能启动采集或清除选区
        session.move(280, 661)
        session.scroll(down=False)
        session.screenshot("fps-increased")
        session.scroll(down=True)
        visible_selection(session.screenshot("fps-restored"))
        assert status(session, binary)["phase"] == "idle"
        session.key("ret")
        portal = session.wait(lambda: next((w for w in session.windows() if "portal.desktop.kde" in w["app"]), None), "KDE 没有出现录制授权")
        session.screenshot("portal")
        # 2. 【KDE虚拟机】【来源授权】不保存永久授权，选择本次唯一显示器卡片
        session.click(portal["x"] + 25, portal["y"] + portal["height"] - 28)
        session.click(portal["x"] + portal["width"] / 2, portal["y"] + 280)
        started = phase(session, binary, "recording")
        session.screenshot("recording")
        time.sleep(1.2)
        session.command(binary, ["--pause-recording"])
        paused = phase(session, binary, "paused")
        time.sleep(0.4)
        session.command(binary, ["--resume-recording"])
        phase(session, binary, "recording")
        time.sleep(1.2)
        session.command(binary, ["--stop-recording"])
        completed = phase(session, binary, "finished")
        video = session.download(completed["output"], "recording." + format)
        probe = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", str(video)], timeout=10))
        stream = next(value for value in probe["streams"] if value["codec_type"] == "video")
        assert (stream["width"], stream["height"], stream["codec_name"]) == (640, 480, "gif" if format == "gif" else "h264"), stream
        assert float(probe["format"]["duration"]) > 1.5, probe
        subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(video), "-f", "null", "-"], check=True, timeout=20, capture_output=True)
        frame = subprocess.check_output(["ffmpeg", "-v", "error", "-i", str(video), "-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"], timeout=10)
        (session.root / "recorded-frame.png").write_bytes(frame)
        with Image.open(io.BytesIO(frame)) as image:
            bright = sum(1 for pixel in image.convert("RGB").get_flattened_data() if min(pixel) > 200)
            assert bright > 100000, f"视频没有捕获实际桌面: {bright}"
        return {"tool_parameters": True, "format": format, "capture": started["detail"], "pause_resume": paused["phase"] == "paused",
                "saved_video_size": [stream["width"], stream["height"]], "duration": probe["format"]["duration"], "decoded": True}
    finally:
        session.stop(pid)
        session.log("capture")
        log = session.guest("path=Path(values)/'state/mark-shot/mark-shot.log';print(path.read_text() if path.exists() else '')", session.remote)
        (session.root / "worker.log").write_text(log)
        session.cleanup()
