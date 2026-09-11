#include "ocr_result_window_config.h"

namespace markshot::shot {

bool ocrResultWindowAlwaysOnTopFromRoot(const QJsonObject &root)
{
    return root.value(QStringLiteral("ocrResultWindow")).toObject()
        .value(QStringLiteral("alwaysOnTop")).toBool(false);
}

}
