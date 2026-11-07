"""【工具画面验证】【连续呈现】保存过渡帧，并在完整画面连续静止后交付最终证据"""

import hashlib
import json
import time

from PIL import ImageChops


def changed_pixels(before, after):
    """【工具画面验证】【像素计数】比较before与after同尺寸RGB画面，返回任一通道改变的像素数"""
    difference = ImageChops.difference(before, after)
    channels = difference.split()
    combined = ImageChops.lighter(ImageChops.lighter(channels[0], channels[1]), channels[2])
    return before.width * before.height - combined.histogram()[0]


def capture_stable(capture, before, body, destination, timeout=10, quiet_seconds=0.8,
                   clock=time.monotonic, pause=time.sleep):
    """【工具画面验证】【稳定采样】用capture连续采集，与before/body确认标注，destination保存最终图；timeout与quiet_seconds为秒，clock/pause提供时钟，返回帧记录"""
    started, previous, unchanged_since = clock(), None, None
    frames, samples = [], 0
    while True:
        image = capture()
        samples += 1
        now = clock()
        digest = hashlib.sha256(image.tobytes()).hexdigest()
        pixels = changed_pixels(before.crop(body), image.crop(body))
        # 1. 【工具画面验证】【过渡保留】完整画面任一像素变化都重新计时，保存每次变化，正文出现不代表手势最终呈现
        if digest != previous:
            previous, unchanged_since = digest, now
            path = destination.with_name(f"{destination.stem}-transition-{len(frames):02d}.png")
            image.save(path)
            frames.append({"seconds": now - started, "sha256_rgb": digest,
                           "changed_body_pixels": pixels, "path": path.name})
        if pixels >= 200 and now - unchanged_since >= quiet_seconds:
            image.save(destination)
            result = {"changed_pixels": pixels, "body": list(body), "samples": samples,
                      "stable_seconds": now - unchanged_since, "settle_seconds": now - started,
                      "transitions": frames}
            destination.with_suffix(".frames.json").write_text(json.dumps(result, indent=2) + "\n")
            return result
        # 2. 【工具画面验证】【严格超时】未出现足够正文或画面持续变化均保留记录并失败，不以耗时或内部状态替代实际像素
        if now - started >= timeout:
            destination.with_suffix(".frames.json").write_text(json.dumps({"samples": samples, "transitions": frames}, indent=2) + "\n")
            raise AssertionError("the actual tool frame did not settle with enough annotation pixels")
        pause(0.04)
