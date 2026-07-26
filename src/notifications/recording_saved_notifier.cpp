#include "notifications/recording_saved_notifier.h"

#include "notifications/app_notifications.h"
#include "ui/i18n.h"

#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusReply>
#endif

#include <QDesktopServices>
#include <QFileInfo>
#include <QStringList>
#include <QUrl>
#include <QVariantMap>

namespace markshot::notifications {
namespace {

// 通知动作标识，回调时用于区分用户点击了哪个按钮
const QString kOpenFolderAction = QStringLiteral("mark-shot-open-folder");

}  // namespace

RecordingSavedNotifier &RecordingSavedNotifier::instance()
{
    static RecordingSavedNotifier notifier;
    return notifier;
}

RecordingSavedNotifier::RecordingSavedNotifier(QObject *parent)
    : QObject(parent)
{
}

bool RecordingSavedNotifier::notifySaved(const QString &path)
{
#ifndef MARK_SHOT_WITH_DBUS
    // 没有通知服务动作支持时退回普通通知
    return sendDesktopNotification(MS_TR("Recording saved"), MS_TR("Saved to %1").arg(path), 4000);
#else
    if (!ensureActionListener()) {
        return sendDesktopNotification(MS_TR("Recording saved"),
                                       MS_TR("Saved to %1").arg(path),
                                       4000);
    }

    QDBusInterface notifications(QStringLiteral("org.freedesktop.Notifications"),
                                 QStringLiteral("/org/freedesktop/Notifications"),
                                 QStringLiteral("org.freedesktop.Notifications"),
                                 QDBusConnection::sessionBus());
    if (!notifications.isValid()) {
        return false;
    }

    // 1. 通知带一个打开目录的动作按钮
    const QStringList actions{kOpenFolderAction, MS_TR("Open Folder")};
    const QDBusMessage reply = notifications.call(QStringLiteral("Notify"),
                                                  QStringLiteral("mark-shot"),
                                                  static_cast<uint>(0),
                                                  QString(),
                                                  MS_TR("Recording saved"),
                                                  MS_TR("Saved to %1").arg(path),
                                                  actions,
                                                  QVariantMap(),
                                                  4000);
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty()) {
        return false;
    }

    // 2. 记录通知编号，等待用户点击后再定位文件
    const uint notificationId = reply.arguments().first().toUInt();
    if (notificationId != 0) {
        m_pendingPaths.insert(notificationId, path);
    }
    return true;
#endif
}

bool RecordingSavedNotifier::ensureActionListener()
{
#ifndef MARK_SHOT_WITH_DBUS
    return false;
#else
    if (m_listenerConnected) {
        return true;
    }
    m_listenerConnected = QDBusConnection::sessionBus()
                              .connect(QStringLiteral("org.freedesktop.Notifications"),
                                       QStringLiteral("/org/freedesktop/Notifications"),
                                       QStringLiteral("org.freedesktop.Notifications"),
                                       QStringLiteral("ActionInvoked"),
                                       this,
                                       SLOT(handleActionInvoked(uint, QString)));
    return m_listenerConnected;
#endif
}

void RecordingSavedNotifier::handleActionInvoked(uint notificationId, const QString &actionKey)
{
    const QString path = m_pendingPaths.take(notificationId);
    if (path.isEmpty() || actionKey != kOpenFolderAction) {
        return;
    }
    revealInFileManager(path);
}

void RecordingSavedNotifier::revealInFileManager(const QString &path)
{
    const QFileInfo info(path);
    if (!info.exists()) {
        return;
    }

#ifdef MARK_SHOT_WITH_DBUS
    // 1. 文件管理器接口可以直接选中文件
    QDBusInterface fileManager(QStringLiteral("org.freedesktop.FileManager1"),
                               QStringLiteral("/org/freedesktop/FileManager1"),
                               QStringLiteral("org.freedesktop.FileManager1"),
                               QDBusConnection::sessionBus());
    if (fileManager.isValid()) {
        const QStringList uris{QUrl::fromLocalFile(info.absoluteFilePath()).toString()};
        const QDBusMessage reply = fileManager.call(QStringLiteral("ShowItems"), uris, QString());
        if (reply.type() != QDBusMessage::ErrorMessage) {
            return;
        }
    }
#endif

    // 2. 退回到打开所在目录
    QDesktopServices::openUrl(QUrl::fromLocalFile(info.absolutePath()));
}

}  // namespace markshot::notifications
