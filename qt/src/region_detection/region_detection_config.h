#pragma once

#include <QJsonObject>

namespace markshot::region_detection {

/**
 * 【截图】【区域识别】读取图像区域分析开关，不影响独立窗口识别。
 * @param root 应用配置根对象。
 * @return 缺省或配置无效时返回 true，明确关闭时返回 false。
 */
bool regionDetectionEnabledFromConfigRoot(const QJsonObject &root);

}  // namespace markshot::region_detection
