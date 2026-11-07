#include "probe.h"
#include "production.inc"
#include <QApplication>
#include <QDir>
#include <QFile>
#include <QJsonDocument>

/** 【标注选框对照】【位置输出】按 point 返回两个原始浮点坐标 */
QJsonArray point(QPointF point) { return {point.x(), point.y()}; }
/** 【标注选框对照】【矩形输出】按 rect 返回原始位置和宽高 */
QJsonArray rectangle(QRectF rect) { return {rect.x(), rect.y(), rect.width(), rect.height()}; }

/** 【标注选框对照】【生产绘制】按 argv 新目录输出真实生产函数的矩形选框和删除位置，返回文件写入状态 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    const QString root = QString::fromLocal8Bit(argv[1]);
    QDir().mkpath(root + QStringLiteral("/render"));
    QJsonArray samples;
    int index = 0;
    for (const QRectF bounds : {QRectF(80, 80, 100, 80), QRectF(2, 2, 20, 30), QRectF(490, 300, 108, 58), QRectF(10, 110, 180, 15)}) {
        for (const qreal angle : {0.0, 30.0, 45.0, 90.0, 180.0, -30.0}) {
            for (const qreal dpr : {1.0, 1.5, 2.0}) {
                ShotWindow window;
                window.annotation.rect = bounds;
                window.annotation.rotationDegrees = angle;
                const QRectF button = window.selectedAnnotationDeleteButtonRect();
                const auto corners = QList<QPointF>{bounds.topLeft(), bounds.topRight(), bounds.bottomRight(), bounds.bottomLeft()};
                QJsonArray handles;
                QJsonArray polygon;
                for (const QPointF corner : corners) { polygon.append(point(window.rotatedPoint(corner, bounds.center(), angle))); }
                for (const QPointF handle : window.selectionHandlePoints(bounds)) { handles.append(point(window.rotatedPoint(handle, bounds.center(), angle))); }
                QImage image(qRound(window.width() * dpr), qRound(window.height() * dpr), QImage::Format_ARGB32_Premultiplied);
                image.setDevicePixelRatio(dpr);
                image.fill(Qt::transparent);
                QPainter painter(&image);
                // 1. 【标注选框对照】【调用上下文】生产画布在调用选框函数前启用抗锯齿，探针保持同一绘制状态
                painter.setRenderHint(QPainter::Antialiasing, true);
                window.drawSelectedAnnotationFrame(painter);
                painter.end();
                const QString name = QStringLiteral("frame-%1.png").arg(index++, 3, 10, QLatin1Char('0'));
                if (!image.save(root + QStringLiteral("/render/") + name)) { return 1; }
                samples.append(QJsonObject{{"bounds", rectangle(bounds)}, {"angle", angle}, {"dpr", dpr},
                    {"delete", rectangle(button)}, {"corners", polygon}, {"handles", handles},
                    {"rotation", point(window.annotationRotationHandlePoint(window.annotation, true))}, {"render", name}});
            }
        }
    }
    QFile file(root + QStringLiteral("/reference.json"));
    if (!file.open(QIODevice::WriteOnly)) { return 1; }
    file.write(QJsonDocument(QJsonObject{{"qt", QString::fromLatin1(qVersion())}, {"scope", "single rectangle selection chrome; input bounds supplied"}, {"samples", samples}}).toJson(QJsonDocument::Compact));
    file.write("\n");
    return 0;
}
