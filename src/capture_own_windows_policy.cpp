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
    // KWin 支持 hide-caller-windows=false/true 表达自身窗口保留/隐藏策略；
    // 无论是单帧捕获还是滚动截图的高频采集，均可优先走 KWin CaptureArea 接口，
    // 避免在多席位或双 GPU 拓扑下被错误分流至 PipeWire Screencast。
    Q_UNUSED(hideOwnWindows);
    Q_UNUSED(preferScreencast);
    return true;
}

}  // namespace markshot
