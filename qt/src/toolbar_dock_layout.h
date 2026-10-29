#pragma once

#include <QRect>
#include <QSize>

namespace markshot {

/// @brief 【截图工具栏】【停靠布局】工具栏可停靠的屏幕边缘
enum class ToolbarDockEdge {
    Bottom,
    Right,
    Top,
};

/// @brief 【截图工具栏】【停靠布局】主、副工具栏停靠屏幕边缘后的几何结果
struct DockedToolbarLayout {
    QRect toolbar;
    QRect actionToolbar;
};

/// @brief 【截图工具栏】【停靠布局】计算两条工具栏停靠在屏幕顶部、底部或右侧时的几何
///
/// 停靠顶部或底部时两条工具栏在该边居中排成一行，宽度不足时主工具栏贴边，
/// 副工具栏移到主工具栏靠屏幕内侧的一边并右对齐；
/// 停靠右侧时两条工具栏在右侧居中排成一列，高度不足时副工具栏移到主工具栏左侧并底对齐。
/// @param edge 停靠的屏幕边缘
/// @param bounds 可用区域，通常为截图窗口矩形
/// @param toolbarSize 主工具栏尺寸
/// @param actionToolbarSize 副工具栏尺寸；为空时只摆放主工具栏
/// @param edgeMargin 工具栏与屏幕边缘的距离
/// @param spacing 两条工具栏之间的间距
/// @return 两条工具栏的几何；副工具栏尺寸为空时 actionToolbar 为空矩形
DockedToolbarLayout dockedToolbarLayout(ToolbarDockEdge edge,
                                        const QRect &bounds,
                                        const QSize &toolbarSize,
                                        const QSize &actionToolbarSize,
                                        int edgeMargin,
                                        int spacing);

}  // namespace markshot
