#include "capture_history/history_store.h"

#include "app_config_store.h"
#include "debug_log.h"

namespace markshot::history {

bool rememberScreenshot(const QImage &image)
{
    const HistoryConfig config = historyConfigFromRoot(markshot::readAppConfigRoot());
    if (!config.enabled) {
        return true;
    }
    QString error;
    const bool saved = HistoryStore().append(image, config.maximumEntries, &error);
    if (!saved) {
        markshot::debugLog("history", "【截图历史】【保存】%s", error.toUtf8().constData());
    }
    return saved;
}

}
