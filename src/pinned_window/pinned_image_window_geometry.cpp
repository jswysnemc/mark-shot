#include "pinned_window/pinned_image_window.h"

#include "pinned_window/pinned_layer_shell_geometry.h"
#include "pinned_window_top.h"

#include <QGuiApplication>
#include <QMouseEvent>
#include <QScreen>
#include <QVariant>

#include <algorithm>

namespace {

constexpr int kPinnedMinimumExtent = 24;
constexpr qreal kPinnedResizeMargin = 8.0;

/// @brief 收集当前屏幕全局逻辑几何。
/// @return 屏幕几何列表。
QVector<QRect> currentScreenGeometries()
{
    const QList<QScreen *> screens = QGuiApplication::screens();
    QVector<QRect> screenGeometries;
    screenGeometries.reserve(screens.size());
    for (QScreen *screen : screens) {
        screenGeometries.append(screen ? screen->geometry() : QRect());
    }
    return screenGeometries;
}

}  // namespace

namespace markshot::shot {

QSize PinnedImageWindow::displayBaseSizeForPixmap() const
{
    const QSizeF logicalSize = m_pixmap.deviceIndependentSize();
    return QSize(std::max(1, qRound(logicalSize.width())),
                 std::max(1, qRound(logicalSize.height())));
}

void PinnedImageWindow::resizeByScale(qreal scale, QPoint globalAnchor, QPointF localAnchor)
{
    scale = std::clamp(scale, 0.1, 6.0);
    QSize targetSize(qMax(kPinnedMinimumExtent, qRound(m_displayBaseSize.width() * scale)),
                     qMax(kPinnedMinimumExtent, qRound(m_displayBaseSize.height() * scale)));
    targetSize.scale(targetSize, Qt::KeepAspectRatio);

    const QRectF imageRect = displayedImageRect();
    const qreal xRatio = imageRect.width() > 0.0 ? (localAnchor.x() - imageRect.left()) / imageRect.width() : 0.5;
    const qreal yRatio = imageRect.height() > 0.0 ? (localAnchor.y() - imageRect.top()) / imageRect.height() : 0.5;
    const QPoint topLeft(globalAnchor.x() - qRound(targetSize.width() * xRatio),
                         globalAnchor.y() - qRound(targetSize.height() * yRatio));
    m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_displayBaseSize.width());
    setPinnedGeometry(QRect(topLeft, targetSize), !pinnedWindowHasLayerShellTop(this));
}

QSize PinnedImageWindow::logicalPinnedSize() const
{
    return m_logicalGeometry.isValid() && !m_logicalGeometry.isEmpty()
        ? m_logicalGeometry.size()
        : size();
}

QRectF PinnedImageWindow::displayedImageRect() const
{
    return QRectF(QPointF(m_layerShellContentOffset), QSizeF(logicalPinnedSize()));
}

QPoint PinnedImageWindow::pinnedTopLeft() const
{
    return m_logicalGeometry.isValid() ? m_logicalGeometry.topLeft() : frameGeometry().topLeft();
}

QPoint PinnedImageWindow::logicalGlobalPointForLocalAnchor(QPointF localAnchor, QPoint fallbackGlobal)
{
    if (!m_config.alwaysOnTop || !pinnedWindowHasLayerShellTop(this)) {
        return fallbackGlobal;
    }
    if (m_layerShellVisibleGeometry.isValid() && !m_layerShellVisibleGeometry.isEmpty()) {
        return m_layerShellVisibleGeometry.topLeft() + localAnchor.toPoint();
    }
    return pinnedTopLeft() + localAnchor.toPoint();
}

void PinnedImageWindow::setPinnedGeometry(QRect geometry, bool moveWidget)
{
    if (!geometry.isValid() || geometry.isEmpty()) {
        return;
    }

    if (pinnedWindowHasLayerShellTop(this)) {
        const QVector<QRect> screenGeometries = currentScreenGeometries();
        const int screenIndex = bestPinnedLayerShellScreenIndex(geometry, screenGeometries);
        if (screenIndex >= 0 && screenIndex < screenGeometries.size()) {
            const PinnedLayerShellPlacement placement = pinnedLayerShellPlacement(
                geometry,
                screenGeometries.at(screenIndex),
                QSize(kPinnedMinimumExtent, kPinnedMinimumExtent));

            // 1. 记录完整图片逻辑几何,但 QWidget 只保留屏幕内可见 surface
            m_logicalGeometry = placement.logicalGeometry;
            m_layerShellVisibleGeometry = placement.visibleGeometry;
            m_layerShellContentOffset = placement.contentOffset;
            setProperty("markShotPinnedGeometry", m_logicalGeometry);
            setMinimumSize(QSize(kPinnedMinimumExtent, kPinnedMinimumExtent));
            setMaximumSize(QSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX));
            if (size() != placement.desiredSize) {
                setFixedSize(placement.desiredSize);
            }
            syncPinnedWindowTopGeometry(this, m_logicalGeometry);
            update();
            return;
        }
    }

    m_logicalGeometry = geometry;
    m_layerShellVisibleGeometry = {};
    m_layerShellContentOffset = {};
    setProperty("markShotPinnedGeometry", m_logicalGeometry);
    if (moveWidget) {
        setMinimumSize(QSize(kPinnedMinimumExtent, kPinnedMinimumExtent));
        setMaximumSize(QSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX));
        setGeometry(m_logicalGeometry);
    }
    if (m_config.alwaysOnTop) {
        syncPinnedWindowTopGeometry(this, m_logicalGeometry);
    }
}

PinnedResizeDirection PinnedImageWindow::resizeDirectionAt(QPointF widgetPoint) const
{
    const qreal margin = std::max(kPinnedResizeMargin, m_config.borderWidth + 5.0);
    return pinnedResizeDirectionAt(QRectF(rect()), widgetPoint, margin);
}

bool PinnedImageWindow::shouldBlockResizeAtEmbeddedEdge(PinnedResizeDirection direction) const
{
    if (!isPinnedResizeDirection(direction) || !property("markShotPinnedLayerShellActive").toBool()) {
        return false;
    }

    const QRect geometry(pinnedTopLeft(), logicalPinnedSize());
    const QVector<QRect> screenGeometries = currentScreenGeometries();
    const int screenIndex = bestPinnedLayerShellScreenIndex(geometry, screenGeometries);
    if (screenIndex < 0 || screenIndex >= screenGeometries.size()) {
        return false;
    }

    const QRect screenGeometry = screenGeometries.at(screenIndex);

    // 1. 贴住或越过屏幕边缘时,对应边缘拖拽优先作为移动处理
    return pinnedResizeDirectionTouchesScreenEdge(direction, geometry, screenGeometry);
}

bool PinnedImageWindow::startResizeDrag(QMouseEvent *event)
{
    const PinnedResizeDirection direction = resizeDirectionAt(event->position());
    if (!isPinnedResizeDirection(direction) || shouldBlockResizeAtEmbeddedEdge(direction)) {
        return false;
    }

    clearTextSelection();
    const QRect startGeometry(pinnedTopLeft(), logicalPinnedSize());
    m_resizeDrag = beginPinnedResizeDrag(direction, startGeometry, event->globalPosition().toPoint());
    setCursor(cursorForPinnedResizeDirection(direction));
    return true;
}

bool PinnedImageWindow::continueResizeDrag(QMouseEvent *event)
{
    if (!isPinnedResizeDirection(m_resizeDrag.direction)) {
        return false;
    }
    if (!event->buttons().testFlag(Qt::LeftButton)) {
        finishResizeDrag(event->position());
        return true;
    }

    const QRect geometry = pinnedResizeGeometry(m_resizeDrag,
                                                event->globalPosition().toPoint(),
                                                QSize(kPinnedMinimumExtent, kPinnedMinimumExtent));
    applyResizeGeometry(geometry);
    setCursor(cursorForPinnedResizeDirection(m_resizeDrag.direction));
    return true;
}

void PinnedImageWindow::finishResizeDrag(QPointF widgetPoint)
{
    m_resizeDrag = {};
    updateCursorForPosition(widgetPoint);
}

void PinnedImageWindow::applyResizeGeometry(QRect geometry)
{
    if (!geometry.isValid() || geometry.isEmpty()) {
        return;
    }

    // 1. 更新缩放比例，保持滚轮缩放和菜单缩放从当前尺寸继续计算
    m_scale = static_cast<qreal>(geometry.width()) / std::max(1, m_displayBaseSize.width());

    // 2. 根据平台协议同步窗口位置，layer-shell 场景会裁剪出可见 surface
    setPinnedGeometry(geometry, !pinnedWindowHasLayerShellTop(this));
}

}  // namespace markshot::shot
