// 【尺子说明对照】【完整入口】执行生产drawLabels，几何报告另用生产布局函数生成并核对完整绘制像素
#include "probe.h"
#include "production.inc"
#include "ruler/ruler_painter.cpp"
#include "ruler/angle_painter.cpp"
#include <QDir>
#include <QFile>
#include <QFontMetricsF>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPainter>
#include <QCryptographicHash>

/** 【尺子说明对照】【矩形记录】按rect返回左、上、宽、高，保留退化尺寸 */
QJsonArray rectangle(QRectF rect) { return {rect.x(), rect.y(), rect.width(), rect.height()}; }
/** 【尺子说明对照】【标注记录】按annotation返回实际模式、定点、范围与笔宽 */
QJsonObject annotationValue(const ShotWindow::Annotation &annotation)
{
    QJsonArray points;
    for (const auto &point : annotation.points) points.append(QJsonArray{point.x(), point.y()});
    return {{"id", annotation.id}, {"tool", int(annotation.tool)}, {"angle", annotation.rulerMode == markshot::shot::types::RulerMode::Angle},
            {"rect", rectangle(annotation.rect)}, {"points", points}, {"width", annotation.width}, {"rotation", annotation.rotationDegrees}};
}

/** 【尺子说明对照】【场景构造】按pattern和width生成包含持久、草稿及非尺子标注的窗口状态，无返回值 */
void populate(LabelWindow &window, int pattern, qreal width)
{
    using namespace markshot::shot::types;
    auto length = [&](int id, QRectF rect, qreal rotation = 0) {
        Annotation value;
        value.id = id; value.tool = Tool::Ruler; value.rulerMode = RulerMode::Length;
        value.rect = rect; value.width = width; value.rotationDegrees = rotation;
        return value;
    };
    auto angle = [&](int id, QVector<QPointF> points) {
        Annotation value;
        value.id = id; value.tool = Tool::Ruler; value.rulerMode = RulerMode::Angle;
        value.points = points; value.width = width;
        return value;
    };
    switch (pattern) {
    case 0: window.m_annotations = {length(1, {220, 160, 200, 160})}; break;
    case 1: window.m_annotations = {angle(1, {{240, 220}, {360, 220}, {280, 320}})}; break;
    case 2: window.m_annotations = {length(1, {140, 100, 200, 160}), angle(2, {{240, 220}, {360, 220}, {280, 320}}), length(3, {280, 180, 180, 140}, 37)}; break;
    case 3: window.m_annotations = {length(1, {4, 3, 200, 160}), length(2, {180, 100, 200, 160}, -123), angle(3, {{420, 320}, {300, 320}, {420, 180}})}; break;
    case 4: window.m_annotations = {length(1, {220, 190, 20, 6}), angle(2, {{220, 190}, {260, 190}, {220, 230}})}; break;
    case 5: window.m_annotations = {length(1, {120, 100, 220, 0}), length(2, {460, 130, 0, 120})}; break;
    case 6: window.m_annotations = {angle(1, {{240, 220}, {240, 220}, {240, 220}}), angle(2, {{240, 220}, {360, 220}, {120, 220}}), angle(3, {{240, 220}, {360, 220}})}; break;
    case 7: window.m_annotations = {length(1, {-10, -20, 660, 510}), length(2, {400, 330, 17.5, 3.5}, 90)}; break;
    case 8:
        window.m_annotations = {length(1, {220, 160, 200, 160})};
        window.m_draft = length(2, {230, 170, 200, 160}, 37);
        break;
    case 9:
        window.m_annotations = {length(1, {220, 160, 200, 160})};
        window.m_draft = angle(2, {{240, 220}, {360, 220}, {280, 320}});
        break;
    default: qFatal("unknown label pattern");
    }
    Annotation unrelated;
    unrelated.id = 99; unrelated.tool = Tool::Rectangle;
    window.m_annotations.prepend(unrelated);
}

/** 【尺子说明对照】【完整报告】按window和widget收集实际标签，完整生产绘制与报告绘制必须逐像素相同，返回场景记录 */
QJsonObject sample(const LabelWindow &window, bool widget, QDir output, int serial)
{
    using namespace markshot::ruler;
    const ShotWindowRulerPainter painter(window);
    const QRectF bounds = painter.labelBounds(widget);
    QVector<const ShotWindow::Annotation *> sources;
    for (const auto &a : window.m_annotations) if (a.tool == ShotWindow::Tool::Ruler) sources.append(&a);
    if (widget && window.m_draft && window.m_draft->tool == ShotWindow::Tool::Ruler) sources.append(&*window.m_draft);
    QVector<QRectF> occupied;
    QVector<RulerGeometry> lengths;
    QVector<const ShotWindow::Annotation *> lengthAnnotations;
    QJsonArray labels, inputs;
    QImage reported(window.windowRect.size(), QImage::Format_ARGB32_Premultiplied);
    reported.fill(Qt::transparent);
    QPainter reportPainter(&reported);
    reportPainter.setRenderHint(QPainter::Antialiasing);
    auto add = [&](const ShotWindow::Annotation &annotation, QRectF rect, QString text, qreal unit) {
        const auto font = rulerFont(unit);
        labels.append(QJsonObject{{"id", annotation.id}, {"rect", rectangle(rect)}, {"text", text}, {"unit", unit}, {"font_pixels", font.pixelSize()},
                                  {"text_width", QFontMetricsF(font).horizontalAdvance(text)}, {"text_height", QFontMetricsF(font).height()}});
        drawRulerLabel(reportPainter, rect, text, unit);
    };
    // 1. 【尺子说明对照】【生产几何报告】报告只调用生产角度和长度布局函数，不使用Rust公式构造预期
    for (const auto *a : sources) {
        inputs.append(annotationValue(*a));
        if (a->rulerMode == markshot::shot::types::RulerMode::Angle) {
            if (a->points.size() < 3) continue;
            const auto geometry = painter.angleGeometry(*a, widget);
            const QRectF rect = angleInfoLabelRect(geometry, bounds);
            occupied.append(rect);
            add(*a, rect, angleInfoText(angleDegrees(a->points[0], a->points[1], a->points[2])), geometry.unit);
        } else if (rulerMeasures(*a)) {
            lengths.append(painter.lengthGeometry(*a, widget));
            lengthAnnotations.append(a);
        }
    }
    const auto positions = layoutRulerInfoLabels(lengths, bounds, occupied);
    QJsonArray diagnostics;
    for (int i = 0; i < lengths.size(); ++i) {
        const auto &geometry = lengths[i];
        const QRectF body = rotatedAroundRuler(geometry, geometry.paintRect);
        const QRectF scale = rulerScaleBounds(geometry);
        const qreal gap = 8 * geometry.unit;
        const auto size = rulerLabelSize(QFontMetricsF(rulerFont(geometry.unit)), rulerInfoText(geometry.imageRect), geometry.unit);
        const QVector<QRectF> candidates = {QRectF(QPointF(body.left(), scale.bottom() + gap), size), QRectF(QPointF(body.left(), scale.top() - size.height() - gap), size),
            QRectF(QPointF(scale.right() + gap, body.top()), size), QRectF(QPointF(scale.left() - size.width() - gap, body.top()), size)};
        QJsonArray values;
        for (const auto &candidate : candidates) {
            const auto clamped = clampRulerFloatingRect(candidate, bounds, gap);
            values.append(QJsonObject{{"rect", rectangle(clamped)}, {"self_overlap", clamped.intersects(scale.adjusted(-gap, -gap, gap, gap))}});
        }
        diagnostics.append(QJsonObject{{"id", lengthAnnotations[i]->id}, {"body", rectangle(body)}, {"scale", rectangle(scale)}, {"candidates", values}});
    }
    for (int i = 0; i < lengths.size(); ++i) add(*lengthAnnotations[i], positions[i], rulerInfoText(lengths[i].imageRect), lengths[i].unit);
    reportPainter.end();
    // 2. 【尺子说明对照】【完整入口校验】报告的顺序和位置必须复现完整生产drawLabels的每个像素
    QImage complete(window.windowRect.size(), QImage::Format_ARGB32_Premultiplied);
    complete.fill(Qt::transparent);
    QPainter completePainter(&complete);
    completePainter.setRenderHint(QPainter::Antialiasing);
    painter.drawLabels(completePainter, widget);
    completePainter.end();
    if (complete != reported) qFatal("reported label geometry does not match complete production drawLabels");
    QString image;
    if (serial % 97 == 0) {
        image = QString("frame-%1.png").arg(serial);
        complete.save(output.filePath(image));
    }
    const QByteArray bytes(reinterpret_cast<const char *>(complete.constBits()), complete.sizeInBytes());
    return {{"widget", widget}, {"allowed", rectangle(bounds)}, {"inputs", inputs}, {"labels", labels}, {"length_diagnostics", diagnostics}, {"frame", image},
            {"frame_sha256", QString::fromLatin1(QCryptographicHash::hash(bytes, QCryptographicHash::Sha256).toHex())}};
}

/** 【尺子说明对照】【入口】按新输出目录采集窗口与导出矩阵，返回退出状态，无可见窗口 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    QDir output(argv[1]);
    QJsonArray scenes;
    int serial = 0;
    for (qreal scale : {0.125, 0.5, 1.0, 2.75}) for (qreal capture : {0.5, 1.0, 1.5, 4.0})
    for (qreal width : {1.0, 6.0, 12.0}) for (int context = 0; context < 3; ++context) for (int pattern = 0; pattern < 10; ++pattern) {
        LabelWindow window;
        window.m_frozenFrame = QImage(640, 480, QImage::Format_ARGB32_Premultiplied);
        window.m_sourceGeometry = QRect(0, 0, qRound(640 / capture), qRound(480 / capture));
        window.windowRect = QRect(0, 0, 640, 480);
        const QPointF offset = context == 1 ? QPointF(-100, -80) : QPointF(0, 0);
        window.m_frozenImageRect = QRectF(offset, QSizeF(640 * scale, 480 * scale));
        window.m_selection = context == 2 ? QRectF(210, 180, 80, 30) : QRectF(0, 0, 640, 480);
        populate(window, pattern, width);
        for (bool widget : {true, false}) {
            auto value = sample(window, widget, output, serial);
            value.insert("serial", serial++); value.insert("scale", scale); value.insert("capture", capture); value.insert("width", width);
            value.insert("context", context); value.insert("pattern", pattern); value.insert("offset", QJsonArray{offset.x(), offset.y()});
            value.insert("selection", rectangle(window.normalizedSelection()));
            scenes.append(value);
        }
    }
    QFile result(output.filePath("reference.json"));
    if (!result.open(QIODevice::WriteOnly)) return 2;
    result.write(QJsonDocument(QJsonObject{{"qt", qVersion()}, {"scenes", scenes}}).toJson(QJsonDocument::Compact));
    return 0;
}
