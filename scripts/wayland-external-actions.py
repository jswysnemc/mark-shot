#!/usr/bin/env python3
"""【外部动作回归】【真实交付】私有 Wayland 中验证列表、左右键、原图导出、参数和进程关闭，不操作用户桌面"""

import argparse
import hashlib
import json
from pathlib import Path
import time

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import click, pointer_context
import importlib.util


PROJECT = Path(__file__).resolve().parents[1]
FIXTURES = PROJECT / "tests/fixtures/qt-export-image"
SPEC = importlib.util.spec_from_file_location("external_export_support", PROJECT / "scripts/wayland-export-image.py")
EXPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXPORT)


def wait_result(path):
    """【外部动作回归】【结果等待】限时读取 path 的完整子进程 JSON，返回记录；缺失或未完成时不报告成功"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        try:
            return json.loads(path.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(0.04)
    raise AssertionError(f"external receiver did not finish: {path}")


def frame(root, name):
    """【外部动作回归】【原始画面】把本次唯一输出保存到 root/name，返回 RGB 图像，不从应用日志推测范围"""
    path = root / f"{name}.png"
    time.sleep(0.2)
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def changed_panel(before, after, minimum, expected_height):
    """【外部动作回归】【面板范围】比较 before、after 的实际像素，验证 minimum 宽度和 expected_height，返回新增面板范围"""
    difference = ImageChops.difference(before, after)
    bounds = difference.getbbox()
    if bounds is None:
        raise AssertionError("enabled external toolbar action did not display a panel")
    width, height = bounds[2] - bounds[0], bounds[3] - bounds[1]
    if not minimum - 2 <= width <= 382 or abs(height - expected_height) > 2:
        raise AssertionError(f"original external panel geometry differs: {bounds}, expected height={expected_height}")
    return bounds


def quote(value):
    """【外部动作回归】【配置参数】为 value 返回 shell 单参数引用，只用于本次受控扩展配置"""
    return "'" + str(value).replace("'", "'\"'\"'") + "'"


def hovered_row(root, name, bounds):
    """【外部动作回归】【实际命中】验证 root/name 中 bounds 首行真实悬停颜色；输入未到达列表时直接失败，不能把未命中当作右键隔离"""
    image = frame(root, name)
    pixel = image.getpixel((bounds[0] + 20, bounds[1] + 38))
    if pixel[1] - pixel[0] < 8 or pixel[2] - pixel[0] < 8:
        raise AssertionError(f"right pointer did not reach the intended row: {pixel}, bounds={bounds}")


def receiver(root):
    """【外部动作回归】【接收脚本】在 root 创建受控进程接收端，记录实际参数、目录和 PNG；不连接外部服务"""
    script = root / "receiver.py"
    script.write_text("""import json, os, pathlib, shutil, sys
output = pathlib.Path(sys.argv[1])
args = sys.argv[2:]
record = {'args': args, 'cwd': os.getcwd(), 'ppid': os.getppid()}
if args and pathlib.Path(args[0]).is_file():
    image = pathlib.Path(args[0])
    shutil.copyfile(image, output.with_suffix('.png'))
    record['image'] = str(image)
    record['permissions'] = image.stat().st_mode & 0o777
output.write_text(json.dumps(record))
""")
    return script


def start(binary, environment, root, name):
    """【外部动作回归】【窗口启动】在 environment 用 binary 打开固定图片，root/name 保存日志，返回就绪编辑器和日志句柄"""
    process, log = EXPORT.launch(binary, environment, root, name, "")
    try:
        EXPORT.ready(process, environment)
        time.sleep(0.4)
        return process, log
    except BaseException:
        EXPORT.close(process, log)
        raise


def consume_image(path, expected):
    """【外部动作回归】【完整图片】核对 path 接收端的私有图片及 expected 完整 RGBA，返回记录并删除已交付临时图片"""
    record = wait_result(path)
    if record.get("permissions") != 0o600:
        raise AssertionError(f"external image was not private: {record}")
    EXPORT.equal(path.with_suffix(".png").read_bytes(), expected, "external framed PNG")
    Path(record["image"]).unlink()
    return record


def run(binary, root):
    """【外部动作回归】【完整场景】用 binary 在私有会话执行列表和进程交付，root 保存同一构建的全部证据，返回报告"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
                       XDG_DATA_DIRS=str(root / "system-data"), XDG_STATE_HOME=str(root / "state"),
                       TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english")
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[keys]\nopen_with="F6"\nextensions="F7"\n' + EXPORT.FRAME)
    Path(environment["TMPDIR"]).mkdir()
    apps = root / "data/applications"
    apps.mkdir(parents=True)
    commands_path = config.parent / "extensions.json"
    script = receiver(root)
    expected = (FIXTURES / "case-17-expected.png").read_bytes()
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": session.backend().__name__}
    commands = [{"name": f"Command {index:02}", "command": f"/usr/bin/python3 {quote(script)} {quote(root / ('command-' + str(index) + '.json'))} {{image}} {{slurp}}",
                 "cwd": str(root), "closeOnStart": False} for index in range(24)]
    commands_path.write_text(json.dumps({"commands": commands}))
    with session.input_context(), pointer_context():
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        process, log = start(binary, environment, root, "extension")
        try:
            before = frame(root, "extension-before")
            session.key("F7")
            bounds = changed_panel(before, frame(root, "extension-open"), 300, 397)
            click((bounds[0] + 25, bounds[1] + 49), "right")
            hovered_row(root, "extension-right", bounds)
            if list(root.glob("command-*.json")) or process.poll() is not None:
                raise AssertionError("right click activated extension or closed editor")
            session.key("Escape")
            after = frame(root, "extension-escaped")
            if ImageChops.difference(before, after).getbbox() is not None:
                raise AssertionError("Escape did not restore unchanged screenshot canvas")
            session.key("F7")
            session.key("End")
            session.key("Return")
            record = consume_image(root / "command-23.json", expected)
            if process.poll() is not None or record["cwd"] != str(root) or len(record["args"]) != 2:
                raise AssertionError(f"extension close flag, working directory or arguments differ: {record}")
            if "x" not in record["args"][1] or "{" in record["args"][1]:
                raise AssertionError("selection geometry was not substituted")
            report["extension"] = {"panel_bounds": bounds, "receiver": record}
            # 1. 【外部动作回归】【区域独立】没有图片依赖的扩展不生成 PNG，不隐式追加图片路径
            geometry = root / "geometry.json"
            commands_path.write_text(json.dumps({"name": "Geometry", "command": f"/usr/bin/python3 {quote(script)} {quote(geometry)} {{slurp}}", "closeOnStart": False}))
            session.key("F7")
            session.key("Home")
            session.key("Return")
            record = wait_result(geometry)
            if len(record["args"]) != 1 or "image" in record or list(Path(environment["TMPDIR"]).glob("*.png")):
                raise AssertionError(f"geometry-only extension saved an image: {record}")
            report["geometry_only"] = record
            time.sleep(0.4)
            first = session.resources(process.pid)
            damage = (root / "extension.log").read_text().count(".damage_buffer(")
            time.sleep(0.8)
            second = session.resources(process.pid)
            final_damage = (root / "extension.log").read_text().count(".damage_buffer(")
            if first["cpu_ticks"] != second["cpu_ticks"] or second["threads"] != 1 or damage != final_damage:
                raise AssertionError(f"idle external integration keeps polling: {first} -> {second}")
            report["idle"] = {"before": first, "after": second, "damage": damage}
            session.key("Escape")
            EXPORT.finish(process)
        finally:
            EXPORT.close(process, log)
        # 2. 【外部动作回归】【桌面列表】隐藏条目不能进入列表，左键选择把同一带框图片直接交给独立程序
        desktop_result = root / "desktop.json"
        entry = apps / "viewer.desktop"
        entry.write_text(f'[Desktop Entry]\nType=Application\nName=Controlled Viewer\nExec=/usr/bin/python3 "{script}" "{desktop_result}" %f %u %c %k\nMimeType=image/png;\n')
        (apps / "hidden.desktop").write_text('[Desktop Entry]\nType=Application\nName=Hidden\nExec=/absent/never-run\nMimeType=image/png\nNoDisplay=true\n')
        process, log = start(binary, environment, root, "desktop")
        try:
            before = frame(root, "desktop-before")
            session.key("F6")
            bounds = changed_panel(before, frame(root, "desktop-open"), 280, 85)
            click((bounds[0] + 30, bounds[1] + 49), "right")
            hovered_row(root, "desktop-right", bounds)
            if desktop_result.exists() or process.poll() is not None:
                raise AssertionError("right click activated a desktop application")
            click((bounds[0] + 30, bounds[1] + 49))
            EXPORT.finish(process)
            record = consume_image(desktop_result, expected)
            if record["args"][2:] != ["Controlled Viewer", str(entry)] or not record["args"][1].startswith("file:///"):
                raise AssertionError(f"desktop argument boundaries differ: {record}")
            report["desktop"] = {"panel_bounds": bounds, "receiver": record}
        finally:
            EXPORT.close(process, log)
        # 3. 【外部动作回归】【失败恢复】真正的 exec 失败不能关闭会话，重新创建表面后仍可保存原始标注
        entry.write_text('[Desktop Entry]\nType=Application\nName=Missing Viewer\nExec=/absent/mark-shot-test-viewer %f\nMimeType=image/png\n')
        process, log = start(binary, environment, root, "failure")
        try:
            session.key("F6")
            session.key("Home")
            session.key("Return")
            EXPORT.ready(process, environment)
            if process.poll() is not None or list(Path(environment["TMPDIR"]).glob("*.png")):
                raise AssertionError("failed desktop launch closed editor or leaked temporary PNG")
            frame(root, "failure-restored")
            session.key("ctrl+s")
            EXPORT.finish(process)
            saved = list((root / "failure").glob("*.png"))
            if len(saved) != 1:
                raise AssertionError("restored editor did not save exactly once")
            report["failure_saved"] = EXPORT.equal(saved[0].read_bytes(), expected, "save after failed desktop exec")
        finally:
            EXPORT.close(process, log)
    if (root / "data/mark-shot/models").exists():
        raise AssertionError("external integration initialized OCR models")
    (root / "result.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report


def main():
    """【外部动作回归】【命令入口】读取 binary、root，执行完整私有交付验证并返回报告"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
