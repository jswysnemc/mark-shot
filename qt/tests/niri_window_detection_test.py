#!/usr/bin/env python3
import os
import sys
import unittest
from importlib.machinery import SourceFileLoader
from importlib.util import module_from_spec, spec_from_loader
from unittest.mock import patch


PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
sys.path.insert(0, os.path.join(PROJECT_ROOT, "scripts", "lib"))

from mark_shot_window_detection.niri_dms import dms_insets
from mark_shot_window_detection.niri_geometry import clipped_rect

LOADER = SourceFileLoader("mark_shot_niri_geometry_repro", os.path.join(PROJECT_ROOT, "scripts", "mark-shot-window-detection-niri"))
SPEC = spec_from_loader(LOADER.name, LOADER)
NIRI = module_from_spec(SPEC)
LOADER.exec_module(NIRI)


class NiriWindowDetectionTest(unittest.TestCase):
    """验证 niri 窗口检测的 DMS 与裁剪计算。"""

    def test_frame_reserves_all_edges_and_uses_bar_size(self):
        """DMS frame 应按 bar 边缘和普通边缘分别保留空间。"""
        layers = [
            {"namespace": "dms:frame", "output": "DP-1"},
            {"namespace": "dms:frame-exclusion", "output": "DP-1"},
            {"namespace": "dms:bar", "output": "DP-1"},
        ]
        settings = {
            "frameEnabled": True,
            "frameThickness": 16,
            "frameBarSize": 40,
            "frameScreenPreferences": ["all"],
            "barConfigs": [{
                "enabled": True,
                "visible": True,
                "position": 0,
                "screenPreferences": ["DP-1"],
            }],
            "showDock": False,
        }

        self.assertEqual(dms_insets(layers, "DP-1", settings), {
            "top": 40.0,
            "bottom": 16.0,
            "left": 16.0,
            "right": 16.0,
        })

    def test_frame_respects_screen_preferences(self):
        """未分配给目标输出的 frame 不应改变工作区。"""
        layers = [{"namespace": "dms:frame", "output": "DP-2"}]
        settings = {
            "frameEnabled": True,
            "frameThickness": 16,
            "frameScreenPreferences": ["DP-1"],
            "showDock": False,
        }

        self.assertEqual(dms_insets(layers, "DP-2", settings), {
            "top": 0.0,
            "bottom": 0.0,
            "left": 0.0,
            "right": 0.0,
        })

    def test_candidate_rect_is_clipped_to_capture_bounds(self):
        """跨越捕获边界的窗口矩形应裁剪到当前输出。"""
        self.assertEqual(clipped_rect((90, 10, 40, 30), (0, 0, 100, 100)),
                         (90, 10, 10, 30))

    def test_reconstructed_window_includes_top_gap_after_dms_bar(self):
        """【niri测试】【窗口坐标】重建平铺窗口时，工作区顶边仍需保留 niri 的上方间隙，无参数和返回值。"""
        outputs = {"DP-1": {"logical": {"x": 0, "y": 0, "width": 1000, "height": 800}}}
        window = {"id": 1, "workspace_id": 1, "layout": {
            "pos_in_scrolling_layout": [1, 1], "tile_size": [980, 740],
            "window_size": [972, 732], "window_offset_in_tile": [4, 4],
        }}
        config = {"gaps": 8, "strut_left": 0, "strut_right": 0, "strut_top": 0, "strut_bottom": 0}
        insets = {"top": 40, "bottom": 0, "left": 0, "right": 0}
        with patch.object(NIRI, "dms_insets", return_value=insets):
            rects = NIRI.tiled_rects_for_workspace([window], outputs, [], config,
                                                  1, {"output": "DP-1", "active_window_id": 1}, set())
        self.assertEqual(rects, [(8, 48, 980, 740)])

    def test_visible_tile_rect_preserves_niri_decoration_bounds(self):
        """【niri测试】【窗口坐标】直接上报的可见矩形使用 tile 边界，而不是调试用窗口内容尺寸，无参数和返回值。"""
        outputs = {"DP-1": {"logical": {"x": 200, "y": 300, "width": 1000, "height": 800}}}
        window = {"workspace_id": 1, "layout": {
            "tile_pos_in_workspace_view": [8, 48], "tile_size": [980, 740],
            "window_size": [972, 732], "window_offset_in_tile": [4, 4],
        }}
        rect = NIRI.window_rect(window, outputs, [], {}, {1: {"output": "DP-1"}}, {})
        self.assertEqual(rect, (208, 348, 980, 740))

    def test_legacy_geometry_without_tile_size_remains_supported(self):
        """【niri测试】【窗口坐标】旧数据没有 tile 尺寸时仍使用窗口内容几何，无参数和返回值。"""
        outputs = {"DP-1": {"logical": {"x": 200, "y": 300, "width": 1000, "height": 800}}}
        window = {"workspace_id": 1, "layout": {
            "tile_pos_in_workspace_view": [8, 48],
            "window_size": [972, 732], "window_offset_in_tile": [4, 4],
        }}
        rect = NIRI.window_rect(window, outputs, [], {}, {1: {"output": "DP-1"}}, {})
        self.assertEqual(rect, (212, 352, 972, 732))

    def test_stacked_tiles_preserve_gap_and_fullscreen_starts_at_output_origin(self):
        """【niri测试】【窗口坐标】重建时保留列内间隙，全屏 tile 不增加面板或上方间隙，无参数和返回值。"""
        outputs = {"DP-1": {"logical": {"x": 0, "y": 0, "width": 1000, "height": 800}}}
        config = {"gaps": 8, "strut_left": 0, "strut_right": 0, "strut_top": 0, "strut_bottom": 0}
        insets = {"top": 40, "bottom": 0, "left": 0, "right": 0}
        windows = [{"id": row, "workspace_id": 1, "layout": {
            "pos_in_scrolling_layout": [1, row], "tile_size": [980, 300],
            "window_size": [972, 292], "window_offset_in_tile": [4, 4],
        }} for row in (1, 2)]
        with patch.object(NIRI, "dms_insets", return_value=insets):
            rects = NIRI.tiled_rects_for_workspace(windows, outputs, [], config,
                                                  1, {"output": "DP-1", "active_window_id": 1}, set())
            self.assertEqual(rects, [(8, 48, 980, 300), (8, 356, 980, 300)])
            windows[0]["layout"]["tile_size"] = [1000, 800]
            rects = NIRI.tiled_rects_for_workspace(windows[:1], outputs, [], config,
                                                  1, {"output": "DP-1", "active_window_id": 1}, set())
            self.assertEqual(rects, [(0, 0, 1000, 800)])


if __name__ == "__main__":
    unittest.main()
