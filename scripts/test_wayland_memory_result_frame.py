"""【内存对照回归】【真实结果首帧】协议屏障不得把滚动控制表面当作已经完成的结果编辑器"""

import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("memory_result_support", Path(__file__).with_name("wayland-memory-baseline.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ResultFrameTests(unittest.TestCase):
    """【内存对照回归】【协议条件】验证真实前景提交、窗口角色及物理缓冲尺寸"""

    def test_requires_a_new_full_editor_frame(self):
        """【内存对照回归】【配置屏障】新编辑表面只有配置缓冲时返回假，真实前景提交完成后返回真，无参数、无返回值"""
        text = 'get_layer_surface(zwlr_layer_surface_v1@58, wl_surface@56, wl_output@18, 3, Some("mark-shot"))\n'
        text += 'create_buffer(wl_buffer@53, 0, 3840, 2160, 15360, 0)\n'
        text += 'wl_surface@56.attach(wl_buffer@53, 0, 0)\nwl_surface@56.commit()\n'
        self.assertFalse(MODULE.result_frame_ready(text))
        text += 'wl_surface@56.attach(wl_buffer@53, 0, 0)\n'
        self.assertFalse(MODULE.result_frame_ready(text))
        self.assertTrue(MODULE.result_frame_ready(text + 'wl_surface@56.commit()\n'))

    def test_probe_and_small_controls_cannot_complete_the_editor(self):
        """【内存对照回归】【角色隔离】探测表面与小控制缓冲不得通过结果编辑屏障，无参数、无返回值"""
        probe = 'get_layer_surface(zwlr_layer_surface_v1@58, wl_surface@56, wl_output@18, 3, Some("lumine-probe"))\n'
        buffer = 'create_buffer(wl_buffer@53, 0, 3840, 2160, 15360, 0)\n'
        frames = 'wl_surface@56.attach(wl_buffer@53, 0, 0)\nwl_surface@56.commit()\n' * 2
        self.assertFalse(MODULE.result_frame_ready(probe + buffer + frames))
        parent = probe.replace('lumine-probe', 'mark-shot')
        self.assertFalse(MODULE.result_frame_ready(parent + buffer.replace('3840, 2160, 15360', '320, 200, 1280') + frames))

    def test_initial_rust_frame_requires_main_keyboard_focus_and_foreground(self):
        """【内存对照回归】【真实主表面】无参数，输出探测、配置帧与其他表面焦点不能满足首次主画面条件，无返回值"""
        probe = 'get_layer_surface(zwlr_layer_surface_v1@7, wl_surface@5, wl_output@2, 3, Some("lumine-probe"))\n'
        text = probe + 'create_buffer(wl_buffer@6, 0, 1, 1, 4, 0)\nwl_surface@5.attach(wl_buffer@6, 0, 0)\nwl_surface@5.commit()\nwl_keyboard@9.enter, (12, wl_surface@5, [])\n'
        self.assertFalse(MODULE.initial_frame_ready(text, "rust"))
        text += 'get_layer_surface(zwlr_layer_surface_v1@58, wl_surface@56, wl_output@2, 3, Some("mark-shot"))\ncreate_buffer(wl_buffer@53, 0, 3840, 2160, 15360, 0)\n'
        frames = 'wl_surface@56.attach(wl_buffer@53, 0, 0)\nwl_surface@56.commit()\n'
        self.assertFalse(MODULE.initial_frame_ready(text + frames * 3, "rust"))
        text += 'wl_keyboard@9.enter, (15, wl_surface@56, [])\n'
        self.assertFalse(MODULE.initial_frame_ready(text + frames * 2, "rust"))
        self.assertTrue(MODULE.initial_frame_ready(text + frames * 3, "rust"))

    def test_initial_qt_frame_uses_its_full_buffer_surface(self):
        """【内存对照回归】【Qt主表面】无参数，Qt日志的井号对象仅在同一4K表面附着并获得焦点后通过，无返回值"""
        text = 'wl_shm_pool#21.create_buffer(new id wl_buffer#23, 0, 3840, 2160, 15360, 1)\n'
        text += 'wl_surface#32.attach(wl_buffer#23, 0, 0)\nwl_surface#32.commit()\n' * 3
        self.assertFalse(MODULE.initial_frame_ready(text, "qt"))
        self.assertTrue(MODULE.initial_frame_ready(text + 'wl_keyboard#24.enter(12, wl_surface#32, array[0])\n', "qt"))


if __name__ == "__main__":
    unittest.main()
