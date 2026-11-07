"""【内存对照】【文件驻留】查询原件文件的物理缓存页，不读取或重新驻留原件像素"""

import ctypes
import os


def resident_file_kib(path, length):
    """【内存对照】【原件驻留】按path与length查询Linux文件缓存，返回KiB；系统错误抛出异常，不访问图像字节"""
    if length <= 0:
        return 0
    native = ctypes.CDLL(None, use_errno=True)
    native.mmap.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_int,
                           ctypes.c_int, ctypes.c_int, ctypes.c_long]
    native.mmap.restype = ctypes.c_void_p
    native.mincore.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p]
    native.mincore.restype = ctypes.c_int
    native.munmap.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    native.munmap.restype = ctypes.c_int
    descriptor = os.open(path, os.O_RDONLY)
    try:
        # 1. 【内存对照】【查询映射】只建立只读共享地址映射，不读取原件，不触发文件页缺页
        address = native.mmap(None, length, 1, 1, descriptor, 0)
        if address == ctypes.c_void_p(-1).value:
            number = ctypes.get_errno()
            raise OSError(number, os.strerror(number))
        try:
            page = os.sysconf("SC_PAGE_SIZE")
            flags = (ctypes.c_ubyte * ((length + page - 1) // page))()
            # 2. 【内存对照】【物理查询】mincore只写驻留标志，文件缓存和客户端RSS分别记录
            if native.mincore(address, length, flags) != 0:
                number = ctypes.get_errno()
                raise OSError(number, os.strerror(number))
            return sum(bool(value & 1) for value in flags) * page // 1024
        finally:
            native.munmap(address, length)
    finally:
        os.close(descriptor)
