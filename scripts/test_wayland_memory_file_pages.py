"""【内存对照回归】【文件页】使用真实磁盘文件验证驻留查询不重新读取已回收像素"""

import os
from pathlib import Path
import tempfile
import unittest

from wayland_memory_file_pages import resident_file_kib


class FilePageTests(unittest.TestCase):
    """【内存对照回归】【真实介质】验证物理页查询与异常资源回收"""

    def test_reclaimed_file_is_not_read_back_by_the_query(self):
        """【内存对照回归】【读取隔离】无参数，真实写入4MiB并回收，再次查询不得使文件驻留，无返回值"""
        root = Path(__file__).resolve().parents[1] / "target/memory-file-page-tests"
        root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryFile(dir=root) as source:
            source.write(b"\x35" * (4 * 1024 * 1024))
            source.flush()
            os.fsync(source.fileno())
            path = f"/proc/self/fd/{source.fileno()}"
            self.assertEqual(resident_file_kib(path, 4 * 1024 * 1024), 4096)
            os.posix_fadvise(source.fileno(), 0, 0, os.POSIX_FADV_DONTNEED)
            self.assertEqual(resident_file_kib(path, 4 * 1024 * 1024), 0)
            self.assertEqual(resident_file_kib(path, 4 * 1024 * 1024), 0)

    def test_missing_file_reports_an_error(self):
        """【内存对照回归】【缺失文件】无参数，文件不可用时必须报告错误，不能以零页假装测量成功，无返回值"""
        with self.assertRaises(FileNotFoundError):
            resident_file_kib("/proc/self/fd/nonexistent", 4096)


if __name__ == "__main__":
    unittest.main()
