#include "recording/ui/recording_overlay_service.h"

#include "debug_log.h"
#include "recording/recording_session_manager.h"
#include "recording/recording_status.h"
#include "recording/ui/recording_overlay_window.h"

#include <QGuiApplication>
#include <QRect>
#include <QScreen>
#include <QTimer>
#include <QtGlobal>

namespace markshot::recording::ui {
namespace {

// 控制条计时刷新间隔，兼顾秒级精度与开销
constexpr int kRefreshIntervalMs = 500;

/**
 * 判断是否禁用录制覆盖层。
 * @return 环境变量要求关闭覆盖层时返回 true。
 */
bool overlayDisabled()
{
    return qEnvironmentVariable("MARK_SHOT_RECORDING_OVERLAY").trimmed() == QStringLiteral("0");
}

/**
 * 查找录制区域所在的屏幕。
 * @param status 录制状态。
 * @return 目标屏幕，找不到时返回主屏幕。
 */
QScreen *screenForStatus(const RecordingStatus &status)
{
    const QList<QScreen *> screens = QGuiApplication::screens();
    // 1. 优先按录制配置中的显示器名称匹配
    if (!status.screenName.trimmed().isEmpty()) {
        for (QScreen *screen : screens) {
            if (screen && screen->name() == status.screenName) {
                return screen;
            }
        }
    }
    // 2. 其次按录制区域中心点归属判断
    if (!status.captureGeometry.isEmpty()) {
        if (QScreen *screen = QGuiApplication::screenAt(status.captureGeometry.center())) {
            return screen;
        }
    }
    return QGuiApplication::primaryScreen();
}

/**
 * 判断录制区域是否覆盖整块屏幕。
 * @param status 录制状态。
 * @param screen 目标屏幕。
 * @return 覆盖整屏时返回 true。
 */
bool recordsWholeScreen(const RecordingStatus &status, QScreen *screen)
{
    if (status.scope == RecordingScope::Display) {
        return true;
    }
    if (!screen || status.captureGeometry.isEmpty()) {
        return true;
    }
    return status.captureGeometry.contains(screen->geometry());
}

}  // namespace

RecordingOverlayService &RecordingOverlayService::instance()
{
    static RecordingOverlayService service;
    return service;
}

RecordingOverlayService::RecordingOverlayService(QObject *parent)
    : QObject(parent)
{
}

void RecordingOverlayService::attach()
{
    if (m_attached) {
        return;
    }
    m_attached = true;

    m_refreshTimer = new QTimer(this);
    m_refreshTimer->setInterval(kRefreshIntervalMs);
    connect(m_refreshTimer, &QTimer::timeout, this, &RecordingOverlayService::refreshOverlay);

    connect(&RecordingSessionManager::instance(),
            &RecordingSessionManager::statusChanged,
            this,
            &RecordingOverlayService::handleStatusChanged);
}

void RecordingOverlayService::handleStatusChanged()
{
    const RecordingStatus status = RecordingSessionManager::instance().status();
    if (!status.active) {
        destroyOverlay();
        return;
    }
    if (!m_overlay) {
        createOverlay(status);
    }
    refreshOverlay();
}

void RecordingOverlayService::createOverlay(const RecordingStatus &status)
{
    if (overlayDisabled()) {
        return;
    }

    QScreen *screen = screenForStatus(status);
    auto *overlay = new RecordingOverlayWindow(screen,
                                               status.captureGeometry,
                                               recordsWholeScreen(status, screen));
    if (!overlay->hasVisibleContent()) {
        // 整屏录制时控制条会被录进画面，改由托盘与快捷键控制
        markshot::debugLog("recording", "【录制】【覆盖层】skipped reason=whole-screen");
        overlay->deleteLater();
        return;
    }

    connect(overlay, &RecordingOverlayWindow::pauseToggleRequested, this, [] {
        RecordingSessionManager::instance().togglePause();
    });
    connect(overlay, &RecordingOverlayWindow::stopRequested, this, [] {
        RecordingSessionManager::instance().stop(nullptr);
    });

    overlay->show();
    m_overlay = overlay;
    m_refreshTimer->start();
    markshot::debugLog("recording", "【录制】【覆盖层】shown screen=%s",
                       screen ? screen->name().toUtf8().constData() : "unknown");
}

void RecordingOverlayService::destroyOverlay()
{
    if (m_refreshTimer) {
        m_refreshTimer->stop();
    }
    if (!m_overlay) {
        return;
    }
    m_overlay->hide();
    m_overlay->deleteLater();
    m_overlay = nullptr;
}

void RecordingOverlayService::refreshOverlay()
{
    if (!m_overlay) {
        return;
    }
    const RecordingStatus status = RecordingSessionManager::instance().status();
    if (!status.active) {
        destroyOverlay();
        return;
    }
    m_overlay->updateStatus(status.elapsedMs, status.paused);
}

}  // namespace markshot::recording::ui
