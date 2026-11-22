"""【KDE虚拟机】【贴图性能】用固定输入对比实际帧提交、CPU 和内存，原图复制另行验证"""

import argparse
import json
from pathlib import Path
import re
import statistics
import subprocess
import time

import libvirt
import libvirt_qemu

from session import Session

CONFIG = '''[general]
to=""
[desktop]
tray_enabled=false
shortcuts_enabled=false
[notifications]
enabled=false
[history]
enabled=false
[pinned]
auto_text_selection=false
always_on_top=true
'''


def sample(session, pid):
    """【贴图测试】【资源读取】按 session、pid 返回客体进程 CPU 秒数及驻留内存，失败直接报告，不读取图像内容"""
    return json.loads(session.guest("""
import sys
sys.path.insert(0,values['root'])
from wayland_memory_metrics import process_sample,original_source_files,memory_mappings,source_kind
pid=values['pid'];memory=process_sample(pid)
files=original_source_files(pid)
resident=sum(file['resident_file_kib'] for file in files if file['kind']=='native')
mapped=sum(mapping['rss_kib'] for mapping in memory_mappings(pid,complete=True) if source_kind(mapping['path'])=='native')
extra=max(0,resident-mapped)
print(json.dumps({'cpu_seconds':memory['cpu_ticks']/os.sysconf('SC_CLK_TCK'),
 'rss_kib':memory['rss_kib'],'pss_kib':memory['pss_kib'],'private_kib':memory['private_kib'],
 'shm_resident_kib':resident,'shm_mapped_kib':mapped,
 'rss_with_unmapped_shm_kib':memory['rss_kib']+extra,
 'pss_with_unmapped_shm_kib':memory['pss_kib']+extra}))
""", {'pid': pid, 'root': session.remote}))


class Input:
    """【贴图测试】【连续输入】复用 libvirt 连接，以固定频率发送真实鼠标事件，不启动每步辅助进程"""

    def __init__(self, session):
        """【贴图测试】【连接创建】按 session 绑定单个测试虚拟机，返回值为空"""
        self.connection = libvirt.open('qemu:///system')
        self.domain = self.connection.lookupByName(session.domain)
        self.size = session.size

    def send(self, events):
        """【贴图测试】【事件发送】按 events 交付 QMP 输入数组，无返回值，协议错误停止测试"""
        response = json.loads(libvirt_qemu.qemuMonitorCommand(
            self.domain, json.dumps({'execute': 'input-send-event', 'arguments': {'events': events}}), 0))
        assert 'error' not in response, response

    def move(self, x, y):
        """【贴图测试】【指针定位】按 x、y 客体像素坐标发送绝对位置，无返回值"""
        self.send([{'type': 'abs', 'data': {'axis': axis, 'value': round(value * 32767 / (extent - 1))}}
                   for axis, value, extent in zip(('x', 'y'), (x, y), self.size)])

    def button(self, button, down):
        """【贴图测试】【按钮状态】按 button、down 发送按键状态，无返回值"""
        self.send([{'type': 'btn', 'data': {'button': button, 'down': down}}])

    def drag(self, start, delta, steps=120):
        """【贴图测试】【匀速拖动】从 start 沿 delta 分 steps 步输入，每秒 120 步，最终释放左键，无返回值"""
        self.move(*start)
        time.sleep(.1)
        self.button('left', True)
        began = time.monotonic()
        try:
            for index in range(1, steps + 1):
                deadline = began + index / 120
                time.sleep(max(0, deadline - time.monotonic()))
                self.move(*(start[axis] + delta[axis] * index / steps for axis in range(2)))
        finally:
            self.button('left', False)

    def zoom(self, steps=12):
        """【贴图测试】【往返缩放】按 steps 发送放大再缩小的真实滚轮，固定每秒 30 个刻度，无返回值"""
        for button in ['wheel-up'] * steps + ['wheel-down'] * steps:
            self.button(button, True)
            self.button(button, False)
            time.sleep(1 / 30)

    def close(self):
        """【贴图测试】【连接释放】无参数，关闭本次 libvirt 连接，无返回值"""
        self.connection.close()


def frame_metrics(log):
    """【贴图测试】【提交统计】按 log 只统计带帧回调的图像提交，返回各角色帧间隔，排除仅修改几何的 commit"""
    names = {}
    for surface, role in re.findall(r'get_layer_surface\(.*?, wl_surface@(\d+),.*?Some\("(mark-shot-pin(?:-preview)?)"\)', log):
        names[surface] = role
    result = {}
    for surface, role in names.items():
        times = [float(value) for value in re.findall(r'\[\s*([\d.]+)\].* -> wl_surface@' + surface + r'\.frame\(', log)]
        intervals = [right - left for left, right in zip(times, times[1:]) if right - left < 200]
        result[role] = {'frames': len(times), 'intervals': len(intervals),
                        'median_ms': statistics.median(intervals) if intervals else None,
                        'p95_ms': sorted(intervals)[min(len(intervals) - 1, int(len(intervals) * .95))] if intervals else None}
    assert result and all(item['frames'] > 0 for item in result.values()), '日志没有实际贴图帧'
    return result


def run(session, binary, source):
    """【贴图测试】【对照执行】按 session、binary、source 测量相同移动和缩放，返回资源及帧指标；结束后回收本次进程"""
    session.setup(CONFIG)
    scripts = Path(__file__).resolve().parent.parent
    subprocess.run(['scp', '-q', str(scripts / 'wayland_memory_metrics.py'),
                    str(scripts / 'wayland_memory_file_pages.py'), session.host + ':' + session.remote + '/'],
                   check=True, timeout=20)
    session.guest("""
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
subprocess.run(['kscreen-doctor','--dpms','on'],env=environment,check=True,capture_output=True)
subprocess.run(['sudo','-n','loginctl','unlock-session',environment['XDG_SESSION_ID']],check=True,capture_output=True)
""")
    pid = session.launch(binary, ['--pin-image', source], overrides={'WAYLAND_DEBUG': '1'})
    control = Input(session)
    try:
        time.sleep(1)
        assert session.alive(pid), session.log('capture')
        session.screenshot('initial')
        before = sample(session, pid)
        began = time.monotonic()
        control.drag((500, 400), (100, 20))
        time.sleep(.25)
        session.screenshot('moved')
        control.zoom()
        time.sleep(.25)
        elapsed = time.monotonic() - began
        after = sample(session, pid)
        session.screenshot('zoomed')
        idle_before = sample(session, pid)
        time.sleep(1)
        idle_after = sample(session, pid)
        log = session.log('capture')
        assert session.alive(pid), log[-2000:]
        motion = len(re.findall(r'<- wl_pointer@\d+\.motion', log))
        axis = len(re.findall(r'<- wl_pointer@\d+\.axis,', log))
        assert motion >= 60 and axis >= 20, '真实鼠标输入未到达贴图，性能样本无效'
        return {'binary': binary, 'cpu_seconds': round(after['cpu_seconds'] - before['cpu_seconds'], 3),
                'elapsed_seconds': round(elapsed, 3), 'initial_memory': before, 'final_memory': after,
                'idle_cpu_seconds': round(idle_after['cpu_seconds'] - idle_before['cpu_seconds'], 3),
                'frames': frame_metrics(log),
                'motion_events': motion, 'axis_events': axis}
    finally:
        control.close()
        session.cleanup()
        session.log('capture')


def main():
    """【贴图测试】【命令入口】读取目标虚拟机、二进制与证据目录，执行可复现对照，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument('--domain', default='arch-kde')
    parser.add_argument('--host', default='snemc@192.168.122.214')
    parser.add_argument('--binary', required=True)
    parser.add_argument('--source', default='/tmp/mark-shot-pin-perf-source.png')
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    options = parser.parse_args()
    session = Session(options.domain, options.host, options.root)
    result = run(session, options.binary, options.source)
    (session.root / 'result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result, ensure_ascii=False))
    if options.check:
        preview = result['frames']['mark-shot-pin-preview']
        assert preview['median_ms'] <= 18 and preview['frames'] >= 50, '移动帧未达到性能要求'
        assert result['idle_cpu_seconds'] <= .02, '空闲 CPU 超出性能要求'


if __name__ == '__main__':
    main()
