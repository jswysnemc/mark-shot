import json
import os


def number(value, default=0.0):
    """将配置值转换为浮点数。
    参数 value: 待转换的配置值; default: 转换失败时使用的默认值
    返回: 浮点数配置值
    """
    if isinstance(value, bool):
        return default
    if isinstance(value, (int, float)):
        return float(value)
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def config_home():
    """读取当前用户配置目录。
    返回: XDG 配置目录路径
    """
    return os.environ.get("XDG_CONFIG_HOME") or os.path.join(os.path.expanduser("~"), ".config")


def dms_settings_path():
    """返回 DMS 设置文件路径。
    返回: settings.json 绝对路径
    """
    return os.path.join(config_home(), "DankMaterialShell", "settings.json")


def dms_settings():
    """读取 DMS 设置。
    返回: 设置字典，读取失败时返回空字典
    """
    try:
        with open(dms_settings_path(), "r", encoding="utf-8") as file:
            payload = json.load(file)
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def dms_layer_present(layers, output_name, namespace):
    """判断指定输出是否存在 DMS layer-shell 表面。
    参数 layers: niri layer 列表; output_name: 输出名称; namespace: layer namespace
    返回: 存在匹配表面时返回 True
    """
    for layer in layers:
        if not isinstance(layer, dict):
            continue
        if layer.get("namespace") != namespace:
            continue
        if output_name and layer.get("output") != output_name:
            continue
        return True
    return False


def any_dms_layer_present(layers, output_name, namespaces):
    """判断指定输出是否存在任一 DMS layer-shell 表面。
    参数 layers: niri layer 列表; output_name: 输出名称; namespaces: namespace 集合
    返回: 存在匹配表面时返回 True
    """
    return any(dms_layer_present(layers, output_name, namespace) for namespace in namespaces)


def dms_edge(position, default="top"):
    """将 DMS 位置配置转换为边缘名称。
    参数 position: 字符串或枚举整数; default: 无法识别时使用的边缘
    返回: top、bottom、left 或 right
    """
    if isinstance(position, str):
        lowered = position.strip().lower()
        if lowered in ("top", "bottom", "left", "right"):
            return lowered
        try:
            position = int(lowered)
        except ValueError:
            return default
    mapping = {0: "top", 1: "bottom", 2: "left", 3: "right"}
    return mapping.get(position, default)


def dms_screen_matches(preferences, output_name):
    """判断 DMS 显示器偏好是否包含目标输出。
    参数 preferences: 显示器名称列表; output_name: 目标输出名称
    返回: 目标输出启用该组件时返回 True
    """
    if not isinstance(preferences, list) or not preferences:
        return True
    if "all" in preferences:
        return True
    if not output_name:
        return True
    return output_name in preferences


def add_inset(insets, edge, amount):
    """向指定边缘累加保留尺寸。
    参数 insets: 四边尺寸字典; edge: 边缘名称; amount: 增量
    返回: 无返回值
    """
    if edge not in insets or amount <= 0:
        return
    insets[edge] += amount


def active_bar_edges(settings, output_name):
    """读取目标输出上的活动 DMS bar 边缘。
    参数 settings: DMS 设置; output_name: 输出名称
    返回: 活动边缘集合
    """
    edges = set()
    configs = settings.get("barConfigs")
    if not isinstance(configs, list):
        return edges
    for config in configs:
        if not isinstance(config, dict) or not config.get("enabled", True):
            continue
        if not dms_screen_matches(config.get("screenPreferences"), output_name):
            continue
        edges.add(dms_edge(config.get("position"), "top"))
    return edges


def dms_bar_insets(settings, layers, output_name):
    """计算 DMS bar 对工作区的四边占用。
    参数 settings: DMS 设置; layers: niri layer 列表; output_name: 输出名称
    返回: 四边保留尺寸字典
    """
    insets = {"top": 0.0, "bottom": 0.0, "left": 0.0, "right": 0.0}
    if not dms_layer_present(layers, output_name, "dms:bar"):
        return insets

    configs = settings.get("barConfigs")
    if not isinstance(configs, list):
        configs = []
    for config in configs:
        if not isinstance(config, dict):
            continue
        if not config.get("enabled", True) or not config.get("visible", True):
            continue
        if config.get("autoHide", False):
            continue
        if not dms_screen_matches(config.get("screenPreferences"), output_name):
            continue

        inner_padding = number(config.get("innerPadding"), 4.0)
        widget_thickness = max(20.0, 26.0 + inner_padding * 0.6)
        bar_thickness = max(widget_thickness + inner_padding + 4.0,
                            48.0 - 4.0 - (8.0 - inner_padding))
        spacing = number(config.get("spacing"), 4.0)
        bottom_gap = number(config.get("bottomGap"), 0.0)
        add_inset(insets,
                  dms_edge(config.get("position"), "top"),
                  bar_thickness + spacing + bottom_gap)
    return insets


def dms_dock_insets(settings, layers, output_name, bar_insets):
    """计算 DMS dock 对工作区的四边占用。
    参数 settings: DMS 设置; layers: niri layer 列表; output_name: 输出名称;
        bar_insets: bar 四边占用
    返回: 四边保留尺寸字典
    """
    insets = {"top": 0.0, "bottom": 0.0, "left": 0.0, "right": 0.0}
    if not any_dms_layer_present(layers, output_name, {"dms:dock", "dms:dock-exclusion"}):
        return insets
    if not settings.get("showDock", True):
        return insets
    if settings.get("dockAutoHide", False) or settings.get("dockSmartAutoHide", False):
        return insets

    edge = dms_edge(settings.get("dockPosition", 1), "bottom")
    if bar_insets.get(edge, 0.0) > 0:
        return insets

    border = number(settings.get("dockBorderThickness"), 1.0) if settings.get("dockBorderEnabled", False) else 0.0
    spacing = number(settings.get("dockSpacing"), 4.0)
    icon_size = number(settings.get("dockIconSize"), 35.0)
    bottom_gap = number(settings.get("dockBottomGap"), 0.0)
    margin = number(settings.get("dockMargin"), 10.0)
    effective_bar_height = icon_size + spacing * 2.0 + 10.0 + border * 2.0
    add_inset(insets, edge, effective_bar_height + spacing + bottom_gap + margin)
    return insets


def dms_frame_insets(settings, layers, output_name):
    """计算 DMS 显示器框架形成的工作区占用。
    参数 settings: DMS 设置; layers: niri layer 列表; output_name: 输出名称
    返回: 四边保留尺寸字典，框架未启用时返回 None
    """
    if not settings.get("frameEnabled", False):
        return None
    if not dms_screen_matches(settings.get("frameScreenPreferences"), output_name):
        return None
    if not any_dms_layer_present(layers,
                                 output_name,
                                 {"dms:frame", "dms:frame-exclusion"}):
        return None

    thickness = max(0.0, number(settings.get("frameThickness"), 16.0))
    bar_size = max(thickness, number(settings.get("frameBarSize"), 40.0))
    bar_edges = active_bar_edges(settings, output_name)
    return {
        edge: bar_size if edge in bar_edges else thickness
        for edge in ("top", "bottom", "left", "right")
    }


def dms_insets(layers, output_name, settings=None):
    """汇总 DMS frame、bar 和 dock 对工作区的占用。
    参数 layers: niri layer 列表; output_name: 输出名称; settings: 可选 DMS 设置
    返回: 四边保留尺寸字典，没有 DMS 表面时返回 None
    """
    namespaces = {
        "dms:bar",
        "dms:dock",
        "dms:dock-exclusion",
        "dms:frame",
        "dms:frame-exclusion",
    }
    if not any_dms_layer_present(layers, output_name, namespaces):
        return None

    resolved_settings = settings if isinstance(settings, dict) else dms_settings()
    bar = dms_bar_insets(resolved_settings, layers, output_name)
    dock = dms_dock_insets(resolved_settings, layers, output_name, bar)
    frame = dms_frame_insets(resolved_settings, layers, output_name)
    if frame is not None:
        return {
            edge: max(frame[edge], dock[edge])
            for edge in ("top", "bottom", "left", "right")
        }
    return {
        edge: bar[edge] + dock[edge]
        for edge in ("top", "bottom", "left", "right")
    }
