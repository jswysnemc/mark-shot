"""【X11虚拟机】【录屏回归】核对原生 X11 视频的参数、控制状态和桌面内容"""

import io
import json
import subprocess
import time

from PIL import Image, ImageChops, ImageStat
from annotations import config
from checks import visible_selection
from gnome_recording import parameters
from recording import phase, status


def run(session, binary, overrides):
    """【X11虚拟机】【录制流程】按 session、binary、overrides 录制并解码 MP4 或 GIF，返回尺寸、时长及内容误差"""
    format = overrides.pop("MARK_SHOT_TEST_FORMAT", "mp4")
    assert format in ("mp4", "gif")
    environment = {"MARK_SHOT_MEDIA_LIBRARY": "/tmp/libmarkshot-media.so", **overrides}
    session.setup(config(session) + '[recording]\nfps=12\ncountdown=0\naudio="off"\nencoder="software"\nformat=' + json.dumps(format) + '\ndirectory=' + json.dumps(session.remote + "/output") + '\n')
    source = session.screenshot("recording-source")
    pid = session.launch(binary, ["--speed"], overrides=environment)
    try:
        session.wait(lambda: any(w["pid"] == pid for w in session.windows()), "录屏选区没有显示")
        time.sleep(0.3)
        session.drag((150, 100), (790, 580))
        session.key("ctrl+shift+g" if format == "gif" else "ctrl+shift+r")
        visible_selection(session.screenshot("recording-tool"))
        parameters(session)
        assert status(session, binary)["phase"] == "idle", "工具参数操作提前开始采集"
        session.key("ret")
        started = phase(session, binary, "recording")
        assert not any("portal" in w["app"] for w in session.windows()), "原生 X11 不应显示门户授权"
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
        assert (stream["width"], stream["height"], stream["codec_name"]) == (640, 480, "gif" if format == "gif" else "h264"), stream
        assert float(probe["format"]["duration"]) > 1.5
        subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(video), "-f", "null", "-"], capture_output=True, check=True, timeout=20)
        frame = subprocess.check_output(["ffmpeg", "-v", "error", "-i", str(video), "-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"], timeout=10)
        (session.root / "recorded-frame.png").write_bytes(frame)
        with Image.open(source) as desktop, Image.open(io.BytesIO(frame)) as result:
            difference = ImageChops.difference(result.convert("RGB"), desktop.convert("RGB").crop((150, 100, 790, 580)))
            mean = sum(ImageStat.Stat(difference).mean) / 3
            assert mean < 12, f"录屏内容与真实桌面不一致: {mean}"
        return {"format": format, "capture": started["detail"], "native_capture_without_portal": True,
                "tool_parameters_adjusted": True, "pause_resume_stop": True, "size": [640, 480],
                "duration": probe["format"]["duration"], "desktop_mean_error": mean, "decoded": True}
    finally:
        session.log("capture")
        session.cleanup()
