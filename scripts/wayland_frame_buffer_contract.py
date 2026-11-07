"""【原生缓冲验证】【释放契约】按实际协议提交检查普通绘制缓冲是否在服务端释放前重复提交"""

import re


def inspect(text, minimum_pixels=1024 * 1024):
    """【原生缓冲验证】【日志检查】按text与minimum_pixels检查完整绘制缓冲，返回重复活动提交及每表面缓冲数量，不把帧回调当作释放"""
    buffers, pending, active, surfaces, violations = {}, {}, set(), {}, []
    for number, line in enumerate(text.splitlines(), 1):
        created = re.search(r"create_buffer\((?:new id )?wl_buffer([@#]\d+),\s*\d+,\s*(\d+),\s*(\d+),", line)
        if created:
            key, width, height = created.groups()
            if int(width) * int(height) >= minimum_pixels:
                buffers[key] = (int(width), int(height))
        released = re.search(r"wl_buffer([@#]\d+)\.release\b", line)
        if released:
            active.discard(released.group(1))
        attached = re.search(r"wl_surface([@#]\d+)\.attach\(wl_buffer([@#]\d+),", line)
        if attached and attached.group(2) in buffers:
            pending[attached.group(1)] = attached.group(2)
        committed = re.search(r"wl_surface([@#]\d+)\.commit\(\)", line)
        if committed and committed.group(1) in pending:
            surface = committed.group(1)
            buffer = pending.pop(surface)
            if buffer in active:
                violations.append({"line": number, "surface": surface, "buffer": buffer})
            active.add(buffer)
            surfaces.setdefault(surface, set()).add(buffer)
    counts = {surface: len(owned) for surface, owned in surfaces.items()}
    return {"safe": bool(surfaces) and not violations, "violations": violations,
            "buffers_per_surface": counts, "maximum_buffers_per_surface": max(counts.values(), default=0)}
