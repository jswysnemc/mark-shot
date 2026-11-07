"""【删除悬停回归】【真实像素】核对原版扩大、提亮、边框及指针离开后的完整恢复"""

from PIL import ImageChops


def check(flow, root, selected, point, region):
    """【删除悬停回归】【往返与颜色】按flow实际移动point并观察root画面，比较selected的region，返回验证报告，不点击按钮"""
    flow.INPUT.move(point)
    hovered = flow.snapshot(root, "delete-hover")
    button = (point[0] - 16, point[1] - 16, point[0] + 16, point[1] + 16)
    bright = flow.pixels(hovered, button, (248, 113, 113))
    normal = flow.pixels(selected, button, (239, 68, 68))
    # 1. 【删除悬停回归】【同色隔离】原图标注也可能为红色，只统计悬停时发生变化的按钮像素
    normal = [(x, y) for x, y in normal if selected.getpixel((x, y)) != hovered.getpixel((x, y))]
    assert len(normal) >= 50 and len(bright) >= 80, "delete hover did not replace the original red disc with its highlighted color"
    assert len(bright) > len(normal) * 1.4, "delete hover did not brighten and expand its original red disc"
    normal_width = max(x for x, y in normal) - min(x for x, y in normal) + 1
    hover_width = max(x for x, y in bright) - min(x for x, y in bright) + 1
    assert hover_width >= normal_width + 3, "delete hover did not expand by the reference radius"
    # 1. 【删除悬停回归】【原版边框】半透明粉色边框不能继续使用纯白描边，排除叉号和红色内部
    border = sum(1 for r, g, b in hovered.crop(button).getdata()
                 if r > 200 and 145 < g < 225 and 145 < b < 225 and abs(g - b) < 12)
    assert border >= 8, "delete hover retained the old white border"
    flow.INPUT.move((230, 170))
    restored = flow.snapshot(root, "delete-hover-restored")
    assert ImageChops.difference(selected.crop(region), restored.crop(region)).getbbox() is None, "delete hover left enlarged edge residue"
    # 2. 【删除悬停回归】【重复进入】重新进入得到相同像素，不能不断叠加透明边框
    flow.INPUT.move(point)
    repeated = flow.snapshot(root, "delete-hover-repeated")
    assert ImageChops.difference(hovered.crop(region), repeated.crop(region)).getbbox() is None, "delete hover accumulated translucent edges"
    return {"normal_pixels": len(normal), "hover_pixels": len(bright),
            "normal_width": normal_width, "hover_width": hover_width,
            "pink_border_pixels": border, "leave_exact_restore": True, "repeat_exact": True}
