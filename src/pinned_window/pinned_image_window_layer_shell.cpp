#include "pinned_window/pinned_image_window.h"

#include "pinned_window_top.h"

#include <QApplication>
#include <QScreen>
#include <QTimer>

namespace markshot::shot {

void PinnedImageWindow::scheduleLayerShellScreenRebind()
{
    if (m_layerShellScreenRebindPending
        || !pinnedWindowNeedsLayerShellScreenRebind(this, m_logicalGeometry)) {
        return;
    }

    m_layerShellScreenRebindPending = true;
    QTimer::singleShot(0, this, [this] {
        m_layerShellScreenRebindPending = false;
        rebindLayerShellScreen();
    });
}

void PinnedImageWindow::rebindLayerShellScreen()
{
    if (!pinnedWindowHasLayerShellTop(this)
        || !pinnedWindowNeedsLayerShellScreenRebind(this, m_logicalGeometry)) {
        return;
    }

    QScreen *targetScreen = pinnedWindowTargetLayerShellScreen(m_logicalGeometry);
    if (!targetScreen) {
        return;
    }

    const QRect logicalGeometry = m_logicalGeometry;
    const bool wasVisible = isVisible();
    const bool continueMouseDrag = QApplication::mouseButtons().testFlag(Qt::LeftButton);
    if (QWidget::mouseGrabber() == this) {
        releaseMouse();
    }

    // 1. 销毁旧 wl_surface，避免继续向原输出提交不可见内容
    hide();
    destroy();
    setProperty("markShotPinnedLayerShellActive", false);
    setScreen(targetScreen);

    // 2. 在目标输出重新创建 layer-shell surface，并恢复完整逻辑几何
    setMinimumSize(QSize(24, 24));
    setMaximumSize(QSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX));
    setFixedSize(logicalGeometry.size());
    m_logicalGeometry = logicalGeometry;
    setProperty("markShotPinnedGeometry", m_logicalGeometry);
    applyPinnedWindowTopState(this, m_config.alwaysOnTop);
    setPinnedGeometry(m_logicalGeometry, false);

    // 3. 恢复可见状态和鼠标抓取，使跨屏拖拽连续进行
    if (wasVisible) {
        show();
    }
    if (continueMouseDrag) {
        grabMouse();
        setCursor(Qt::ClosedHandCursor);
    }
    schedulePinnedWindowRaise();
}

}  // namespace markshot::shot
