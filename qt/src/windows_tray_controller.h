#pragma once

#include <QAbstractNativeEventFilter>
#include <QKeySequence>
#include <QObject>
#include <QString>

#include "capture_delay/delayed_capture_tray_menu.h"
#include "recording/recording_options.h"

#include <functional>

class QApplication;
class QAction;
class QMenu;
class QSystemTrayIcon;
class QTimer;

namespace markshot {

namespace shortcuts {
class GlobalShortcutManager;
}

namespace ui {
class ContextMenu;
}

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
        QKeySequence stopRecordingHotkey;
        QKeySequence pauseRecordingHotkey;
    };

    using Callback = std::function<void()>;
    using RecordingRegionCallback = std::function<void(recording::RecordingOptions)>;

    explicit WindowsTrayController(QApplication *application, Config config, QObject *parent = nullptr);
    ~WindowsTrayController() override;

    static bool hotkeysSupported();
    static Config readConfig();

    void setCaptureCallbacks(Callback capture, Callback fullscreen);

    /**
     * 设置托盘「延时截图」子菜单的调度回调，需在 start() 之前调用。
     * @param callbacks 计划、取消与查询回调。
     * @return 无返回值。
     */
    void setDelayedCaptureCallbacks(capture_delay::DelayedCaptureMenuCallbacks callbacks);
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

    /// @brief 在托盘中切换录制暂停状态。
    void togglePauseRecordingFromTray();

    /**
     * 刷新托盘中的录制状态和停止动作。
     * @return 无返回值。
     */
    void updateRecordingState();
    void registerHotkeys();
    void unregisterHotkeys();

    /**
     * 【托盘】【全局快捷键】设置保存后重新读取配置，快捷键变化时注销旧组合并注册新组合。
     * @return 无返回值。
     */
    void reloadHotkeys();

    QApplication *m_application = nullptr;
    Config m_config;
    Callback m_captureCallback;
    Callback m_fullscreenCaptureCallback;
    capture_delay::DelayedCaptureMenuCallbacks m_delayedCaptureCallbacks;
    RecordingRegionCallback m_recordingRegionCallback;
    // 自绘托盘菜单，持有全部 QAction
    ui::ContextMenu *m_menu = nullptr;
    // 平台托盘（DBusMenu 导出）需要的原生菜单镜像，与 m_menu 共享 QAction
    QMenu *m_nativeMenu = nullptr;
    QSystemTrayIcon *m_tray = nullptr;
    QAction *m_startRecordingAction = nullptr;
    QAction *m_recordingStatusAction = nullptr;
    QAction *m_stopRecordingAction = nullptr;
    QAction *m_pauseRecordingAction = nullptr;
    QTimer *m_recordingStatusTimer = nullptr;
    QString m_errorString;
    bool m_nativeEventFilterInstalled = false;
    bool m_captureHotkeyRegistered = false;
    bool m_fullscreenHotkeyRegistered = false;
    bool m_stopRecordingHotkeyRegistered = false;
    bool m_pauseRecordingHotkeyRegistered = false;
    shortcuts::GlobalShortcutManager *m_globalShortcuts = nullptr;
};

}  // namespace markshot
