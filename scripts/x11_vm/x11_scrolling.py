"""【X11虚拟机】【长截图回归】验证 X11 真实滚轮、持续取帧及原始纹理拼接"""

from scrolling import run as scrolling_run


def run(session, binary, overrides):
    """【X11虚拟机】【长图流程】按 session、binary、overrides 使用客体页面核对长图，返回验证结果"""
    return scrolling_run(session, binary, overrides, "/tmp/scroll-page", "", hidden_controls=False)
