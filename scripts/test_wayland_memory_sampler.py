"""【内存对照回归】【缓存统计】验证原件文件页去重和后台采样错误传播"""

import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

from wayland_memory_metrics import MemorySampler


class MemorySamplerTests(unittest.TestCase):
    """【内存对照回归】【真实统计入口】从采样边界验证物理页累计与错误传播"""

    def test_original_file_cache_is_added_without_counting_mapped_pages_twice(self):
        """【内存对照回归】【物理去重】无参数，真实采样入口接收包含映射和额外缓存的记录，仅累计未映射原件页，无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            sampler = MemorySampler(SimpleNamespace(pid=42), Path(directory) / "samples.jsonl")
            process = dict(pid=42, start_ticks=11, rss_kib=140, pss_kib=120, private_kib=110,
                           hwm_kib=160, threads=1, fds=3, cpu_ticks=4)
            source = dict(device=57, inode=11, resident_file_kib=30)
            mapping = dict(path="/cache/mark-shot/image-source/source-1/image", rss_kib=10, pss_kib=10,
                           private_dirty_kib=10, private_clean_kib=0, device=57, inode=11)
            rest = dict(path="[heap]", rss_kib=90, pss_kib=70, private_dirty_kib=60,
                        private_clean_kib=0, device=0, inode=0)
            with patch("wayland_memory_metrics.descendants", return_value={42}), \
                    patch("wayland_memory_metrics.process_sample", return_value=process), \
                    patch("wayland_memory_metrics.original_source_files", return_value=[source]), \
                    patch("wayland_memory_metrics.memory_mappings", return_value=[mapping, rest]):
                sample = sampler._sample()
            sampler.samples = [sample]
            # 1. 【内存对照回归】【应用合计】同次映射中的RSS为100KiB，不能沿用回收前的140KiB，只添加剩余20KiB原件页
            self.assertEqual(sample["rss_kib"], 100)
            self.assertEqual(sample["rss_with_unmapped_original_kib"], 120)
            self.assertEqual(sample["pss_with_unmapped_original_kib"], 100)
            self.assertEqual(sampler.report()["peak_original_file_resident_kib"], 30)
            self.assertEqual(sampler.report()["peak_rss_with_unmapped_original_kib"], 120)

    def test_source_versions_are_accounted_for_separately(self):
        """【内存对照回归】【版本隔离】无参数，不同文件的映射与驻留变化分别去重，禁止聚合后互相抵消，无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            sampler = MemorySampler(SimpleNamespace(pid=42), Path(directory) / "samples.jsonl")
            process = dict(pid=42, start_ticks=11, rss_kib=140, pss_kib=120, private_kib=110,
                           hwm_kib=160, threads=1, fds=3, cpu_ticks=4)
            sources = [dict(device=57, inode=11, resident_file_kib=30),
                       dict(device=57, inode=12, resident_file_kib=5)]
            mappings = [dict(path="/cache/mark-shot/frozen-source/first/file", device=57, inode=11,
                             rss_kib=10, pss_kib=10, private_dirty_kib=10, private_clean_kib=0),
                        dict(path="/cache/mark-shot/frozen-source/second/file", device=57, inode=12,
                             rss_kib=7, pss_kib=7, private_dirty_kib=7, private_clean_kib=0)]
            with patch("wayland_memory_metrics.descendants", return_value={42}), \
                    patch("wayland_memory_metrics.process_sample", return_value=process), \
                    patch("wayland_memory_metrics.original_source_files", return_value=sources), \
                    patch("wayland_memory_metrics.memory_mappings", return_value=mappings):
                sample = sampler._sample()
            self.assertEqual(sample["rss_with_unmapped_original_kib"], 37)

    def test_unmapped_native_pool_pages_are_counted_outside_the_original_cache_budget(self):
        """【内存对照回归】【原生共享页】无参数，共享池重新映射后的额外驻留计入总量，原件预算保持独立，无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            sampler = MemorySampler(SimpleNamespace(pid=42), Path(directory) / "samples.jsonl")
            process = dict(pid=42, start_ticks=11, rss_kib=140, pss_kib=120, private_kib=110,
                           hwm_kib=160, threads=1, fds=3, cpu_ticks=4)
            source = dict(device=1, inode=11, resident_file_kib=100, kind="native")
            mappings = [dict(path="/memfd:smithay-client-toolkit (deleted)", rss_kib=40, pss_kib=20,
                             private_dirty_kib=0, private_clean_kib=0, device=1, inode=11),
                        dict(path="[heap]", rss_kib=60, pss_kib=60, private_dirty_kib=60,
                             private_clean_kib=0, device=0, inode=0)]
            with patch("wayland_memory_metrics.descendants", return_value={42}), \
                    patch("wayland_memory_metrics.process_sample", return_value=process), \
                    patch("wayland_memory_metrics.original_source_files", return_value=[source]), \
                    patch("wayland_memory_metrics.memory_mappings", return_value=mappings):
                sample = sampler._sample()
            self.assertEqual(sample["rss_with_unmapped_original_kib"], 160)
            self.assertEqual(sample["pss_with_unmapped_original_kib"], 140)
            self.assertEqual(sample["native_file_resident_kib"], 100)
            self.assertEqual(sample["native_unmapped_resident_kib"], 60)
            self.assertEqual(sample["original_file_resident_kib"], 0)

    def test_shared_native_files_are_not_added_twice_across_parent_and_child(self):
        """【内存对照回归】【进程共享去重】无参数，父子进程同时持有同一共享文件时仅累计一次额外物理驻留，无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            sampler = MemorySampler(SimpleNamespace(pid=42), Path(directory) / "samples.jsonl")
            process = dict(start_ticks=11, rss_kib=40, pss_kib=35, private_kib=30,
                           hwm_kib=60, threads=1, fds=3, cpu_ticks=4)
            source = dict(device=1, inode=11, resident_file_kib=50, kind="native")
            mappings = [dict(path="/memfd:wayland-shm (deleted)", rss_kib=10, pss_kib=5,
                             private_dirty_kib=0, private_clean_kib=0, device=1, inode=11),
                        dict(path="[heap]", rss_kib=30, pss_kib=30, private_dirty_kib=30,
                             private_clean_kib=0, device=0, inode=0)]
            with patch("wayland_memory_metrics.descendants", return_value={42, 43}), \
                    patch("wayland_memory_metrics.process_sample", side_effect=lambda pid: dict(process, pid=pid)), \
                    patch("wayland_memory_metrics.original_source_files", return_value=[source]), \
                    patch("wayland_memory_metrics.memory_mappings", return_value=mappings):
                sample = sampler._sample()
            self.assertEqual(sample["rss_with_unmapped_original_kib"], 110)
            self.assertEqual(sample["pss_with_unmapped_original_kib"], 100)
            self.assertEqual(sample["native_file_resident_kib"], 50)
            self.assertEqual(sample["native_unmapped_resident_kib"], 30)

    def test_sampling_errors_reach_the_calling_thread(self):
        """【内存对照回归】【异常传播】无参数，后台驻留查询失败必须使上下文失败，禁止使用残缺样本宣称测量通过，无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            sampler = MemorySampler(SimpleNamespace(pid=42), Path(directory) / "samples.jsonl")
            with patch.object(sampler, "_sample", side_effect=OSError("residency unavailable")):
                with self.assertRaisesRegex(RuntimeError, "memory sampling failed"):
                    with sampler:
                        sampler.thread.join(timeout=2)
            self.assertIsInstance(sampler.error, OSError)
            self.assertEqual(sampler.samples, [])
            self.assertEqual((Path(directory) / "samples.jsonl").read_text(), "")

    def test_samples_keep_the_native_file_evidence(self):
        """【内存对照回归】【证据持久化】无参数，序列化样本必须保留实际原件驻留和文件身份，返回值为空"""
        with tempfile.TemporaryDirectory() as directory:
            sampler = MemorySampler(SimpleNamespace(pid=42), Path(directory) / "samples.jsonl")
            source = {"fd": 3, "bytes": 4096, "resident_file_kib": 4}
            record = {"seconds": 0, "phase": "handoff", "processes": [{"original_source_files": [source]}]}
            with patch.object(sampler, "_sample", return_value=record):
                with sampler:
                    sampler.stop_event.wait(0.05)
            saved = json.loads((Path(directory) / "samples.jsonl").read_text().splitlines()[0])
            self.assertEqual(saved["processes"][0]["original_source_files"], [source])


if __name__ == "__main__":
    unittest.main()
