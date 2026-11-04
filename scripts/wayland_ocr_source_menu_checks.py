"""【原文菜单测试】【协议校验】解析真实弹层配置，不使用父窗口配置替代独立菜单证据"""

import re


def popup_configurations(text):
    """【原文菜单测试】【弹层矩形】从 text 的 Rust 请求或事件格式返回菜单配置列表，支持负坐标"""
    pattern = r"xdg_popup@\d+\.configure(?:, \(|\()(-?\d+), (-?\d+), (\d+), (\d+)\)"
    return [tuple(map(int, values)) for values in re.findall(pattern, text)]
