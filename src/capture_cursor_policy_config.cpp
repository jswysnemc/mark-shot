#include "capture_cursor_policy.h"

#include "app_config_store.h"

namespace markshot {

bool configuredCaptureIncludeCursor()
{
    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    if (!ok) {
        return defaultCaptureIncludeCursor();
    }
    return captureIncludeCursorFromConfigRoot(root);
}

}  // namespace markshot
