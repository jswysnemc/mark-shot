#!/usr/bin/env python3
"""【贴图翻译回归】【真实会话】验证父子菜单、原图覆盖、失败取消、交接与首次模型引导"""

import argparse
from contextlib import contextmanager
import hashlib
import io
import json
from pathlib import Path
import shlex
import struct
import subprocess
import time
import tomllib

from PIL import Image, ImageChops, ImageDraw, ImageFont
import wayland_text_format_session as session
from wayland_text_pointer import click, pointer_context
from wayland_ocr_popup_tree import active_tree, popup_tree

PROJECT = Path(__file__).resolve().parents[1]
SWAYMSG = PROJECT / "target/test-tools/sway/usr/bin/swaymsg"


def windows(environment):
    """【贴图翻译回归】【窗口列表】按 environment 读取已校验私有窗口，返回普通窗口身份"""
    session.environment()
    if session.backend() is session.niri:
        return session.niri.windows()
    tree = json.loads(subprocess.check_output([str(SWAYMSG), "-r", "-t", "get_tree"], env=environment))
    result, pending = [], [tree]
    while pending:
        node = pending.pop()
        if node.get("app_id") and node.get("pid"):
            result.append(node)
        pending.extend(node.get("nodes", []) + node.get("floating_nodes", []))
    return result


def wait_window(environment, process, app_id):
    """【贴图翻译回归】【窗口等待】等待 process 对应唯一 app_id，返回真实窗口，退出或超时报错"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AssertionError(f"pin exited before mapping: {process.returncode}")
        found = [node for node in windows(environment) if node["pid"] == process.pid and node["app_id"] == app_id]
        if len(found) == 1:
            return found[0]
        if len(found) > 1:
            raise AssertionError("ambiguous private window")
        time.sleep(0.03)
    raise AssertionError(f"private window did not map: {app_id}")


def geometry(environment, node, size=(400, 200)):
    """【贴图翻译回归】【窗口定位】按 node 在私有输出设置实际浮动尺寸，返回逻辑矩形"""
    if session.backend() is session.niri:
        node = session.niri.size_window(node, *size)
        session.niri.control("focus", node["id"])
        return dict(x=node["floating_pos"][0], y=node["floating_pos"][1], width=node["size"][0], height=node["size"][1])
    subprocess.run([str(SWAYMSG), f"[con_id={node['id']}] floating enable, resize set width {size[0]} px height {size[1]} px, move position center, focus"], env=environment, check=True, stdout=subprocess.DEVNULL)
    identifier, pid = node["id"], node["pid"]
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        node = next(node for node in windows(environment) if node["id"] == identifier and node["pid"] == pid)
        rect = node["rect"]
        if (rect["width"], rect["height"]) == size:
            time.sleep(0.12)
            return rect
        time.sleep(0.03)
    raise AssertionError("pin dimensions were not applied")


def screenshot(root, name, rect=None):
    """【贴图翻译回归】【画面采集】按 root/name 采集私有输出，rect 可裁切贴图，返回 RGB 像素"""
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        image = image.convert("RGB")
        if rect is not None:
            image = image.crop((rect["x"], rect["y"], rect["x"] + rect["width"], rect["y"] + rect["height"]))
        return image


def equal(actual, expected, label):
    """【贴图翻译回归】【完整恢复】按 label 比较完整画面，任一像素差异均报告失败"""
    if actual.size != expected.size or ImageChops.difference(actual, expected).getbbox() is not None:
        raise AssertionError(f"{label} differs in image pixels")


def menu(rect, steps, activate=True):
    """【贴图翻译回归】【菜单导航】在 rect 中打开真实右键菜单，按 steps 导航，activate 决定是否明确确认"""
    click((rect["x"] + 40, rect["y"] + 40), "right")
    time.sleep(0.15)
    session.key("Home")
    for _ in range(steps):
        session.key("Down")
    if activate:
        session.key("Return")


def language(rect, index, busy=False):
    """【贴图翻译回归】【语言选择】按 index 经真实父子菜单确认目标，busy 时翻译行禁用且导航跳过"""
    menu(rect, 10 if busy else 11, activate=False)
    session.key("Right")
    time.sleep(0.15)
    session.key("Home")
    for _ in range(index):
        session.key("Down")
    session.key("Return")


def menu_theme(root, config, light, expected):
    """【贴图翻译回归】【主题等待】只发送一次主题快捷键，等待配置及真实菜单像素同时变化或恢复，返回实际画面"""
    session.key("ctrl+shift+l")
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        if tomllib.loads(config.read_text())["ui"]["theme"] == expected:
            image = screenshot(root, "menus-theme-" + expected)
            difference = ImageChops.difference(light, image).getbbox()
            if expected == "light" and difference is None:
                return image
            if expected == "dark" and difference is not None:
                count = sum(1 for before, after in zip(light.get_flattened_data(), image.get_flattened_data()) if sum(before) - sum(after) > 300)
                if count > 10000:
                    return image
        time.sleep(0.03)
    raise AssertionError("parent and child menu did not present the persisted theme")


def records(path):
    """【贴图翻译回归】【请求记录】按 path 读取完整真实请求，忽略尚未写完的最后一行，返回独立记录"""
    if not path.exists():
        return []
    result = []
    for line in path.read_text().splitlines():
        try:
            result.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return result


def wait_record(path, count):
    """【贴图翻译回归】【请求等待】等待 path 至少 count 条真实请求，返回末条，超时报错"""
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline:
        values = records(path)
        if len(values) >= count:
            return values[-1]
        time.sleep(0.03)
    raise AssertionError("translation did not send the original image request")


def wait_text(environment, expected):
    """【贴图翻译回归】【文本交付】等待明确文本 MIME 类型及 expected 完整原文，旧 PNG 不参与文字解码，返回实际文本"""
    deadline = time.monotonic() + 6
    actual = None
    while time.monotonic() < deadline:
        process = subprocess.run(["wl-paste", "--no-newline", "--type", "text/plain;charset=utf-8"],
            env=dict(environment, WAYLAND_DEBUG=""), capture_output=True, check=False, timeout=3)
        if process.returncode == 0:
            actual = process.stdout.decode("utf-8")
            if actual == expected:
                return actual
        time.sleep(0.03)
    raise AssertionError(f"actual copied text did not match: {actual!r} != {expected!r}")


def idle(process, log, completion_notice=False):
    """【贴图翻译回归】【空闲资源】按 completion_notice 等待完成提示到期，再核对 process 的 CPU、线程和 log 提交，返回采样"""
    session.wait_single_thread(process.pid)
    if completion_notice:
        time.sleep(3.2)
    time.sleep(0.3)
    before = session.resources(process.pid)
    damage = log.read_text().count(".damage_buffer(")
    time.sleep(1)
    after = session.resources(process.pid)
    assert before["threads"] == after["threads"] == 1
    assert before["cpu_ticks"] == after["cpu_ticks"], (before, after)
    assert damage == log.read_text().count(".damage_buffer("), "idle pin keeps submitting frames"
    return dict(before=before, after=after, damage=damage)


@contextmanager
def launch(binary, config, payload, root, name, seed=True):
    """【贴图翻译回归】【进程归属】启动本次 binary 和有界 payload，返回进程，结束时只回收当前句柄"""
    log_path = root / f"{name}.log"
    environment = dict(session.environment(), WAYLAND_DEBUG="client", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/mark-shot-test")
    arguments = [str(binary), "--config", str(config), "--pin"] + (["--translation-seed"] if seed else [])
    with log_path.open("w") as log:
        process = subprocess.Popen(arguments, env=environment, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=log)
        process.stdin.write(payload)
        process.stdin.close()
        try:
            yield process, log_path, environment
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)


def run(binary, root, download_model=False):
    """【贴图翻译回归】【完整流程】按 binary、root 和 download_model 验证真实菜单、覆盖、取消及模型引导，返回构建指纹报告"""
    root.mkdir(parents=True, exist_ok=False)
    image = Image.new("RGB", (400, 200), "white")
    draw = ImageDraw.Draw(image)
    font = ImageFont.truetype("DejaVuSans.ttf", 24)
    draw.text((20, 25), "Original screenshot text", font=font, fill=(17, 24, 39))
    draw.text((20, 90), "Second original paragraph", font=font, fill=(17, 24, 39))
    png = io.BytesIO()
    image.save(png, "PNG")
    seed = {"source_tokens": [{"text": text, "box": [20, y, 335, 28], "line": index, "index": 0, "confidence": 1.0}
        for index, (text, y) in enumerate((("Original screenshot text.", 25), ("Second original paragraph.", 90)))], "blocks": [], "show_translation": False}
    metadata = json.dumps(seed).encode()
    payload = b"MS-PIN-TRANSLATION\0" + struct.pack("<I", len(metadata)) + metadata + png.getvalue()
    record, mode = root / "requests.jsonl", root / "mode"
    command = root / "translate.py"
    command.write_text("""import json,os,pathlib,sys,time
p=sys.argv[1];data=json.load(open(p));mode=pathlib.Path(sys.argv[4]).read_text()
with open(sys.argv[3],'a') as out: out.write(json.dumps({'pid':os.getpid(),'input':p,'mode':os.stat(p).st_mode & 511,'data':data})+'\\n')
if mode=='delay': time.sleep(60)
if mode=='empty': print(json.dumps({'tokens':[],'errors':['no translation result']}))
else:
 for i,token in enumerate(data['tokens']): token['text']=sys.argv[2]+' translated paragraph '+str(i);token['line']=99+i
 print(json.dumps({'tokens':list(reversed(data['tokens']))}))
""")
    template = f"exec /usr/bin/python {shlex.quote(str(command))} {{input}} {{targetLanguage}} {shlex.quote(str(record))} {shlex.quote(str(mode))}"
    config = root / "config.toml"
    config.write_text("[ui]\ntheme='light'\n[pinned]\nalways_on_top=false\n[translation]\ntarget_language='English'\ntimeout_ms=3000\ncommand=" + json.dumps(template) + "\n[custom]\nsentinel='retained'\n")
    mode.write_text("success")
    result = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": session.backend().__name__}
    with session.input_context(), pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        with launch(binary, config, payload, root, "pin") as (process, log, environment):
            rect = geometry(environment, wait_window(environment, process, "mark-shot-pin"))
            source = screenshot(root, "source", rect)
            result["idle_source"] = idle(process, log)
            menu(rect, 10)
            first = wait_record(record, 1)
            result["idle_translated"] = idle(process, log, completion_notice=True)
            translated = screenshot(root, "translated", rect)
            assert ImageChops.difference(source, translated).getbbox() is not None, "translation did not cover original paragraphs"
            assert first["mode"] == 0o600 and first["data"]["tokens"][0]["box"] == [20, 25, 335, 28]
            assert not Path(first["input"]).exists()
            menu(rect, 12)
            time.sleep(0.15)
            equal(screenshot(root, "show-original", rect), source, "source toggle")
            menu(rect, 12)
            time.sleep(0.15)
            equal(screenshot(root, "show-translated", rect), translated, "cached translation toggle")
            assert len(records(record)) == 1, "toggle unexpectedly repeated translation"
            menu(rect, 11, activate=False)
            session.key("Right")
            time.sleep(0.2)
            chain = active_tree(log.read_text())
            assert len(chain) == 2 and chain[1]["parent"] == chain[0]["xdg"]
            parent, child = chain[0]["rect"], chain[1]["rect"]
            click((rect["x"] + parent[0] + child[0] + 25, rect["y"] + parent[1] + child[1] + 16), "right")
            time.sleep(0.1)
            assert tomllib.loads(config.read_text())["translation"]["target_language"] == "English"
            assert len(records(record)) == 1 and not active_tree(log.read_text())
            menu(rect, 11, activate=False)
            session.key("Right")
            time.sleep(0.3)
            light_menus = screenshot(root, "menus-light")
            menu_theme(root, config, light_menus, "dark")
            menu_theme(root, config, light_menus, "light")
            session.key("Escape")
            session.key("Escape")
            time.sleep(0.15)
            equal(screenshot(root, "menus-closed-translation", rect), translated, "menu themes did not alter image overlay")
            menu(rect, 8)
            wait_text(environment, "English translated paragraph 0\nEnglish translated paragraph 1")
            menu(rect, 9)
            click((rect["x"] + 45, rect["y"] + 40))
            click((rect["x"] + 45, rect["y"] + 40))
            session.key("ctrl+c")
            wait_text(environment, "English translated paragraph 0")
            # 1. 【贴图翻译回归】【选择隔离】点击空白清除字符选择，再关闭文字选择，后续菜单序号保持原版可用行顺序
            click((rect["x"] + 380, rect["y"] + 170))
            menu(rect, 9)
            time.sleep(0.15)
            equal(screenshot(root, "selection-cleared", rect), translated, "selection overlay did not alter translation")
            session.key("ctrl+c")
            copied = subprocess.check_output(["wl-paste", "--type", "image/png"], env=dict(environment, WAYLAND_DEBUG=""), timeout=3)
            with Image.open(io.BytesIO(copied)) as copied:
                equal(copied.convert("RGB"), image, "copy keeps original image")
            language(rect, 3)
            assert tomllib.loads(config.read_text())["translation"]["target_language"] == "Japanese"
            time.sleep(0.15)
            equal(screenshot(root, "language-invalidated", rect), source, "language invalidates translation")
            menu(rect, 10)
            second = wait_record(record, 2)
            result["idle_japanese"] = idle(process, log, completion_notice=True)
            assert second["data"]["targetLanguage"] == "Japanese"
            language(rect, 2)
            mode.write_text("empty")
            menu(rect, 10)
            wait_record(record, 3)
            result["idle_failed"] = idle(process, log)
            failed = screenshot(root, "failed-keeps-source", rect)
            body = (0, 0, source.width, source.height - 60)
            equal(failed.crop(body), source.crop(body), "failed response preserves source text")
            assert ImageChops.difference(failed, source).getbbox() is not None, "failed translation has no in-window feedback"
            mode.write_text("delay")
            menu(rect, 10)
            delayed = wait_record(record, 4)
            language(rect, 5, busy=True)
            result["idle_canceled"] = idle(process, log)
            assert tomllib.loads(config.read_text())["translation"]["target_language"] == "French"
            assert not Path(delayed["input"]).exists() and not Path(f"/proc/{delayed['pid']}").exists()
            equal(screenshot(root, "canceled-keeps-source", rect), source, "language cancellation preserves source")
            result["popup_creations"] = log.read_text().count(".get_popup(")
            assert result["popup_creations"] >= 12
            result["popup_tree"] = popup_tree(log.read_text(), closed=True)
            session.key("Escape")
            assert process.wait(timeout=4) == 0
        with launch(binary, config, png.getvalue(), root, "guide", seed=False) as (process, log, environment):
            rect = geometry(environment, wait_window(environment, process, "mark-shot-pin"))
            before = screenshot(root, "guide-parent-before", rect)
            menu(rect, 10)
            guide = wait_window(environment, process, "mark-shot-ocr-model-guide")
            guide_rect = geometry(environment, guide, (360, 540))
            click((guide_rect["x"] + 150, guide_rect["y"] + 95), "right")
            assert len(records(record)) == 4
            result["guide_idle"] = idle(process, log)
            session.key("Escape")
            time.sleep(0.2)
            rect = geometry(environment, wait_window(environment, process, "mark-shot-pin"))
            equal(screenshot(root, "guide-cancel-restored", rect), before, "guide cancellation restores pin")
            result["guide_cancel_idle"] = idle(process, log)
            if download_model:
                mode.write_text("success")
                menu(rect, 10)
                guide = wait_window(environment, process, "mark-shot-ocr-model-guide")
                guide_rect = geometry(environment, guide, (360, 540))
                screenshot(root, "first-use-download-guide")
                click((guide_rect["x"] + 150, guide_rect["y"] + 95))
                deadline = time.monotonic() + 90
                while time.monotonic() < deadline:
                    if len(records(record)) >= 5:
                        break
                    if "【识别模型】【引导下载】下载失败" in log.read_text():
                        raise AssertionError("first-use model download failed; see guide.log")
                    time.sleep(0.1)
                else:
                    raise AssertionError("first-use download did not continue the intended translation")
                result["download_then_translation"] = records(record)[-1]
                result["download_then_translation_idle"] = idle(process, log, completion_notice=True)
                assert not any(node["app_id"] == "mark-shot-ocr-model-guide" for node in windows(environment))
                assert result["download_then_translation"]["data"]["tokens"]
                for token in result["download_then_translation"]["data"]["tokens"]:
                    assert token["box"][0] >= 0 and token["box"][1] >= 0
                rect = geometry(environment, wait_window(environment, process, "mark-shot-pin"))
                after_download = screenshot(root, "download-continued-translation", rect)
                assert ImageChops.difference(before, after_download).getbbox() is not None
            session.key("Escape")
            assert process.wait(timeout=4) == 0
        if download_model:
            with launch(binary, config, png.getvalue(), root, "copy-native", seed=False) as (process, log, environment):
                rect = geometry(environment, wait_window(environment, process, "mark-shot-pin"))
                before = screenshot(root, "copy-native-before", rect)
                menu(rect, 8)
                wait_text(environment, "Original screenshot text\nSecond original paragraph")
                result["copy_native_idle"] = idle(process, log)
                equal(screenshot(root, "copy-native-after", rect), before, "native OCR copy preserves image")
                assert len(records(record)) == 5
                assert not any(node["app_id"] == "mark-shot-ocr-result" for node in windows(environment))
                session.key("Escape")
                assert process.wait(timeout=4) == 0
    result["requests"] = records(record)
    assert tomllib.loads(config.read_text())["custom"]["sentinel"] == "retained"
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【贴图翻译回归】【命令入口】解析本次 binary 与 root，执行已校验私有回归，输出证据路径"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=PROJECT / "target/release/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--download-model", action="store_true")
    options = parser.parse_args()
    result = run(options.binary.resolve(), options.root.resolve(), options.download_model)
    print(json.dumps({"binary_sha256": result["binary_sha256"], "compositor": result["compositor"], "requests": len(result["requests"]), "result": str(options.root / "result.json")}, indent=2))


if __name__ == "__main__":
    main()
