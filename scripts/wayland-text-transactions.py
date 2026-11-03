#!/usr/bin/env python3
"""【文字事务回归】【实际输入】只连接私有 Wayland，验证草稿、局部历史、提交与删除恢复，不操作用户配置"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops
import wayland_test_session as session
from wayland_text_property_locator import font_button

SOURCE = (30, 34, 38)


def capture(process, root, name):
    """【文字事务回归】【画面记录】将 process 的 name 画面保存到 root，返回 RGB 图；提前退出时报告状态"""
    time.sleep(0.15)
    if process.poll() is not None:
        raise AssertionError(f"editor exited unexpectedly: {process.returncode}")
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def click(point):
    """【文字事务回归】【左键输入】在私有 point 发送完整左键点击，无返回值"""
    session.control("click", "--logical", *map(round, point))
    time.sleep(0.04)


def editor_text(environment, expected):
    """【文字事务回归】【正文核验】从当前真实编辑器选择复制全文，在 environment 的私有剪贴板验证 expected，无返回值"""
    session.key("ctrl+a")
    session.key("ctrl+c")
    session.wait_clipboard(environment, expected)


def body_bounds(image):
    """【文字事务回归】【画布核验】返回 image 内容区与固定原图颜色的差异边界，避开工具栏和选区边缘"""
    content = image.crop((50, 160, image.width - 50, image.height - 60))
    return ImageChops.difference(content, Image.new("RGB", content.size, SOURCE)).getbbox()


def edit(process, root, name):
    """【文字事务回归】【已有入口】通过 process 实际属性栏编辑按钮打开文字，在 root 保存 name 证据，无返回值"""
    button = font_button(capture(process, root, name + "-before"))
    click((button[0] + 38, button[1]))
    capture(process, root, name + "-opened")


def scenario(binary, root):
    """【文字事务回归】【完整场景】用 binary 验证独立文字事务，root 保存截图与导出；返回实际结果和空闲资源"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    with tempfile.TemporaryDirectory(prefix="mark-shot-text-session-") as directory:
        private = Path(directory)
        config, state = private / "config/mark-shot", private / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "s"\n[notifications]\nenabled = false\n')
        (state / "tools.toml").write_text('magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Inter"\nbold = false\nitalic = false\n')
        environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_DATA_HOME=str(private / "data"), XDG_STATE_HOME=str(private / "state"))
        source = private / "source.png"
        Image.new("RGB", (1600, 900), SOURCE).save(source)
        with (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text", "--default-color", "#FF4D4D", "--to", "s", "--output", str(private / "export"), str(source)], env=environment, stderr=log)
            try:
                initial = capture(process, root, "initial")
                session.wait_single_thread(process.pid)
                point = (round(initial.width * 0.58), round(initial.height * 0.52))
                # 1. 【文字事务回归】【新建提交】Esc 只提交并裁掉末尾空白，空编辑器不增加全局历史
                click(point)
                session.paste_text("  first\nsecond  ")
                capture(process, root, "draft")
                session.key("Escape")
                committed = capture(process, root, "escape-committed")
                if not body_bounds(committed):
                    raise AssertionError("Escape discarded the new text")
                click((point[0] - 100, point[1] + 100))
                session.key("Escape")
                capture(process, root, "empty-committed")
                session.key("ctrl+z")
                if body_bounds(capture(process, root, "new-undo")):
                    raise AssertionError("empty draft consumed the global undo step")
                session.key("ctrl+y")
                if not body_bounds(capture(process, root, "new-redo")):
                    raise AssertionError("committed text did not survive redo")
                # 2. 【文字事务回归】【已有编辑】编辑按钮保留选择工具，Ctrl+Z 和 Ctrl+Y 只修改编辑器
                session.key("v")
                click((point[0] + 12, point[1] + 12))
                edit(process, root, "existing")
                editor_text(environment, "  first\nsecond")
                session.paste_text("updated   ")
                capture(process, root, "existing-draft")
                session.key("ctrl+z")
                editor_text(environment, "  first\nsecond")
                capture(process, root, "local-undo")
                session.key("ctrl+y")
                editor_text(environment, "updated   ")
                session.key("ctrl+Return")
                capture(process, root, "existing-committed")
                # 3. 【文字事务回归】【删除恢复】清空已有文字后提交删除，撤销恢复对象和选中，编辑按钮仍可使用
                edit(process, root, "delete")
                editor_text(environment, "updated")
                session.key("BackSpace")
                session.key("Escape")
                if body_bounds(capture(process, root, "deleted")):
                    raise AssertionError("cleared text was not deleted on commit")
                session.key("ctrl+z")
                restored = capture(process, root, "delete-undo")
                if not body_bounds(restored):
                    raise AssertionError("deleted text was not restored")
                before = session.wait_single_thread(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if before["cpu_ticks"] != after["cpu_ticks"] or after["threads"] != 1:
                    raise AssertionError(f"committed editing stayed active while idle: {before}, {after}")
                edit(process, root, "direct-save")
                editor_text(environment, "updated")
                session.paste_text("saved directly  ")
                session.key("ctrl+s")
                process.wait(timeout=6)
                files = list((private / "export").glob("*.png"))
                if process.returncode or len(files) != 1:
                    raise AssertionError(f"direct editor export failed: {process.returncode}, {files}")
                with Image.open(files[0]) as image:
                    output = image.convert("RGB")
                if output.size != (1600, 900) or not ImageChops.difference(output, Image.new("RGB", output.size, SOURCE)).getbbox():
                    raise AssertionError("direct save omitted text or changed original resolution")
                shutil.copyfile(files[0], root / "export.png")
                shutil.copyfile(state / "tools.toml", root / "tools.toml")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["font_size"] != 20 or (private / "data/mark-shot/models").exists():
                    raise AssertionError("text transaction changed defaults or initialized OCR")
                return {"escape_commits": True, "trailing_whitespace_trimmed": True, "empty_draft_has_no_history": True, "editor_local_undo_redo": True, "selection_tool_preserved": True, "deleted_text_selection_restored": True, "direct_save_commits": True, "export_size": list(output.size), "idle_before": before, "idle_after": after}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【文字事务回归】【程序入口】解析 binary 与 artifacts，保存同一构建指纹和实际场景结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    options = parser.parse_args()
    binary = options.binary.resolve()
    result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "scenario": scenario(binary, options.artifacts.resolve())}
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
