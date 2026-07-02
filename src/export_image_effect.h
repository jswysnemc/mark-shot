#pragma once

#include <QColor>
#include <QImage>
#include <QJsonObject>

namespace markshot {

struct ExportImageEffectConfig {
    bool enabled = true;
    int padding = 112;
    qreal cornerRadius = 18.0;
    int shadowRadius = 72;
    int shadowOffsetY = 28;
    qreal shadowOpacity = 0.32;
    QColor shadowColor = QColor(0, 0, 0);
};

ExportImageEffectConfig exportImageEffectConfigFromRoot(const QJsonObject &root);
QImage applyExportImageEffect(const QImage &source, const ExportImageEffectConfig &config);

}  // namespace markshot
