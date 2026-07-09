#!/usr/bin/env python3
import os
import sys
import unittest


PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
sys.path.insert(0, os.path.join(PROJECT_ROOT, "scripts", "lib"))

from mark_shot_window_detection.niri_dms import dms_insets
from mark_shot_window_detection.niri_geometry import clipped_rect


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


if __name__ == "__main__":
    unittest.main()
