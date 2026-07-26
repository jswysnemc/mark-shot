#include "recording/ui/recording_overlay_window.h"

#include "layer_shell_runtime.h"
#include "recording/ui/recording_control_bar.h"
#include "recording/ui/recording_overlay_layout.h"
#include "windows_integration.h"

#include <QGuiApplication>
#include <QPainter>
#include <QPaintEvent>
#include <QRegion>
#include <QScreen>

namespace markshot::recording::ui {
namespace {

// 录制区域边框线宽，边框画在录制区域外侧，不会进入录制画面
constexpr int kFrameWidth = 2;

/**
 * 读取录制区域边框颜色。
 * @return 边框颜色。
 */
QColor frameColor()
{
    return QColor(239, 68, 68, 220);
}

}  // namespace

RecordingOverlayWindow::RecordingOverlayWindow(QScreen *screen,
                                               const QRect &regionRect,
                                               bool recordsWholeScreen,
                                               QWidget *parent)
    : QWidget(parent)
    , m_regionRect(regionRect)
    , m_recordsWholeScreen(recordsWholeScreen)
{
    QScreen *targetScreen = screen ? screen : QGuiApplication::primaryScreen();
    m_screenRect = targetScreen ? targetScreen->geometry() : QRect();

    configureWindow(targetScreen);

    m_controlBar = new RecordingControlBar(this);
    connect(m_controlBar,
            &RecordingControlBar::pauseToggleRequested,
            this,
            &RecordingOverlayWindow::pauseToggleRequested);
    connect(m_controlBar,
            &RecordingControlBar::stopRequested,
            this,
            &RecordingOverlayWindow::stopRequested);

    applyPlacement();
}

void RecordingOverlayWindow::configureWindow(QScreen *screen)
{
    setWindowFlags(Qt::Tool | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint
                   | Qt::WindowDoesNotAcceptFocus | Qt::BypassWindowManagerHint);
    setAttribute(Qt::WA_TranslucentBackground, true);
    setAttribute(Qt::WA_ShowWithoutActivating, true);
    setAttribute(Qt::WA_DeleteOnClose, false);
    setFocusPolicy(Qt::NoFocus);

    if (screen) {
        setScreen(screen);
    }
    if (!m_screenRect.isEmpty()) {
        setGeometry(m_screenRect);
    }

    // 1. Wayland 下普通窗口无法自定位，优先走 layer-shell 浮动层
    markshot::layershell::FloatingOverlayConfig config;
    config.scope = QStringLiteral("mark-shot-recording");
    config.keyboardInteractivity = markshot::layershell::KeyboardInteractivity::OnDemand;
    config.closeOnDismissed = false;
    config.wantsActiveScreenWhenNoScreen = true;
    config.activateOnShow = false;
    config.desiredSize = m_screenRect.size();
    markshot::layershell::configureFloatingOverlay(this, screen, config);

    // 2. Windows 上把覆盖层排除在自身截图之外
    markshot::windows::setExcludedFromCapture(this);
}

void RecordingOverlayWindow::applyPlacement()
{
    const RecordingOverlayPlacement placement = recordingOverlayPlacement(m_regionRect,
                                                                          m_screenRect,
                                                                          m_recordsWholeScreen,
                                                                          m_controlBar
                                                                              ? m_controlBar->sizeHint()
                                                                              : QSize());
    m_showRegionFrame = placement.showRegionFrame;
    m_showControlBar = placement.showControlBar;
    m_controlBarRect = placement.controlBarRect;

    if (m_controlBar) {
        m_controlBar->setVisible(m_showControlBar);
        if (m_showControlBar) {
            m_controlBar->setGeometry(m_controlBarRect.translated(-m_screenRect.topLeft()));
        }
    }
    setMask(buildMask());
}

QRect RecordingOverlayWindow::frameRectInWindow() const
{
    return m_regionRect.translated(-m_screenRect.topLeft())
        .adjusted(-kFrameWidth, -kFrameWidth, kFrameWidth, kFrameWidth);
}

QRegion RecordingOverlayWindow::buildMask() const
{
    QRegion mask;
    // 1. 边框只保留四条细线，录制区域内部保持完全透传
    if (m_showRegionFrame) {
        const QRect outer = frameRectInWindow();
        const QRect inner = outer.adjusted(kFrameWidth, kFrameWidth, -kFrameWidth, -kFrameWidth);
        mask += QRegion(outer).subtracted(QRegion(inner));
    }
    // 2. 控制条区域需要接收点击
    if (m_showControlBar) {
        mask += QRegion(m_controlBarRect.translated(-m_screenRect.topLeft()));
    }
    return mask;
}

void RecordingOverlayWindow::updateStatus(qint64 elapsedMs, bool paused)
{
    if (m_controlBar && m_showControlBar) {
        m_controlBar->updateStatus(elapsedMs, paused);
    }
}

bool RecordingOverlayWindow::hasVisibleContent() const
{
    return m_showRegionFrame || m_showControlBar;
}

void RecordingOverlayWindow::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event)
    if (!m_showRegionFrame) {
        return;
    }

    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, false);
    QPen pen(frameColor());
    pen.setWidth(kFrameWidth);
    pen.setJoinStyle(Qt::MiterJoin);
    painter.setPen(pen);
    painter.setBrush(Qt::NoBrush);
    // 线宽向内绘制，保证边框完全落在录制区域之外
    painter.drawRect(frameRectInWindow().adjusted(kFrameWidth / 2,
                                                  kFrameWidth / 2,
                                                  -kFrameWidth / 2,
                                                  -kFrameWidth / 2));
}

}  // namespace markshot::recording::ui
