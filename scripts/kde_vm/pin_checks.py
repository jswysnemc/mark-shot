"""【KDE虚拟机】【贴图回归】真实输入验证缩放、旋转、置顶切换和透明图像原件交付"""

import argparse
import json
from pathlib import Path
import re
import subprocess
import time

from PIL import Image, ImageDraw

from pin_performance import CONFIG, Input
from session import Session


def copy_image(session, expected, name):
    """【贴图测试】【原图断言】按 session 复制并逐像素比较 expected，name 保存产物，差异立即失败，无返回值"""
    session.key('ctrl+c')
    actual = Image.open(session.clipboard(name + '.png')).convert('RGBA')
    assert actual.size == expected.size, (actual.size, expected.size)
    assert actual.tobytes() == expected.tobytes(), f'{name}: 原图像素发生变化'


def menu_action(session, point, index):
    """【贴图测试】【菜单执行】在 point 打开真实菜单，按 index 选择有效行并确认，无返回值"""
    session.click(*point, button='right')
    session.key('home')
    for _ in range(index):
        session.key('down')
    session.key('ret')
    time.sleep(.3)


def geometry(session):
    """【贴图测试】【实际几何】按 session 最近的主浮层协议返回左、上、宽、高，避免累计滚轮舍入使输入落到其他窗口"""
    log = session.log('capture')
    role = re.findall(r'get_layer_surface\(zwlr_layer_surface_v1@(\d+),.*?Some\("mark-shot-pin"\)', log)[-1]
    width, height = map(int, re.findall(r'zwlr_layer_surface_v1@' + role + r'\.set_size\((\d+), (\d+)\)', log)[-1])
    top, _, _, left = map(int, re.findall(r'zwlr_layer_surface_v1@' + role + r'\.set_margin\((-?\d+), (-?\d+), (-?\d+), (-?\d+)\)', log)[-1])
    return left, top, width, height


def run(session, binary):
    """【贴图测试】【完整回归】以 session、binary 测试原图交付和图像子表面重建，返回结果，结束时回收本次窗口"""
    source = Image.new('RGBA', (640, 400), (0, 0, 0, 0))
    draw = ImageDraw.Draw(source)
    draw.rectangle((0, 0, 319, 199), fill=(240, 40, 60, 255))
    draw.rectangle((320, 0, 639, 199), fill=(20, 180, 60, 255))
    draw.rectangle((0, 200, 319, 399), fill=(40, 100, 240, 255))
    draw.rectangle((320, 200, 639, 399), fill=(99, 51, 201, 85))
    # 1. 【贴图测试】【透明往返】使用可以无损预乘再恢复的颜色，避免把透明颜色舍入误当作显示路径回归
    source.putpixel((639, 399), (0, 0, 0, 0))
    path = session.root / 'source.png'
    source.save(path)
    session.setup(CONFIG)
    remote = session.remote + '/source.png'
    subprocess.run(['scp', '-q', str(path), session.host + ':' + remote], check=True, timeout=20)
    pid = session.launch(binary, ['--pin-image', remote], overrides={'WAYLAND_DEBUG': '1'})
    control = Input(session)
    try:
        time.sleep(.8)
        assert session.alive(pid), session.log('capture')
        session.screenshot('initial')
        left, top = (session.size[0] - 640) // 2, (session.size[1] - 400) // 2
        point = (left + 150, top + 130)
        session.click(*point)
        copy_image(session, source, 'initial-copy')
        control.drag(point, (60, 30))
        time.sleep(.3)
        session.screenshot('moved')
        copy_image(session, source, 'moved-copy')
        point = (point[0] + 60, point[1] + 30)
        control.move(*point)
        control.zoom(8)
        time.sleep(.3)
        session.screenshot('zoomed')
        copy_image(session, source, 'zoomed-copy')
        left, top, width, height = geometry(session)
        control.drag((left + width - 3, top + height - 3), (80, 50))
        time.sleep(.3)
        session.screenshot('resized')
        copy_image(session, source, 'resized-copy')
        left, top, width, height = geometry(session)
        point = (left + width // 2, top + height // 2)
        # 2. 【贴图测试】【图像身份】依次旋转后验证实际像素，方位及共享源不能沿用旧图
        menu_action(session, point, 1)
        session.screenshot('rotated')
        copy_image(session, source.transpose(Image.Transpose.ROTATE_270), 'rotated-copy')
        left, top, width, height = geometry(session)
        point = (left + width // 2, top + height // 2)
        menu_action(session, point, 0)
        copy_image(session, source, 'restored-copy')
        # 3. 【贴图测试】【角色重建】切换置顶时销毁旧图像子表面，普通窗口与 layer 均交付相同原图
        left, top, width, height = geometry(session)
        point = (left + width // 2, top + height // 2)
        menu_action(session, point, 5)
        session.screenshot('normal-window')
        windows = session.wait(lambda: [window for window in session.windows()
                                      if window['caption'] == 'Pinned Mark Shot'], '普通贴图窗口没有创建')
        window = windows[0]
        point = (window['x'] + window['width'] // 2, window['y'] + window['height'] // 2)
        session.click(*point)
        copy_image(session, source, 'normal-copy')
        menu_action(session, point, 5)
        session.screenshot('top-restored')
        assert session.alive(pid), session.log('capture')
        return {'copy_pixels_preserved': True, 'rotation_preserved': True,
                'resize_and_zoom_preserved': True, 'shell_recreation_preserved': True}
    finally:
        control.close()
        session.cleanup()
        session.log('capture')


def main():
    """【贴图测试】【命令入口】读取客体身份、二进制与证据目录，执行真实界面回归，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument('--domain', default='arch-kde')
    parser.add_argument('--host', default='snemc@192.168.122.214')
    parser.add_argument('--binary', required=True)
    parser.add_argument('--root', type=Path, required=True)
    options = parser.parse_args()
    session = Session(options.domain, options.host, options.root)
    result = run(session, options.binary)
    (session.root / 'result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
