#include "capture_own_windows_policy.h"

#include "app_config_store.h"

namespace markshot {

bool configuredHideOwnWindowsDuringCapture()
{
    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    if (!ok) {
        return defaultHideOwnWindowsDuringCapture();
    }
    return hideOwnWindowsDuringCaptureFromConfigRoot(root);
}

}  // namespace markshot
