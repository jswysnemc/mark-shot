#include "toolbar_dock_layout.h"

#include <algorithm>

namespace {

/// @brief 把矩形限制在可用区域内，区域放不下时贴靠左上
/// @param rect 待限制的矩形
/// @param bounds 可用区域
/// @param margin 与区域边缘保持的距离
/// @return 限制后的矩形，尺寸不变
QRect clampedInto(QRect rect, const QRect &bounds, int margin)
{
    const int minLeft = bounds.x() + margin;
    const int minTop = bounds.y() + margin;
    const int maxLeft = std::max(minLeft, bounds.x() + bounds.width() - margin - rect.width());
    const int maxTop = std::max(minTop, bounds.y() + bounds.height() - margin - rect.height());
    rect.moveLeft(std::clamp(rect.left(), minLeft, maxLeft));
    rect.moveTop(std::clamp(rect.top(), minTop, maxTop));
    return rect;
}

/// @brief 计算两条工具栏停靠底部时的几何
/// @param bounds 可用区域
/// @param toolbarSize 主工具栏尺寸
/// @param actionSize 副工具栏尺寸，为空时只摆放主工具栏
/// @param edgeMargin 与屏幕边缘的距离
/// @param spacing 两条工具栏之间的间距
/// @return 停靠后的几何
markshot::DockedToolbarLayout bottomDockedLayout(const QRect &bounds,
                                                 const QSize &toolbarSize,
                                                 const QSize &actionSize,
                                                 int edgeMargin,
                                                 int spacing)
{
    markshot::DockedToolbarLayout layout;
    const bool hasAction = !actionSize.isEmpty();
    const int bottom = bounds.y() + bounds.height() - edgeMargin;
    const int rowWidth = toolbarSize.width() + (hasAction ? spacing + actionSize.width() : 0);
    if (!hasAction || rowWidth <= bounds.width() - 2 * edgeMargin) {
        // 1. 一行放得下时两条工具栏整体水平居中，并按行高垂直居中对齐
        const int rowHeight = std::max(toolbarSize.height(), hasAction ? actionSize.height() : 0);
        const int rowTop = bottom - rowHeight;
        const int rowLeft = bounds.x() + (bounds.width() - rowWidth) / 2;
        layout.toolbar = clampedInto(
            QRect(QPoint(rowLeft, rowTop + (rowHeight - toolbarSize.height()) / 2), toolbarSize), bounds, edgeMargin);
        if (hasAction) {
            const QPoint actionTopLeft(layout.toolbar.x() + toolbarSize.width() + spacing,
                                       rowTop + (rowHeight - actionSize.height()) / 2);
            layout.actionToolbar = clampedInto(QRect(actionTopLeft, actionSize), bounds, edgeMargin);
        }
        return layout;
    }

    // 2. 宽度不足时主工具栏贴底居中，副工具栏移到其上方并与右端对齐
    const int left = bounds.x() + (bounds.width() - toolbarSize.width()) / 2;
    layout.toolbar = clampedInto(QRect(QPoint(left, bottom - toolbarSize.height()), toolbarSize), bounds, edgeMargin);
    const QPoint actionTopLeft(layout.toolbar.x() + toolbarSize.width() - actionSize.width(),
                               layout.toolbar.y() - spacing - actionSize.height());
    layout.actionToolbar = clampedInto(QRect(actionTopLeft, actionSize), bounds, edgeMargin);
    return layout;
}

/// @brief 计算两条工具栏停靠右侧时的几何
/// @param bounds 可用区域
/// @param toolbarSize 主工具栏尺寸
/// @param actionSize 副工具栏尺寸，为空时只摆放主工具栏
/// @param edgeMargin 与屏幕边缘的距离
/// @param spacing 两条工具栏之间的间距
/// @return 停靠后的几何
markshot::DockedToolbarLayout rightDockedLayout(const QRect &bounds,
                                                const QSize &toolbarSize,
                                                const QSize &actionSize,
                                                int edgeMargin,
                                                int spacing)
{
    markshot::DockedToolbarLayout layout;
    const bool hasAction = !actionSize.isEmpty();
    const int right = bounds.x() + bounds.width() - edgeMargin;
    const int columnHeight = toolbarSize.height() + (hasAction ? spacing + actionSize.height() : 0);
    if (!hasAction || columnHeight <= bounds.height() - 2 * edgeMargin) {
        // 1. 一列放得下时两条工具栏整体垂直居中，并按列宽水平居中对齐
        const int columnWidth = std::max(toolbarSize.width(), hasAction ? actionSize.width() : 0);
        const int columnLeft = right - columnWidth;
        const int columnTop = bounds.y() + (bounds.height() - columnHeight) / 2;
        layout.toolbar = clampedInto(
            QRect(QPoint(columnLeft + (columnWidth - toolbarSize.width()) / 2, columnTop), toolbarSize),
            bounds, edgeMargin);
        if (hasAction) {
            const QPoint actionTopLeft(columnLeft + (columnWidth - actionSize.width()) / 2,
                                       layout.toolbar.y() + toolbarSize.height() + spacing);
            layout.actionToolbar = clampedInto(QRect(actionTopLeft, actionSize), bounds, edgeMargin);
        }
        return layout;
    }

    // 2. 高度不足时主工具栏贴右垂直居中，副工具栏移到其左侧并与底端对齐
    const int top = bounds.y() + (bounds.height() - toolbarSize.height()) / 2;
    layout.toolbar = clampedInto(QRect(QPoint(right - toolbarSize.width(), top), toolbarSize), bounds, edgeMargin);
    const QPoint actionTopLeft(layout.toolbar.x() - spacing - actionSize.width(),
                               layout.toolbar.y() + toolbarSize.height() - actionSize.height());
    layout.actionToolbar = clampedInto(QRect(actionTopLeft, actionSize), bounds, edgeMargin);
    return layout;
}

}  // namespace

namespace markshot {

DockedToolbarLayout dockedToolbarLayout(Qt::Orientation edge,
                                        const QRect &bounds,
                                        const QSize &toolbarSize,
                                        const QSize &actionToolbarSize,
                                        int edgeMargin,
                                        int spacing)
{
    if (edge == Qt::Vertical) {
        return rightDockedLayout(bounds, toolbarSize, actionToolbarSize, edgeMargin, spacing);
    }
    return bottomDockedLayout(bounds, toolbarSize, actionToolbarSize, edgeMargin, spacing);
}

}  // namespace markshot
