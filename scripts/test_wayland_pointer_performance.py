"""【交互性能测试】【日志解析】校验两种原生协议时间格式及主缓冲筛选"""

import importlib.util
from pathlib import Path
import unittest


SPEC = importlib.util.spec_from_file_location("pointer_performance", Path(__file__).with_name("wayland-pointer-performance.py"))
PERFORMANCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PERFORMANCE)


class SubmissionsTest(unittest.TestCase):
    """【交互性能测试】【协议样本】使用固定日志排除光标提交和格式变化造成的误计数"""

    def test_resize_separates_old_buffers_from_measured_output(self):
        """【交互性能测试】【尺寸切换】无参数，旧窗口缓冲仍检查释放安全，四千分辨率提交只计算自身两个缓冲，无返回值"""
        text = '\n'.join([
            'create_buffer(wl_buffer@3, 0, 1920, 1080, 7680, 0)',
            'wl_surface@2.attach(wl_buffer@3, 0, 0)',
            'wl_surface@2.commit()',
            'wl_buffer@3.release()',
            'create_buffer(wl_buffer@4, 0, 3840, 2160, 15360, 0)',
            'create_buffer(wl_buffer@5, 0, 3840, 2160, 15360, 0)',
            'wl_surface@2.attach(wl_buffer@4, 0, 0)',
            'wl_surface@2.commit()',
            'wl_surface@2.attach(wl_buffer@5, 0, 0)',
            'wl_surface@2.commit()',
        ])
        self.assertTrue(PERFORMANCE.inspect(text)['safe'])
        self.assertEqual(PERFORMANCE.inspect(text)['maximum_buffers_per_surface'], 3)
        self.assertEqual(PERFORMANCE.inspect(text, minimum_pixels=3840 * 2160)['maximum_buffers_per_surface'], 2)

    def test_rust_timestamps_exclude_cursor_buffers(self):
        """【交互性能测试】【毫秒时间】无参数，验证旧式日志只计算完整窗口缓冲，无返回值"""
        text = "create_buffer(wl_buffer@3, 0, 3840, 2160, 15360, 0)\ncreate_buffer(wl_buffer@4, 0, 24, 24, 96, 0)"
        segment = "[ 1234.5] -> wl_surface@2.attach(wl_buffer@3, 0, 0)\n[ 1235.5] -> wl_surface@5.attach(wl_buffer@4, 0, 0)"
        self.assertEqual(PERFORMANCE.submissions(text, segment), [1234.5])

    def test_desktop_dimensions_select_the_actual_main_buffer(self):
        """【交互性能测试】【桌面尺寸】无参数，按指定尺寸识别主缓冲，排除不同尺寸表面，无返回值"""
        text = 'create_buffer(wl_buffer@3, 0, 2560, 1600, 10240, 0)'
        segment = '[ 1234.5] -> wl_surface@2.attach(wl_buffer@3, 0, 0)'
        self.assertEqual(PERFORMANCE.submissions(text, segment), [])
        self.assertEqual(PERFORMANCE.submissions(text, segment, (2560, 1600)), [1234.5])

    def test_qt_wall_clock_timestamps_cross_minute_boundary(self):
        """【交互性能测试】【时钟时间】无参数，验证新式日志的对象编号与跨分钟间隔，无返回值"""
        text = "create_buffer(new id wl_buffer#3, 0, 3840, 2160, 15360, 0)"
        segment = "[12:59:59.990] -> wl_surface#2.attach(wl_buffer#3, 0, 0)\n[13:00:00.010] -> wl_surface#2.attach(wl_buffer#3, 0, 0)"
        stamps = PERFORMANCE.submissions(text, segment)
        self.assertEqual(len(stamps), 2)
        self.assertAlmostEqual(stamps[1] - stamps[0], 20)

    def test_image_window_requires_actual_frame_and_focus(self):
        """【图片窗口测试】【首帧识别】真实角色映射、内容附着及焦点同时就绪才通过，无参数、无返回值"""
        roles = '\n'.join([
            'xdg_wm_base@9.get_xdg_surface(xdg_surface@26, wl_surface@24)',
            'xdg_surface@26.get_toplevel(xdg_toplevel@27)',
            'xdg_toplevel@27.set_app_id(Some("mark-shot-editor"))',
            'wl_surface@24.attach(wl_buffer@29, 0, 0)',
            'wl_surface@24.commit()',
            'wl_keyboard@19.enter, (7, wl_surface@24, [])',
        ])
        self.assertFalse(PERFORMANCE.BASELINE.image_editor_ready(roles))
        complete = roles + '\nwl_surface@24.attach(wl_buffer@30, 0, 0)\nwl_surface@24.commit()'
        self.assertTrue(PERFORMANCE.BASELINE.image_editor_ready(complete))
        self.assertFalse(PERFORMANCE.BASELINE.image_editor_ready(complete.replace('wl_keyboard@19.enter', 'wl_keyboard@19.leave')))


if __name__ == "__main__":
    unittest.main()
