"""【原生发布】【文件命名】统一 GitHub 资产名称，保留安装包内部版本规则"""

from pathlib import Path
import re


def asset_name(path):
    """【原生发布】【名称规范】按 path 返回只含字母、数字、点号、下划线和连字符的发布文件名"""
    return re.sub(r"[^A-Za-z0-9_.-]", ".", Path(path).name)
