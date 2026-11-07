"""【原生缓冲验证测试】【提交顺序】覆盖真实Rust和Qt日志形式、释放、重新附着及仅提交元数据"""

import unittest

from wayland_frame_buffer_contract import inspect


class BufferContractTests(unittest.TestCase):
    """【原生缓冲验证测试】【契约集合】在独立文本中保留真实缓冲尺寸和协议事件"""

    def test_frame_done_does_not_release_an_active_buffer(self):
        """【原生缓冲验证测试】【帧与释放】无参数，帧回调后重复提交同一活动缓冲必须失败，无返回值"""
        result = inspect("""
wl_shm_pool@1.create_buffer(wl_buffer@2, 0, 3840, 2160, 15360, 0)
wl_surface@3.attach(wl_buffer@2, 0, 0)
wl_surface@3.commit()
wl_callback@4.done, (100)
wl_surface@3.attach(wl_buffer@2, 0, 0)
wl_surface@3.commit()
""")
        self.assertFalse(result["safe"])
        self.assertEqual(len(result["violations"]), 1)

    def test_two_buffers_alternate_only_after_release(self):
        """【原生缓冲验证测试】【交替提交】无参数，两缓冲交替并真实释放后复用必须通过，无返回值"""
        result = inspect("""
wl_shm_pool@1.create_buffer(wl_buffer@2, 0, 3840, 2160, 15360, 0)
wl_shm_pool@1.create_buffer(wl_buffer@5, 33177600, 3840, 2160, 15360, 0)
wl_surface@3.attach(wl_buffer@2, 0, 0)
wl_surface@3.commit()
wl_surface@3.attach(wl_buffer@5, 0, 0)
wl_surface@3.commit()
wl_buffer@2.release, ()
wl_surface@3.attach(wl_buffer@2, 0, 0)
wl_surface@3.commit()
""")
        self.assertTrue(result["safe"])
        self.assertEqual(result["maximum_buffers_per_surface"], 2)

    def test_metadata_commits_and_probe_buffers_are_excluded(self):
        """【原生缓冲验证测试】【无像素提交】无参数，仅提交元数据和小探测缓冲不会产生重复活动帧，无返回值"""
        result = inspect("""
wl_shm_pool@1.create_buffer(wl_buffer@2, 0, 3840, 2160, 15360, 0)
wl_shm_pool@1.create_buffer(wl_buffer@5, 33177600, 1, 1, 4, 0)
wl_surface@3.attach(wl_buffer@2, 0, 0)
wl_surface@3.commit()
wl_surface@3.commit()
wl_surface@6.attach(wl_buffer@5, 0, 0)
wl_surface@6.commit()
wl_surface@6.attach(wl_buffer@5, 0, 0)
wl_surface@6.commit()
""")
        self.assertTrue(result["safe"])
        self.assertEqual(result["buffers_per_surface"], {"@3": 1})


if __name__ == "__main__":
    unittest.main()
