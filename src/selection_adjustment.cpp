#include "selection_adjustment.h"

#include <algorithm>

namespace markshot::shot {
namespace {

using Handle = types::SelectionDrag;

/// @brief 表示参与调整的四条边
struct Edges {
    bool left;
    bool right;
    bool top;
    bool bottom;
};

/// @brief 返回拖动目标对应的边
/// @param handle 拖动目标
/// @return 各条边是否参与调整
Edges adjustedEdges(Handle handle)
{
    return {
        handle == Handle::Left || handle == Handle::TopLeft || handle == Handle::BottomLeft,
        handle == Handle::Right || handle == Handle::TopRight || handle == Handle::BottomRight,
        handle == Handle::Top || handle == Handle::TopLeft || handle == Handle::TopRight,
        handle == Handle::Bottom || handle == Handle::BottomLeft || handle == Handle::BottomRight,
    };
}

}

QRectF adjustedSelectionRect(QRectF before, Handle handle,
                             QPointF dragStart, QPointF pointer, QSize imageSize)
{
    constexpr qreal minimum = 8.0;
    const QRectF bounds{QPointF(), QSizeF(imageSize)};
    if (!bounds.contains(before) || before.width() < minimum || before.height() < minimum) {
        return before;
    }

    // 1. 【截图】【选区调整】移动时保持尺寸，并限制整个选区位于图像内
    if (handle == Handle::Move) {
        const QPointF delta = pointer - dragStart;
        before.moveLeft(std::clamp(before.left() + delta.x(), 0.0, bounds.width() - before.width()));
        before.moveTop(std::clamp(before.top() + delta.y(), 0.0, bounds.height() - before.height()));
        return before;
    }

    // 2. 【截图】【选区调整】只修改命中的边，禁止边框交叉或小于最小尺寸
    const Edges edges = adjustedEdges(handle);
    if (edges.left) {
        before.setLeft(std::clamp(pointer.x(), 0.0, before.right() - minimum));
    }
    if (edges.right) {
        before.setRight(std::clamp(pointer.x(), before.left() + minimum, bounds.width()));
    }
    if (edges.top) {
        before.setTop(std::clamp(pointer.y(), 0.0, before.bottom() - minimum));
    }
    if (edges.bottom) {
        before.setBottom(std::clamp(pointer.y(), before.top() + minimum, bounds.height()));
    }
    return before;
}

QPointF selectionAdjustmentPoint(QRectF selection, Handle handle, QPointF pointer)
{
    const Edges edges = adjustedEdges(handle);
    if (edges.left || edges.right) {
        pointer.setX(edges.left ? selection.left() : selection.right());
    }
    if (edges.top || edges.bottom) {
        pointer.setY(edges.top ? selection.top() : selection.bottom());
    }
    return pointer;
}

}
