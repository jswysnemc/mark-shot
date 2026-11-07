// 【文字存储对照】【生产状态】连续操作每次读取实际原图选框，比较存储矩形与绘制尺寸
#include "probe.h"
#include "production.inc"
#include <QApplication>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QScreen>

using namespace markshot::shot;

/** 【文字存储对照】【矩形编码】按value返回左、上、宽、高，保持生产函数的双精度结果 */
QJsonArray rect(QRectF value) { return {value.x(), value.y(), value.width(), value.height()}; }

/** 【文字存储对照】【状态编码】按annotation返回存储矩形、原图选框、字号及换行状态，scale是预览比例 */
QJsonObject state(const types::Annotation &annotation, qreal scale = 1.0)
{
    const QSizeF image = textAnnotationBoxSize(annotation, 1.0), painted = textAnnotationBoxSize(annotation, scale);
    const QPointF anchor = annotation.points.value(0);
    return {{"stored", rect(annotation.rect)}, {"anchor", QJsonArray{anchor.x(), anchor.y()}},
            {"source_frame", rect(QRectF(textAnnotationBaseRect(annotation).topLeft(), image))}, {"source", QJsonArray{image.width(), image.height()}},
            {"painted", QJsonArray{painted.width(), painted.height()}}, {"points", 19.0 + annotation.width},
            {"wraps", textAnnotationWraps(annotation)}};
}

/** 【文字存储对照】【完整入口】argc、argv提供输出文件，返回零表示全部样本已写入 */
int main(int argc, char **argv)
{
    QApplication application(argc, argv);
    if (argc != 2) return 2;
    ShotWindow window;
    QJsonArray sequences, single;
    // 1. 【文字存储对照】【连续手势】覆盖重复扩宽、跨过换行阈值、重复改高及编辑收框
    for (QString text : {QStringLiteral("Rotation"), QStringLiteral("alpha beta gamma")}) {
        for (QString name : {QStringLiteral("widen-twice"), QStringLiteral("widen-shrink"), QStringLiteral("height-twice")}) {
            types::Annotation annotation;
            annotation.text = text;
            annotation.points = {QPointF(80, 90)};
            annotation.rect = QRectF(80, 90, 400, 140);
            fitTextAnnotationRect(annotation);
            QJsonArray steps{state(annotation)};
            for (int step = 0; step < 2; ++step) {
                const QRectF before(annotation.rect.topLeft(), textAnnotationBoxSize(annotation, 1.0));
                QRectF target = before;
                if (name == QStringLiteral("height-twice")) {
                    window.m_annotationDrag = ShotWindow::SelectionDrag::Bottom;
                    target.setHeight(before.height() + 24.0);
                } else {
                    window.m_annotationDrag = ShotWindow::SelectionDrag::Right;
                    target.setWidth(before.width() + (name == QStringLiteral("widen-shrink") && step == 1 ? -40.0 : 24.0));
                }
                window.transformAnnotation(annotation, before, target);
                steps.append(QJsonObject{{"target", rect(target)}, {"after", state(annotation)}});
            }
            types::Annotation committed = annotation;
            fitTextAnnotationRect(committed, textAnnotationWraps(annotation) ? std::optional<qreal>(annotation.rect.width()) : std::nullopt);
            sequences.append(QJsonObject{{"name", name}, {"text", text}, {"steps", steps}, {"fitted", state(committed)}});
        }
    }
    // 2. 【文字存储对照】【八向映射】已有宽高可能与可见框不同，原图边缘的映射点使用最后一个像素
    const ShotWindow::SelectionDrag handles[] = {ShotWindow::SelectionDrag::TopLeft, ShotWindow::SelectionDrag::Top,
        ShotWindow::SelectionDrag::TopRight, ShotWindow::SelectionDrag::Right, ShotWindow::SelectionDrag::BottomRight,
        ShotWindow::SelectionDrag::Bottom, ShotWindow::SelectionDrag::BottomLeft, ShotWindow::SelectionDrag::Left};
    for (QRectF raw : {QRectF(80, 90, 400, 140), QRectF(80, 90, 70, 8), QRectF(600, 440, 100, 100)}) {
        for (int handle = 0; handle < 8; ++handle) {
            for (QPointF pointer : {QPointF(30, 30), QPointF(220.125, 240.75), QPointF(700, 520)}) {
                for (bool aspect : {false, true}) {
                    types::Annotation annotation;
                    annotation.rect = raw;
                    annotation.points = {raw.topLeft()};
                    const QRectF visible(raw.topLeft(), textAnnotationBoxSize(annotation, 1.0));
                    const QRectF before = visible.intersected(QRectF(0, 0, 640, 480));
                    window.m_annotationDrag = handles[handle];
                    const QRectF target = window.resizedBounds(before, handles[handle], pointer, aspect);
                    window.transformAnnotation(annotation, before, target);
                    single.append(QJsonObject{{"before", rect(raw)}, {"frame", rect(before)}, {"target", rect(target)},
                        {"handle", handle}, {"aspect", aspect}, {"after", state(annotation)}});
                }
            }
        }
    }
    // 3. 【文字存储对照】【原图与预览】同一存储矩形的选框以原图度量，字形尺寸随字号取整单独变化
    QJsonArray scales;
    for (qreal scale : {0.5, 2.0/3.0, 0.8, 1.0, 1.5, 2.0}) {
        types::Annotation annotation;
        annotation.rect = QRectF(80, 90, 400, 140);
        fitTextAnnotationRect(annotation);
        scales.append(QJsonObject{{"scale", scale}, {"after", state(annotation, scale)}});
    }
    // 4. 【文字存储对照】【空框移动】空矩形使用独立首个定点回退，矩形平移与定点像素限制分别保留
    QJsonArray fallbacks;
    for (QRectF raw : {QRectF(80, 90, 0, 8), QRectF(80, 90, 70, 0), QRectF(600, 440, 0, 0)}) {
        for (QPointF delta : {QPointF(0, 0), QPointF(-200, -200), QPointF(100, 100)}) {
            types::Annotation annotation;
            annotation.rect = raw;
            annotation.points = {raw.topLeft()};
            const QJsonObject before = state(annotation);
            window.moveAnnotation(annotation, delta);
            fallbacks.append(QJsonObject{{"before", before}, {"delta", QJsonArray{delta.x(), delta.y()}}, {"after", state(annotation)}});
        }
    }
    QFile output(QString::fromLocal8Bit(argv[1]));
    if (!output.open(QIODevice::WriteOnly)) return 3;
    output.write(QJsonDocument(QJsonObject{{"qt", QT_VERSION_STR}, {"dpi", application.primaryScreen()->logicalDotsPerInch()},
        {"sequences", sequences}, {"single", single}, {"scales", scales}, {"fallbacks", fallbacks}}).toJson());
    return 0;
}
