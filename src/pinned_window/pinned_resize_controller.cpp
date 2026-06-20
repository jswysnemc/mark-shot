#include "pinned_window/pinned_resize_controller.h"

#include <QtGlobal>

#include <algorithm>
#include <cmath>

namespace {

/// @brief 将数值限制在有效范围内。
/// @param value 原始数值。
/// @param minimum 最小值。
/// @return 限制后的数值。
qreal clampAtLeast(qreal value, qreal minimum)
{
    return std::max(value, minimum);
}

/// @brief 判断方向是否包含左边界。
/// @param direction 边界方向。
/// @return 包含左边界时返回 true。
bool hasLeft(markshot::shot::PinnedResizeDirection direction)
{
    using markshot::shot::PinnedResizeDirection;
    return direction == PinnedResizeDirection::Left
        || direction == PinnedResizeDirection::TopLeft
        || direction == PinnedResizeDirection::BottomLeft;
}

/// @brief 判断方向是否包含右边界。
/// @param direction 边界方向。
/// @return 包含右边界时返回 true。
bool hasRight(markshot::shot::PinnedResizeDirection direction)
{
    using markshot::shot::PinnedResizeDirection;
    return direction == PinnedResizeDirection::Right
        || direction == PinnedResizeDirection::TopRight
        || direction == PinnedResizeDirection::BottomRight;
}

/// @brief 判断方向是否包含上边界。
/// @param direction 边界方向。
/// @return 包含上边界时返回 true。
bool hasTop(markshot::shot::PinnedResizeDirection direction)
{
    using markshot::shot::PinnedResizeDirection;
    return direction == PinnedResizeDirection::Top
        || direction == PinnedResizeDirection::TopLeft
        || direction == PinnedResizeDirection::TopRight;
}

/// @brief 判断方向是否包含下边界。
/// @param direction 边界方向。
/// @return 包含下边界时返回 true。
bool hasBottom(markshot::shot::PinnedResizeDirection direction)
{
    using markshot::shot::PinnedResizeDirection;
    return direction == PinnedResizeDirection::Bottom
        || direction == PinnedResizeDirection::BottomLeft
        || direction == PinnedResizeDirection::BottomRight;
}

/// @brief 根据目标尺寸和拖拽方向构建新窗口几何。
/// @param startGeometry 拖拽开始时的窗口几何。
/// @param direction 拖拽方向。
/// @param targetSize 调整后的窗口尺寸。
/// @return 调整后的窗口几何。
QRect geometryFromDirection(QRect startGeometry,
                            markshot::shot::PinnedResizeDirection direction,
                            QSize targetSize)
{
    const qreal centerX = startGeometry.x() + startGeometry.width() / 2.0;
    const qreal centerY = startGeometry.y() + startGeometry.height() / 2.0;
    int x = startGeometry.x();
    int y = startGeometry.y();

    if (hasLeft(direction)) {
        x = startGeometry.x() + startGeometry.width() - targetSize.width();
    } else if (!hasRight(direction)) {
        x = qRound(centerX - targetSize.width() / 2.0);
    }

    if (hasTop(direction)) {
        y = startGeometry.y() + startGeometry.height() - targetSize.height();
    } else if (!hasBottom(direction)) {
        y = qRound(centerY - targetSize.height() / 2.0);
    }

    return QRect(QPoint(x, y), targetSize);
}

}  // namespace

namespace markshot::shot {

bool isPinnedResizeDirection(PinnedResizeDirection direction)
{
    return direction != PinnedResizeDirection::None;
}

bool pinnedResizeDirectionIncludesLeft(PinnedResizeDirection direction)
{
    return hasLeft(direction);
}

bool pinnedResizeDirectionIncludesRight(PinnedResizeDirection direction)
{
    return hasRight(direction);
}

bool pinnedResizeDirectionIncludesTop(PinnedResizeDirection direction)
{
    return hasTop(direction);
}

bool pinnedResizeDirectionIncludesBottom(PinnedResizeDirection direction)
{
    return hasBottom(direction);
}

bool pinnedResizeDirectionTouchesScreenEdge(PinnedResizeDirection direction,
                                            QRect geometry,
                                            QRect screenGeometry)
{
    if (!isPinnedResizeDirection(direction) || !geometry.isValid() || !screenGeometry.isValid()) {
        return false;
    }

    return (hasLeft(direction) && geometry.left() <= screenGeometry.left())
        || (hasRight(direction) && geometry.right() >= screenGeometry.right())
        || (hasTop(direction) && geometry.top() <= screenGeometry.top())
        || (hasBottom(direction) && geometry.bottom() >= screenGeometry.bottom());
}

PinnedResizeDirection pinnedResizeDirectionAt(QRectF localRect, QPointF localPoint, qreal margin)
{
    if (localRect.isEmpty() || margin <= 0.0) {
        return PinnedResizeDirection::None;
    }

    if (!localRect.adjusted(-margin, -margin, margin, margin).contains(localPoint)) {
        return PinnedResizeDirection::None;
    }

    // 1. 优先识别角落，避免角落位置被单边调整抢占
    const bool nearLeft = std::abs(localPoint.x() - localRect.left()) <= margin;
    const bool nearRight = std::abs(localPoint.x() - localRect.right()) <= margin;
    const bool nearTop = std::abs(localPoint.y() - localRect.top()) <= margin;
    const bool nearBottom = std::abs(localPoint.y() - localRect.bottom()) <= margin;

    if (nearLeft && nearTop) {
        return PinnedResizeDirection::TopLeft;
    }
    if (nearRight && nearTop) {
        return PinnedResizeDirection::TopRight;
    }
    if (nearLeft && nearBottom) {
        return PinnedResizeDirection::BottomLeft;
    }
    if (nearRight && nearBottom) {
        return PinnedResizeDirection::BottomRight;
    }
    if (nearLeft) {
        return PinnedResizeDirection::Left;
    }
    if (nearRight) {
        return PinnedResizeDirection::Right;
    }
    if (nearTop) {
        return PinnedResizeDirection::Top;
    }
    if (nearBottom) {
        return PinnedResizeDirection::Bottom;
    }
    return PinnedResizeDirection::None;
}

Qt::CursorShape cursorForPinnedResizeDirection(PinnedResizeDirection direction)
{
    switch (direction) {
    case PinnedResizeDirection::Left:
    case PinnedResizeDirection::Right:
        return Qt::SizeHorCursor;
    case PinnedResizeDirection::Top:
    case PinnedResizeDirection::Bottom:
        return Qt::SizeVerCursor;
    case PinnedResizeDirection::TopLeft:
    case PinnedResizeDirection::BottomRight:
        return Qt::SizeFDiagCursor;
    case PinnedResizeDirection::TopRight:
    case PinnedResizeDirection::BottomLeft:
        return Qt::SizeBDiagCursor;
    case PinnedResizeDirection::None:
        return Qt::ArrowCursor;
    }
    return Qt::ArrowCursor;
}

PinnedResizeDragState beginPinnedResizeDrag(PinnedResizeDirection direction,
                                            QRect startGeometry,
                                            QPoint startGlobalPosition)
{
    PinnedResizeDragState state;
    state.direction = direction;
    state.startGeometry = startGeometry;
    state.startGlobalPosition = startGlobalPosition;
    state.aspectRatio = startGeometry.height() > 0
        ? static_cast<qreal>(startGeometry.width()) / static_cast<qreal>(startGeometry.height())
        : 1.0;
    return state;
}

QRect pinnedResizeGeometry(const PinnedResizeDragState &state,
                           QPoint currentGlobalPosition,
                           QSize minimumSize)
{
    if (!isPinnedResizeDirection(state.direction) || !state.startGeometry.isValid()) {
        return state.startGeometry;
    }

    const QPoint delta = currentGlobalPosition - state.startGlobalPosition;
    const qreal startWidth = std::max(1, state.startGeometry.width());
    const qreal startHeight = std::max(1, state.startGeometry.height());
    const qreal aspectRatio = state.aspectRatio > 0.0 ? state.aspectRatio : startWidth / startHeight;

    qreal requestedWidth = startWidth;
    qreal requestedHeight = startHeight;
    if (hasLeft(state.direction)) {
        requestedWidth = startWidth - delta.x();
    } else if (hasRight(state.direction)) {
        requestedWidth = startWidth + delta.x();
    }
    if (hasTop(state.direction)) {
        requestedHeight = startHeight - delta.y();
    } else if (hasBottom(state.direction)) {
        requestedHeight = startHeight + delta.y();
    }

    const bool horizontalOnly = (hasLeft(state.direction) || hasRight(state.direction))
        && !hasTop(state.direction)
        && !hasBottom(state.direction);
    const bool verticalOnly = (hasTop(state.direction) || hasBottom(state.direction))
        && !hasLeft(state.direction)
        && !hasRight(state.direction);

    qreal scale = 1.0;
    if (horizontalOnly) {
        scale = requestedWidth / startWidth;
    } else if (verticalOnly) {
        scale = requestedHeight / startHeight;
    } else {
        const qreal widthScale = requestedWidth / startWidth;
        const qreal heightScale = requestedHeight / startHeight;
        scale = std::abs(widthScale - 1.0) >= std::abs(heightScale - 1.0)
            ? widthScale
            : heightScale;
    }

    const qreal minWidthScale = static_cast<qreal>(std::max(1, minimumSize.width())) / startWidth;
    const qreal minHeightScale = static_cast<qreal>(std::max(1, minimumSize.height())) / startHeight;
    scale = clampAtLeast(scale, std::max(minWidthScale, minHeightScale));

    QSize targetSize(qRound(startWidth * scale), qRound(startHeight * scale));
    targetSize.setWidth(std::max(minimumSize.width(), targetSize.width()));
    targetSize.setHeight(std::max(minimumSize.height(), targetSize.height()));
    if (targetSize.height() > 0) {
        targetSize.setWidth(std::max(minimumSize.width(), qRound(targetSize.height() * aspectRatio)));
    }

    return geometryFromDirection(state.startGeometry, state.direction, targetSize);
}

}  // namespace markshot::shot
