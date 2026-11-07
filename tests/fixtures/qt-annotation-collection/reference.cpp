// 【标注多选对照】【生产调用】编号、框选、共同变换和批量宽度全部调用采集的完整生产函数
#include "probe.h"
#include "production.inc"
#include <QFile>
#include <QJsonDocument>

using namespace markshot::shot;

/** 【标注多选对照】【坐标编码】按point返回双精度原始坐标 */
QJsonArray point(QPointF value) { return {value.x(), value.y()}; }
/** 【标注多选对照】【矩形编码】按rect返回原始左、上、宽、高 */
QJsonArray rect(QRectF value) { return {value.x(), value.y(), value.width(), value.height()}; }
/** 【标注多选对照】【成员编码】按window与annotation返回全部几何、宽度和可见范围，不改变生产状态 */
QJsonObject encode(const ShotWindow &window, const Annotation &a) {
    QJsonArray points;
    for (auto p : a.points) points.append(point(p));
    return {{"id", a.id}, {"tool", static_cast<int>(a.tool)}, {"rect", rect(a.rect)}, {"points", points}, {"width", a.width}, {"rotation", a.rotationDegrees}, {"bounds", rect(window.annotationBounds(a))}, {"pivot", point(window.annotationRotationCenter(a, false))}, {"ruler_mode", static_cast<int>(a.rulerMode)}, {"magnifier_shape", static_cast<int>(a.magnifierShape)}, {"text", a.text}, {"color", QJsonArray{a.color.red(), a.color.green(), a.color.blue(), a.color.alpha()}}};
}
/** 【标注多选对照】【成员列表】按window返回持久图层顺序的成员数据，选择顺序另行记录 */
QJsonArray annotations(const ShotWindow &window) { QJsonArray result; for (const auto &a : window.m_annotations) result.append(encode(window, a)); return result; }
/** 【标注多选对照】【场景构造】按scene返回未旋转矩形、带角度矩形或全部工具混合场景 */
ShotWindow scene(int index) {
    ShotWindow window;
    Annotation first; first.id = 7; first.rect = QRectF(180, 130, 100, 80); first.width = 3;
    Annotation second = first; second.id = 17; second.rect = QRectF(340, 250, 80, 60);
    window.m_annotations = {first, second};
    if (index == 1) { window.m_annotations[0].rotationDegrees = 13; window.m_annotations[1].rotationDegrees = -21; }
    if (index >= 2) {
        const Tool tools[] = {Tool::Ellipse, Tool::Marker, Tool::Mosaic, Tool::Magnifier, Tool::Ruler, Tool::Text, Tool::Pen, Tool::Highlighter, Tool::Line, Tool::Arrow, Tool::Number};
        for (int i = 0; i < 11; ++i) {
            Annotation a; a.id = 30 + i; a.tool = tools[i]; a.width = 3; a.rotationDegrees = i % 3 == 0 ? 15 : -11;
            a.rect = QRectF(70 + i * 29, 80 + i * 18, 80, 50);
            if (index >= 4) a.rect = QRectF(i % 2 == 0 ? 2 : 540, i % 3 == 0 ? 2 : 370, 80, 50);
            if (a.tool == Tool::Magnifier && (index == 3 || index == 4)) { a.magnifierShape = MagnifierShape::Circle; a.rect.setHeight(80); }
            a.points = {a.rect.topLeft(), a.rect.bottomRight()};
            if (a.tool == Tool::Text) { a.width = 5; a.points.resize(1); fitTextAnnotationRect(a); }
            if (a.tool == Tool::Text && index >= 3) { a.text = QStringLiteral("Wrapped text at the edge\nSecond paragraph with words"); fitTextAnnotationRect(a, 110); }
            if (a.tool == Tool::Mosaic) a.width = 14;
            if (a.tool == Tool::Magnifier) a.points.resize(1);
            window.m_annotations.append(a);
        }
        Annotation angle; angle.id = 80; angle.tool = Tool::Ruler; angle.rulerMode = RulerMode::Angle; angle.points = {QPointF(270, 210), QPointF(360, 220), QPointF(290, 300)};
        window.m_annotations.append(angle);
    }
    QVector<int> ids;
    for (int i = window.m_annotations.size() - 1; i >= 0; --i) ids.append(window.m_annotations.at(i).id);
    window.setSelectedAnnotations(ids);
    return window;
}
/** 【标注多选对照】【采集入口】argc与argv提供输出文件，返回零表示所有真实函数结果已写入 */
int main(int argc, char **argv) {
    QApplication app(argc, argv);
    if (argc != 2) return 2;
    QJsonArray selections, transforms, widths, sequences, colors, lineEdits;
    for (int index = 0; index < 6; ++index) {
        auto window = scene(index);
        const auto before = annotations(window);
        for (auto marquee : {QRectF(60, 60, 290, 200), QRectF(180, 130, 1.5, 2), QRectF(180, 130, 2, 2), QRectF(0, 0, 640, 480), QRectF(500, 400, 10, 10)}) {
            QJsonArray ids; for (int id : window.annotationsInRect(marquee)) ids.append(id);
            selections.append(QJsonObject{{"scene", index}, {"before", before}, {"marquee", rect(marquee)}, {"ids", ids}, {"bounds", rect(window.selectedAnnotationsBounds())}});
        }
        // 1. 【标注多选对照】【共同变换】同一初始全部成员快照，覆盖边界、反向尺寸、等比和增量旋转吸附
        const ShotWindow::SelectionDrag handles[] = {ShotWindow::SelectionDrag::Move, ShotWindow::SelectionDrag::Rotate, ShotWindow::SelectionDrag::TopLeft, ShotWindow::SelectionDrag::Right, ShotWindow::SelectionDrag::BottomRight};
        for (int h = 0; h < 5; ++h) for (bool aspect : {false, true}) for (bool snap : {false, true}) for (QPointF current : {QPointF(400.25, 330.75), QPointF(639, 479), QPointF(0, 0)}) {
            auto sample = scene(index);
            sample.m_annotationsBeforeDrag = sample.m_annotations;
            sample.m_annotationBeforeDrag = sample.m_annotations.first();
            sample.m_annotationBoundsBeforeDrag = sample.selectedAnnotationsBounds();
            const auto bounds = sample.m_annotationBoundsBeforeDrag;
            sample.m_dragStart = h == 1 ? QPointF(bounds.center().x(), bounds.top() - 26) : bounds.center();
            sample.m_annotationDrag = handles[h];
            sample.updateAnnotationDrag(current, aspect, snap);
            transforms.append(QJsonObject{{"scene", index}, {"before", before}, {"bounds", rect(bounds)}, {"handle", h}, {"start", point(sample.m_dragStart)}, {"current", point(current)}, {"aspect", aspect}, {"snap", snap}, {"after", annotations(sample)}, {"history", sample.history}});
        }
        // 2. 【标注多选对照】【批量宽度】选择顺序与图层不同，各工具采用生产合法范围，默认值记录实际结果
        for (int requested : {-50, 1, 12, 24, 48, 72, 281, 1000}) {
            auto sample = scene(index);
            const bool changed = sample.setSelectedAnnotationWidth(requested, true);
            widths.append(QJsonObject{{"scene", index}, {"before", before}, {"requested", requested}, {"changed", changed}, {"after", annotations(sample)}, {"history", sample.history}, {"defaults", QJsonArray{sample.m_strokeWidth, sample.m_highlighterWidth, sample.m_textSize, sample.m_numberWidth, sample.m_mosaicBlockSize, sample.m_ruler.width}}});
        }
        // 3. 【标注多选对照】【连续变换】每次以上一次完整生产结果开始，验证存储尺寸与轴心没有累积漂移
        auto sample = scene(index);
        for (int step = 0; step < 4; ++step) {
            sample.m_annotationsBeforeDrag = sample.m_annotations;
            sample.m_annotationBeforeDrag = sample.m_annotations.first();
            sample.m_annotationBoundsBeforeDrag = sample.selectedAnnotationsBounds();
            sample.m_annotationHistoryCaptured = false;
            const auto bounds = sample.m_annotationBoundsBeforeDrag;
            const int h = step % 3 == 0 ? 1 : (step % 3 == 1 ? 4 : 0);
            sample.m_dragStart = h == 1 ? QPointF(bounds.center().x(), bounds.top() - 26) : bounds.center();
            sample.m_annotationDrag = handles[h];
            const QPointF current = h == 1 ? bounds.center() + QPointF(90, -40) : bounds.center() + QPointF(15.25, 12.75);
            const auto initial = annotations(sample);
            sample.updateAnnotationDrag(current, false, step == 3);
            sequences.append(QJsonObject{{"scene", index}, {"step", step}, {"before", initial}, {"bounds", rect(bounds)}, {"handle", h}, {"start", point(sample.m_dragStart)}, {"current", point(current)}, {"aspect", false}, {"snap", step == 3}, {"after", annotations(sample)}, {"history", sample.history}});
        }
    }
    // 4. 【标注多选对照】【同色历史】快捷颜色每次捕获，属性颜色每次打开只捕获首个输入，同色输入也清除重做
    for (bool property : {false, true}) {
        auto sample = scene(0);
        for (const QColor color : {QColor(30, 40, 50), QColor(90, 120, 150, 128), QColor(90, 120, 150, 128)}) {
            const auto initial = annotations(sample);
            if (property) sample.applyPropertyColor(color); else sample.setCurrentColor(color);
            colors.append(QJsonObject{{"property", property}, {"before", initial}, {"requested", QJsonArray{color.red(), color.green(), color.blue(), color.alpha()}}, {"after", annotations(sample)}, {"history", sample.history}, {"default", QJsonArray{sample.m_currentColor.red(), sample.m_currentColor.green(), sample.m_currentColor.blue(), sample.m_currentColor.alpha()}}});
        }
    }
    // 5. 【标注多选对照】【单项衔接】先执行完整共同旋转，再按完整生产骨架函数编辑端点和控制点，包括原图边缘
    for (int pointIndex = 0; pointIndex < 3; ++pointIndex) for (const QPointF delta : {QPointF(15.25, -8.5), QPointF(500, 500), QPointF(-500, -500)}) {
        auto sample = scene(2);
        sample.annotationById(38)->points.append(QPointF(350, 245));
        sample.m_annotationsBeforeDrag = sample.m_annotations;
        sample.m_annotationBeforeDrag = sample.m_annotations.first();
        sample.m_annotationBoundsBeforeDrag = sample.selectedAnnotationsBounds();
        const auto bounds = sample.m_annotationBoundsBeforeDrag;
        sample.m_dragStart = QPointF(bounds.center().x(), bounds.top() - 26);
        sample.m_annotationDrag = ShotWindow::SelectionDrag::Rotate;
        sample.updateAnnotationDrag(bounds.center() + QPointF(bounds.center().y() - sample.m_dragStart.y(), 0), false, true);
        const auto line = *sample.annotationById(38);
        sample.m_annotations = {line}; sample.setSelectedAnnotations({38});
        sample.m_annotationBeforeDrag = line;
        sample.m_lineSkeletonDragPointIndex = pointIndex;
        sample.m_annotationDrag = pointIndex == 0 ? ShotWindow::SelectionDrag::LineStart : pointIndex == 1 ? ShotWindow::SelectionDrag::LineEnd : ShotWindow::SelectionDrag::LineControl;
        const QPointF start = sample.rotatedPoint(line.points.at(pointIndex), sample.annotationUnrotatedBounds(line).center(), line.rotationDegrees);
        const auto initial = annotations(sample);
        assert(sample.updateLineAnchorDrag(start + delta));
        lineEdits.append(QJsonObject{{"point_index", pointIndex}, {"before", initial}, {"start", point(start)}, {"current", point(start + delta)}, {"after", annotations(sample)}});
    }
    QFile output(QString::fromLocal8Bit(argv[1]));
    if (!output.open(QIODevice::WriteOnly)) return 3;
    output.write(QJsonDocument(QJsonObject{{"qt", QT_VERSION_STR}, {"selection", selections}, {"transform", transforms}, {"width", widths}, {"sequence", sequences}, {"color", colors}, {"line_edit", lineEdits}}).toJson());
    return 0;
}
