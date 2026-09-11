#include "ocr_result_window.h"

#include "app_config_store.h"
#include "debug_log.h"
#include "pinned_window_top.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QJsonValue>
#include <QMouseEvent>
#include <QPushButton>
#include <QScreen>
#include <QShowEvent>
#include <QSignalBlocker>
#include <QTimer>
#include <QWindow>

namespace markshot::shot {

void OcrResultWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    if (!m_alwaysOnTop || pinnedWindowHasLayerShellTop(this)) {
        return;
    }
    // 1. 【OCR】【窗口置顶】等待窗口映射完成，供 GNOME 等桌面按标题查找新窗口
    for (int delayMs : {0, 80, 250, 600}) {
        QTimer::singleShot(delayMs, this, [this] {
            if (isVisible() && m_alwaysOnTop && !pinnedWindowHasLayerShellTop(this)) {
                raisePinnedWindowOnPlatform(this);
            }
        });
    }
}

void OcrResultWindow::recreateWindowSurface()
{
    const bool visible = isVisible();
    const bool resumeDrag = m_dragging && QApplication::mouseButtons().testFlag(Qt::LeftButton);
    QScreen *target = pinnedWindowTargetLayerShellScreen(m_logicalGeometry);

    // 1. 【OCR】【窗口模式】销毁旧协议窗口，解除 layer-shell 对窗口管理器操作的限制
    if (QWidget::mouseGrabber() == this) {
        releaseMouse();
    }
    hide();
    destroy();
    setProperty("markShotPinnedLayerShellActive", false);
    if (target) {
        setScreen(target);
    }

    // 2. 【OCR】【窗口模式】按当前置顶开关重新创建窗口并恢复逻辑几何
    setGeometry(m_logicalGeometry);
    setProperty("markShotPinnedGeometry", m_logicalGeometry);
    applyPinnedWindowTopState(this, m_alwaysOnTop);
    if (visible) {
        show();
        raise();
        activateWindow();
    }
    if (resumeDrag) {
        grabMouse();
    }
}

bool OcrResultWindow::beginWindowDrag(QMouseEvent *event)
{
    if (!event || event->button() != Qt::LeftButton) {
        return false;
    }

    // 1. 【OCR】【窗口拖动】普通窗口优先交给窗口管理器移动
    const bool layerShell = pinnedWindowHasLayerShellTop(this);
    if (!layerShell && windowHandle() && windowHandle()->startSystemMove()) {
        event->accept();
        return true;
    }

    // 2. 【OCR】【窗口拖动】置顶浮层使用记录的逻辑位置计算拖动偏移
    if (!layerShell) {
        m_logicalGeometry = geometry();
    }
    m_dragging = true;
    m_dragOffset = event->globalPosition().toPoint() - m_logicalGeometry.topLeft();
    setCursor(Qt::SizeAllCursor);
    grabMouse();
    event->accept();
    return true;
}

bool OcrResultWindow::updateWindowDrag(QMouseEvent *event)
{
    if (!event || !m_dragging) {
        return false;
    }

    // 1. 【OCR】【窗口拖动】同步普通窗口位置与浮层边距
    m_logicalGeometry.moveTopLeft(event->globalPosition().toPoint() - m_dragOffset);
    m_logicalGeometry.setSize(size());
    setGeometry(m_logicalGeometry);
    if (pinnedWindowHasLayerShellTop(this)) {
        if (pinnedWindowNeedsLayerShellScreenRebind(this, m_logicalGeometry)) {
            recreateWindowSurface();
        } else {
            syncPinnedWindowTopGeometry(this, m_logicalGeometry);
        }
    }
    event->accept();
    return true;
}

bool OcrResultWindow::finishWindowDrag(QMouseEvent *event)
{
    if (!event || event->button() != Qt::LeftButton || !m_dragging) {
        return false;
    }
    m_dragging = false;
    if (QWidget::mouseGrabber() == this) {
        releaseMouse();
    }
    unsetCursor();
    event->accept();
    return true;
}

void OcrResultWindow::setAlwaysOnTop(bool alwaysOnTop)
{
    if (m_alwaysOnTop == alwaysOnTop) {
        return;
    }

    // 1. 【OCR】【置顶配置】只保存 OCR 窗口的开关，不修改钉图配置
    QString error;
    if (!markshot::writeAppConfigValue({QStringLiteral("ocrResultWindow"), QStringLiteral("alwaysOnTop")},
                                       QJsonValue(alwaysOnTop), &error)) {
        const QSignalBlocker blocker(m_pinButton);
        m_pinButton->setChecked(m_alwaysOnTop);
        showToast(MS_TR("Failed to save settings"));
        markshot::debugLog("ocr", "【OCR】【置顶配置】保存失败: %s", error.toUtf8().constData());
        return;
    }

    // 2. 【OCR】【窗口模式】只有切换协议角色时才重建窗口
    const bool wasLayerShell = pinnedWindowHasLayerShellTop(this);
    const bool changeSurface = wasLayerShell || (alwaysOnTop && pinnedWindowUsesLayerShellTop());
    if (!wasLayerShell) {
        if (changeSurface && QGuiApplication::platformName().contains(QStringLiteral("wayland")) && screen()) {
            // 3. 【OCR】【窗口位置】普通 Wayland 窗口不提供全局位置，置顶时在当前屏幕居中
            const QRect available = screen()->availableGeometry();
            m_logicalGeometry = QRect(QPoint(), size().boundedTo(available.size()));
            m_logicalGeometry.moveCenter(available.center());
        } else {
            m_logicalGeometry = geometry();
        }
    }
    m_alwaysOnTop = alwaysOnTop;
    if (changeSurface) {
        recreateWindowSurface();
    } else {
        applyPinnedWindowTopState(this, m_alwaysOnTop);
    }
    m_pinButton->setToolTip(alwaysOnTop ? MS_TR("Always on Top: On") : MS_TR("Always on Top: Off"));
}

}
