"""【KDE虚拟机】【字体规则】隔离配置改变真实设置页字体与抗锯齿，保留完整画面和差异证据"""

import argparse
import json
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops

from font_memory import prepare
from session import Session


def capture(session, binary, mode):
    """【字体测试】【规则画面】按 session、binary、mode 创建隔离 Fontconfig 并拍摄稳定设置页，返回窗口图像，结束后回收进程"""
    # 1. 【字体测试】【隔离配置】固定语言并生成对应字体家族与抗锯齿规则
    prepare(session, 'english')
    family = 'DejaVu Serif' if mode == 'serif' else 'DejaVu Sans'
    antialias = 'false' if mode == 'mono' else 'true'
    config = f'''<?xml version="1.0"?><!DOCTYPE fontconfig SYSTEM "fonts.dtd"><fontconfig>
      <dir>/usr/share/fonts</dir><cachedir>{session.remote}/cache/fonts</cachedir>
      <alias binding="strong"><family>sans-serif</family><prefer><family>{family}</family></prefer></alias>
      <match target="font"><edit name="antialias" mode="assign"><bool>{antialias}</bool></edit>
        <edit name="rgba" mode="assign"><const>none</const></edit>
        <edit name="hintstyle" mode="assign"><const>hintfull</const></edit></match></fontconfig>'''
    path = session.root / 'fonts.conf'
    path.write_text(config)
    remote = session.remote + '/fonts.conf'
    subprocess.run(['scp', '-q', str(path), session.host + ':' + remote], check=True, timeout=20)
    # 2. 【字体测试】【启动设置】仅向本次进程传入隔离规则，定位实际设置窗口
    pid = session.launch(binary, ['--settings'], overrides={'FONTCONFIG_FILE': remote})
    try:
        time.sleep(1)
        assert session.alive(pid), session.log('capture')
        windows = session.wait(lambda: session.windows('mark-shot-settings'), '设置窗口未创建')
        window = windows[0]
        session.move(2, 2)
        box = tuple(round(value) for value in (window['x'], window['y'], window['x'] + window['width'], window['y'] + window['height']))
        # 3. 【字体测试】【稳定画面】连续两帧相同后保存，排除动画与光标闪烁干扰
        previous = None
        for index in range(8):
            time.sleep(.25)
            frame = Image.open(session.screenshot(f'frame-{index}')).convert('RGB').crop(box)
            if previous is not None and frame.tobytes() == previous.tobytes():
                frame.save(session.root / 'window.png')
                return frame
            previous = frame
        raise AssertionError('设置页没有达到稳定画面')
    finally:
        # 4. 【字体测试】【回收进程】失败或完成时均退出本次进程并保存日志
        session.cleanup()
        session.log('capture')


def run(options):
    """【字体测试】【真实差异】按 options 对比灰度、单色和另一家族，返回差异像素数量，配置未应用时失败"""
    # 1. 【字体测试】【分组采集】分别启动三种隔离规则并采集稳定设置页
    frames = {}
    for mode in ['gray', 'mono', 'serif']:
        session = Session(options.domain, options.host, options.root / mode)
        frames[mode] = capture(session, options.binary, mode)
    result = {}
    for mode in ['mono', 'serif']:
        # 2. 【字体测试】【像素对照】固定窗口尺寸，保存相对灰度基线的差异图
        assert frames['gray'].size == frames[mode].size, '界面窗口尺寸不一致'
        difference = ImageChops.difference(frames['gray'], frames[mode])
        difference.save(options.root / (mode + '-difference.png'))
        count = sum(pixel != (0, 0, 0) for pixel in difference.get_flattened_data())
        # 3. 【字体测试】【规则断言】配置必须改变实际文字像素，返回各模式差异数量
        assert count >= 500, f'{mode}: Fontconfig 没有改变实际文字，差异像素={count}'
        result[mode + '_changed_pixels'] = count
    return result


def main():
    """【字体测试】【命令入口】读取虚拟机、二进制与证据目录，保存实际规则验证结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--domain', default='arch-kde')
    parser.add_argument('--host', default='snemc@192.168.122.214')
    parser.add_argument('--root', type=Path, required=True)
    options = parser.parse_args()
    options.root.mkdir(parents=True, exist_ok=True)
    result = run(options)
    (options.root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
