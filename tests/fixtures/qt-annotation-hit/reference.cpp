// 【标注命中对照】【完整入口】首次选择、已选操作和尺子正文执行未经改写的生产函数
#include "probe.h"
#include "production.inc"
#include <QFile>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>

using namespace markshot::shot;

/** 【标注命中对照】【位置编码】按value返回双精度原图坐标 */
QJsonArray point(QPointF value) { return {value.x(), value.y()}; }
/** 【标注命中对照】【矩形编码】按value返回左、上、宽、高，不舍入 */
QJsonArray rectangle(QRectF value) { return {value.x(), value.y(), value.width(), value.height()}; }
/** 【标注命中对照】【操作名称】按value返回生产枚举的稳定名称，未知操作终止采集 */
QString handle(SelectionDrag value) {
#define HANDLE(name) case SelectionDrag::name: return QStringLiteral(#name)
    switch (value) {
        HANDLE(None); HANDLE(Move); HANDLE(Rotate); HANDLE(LineControl); HANDLE(LineStart); HANDLE(LineEnd);
        HANDLE(MagnifierSource); HANDLE(MagnifierLens); HANDLE(NumberTip); HANDLE(NumberBubble);
        HANDLE(Left); HANDLE(Right); HANDLE(Top); HANDLE(Bottom); HANDLE(TopLeft); HANDLE(TopRight); HANDLE(BottomLeft); HANDLE(BottomRight);
        HANDLE(MagnifierSourceLeft); HANDLE(MagnifierSourceRight); HANDLE(MagnifierSourceTop); HANDLE(MagnifierSourceBottom);
        HANDLE(MagnifierSourceTopLeft); HANDLE(MagnifierSourceTopRight); HANDLE(MagnifierSourceBottomLeft); HANDLE(MagnifierSourceBottomRight);
        HANDLE(AngleVertex); HANDLE(AngleFirstArm); HANDLE(AngleSecondArm);
    }
    qFatal("unknown drag");
#undef HANDLE
}

/** 【标注命中对照】【样本模型】按index与rotation返回全部非文字持久工具，几何统一采用原图像素 */
Annotation annotation(int index, qreal rotation) {
    const Tool tools[] = {Tool::Rectangle, Tool::Ellipse, Tool::Marker, Tool::Mosaic, Tool::Pen, Tool::Highlighter,
                          Tool::Line, Tool::Arrow, Tool::Number, Tool::Magnifier, Tool::Magnifier, Tool::Ruler, Tool::Ruler};
    Annotation a; a.id = 7; a.tool = tools[index]; a.width = 6; a.rotationDegrees = rotation;
    a.rect = QRectF(220, 160, 200, 160); a.points = {QPointF(220, 160), QPointF(420, 320)};
    if (index == 4 || index == 5) a.points.append(QPointF(280, 300));
    if (index == 5) a.highlighterStyle = HighlighterStyle::Freehand;
    if (index == 6 || index == 7) a.points = {QPointF(220, 240), QPointF(420, 240)};
    if (index == 8) a.points = {QPointF(220, 240), QPointF(420, 240)};
    if (index == 9 || index == 10) { a.rect = QRectF(360, 180, 120, 120); a.points = {QPointF(220, 240)}; a.magnifierShape = index == 9 ? MagnifierShape::Rectangle : MagnifierShape::Circle; }
    if (index == 12) { a.rulerMode = RulerMode::Angle; a.points = {QPointF(220, 240), QPointF(420, 240), QPointF(300, 360)}; a.rotationDegrees = 0; }
    return a;
}

/** 【标注命中对照】【采集入口】argc和argv提供结果文件，返回零表示完整生产样本已写入 */
int main(int argc, char **argv) {
    QApplication app(argc, argv);
    if (argc != 2) return 2;
    QJsonArray scenes, samples;
    for (int index = 0; index < 13; ++index) for (qreal rotation : {0.0, 31.0}) for (qreal scale : {0.125, 0.5, 2.0}) {
        ShotWindow window;
        window.m_frozenImageRect = QRectF(320 - 320 * scale, 240 - 240 * scale, 640 * scale, 480 * scale);
        const Annotation a = annotation(index, rotation);
        window.m_annotations = {a};
        QJsonArray points; for (auto p : a.points) points.append(point(p));
        const int scene = scenes.size();
        scenes.append(QJsonObject{{"index", index}, {"scale", scale}, {"rect", rectangle(a.rect)}, {"points", points}, {"width", a.width}, {"rotation", a.rotationDegrees}, {"bounds", rectangle(window.annotationBounds(a))}, {"pivot", point(window.annotationRotationCenter(a, false))}, {"rotation_handle", point(window.annotationRotationHandlePoint(a, false))}});
        QVector<QPointF> probes = {QPointF(320, 240), QPointF(320, 300), QPointF(220, 240), QPointF(420, 240)};
        const QRectF bounds = window.annotationBounds(a);
        for (QPointF p : {bounds.topLeft(), bounds.topRight(), bounds.bottomLeft(), bounds.bottomRight(), bounds.center(), QPointF(bounds.center().x(), bounds.top()), QPointF(bounds.right(), bounds.center().y())}) {
            for (qreal offset : {0.0, 7.0, 9.0, 13.0}) for (QPointF direction : {QPointF(1, 0), QPointF(-1, 0), QPointF(0, 1), QPointF(0, -1)}) probes.append(p + direction * offset / scale);
        }
        for (auto p : a.points) for (qreal offset : {7.0, 9.0, 11.0, 13.0}) probes.append(window.rotatedPoint(p, window.annotationRotationCenter(a, false), a.rotationDegrees) + QPointF(0, offset / scale));
        for (qreal outside : {7.0, 9.0}) probes.append(window.rotatedPoint(QPointF(320, 240 + a.width / 2.0 + outside / scale), window.annotationRotationCenter(a, false), a.rotationDegrees));
        for (auto p : probes) {
            const bool first = window.annotationAt(p).has_value();
            const auto selected = window.annotationDragAt(p, a.id);
            window.m_tool = Tool::Ruler;
            const auto rulerSelected = a.tool == Tool::Ruler ? ShotWindowRulerTool::dragAt(window, a, p) : SelectionDrag::None;
            const auto rulerFirst = a.tool == Tool::Ruler ? markshot::ruler::rulerDragAt(a, p, ShotWindowRulerTool::imageTolerance(window), false) : SelectionDrag::None;
            window.m_tool = Tool::Select;
            samples.append(QJsonObject{{"scene", scene}, {"point", point(p)}, {"first", first}, {"selected", handle(selected)}, {"ruler_first", handle(rulerFirst)}, {"ruler_selected", handle(rulerSelected)}});
        }
    }
    QFile output(QString::fromLocal8Bit(argv[1]));
    if (!output.open(QIODevice::WriteOnly)) return 3;
    output.write(QJsonDocument(QJsonObject{{"qt", QT_VERSION_STR}, {"scenes", scenes}, {"samples", samples}}).toJson());
    return 0;
}
