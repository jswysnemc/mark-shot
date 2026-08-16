#include "capture_double_click_action.h"

#include "app_config_store.h"

namespace markshot {

CaptureDoubleClickAction configuredCaptureDoubleClickAction()
{
    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    if (!ok) {
        return defaultCaptureDoubleClickAction();
    }
    return captureDoubleClickActionFromConfigRoot(root);
}

}  // namespace markshot
