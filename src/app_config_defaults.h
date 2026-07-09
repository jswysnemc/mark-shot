#pragma once

#include <QJsonObject>
#include <QString>

namespace markshot {

/**
 * 创建应用默认配置。
 * @param windowDetectionCommand 当前桌面环境对应的窗口检测命令。
 * @return 可直接写入配置文件的默认 JSON 对象。
 */
QJsonObject defaultAppConfigRoot(const QString &windowDetectionCommand);

}  // namespace markshot
