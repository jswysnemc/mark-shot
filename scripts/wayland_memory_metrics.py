"""【内存对照】【进程采样】记录指定进程及子进程的实际资源，不计入测试合成器"""

import json
import os
from pathlib import Path
import statistics
import threading
import time

from wayland_memory_file_pages import resident_file_kib


def source_kind(path):
    """【内存对照】【来源身份】按path区分图片原件、冻结快照和两版原生共享文件，返回身份或空值，未知文件不纳入物理统计"""
    if "/mark-shot/image-source/" in path:
        return "original"
    if "/mark-shot/frozen-source/" in path:
        return "frozen"
    if path.startswith(("/memfd:smithay-client-toolkit", "/memfd:wayland-shm")):
        return "native"
    return None


def process_sample(pid):
    """【内存对照】【单进程】读取 pid 的内存、线程、CPU 和描述符；返回记录，已退出时返回 None"""
    root = Path(f"/proc/{pid}")
    try:
        status = {}
        for line in (root / "status").read_text().splitlines():
            key, value = line.split(":", 1)
            status[key] = value.strip()
        smaps = {}
        for line in (root / "smaps_rollup").read_text().splitlines()[1:]:
            key, value = line.split(":", 1)
            smaps[key] = int(value.split()[0])
        stat = (root / "stat").read_text().rsplit(")", 1)[1].split()
        return {"pid": pid, "rss_kib": smaps["Rss"], "pss_kib": smaps["Pss"],
                "private_kib": smaps["Private_Clean"] + smaps["Private_Dirty"],
                "hwm_kib": int(status.get("VmHWM", "0 kB").split()[0]),
                "threads": int(status["Threads"]), "fds": len(list((root / "fd").iterdir())),
                "cpu_ticks": int(stat[11]) + int(stat[12]), "start_ticks": int(stat[19])}
    except (FileNotFoundError, ProcessLookupError, PermissionError):
        return None


def descendants(pid):
    """【内存对照】【进程树】读取 pid 各线程所属子进程，返回包含根进程的 PID 集合"""
    pending, result = [pid], set()
    while pending:
        current = pending.pop()
        if current in result:
            continue
        result.add(current)
        for task in Path(f"/proc/{current}/task").glob("*/children"):
            try:
                pending.extend(map(int, task.read_text().split()))
            except FileNotFoundError:
                continue
    return result


def memory_mappings(pid, complete=False):
    """【内存对照】【映射明细】按pid读取smaps，complete返回全部映射，否则只保留大映射和来源；已退出时返回空列表"""
    mappings, current = [], None
    try:
        lines = Path(f"/proc/{pid}/smaps").read_text().splitlines()
    except (FileNotFoundError, ProcessLookupError):
        return []
    for line in lines:
        parts = line.split()
        if len(parts) >= 5 and "-" in parts[0]:
            major, minor = parts[3].split(":")
            current = {"range": parts[0], "permissions": parts[1], "path": " ".join(parts[5:]),
                       "device": os.makedev(int(major, 16), int(minor, 16)), "inode": int(parts[4])}
            mappings.append(current)
        elif current is not None and parts and parts[0] in ("Rss:", "Pss:", "Private_Clean:", "Private_Dirty:", "Size:"):
            current[parts[0].rstrip(":").lower() + "_kib"] = int(parts[1])
    return sorted((mapping for mapping in mappings if complete or mapping.get("rss_kib", 0) >= 1024
                   or "/memfd:smithay-client-toolkit" in mapping["path"]
                   or source_kind(mapping["path"])),
                  key=lambda mapping: mapping["rss_kib"], reverse=True)


def original_source_files(pid):
    """【内存对照】【来源文件】按pid读取实际打开的原件、冻结来源和原生共享描述符，返回去重后的文件大小和身份，不读取图像字节"""
    result, seen = [], set()
    try:
        descriptors = list(Path(f"/proc/{pid}/fd").iterdir())
    except FileNotFoundError:
        return []
    for descriptor in descriptors:
        try:
            path = os.readlink(descriptor)
            kind = source_kind(path)
            if kind is None:
                continue
            metadata = descriptor.stat()
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
        identity = (metadata.st_dev, metadata.st_ino)
        if identity in seen:
            continue
        seen.add(identity)
        try:
            resident = resident_file_kib(descriptor, metadata.st_size) if metadata.st_size else 0
        except FileNotFoundError:
            continue
        result.append({"fd": int(descriptor.name), "path": path, "bytes": metadata.st_size,
                       "device": metadata.st_dev, "inode": metadata.st_ino,
                       "resident_file_kib": resident, "kind": kind})
    return result


class MemorySampler:
    """【内存对照】【采样生命周期】测试进程使用独立采样线程，产品进程不增加线程"""

    def __init__(self, process, destination, interval=0.025):
        """【内存对照】【构造】保存 process、JSONL destination 和秒间隔；返回初始化实例"""
        self.process, self.destination, self.interval = process, destination, interval
        self.samples, self.identities = [], {}
        self.source_files = {}
        self.error = None
        self.phase, self.started = "startup", time.monotonic()
        self.stop_event = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True)

    def _sample(self):
        """【内存对照】【完整样本】无参数，返回进程树RSS和额外来源及原生文件页；同一物理页跨父子进程不重复计入"""
        records, files = [], {}
        for pid in descendants(self.process.pid) | self.identities.keys():
            sample = process_sample(pid)
            if sample is None:
                continue
            identity = self.identities.setdefault(pid, sample["start_ticks"])
            if sample["start_ticks"] != identity:
                continue
            sources = original_source_files(pid)
            file_pages = sum(source["resident_file_kib"] for source in sources if source.get("kind") != "native")
            native_pages = sum(source["resident_file_kib"] for source in sources if source.get("kind") == "native")
            mapped, native_mapped = 0, 0
            if sources:
                mappings = memory_mappings(pid, complete=True)
                if not mappings:
                    continue
                # 1. 【内存对照】【同次读取】来源映射和应用RSS来自同次smaps读取，避免回收期间使用两次不同状态重复统计原件页
                sample["rss_kib"] = sum(mapping["rss_kib"] for mapping in mappings)
                sample["pss_kib"] = sum(mapping["pss_kib"] for mapping in mappings)
                sample["private_kib"] = sum(mapping["private_dirty_kib"] + mapping["private_clean_kib"] for mapping in mappings)
                by_file = {}
                for mapping in mappings:
                    kind = source_kind(mapping["path"])
                    if kind:
                        identity = mapping["device"], mapping["inode"]
                        by_file[identity] = by_file.get(identity, 0) + mapping["rss_kib"]
                        if kind == "native":
                            native_mapped += mapping["rss_kib"]
                        else:
                            mapped += mapping["rss_kib"]
                # 2. 【内存对照】【全树去重】来源版本分别统计，父子进程共享同一文件时使用一份物理驻留和全部已计入RSS的映射
                for source in sources:
                    identity = source["device"], source["inode"]
                    entry = files.setdefault(identity, {"kind": source.get("kind", "original"), "resident": 0, "mapped": 0, "owner": len(records)})
                    entry["resident"] = max(entry["resident"], source["resident_file_kib"])
                    entry["mapped"] += by_file.get(identity, 0)
            sample.update(original_source_files=sources, original_file_resident_kib=file_pages,
                          original_mapping_resident_kib=mapped,
                          native_file_resident_kib=native_pages, native_mapping_resident_kib=native_mapped,
                          native_unmapped_resident_kib=0,
                          rss_with_unmapped_original_kib=sample["rss_kib"],
                          pss_with_unmapped_original_kib=sample["pss_kib"])
            records.append(sample)
        # 3. 【内存对照】【完整物理占用】原件预算保持独立，原生共享页仍加入总量，池扩容后的未映射驻留不能隐藏
        for entry in files.values():
            extra = max(0, entry["resident"] - entry["mapped"])
            owner = records[entry["owner"]]
            owner["rss_with_unmapped_original_kib"] += extra
            owner["pss_with_unmapped_original_kib"] += extra
            if entry["kind"] == "native":
                owner["native_unmapped_resident_kib"] += extra
        record = {"seconds": time.monotonic() - self.started, "phase": self.phase,
                  "processes": records}
        for key in ("rss_kib", "pss_kib", "private_kib", "threads", "fds", "cpu_ticks",
                    "original_file_resident_kib", "original_mapping_resident_kib",
                    "native_file_resident_kib", "native_mapping_resident_kib", "native_unmapped_resident_kib",
                    "rss_with_unmapped_original_kib", "pss_with_unmapped_original_kib"):
            record[key] = sum(item[key] for item in records)
        record["original_file_resident_kib"] = sum(entry["resident"] for entry in files.values() if entry["kind"] != "native")
        record["native_file_resident_kib"] = sum(entry["resident"] for entry in files.values() if entry["kind"] == "native")
        return record

    def _run(self):
        """【内存对照】【循环】逐次记录当前进程树和原件物理页，异常交给主线程，防止残缺样本被当作完整验证，无参数、无返回值"""
        try:
            with self.destination.open("w") as stream:
                while not self.stop_event.is_set():
                    record = self._sample()
                    self.samples.append(record)
                    stream.write(json.dumps(record) + "\n")
                    stream.flush()
                    self.stop_event.wait(self.interval)
        except Exception as error:
            self.error = error
            self.stop_event.set()

    def mark(self, phase):
        """【内存对照】【阶段】切换 phase 标签，不清空既有样本，无返回值"""
        self.phase = phase

    def mappings(self):
        """【内存对照】【阶段映射】保存主进程当前阶段的实际大映射，无参数、无返回值，不改变产品进程"""
        target = self.destination.parent / f"mappings-{self.phase}.json"
        target.write_text(json.dumps(memory_mappings(self.process.pid), indent=2) + "\n")
        sources = original_source_files(self.process.pid)
        self.source_files[self.phase] = sources
        (self.destination.parent / f"source-files-{self.phase}.json").write_text(json.dumps(sources, indent=2) + "\n")

    def report(self):
        """【内存对照】【统计】返回阶段中位数、观测峰值、内核主进程峰值和结束资源"""
        phases = {}
        for phase in sorted({sample["phase"] for sample in self.samples}):
            values = [sample for sample in self.samples if sample["phase"] == phase]
            phases[phase] = {"samples": len(values)}
            for key in ("rss_kib", "pss_kib", "private_kib", "threads", "fds",
                        "original_file_resident_kib", "original_mapping_resident_kib",
                        "native_file_resident_kib", "native_mapping_resident_kib", "native_unmapped_resident_kib",
                        "rss_with_unmapped_original_kib", "pss_with_unmapped_original_kib"):
                phases[phase][f"median_{key}"] = statistics.median(item[key] for item in values)
                phases[phase][f"peak_{key}"] = max(item[key] for item in values)
        return {"interval_seconds": self.interval, "clock_ticks": os.sysconf("SC_CLK_TCK"),
                "phases": phases, "peak_rss_kib": max(item["rss_kib"] for item in self.samples),
                "peak_pss_kib": max(item["pss_kib"] for item in self.samples),
                "peak_original_file_resident_kib": max(item["original_file_resident_kib"] for item in self.samples),
                "peak_native_file_resident_kib": max(item["native_file_resident_kib"] for item in self.samples),
                "peak_native_unmapped_resident_kib": max(item["native_unmapped_resident_kib"] for item in self.samples),
                "peak_rss_with_unmapped_original_kib": max(item["rss_with_unmapped_original_kib"] for item in self.samples),
                "peak_pss_with_unmapped_original_kib": max(item["pss_with_unmapped_original_kib"] for item in self.samples),
                "main_hwm_kib": max((item["hwm_kib"] for sample in self.samples
                                     for item in sample["processes"] if item["pid"] == self.process.pid), default=0),
                "remaining_processes": self.samples[-1]["processes"],
                "original_source_files": self.source_files}

    def __enter__(self):
        """【内存对照】【开始】启动测试采样线程并返回本实例，无参数"""
        self.thread.start()
        return self

    def __exit__(self, *_arguments):
        """【内存对照】【结束】停止并回收测试采样线程，不抑制 _arguments 异常"""
        self.stop_event.set()
        self.thread.join(timeout=3)
        if self.thread.is_alive():
            raise RuntimeError("memory sampling thread did not exit")
        if self.error is not None:
            raise RuntimeError("memory sampling failed") from self.error
