#include "export_image_effect.h"

#include "config_value.h"

#include <QGraphicsEffect>
#include <QGraphicsPixmapItem>
#include <QGraphicsScene>
#include <QPainter>
#include <QPainterPath>
#include <QPixmap>

#include <algorithm>
#include <cmath>
#include <optional>

namespace markshot {
namespace {

constexpr int kMaxPadding = 256;
constexpr qreal kMaxCornerRadius = 128.0;
constexpr int kMaxShadowRadius = 128;
constexpr int kMaxShadowOffsetY = 128;
constexpr qreal kMinShadowOpacity = 0.0;
constexpr qreal kMaxShadowOpacity = 1.0;

std::optional<qreal> realValue(const QJsonValue &value)
{
    if (value.isDouble()) {
        return value.toDouble();
    }
    if (value.isString()) {
        bool ok = false;
        const qreal number = value.toString().trimmed().toDouble(&ok);
        if (ok) {
            return number;
        }
    }
    return std::nullopt;
}

qreal clampedRealValue(const QJsonObject &object,
                       const QStringList &keys,
                       qreal current,
                       qreal minimum,
                       qreal maximum)
{
    if (const std::optional<qreal> number = realValue(config::valueForKeys(object, keys))) {
        return std::clamp(*number, minimum, maximum);
    }
    return current;
}

int clampedIntValue(const QJsonObject &object,
                    const QStringList &keys,
                    int current,
                    int minimum,
                    int maximum)
{
    if (const std::optional<int> number =
            config::clampedIntValue(config::valueForKeys(object, keys), minimum, maximum)) {
        return *number;
    }
    return current;
}

QImage roundedImage(const QImage &source, qreal radius)
{
    QImage output(source.size(), QImage::Format_ARGB32_Premultiplied);
    output.fill(Qt::transparent);

    QPainter painter(&output);
    painter.setRenderHint(QPainter::Antialiasing, true);
    if (radius > 0.0) {
        const qreal fringeInset =
            source.width() > 4 && source.height() > 4 ? 2.0 : 0.0;
        const QRectF clipRect(QPointF(fringeInset, fringeInset),
                              QSizeF(source.width() - fringeInset * 2.0,
                                     source.height() - fringeInset * 2.0));
        QPainterPath clipPath;
        clipPath.addRoundedRect(clipRect,
                                std::max<qreal>(0.0, radius - fringeInset),
                                std::max<qreal>(0.0, radius - fringeInset));
        painter.setClipPath(clipPath);
    }
    painter.drawImage(QPoint(0, 0), source);
    return output;
}

void drawSoftShadow(QPainter *painter,
                    const QImage &roundedContent,
                    QRectF contentRect,
                    QSize outputSize,
                    const ExportImageEffectConfig &config)
{
    if (!painter || roundedContent.isNull() || config.shadowOpacity <= 0.0) {
        return;
    }

    const int radius = std::max(0, config.shadowRadius);
    if (radius <= 0) {
        return;
    }

    QImage shadowLayer(outputSize, QImage::Format_ARGB32_Premultiplied);
    shadowLayer.fill(Qt::transparent);

    QPainter shadowPainter(&shadowLayer);
    shadowPainter.setRenderHint(QPainter::Antialiasing, true);
    shadowPainter.setRenderHint(QPainter::SmoothPixmapTransform, true);

    QGraphicsScene scene;
    scene.setSceneRect(QRectF(QPointF(0.0, 0.0), QSizeF(outputSize)));

    auto *pixmapItem = new QGraphicsPixmapItem;
    pixmapItem->setPixmap(QPixmap::fromImage(roundedContent));
    pixmapItem->setPos(contentRect.topLeft());

    auto *shadowEffect = new QGraphicsDropShadowEffect;
    QColor shadowColor = config.shadowColor;
    shadowColor.setAlphaF(std::sqrt(std::clamp(config.shadowOpacity, 0.0, 1.0)));
    shadowEffect->setColor(shadowColor);
    shadowEffect->setBlurRadius(radius);
    shadowEffect->setOffset(0.0, config.shadowOffsetY);
    pixmapItem->setGraphicsEffect(shadowEffect);

    scene.addItem(pixmapItem);
    scene.render(&shadowPainter,
                 QRectF(QPointF(0.0, 0.0), QSizeF(outputSize)),
                 scene.sceneRect());
    shadowPainter.end();

    painter->save();
    painter->setCompositionMode(QPainter::CompositionMode_SourceOver);
    painter->drawImage(QPoint(0, 0), shadowLayer);
    painter->restore();
}

}  // namespace

ExportImageEffectConfig exportImageEffectConfigFromRoot(const QJsonObject &root)
{
    ExportImageEffectConfig config;
    const QJsonObject exportObject = config::objectValue(root, QStringLiteral("export"));
    const QJsonValue imageFrameValue = exportObject.value(QStringLiteral("imageFrame"));
    if (const std::optional<bool> enabled = config::boolValue(imageFrameValue)) {
        config.enabled = *enabled;
        return config;
    }

    const QJsonObject imageFrame = imageFrameValue.isObject() ? imageFrameValue.toObject() : QJsonObject();
    if (imageFrame.isEmpty()) {
        return config;
    }

    if (const std::optional<bool> enabled =
            config::boolValue(config::valueForKeys(imageFrame,
                                                   {QStringLiteral("enabled"),
                                                    QStringLiteral("show"),
                                                    QStringLiteral("visible")}))) {
        config.enabled = *enabled;
    }
    config.padding =
        clampedIntValue(imageFrame,
                        {QStringLiteral("padding"), QStringLiteral("margin")},
                        config.padding,
                        0,
                        kMaxPadding);
    config.cornerRadius =
        clampedRealValue(imageFrame,
                         {QStringLiteral("cornerRadius"), QStringLiteral("radius")},
                         config.cornerRadius,
                         0.0,
                         kMaxCornerRadius);
    config.shadowRadius =
        clampedIntValue(imageFrame,
                        {QStringLiteral("shadowRadius"), QStringLiteral("shadowBlur")},
                        config.shadowRadius,
                        0,
                        kMaxShadowRadius);
    config.shadowOffsetY =
        clampedIntValue(imageFrame,
                        {QStringLiteral("shadowOffsetY"), QStringLiteral("shadowYOffset")},
                        config.shadowOffsetY,
                        0,
                        kMaxShadowOffsetY);
    config.shadowOpacity =
        clampedRealValue(imageFrame,
                         {QStringLiteral("shadowOpacity"), QStringLiteral("shadowAlpha")},
                         config.shadowOpacity,
                         kMinShadowOpacity,
                         kMaxShadowOpacity);
    return config;
}

QImage applyExportImageEffect(const QImage &source, const ExportImageEffectConfig &config)
{
    if (source.isNull()) {
        return {};
    }
    if (!config.enabled) {
        return source.copy();
    }

    const QImage content = source.format() == QImage::Format_ARGB32_Premultiplied
        ? source
        : source.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const int padding = std::max(0, config.padding);
    const QSize outputSize(content.width() + padding * 2,
                           content.height() + padding * 2);
    QImage output(outputSize, QImage::Format_ARGB32_Premultiplied);
    output.setDevicePixelRatio(source.devicePixelRatio());
    output.fill(Qt::transparent);

    QPainter painter(&output);
    painter.setRenderHint(QPainter::Antialiasing, true);

    const QRectF contentRect(QPointF(padding, padding), QSizeF(content.size()));
    const QImage roundedContent = roundedImage(content, config.cornerRadius);
    drawSoftShadow(&painter, roundedContent, contentRect, outputSize, config);
    painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
    painter.drawImage(QPoint(padding, padding), roundedContent);
    painter.end();

    return output;
}

}  // namespace markshot
