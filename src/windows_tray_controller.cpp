#include "windows_tray_controller.h"

#include "config_value.h"
#include "debug_log.h"
#if defined(MARK_SHOT_WITH_DBUS)
#include "global_shortcut_portal.h"
#endif
#include "settings/settings_dialog.h"
#include "shot_window.h"
#include "ui/i18n.h"
#include "ui/icons.h"
#include "window_detection.h"

#include <QAction>
#include <QApplication>
#include <QByteArray>
#include <QFile>
#include <QIcon>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QMenu>
#include <QSystemTrayIcon>

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

/// @brief Applies system tray configurations from a JSON object.
/// @param object The JSON object containing tray configuration values.
/// @param config Pointer to the configuration structure to update.
void applyTrayConfig(const QJsonObject &object, WindowsTrayController::Config *config)
{
    if (!config || object.isEmpty()) {
        return;
    }

    if (const std::optional<bool> enabled = config::boolValue(object.value(QStringLiteral("enabled")))) {
        config->autoStart = *enabled;
    }
    if (const std::optional<bool> autoStart = config::boolValue(object.value(QStringLiteral("autoStart")))) {
        config->autoStart = *autoStart;
    }
    if (const std::optional<bool> startInTray = config::boolValue(object.value(QStringLiteral("startInTray")))) {
        config->autoStart = *startInTray;
    }
    if (const std::optional<bool> hotkeysEnabled = config::boolValue(object.value(QStringLiteral("hotkeysEnabled")))) {
        config->hotkeysEnabled = *hotkeysEnabled;
    }
    if (const std::optional<bool> hotkeyEnabled = config::boolValue(object.value(QStringLiteral("hotkeyEnabled")))) {
        config->hotkeysEnabled = *hotkeyEnabled;
    }
}

/// @brief Applies hotkey-related configurations from a JSON object.
/// @param object The JSON object containing hotkey configuration values.
/// @param config Pointer to the configuration structure to update.
void applyHotkeyConfig(const QJsonObject &object, WindowsTrayController::Config *config)
{
    if (!config || object.isEmpty()) {
        return;
    }

    if (const std::optional<bool> enabled = config::boolValue(object.value(QStringLiteral("enabled")))) {
        config->hotkeysEnabled = *enabled;
    }
    for (const QString &key : {QStringLiteral("capture"),
                               QStringLiteral("screenshot"),
                               QStringLiteral("shot"),
                               QStringLiteral("captureHotkey"),
                               QStringLiteral("hotkey")}) {
        if (const std::optional<QKeySequence> sequence = config::keySequenceValue(object.value(key))) {
            config->captureHotkey = *sequence;
            break;
        }
    }
    for (const QString &key : {QStringLiteral("fullscreen"),
                               QStringLiteral("fullScreen"),
                               QStringLiteral("fullscreenCapture"),
                               QStringLiteral("fullscreenHotkey")}) {
        if (const std::optional<QKeySequence> sequence = config::keySequenceValue(object.value(key))) {
            config->fullscreenHotkey = *sequence;
            break;
        }
    }
}

/// @brief Applies general Windows-specific configurations from a JSON object.
/// @param object The JSON object containing configuration values.
/// @param config Pointer to the configuration structure to update.
void applyWindowsConfig(const QJsonObject &object, WindowsTrayController::Config *config)
{
    if (!config || object.isEmpty()) {
        return;
    }

    applyTrayConfig(config::firstNonEmptyObjectValue(object, {QStringLiteral("tray"), QStringLiteral("systemTray")}), config);
    applyHotkeyConfig(config::firstNonEmptyObjectValue(object, {QStringLiteral("hotkeys"), QStringLiteral("globalHotkeys")}), config);

    if (const std::optional<bool> trayEnabled = config::boolValue(object.value(QStringLiteral("trayEnabled")))) {
        config->autoStart = *trayEnabled;
    }
    if (const std::optional<bool> startInTray = config::boolValue(object.value(QStringLiteral("startInTray")))) {
        config->autoStart = *startInTray;
    }
    if (const std::optional<bool> hotkeysEnabled = config::boolValue(object.value(QStringLiteral("hotkeysEnabled")))) {
        config->hotkeysEnabled = *hotkeysEnabled;
    }
    if (const std::optional<QKeySequence> hotkey = config::keySequenceValue(object.value(QStringLiteral("hotkey")))) {
        config->captureHotkey = *hotkey;
    }
    if (const std::optional<QKeySequence> captureHotkey = config::keySequenceValue(object.value(QStringLiteral("captureHotkey")))) {
        config->captureHotkey = *captureHotkey;
    }
    if (const std::optional<QKeySequence> fullscreenHotkey = config::keySequenceValue(object.value(QStringLiteral("fullscreenHotkey")))) {
        config->fullscreenHotkey = *fullscreenHotkey;
    }
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
    delete m_menu;
}

bool WindowsTrayController::hotkeysSupported()
{
#if defined(Q_OS_WIN)
    return true;
#elif defined(MARK_SHOT_WITH_DBUS)
    return GlobalShortcutPortal::isAvailable();
#else
    return false;
#endif
}

WindowsTrayController::Config WindowsTrayController::readConfig()
{
    Config config;

    QFile file(markshot::appConfigPath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return config;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return config;
    }

    const QJsonObject root = document.object();
    applyTrayConfig(config::firstNonEmptyObjectValue(root, {QStringLiteral("tray"), QStringLiteral("systemTray")}), &config);
    applyHotkeyConfig(config::firstNonEmptyObjectValue(root, {QStringLiteral("globalHotkeys"), QStringLiteral("windowsHotkeys")}), &config);
    applyWindowsConfig(config::objectValue(root, QStringLiteral("windows")), &config);
    return config;
}

void WindowsTrayController::setCaptureCallbacks(Callback capture, Callback fullscreen)
{
    m_captureCallback = std::move(capture);
    m_fullscreenCaptureCallback = std::move(fullscreen);
}

bool WindowsTrayController::start()
{
    if (!m_application) {
        m_errorString = QStringLiteral("QApplication is not available");
        return false;
    }

    m_application->setQuitOnLastWindowClosed(false);

    const QIcon icon = markshot::ui::applicationIcon();
    m_application->setWindowIcon(icon);

    m_menu = new QMenu;
    m_menu->addAction(MS_TR("Capture"), this, [this] { triggerCapture(); });
    m_menu->addAction(MS_TR("Fullscreen Capture"), this, [this] { triggerFullscreenCapture(); });
    m_menu->addAction(MS_TR("Settings"), this, [] { settings::showSettingsDialog(); });
    m_menu->addSeparator();
    m_menu->addAction(MS_TR("Quit"), m_application, [this] {
        unregisterHotkeys();
        if (m_tray) {
            m_tray->hide();
        }
        m_application->quit();
    });

    m_tray = new QSystemTrayIcon(icon, this);
    m_tray->setToolTip(QStringLiteral("Mark Shot"));
    m_tray->setContextMenu(m_menu);
    connect(m_tray, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Trigger || reason == QSystemTrayIcon::DoubleClick) {
            triggerCapture();
        }
    });
    m_tray->show();

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
#elif defined(MARK_SHOT_WITH_DBUS)
    QList<GlobalShortcutPortal::Shortcut> shortcuts;
    if (!m_config.captureHotkey.isEmpty()) {
        shortcuts.append({QStringLiteral("capture"),
                          MS_TR("Capture"),
                          m_config.captureHotkey,
                          [this] { triggerCapture(); }});
    }
    if (!m_config.fullscreenHotkey.isEmpty() && m_config.fullscreenHotkey != m_config.captureHotkey) {
        shortcuts.append({QStringLiteral("fullscreen"),
                          MS_TR("Fullscreen Capture"),
                          m_config.fullscreenHotkey,
                          [this] { triggerFullscreenCapture(); }});
    }

    if (!m_globalShortcutPortal) {
        m_globalShortcutPortal = new GlobalShortcutPortal(this);
    }
    if (m_globalShortcutPortal->registerShortcuts(shortcuts)) {
        markshot::debugLog("tray", "【托盘】【全局快捷键】registered through xdg-desktop-portal");
        return;
    }

    m_errorString = m_globalShortcutPortal->errorString();
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
#else
    m_errorString = MS_TR("Global hotkeys are not supported on this platform. "
                          "Use the tray menu or bind a desktop shortcut instead.");
    markshot::debugLog("tray",
                       "【托盘】【全局快捷键】registration failed: %s",
                       m_errorString.toUtf8().constData());
    if (m_tray) {
        m_tray->showMessage(QStringLiteral("Mark Shot"), m_errorString, QSystemTrayIcon::Information, 4000);
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
    if (m_application && m_nativeEventFilterInstalled) {
        m_application->removeNativeEventFilter(this);
        m_nativeEventFilterInstalled = false;
    }
#elif defined(MARK_SHOT_WITH_DBUS)
    if (m_globalShortcutPortal) {
        m_globalShortcutPortal->unregisterShortcuts();
    }
#endif
}

}  // namespace markshot
