/** 【区域识别参考】【原版调用】从结构化输入采集实际 Qt 候选、像素算法与滚轮状态 */

#include "region_detection/region_detector.h"
#include "region_detection/region_detection_internal.h"
#include "region_detection/image_components.h"
#include "region_detection/image_filters.h"
#include "window_hover_selection.h"

#include <QCoreApplication>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <memory>

namespace rd = markshot::region_detection;

/** 【区域识别参考】【矩形读取】按 value 读取 x、y、width、height，返回 Qt 矩形 */
QRect rect(const QJsonValue &value)
{
    const QJsonArray a = value.toArray();
    return QRect(a[0].toInt(), a[1].toInt(), a[2].toInt(), a[3].toInt());
}

/** 【区域识别参考】【矩形输出】按 value 编码 x、y、width、height，返回结构化数组 */
QJsonArray encodedRect(QRect value)
{
    return {value.x(), value.y(), value.width(), value.height()};
}

/** 【区域识别参考】【候选读取】按 object 读取边界、质量和来源，返回原版候选 */
rd::RegionCandidate candidate(const QJsonObject &object)
{
    rd::RegionCandidate result;
    result.rect = rect(object["rect"]);
    result.quality = object["quality"].toDouble();
    const QJsonArray sides = object["sides"].toArray();
    for (int index = 0; index < 4; ++index) result.sides[index] = sides[index].toDouble();
    result.interiorEdges = object["interior_edges"].toDouble();
    for (const QJsonValue &source : object["sources"].toArray()) result.sources.append(source.toString());
    return result;
}

/** 【区域识别参考】【候选输出】按 values 编码全部实际边界及评分，返回候选数组 */
QJsonArray encodedCandidates(const QVector<rd::RegionCandidate> &values)
{
    QJsonArray result;
    for (const auto &value : values) {
        QJsonArray sides, sources;
        for (float side : value.sides) sides.append(side);
        for (const QString &source : value.sources) sources.append(source);
        result.append(QJsonObject{{"rect", encodedRect(value.rect)}, {"quality", value.quality},
                                  {"sides", sides}, {"interior_edges", value.interiorEdges}, {"sources", sources}});
    }
    return result;
}

/** 【区域识别参考】【像素摘要】按 bytes 计算确定性六十四位 FNV 摘要，返回十六进制字符串 */
QString hash(const QVector<quint8> &bytes)
{
    quint64 hash = 14695981039346656037ULL;
    for (quint8 value : bytes) hash = (hash ^ value) * 1099511628211ULL;
    return QString::number(hash, 16);
}

/** 【区域识别参考】【彩色摘要】按 image 以 RGBA 顺序读取通道，返回与主机字节序无关的像素摘要 */
QString rgbHash(const QImage &image)
{
    QVector<quint8> bytes;
    bytes.reserve(image.width() * image.height() * 4);
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            const QRgb value = image.pixel(x, y);
            bytes.append(qRed(value)); bytes.append(qGreen(value)); bytes.append(qBlue(value)); bytes.append(qAlpha(value));
        }
    }
    return hash(bytes);
}

/** 【区域识别参考】【连通域输出】按 image、connectivity 调用原版连通域，返回非空矩形与像素数 */
QJsonArray components(const rd::GrayImage &image, rd::Connectivity connectivity)
{
    const auto items = rd::connectedComponents(image, connectivity);
    QJsonArray result;
    for (int index = 1; index < items.size(); ++index) {
        const auto &item = items[index];
        result.append(QJsonObject{{"rect", encodedRect(QRect(item.left, item.top, item.right - item.left, item.bottom - item.top))},
                                  {"pixels", qint64(item.pixels)}});
    }
    return result;
}

/** 【区域识别参考】【图像步骤】按 image 调用原版缩图、边缘和形态运算，返回中间摘要和连通域 */
QJsonObject imageSteps(const QImage &image)
{
    QImage normalized = rd::normalizedRgbImage(image);
    const QSize size = rd::analysisSize(image.width(), image.height());
    if (normalized.size() != size) normalized = normalized.scaled(size, Qt::IgnoreAspectRatio, Qt::SmoothTransformation);
    if (normalized.format() != QImage::Format_ARGB32 && normalized.format() != QImage::Format_RGB32)
        normalized = normalized.convertToFormat(QImage::Format_ARGB32);
    const auto field = rd::buildEdgeField(normalized);
    const auto gray = rd::grayscaleFromRgba(normalized);
    const float high = qBound(15.0f, float(field.threshold) * 3.0f, 54.0f);
    const auto canny = rd::canny(gray, high * 0.4f, high);
    const auto closed = rd::closeLInf1(field.combined);
    return {{"size", QJsonArray{size.width(), size.height()}}, {"rgba", rgbHash(normalized)},
            {"gray", hash(gray.pixels)}, {"canny", hash(canny.pixels)}, {"vertical", hash(field.vertical.pixels)},
            {"horizontal", hash(field.horizontal.pixels)}, {"combined", hash(field.combined.pixels)},
            {"closed", hash(closed.pixels)}, {"dilated", hash(rd::dilateLInf1(field.combined).pixels)},
            {"threshold", field.threshold}, {"components_four", components(closed, rd::Connectivity::Four)},
            {"components_eight", components(closed, rd::Connectivity::Eight)}};
}

/** 【区域识别参考】【动作坐标】按 detector、action 读取图像或归一化坐标，返回原版整数图像位置 */
QPoint point(const rd::RegionDetector &detector, const QJsonObject &action)
{
    if (action.contains("normalized")) {
        const auto value = action["normalized"].toArray();
        return detector.pointFromNormalized(value[0].toDouble(), value[1].toDouble());
    }
    const auto value = action["point"].toArray();
    return {value[0].toInt(), value[1].toInt()};
}

/** 【区域识别参考】【单个场景】按 input 调用真实 Qt 实现，返回候选、层级和每个动作的实际结果 */
QJsonObject run(const QJsonObject &input)
{
    const QString mode = input["mode"].toString();
    QJsonObject result{{"name", input["name"]}};
    if (mode == "windows") {
        QVector<markshot::WindowInfo> windows;
        for (const auto &value : input["windows"].toArray()) {
            const auto object = value.toObject();
            markshot::WindowInfo info;
            info.rect = rect(object["rect"]);
            if (!object["z"].isNull() && object.contains("z")) info.zOrder = object["z"].toInt();
            windows.append(info);
        }
        QJsonArray actions;
        for (const auto &value : input["actions"].toArray()) {
            const auto p = value.toObject()["point"].toArray();
            const auto rect = markshot::window_selection::topmostWindowRectAt(windows, {p[0].toInt(), p[1].toInt()});
            actions.append(rect ? QJsonValue(encodedRect(*rect)) : QJsonValue(QJsonValue::Null));
        }
        result["actions"] = actions;
        return result;
    }
    QImage image;
    QVector<rd::RegionCandidate> candidates;
    std::unique_ptr<rd::RegionDetector> detector;
    if (mode == "image") {
        image = QImage(input["image"].toString());
        if (image.isNull()) return QJsonObject{{"error", "image could not be loaded"}};
        if (input["premultiplied"].toBool()) image = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        detector = std::make_unique<rd::RegionDetector>(image);
        result["steps"] = imageSteps(image);
    } else {
        for (const auto &value : input["candidates"].toArray()) candidates.append(candidate(value.toObject()));
        for (const auto &value : input["merges"].toArray()) {
            const auto merge = value.toObject();
            rd::mergeCandidate(&candidates, candidate(merge["candidate"].toObject()), merge["tolerance"].toInt());
        }
        detector = std::make_unique<rd::RegionDetector>(input["width"].toInt(), input["height"].toInt(), candidates);
    }
    QVector<QRect> windows;
    for (const auto &value : input["window_rects"].toArray()) windows.append(rect(value));
    detector->addWindowRects(windows);
    result["bounds"] = encodedRect(detector->bounds());
    result["candidates"] = encodedCandidates(detector->candidates());
    rd::SelectionCursor cursor;
    QJsonArray actions;
    for (const auto &value : input["actions"].toArray()) {
        const auto action = value.toObject();
        const auto p = point(*detector, action);
        const auto kind = action["kind"].toString();
        QJsonValue selection;
        if (kind == "region") selection = encodedRect(detector->regionAt(p));
        else if (kind == "regions") {
            QJsonArray chain;
            for (const auto &rect : detector->regionsAt(p)) chain.append(encodedRect(rect));
            selection = chain;
        } else if (kind == "hover") selection = encodedRect(cursor.hover(*detector, p));
        else if (kind == "step") selection = encodedRect(cursor.step(*detector, p, action["direction"].toInt()));
        else if (kind == "reset") { cursor.reset(); selection = QJsonValue(QJsonValue::Null); }
        actions.append(QJsonObject{{"point", QJsonArray{p.x(), p.y()}}, {"selection", selection},
                                   {"manual", cursor.manualLevel()}, {"current", cursor.current() ? QJsonValue(encodedRect(*cursor.current())) : QJsonValue(QJsonValue::Null)}});
    }
    result["actions"] = actions;
    return result;
}

/** 【区域识别参考】【命令入口】按 argv 中输入和输出路径运行结构化场景，argc 为参数数量；返回文件或采集状态 */
int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    if (argc != 3) return 2;
    QFile input(QString::fromLocal8Bit(argv[1]));
    if (!input.open(QIODevice::ReadOnly)) return 3;
    const auto request = QJsonDocument::fromJson(input.readAll());
    if (!request.isObject()) return 4;
    QJsonArray cases;
    for (const auto &value : request.object()["cases"].toArray()) {
        const auto result = run(value.toObject());
        if (result.contains("error")) return 5;
        cases.append(result);
    }
    QFile output(QString::fromLocal8Bit(argv[2]));
    if (!output.open(QIODevice::WriteOnly)) return 6;
    output.write(QJsonDocument(QJsonObject{{"qt_version", qVersion()}, {"cases", cases}}).toJson(QJsonDocument::Compact));
    return 0;
}
