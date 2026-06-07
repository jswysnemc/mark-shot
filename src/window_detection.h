#pragma once

#include <QRect>
#include <QString>
#include <QVector>

namespace markshot {

QString markShotConfigDir();
QString appConfigPath();
bool ensureAppConfigFile();
bool windowDetectionEnabled();

QVector<QRect> collectConfiguredWindowGeometries(const QRect &captureGeometry,
                                                 const QString &outputName,
                                                 bool allOutputs);

} // namespace markshot
