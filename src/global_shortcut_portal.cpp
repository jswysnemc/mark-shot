#include "global_shortcut_portal.h"

#include "debug_log.h"
#include "ui/i18n.h"

#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusError>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusMetaType>
#include <QDBusObjectPath>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusVariant>
#include <QEventLoop>
#include <QFile>
#include <QGuiApplication>
#include <QMetaType>
#include <QSet>
#include <QStringList>
#include <QTimer>
#include <QUuid>

#include <mutex>
#include <utility>

namespace markshot {

struct PortalShortcutEntry {
    QString id;
    QVariantMap options;
};

using PortalShortcutEntryList = QList<PortalShortcutEntry>;

}  // namespace markshot

Q_DECLARE_METATYPE(markshot::PortalShortcutEntry)
Q_DECLARE_METATYPE(markshot::PortalShortcutEntryList)

namespace markshot {

QDBusArgument &operator<<(QDBusArgument &argument, const PortalShortcutEntry &shortcut)
{
    argument.beginStructure();
    argument << shortcut.id << shortcut.options;
    argument.endStructure();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, PortalShortcutEntry &shortcut)
{
    argument.beginStructure();
    argument >> shortcut.id >> shortcut.options;
    argument.endStructure();
    return argument;
}

class ShortcutPortalResponseReceiver : public QObject {
    Q_OBJECT

public:
    bool received = false;
    uint response = 2;
    QVariantMap results;

public slots:
    /// @brief 处理 Portal Request.Response 信号。
    /// @param responseCode Portal 返回码，0 表示成功。
    /// @param responseResults Portal 返回的结果字典。
    void handleResponse(uint responseCode, const QVariantMap &responseResults)
    {
        received = true;
        response = responseCode;
        results = responseResults;
        emit finished();
    }

signals:
    void finished();
};

namespace {

constexpr int kPortalRequestTimeoutMs = 120000;

struct PortalRequestResult {
    bool ok = false;
    QVariantMap results;
    QString error;
};

struct PortalShortcutIdResult {
    bool parsed = false;
    QSet<QString> ids;
};

/// @brief 生成可用于 Portal handle_token 的随机标识。
/// @return Portal 请求标识。
QString shortcutPortalToken()
{
    QString token = QUuid::createUuid().toString(QUuid::WithoutBraces);
    token.replace(QLatin1Char('-'), QLatin1Char('_'));
    return QStringLiteral("mark_shot_%1").arg(token);
}

/// @brief 根据 handle_token 计算 Portal Request 信号路径。
/// @param handleToken 调用 Portal 方法时传入的 handle_token。
/// @return 预期的 Request 对象路径。
QString shortcutPortalRequestPath(const QString &handleToken)
{
    const QString connectionName =
        QDBusConnection::sessionBus().baseService().mid(1).replace(QLatin1Char('.'), QLatin1Char('_'));
    return QStringLiteral("/org/freedesktop/portal/desktop/request/%1/%2").arg(connectionName, handleToken);
}

/// @brief 解开 QDBusVariant 包装，便于读取 Portal 返回值。
/// @param value 原始 QVariant。
/// @return 解包后的 QVariant。
QVariant unwrappedPortalVariant(QVariant value)
{
    while (value.canConvert<QDBusVariant>()) {
        const QVariant nested = qvariant_cast<QDBusVariant>(value).variant();
        if (!nested.isValid() || nested == value) {
            break;
        }
        value = nested;
    }
    return value;
}

/// @brief 从 Portal BindShortcuts 返回值中读取已绑定快捷键标识。
/// @param value shortcuts 字段的原始 QVariant。
/// @return 解析结果；无法识别返回类型时 parsed 为 false。
PortalShortcutIdResult boundShortcutIdsFromVariant(const QVariant &value)
{
    PortalShortcutIdResult result;
    const QVariant unwrapped = unwrappedPortalVariant(value);
    if (!unwrapped.isValid()) {
        return result;
    }

    PortalShortcutEntryList entries;
    if (unwrapped.canConvert<PortalShortcutEntryList>()) {
        entries = qvariant_cast<PortalShortcutEntryList>(unwrapped);
        result.parsed = true;
    } else if (unwrapped.metaType() == QMetaType::fromType<QDBusArgument>()) {
        const QDBusArgument argument = qvariant_cast<QDBusArgument>(unwrapped);
        argument >> entries;
        result.parsed = true;
    }

    if (!result.parsed) {
        return result;
    }

    for (const PortalShortcutEntry &entry : std::as_const(entries)) {
        if (!entry.id.isEmpty()) {
            result.ids.insert(entry.id);
        }
    }
    return result;
}

/// @brief 将当前宿主应用注册给 host portal，便于未沙箱应用使用 Portal 权限。
void registerShortcutHostPortalApplication()
{
    static std::once_flag flag;
    std::call_once(flag, [] {
        if (QFile::exists(QStringLiteral("/.flatpak-info")) || qEnvironmentVariableIsSet("SNAP")) {
            return;
        }

        const QString desktopFileName = QGuiApplication::desktopFileName();
        if (desktopFileName.isEmpty()) {
            return;
        }

        QDBusMessage message = QDBusMessage::createMethodCall(QStringLiteral("org.freedesktop.portal.Desktop"),
                                                              QStringLiteral("/org/freedesktop/portal/desktop"),
                                                              QStringLiteral("org.freedesktop.host.portal.Registry"),
                                                              QStringLiteral("Register"));
        message << desktopFileName << QVariantMap();

        const QDBusMessage reply = QDBusConnection::sessionBus().call(message, QDBus::Block, 3000);
        if (reply.type() != QDBusMessage::ErrorMessage) {
            return;
        }

        const QDBusError error(reply);
        if (error.type() == QDBusError::UnknownInterface || error.type() == QDBusError::UnknownMethod) {
            return;
        }
        if (error.name() == QStringLiteral("org.freedesktop.portal.Error.Failed")
            && error.message().contains(QStringLiteral("Connection already associated"))) {
            return;
        }
        markshot::debugLog("tray",
                           "【托盘】【全局快捷键】host portal registration failed: %s",
                           error.message().toUtf8().constData());
    });
}

/// @brief 连接指定 Portal Request 路径的 Response 信号。
/// @param signalPath Request 对象路径。
/// @param receiver 接收 Response 的对象。
/// @return 连接成功返回 true，否则返回 false。
bool connectShortcutPortalResponse(const QString &signalPath, ShortcutPortalResponseReceiver *receiver)
{
    return QDBusConnection::sessionBus().connect(QStringLiteral("org.freedesktop.portal.Desktop"),
                                                 signalPath,
                                                 QStringLiteral("org.freedesktop.portal.Request"),
                                                 QStringLiteral("Response"),
                                                 receiver,
                                                 SLOT(handleResponse(uint,QVariantMap)));
}

/// @brief 断开指定 Portal Request 路径的 Response 信号。
/// @param signalPath Request 对象路径。
/// @param receiver 接收 Response 的对象。
void disconnectShortcutPortalResponse(const QString &signalPath, ShortcutPortalResponseReceiver *receiver)
{
    QDBusConnection::sessionBus().disconnect(QStringLiteral("org.freedesktop.portal.Desktop"),
                                             signalPath,
                                             QStringLiteral("org.freedesktop.portal.Request"),
                                             QStringLiteral("Response"),
                                             receiver,
                                             SLOT(handleResponse(uint,QVariantMap)));
}

/// @brief 等待 Portal Request.Response 信号。
/// @param receiver 已连接 Response 信号的接收对象。
/// @return Portal 请求结果。
PortalRequestResult waitForShortcutPortalResponse(ShortcutPortalResponseReceiver *receiver)
{
    if (!receiver->received) {
        QEventLoop loop;
        QTimer timeout;
        timeout.setSingleShot(true);
        QObject::connect(receiver, &ShortcutPortalResponseReceiver::finished, &loop, &QEventLoop::quit);
        QObject::connect(&timeout, &QTimer::timeout, &loop, &QEventLoop::quit);
        timeout.start(kPortalRequestTimeoutMs);
        loop.exec();
    }

    if (!receiver->received) {
        return {false, {}, QStringLiteral("xdg-desktop-portal global shortcut request timed out")};
    }
    if (receiver->response == 0) {
        return {true, receiver->results, {}};
    }
    if (receiver->response == 1) {
        return {false, {}, QStringLiteral("global shortcut request was cancelled")};
    }
    return {false, {}, QStringLiteral("global shortcut request failed with response code %1").arg(receiver->response)};
}

/// @brief 调用带 Request.Response 流程的 Portal 方法。
/// @param portal Portal D-Bus 接口。
/// @param method 要调用的方法名。
/// @param arguments 方法参数。
/// @param errorPrefix 错误前缀。
/// @return Portal 请求结果。
PortalRequestResult callShortcutPortalRequest(QDBusInterface *portal,
                                              const QString &method,
                                              const QVariantList &arguments,
                                              const QString &errorPrefix)
{
    if (!portal || !portal->isValid()) {
        return {false, {}, QStringLiteral("%1 interface is not available").arg(errorPrefix)};
    }

    QString expectedSignalPath;
    for (auto it = arguments.crbegin(); it != arguments.crend(); ++it) {
        const QVariantMap options = it->toMap();
        const QString handleToken = options.value(QStringLiteral("handle_token")).toString();
        if (!handleToken.isEmpty()) {
            expectedSignalPath = shortcutPortalRequestPath(handleToken);
            break;
        }
    }

    ShortcutPortalResponseReceiver receiver;
    if (!expectedSignalPath.isEmpty()
        && !connectShortcutPortalResponse(expectedSignalPath, &receiver)) {
        return {false,
                {},
                QStringLiteral("%1: failed to connect to xdg-desktop-portal response signal").arg(errorPrefix)};
    }

    QDBusPendingReply<QDBusObjectPath> pending = portal->asyncCallWithArgumentList(method, arguments);
    QDBusPendingCallWatcher watcher(pending);
    QEventLoop callLoop;
    QObject::connect(&watcher, &QDBusPendingCallWatcher::finished, &callLoop, &QEventLoop::quit);
    callLoop.exec();

    pending = watcher;
    if (pending.isError()) {
        if (!expectedSignalPath.isEmpty()) {
            disconnectShortcutPortalResponse(expectedSignalPath, &receiver);
        }
        return {false, {}, QStringLiteral("%1: %2").arg(errorPrefix, pending.error().message())};
    }

    const QString returnedSignalPath = pending.value().path();
    if (expectedSignalPath.isEmpty()) {
        expectedSignalPath = returnedSignalPath;
        if (!receiver.received
            && !connectShortcutPortalResponse(expectedSignalPath, &receiver)) {
            return {false,
                    {},
                    QStringLiteral("%1: failed to connect to xdg-desktop-portal response signal").arg(errorPrefix)};
        }
    } else if (returnedSignalPath != expectedSignalPath && !receiver.received) {
        connectShortcutPortalResponse(returnedSignalPath, &receiver);
    }

    PortalRequestResult result = waitForShortcutPortalResponse(&receiver);
    disconnectShortcutPortalResponse(expectedSignalPath, &receiver);
    if (returnedSignalPath != expectedSignalPath) {
        disconnectShortcutPortalResponse(returnedSignalPath, &receiver);
    }
    if (!result.ok && !result.error.isEmpty()) {
        result.error = QStringLiteral("%1: %2").arg(errorPrefix, result.error);
    }
    return result;
}

/// @brief 注册 Portal 快捷键列表需要的 D-Bus 元类型。
void registerShortcutPortalMetaTypes()
{
    static bool registered = [] {
        qRegisterMetaType<PortalShortcutEntry>("PortalShortcutEntry");
        qRegisterMetaType<PortalShortcutEntryList>("PortalShortcutEntryList");
        qDBusRegisterMetaType<PortalShortcutEntry>();
        qDBusRegisterMetaType<PortalShortcutEntryList>();
        return true;
    }();
    Q_UNUSED(registered);
}

/// @brief 将 Qt 键值转换为 XDG Shortcuts 规范中的按键名。
/// @param key Qt 键值。
/// @return 转换后的按键名；不支持时为空字符串。
QString xdgShortcutKeyName(int key)
{
    if (key >= Qt::Key_A && key <= Qt::Key_Z) {
        const QChar letter(QLatin1Char(static_cast<char>('a' + key - Qt::Key_A)));
        return QString(letter);
    }
    if (key >= Qt::Key_0 && key <= Qt::Key_9) {
        const QChar digit(QLatin1Char(static_cast<char>('0' + key - Qt::Key_0)));
        return QString(digit);
    }
    if (key >= Qt::Key_F1 && key <= Qt::Key_F24) {
        return QStringLiteral("F%1").arg(key - Qt::Key_F1 + 1);
    }

    switch (key) {
    case Qt::Key_Backspace: return QStringLiteral("BackSpace");
    case Qt::Key_Tab: return QStringLiteral("Tab");
    case Qt::Key_Return:
    case Qt::Key_Enter: return QStringLiteral("Return");
    case Qt::Key_Escape: return QStringLiteral("Escape");
    case Qt::Key_Space: return QStringLiteral("space");
    case Qt::Key_PageUp: return QStringLiteral("Page_Up");
    case Qt::Key_PageDown: return QStringLiteral("Page_Down");
    case Qt::Key_End: return QStringLiteral("End");
    case Qt::Key_Home: return QStringLiteral("Home");
    case Qt::Key_Left: return QStringLiteral("Left");
    case Qt::Key_Up: return QStringLiteral("Up");
    case Qt::Key_Right: return QStringLiteral("Right");
    case Qt::Key_Down: return QStringLiteral("Down");
    case Qt::Key_Insert: return QStringLiteral("Insert");
    case Qt::Key_Delete: return QStringLiteral("Delete");
    case Qt::Key_Print: return QStringLiteral("Print");
    case Qt::Key_Pause: return QStringLiteral("Pause");
    case Qt::Key_CapsLock: return QStringLiteral("Caps_Lock");
    case Qt::Key_NumLock: return QStringLiteral("Num_Lock");
    case Qt::Key_ScrollLock: return QStringLiteral("Scroll_Lock");
    case Qt::Key_Plus: return QStringLiteral("plus");
    case Qt::Key_Comma: return QStringLiteral("comma");
    case Qt::Key_Minus: return QStringLiteral("minus");
    case Qt::Key_Period: return QStringLiteral("period");
    case Qt::Key_Slash: return QStringLiteral("slash");
    case Qt::Key_Backslash: return QStringLiteral("backslash");
    case Qt::Key_Semicolon: return QStringLiteral("semicolon");
    case Qt::Key_Apostrophe: return QStringLiteral("apostrophe");
    case Qt::Key_BracketLeft: return QStringLiteral("bracketleft");
    case Qt::Key_BracketRight: return QStringLiteral("bracketright");
    case Qt::Key_QuoteLeft: return QStringLiteral("grave");
    default: return {};
    }
}

/// @brief 将 Qt 快捷键序列转换为 XDG Shortcuts 触发字符串。
/// @param sequence Qt 快捷键序列。
/// @return 转换后的触发字符串；无法转换时为空字符串。
QString preferredTriggerForSequence(const QKeySequence &sequence)
{
    if (sequence.isEmpty()) {
        return {};
    }

    const QKeyCombination combination = sequence[0];
    const QString keyName = xdgShortcutKeyName(combination.key());
    if (keyName.isEmpty()) {
        return {};
    }

    QStringList parts;
    const Qt::KeyboardModifiers modifiers = combination.keyboardModifiers();
    if (modifiers.testFlag(Qt::ControlModifier)) {
        parts.append(QStringLiteral("CTRL"));
    }
    if (modifiers.testFlag(Qt::AltModifier)) {
        parts.append(QStringLiteral("ALT"));
    }
    if (modifiers.testFlag(Qt::ShiftModifier)) {
        parts.append(QStringLiteral("SHIFT"));
    }
    if (modifiers.testFlag(Qt::MetaModifier)) {
        parts.append(QStringLiteral("LOGO"));
    }
    parts.append(keyName);
    return parts.join(QLatin1Char('+'));
}

/// @brief 创建 Portal GlobalShortcuts 接口对象。
/// @return Portal D-Bus 接口。
QDBusInterface createGlobalShortcutsInterface()
{
    return QDBusInterface(QStringLiteral("org.freedesktop.portal.Desktop"),
                          QStringLiteral("/org/freedesktop/portal/desktop"),
                          QStringLiteral("org.freedesktop.portal.GlobalShortcuts"),
                          QDBusConnection::sessionBus());
}

}  // namespace

GlobalShortcutPortal::GlobalShortcutPortal(QObject *parent)
    : QObject(parent)
{
}

GlobalShortcutPortal::~GlobalShortcutPortal()
{
    unregisterShortcuts();
}

bool GlobalShortcutPortal::isAvailable()
{
    if (!QDBusConnection::sessionBus().isConnected()) {
        return false;
    }
    QDBusInterface portal = createGlobalShortcutsInterface();
    return portal.isValid();
}

bool GlobalShortcutPortal::registerShortcuts(const QList<Shortcut> &shortcuts)
{
    m_errorString.clear();
    unregisterShortcuts();

    QList<Shortcut> validShortcuts;
    for (const Shortcut &shortcut : shortcuts) {
        if (!shortcut.id.isEmpty() && shortcut.callback) {
            validShortcuts.append(shortcut);
        }
    }
    if (validShortcuts.isEmpty()) {
        m_errorString = MS_TR("No global shortcuts are configured.");
        return false;
    }

    // 1. 创建会话，Portal 要求所有快捷键都绑定到一个会话
    if (!createSession()) {
        return false;
    }

    // 2. 连接激活信号，确保绑定完成后能接收快捷键事件
    if (!connectActivationSignal()) {
        unregisterShortcuts();
        return false;
    }

    // 3. 绑定动作列表，Portal 可能在这里弹出系统授权窗口
    QSet<QString> boundShortcutIds;
    if (!bindShortcuts(validShortcuts, &boundShortcutIds)) {
        unregisterShortcuts();
        return false;
    }

    for (const Shortcut &shortcut : std::as_const(validShortcuts)) {
        if (!boundShortcutIds.isEmpty() && !boundShortcutIds.contains(shortcut.id)) {
            continue;
        }
        m_callbacks.insert(shortcut.id, shortcut.callback);
    }
    return true;
}

void GlobalShortcutPortal::unregisterShortcuts()
{
    if (m_activationSignalConnected) {
        QDBusConnection::sessionBus().disconnect(QStringLiteral("org.freedesktop.portal.Desktop"),
                                                 QStringLiteral("/org/freedesktop/portal/desktop"),
                                                 QStringLiteral("org.freedesktop.portal.GlobalShortcuts"),
                                                 QStringLiteral("Activated"),
                                                 this,
                                                 SLOT(handleActivated(QDBusObjectPath,QString,qulonglong,QVariantMap)));
        m_activationSignalConnected = false;
    }

    if (!m_sessionHandle.path().isEmpty()) {
        QDBusMessage closeMessage = QDBusMessage::createMethodCall(QStringLiteral("org.freedesktop.portal.Desktop"),
                                                                   m_sessionHandle.path(),
                                                                   QStringLiteral("org.freedesktop.portal.Session"),
                                                                   QStringLiteral("Close"));
        QDBusConnection::sessionBus().asyncCall(closeMessage);
        m_sessionHandle = QDBusObjectPath();
    }
    m_callbacks.clear();
}

QString GlobalShortcutPortal::errorString() const
{
    return m_errorString;
}

void GlobalShortcutPortal::handleActivated(QDBusObjectPath sessionHandle,
                                           QString shortcutId,
                                           qulonglong timestamp,
                                           QVariantMap options)
{
    Q_UNUSED(timestamp);
    Q_UNUSED(options);

    if (sessionHandle.path() != m_sessionHandle.path()) {
        return;
    }

    const auto callback = m_callbacks.value(shortcutId);
    if (callback) {
        callback();
    }
}

bool GlobalShortcutPortal::createSession()
{
    registerShortcutPortalMetaTypes();
    registerShortcutHostPortalApplication();

    QDBusInterface portal = createGlobalShortcutsInterface();
    if (!portal.isValid()) {
        m_errorString = MS_TR("Global shortcuts portal is not available. "
                              "Use the tray menu or bind a desktop shortcut instead.");
        return false;
    }

    QVariantMap options;
    options.insert(QStringLiteral("handle_token"), shortcutPortalToken());
    options.insert(QStringLiteral("session_handle_token"), shortcutPortalToken());

    const PortalRequestResult result =
        callShortcutPortalRequest(&portal,
                                  QStringLiteral("CreateSession"),
                                  {options},
                                  QStringLiteral("GlobalShortcuts CreateSession"));
    if (!result.ok) {
        m_errorString = MS_TR("Failed to create global shortcut session: %1").arg(result.error);
        return false;
    }

    const QString sessionPath =
        unwrappedPortalVariant(result.results.value(QStringLiteral("session_handle"))).toString();
    if (sessionPath.isEmpty() || !sessionPath.startsWith(QLatin1Char('/'))) {
        m_errorString = MS_TR("Global shortcuts portal returned an invalid session.");
        return false;
    }

    m_sessionHandle = QDBusObjectPath(sessionPath);
    return true;
}

bool GlobalShortcutPortal::bindShortcuts(const QList<Shortcut> &shortcuts, QSet<QString> *boundShortcutIds)
{
    if (m_sessionHandle.path().isEmpty()) {
        m_errorString = MS_TR("Global shortcuts portal returned an invalid session.");
        return false;
    }

    PortalShortcutEntryList entries;
    for (const Shortcut &shortcut : shortcuts) {
        QVariantMap options;
        options.insert(QStringLiteral("description"), shortcut.description);

        const QString preferredTrigger = preferredTriggerForSequence(shortcut.sequence);
        if (!preferredTrigger.isEmpty()) {
            options.insert(QStringLiteral("preferred_trigger"), preferredTrigger);
        }

        entries.append({shortcut.id, options});
    }

    QVariantMap bindOptions;
    bindOptions.insert(QStringLiteral("handle_token"), shortcutPortalToken());

    QDBusInterface portal = createGlobalShortcutsInterface();
    const PortalRequestResult result =
        callShortcutPortalRequest(&portal,
                                  QStringLiteral("BindShortcuts"),
                                  {QVariant::fromValue(m_sessionHandle),
                                   QVariant::fromValue(entries),
                                   QString(),
                                   bindOptions},
                                  QStringLiteral("GlobalShortcuts BindShortcuts"));
    if (!result.ok) {
        m_errorString = MS_TR("Failed to bind global shortcuts: %1").arg(result.error);
        return false;
    }

    const PortalShortcutIdResult boundIds =
        boundShortcutIdsFromVariant(result.results.value(QStringLiteral("shortcuts")));
    if (boundIds.parsed && boundIds.ids.isEmpty()) {
        m_errorString = MS_TR("Global shortcuts portal did not bind any shortcuts.");
        return false;
    }
    if (boundShortcutIds && boundIds.parsed) {
        *boundShortcutIds = boundIds.ids;
    }
    return true;
}

bool GlobalShortcutPortal::connectActivationSignal()
{
    if (m_activationSignalConnected) {
        return true;
    }

    m_activationSignalConnected =
        QDBusConnection::sessionBus().connect(QStringLiteral("org.freedesktop.portal.Desktop"),
                                              QStringLiteral("/org/freedesktop/portal/desktop"),
                                              QStringLiteral("org.freedesktop.portal.GlobalShortcuts"),
                                              QStringLiteral("Activated"),
                                              this,
                                              SLOT(handleActivated(QDBusObjectPath,QString,qulonglong,QVariantMap)));
    if (!m_activationSignalConnected) {
        m_errorString = MS_TR("Failed to connect to global shortcut activation signal.");
        return false;
    }
    return true;
}

}  // namespace markshot

#include "global_shortcut_portal.moc"
