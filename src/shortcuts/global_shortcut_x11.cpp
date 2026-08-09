#include "shortcuts/global_shortcut_x11.h"

#include "debug_log.h"
#include "shortcuts/global_shortcut_keymap.h"
#include "ui/i18n.h"

#include <QGuiApplication>
#include <QStringList>
#include <QtGui/qguiapplication_platform.h>

#include <xcb/xcb.h>

#include <array>
#include <cstdlib>

namespace markshot::shortcuts {
namespace {

/// @brief X11 锁定键掩码，抓取时需要枚举其组合，否则开着 NumLock 快捷键就失效。
constexpr std::uint16_t kCapsLockMask = 1 << 1;
constexpr std::uint16_t kNumLockMask = 1 << 4;

/// @brief xcb 的按键事件响应类型，取自 xcb/xproto.h 的 XCB_KEY_PRESS。
constexpr std::uint8_t kKeyPressResponseType = 2;

/**
 * 返回需要一并抓取的锁定键掩码组合。
 *
 * X11 抓取按键时修饰键掩码必须完全匹配，CapsLock 或 NumLock 打开会让掩码多出
 * 一位而导致快捷键不触发，因此对每种组合都注册一次。
 *
 * @return 四种锁定键掩码组合。
 */
constexpr std::array<std::uint16_t, 4> lockModifierVariants()
{
    return {0, kCapsLockMask, kNumLockMask, static_cast<std::uint16_t>(kCapsLockMask | kNumLockMask)};
}

/**
 * 获取当前 Qt 会话使用的 xcb 连接。
 * @return xcb 连接；当前不是 X11 会话时返回 nullptr。
 */
xcb_connection_t *sessionConnection()
{
    auto *application = qGuiApp;
    if (!application) {
        return nullptr;
    }
    auto *x11 = application->nativeInterface<QNativeInterface::QX11Application>();
    return x11 ? x11->connection() : nullptr;
}

/**
 * 获取默认屏幕的根窗口。
 * @param connection xcb 连接。
 * @return 根窗口标识；获取失败时返回 0。
 */
xcb_window_t rootWindow(xcb_connection_t *connection)
{
    if (!connection) {
        return 0;
    }
    const xcb_setup_t *setup = xcb_get_setup(connection);
    if (!setup) {
        return 0;
    }
    xcb_screen_iterator_t iterator = xcb_setup_roots_iterator(setup);
    if (!iterator.data) {
        return 0;
    }
    return iterator.data->root;
}

/**
 * 把键码与修饰键掩码合成查表键。
 * @param keycode X11 键码。
 * @param modifiers 修饰键掩码。
 * @return 合成后的查表键。
 */
quint32 bindingKey(std::uint8_t keycode, std::uint16_t modifiers)
{
    return (static_cast<quint32>(modifiers) << 8) | keycode;
}

}  // namespace

X11GlobalShortcutBackend::X11GlobalShortcutBackend(QObject *parent)
    : QObject(parent)
{
}

X11GlobalShortcutBackend::~X11GlobalShortcutBackend()
{
    unregisterShortcuts();
}

bool X11GlobalShortcutBackend::isAvailable()
{
    return sessionConnection() != nullptr;
}

std::uint8_t X11GlobalShortcutBackend::keycodeForKeysym(std::uint32_t keysym) const
{
    xcb_connection_t *connection = sessionConnection();
    if (!connection) {
        return 0;
    }

    const xcb_setup_t *setup = xcb_get_setup(connection);
    if (!setup) {
        return 0;
    }

    // 1. 读取整张键盘映射表，逐个键码比对其绑定的 keysym
    const std::uint8_t minKeycode = setup->min_keycode;
    const std::uint8_t maxKeycode = setup->max_keycode;
    const std::uint8_t keycodeCount = static_cast<std::uint8_t>(maxKeycode - minKeycode + 1);

    xcb_get_keyboard_mapping_cookie_t cookie =
        xcb_get_keyboard_mapping(connection, minKeycode, keycodeCount);
    xcb_get_keyboard_mapping_reply_t *reply =
        xcb_get_keyboard_mapping_reply(connection, cookie, nullptr);
    if (!reply) {
        return 0;
    }

    const xcb_keysym_t *keysyms = xcb_get_keyboard_mapping_keysyms(reply);
    const int keysymCount = xcb_get_keyboard_mapping_keysyms_length(reply);
    const int perKeycode = reply->keysyms_per_keycode;

    std::uint8_t found = 0;
    if (keysyms && perKeycode > 0) {
        // 2. 只比对每个键码的第一个 keysym，即未叠加 Shift 时的符号
        for (int index = 0; index < keysymCount && found == 0; index += perKeycode) {
            if (keysyms[index] == keysym) {
                found = static_cast<std::uint8_t>(minKeycode + index / perKeycode);
            }
        }
    }

    free(reply);
    return found;
}

bool X11GlobalShortcutBackend::grabKey(std::uint32_t keysym,
                                       std::uint16_t modifiers,
                                       const QString &shortcutId)
{
    xcb_connection_t *connection = sessionConnection();
    const xcb_window_t root = rootWindow(connection);
    if (!connection || root == 0) {
        return false;
    }

    const std::uint8_t keycode = keycodeForKeysym(keysym);
    if (keycode == 0) {
        return false;
    }

    // 1. 对每种锁定键组合各抓取一次，确保 CapsLock / NumLock 打开时仍能触发
    bool anyGrabbed = false;
    for (const std::uint16_t lockMask : lockModifierVariants()) {
        const std::uint16_t effectiveModifiers = static_cast<std::uint16_t>(modifiers | lockMask);
        xcb_void_cookie_t cookie = xcb_grab_key_checked(connection,
                                                        1,  // owner_events：事件仍投递给拥有者
                                                        root,
                                                        effectiveModifiers,
                                                        keycode,
                                                        XCB_GRAB_MODE_ASYNC,
                                                        XCB_GRAB_MODE_ASYNC);
        xcb_generic_error_t *error = xcb_request_check(connection, cookie);
        if (error) {
            // 已被其他程序占用时该组合抓取失败，跳过但不影响其余组合
            free(error);
            continue;
        }
        m_grabbedKeys.append({keycode, effectiveModifiers});
        m_shortcutIdByBinding.insert(bindingKey(keycode, effectiveModifiers), shortcutId);
        anyGrabbed = true;
    }

    return anyGrabbed;
}

void X11GlobalShortcutBackend::installEventFilter()
{
    if (m_eventFilterInstalled || !qGuiApp) {
        return;
    }
    qGuiApp->installNativeEventFilter(this);
    m_eventFilterInstalled = true;
}

bool X11GlobalShortcutBackend::registerShortcuts(const QList<Shortcut> &shortcuts)
{
    m_errorString.clear();
    unregisterShortcuts();

    xcb_connection_t *connection = sessionConnection();
    if (!connection) {
        m_errorString = MS_TR("The X11 global shortcut backend is unavailable in this session.");
        return false;
    }

    // 1. 逐条转换并抓取，单条失败不影响其余快捷键
    QStringList failed;
    for (const Shortcut &shortcut : shortcuts) {
        if (shortcut.id.isEmpty() || !shortcut.callback) {
            continue;
        }

        const X11KeyBinding binding = x11BindingForSequence(shortcut.sequence);
        if (!binding.valid) {
            failed.append(shortcut.sequence.toString(QKeySequence::NativeText));
            continue;
        }
        if (!grabKey(binding.keysym, binding.modifiers, shortcut.id)) {
            failed.append(shortcut.sequence.toString(QKeySequence::NativeText));
            continue;
        }
        m_callbacks.insert(shortcut.id, shortcut.callback);
    }

    // 2. 抓取请求是异步发出的，这里刷新连接确保请求送达 X 服务器
    xcb_flush(connection);

    if (m_callbacks.isEmpty()) {
        unregisterShortcuts();
        m_errorString = MS_TR("Failed to grab any global shortcut from the X server. "
                              "Another application may already use the same key combination.");
        return false;
    }

    if (!failed.isEmpty()) {
        markshot::debugLog("shortcuts",
                           "【全局快捷键】【X11】partially registered, unavailable combinations: %s",
                           failed.join(QStringLiteral(", ")).toUtf8().constData());
    }

    installEventFilter();
    return true;
}

void X11GlobalShortcutBackend::unregisterShortcuts()
{
    xcb_connection_t *connection = sessionConnection();
    if (connection) {
        const xcb_window_t root = rootWindow(connection);
        if (root != 0) {
            for (const GrabbedKey &grabbed : m_grabbedKeys) {
                xcb_ungrab_key(connection, grabbed.keycode, root, grabbed.modifiers);
            }
            xcb_flush(connection);
        }
    }

    m_grabbedKeys.clear();
    m_shortcutIdByBinding.clear();
    m_callbacks.clear();

    if (m_eventFilterInstalled && qGuiApp) {
        qGuiApp->removeNativeEventFilter(this);
        m_eventFilterInstalled = false;
    }
}

QString X11GlobalShortcutBackend::errorString() const
{
    return m_errorString;
}

QString X11GlobalShortcutBackend::backendName() const
{
    return QStringLiteral("x11");
}

bool X11GlobalShortcutBackend::nativeEventFilter(const QByteArray &eventType,
                                                 void *message,
                                                 qintptr *result)
{
    Q_UNUSED(result);

    if (eventType != QByteArrayLiteral("xcb_generic_event_t") || !message) {
        return false;
    }

    auto *event = static_cast<xcb_generic_event_t *>(message);
    // response_type 的最高位标记事件是否由 SendEvent 合成，比对前需要屏蔽
    if ((event->response_type & 0x7f) != kKeyPressResponseType) {
        return false;
    }

    auto *keyPress = reinterpret_cast<xcb_key_press_event_t *>(event);
    const QString shortcutId =
        m_shortcutIdByBinding.value(bindingKey(keyPress->detail, keyPress->state));
    if (shortcutId.isEmpty()) {
        return false;
    }

    const auto callback = m_callbacks.value(shortcutId);
    if (!callback) {
        return false;
    }

    callback();
    return true;
}

}  // namespace markshot::shortcuts
