#include "platform/wayland/compositor_pointer.h"

#include "debug_log.h"
#include "window_detection_session.h"

#include <QProcessEnvironment>

#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusConnection>
#include <QDBusMessage>
#include <QVariant>
#endif

namespace markshot::wayland {

std::optional<QPoint> compositorPointerPosition()
{
#ifdef MARK_SHOT_WITH_DBUS
    // 1. 【Wayland】【指针查询】只在 GNOME Wayland 使用扩展，其他桌面不发送无关 D-Bus 请求
    const auto session = window_detection::detectSession(QProcessEnvironment::systemEnvironment());
    if (!session.wayland || session.compositor != QStringLiteral("gnome")) {
        return std::nullopt;
    }

    // 2. 【GNOME】【指针查询】限制同步等待时间，旧扩展或不可用接口不阻止截图启动
    const auto request = QDBusMessage::createMethodCall(
        QStringLiteral("org.gnome.Shell"),
        QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"),
        QStringLiteral("org.gnome.Shell.Extensions.MarkShotScrollHelper"),
        QStringLiteral("PointerPosition"));
    const auto reply = QDBusConnection::sessionBus().call(request, QDBus::Block, 250);
    const QVariantList coordinates = reply.arguments();
    if (reply.type() != QDBusMessage::ReplyMessage || coordinates.size() != 2
        || coordinates.at(0).metaType().id() != QMetaType::Int
        || coordinates.at(1).metaType().id() != QMetaType::Int) {
        markshot::debugLog("pointer", "【GNOME】【指针查询】pointer-unavailable name=%s message=%s",
                           reply.errorName().toUtf8().constData(), reply.errorMessage().toUtf8().constData());
        return std::nullopt;
    }

    // 3. 【GNOME】【指针查询】保留负坐标，交由 Qt 当前屏幕几何判断输出
    const QPoint pointer(coordinates.at(0).toInt(), coordinates.at(1).toInt());
    markshot::debugLog("pointer", "【GNOME】【指针查询】pointer x=%d y=%d", pointer.x(), pointer.y());
    return pointer;
#else
    return std::nullopt;
#endif
}

}  // namespace markshot::wayland
