#pragma once

#include <QAbstractNativeEventFilter>
#include <QKeySequence>
#include <QObject>
#include <QString>

#include <functional>

class QApplication;
class QMenu;
class QSystemTrayIcon;

namespace markshot {

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

    explicit WindowsTrayController(QApplication *application, Config config, QObject *parent = nullptr);
    ~WindowsTrayController() override;

    static bool isSupported();
    static Config readConfig();

    void setCaptureCallbacks(Callback capture, Callback fullscreen);
    bool start();
    QString errorString() const;

    bool nativeEventFilter(const QByteArray &eventType, void *message, qintptr *result) override;

private:
    void triggerCapture();
    void triggerFullscreenCapture();
    void registerHotkeys();
    void unregisterHotkeys();

    QApplication *m_application = nullptr;
    Config m_config;
    Callback m_captureCallback;
    Callback m_fullscreenCaptureCallback;
    QMenu *m_menu = nullptr;
    QSystemTrayIcon *m_tray = nullptr;
    QString m_errorString;
    bool m_nativeEventFilterInstalled = false;
    bool m_captureHotkeyRegistered = false;
    bool m_fullscreenHotkeyRegistered = false;
};

}  // namespace markshot
