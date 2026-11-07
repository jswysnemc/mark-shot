"""【Windows DPI回归】【隔离配置】只修改本次私有Wine前缀，不接触用户服务或注册表"""

from pathlib import Path
import subprocess


def configure(environment, root, log, dpi):
    """【Windows DPI回归】【会话配置】校验environment前缀属于root后设置dpi并重启本次服务，返回携带期望DPI的新环境"""
    assert environment.get("MARK_SHOT_PRIVATE_WINDOWS_TEST") == "1"
    assert Path(environment["WINEPREFIX"]).resolve() == (root / "wine-prefix").resolve()
    assert int(dpi) in (96, 120, 144, 192)
    subprocess.run(["wine", "reg", "add", r"HKCU\Control Panel\Desktop", "/v", "LogPixels", "/t", "REG_DWORD", "/d", str(dpi), "/f"],
                   env=environment, cwd=root, stdout=log, stderr=log, check=True, timeout=15)
    # 1. 【Windows DPI回归】【缓存更新】只停止已校验前缀，新的窗口必须读取新的DPI
    subprocess.run(["wineserver", "-k"], env=environment, check=True, timeout=10)
    subprocess.run(["wineserver", "-w"], env=environment, check=True, timeout=10)
    return dict(environment, MARK_SHOT_TEST_EXPECTED_DPI=str(dpi))
