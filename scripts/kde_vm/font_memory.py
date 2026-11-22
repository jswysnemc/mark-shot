"""【KDE虚拟机】【字体内存】在相同界面比较系统字体支持前后资源，原图和窗口动作保持一致"""

import argparse
import json
from pathlib import Path
import statistics
import subprocess
import time

from PIL import Image

from pin_performance import CONFIG, awake, sample
from session import Session


def prepare(session, language):
    """【字体测试】【环境准备】按 session、language 创建隔离配置和采样依赖，无返回值，不改用户字体配置"""
    session.setup(CONFIG + f'[ui]\nlanguage="{language}"\ntheme="light"\n')
    scripts = Path(__file__).resolve().parent.parent
    source = session.root / 'source.png'
    Image.new('RGB', (800, 500), (32, 36, 40)).save(source)
    subprocess.run(['scp', '-q', str(scripts / 'wayland_memory_metrics.py'),
                    str(scripts / 'wayland_memory_file_pages.py'), str(source),
                    session.host + ':' + session.remote + '/'], check=True, timeout=20)
    awake(session)


def scenario(session, binary, case, language):
    """【字体测试】【相同界面】按 session、binary、case、language 测量稳定界面，返回五次资源采样中位数，结束时回收派生进程"""
    prepare(session, language)
    source = session.remote + '/source.png'
    args = ['--settings'] if case == 'settings' else [source] if case == 'editor' else ['--pin-image', source]
    pid = session.launch(binary, args)
    try:
        time.sleep(1)
        assert session.alive(pid), session.log('capture')
        if case == 'pin-menu':
            session.click(session.size[0] // 2, session.size[1] // 2, 'right')
            time.sleep(.25)
        elif case == 'editor':
            session.move(1048, 142)
            time.sleep(.8)
        session.screenshot('visible')
        records = []
        for _ in range(5):
            records.append(sample(session, pid))
            time.sleep(.1)
        return {key: statistics.median(record[key] for record in records)
                for key in records[0] if key != 'cpu_seconds'}
    finally:
        session.cleanup()
        session.log('capture')


def main():
    """【字体测试】【对比入口】读取两版程序、虚拟机和证据目录，各场景交替重复三次，返回值为空，保存完整样本"""
    parser = argparse.ArgumentParser()
    parser.add_argument('--before', required=True)
    parser.add_argument('--after', required=True)
    parser.add_argument('--domain', default='arch-kde')
    parser.add_argument('--host', default='snemc@192.168.122.214')
    parser.add_argument('--root', type=Path, required=True)
    options = parser.parse_args()
    options.root.mkdir(parents=True, exist_ok=True)
    records, summary = [], []
    for language in ['english', 'chinese']:
        for case in ['editor', 'settings', 'pin-menu']:
            groups = {'before': [], 'after': []}
            for repeat in range(3):
                order = ['before', 'after'] if repeat % 2 == 0 else ['after', 'before']
                for version in order:
                    root = options.root / f'{language}-{case}-{repeat}-{version}'
                    session = Session(options.domain, options.host, root)
                    record = scenario(session, getattr(options, version), case, language)
                    groups[version].append(record)
                    records.append({'language': language, 'case': case, 'repeat': repeat, 'version': version, **record})
                    (options.root / 'samples.json').write_text(json.dumps(records, indent=2) + '\n')
            medians = {version: {key: statistics.median(record[key] for record in group)
                                  for key in group[0]} for version, group in groups.items()}
            result = {'language': language, 'case': case, **medians,
                      'delta_kib': {key: medians['after'][key] - medians['before'][key] for key in medians['before']}}
            summary.append(result)
            print(json.dumps(result), flush=True)
            (options.root / 'result.json').write_text(json.dumps(summary, indent=2) + '\n')


if __name__ == '__main__':
    main()
