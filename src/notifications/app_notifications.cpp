#include "notifications/app_notifications.h"

#include "recording/recording_options.h"
#include "ui/i18n.h"

#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#endif

#include <QStringList>
#include <QVariantMap>

namespace markshot::notifications {

bool sendDesktopNotification(const QString &summary, const QString &body, int timeoutMs)
{
#ifdef MARK_SHOT_WITH_DBUS
    QDBusInterface notifications(QStringLiteral("org.freedesktop.Notifications"),
                                 QStringLiteral("/org/freedesktop/Notifications"),
                                 QStringLiteral("org.freedesktop.Notifications"),
                                 QDBusConnection::sessionBus());
    if (!notifications.isValid()) {
        return false;
    }

    QDBusMessage reply = notifications.call(QStringLiteral("Notify"),
                                            QStringLiteral("mark-shot"),
                                            static_cast<uint>(0),
                                            QString(),
                                            summary,
                                            body,
                                            QStringList(),
                                            QVariantMap(),
                                            timeoutMs);
    return reply.type() != QDBusMessage::ErrorMessage;
#else
    Q_UNUSED(summary);
    Q_UNUSED(body);
    Q_UNUSED(timeoutMs);
    return false;
#endif
}

bool notifyScreenshotSaved(const QString &path)
{
    return sendDesktopNotification(MS_TR("Screenshot saved"),
                                   MS_TR("Saved to %1").arg(path),
                                   3000);
}

bool notifyRecordingStarted(const recording::RecordingOptions &options)
{
    return sendDesktopNotification(MS_TR("Recording started"),
                                   MS_TR("Output: %1").arg(options.outputPath),
                                   3000);
}

bool notifyRecordingSaved(const QString &path)
{
    return sendDesktopNotification(MS_TR("Recording saved"),
                                   MS_TR("Saved to %1").arg(path),
                                   3500);
}

bool notifyRecordingFailed(const QString &message)
{
    const QString body = message.trimmed().isEmpty() ? MS_TR("Recording failed") : message;
    return sendDesktopNotification(MS_TR("Recording failed"), body, 4000);
}

}  // namespace markshot::notifications
