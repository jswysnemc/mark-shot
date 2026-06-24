#pragma once

#include <QRect>
#include <QString>
#include <QVector>
#include <optional>

namespace markshot {

struct WindowInfo {
    QRect rect;
    std::optional<int> zOrder;
};

QString markShotConfigDir();
QString appConfigPath();
bool ensureAppConfigFile();
bool windowDetectionEnabled();

QVector<WindowInfo> collectConfiguredWindowInfos(const QRect &captureGeometry,
                                                 const QString &outputName,
                                                 bool allOutputs);

} // namespace markshot
