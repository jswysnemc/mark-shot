"""【X11虚拟机】【识别引导】使用支持置顶的原生菜单验证模型缺失与取消"""

from gnome_ocr import run as ocr_run


def run(session, binary, overrides):
    """【X11虚拟机】【取消流程】按 session、binary、overrides 打开模型引导，返回无下载及来源保留结果"""
    return ocr_run(session, binary, overrides, menu_steps=8)
