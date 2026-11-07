#!/usr/bin/env python3
"""【主题通知回归】【跨进程保存】私有niri验证外部原子配置更新，不用输入事件推动刷新"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops
from wayland_ocr_popup_tree import active_tree
from wayland_window_input_session import frame as ocr_frame

SPEC = importlib.util.spec_from_file_location("theme_notification_support", Path(__file__).with_name("wayland-theme.py"))
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)
session = SUPPORT.session


def save(config, mode):
    """【主题通知回归】【另一进程】使用独立进程原子保存config的mode，返回子进程状态，界面不接受辅助输入"""
    code = """import os,sys,tempfile
from pathlib import Path
p=Path(sys.argv[1]); text=p.read_text(); previous='dark' if sys.argv[2]=='light' else 'light'
assert 'theme="'+previous+'"' in text
text=text.replace('theme="'+previous+'"','theme="'+sys.argv[2]+'"')
with tempfile.NamedTemporaryFile(dir=p.parent,delete=False) as f:
 f.write(text.encode()); f.flush(); os.fsync(f.fileno()); name=f.name
os.replace(name,p)
"""
    subprocess.run(["/usr/bin/python", "-c", code, str(config), mode], check=True, timeout=3)


def wait_changed(process, root, name, expected, minimum):
    """【主题通知回归】【事件呈现】无额外输入，等待process画面相对expected变暗minimum像素，root/name保存证据，返回图像"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        assert process.poll() is None, "live window exited during configuration update"
        current = SUPPORT.screenshot(root, name)
        count = sum(sum(a) - sum(b) > 300 for a, b in zip(expected.get_flattened_data(), current.get_flattened_data()))
        if count >= minimum:
            return current
        time.sleep(0.05)
    raise AssertionError(f"{name}: external theme produced only {count} darkened pixels")


def wait_equal(process, root, name, expected):
    """【主题通知回归】【完整恢复】无额外输入，等待process完整帧与expected完全相同，root/name保存证据，返回图像"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        assert process.poll() is None
        current = SUPPORT.screenshot(root, name)
        difference = ImageChops.difference(expected, current).getbbox()
        if difference is None:
            return current
        time.sleep(0.05)
    raise AssertionError(f"{name}: theme roundtrip did not restore full frame: {difference}")


def roundtrip(process, root, name, config, minimum):
    """【主题通知回归】【往返更新】另一进程两次替换config，验证process完整帧与存活状态，返回变化数量和空闲证据"""
    time.sleep(0.3)
    before = SUPPORT.screenshot(root, name + "-light")
    save(config, "dark")
    dark = wait_changed(process, root, name + "-dark", before, minimum)
    save(config, "light")
    wait_equal(process, root, name + "-restored", before)
    return {"changed_pixels": sum(a != b for a, b in zip(before.get_flattened_data(), dark.get_flattened_data())),
            "exact_roundtrip": True}


def editor(binary, environment, root, config, source):
    """【主题通知回归】【截图界面】用binary打开source和属性，验证外部刷新不依赖鼠标事件，返回完整恢复证据"""
    with SUPPORT.launch(binary, environment, root, "editor", ["--fullscreen", "--to", "", "--config", config, source]) as process:
        SUPPORT.wait_window(process, "mark-shot")
        session.control("move", "--logical", 30, 30)
        session.key("t")
        result = roundtrip(process, root, "editor-properties", config, 7000)
        session.key("s")
        result["idle"] = SUPPORT.idle(process, root, "editor")
        # 1. 【主题通知回归】【其他配置隔离】另一文件改变主题不会触发当前窗口重绘，比较整个静止帧
        unrelated = root / "unrelated.toml"
        unrelated.write_text('[ui]\ntheme="light"\n')
        previous = SUPPORT.screenshot(root, "unrelated-before")
        save(unrelated, "dark")
        time.sleep(0.2)
        SUPPORT.equal(previous, SUPPORT.screenshot(root, "unrelated-after"), "unrelated configuration")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
        result["unrelated_path_ignored"] = True
        return result


def ocr(binary, environment, root, config):
    """【主题通知回归】【结果父子菜单】向binary发送固定识别结果，外部更新保持整棵菜单原生身份，返回恢复证据"""
    with SUPPORT.launch(binary, environment, root, "ocr", ["--ocr-result", "--config", config], ocr_frame()) as process:
        window = session.size_window(SUPPORT.wait_window(process, "mark-shot-ocr-result"), 760, 460)
        result = roundtrip(process, root, "ocr-main", config, 10000)
        session.click(window, 675, 19)
        session.key("Down")
        session.key("Right")
        time.sleep(0.2)
        nodes = active_tree((root / "ocr.log").read_text())
        assert len(nodes) == 2 and all(node["rect"] is not None for node in nodes)
        result["menus"] = roundtrip(process, root, "ocr-menus", config, 10000)
        after = active_tree((root / "ocr.log").read_text())
        assert [node["surface"] for node in nodes] == [node["surface"] for node in after]
        session.key("Escape")
        session.key("Escape")
        result["idle"] = SUPPORT.idle(process, root, "ocr")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
        result["menu_grabs_preserved"] = True
        return result


def history(binary, environment, root, config):
    """【主题通知回归】【历史窗口】用binary打开独立历史，验证外部主题、完整恢复和空闲等待，返回证据"""
    with SUPPORT.launch(binary, environment, root, "history", ["--history", "--config", config]) as process:
        SUPPORT.wait_window(process, "mark-shot-history")
        result = roundtrip(process, root, "history-window", config, 10000)
        result["idle"] = SUPPORT.idle(process, root, "history")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
        return result


def pin(binary, environment, root, config, source):
    """【主题通知回归】【活动贴图菜单】用binary贴图source，外部更新保持菜单与原始剪贴板像素，返回证据"""
    with SUPPORT.launch(binary, environment, root, "pin", ["--pin", "--position", "80,160", "--config", config, source]) as process:
        session.wait_single_thread(process.pid)
        time.sleep(0.5)
        session.control("click", "--logical", 130, 200, "--button", "right")
        result = roundtrip(process, root, "pin-menu", config, 10000)
        session.key("Escape")
        session.key("ctrl+c")
        with Image.open(source) as expected:
            SUPPORT.equal(SUPPORT.clipboard(environment), expected.convert("RGBA"), "pin original pixels")
        result["idle"] = SUPPORT.idle(process, root, "pin")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
        result["original_pixels_preserved"] = True
        return result


def run(binary, root):
    """【主题通知回归】【私有执行】在私有会话依次验证binary全部存活窗口类别，root保存原始证据，返回结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "explicit/config.toml"
    config.parent.mkdir()
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n'
                      '[history]\nenabled=false\n[ui]\ntheme="light"\n')
    source = root / "source.png"
    Image.new("RGBA", (900, 560), (205, 205, 205, 255)).save(source)
    previous = dict(os.environ)
    os.environ.update(environment)
    try:
        result = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                  "editor": editor(binary, environment, root, config, source),
                  "ocr": ocr(binary, environment, root, config),
                  "history": history(binary, environment, root, config),
                  "pin": pin(binary, environment, root, config, source),
                  "external_atomic_writer": True, "explicit_config_path": True}
    finally:
        os.environ.clear()
        os.environ.update(previous)
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【主题通知回归】【命令入口】解析binary和root，执行实际回归并输出结果，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
