#include "annotation_snapshot_codec.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

namespace markshot::rust {
namespace {

using namespace shot::types;

/** 将 annotation 的绘制数据映射为协议字段，返回 JSON 对象，不改变坐标或样式。 */
QJsonObject encodeAnnotation(const Annotation &annotation)
{
    QJsonArray points;
    for (const QPointF &point : annotation.points) {
        points.append(QJsonArray{point.x(), point.y()});
    }
    return {
        {QStringLiteral("id"), annotation.id},
        {QStringLiteral("tool"), static_cast<int>(annotation.tool)},
        {QStringLiteral("rect"), QJsonArray{annotation.rect.x(), annotation.rect.y(), annotation.rect.width(), annotation.rect.height()}},
        {QStringLiteral("points"), points},
        {QStringLiteral("text"), annotation.text},
        {QStringLiteral("number"), annotation.number},
        {QStringLiteral("color"), static_cast<qint64>(annotation.color.rgba())},
        {QStringLiteral("backgroundColor"), static_cast<qint64>(annotation.backgroundColor.rgba())},
        {QStringLiteral("width"), annotation.width},
        {QStringLiteral("filled"), annotation.filled},
        {QStringLiteral("cornerRadius"), annotation.cornerRadius},
        {QStringLiteral("arrowStyle"), static_cast<int>(annotation.arrowStyle)},
        {QStringLiteral("arrowLineStyle"), static_cast<int>(annotation.arrowLineStyle)},
        {QStringLiteral("highlighterStyle"), static_cast<int>(annotation.highlighterStyle)},
        {QStringLiteral("rotationDegrees"), annotation.rotationDegrees},
        {QStringLiteral("magnifierScale"), annotation.magnifierScale},
        {QStringLiteral("magnifierShape"), static_cast<int>(annotation.magnifierShape)},
        {QStringLiteral("numberStyle"), static_cast<int>(annotation.numberStyle)},
        {QStringLiteral("fontFamily"), annotation.fontFamily},
        {QStringLiteral("fontWeight"), static_cast<int>(annotation.fontWeight)},
        {QStringLiteral("textItalic"), annotation.textItalic},
        {QStringLiteral("rectangleStyle"), static_cast<int>(annotation.rectangleStyle)},
        {QStringLiteral("markerShape"), static_cast<int>(annotation.markerShape)},
        {QStringLiteral("mosaicStyle"), static_cast<int>(annotation.mosaicStyle)},
        {QStringLiteral("rulerMode"), static_cast<int>(annotation.rulerMode)},
    };
}

/** 将 Rust 已校验的 root 映射为 Qt 标注，返回标注或坐标协议错误空值。 */
std::optional<Annotation> decodeAnnotation(const QJsonObject &root)
{
    const QJsonArray rect = root.value(QStringLiteral("rect")).toArray();
    const QJsonValue pointsValue = root.value(QStringLiteral("points"));
    if (rect.size() != 4 || !pointsValue.isArray() || root.size() != 25) {
        return std::nullopt;
    }
    Annotation annotation;
    annotation.id = root.value(QStringLiteral("id")).toInt();
    annotation.tool = static_cast<Tool>(root.value(QStringLiteral("tool")).toInt());
    annotation.rect = QRectF(rect[0].toDouble(), rect[1].toDouble(), rect[2].toDouble(), rect[3].toDouble());
    const QJsonArray points = pointsValue.toArray();
    annotation.points.reserve(points.size());
    for (const QJsonValue &value : points) {
        const QJsonArray point = value.toArray();
        if (point.size() != 2) {
            return std::nullopt;
        }
        annotation.points.append(QPointF(point[0].toDouble(), point[1].toDouble()));
    }
    annotation.text = root.value(QStringLiteral("text")).toString();
    annotation.number = root.value(QStringLiteral("number")).toInt();
    annotation.color = QColor::fromRgba(static_cast<QRgb>(root.value(QStringLiteral("color")).toInteger()));
    annotation.backgroundColor = QColor::fromRgba(static_cast<QRgb>(root.value(QStringLiteral("backgroundColor")).toInteger()));
    annotation.width = root.value(QStringLiteral("width")).toDouble();
    annotation.filled = root.value(QStringLiteral("filled")).toBool();
    annotation.cornerRadius = root.value(QStringLiteral("cornerRadius")).toDouble();
    annotation.arrowStyle = static_cast<ArrowStyle>(root.value(QStringLiteral("arrowStyle")).toInt());
    annotation.arrowLineStyle = static_cast<ArrowLineStyle>(root.value(QStringLiteral("arrowLineStyle")).toInt());
    annotation.highlighterStyle = static_cast<HighlighterStyle>(root.value(QStringLiteral("highlighterStyle")).toInt());
    annotation.rotationDegrees = root.value(QStringLiteral("rotationDegrees")).toDouble();
    annotation.magnifierScale = root.value(QStringLiteral("magnifierScale")).toDouble();
    annotation.magnifierShape = static_cast<MagnifierShape>(root.value(QStringLiteral("magnifierShape")).toInt());
    annotation.numberStyle = static_cast<NumberStyle>(root.value(QStringLiteral("numberStyle")).toInt());
    annotation.fontFamily = root.value(QStringLiteral("fontFamily")).toString();
    annotation.fontWeight = static_cast<QFont::Weight>(root.value(QStringLiteral("fontWeight")).toInt());
    annotation.textItalic = root.value(QStringLiteral("textItalic")).toBool();
    annotation.rectangleStyle = static_cast<RectangleStyle>(root.value(QStringLiteral("rectangleStyle")).toInt());
    annotation.markerShape = static_cast<MarkerShape>(root.value(QStringLiteral("markerShape")).toInt());
    annotation.mosaicStyle = static_cast<MosaicStyle>(root.value(QStringLiteral("mosaicStyle")).toInt());
    annotation.rulerMode = static_cast<RulerMode>(root.value(QStringLiteral("rulerMode")).toInt());
    return annotation;
}

} // namespace

QByteArray encodeSnapshot(const shot::types::HistorySnapshot &snapshot)
{
    QJsonArray annotations;
    for (const auto &annotation : snapshot.annotations) {
        annotations.append(encodeAnnotation(annotation));
    }
    QJsonArray selected;
    for (int id : snapshot.selectedAnnotationIds) {
        selected.append(id);
    }
    return QJsonDocument(QJsonObject{
        {QStringLiteral("version"), 1},
        {QStringLiteral("annotations"), annotations},
        {QStringLiteral("selectedAnnotationId"), snapshot.selectedAnnotationId
            ? QJsonValue(*snapshot.selectedAnnotationId) : QJsonValue(QJsonValue::Null)},
        {QStringLiteral("selectedAnnotationIds"), selected},
        {QStringLiteral("nextNumber"), snapshot.nextNumber},
        {QStringLiteral("nextAnnotationId"), snapshot.nextAnnotationId},
    }).toJson(QJsonDocument::Compact);
}

std::optional<shot::types::HistorySnapshot> decodeSnapshot(const QByteArray &bytes)
{
    const QJsonDocument document = QJsonDocument::fromJson(bytes);
    const QJsonObject root = document.object();
    if (root.value(QStringLiteral("version")).toInt() != 1
        || !root.value(QStringLiteral("annotations")).isArray()
        || !root.value(QStringLiteral("selectedAnnotationIds")).isArray()) {
        return std::nullopt;
    }
    shot::types::HistorySnapshot snapshot;
    const QJsonArray annotations = root.value(QStringLiteral("annotations")).toArray();
    snapshot.annotations.reserve(annotations.size());
    for (const QJsonValue &value : annotations) {
        const auto annotation = decodeAnnotation(value.toObject());
        if (!annotation) {
            return std::nullopt;
        }
        snapshot.annotations.append(*annotation);
    }
    const QJsonValue selected = root.value(QStringLiteral("selectedAnnotationId"));
    if (!selected.isNull()) {
        snapshot.selectedAnnotationId = selected.toInt();
    }
    const QJsonArray ids = root.value(QStringLiteral("selectedAnnotationIds")).toArray();
    for (const QJsonValue &id : ids) {
        snapshot.selectedAnnotationIds.append(id.toInt());
    }
    snapshot.nextNumber = root.value(QStringLiteral("nextNumber")).toInt();
    snapshot.nextAnnotationId = root.value(QStringLiteral("nextAnnotationId")).toInt();
    return snapshot;
}

} // namespace markshot::rust
