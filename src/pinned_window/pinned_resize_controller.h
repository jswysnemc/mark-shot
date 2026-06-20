#pragma once

#include <QCursor>
#include <QPoint>
#include <QPointF>
#include <QRect>
#include <QRectF>
#include <QSize>

namespace markshot::shot {

enum class PinnedResizeDirection {
    None,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
};

struct PinnedResizeDragState {
    PinnedResizeDirection direction = PinnedResizeDirection::None;
    QRect startGeometry;
    QPoint startGlobalPosition;
    qreal aspectRatio = 1.0;
};

/// @brief 判断指定方向是否表示有效的置顶图片调整大小操作。
/// @param direction 鼠标命中的窗口边界方向。
/// @return 命中边界时返回 true。
bool isPinnedResizeDirection(PinnedResizeDirection direction);

/// @brief 判断缩放方向是否包含左边界。
/// @param direction 缩放方向。
/// @return 包含左边界时返回 true。
bool pinnedResizeDirectionIncludesLeft(PinnedResizeDirection direction);

/// @brief 判断缩放方向是否包含右边界。
/// @param direction 缩放方向。
/// @return 包含右边界时返回 true。
bool pinnedResizeDirectionIncludesRight(PinnedResizeDirection direction);

/// @brief 判断缩放方向是否包含上边界。
/// @param direction 缩放方向。
/// @return 包含上边界时返回 true。
bool pinnedResizeDirectionIncludesTop(PinnedResizeDirection direction);

/// @brief 判断缩放方向是否包含下边界。
/// @param direction 缩放方向。
/// @return 包含下边界时返回 true。
bool pinnedResizeDirectionIncludesBottom(PinnedResizeDirection direction);

/// @brief 判断贴边窗口的缩放方向是否应让位给移动。
/// @param direction 缩放方向。
/// @param geometry 窗口全局逻辑几何。
/// @param screenGeometry 屏幕全局逻辑几何。
/// @return 命中贴边边缘时返回 true。
bool pinnedResizeDirectionTouchesScreenEdge(PinnedResizeDirection direction,
                                            QRect geometry,
                                            QRect screenGeometry);

/// @brief 根据鼠标位置判断置顶图片窗口的边界命中方向。
/// @param localRect 窗口本地矩形。
/// @param localPoint 鼠标在窗口内的位置。
/// @param margin 边界热区宽度。
/// @return 命中的边界或角落方向。
PinnedResizeDirection pinnedResizeDirectionAt(QRectF localRect, QPointF localPoint, qreal margin);

/// @brief 返回边界调整大小方向对应的鼠标光标。
/// @param direction 鼠标命中的窗口边界方向。
/// @return Qt 鼠标光标形状。
Qt::CursorShape cursorForPinnedResizeDirection(PinnedResizeDirection direction);

/// @brief 创建一次置顶图片边界拖拽状态。
/// @param direction 拖拽开始时命中的边界方向。
/// @param startGeometry 拖拽开始时的全局窗口几何。
/// @param startGlobalPosition 拖拽开始时的全局鼠标位置。
/// @return 可继续计算窗口几何的拖拽状态。
PinnedResizeDragState beginPinnedResizeDrag(PinnedResizeDirection direction,
                                            QRect startGeometry,
                                            QPoint startGlobalPosition);

/// @brief 按照固定宽高比计算拖拽后的窗口几何。
/// @param state 拖拽开始时记录的状态。
/// @param currentGlobalPosition 当前全局鼠标位置。
/// @param minimumSize 允许的最小窗口尺寸。
/// @return 调整大小后的全局窗口几何。
QRect pinnedResizeGeometry(const PinnedResizeDragState &state,
                           QPoint currentGlobalPosition,
                           QSize minimumSize);

}  // namespace markshot::shot
