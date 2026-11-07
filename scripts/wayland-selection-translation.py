#!/usr/bin/env python3
"""【框选翻译回归】【真实会话】验证原图位置翻译、语言缓存、输入隔离、取消、导出和贴图交接"""

import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shlex
import shutil
import subprocess
import time
import tomllib

from PIL import Image, ImageChops, ImageDraw, ImageFont
import wayland_text_format_session as session
from wayland_text_pointer import click as device_click, pointer_context, CURRENT
from wayland_ocr_popup_tree import active_tree, popup_tree
from wayland_virtual_pointer import transform_for_pointer

PROJECT = Path(__file__).resolve().parents[1]


def support(name):
    """【框选翻译回归】【共用检查】按 name 载入既有私有会话工具，返回模块，不连接用户桌面"""
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), PROJECT / f"scripts/{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


EXPORT = support("wayland-export-image")
PIN = support("wayland-pin-translation")
UPLOAD = support("wayland-upload")
REFERENCE = json.loads((PROJECT / "tests/fixtures/qt-selection-translation/reference.json").read_text())


def move(point):
    """【框选翻译回归】【实际移动】将 point 交给本次校验设备，返回空值"""
    pointer, logical, _, x11 = CURRENT.get()
    if pointer is None:
        environment, mode = x11
        x = round((point[0] - logical["x"]) * mode["width"] / logical["width"])
        y = round((point[1] - logical["y"]) * mode["height"] / logical["height"])
        # 1. 【框选翻译回归】【私有输入送达】Xvfb 的第一次跳转可能只更新 X11 坐标，先用不同位置触发 winit 再送达实际目标
        adjacent = (x - 2 if x >= 2 else x + 2, y - 2 if y >= 2 else y + 2)
        subprocess.run(["xdotool", "mousemove", "--sync", str(adjacent[0]), str(adjacent[1])], env=environment, check=True, timeout=3)
        time.sleep(0.06)
        subprocess.run(["xdotool", "mousemove", "--sync", str(x), str(y)], env=environment, check=True, timeout=3)
    else:
        pointer.move_abs(*transform_for_pointer(point[0], point[1], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))
    time.sleep(0.06)


def click(point, button="left"):
    """【框选翻译回归】【完整点击】先向同一设备送达 point，再发送 button 完整序列，返回空值"""
    if CURRENT.get()[0] is None:
        move(point)
    device_click(point, button)


def drag(start, end):
    """【框选翻译回归】【连续拖动】用同一已校验设备从 start 拖到 end，始终释放左键，不混用输入来源"""
    pointer, _, _, x11 = CURRENT.get()
    move(start)
    if pointer is None:
        subprocess.run(["xdotool", "mousedown", "1"], env=x11[0], check=True, timeout=3)
    else:
        pointer.press()
    try:
        for step in range(1, 13):
            move((start[0] + (end[0] - start[0]) * step / 12, start[1] + (end[1] - start[1]) * step / 12))
    finally:
        if pointer is None:
            subprocess.run(["xdotool", "mouseup", "1"], env=x11[0], check=True, timeout=3)
        else:
            pointer.release()


def snapshot(root, name):
    """【框选翻译回归】【完整画面】在 root 保存 name 真实截图，返回 RGB，不使用内部状态替代显示"""
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def image_rect(bounds):
    """【框选翻译回归】【原图范围】按 bounds 返回居中四百乘二百原图矩形"""
    return ((bounds[0] - 400) / 2, (bounds[1] - 200) / 2, 400, 200)


def crop(image, bounds):
    """【框选翻译回归】【原图采样】按 bounds 裁切 image 内部原图，避开独立选框和浮条"""
    x, y, width, height = image_rect(bounds)
    return image.crop((round(x + 3), round(y + 3), round(x + width - 3), round(y + height - 3)))


def bar_point(bounds, label, action):
    """【框选翻译回归】【浮条定位】按实际 Qt label 尺寸和 action 返回按钮中心，bounds 为真实输出尺寸"""
    sample = next(case for case in REFERENCE["bars"] if case["label"] == label)
    x, y, width, _ = image_rect(bounds)
    left, top = x + width - sample["size"][0], y - 6 - sample["size"][1]
    item = sample["buttons"][action]["rect"]
    return left + item[0] + item[2] / 2, top + item[1] + item[3] / 2


def toolbar_point(bounds):
    """【框选翻译回归】【工具按钮】按现有完整工具栏布局返回实际翻译按钮中心，bounds 为逻辑窗口尺寸"""
    text = (PROJECT / "src/ui/toolbar/items.rs").read_text().split("pub const FULLSCREEN_ITEMS:", 1)[1].split("];", 1)[0]
    items = [line.strip() for line in text.splitlines() if line.strip() == "S," or line.strip().startswith("I(")]
    along, across, longest, point = 12, 0, 12, None
    for item in items:
        length = 1 if item == "S," else 30
        if along + length > bounds[0] - 28 and along > 12:
            longest = max(longest, along - 2)
            along, across = 0, across + 32
        if "Integration::Translate" in item:
            point = (6 + along + length / 2, 6 + across + 15)
        along += length + 2
    width = max(longest, along - 2) + 12
    if point is None:
        raise AssertionError("translation toolbar entry is missing")
    return (bounds[0] - width) / 2 + point[0], 12 + point[1]


def theme(config, log, expected):
    """【框选翻译回归】【主题完成】切换后等待 config 保存 expected 且 log 当前菜单完成新缓冲，返回空值，超时报错"""
    # 1. 【框选翻译回归】【绘制身份】保留当前菜单角色与已提交帧数，旧菜单或迟到事件不能作为新主题完成证据
    before = active_tree(log.read_text())
    assert len(before) == 1, "theme check requires one active language popup"
    role, frames = before[0]["role"], before[0]["frames"]
    session.key("ctrl+shift+l")
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        # 2. 【框选翻译回归】【原子保存】临时配置内容不代表正式替换，目录同步后菜单才继续绘制
        values = tomllib.loads(config.read_text())
        current = active_tree(log.read_text())
        if values.get("ui", {}).get("theme") == expected and any(node["role"] == role and node["frames"] > frames for node in current):
            return
        time.sleep(0.01)
    raise AssertionError(f"theme did not persist and repaint the language popup: {expected}")


def choose(log, index, point=None, exercise_right=False, root=None, config=None, process=None, report=None):
    """【框选翻译回归】【明确语言】从 point 或快捷键打开真实语言列表，按 index 明确确认目标，返回当前原生角色"""
    if point is None:
        session.key("F8")
    else:
        click(point)
    time.sleep(0.12)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        try:
            tree = active_tree(log.read_text())
        except AssertionError as error:
            if str(error) != "popup has no explicit native grab":
                raise
            time.sleep(0.03)
            continue
        if len(tree) == 1 and tree[0]["rect"] is not None:
            break
        time.sleep(0.03)
    else:
        raise AssertionError("selection language popup did not map")
    if exercise_right:
        rectangle = tree[0]["rect"]
        click((rectangle[0] + 20, rectangle[1] + 24 + index * 26 + 13), "right")
        assert len(active_tree(log.read_text())) == 1, "right button selected language"
        light = snapshot(root, "language-light")
        theme(config, log, "dark")
        dark = snapshot(root, "language-dark")
        assert ImageChops.difference(light, dark).getbbox() is not None
        theme(config, log, "light")
        PIN.equal(snapshot(root, "language-light-restored"), light, "native language popup theme restoration")
        report["idle_popup"] = PIN.idle(process, log)
    session.key("Home")
    for _ in range(index):
        session.key("Down")
    session.key("Return")
    return tree[0]


def reclaimed(record):
    """【框选翻译回归】【真实回收】等待 record 对应私有输入和命令进程消失，不接受只有结果通知"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if not Path(record["input"]).exists() and not Path(f"/proc/{record['pid']}").exists():
            return
        time.sleep(0.03)
    raise AssertionError("translation command or private input survived cancellation")


def capture_before_overlay(log):
    """【框选翻译回归】【捕获顺序】从 log 验证原生帧完成早于覆盖层窗口创建，返回两条实际协议记录"""
    lines = log.read_text().splitlines()
    ready = next((i for i, line in enumerate(lines) if ".ready," in line and ("zwlr_screencopy_frame_v1@" in line or "ext_image_copy_capture_frame_v1@" in line)), None)
    window = next((i for i, line in enumerate(lines) if ".get_toplevel(xdg_toplevel@" in line), None)
    if ready is None or window is None:
        raise AssertionError("native capture or overlay window protocol record is missing")
    if ready >= window:
        raise AssertionError("overlay window was created before native capture completed")
    return {"capture_ready": lines[ready], "overlay_window": lines[window]}


@contextmanager
def launch(binary, environment, config, source, root, name):
    """【框选翻译回归】【进程所属】用 source 在 environment 启动 binary，root/name 保存日志，返回进程与日志"""
    path = root / f"{name}.log"
    with path.open("w") as log:
        arguments = [str(binary), "--to", "", "--config", str(config)]
        if source is not None:
            arguments.extend(["--fullscreen", str(source)])
        process = subprocess.Popen(arguments, env=dict(environment, WAYLAND_DEBUG="client"), stdout=subprocess.DEVNULL, stderr=log)
        try:
            EXPORT.ready(process, environment)
            yield process, path
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=4)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)


def configure(root):
    """【框选翻译回归】【私有命令】在 root 创建真实坐标交换命令和配置，返回配置、请求及控制文件"""
    command, records, mode = root / "translate.py", root / "requests.jsonl", root / "mode"
    command.write_text("""import json,os,pathlib,sys,time
p=sys.argv[1];data=json.load(open(p));mode=pathlib.Path(sys.argv[4]).read_text()
with open(sys.argv[3],'a') as out: out.write(json.dumps({'pid':os.getpid(),'input':p,'mode':os.stat(p).st_mode & 511,'data':data})+'\\n')
if mode=='delay': time.sleep(60)
if mode=='empty': print(json.dumps({'tokens':[]}))
else:
 for i,token in enumerate(data['tokens']): token['text']=sys.argv[2]+' translated paragraph '+str(i);token['line']=99+i
 print(json.dumps({'tokens':list(reversed(data['tokens']))}))
""")
    template = f"exec /usr/bin/python {shlex.quote(str(command))} {{input}} {{targetLanguage}} {shlex.quote(str(records))} {shlex.quote(str(mode))}"
    config = root / "config.toml"
    config.write_text("[general]\noverlay='window'\nto=''\n[notifications]\nenabled=false\n[history]\nenabled=false\n[pinned]\nalways_on_top=false\n[keys]\ntranslate='F8'\n[translation]\ntarget_language='English'\ntimeout_ms=10000\ncommand=" + json.dumps(template) + "\n[custom]\nsentinel='retained'\n")
    mode.write_text("success")
    return config, records, mode


def copy_and_wait(environment, point, expected):
    """【框选翻译回归】【全文复制】点击 point 并等待 environment 的完整 expected，不受显示省略影响"""
    click(point)
    PIN.wait_text(environment, expected)


def run(binary, root, models, download_model, resources_only=False):
    """【框选翻译回归】【完整执行】用 binary 在私有会话验证 root 场景，models 为已核实缓存，download_model 控制真实首次下载"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/mark-shot-test")
    Path(environment["TMPDIR"]).mkdir()
    destination = root / "data/mark-shot/models"
    shutil.copytree(models, destination)
    source = root / "source-image.png"
    image = Image.new("RGB", (400, 200), "white")
    draw = ImageDraw.Draw(image)
    font = ImageFont.truetype("DejaVuSans.ttf", 24)
    draw.text((20, 25), "Original screenshot text", font=font, fill=(17, 24, 39))
    draw.text((20, 90), "Second original paragraph", font=font, fill=(17, 24, 39))
    image.save(source)
    config, records, mode = configure(root)
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": session.backend().__name__}
    with session.input_context(), pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        bounds = UPLOAD.geometry()
        with launch(binary, environment, config, source, root, "editor") as (process, log):
            move((bounds[0] - 30, bounds[1] - 30))
            baseline = snapshot(root, "source")
            if not resources_only:
                report["idle_source"] = PIN.idle(process, log)
            click(toolbar_point(bounds), "right")
            assert not active_tree(log.read_text()) and not PIN.records(records), "right button activated screenshot translation"
            first_popup = choose(log, 2, exercise_right=not resources_only, root=root, config=config, process=process, report=report)
            first = PIN.wait_record(records, 1)
            reclaimed(first)
            if resources_only:
                names = [path.read_text().strip() for path in Path(f"/proc/{process.pid}/task").glob("*/comm")]
                (root / "thread-probe.json").write_text(json.dumps({"resources": session.resources(process.pid), "threads": names}, indent=2) + "\n")
            report["idle_translated"] = PIN.idle(process, log)
            if resources_only:
                report["requests"] = PIN.records(records)
                (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
                return report
            move((bounds[0] - 30, bounds[1] - 30))
            translated = snapshot(root, "translated")
            assert ImageChops.difference(crop(baseline, bounds), crop(translated, bounds)).getbbox() is not None, "translated text did not replace frozen screenshot pixels"
            assert first["mode"] == 0o600 and first["data"]["tokens"][0]["box"][0] > 0
            assert first["data"]["tokens"][0]["text"] == "Original screenshot text"
            assert not any(node["app_id"] == "mark-shot-ocr-result" for node in EXPORT.windows(environment)), "screenshot translation opened independent OCR editor"
            click(bar_point(bounds, "English", 1), "right")
            move((bounds[0] - 30, bounds[1] - 30))
            PIN.equal(snapshot(root, "right-original"), translated, "right button did not change translation")
            click(bar_point(bounds, "English", 1))
            move((bounds[0] - 30, bounds[1] - 30))
            PIN.equal(crop(snapshot(root, "original-view"), bounds), crop(baseline, bounds), "original screenshot restoration")
            click(bar_point(bounds, "English", 2))
            copy_and_wait(environment, bar_point(bounds, "English", 3), "English translated paragraph 0\nEnglish translated paragraph 1")
            time.sleep(4.5)
            move((bounds[0] - 30, bounds[1] - 30))
            choose(log, 3, bar_point(bounds, "English", 0))
            second = PIN.wait_record(records, 2)
            reclaimed(second)
            time.sleep(0.2)
            choose(log, 2, bar_point(bounds, "Japanese", 0))
            time.sleep(0.2)
            assert len(PIN.records(records)) == 2, "returning to another cached language invoked translation"
            # 1. 【框选翻译回归】【语言缓存】已翻译目标切回时不再次识别或请求，原始坐标保持一致
            session.key("F8")
            assert not active_tree(log.read_text()), "translation toolbar toggle should exit active mode"
            PIN.equal(crop(snapshot(root, "exited"), bounds), crop(baseline, bounds), "translation mode exit")
            choose(log, 2)
            third = PIN.wait_record(records, 3)
            reclaimed(third)
            time.sleep(0.2)
            choose(log, 2, bar_point(bounds, "English", 0))
            time.sleep(0.2)
            assert len(PIN.records(records)) == 3, "cached language invoked translator again"
            mode.write_text("empty")
            choose(log, 3, bar_point(bounds, "English", 0))
            failed = PIN.wait_record(records, 4)
            reclaimed(failed)
            time.sleep(4.5)
            move((bounds[0] - 30, bounds[1] - 30))
            PIN.equal(snapshot(root, "failed-preserved"), translated, "failed target preserved original current translation")
            report["idle_failed"] = PIN.idle(process, log)
            mode.write_text("delay")
            choose(log, 5, bar_point(bounds, "English", 0))
            canceled = PIN.wait_record(records, 5)
            session.key("Escape")
            reclaimed(canceled)
            time.sleep(4.5)
            move((bounds[0] - 30, bounds[1] - 30))
            PIN.equal(snapshot(root, "cancel-preserved"), translated, "canceled target preserved display")
            report["idle_canceled"] = PIN.idle(process, log)
            mode.write_text("success")
            # 2. 【框选翻译回归】【贴图交接】当前译文和原图一起交付，贴图切换和原图复制不触发额外识别
            session.key("ctrl+p")
            assert process.wait(timeout=5) == 0
            pin = EXPORT.wait_window(environment, "mark-shot-pin")
            rect = PIN.geometry(environment, pin)
            pin_translated = PIN.screenshot(root, "pin-translated", rect)
            assert ImageChops.difference(pin_translated, image).getbbox() is not None
            PIN.menu(rect, 12)
            time.sleep(0.15)
            session.key("ctrl+c")
            EXPORT.clipboard(environment, source.read_bytes(), "handoff-original", root)
            assert len(PIN.records(records)) == 5
            session.key("Escape")
            report["popup_tree"] = popup_tree(log.read_text())
            assert all(node["destroyed"] for node in report["popup_tree"])
            assert first_popup["grab"] is not None, "selection language popup lacked native grab"
        # 3. 【框选翻译回归】【首次引导】独立空缓存先取消，再真实下载，下载完成继续同一截图翻译
        guide_environment = dict(environment, XDG_DATA_HOME=str(root / "guide-data"), XDG_STATE_HOME=str(root / "guide-state"))
        with launch(binary, guide_environment, config, source, root, "guide") as (process, log):
            choose(log, 2)
            guide = EXPORT.wait_window(guide_environment, "mark-shot-ocr-model-guide", process.pid)
            session.key("Escape")
            EXPORT.wait_window(guide_environment, "mark-shot", process.pid)
            assert len(PIN.records(records)) == 5
            report["guide_cancel_idle"] = PIN.idle(process, log)
            if download_model:
                choose(log, 2)
                guide = EXPORT.wait_window(guide_environment, "mark-shot-ocr-model-guide", process.pid)
                guide_rect = PIN.geometry(guide_environment, guide, (320, 396))
                click((guide_rect["x"] + 150, guide_rect["y"] + 95))
                deadline = time.monotonic() + 90
                while len(PIN.records(records)) < 6 and time.monotonic() < deadline:
                    time.sleep(0.1)
                if len(PIN.records(records)) != 6:
                    raise AssertionError("download did not continue screenshot translation")
                reclaimed(PIN.records(records)[-1])
                copy_and_wait(guide_environment, bar_point(bounds, "English", 3), "English translated paragraph 0\nEnglish translated paragraph 1")
                time.sleep(4.5)
                report["download_then_translation_idle"] = PIN.idle(process, log)
                session.key("ctrl+c")
                assert process.wait(timeout=5) == 0
                exported = subprocess.check_output(["wl-paste", "--type", "image/png"], env=guide_environment, timeout=3)
                (root / "translated-export.png").write_bytes(exported)
                with Image.open(io.BytesIO(exported)) as translated_png:
                    assert translated_png.size == image.size
                    assert translated_png.convert("RGB").tobytes() != image.tobytes(), "screenshot copy omitted translation"
                report["model_fingerprints"] = {path.name: {"size": path.stat().st_size, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()} for path in (root / "guide-data/mark-shot/models").iterdir() if path.is_file()}
            else:
                session.key("Escape")
                assert process.wait(timeout=5) == 0
        # 4. 【框选翻译回归】【真实捕获】识别来自原生桌面截图的区域，保留翻转及分数缩放下的正确坐标与导出
        with PIN.launch(binary, config, source.read_bytes(), root, "capture-background", seed=False) as (background, background_log, background_environment):
            pin = PIN.wait_window(background_environment, background, "mark-shot-pin")
            rectangle = PIN.geometry(background_environment, pin)
            with launch(binary, environment, config, None, root, "capture") as (process, log):
                report["native_capture_timeline"] = capture_before_overlay(log)
                start = (rectangle["x"] + 2, rectangle["y"] + 2)
                end = (rectangle["x"] + rectangle["width"] - 2, rectangle["y"] + rectangle["height"] - 2)
                drag(start, end)
                move((bounds[0] - 30, bounds[1] - 30))
                baseline_capture = snapshot(root, "native-capture-source")
                count = len(PIN.records(records))
                choose(log, 2)
                captured_request = PIN.wait_record(records, count + 1)
                reclaimed(captured_request)
                report["native_capture_idle"] = PIN.idle(process, log)
                visible_capture = snapshot(root, "native-capture-translated")
                assert ImageChops.difference(baseline_capture, visible_capture).getbbox() is not None
                assert any("Original screenshot text" in token["text"] for token in captured_request["data"]["tokens"])
                session.key("ctrl+c")
                assert process.wait(timeout=5) == 0
                exported = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
                (root / "native-capture-export.png").write_bytes(exported)
                with Image.open(io.BytesIO(exported)) as png:
                    assert png.width > rectangle["width"] - 10, "native capture export lost physical resolution"
            EXPORT.focus(background_environment, pin)
            session.key("Escape")
            assert background.wait(timeout=5) == 0
    report["requests"] = PIN.records(records)
    assert tomllib.loads(config.read_text())["custom"]["sentinel"] == "retained"
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【框选翻译回归】【执行入口】解析构建、目录与模型，执行真实私有会话，打印证据路径"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=PROJECT / "target/release/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--models", type=Path, default=PROJECT / "target/wlpt-s5/data/mark-shot/models")
    parser.add_argument("--download-model", action="store_true")
    parser.add_argument("--resources-only", action="store_true")
    options = parser.parse_args()
    report = run(options.binary.resolve(), options.root.resolve(), options.models.resolve(), options.download_model, options.resources_only)
    print(json.dumps({"binary_sha256": report["binary_sha256"], "compositor": report["compositor"], "requests": len(report["requests"]), "result": str(options.root / "result.json")}, indent=2))


if __name__ == "__main__":
    main()
