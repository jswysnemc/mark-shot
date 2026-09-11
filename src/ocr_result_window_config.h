#pragma once

#include <QJsonObject>

namespace markshot::shot {

/// @brief 读取 OCR 结果窗口的独立置顶配置，默认由窗口管理器管理
/// @param root 应用配置根对象
/// @return 明确开启 OCR 窗口置顶时返回 true
bool ocrResultWindowAlwaysOnTopFromRoot(const QJsonObject &root);

}
