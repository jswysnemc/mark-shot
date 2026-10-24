#include "region_detection/region_detection_config.h"

#include "config_value.h"

namespace markshot::region_detection {

bool regionDetectionEnabledFromConfigRoot(const QJsonObject &root)
{
    const QJsonObject capture = config::objectValue(root, QStringLiteral("capture"));
    const QJsonObject detection = config::objectValue(capture, QStringLiteral("regionDetection"));
    return config::boolValue(detection.value(QStringLiteral("enabled"))).value_or(true);
}

}  // namespace markshot::region_detection
