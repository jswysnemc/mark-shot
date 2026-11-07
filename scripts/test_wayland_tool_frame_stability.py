"""【工具画面验证测试】【过渡与终态】覆盖首次正文、正文外变化和持续变化超时"""

from pathlib import Path
import tempfile
import unittest

from PIL import Image

from wayland_tool_frame_stability import capture_stable, changed_pixels


class FrameStabilityTests(unittest.TestCase):
    """【工具画面验证测试】【实际像素】使用明确RGB像素与受控时间验证最终画面，而非模仿绘制状态"""

    def run_sequence(self, images, timeout=2):
        """【工具画面验证测试】【序列输入】按images提供过渡画面，timeout限定秒数，返回采样结果及最终RGB图"""
        elapsed = [0.0]

        def capture():
            """【工具画面验证测试】【采集】每次前进0.1秒并返回对应实际图像，无参数"""
            elapsed[0] += 0.1
            return images[min(round(elapsed[0] * 10) - 1, len(images) - 1)]

        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "drawn.png"
            result = capture_stable(capture, Image.new("RGB", (30, 20)), (0, 0, 20, 20), path,
                                    timeout=timeout, quiet_seconds=0.3,
                                    clock=lambda: elapsed[0], pause=lambda _: None)
            with Image.open(path) as final:
                return result, final.copy()

    def test_first_annotation_and_late_outside_body_change_do_not_finish_early(self):
        """【工具画面验证测试】【延迟呈现】无参数，正文已足够但完整画面后来变化时必须交付最终帧，无返回值"""
        first = Image.new("RGB", (30, 20), (12, 0, 0))
        final = first.copy()
        final.putpixel((29, 19), (0, 0, 200))
        result, image = self.run_sequence([first, first, final])
        self.assertEqual(image.tobytes(), final.tobytes())
        self.assertEqual(len(result["transitions"]), 2)
        self.assertGreaterEqual(result["settle_seconds"], 0.6)

    def test_unchanged_source_and_continuous_motion_fail(self):
        """【工具画面验证测试】【无效终态】无参数，原图不变或持续绘制均必须超时失败，无返回值"""
        for images in [[Image.new("RGB", (30, 20))],
                       [Image.new("RGB", (30, 20), (value, 0, 0)) for value in range(1, 20)]]:
            with self.assertRaises(AssertionError):
                self.run_sequence(images, timeout=0.9)

    def test_count_includes_single_changed_channel(self):
        """【工具画面验证测试】【精确通道】无参数，任一RGB通道改变都计入一个像素，无返回值"""
        before = Image.new("RGB", (10, 10))
        after = before.copy()
        for index, color in enumerate([(1, 0, 0), (0, 1, 0), (0, 0, 1), (1, 1, 1)]):
            after.putpixel((index, 0), color)
        self.assertEqual(changed_pixels(before, after), 4)


if __name__ == "__main__":
    unittest.main()
