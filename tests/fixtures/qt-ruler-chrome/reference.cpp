// 【尺子窗口对照】【完整布局】包含完整生产绘制文件，以采集内部条带布局与真实字体度量
#include "probe.h"
#include "production.inc"
#include "ruler/ruler_painter.cpp"
#include <QFile>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

/** 【尺子窗口对照】【矩形记录】按value返回左、上、宽、高数组，保留平直矩形 */
QJsonArray rectangle(QRectF value) { return {value.x(), value.y(), value.width(), value.height()}; }
/** 【尺子窗口对照】【定点记录】按value返回横纵坐标数组 */
QJsonArray point(QPointF value) { return {value.x(), value.y()}; }

/** 【尺子窗口对照】【入口】按输出路径采集完整生产布局，返回退出状态；覆盖外标注一律拒绝 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    using namespace markshot::ruler;
    QJsonArray scenes;
    const QVector<QRectF> bodies = {
        {220, 160, 200, 160}, {4, 3, 200, 160}, {420, 300, 216, 175},
        {-30, -20, 680, 500}, {210, 160, 17.5, 3.5}, {100, 100, 200, 0},
        {100, 100, 0, 200}, {5, 5, 0.5, 0.75},
    };
    // 1. 【尺子窗口对照】【变量矩阵】分别改变投影、截图补偿、笔宽、旋转、位置和窗口允许范围
    for (qreal scale : {0.125, 0.5, 1.0, 2.75}) {
        for (qreal capture : {0.5, 1.0, 1.5, 4.0}) {
            for (qreal width : {1.0, 3.0, 6.0, 12.0, 30.0}) {
                for (qreal rotation : {0.0, 37.0, 90.0, -123.0}) {
                    for (int context = 0; context < 4; ++context) {
                        ShotWindow window;
                        window.m_frozenFrame = QImage(640, 480, QImage::Format_ARGB32_Premultiplied);
                        window.m_sourceGeometry = QRect(0, 0, qRound(640 / capture), qRound(480 / capture));
                        window.windowRect = context == 2 ? QRect(0, 0, 320, 240) : QRect(0, 0, 640, 480);
                        const QPointF offset = context == 1 ? QPointF(-100, -80) : context == 3 ? QPointF(1000, 1000) : QPointF(0, 0);
                        window.m_frozenImageRect = QRectF(offset, QSizeF(640 * scale, 480 * scale));
                        window.m_selection = context == 2 ? QRectF(20, 10, 600, 460) : QRectF(0, 0, 640, 480);
                        ShotWindowRulerPainter painter(window);
                        for (const QRectF &body : bodies) {
                            ShotWindow::Annotation annotation;
                            annotation.tool = ShotWindow::Tool::Ruler;
                            annotation.rulerMode = markshot::shot::types::RulerMode::Length;
                            annotation.rect = body;
                            annotation.width = width;
                            annotation.rotationDegrees = rotation;
                            const auto geometry = painter.lengthGeometry(annotation, true);
                            const auto layout = rulerTickLayout(geometry);
                            const auto font = rulerFont(geometry.unit);
                            const QFontMetricsF metrics(font);
                            const QPointF center = window.imageToWidget(markshot::ruler::rulerBounds(annotation).center());
                            const QPointF top = window.imageToWidget(window.rotatedPoint(markshot::ruler::rulerBounds(annotation).topLeft() + QPointF(markshot::ruler::rulerBounds(annotation).width() / 2, 0), markshot::ruler::rulerBounds(annotation).center(), rotation));
                            const auto handle = window.annotationRotationHandlePoint(annotation, true);
                            const QLineF stem(top, handle);
                            const auto start = stem.pointAt(painter.handleClearance(annotation) / stem.length());
                            QJsonArray handles;
                            for (const auto &handle : rulerHandles(annotation, 8 / scale)) {
                                handles.append(QJsonObject{{"point", point(window.imageToWidget(handle.point))}, {"drag", int(handle.drag)}});
                            }
                            scenes.append(QJsonObject{
                                {"rect", rectangle(body)}, {"scale", scale}, {"capture", capture}, {"width", width}, {"rotation", rotation}, {"context", context},
                                {"selection", rectangle(window.normalizedSelection())}, {"window", rectangle(window.rect())}, {"offset", point(offset)},
                                {"paint_rect", rectangle(geometry.paintRect)}, {"unit", geometry.unit}, {"allowed", rectangle(geometry.bounds)},
                                {"font_family", QFontInfo(font).family()}, {"font_pixels", font.pixelSize()}, {"font_height", metrics.height()},
                                {"width_advance", metrics.horizontalAdvance(QString::number(qRound(geometry.imageRect.width())))},
                                {"height_advance", metrics.horizontalAdvance(QString::number(qRound(geometry.imageRect.height())))},
                                {"horizontal_band", rectangle(layout.horizontalBand)}, {"vertical_band", rectangle(layout.verticalBand)},
                                {"below", layout.numbersBelow}, {"right", layout.numbersRight}, {"clamp", numbersClampToBounds(geometry)},
                                {"scale_bounds", rectangle(rulerScaleBounds(geometry))}, {"top_reach", rulerTopReach(geometry)},
                                {"step_x", majorTickStep(geometry.imageRect.width(), geometry.paintRect.width(), metrics.horizontalAdvance(QString::number(qRound(geometry.imageRect.width()))) + 6 * geometry.unit)},
                                {"step_y", majorTickStep(geometry.imageRect.height(), geometry.paintRect.height(), metrics.height() + 3 * geometry.unit)},
                                {"center", point(center)}, {"connector_start", point(start)}, {"rotation_handle", point(handle)}, {"handles", handles},
                            });
                        }
                    }
                }
            }
        }
    }
    QFile output(argv[1]);
    if (!output.open(QIODevice::WriteOnly)) return 2;
    output.write(QJsonDocument(QJsonObject{{"qt", qVersion()}, {"scenes", scenes}}).toJson(QJsonDocument::Compact));
    return 0;
}
