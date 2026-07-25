#include "capture_own_windows_policy.h"

#include "config_value.h"

#include <QJsonValue>
#include <QtGlobal>

#include <optional>

namespace {

QJsonValue hideOwnWindowsValue(const QJsonObject &root)
{
    const QJsonObject capture =
        markshot::config::firstNonEmptyObjectValue(root,
                                                   {QStringLiteral("capture"),
                                                    QStringLiteral("screenshot"),
                                                    QStringLiteral("screenCapture")});
    const QJsonValue nestedValue =
        markshot::config::valueForKeys(capture,
                                       {QStringLiteral("hideOwnWindows"),
                                        QStringLiteral("hideOwnWindowsDuringCapture")});
    if (!nestedValue.isUndefined() && !nestedValue.isNull()) {
        return nestedValue;
    }
    return QJsonValue();
}

}  // namespace

namespace markshot {

bool defaultHideOwnWindowsDuringCapture()
{
    return true;
}

bool hideOwnWindowsDuringCaptureFromConfigRoot(const QJsonObject &root)
{
    const std::optional<bool> value = config::boolValue(hideOwnWindowsValue(root));
    return value.value_or(defaultHideOwnWindowsDuringCapture());
}

bool kwinScreenShotSupportsOwnWindowPolicy(bool hideOwnWindows, bool preferScreencast)
{
    // KWin still accepts still captures when own windows should stay visible:
    // captureWithKWinScreenShot passes hide-caller-windows=false in that case.
    // Screencast sessions keep using the portal path instead.
    Q_UNUSED(hideOwnWindows);
    return !preferScreencast;
}

}  // namespace markshot
