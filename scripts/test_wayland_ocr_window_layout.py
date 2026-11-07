"""【结果回归】【命中边界】验证单行标题坐标对页面、窄窗口和反馈尺寸变化的处理"""

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
import wayland_ocr_window_layout as layout


class OcrWindowLayoutTests(unittest.TestCase):
    """【结果回归】【坐标断言】固定本地逻辑矩形，不连接实际桌面"""

    def test_copy_point_keeps_the_qt_right_edge_on_each_view(self):
        """【结果回归】【原文复制】非文字页复制入口右移 32 像素，点击点不依赖反馈文案宽度，无参数、无返回值"""
        window = {"x": 0, "y": 0, "width": 760}
        self.assertEqual(layout.source_point(window), (614, 19))
        self.assertEqual(layout.source_point(window, "Table"), (646, 19))
        self.assertEqual(layout.source_point(window, "Image"), (646, 19))
        self.assertEqual(layout.table_point(window), (740, 54))

    def test_narrow_tabs_use_the_visible_front_of_each_tab(self):
        """【结果回归】【窄屏页签】图标隐藏后页签左移，缺少表格时原图紧接文字页，不访问裁切部分，无参数、无返回值"""
        window = {"x": 20, "y": 30, "width": 380}
        self.assertEqual(layout.tab_point(window, "Text"), (38, 49))
        self.assertEqual(layout.tab_point(window, "Image"), (129, 49))
        self.assertEqual(layout.tab_point(window, "Image", has_table=False), (81, 49))

    def test_niri_window_records_use_the_same_logical_geometry(self):
        """【结果回归】【窗口记录】原有模型回归使用 niri 浮动记录，控件位置仍按相同单行几何计算，无参数、无返回值"""
        window = {"floating_pos": [20, 30], "size": [760, 460]}
        self.assertEqual(layout.source_point(window), (634, 49))
        self.assertEqual(layout.table_point(window), (760, 84))
        self.assertEqual(layout.tab_point(window, "Image"), (153, 49))

    def test_chinese_tabs_use_the_original_equal_widths(self):
        """【结果回归】【中文页签】三种页签均为原版 42 像素，窄屏与缺少表格时仍按相同规则定位，无参数、无返回值"""
        window = {"x": 20, "y": 30, "width": 760}
        self.assertEqual(layout.tab_point(window, "Text", language="zh"), (62, 49))
        self.assertEqual(layout.tab_point(window, "Table", language="zh"), (104, 49))
        self.assertEqual(layout.tab_point(window, "Image", language="zh"), (146, 49))
        window["width"] = 380
        self.assertEqual(layout.tab_point(window, "Image", has_table=False, language="zh"), (80, 49))

    def test_table_cell_points_come_from_the_actual_qt_grid(self):
        """【结果回归】【表格几何】原版单元格从表面第 106 像素开始，固定行高不沿用旧的表头点击点，无参数、无返回值"""
        window = {"x": 20, "y": 30, "width": 760, "height": 460}
        self.assertEqual(layout.table_cell_point(window), (79.5, 149.5))
        self.assertEqual(layout.table_cell_point(window, 1, 1), (123.5, 177.5))
        with self.assertRaises(ValueError):
            layout.table_reference(555)

    def test_model_regression_uses_the_independent_table_menu(self):
        """【结果回归】【模型入口】已有模型回归通过表格分区按钮及真实菜单按键选择 TSV，不再使用标题栏复制，无参数、无返回值"""
        spec = importlib.util.spec_from_file_location("ocr_result_model_check", Path(__file__).with_name("wayland-ocr-result.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        window = {"floating_pos": [20, 30], "size": [760, 460]}
        with patch.object(module.session, "click") as click, patch.object(module.session, "key") as key, patch.object(module.session, "wait_clipboard") as clipboard:
            module.copy_table({}, window, "table text")
        click.assert_called_once_with(window, 740, 54)
        self.assertEqual([call.args for call in key.call_args_list], [("Home",), ("Return",)])
        clipboard.assert_called_once_with({}, "table text")
