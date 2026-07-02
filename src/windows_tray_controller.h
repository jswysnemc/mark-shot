#pragma once

#include <QAbstractNativeEventFilter>
#include <QKeySequence>
#include <QObject>
#include <QString>

#include "recording/recording_options.h"

#include <functional>

class QApplication;
class QAction;
class QMenu;
class QSystemTrayIcon;
class QTimer;

namespace markshot {

class GlobalShortcutPortal;

class WindowsTrayController final : public QObject, public QAbstractNativeEventFilter {
public:
    struct Config {
#if defined(Q_OS_WIN)
        bool autoStart = true;
#else
        bool autoStart = false;
#endif
        bool hotkeysEnabled = true;
        QKeySequence captureHotkey = QKeySequence(QStringLiteral("Ctrl+Alt+S"));
        QKeySequence fullscreenHotkey;
    };

    using Callback = std::function<void()>;
    using RecordingRegionCallback = std::function<void(recording::RecordingOptions)>;

    explicit WindowsTrayController(QApplication *application, Config config, QObject *parent = nullptr);
    ~WindowsTrayController() override;

    static bool hotkeysSupported();
    static Config readConfig();

    void setCaptureCallbacks(Callback capture, Callback fullscreen);
    void setRecordingRegionCallback(RecordingRegionCallback callback);
    bool start();
    QString errorString() const;

    bool nativeEventFilter(const QByteArray &eventType, void *message, qintptr *result) override;

private:
    void triggerCapture();
    void triggerFullscreenCapture();

    /**
     * 从托盘菜单请求开始录制。
     * @return 无返回值。
     */
    void startRecordingFromTray();

    /**
     * 从托盘菜单请求停止当前录制。
     * @return 无返回值。
     */
    void stopRecordingFromTray();

    /**
     * 刷新托盘中的录制状态和停止动作。
     * @return 无返回值。
     */
    void updateRecordingState();
    void registerHotkeys();
    void unregisterHotkeys();

    QApplication *m_application = nullptr;
    Config m_config;
    Callback m_captureCallback;
    Callback m_fullscreenCaptureCallback;
    RecordingRegionCallback m_recordingRegionCallback;
    QMenu *m_menu = nullptr;
    QSystemTrayIcon *m_tray = nullptr;
    QAction *m_startRecordingAction = nullptr;
    QAction *m_recordingStatusAction = nullptr;
    QAction *m_stopRecordingAction = nullptr;
    QTimer *m_recordingStatusTimer = nullptr;
    QString m_errorString;
    bool m_nativeEventFilterInstalled = false;
    bool m_captureHotkeyRegistered = false;
    bool m_fullscreenHotkeyRegistered = false;
    GlobalShortcutPortal *m_globalShortcutPortal = nullptr;
};

}  // namespace markshot
