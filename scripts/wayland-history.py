#!/usr/bin/env python3
"""【截图历史测试】【完整流程】在私有 Wayland 桌面验证记录、原图操作和设置"""

import argparse
import hashlib
import json
from pathlib import Path
import time
import tomllib

from PIL import Image

from wayland_filter_fixture import source_image
import wayland_history_actions as actions
from wayland_history_app import capture_image, configuration, headless, launch, wait_exit, wait_removed
import wayland_history_checks as checks
import wayland_test_session as session


def wait_value(config, key, expected):
    """【截图历史测试】【配置等待】等待 config 中 history.key 等于 expected；返回保存后的完整配置"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        value = tomllib.loads(config.read_text())
        if value["history"].get(key) == expected:
            return value
        time.sleep(0.04)
    raise AssertionError(f"history.{key} did not become {expected}")


def save_cases(item, environment, history, record, expected, artifacts):
    """【截图历史测试】【另存为】验证 Unicode 名称、默认取消覆盖及保存快照；返回独立保存文件"""
    destination = artifacts / "saved-files"
    destination.mkdir()
    actions.footer(item, "save")
    time.sleep(0.25)
    actions.navigate(item, environment, destination)
    actions.filename(item, environment, "历史副本.png")
    session.screenshot(artifacts / "file-picker.png")
    actions.save(item)
    saved = actions.wait_file(destination / "历史副本.png")
    checks.equal_image(saved, expected)
    if checks.idat(saved) != checks.idat(record):
        raise AssertionError("Save As re-encoded the source PNG")

    # 1. 【截图历史测试】【覆盖授权】默认 Enter 必须取消，只有明确 Replace 才能改变现有文件
    Image.new("RGBA", (3, 2), (10, 20, 30, 91)).save(saved)
    sentinel = saved.read_bytes()
    actions.footer(item, "save")
    time.sleep(0.2)
    actions.navigate(item, environment, destination)
    actions.filename(item, environment, saved.name)
    actions.save(item)
    session.screenshot(artifacts / "overwrite-confirmation.png")
    session.key("Return")
    if saved.read_bytes() != sentinel:
        raise AssertionError("default confirmation overwrote the file")
    actions.save(item)
    actions.overwrite(item, True)
    deadline = time.monotonic() + 5
    while saved.read_bytes() == sentinel and time.monotonic() < deadline:
        time.sleep(0.04)
    checks.equal_image(saved, expected)

    # 2. 【截图历史测试】【冻结快照】打开对话框后删除源记录，保存仍使用已经读取的原图
    actions.footer(item, "save")
    time.sleep(0.2)
    actions.navigate(item, environment, destination)
    actions.filename(item, environment, "snapshot-after-delete.png")
    record.unlink()
    actions.save(item)
    snapshot = actions.wait_file(destination / "snapshot-after-delete.png")
    checks.equal_image(snapshot, expected)
    checks.wait_count(history, 1)
    return saved


def index_case(binary, environment, config, history, item, source, artifacts):
    """【截图历史测试】【附加索引】显式启用索引，真实输入中文标注后搜索并复制该完成图片"""
    config.write_text(config.read_text().replace("index_annotations = false", "index_annotations = true"))
    label = "history 索引验证"
    destination = artifacts / "indexed-export"
    image = capture_image(binary, environment, config, source, destination, artifacts, "indexed", label)
    files = checks.wait_count(history, 2)
    indexed = next(path for path in files if Image.open(path).info.get(checks.SEARCH) == label)
    checks.equal_image(indexed, image)
    actions.search(item, "索引验证")
    actions.footer(item, "copy")
    checks.clipboard(environment, image, artifacts / "indexed-copy.png")
    session.screenshot(artifacts / "indexed-history.png")
    return indexed


def reedit_and_pin(environment, history, item, expected, artifacts):
    """【截图历史测试】【原图操作】验证贴图复制不重复记录，重新编辑完成后仅增加一个记录"""
    before = len(checks.owned(history))
    actions.footer(item, "pin")
    pin = actions.window("mark-shot-pin")
    actions.focus(pin)
    time.sleep(0.25)
    session.key("ctrl+c")
    checks.clipboard(environment, expected, artifacts / "pin-copy.png")
    if len(checks.owned(history)) != before:
        raise AssertionError("pinning or pin copying duplicated history")
    session.control("close", pin["id"])
    wait_removed(pin["id"])
    actions.focus(item)
    actions.footer(item, "edit")
    deadline = time.monotonic() + 8
    editor = None
    while time.monotonic() < deadline:
        matching = [window for window in session.windows() if window["app_id"] not in ("mark-shot-history", "mark-shot-pin")]
        if len(matching) == 1:
            editor = matching[0]
            break
        time.sleep(0.04)
    if editor is None:
        raise AssertionError("history re-edit did not open an editor")
    session.wait_single_thread(editor["pid"])
    actions.focus(editor)
    time.sleep(0.25)
    session.key("ctrl+c")
    wait_removed(editor["id"])
    checks.wait_count(history, before + 1)
    checks.clipboard(environment, expected, artifacts / "reedit-copy.png")
    actions.focus(item)


def settings_and_clear(binary, environment, config, history, item, saved, artifacts):
    """【截图历史测试】【设置与清空】窄窗口验证开关、数量、确认隔离与无关文件保留；返回空闲资源"""
    item = session.size_window(item, 360, 360)
    actions.search(item, "")
    session.screenshot(artifacts / "narrow-history.png")
    session.click(item, 64, 56)
    wait_value(config, "enabled", False)
    count = len(checks.owned(history))
    headless(binary, environment, config, artifacts / "disabled-capture.png")
    if len(checks.owned(history)) != count:
        raise AssertionError("disabled history recorded a new capture")
    session.click(item, 64, 56)
    wait_value(config, "enabled", True)

    for limit in (1, 200):
        session.click(item, 210, 56)
        session.paste_text(str(limit))
        session.key("Return")
        wait_value(config, "limit", limit)
        checks.wait_count(history, 1)
        time.sleep(0.12)
    values = tomllib.loads(config.read_text())
    if values["tools"]["marker"]["width"] != 13 or values["custom"]["preserve"] != 17 or "# preserved history preferences" not in config.read_text():
        raise AssertionError("settings overwrote unrelated configuration")
    resources = checks.idle(item["pid"])
    unrelated = history / "keep.txt"
    unrelated.write_text("keep")
    linked = history / "capture-20000101000000000-11111111111111111111111111111111.png"
    linked.symlink_to(saved)
    broken = history / "capture-20000101000000001-22222222222222222222222222222222.png"
    broken.write_bytes(b"broken PNG")
    actions.footer(item, "clear")
    session.key("Return")
    checks.wait_count(history, 2)
    actions.footer(item, "clear")
    actions.footer(item, "delete")
    checks.wait_count(history, 2)
    session.screenshot(artifacts / "clear-confirmation.png")
    panel_width = min(item["size"][0] - 32, 420)
    panel_x = (item["size"][0] - panel_width) / 2
    panel_y = (item["size"][1] - 152) / 2
    session.click(item, panel_x + panel_width - 48, panel_y + 124)
    checks.wait_count(history, 0)
    if not unrelated.exists() or not linked.is_symlink() or broken.exists() or not saved.exists():
        raise AssertionError("Clear crossed the owned-file boundary")
    return item, resources


def run(binary, artifacts):
    """【截图历史测试】【完整入口】执行 binary 的全部历史场景；证据写入新 artifacts，不访问真实桌面"""
    artifacts.mkdir(parents=True)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    environment, config, history, default = configuration(artifacts)
    default_before = default.read_bytes()
    source = artifacts / "source.png"
    source_image().save(source)
    original = capture_image(binary, environment, config, source, artifacts / "original-export", artifacts, "original", "plain secret text")
    record = checks.wait_count(history, 1)[0]
    checks.equal_image(record, original)
    if Image.open(record).info.get(checks.SEARCH):
        raise AssertionError("default recording persisted raw annotation text")
    report = headless(binary, environment, config, artifacts / "headless.png")
    records = checks.wait_count(history, 2)
    checks.equal_image(records[0], artifacts / "headless.png")
    checks.private_files(history)
    with Image.open(artifacts / "original-editor.png") as editor_frame:
        effective_scale = report["width"] / editor_frame.width
    if abs(float(Image.open(records[0]).info[checks.DPR]) - effective_scale) > 1e-6:
        raise AssertionError("headless history lost the actual pixel-to-logical ratio")
    process = launch(binary, environment, config, ["--history"], artifacts / "history-process.log")
    try:
        item = session.size_window(actions.window(pid=process.pid), 760, 460)
        actions.search(item, "1600x900")
        actions.footer(item, "copy")
        checks.clipboard(environment, original, artifacts / "history-copy.png")
        session.screenshot(artifacts / "desktop-history.png")
        initial_idle = checks.idle(process.pid)
        saved = save_cases(item, environment, history, record, original, artifacts)
        indexed = index_case(binary, environment, config, history, item, source, artifacts)
        indexed_image = artifacts / "indexed-copy.png"
        reedit_and_pin(environment, history, item, indexed_image, artifacts)
        checks.wait_count(history, 3)
        actions.footer(item, "delete")
        checks.wait_count(history, 2)
        if indexed.exists():
            raise AssertionError("Delete did not remove the selected record")
        item, compact_idle = settings_and_clear(binary, environment, config, history, item, saved, artifacts)
        session.key("Escape")
        wait_exit(process)
        if default.read_bytes() != default_before:
            raise AssertionError("an override-config helper wrote the default configuration")
        if (artifacts / "app/data/mark-shot/models").exists():
            raise AssertionError("ordinary history initialized OCR models")
        if hashlib.sha256(binary.read_bytes()).hexdigest() != fingerprint:
            raise AssertionError("binary changed during the history regression")
        result = {"sha256": fingerprint, "binary_bytes": binary.stat().st_size, "headless_report": report, "initial_idle": initial_idle, "compact_idle": compact_idle, "automatic_recording": True, "default_plaintext_index_off": True, "unicode_index_search": True, "source_rgba_preserved": True, "idat_reused": True, "persistent_clipboard": True, "pin_without_duplicates": True, "reedit_records_once": True, "unicode_save_as": True, "default_overwrite_cancel": True, "explicit_overwrite": True, "save_snapshot_survives_source_deletion": True, "count_limits": [1, 200], "immediate_pruning": True, "recording_toggle": True, "clear_confirmation_isolated": True, "unrelated_files_kept": True, "custom_config_preserved": True, "ocr_not_initialized": True, "window_sizes": [[760, 460], [360, 360]]}
        (artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=3)


def main():
    """【截图历史测试】【命令行】读取构建和新证据目录，执行隔离会话中的测试"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-history-r0"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
