"""使用测试环境独立安装的 FFprobe/FFmpeg 核验媒体时长、尺寸、音轨与完整解码。"""

import argparse
import json
from pathlib import Path
import subprocess


def verify(directory: Path, media: Path, duration: float, audio: bool) -> dict:
    """按 directory 的媒体工具检查 media，返回探测结果；不一致时抛出异常。"""
    suffix = ".exe" if (directory / "ffprobe.exe").exists() else ""
    report = json.loads(subprocess.check_output([
        str(directory / f"ffprobe{suffix}"), "-v", "error", "-show_streams",
        "-show_format", "-of", "json", str(media),
    ], timeout=30))
    streams = report["streams"]
    video = next(stream for stream in streams if stream["codec_type"] == "video")
    assert video["codec_name"] == "h264", video
    assert video["width"] == 320 and video["height"] == 240, video
    assert abs(float(report["format"]["duration"]) - duration) < 0.3, report
    sound = [stream for stream in streams if stream["codec_type"] == "audio"]
    assert bool(sound) == audio, streams
    if audio:
        assert sound[0]["codec_name"] == "aac", sound
        assert abs(float(sound[0]["duration"]) - duration) < 0.3, sound
    subprocess.run([
        str(directory / f"ffmpeg{suffix}"), "-v", "error", "-xerror", "-i", str(media),
        "-enc_time_base", "demux", "-fps_mode", "passthrough", "-f", "null", "-",
    ], check=True, timeout=60)
    return report


def main() -> None:
    """无参数，读取命令行并把验证报告写入指定文件。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("tools", type=Path)
    parser.add_argument("media", type=Path)
    parser.add_argument("--duration", type=float, default=2.0)
    parser.add_argument("--audio", action="store_true")
    parser.add_argument("--fixture-colors", action="store_true")
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    report = verify(args.tools, args.media, args.duration, args.audio)
    if args.fixture_colors:
        verify_fixture_colors(args.tools, args.media)
    args.report.write_text(json.dumps(report, indent=2), encoding="utf-8")


def verify_fixture_colors(directory: Path, media: Path) -> None:
    """按 directory 解码 media 首帧，验证测试色块的上下方向和红蓝通道。"""
    suffix = ".exe" if (directory / "ffmpeg.exe").exists() else ""
    pixels = subprocess.check_output([
        str(directory / f"ffmpeg{suffix}"), "-v", "error", "-i", str(media),
        "-frames:v", "1", "-pix_fmt", "rgb24", "-f", "rawvideo", "-",
    ], timeout=30)
    assert len(pixels) == 320 * 240 * 3, len(pixels)
    for x, y, expected in [(80, 60, (0, 0, 255)), (240, 60, (255, 0, 0)),
                           (80, 180, (0, 220, 255)), (240, 180, (255, 220, 0))]:
        offset = (y * 320 + x) * 3
        actual = tuple(pixels[offset:offset + 3])
        assert all(abs(a - b) < 40 for a, b in zip(actual, expected)), (x, y, actual, expected)


if __name__ == "__main__":
    main()
