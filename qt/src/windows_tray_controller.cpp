#include "windows_tray_controller.h"

#include "debug_log.h"
#include "clipboard_paste/clipboard_pin_launch.h"
#if !defined(Q_OS_WIN)
#include "shortcuts/global_shortcut_manager.h"
#endif
#include "recording/recording_session_manager.h"
#include "recording/recording_start_flow.h"
#include "recording/ui/recording_countdown.h"
#include "recording/recording_status.h"
#include "settings/settings_dialog.h"
#include "capture_history/history_window.h"
#include "shot_window.h"
#include "ui/application_icon.h"
#include "ui/context_menu.h"
#include "ui/context_menu_config.h"
#include "ui/context_menu_native_bridge.h"
#include "ui/i18n.h"
#include "ui/icons.h"
#include "window_detection.h"

#include <QAction>
#include <QApplication>
#include <QByteArray>
#include <QCursor>
#include <QIcon>
#include <QMenu>
#include <QSystemTrayIcon>
#include <QTimer>

#include <algorithm>
#include <optional>

#if defined(Q_OS_WIN)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#ifndef MOD_NOREPEAT
#define MOD_NOREPEAT 0x4000
#endif
#endif

namespace markshot {
namespace {

/// @brief Unique identifier for the capture hotkey.
constexpr int kCaptureHotkeyId = 0x4d53;
/// @brief Unique identifier for the fullscreen hotkey.
constexpr int kFullscreenHotkeyId = 0x4d54;
/// @brief Unique identifier for the stop recording hotkey.
constexpr int kStopRecordingHotkeyId = 0x4d55;
/// @brief Unique identifier for the pause recording hotkey.
constexpr int kPauseRecordingHotkeyId = 0x4d56;

/**
 * 格式化录制持续时间。
 * @param elapsedMs 已录制毫秒数。
 * @return 录制持续时间文本。
 */
QString formatRecordingElapsed(qint64 elapsedMs)
{
    const qint64 totalSeconds = std::max<qint64>(0, elapsedMs / 1000);
    const qint64 hours = totalSeconds / 3600;
    const qint64 minutes = (totalSeconds % 3600) / 60;
    const qint64 seconds = totalSeconds % 60;
    if (hours > 0) {
        return QStringLiteral("%1:%2:%3")
            .arg(hours, 2, 10, QChar(QLatin1Char('0')))
            .arg(minutes, 2, 10, QChar(QLatin1Char('0')))
            .arg(seconds, 2, 10, QChar(QLatin1Char('0')));
    }
    return QStringLiteral("%1:%2")
        .arg(minutes, 2, 10, QChar(QLatin1Char('0')))
        .arg(seconds, 2, 10, QChar(QLatin1Char('0')));
}

/**
 * 返回录制模式在托盘中的显示名称。
 * @param mode 录制模式。
 * @return 录制模式显示名称。
 */
QString recordingModeLabel(markshot::recording::RecordingMode mode)
{
    return mode == markshot::recording::RecordingMode::Gif
        ? QStringLiteral("GIF")
        : MS_TR("Video");
}

/**
 * 创建托盘录制状态文本。
 * @param status 录制状态。
 * @return 托盘状态文本。
 */
QString recordingStatusText(const markshot::recording::RecordingStatus &status)
{
    if (!status.active) {
        return MS_TR("Recording: idle");
    }
    if (status.paused) {
        return MS_TR("Recording paused: %1 %2")
            .arg(recordingModeLabel(status.mode), formatRecordingElapsed(status.elapsedMs));
    }
    return MS_TR("Recording: %1 %2")
        .arg(recordingModeLabel(status.mode), formatRecordingElapsed(status.elapsedMs));
}

/**
 * 返回系统托盘使用的图标。
 *
 * Linux 托盘走 StatusNotifierItem 协议时，Qt 会优先把 QIcon::name() 作为图标名
 * 发给托盘宿主，只有名字为空才退回传像素数据。图标名只有在宿主进程也能按名
 * 找到图标文件时才可用（applicationIconThemeName 会验证标准安装路径）；否则
 * 必须使用不带名字的纯像素图标，避免宿主按名查找失败显示空白（issue #78）。
 *
 * @return 托盘图标。
 */
QIcon trayIcon()
{
    const QString themeName = markshot::ui::applicationIconThemeName();
    if (!themeName.isEmpty()) {
        return QIcon::fromTheme(themeName);
    }
    return markshot::ui::applicationTrayPixmapIcon();
}

#if defined(Q_OS_WIN)

/// @brief Native Windows hotkey components derived from a Qt key sequence.
struct NativeHotkey {
    UINT modifiers = 0; ///< Win32 modifier mask.
    UINT virtualKey = 0; ///< Win32 virtual key code.
};

/// @brief Maps a Qt key code to a Win32 virtual key code.
UINT virtualKeyForQtKey(int key)
{
    if (key >= Qt::Key_A && key <= Qt::Key_Z) {
        return static_cast<UINT>(key);
    }
    if (key >= Qt::Key_0 && key <= Qt::Key_9) {
        return static_cast<UINT>(key);
    }
    if (key >= Qt::Key_F1 && key <= Qt::Key_F24) {
        return static_cast<UINT>(VK_F1 + key - Qt::Key_F1);
    }

    switch (key) {
    case Qt::Key_Backspace: return VK_BACK;
    case Qt::Key_Tab: return VK_TAB;
    case Qt::Key_Return:
    case Qt::Key_Enter: return VK_RETURN;
    case Qt::Key_Escape: return VK_ESCAPE;
    case Qt::Key_Space: return VK_SPACE;
    case Qt::Key_PageUp: return VK_PRIOR;
    case Qt::Key_PageDown: return VK_NEXT;
    case Qt::Key_End: return VK_END;
    case Qt::Key_Home: return VK_HOME;
    case Qt::Key_Left: return VK_LEFT;
    case Qt::Key_Up: return VK_UP;
    case Qt::Key_Right: return VK_RIGHT;
    case Qt::Key_Down: return VK_DOWN;
    case Qt::Key_Insert: return VK_INSERT;
    case Qt::Key_Delete: return VK_DELETE;
    case Qt::Key_Print: return VK_SNAPSHOT;
    case Qt::Key_Pause: return VK_PAUSE;
    case Qt::Key_CapsLock: return VK_CAPITAL;
    case Qt::Key_NumLock: return VK_NUMLOCK;
    case Qt::Key_ScrollLock: return VK_SCROLL;
    case Qt::Key_Plus: return VK_OEM_PLUS;
    case Qt::Key_Comma: return VK_OEM_COMMA;
    case Qt::Key_Minus: return VK_OEM_MINUS;
    case Qt::Key_Period: return VK_OEM_PERIOD;
    case Qt::Key_Slash: return VK_OEM_2;
    case Qt::Key_Backslash: return VK_OEM_5;
    case Qt::Key_Semicolon: return VK_OEM_1;
    case Qt::Key_Apostrophe: return VK_OEM_7;
    case Qt::Key_BracketLeft: return VK_OEM_4;
    case Qt::Key_BracketRight: return VK_OEM_6;
    case Qt::Key_QuoteLeft: return VK_OEM_3;
    default: return 0;
    }
}

std::optional<NativeHotkey> nativeHotkeyFromSequence(const QKeySequence &sequence)
{
    if (sequence.isEmpty()) {
        return std::nullopt;
    }

    const QKeyCombination combination = sequence[0];
    const int key = combination.key();
    NativeHotkey hotkey;
    hotkey.virtualKey = virtualKeyForQtKey(key);
    if (hotkey.virtualKey == 0) {
        return std::nullopt;
    }

    const Qt::KeyboardModifiers modifiers = combination.keyboardModifiers();
    if (modifiers.testFlag(Qt::ControlModifier)) {
        hotkey.modifiers |= MOD_CONTROL;
    }
    if (modifiers.testFlag(Qt::AltModifier)) {
        hotkey.modifiers |= MOD_ALT;
    }
    if (modifiers.testFlag(Qt::ShiftModifier)) {
        hotkey.modifiers |= MOD_SHIFT;
    }
    if (modifiers.testFlag(Qt::MetaModifier)) {
        hotkey.modifiers |= MOD_WIN;
    }
    hotkey.modifiers |= MOD_NOREPEAT;
    return hotkey;
}

#endif

}  // namespace

WindowsTrayController::WindowsTrayController(QApplication *application, Config config, QObject *parent)
    : QObject(parent)
    , m_application(application)
    , m_config(std::move(config))
{
}

WindowsTrayController::~WindowsTrayController()
{
    unregisterHotkeys();
    // 先释放原生镜像，再释放持有 QAction 的自绘菜单
    delete m_nativeMenu;
    delete m_menu;
}

bool WindowsTrayController::hotkeysSupported()
{
#if defined(Q_OS_WIN)
    return true;
#else
    return shortcuts::GlobalShortcutManager::isAvailable();
#endif
}

void WindowsTrayController::setCaptureCallbacks(Callback capture, Callback fullscreen)
{
    m_captureCallback = std::move(capture);
    m_fullscreenCaptureCallback = std::move(fullscreen);
}

void WindowsTrayController::setDelayedCaptureCallbacks(capture_delay::DelayedCaptureMenuCallbacks callbacks)
{
    m_delayedCaptureCallbacks = std::move(callbacks);
}

void WindowsTrayController::setRecordingRegionCallback(RecordingRegionCallback callback)
{
    m_recordingRegionCallback = std::move(callback);
}

bool WindowsTrayController::start()
{
    if (!m_application) {
        m_errorString = QStringLiteral("QApplication is not available");
        return false;
    }

    m_application->setQuitOnLastWindowClosed(false);

    m_application->setWindowIcon(markshot::ui::applicationIcon());

    // 1. 【托盘】【菜单】自绘菜单承载全部动作，每次打开时按配置刷新明暗主题
    m_menu = new ui::ContextMenu;
    m_menu->setColorScheme(ui::configuredContextMenuScheme());
    connect(m_menu, &ui::ContextMenu::aboutToShow, m_menu, [menu = m_menu] {
        menu->setColorScheme(ui::configuredContextMenuScheme());
    });
    m_menu->addAction(MS_TR("Capture"), this, [this] { triggerCapture(); });
    m_menu->addAction(MS_TR("Fullscreen Capture"), this, [this] { triggerFullscreenCapture(); });
    m_menu->addAction(MS_TR("Pin Clipboard"), this, [] { clipboard::pinContentOrNotify(); });
    if (m_delayedCaptureCallbacks.schedule) {
        m_menu->addMenu(capture_delay::createDelayedCaptureMenu(m_delayedCaptureCallbacks, m_menu));
    }
    m_startRecordingAction = m_menu->addAction(MS_TR("Start Recording"), this, [this] {
        startRecordingFromTray();
    });
    m_menu->addAction(MS_TR("Settings"), this, [] { settings::showSettingsDialog(); });
    m_menu->addAction(MS_TR("Screenshot History"), this, [] { history::showHistoryWindow(); });
    m_menu->addSeparator();
    m_recordingStatusAction = m_menu->addAction(MS_TR("Recording: idle"));
    m_recordingStatusAction->setEnabled(false);
    m_pauseRecordingAction = m_menu->addAction(MS_TR("Pause Recording"), this, [this] {
        togglePauseRecordingFromTray();
    });
    m_pauseRecordingAction->setEnabled(false);
    m_stopRecordingAction = m_menu->addAction(MS_TR("Stop Recording"), this, [this] { stopRecordingFromTray(); });
    m_stopRecordingAction->setEnabled(false);
    m_menu->addSeparator();
    QAction *quitAction = m_menu->addAction(MS_TR("Quit"), m_application, [this] {
        unregisterHotkeys();
        if (m_tray) {
            m_tray->hide();
        }
        m_application->quit();
    });
    m_menu->setActionDanger(quitAction);

    m_tray = new QSystemTrayIcon(trayIcon(), this);
    m_tray->setToolTip(QStringLiteral("Mark Shot"));
#if defined(Q_OS_WIN)
    // 2. 【托盘】【菜单】Windows 托盘右键由应用自行在光标处弹出自绘菜单
    connect(m_tray, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Context) {
            if (m_menu && !m_menu->isVisible()) {
                m_menu->popup(QCursor::pos());
            }
            return;
        }
        if (reason == QSystemTrayIcon::Trigger || reason == QSystemTrayIcon::DoubleClick) {
            triggerCapture();
        }
    });
#else
    // 2. 【托盘】【菜单】Linux 托盘菜单由托盘宿主通过 DBusMenu 渲染，只能交给原生菜单镜像
    m_nativeMenu = ui::createNativeMenuMirror(m_menu);
    m_tray->setContextMenu(m_nativeMenu);
    connect(m_tray, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Trigger || reason == QSystemTrayIcon::DoubleClick) {
            triggerCapture();
        }
    });
#endif
    m_tray->show();

    m_recordingStatusTimer = new QTimer(this);
    m_recordingStatusTimer->setInterval(1000);
    connect(m_recordingStatusTimer, &QTimer::timeout, this, &WindowsTrayController::updateRecordingState);
    connect(&recording::RecordingSessionManager::instance(),
            &recording::RecordingSessionManager::statusChanged,
            this,
            &WindowsTrayController::updateRecordingState);
    updateRecordingState();

    if (m_config.hotkeysEnabled) {
        registerHotkeys();
    }
    return true;
}

QString WindowsTrayController::errorString() const
{
    return m_errorString;
}

bool WindowsTrayController::nativeEventFilter(const QByteArray &eventType, void *message, qintptr *result)
{
    Q_UNUSED(eventType);
    Q_UNUSED(result);

#if defined(Q_OS_WIN)
    const MSG *nativeMessage = static_cast<MSG *>(message);
    if (!nativeMessage || nativeMessage->message != WM_HOTKEY) {
        return false;
    }

    if (nativeMessage->wParam == kCaptureHotkeyId) {
        triggerCapture();
        return true;
    }
    if (nativeMessage->wParam == kStopRecordingHotkeyId) {
        stopRecordingFromTray();
        return true;
    }
    if (nativeMessage->wParam == kPauseRecordingHotkeyId) {
        togglePauseRecordingFromTray();
        return true;
    }
    if (nativeMessage->wParam == kFullscreenHotkeyId) {
        triggerFullscreenCapture();
        return true;
    }
#else
    Q_UNUSED(message);
#endif
    return false;
}

void WindowsTrayController::triggerCapture()
{
    if (m_captureCallback) {
        m_captureCallback();
    }
}

void WindowsTrayController::triggerFullscreenCapture()
{
    if (m_fullscreenCaptureCallback) {
        m_fullscreenCaptureCallback();
    }
}

void WindowsTrayController::startRecordingFromTray()
{
    auto &manager = recording::RecordingSessionManager::instance();
    if (manager.status().active) {
        updateRecordingState();
        return;
    }

    recording::RecordingStartFlowRequest request;
    request.initialMode = recording::RecordingMode::Video;
    request.stayOnTop = true;
    request.startDisplayRecording = [this](recording::RecordingOptions options) {
        recording::ui::runRecordingCountdown(
            options.countdownSeconds,
            options.captureGeometry,
            QGuiApplication::screenAt(options.captureGeometry.center()),
            [this, options] {
                QString error;
                if (!recording::RecordingSessionManager::instance().start(options,
                                                                          m_application,
                                                                          &error)) {
                    if (m_tray) {
                        m_tray->showMessage(QStringLiteral("Mark Shot"),
                                            error.isEmpty() ? MS_TR("Recording failed to start") : error,
                                            QSystemTrayIcon::Warning,
                                            3000);
                    }
                    return;
                }
                updateRecordingState();
            });
    };
    request.selectRegionRecording = [this](recording::RecordingOptions options) {
        if (m_recordingRegionCallback) {
            m_recordingRegionCallback(std::move(options));
            return;
        }
        if (m_tray) {
            m_tray->showMessage(QStringLiteral("Mark Shot"),
                                MS_TR("Failed to start capture session."),
                                QSystemTrayIcon::Warning,
                                3000);
        }
    };
    request.showError = [this](const QString &message) {
        if (m_tray) {
            m_tray->showMessage(QStringLiteral("Mark Shot"), message, QSystemTrayIcon::Warning, 3000);
        }
    };

    recording::runRecordingStartFlow(request);
    updateRecordingState();
}

void WindowsTrayController::stopRecordingFromTray()
{
    QString error;
    if (recording::RecordingSessionManager::instance().stop(&error)) {
        updateRecordingState();
        return;
    }
    if (m_tray && !error.isEmpty()) {
        m_tray->showMessage(QStringLiteral("Mark Shot"), error, QSystemTrayIcon::Information, 3000);
    }
}

void WindowsTrayController::togglePauseRecordingFromTray()
{
    QString error;
    if (recording::RecordingSessionManager::instance().togglePause(&error)) {
        updateRecordingState();
        return;
    }
    if (m_tray && !error.isEmpty()) {
        m_tray->showMessage(QStringLiteral("Mark Shot"), error, QSystemTrayIcon::Information, 3000);
    }
}

void WindowsTrayController::updateRecordingState()
{
    const recording::RecordingStatus status = recording::RecordingSessionManager::instance().status();
    const QString statusText = recordingStatusText(status);

    if (m_recordingStatusAction) {
        m_recordingStatusAction->setText(statusText);
    }
    if (m_stopRecordingAction) {
        m_stopRecordingAction->setEnabled(status.active);
    }
    if (m_pauseRecordingAction) {
        m_pauseRecordingAction->setEnabled(status.active);
        m_pauseRecordingAction->setText(status.paused ? MS_TR("Resume Recording")
                                                      : MS_TR("Pause Recording"));
    }
    if (m_startRecordingAction) {
        m_startRecordingAction->setEnabled(!status.active);
    }
    if (m_tray) {
        m_tray->setToolTip(status.active
                               ? QStringLiteral("Mark Shot - %1").arg(statusText)
                               : QStringLiteral("Mark Shot"));
    }
    if (m_recordingStatusTimer) {
        if (status.active && !m_recordingStatusTimer->isActive()) {
            m_recordingStatusTimer->start();
        } else if (!status.active && m_recordingStatusTimer->isActive()) {
            m_recordingStatusTimer->stop();
        }
    }
}

void WindowsTrayController::registerHotkeys()
{
#if defined(Q_OS_WIN)
    if (!m_application || m_nativeEventFilterInstalled) {
        return;
    }

    m_application->installNativeEventFilter(this);
    m_nativeEventFilterInstalled = true;

    auto registerSequence = [this](int id, const QKeySequence &sequence, bool *registered) {
        if (!registered || sequence.isEmpty()) {
            return;
        }
        const std::optional<NativeHotkey> hotkey = nativeHotkeyFromSequence(sequence);
        if (!hotkey.has_value()) {
            m_errorString = MS_TR("Unsupported Windows hotkey: %1")
                                .arg(sequence.toString(QKeySequence::NativeText));
            markshot::debugLog("windows", "%s", m_errorString.toUtf8().constData());
            return;
        }
        if (RegisterHotKey(nullptr, id, hotkey->modifiers, hotkey->virtualKey)) {
            *registered = true;
            return;
        }

        m_errorString = MS_TR("Failed to register Windows hotkey %1, error %2")
                            .arg(sequence.toString(QKeySequence::NativeText))
                            .arg(static_cast<unsigned long>(GetLastError()));
        markshot::debugLog("windows", "%s", m_errorString.toUtf8().constData());
        if (m_tray) {
            m_tray->showMessage(QStringLiteral("Mark Shot"), m_errorString, QSystemTrayIcon::Warning, 5000);
        }
    };

    registerSequence(kCaptureHotkeyId, m_config.captureHotkey, &m_captureHotkeyRegistered);
    if (m_config.fullscreenHotkey != m_config.captureHotkey) {
        registerSequence(kFullscreenHotkeyId, m_config.fullscreenHotkey, &m_fullscreenHotkeyRegistered);
    }
    registerSequence(kStopRecordingHotkeyId, m_config.stopRecordingHotkey, &m_stopRecordingHotkeyRegistered);
    registerSequence(kPauseRecordingHotkeyId, m_config.pauseRecordingHotkey, &m_pauseRecordingHotkeyRegistered);
#else
    // commandLine 供进程外注册的后端（GNOME gsettings 自定义快捷键）使用：
    // 桌面环境按键后启动新进程，动作经单实例转发路由回本实例。
    const QString selfCommand =
        QStringLiteral("\"%1\"").arg(QCoreApplication::applicationFilePath());
    QList<shortcuts::Shortcut> shortcutList;
    if (!m_config.captureHotkey.isEmpty()) {
        shortcutList.append({QStringLiteral("capture"),
                             MS_TR("Capture"),
                             m_config.captureHotkey,
                             [this] { triggerCapture(); },
                             selfCommand + QStringLiteral(" --capture")});
    }
    if (!m_config.fullscreenHotkey.isEmpty() && m_config.fullscreenHotkey != m_config.captureHotkey) {
        shortcutList.append({QStringLiteral("fullscreen"),
                             MS_TR("Fullscreen Capture"),
                             m_config.fullscreenHotkey,
                             [this] { triggerFullscreenCapture(); },
                             selfCommand + QStringLiteral(" --fullscreen")});
    }

    if (!m_config.stopRecordingHotkey.isEmpty()) {
        shortcutList.append({QStringLiteral("stop-recording"),
                             MS_TR("Stop Recording"),
                             m_config.stopRecordingHotkey,
                             [this] { stopRecordingFromTray(); },
                             selfCommand + QStringLiteral(" --stop-recording")});
    }
    if (!m_config.pauseRecordingHotkey.isEmpty()) {
        shortcutList.append({QStringLiteral("pause-recording"),
                             MS_TR("Pause Recording"),
                             m_config.pauseRecordingHotkey,
                             [this] { togglePauseRecordingFromTray(); },
                             selfCommand + QStringLiteral(" --pause-recording")});
    }

    if (!m_globalShortcuts) {
        m_globalShortcuts = new shortcuts::GlobalShortcutManager(this);
    }
    if (m_globalShortcuts->registerShortcuts(shortcutList)) {
        markshot::debugLog("tray",
                           "【托盘】【全局快捷键】registered through %s backend",
                           m_globalShortcuts->activeBackendName().toUtf8().constData());
        return;
    }

    m_errorString = m_globalShortcuts->errorString();
    if (m_errorString.isEmpty()) {
        m_errorString = MS_TR("Global hotkeys are not supported on this platform. "
                              "Use the tray menu or bind a desktop shortcut instead.");
    }
    markshot::debugLog("tray",
                       "【托盘】【全局快捷键】registration failed: %s",
                       m_errorString.toUtf8().constData());
    if (m_tray) {
        m_tray->showMessage(QStringLiteral("Mark Shot"), m_errorString, QSystemTrayIcon::Information, 5000);
    }
#endif
}

void WindowsTrayController::unregisterHotkeys()
{
#if defined(Q_OS_WIN)
    if (m_captureHotkeyRegistered) {
        UnregisterHotKey(nullptr, kCaptureHotkeyId);
        m_captureHotkeyRegistered = false;
    }
    if (m_fullscreenHotkeyRegistered) {
        UnregisterHotKey(nullptr, kFullscreenHotkeyId);
        m_fullscreenHotkeyRegistered = false;
    }
    if (m_stopRecordingHotkeyRegistered) {
        UnregisterHotKey(nullptr, kStopRecordingHotkeyId);
        m_stopRecordingHotkeyRegistered = false;
    }
    if (m_pauseRecordingHotkeyRegistered) {
        UnregisterHotKey(nullptr, kPauseRecordingHotkeyId);
        m_pauseRecordingHotkeyRegistered = false;
    }
    if (m_application && m_nativeEventFilterInstalled) {
        m_application->removeNativeEventFilter(this);
        m_nativeEventFilterInstalled = false;
    }
#else
    if (m_globalShortcuts) {
        m_globalShortcuts->unregisterShortcuts();
    }
#endif
}

}  // namespace markshot
