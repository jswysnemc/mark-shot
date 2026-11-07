#!/usr/bin/env python3
"""【Windows贴图回归】【实际工作流】只在私有Wine与Xvfb验证贴图菜单、变换、主题和原图交接"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
import tomllib

from PIL import Image, ImageChops
from windows_ocr_actions import key, path_w, snapshot, stop, visible, window, xdo
from windows_pin_checks import absent, activate, copied, menu, row, save_as, size, wheel
from windows_pin_paste_checks import check as check_paste
from windows_test_runtime import run


def execute(command, environment, root, log):
    """【Windows贴图回归】【同构建执行】按command启动真实贴图并保存root证据，返回完成状态，失败保留画面并回收进程"""
    binary, probe = command
    environment = dict(environment, LANG="en_US.UTF-8", LC_ALL="en_US.UTF-8")
    config_path = root / "config.toml"
    config_path.write_text('[general]\nto=""\n[ui]\ntheme="light"\nlanguage="en"\n[notifications]\nenabled=false\n[history]\nenabled=false\n'
                           + '[save]\ndirectory=' + json.dumps(path_w(root, environment)) + '\nmonth_format="private-save"\n')
    source = Image.new("RGBA", (320, 200))
    source.putdata([(31 + x % 171, 37 + y % 163, 43 + (x + y) % 151, 255) for y in range(200) for x in range(320)])
    source_path = root / "source.png"
    source.save(source_path)
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "cases": {},
              "passed": False, "real_windows_validation": False}
    process = subprocess.Popen(["wine", binary, "--config", path_w(config_path, environment), "--nonotify", "--pin",
                                "--position", "180,180", path_w(source_path, environment)], env=environment,
                               cwd=root, stdout=log, stderr=log)
    try:
        identity = window(environment, "Pinned Mark Shot", root, process)
        report["cases"]["initial"] = size(environment, identity, (320, 200))
        copied(probe, environment, root, identity, "initial-original", source)
        # 1. 【Windows贴图回归】【菜单事务】只按下不执行，跨行释放不执行，Escape只关闭菜单
        popup = menu(environment, root, identity)
        row(environment, popup, 1, pressed=True)
        size(environment, identity, (320, 200))
        row(environment, popup, 0, pressed=False)
        assert visible(environment, "Mark Shot menu") == [popup]
        size(environment, identity, (320, 200))
        key(environment, identity, "Escape")
        absent(environment, "Mark Shot menu")
        assert process.poll() is None
        report["cases"]["menu_transaction"] = True
        report["cases"]["wheel"] = wheel(probe, environment, root, identity, source)
        # 2. 【Windows贴图回归】【键盘菜单】首项向下选择右转，Enter确认；Home加Space左转，End加Escape只取消
        menu(environment, root, identity)
        key(environment, identity, "Home")
        key(environment, identity, "Down")
        key(environment, identity, "Return")
        absent(environment, "Mark Shot menu")
        size(environment, identity, (200, 320))
        copied(probe, environment, root, identity, "keyboard-rotated", source.transpose(Image.Transpose.ROTATE_270))
        menu(environment, root, identity)
        key(environment, identity, "Home")
        key(environment, identity, "space")
        absent(environment, "Mark Shot menu")
        size(environment, identity, (320, 200))
        copied(probe, environment, root, identity, "keyboard-restored", source)
        menu(environment, root, identity)
        key(environment, identity, "End")
        key(environment, identity, "Escape")
        absent(environment, "Mark Shot menu")
        assert process.poll() is None
        report["cases"]["keyboard_menu"] = True
        # 2. 【Windows贴图回归】【无损变换】旋转修改原图方向，显示缩放不能修改交付分辨率与颜色
        activate(environment, root, identity, 1)
        size(environment, identity, (200, 320))
        rotated = source.transpose(Image.Transpose.ROTATE_270)
        copied(probe, environment, root, identity, "rotated-original", rotated)
        activate(environment, root, identity, 3)
        size(environment, identity, (236, 378))
        copied(probe, environment, root, identity, "zoomed-original", rotated)
        activate(environment, root, identity, 5)
        size(environment, identity, (200, 320))
        activate(environment, root, identity, 0)
        size(environment, identity, (320, 200))
        copied(probe, environment, root, identity, "restored-original", source)
        report["cases"]["transforms"] = {"rotate_right_left_exact": True, "zoom_keeps_original": True, "reset_size": True}
        report["cases"]["paste"] = check_paste(probe, environment, root, identity, source)
        # 3. 【Windows贴图回归】【外部主题】菜单存活期间修改配置，实际像素必须刷新并精确恢复
        popup = menu(environment, root, identity)
        xdo(environment, "mousemove", "--sync", 20, 20)
        time.sleep(0.2)
        light = snapshot(environment, root, "menu-light")
        config_path.write_text(config_path.read_text().replace('theme="light"', 'theme="dark"'))
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            dark = snapshot(environment, root, "menu-dark")
            if ImageChops.difference(light, dark).getbbox():
                break
            time.sleep(0.05)
        else:
            raise AssertionError("live pin menu did not refresh external dark theme")
        config_path.write_text(config_path.read_text().replace('theme="dark"', 'theme="light"'))
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            restored = snapshot(environment, root, "menu-restored")
            if ImageChops.difference(light, restored).getbbox() is None:
                break
            time.sleep(0.05)
        else:
            raise AssertionError("live pin menu did not restore its exact light frame")
        # 4. 【Windows贴图回归】【菜单主题键】快捷键不交付菜单动作，原生弹层身份保持不变，配置与画面同时确认
        for mode in ("dark", "light"):
            key(environment, identity, "ctrl+shift+l")
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                current = snapshot(environment, root, "menu-key-" + mode)
                different = ImageChops.difference(light, current).getbbox() is not None
                if tomllib.loads(config_path.read_text())["ui"]["theme"] == mode and different == (mode == "dark"):
                    break
                time.sleep(0.05)
            else:
                raise AssertionError(f"live pin menu swallowed theme shortcut: {mode}")
            assert visible(environment, "Mark Shot menu") == [popup], "theme shortcut replaced or dismissed the menu"
        report["cases"]["popup_theme_shortcut"] = True
        key(environment, identity, "Escape")
        absent(environment, "Mark Shot menu")
        report["cases"]["external_theme"] = True
        report["cases"]["save"] = save_as(binary, environment, root, identity, source)
        # 4. 【Windows贴图回归】【编辑交接】实际菜单启动共享编辑器，复制导出仍为贴图原件，原贴图继续存活
        activate(environment, root, identity, 9)
        editor = window(environment, "Mark Shot", root)
        copied(probe, environment, root, editor, "edited-original", source)
        absent(environment, "Mark Shot")
        assert process.poll() is None and visible(environment, "Pinned Mark Shot") == [identity]
        report["cases"]["edit_handoff"] = True
        activate(environment, root, identity, 19)
        assert process.wait(timeout=8) == 0
        absent(environment, "Pinned Mark Shot")
        absent(environment, "Mark Shot menu")
        report["cases"]["close"] = True
        report["passed"] = True
        return subprocess.CompletedProcess(command, 0)
    except Exception as error:
        report["error"] = str(error)
        snapshot(environment, root, "failure")
        raise
    finally:
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")
        stop(process)


def main():
    """【Windows贴图回归】【隔离入口】读取候选、probe和新root，输出实际运行结果，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.root.resolve(), [str(options.binary.resolve()), str(options.probe.resolve())], execute=execute)
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
