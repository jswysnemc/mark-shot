#include "ocr_result.h"
#include "translation_layout/block_colors.h"
#include "translation_layout/block_layout.h"
#include "translation_layout/paragraph_blocks.h"
#include "translation_layout/translated_block.h"
#include "translation_layout/translation_exchange.h"

#include <QApplication>
#include <QFile>
#include <QFontInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPainter>

namespace layout = markshot::translation_layout;

/** 【图片翻译对照】【颜色编码】按 color 返回四通道数组，不转换色彩空间 */
QJsonArray rgba(const QColor &color)
{
    return {color.red(), color.green(), color.blue(), color.alpha()};
}

/** 【图片翻译对照】【颜色读取】按 value 返回测试颜色，缺少透明度时使用不透明值 */
QColor color(const QJsonValue &value)
{
    const auto array = value.toArray();
    return QColor(array[0].toInt(), array[1].toInt(), array[2].toInt(), array.size() > 3 ? array[3].toInt() : 255);
}

/** 【图片翻译对照】【矩形读取】按 value 返回原图矩形，保留浮点值 */
QRectF rect(const QJsonValue &value)
{
    return *markshot::ocr::rectFromJsonValue(value);
}

/** 【图片翻译对照】【输入顺序】按 array 返回未经排序的识别 token，保持行号首次顺序 */
QVector<markshot::ocr::Token> tokens(const QJsonArray &array)
{
    QVector<markshot::ocr::Token> result;
    for (const auto &value : array) {
        const auto object = value.toObject();
        result.append({object["text"].toString(), rect(object["box"]), object["line"].toInt(),
                       object["index"].toInt(), object["confidence"].toDouble(1.0)});
    }
    return result;
}

/** 【图片翻译对照】【原文编码】按 blocks 返回原版段落及行高、行距和全部行框 */
QJsonArray sources(const QVector<layout::SourceBlock> &blocks)
{
    QJsonArray result;
    for (const auto &block : blocks) {
        QJsonArray boxes;
        for (const auto &box : block.lineBoxes) {
            boxes.append(markshot::ocr::rectToJson(box));
        }
        result.append(QJsonObject{{"text", block.text}, {"rect", markshot::ocr::rectToJson(block.box)},
                                  {"line_height", block.lineHeight}, {"line_pitch", block.linePitch},
                                  {"line_boxes", boxes}});
    }
    return result;
}

/** 【图片翻译对照】【译文编码】按 blocks 返回原版全部排版数据，不只保留可见全文 */
QJsonArray translated(const QVector<layout::TranslatedBlock> &blocks)
{
    QJsonArray result;
    for (const auto &block : blocks) {
        QJsonArray lines;
        for (const auto &line : block.lines) {
            lines.append(QJsonObject{{"text", line.text}, {"rect", markshot::ocr::rectToJson(line.rect)},
                                     {"baseline", line.baseline}});
        }
        result.append(QJsonObject{{"source", markshot::ocr::rectToJson(block.sourceBox)},
                                  {"plate", markshot::ocr::rectToJson(block.plateRect)},
                                  {"background", rgba(block.background)}, {"foreground", rgba(block.foreground)},
                                  {"font_size", block.font.pixelSize()}, {"text", block.text}, {"lines", lines}});
    }
    return result;
}

/** 【图片翻译对照】【原图样本】按 object 创建预乘图像，矩形填色使用原版颜色语义 */
QImage image(const QJsonObject &object)
{
    QImage image(object["width"].toInt(), object["height"].toInt(), QImage::Format_ARGB32_Premultiplied);
    image.fill(color(object["background"]));
    QPainter painter(&image);
    painter.setCompositionMode(QPainter::CompositionMode_Source);
    for (const auto &value : object["marks"].toArray()) {
        const auto mark = value.toObject();
        painter.fillRect(rect(mark["rect"]), color(mark["color"]));
    }
    return image;
}

/** 【图片翻译对照】【独立入口】读取 argv 输入并写出真实原版结果，不改写参考仓库 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    QFile input(argv[1]);
    if (!input.open(QIODevice::ReadOnly)) {
        return 2;
    }
    const auto cases = QJsonDocument::fromJson(input.readAll()).object();
    QJsonObject result{{"qt", QT_VERSION_STR}, {"font", QFontInfo(QFont("Noto Sans")).family()}};
    QJsonArray groups;
    for (const auto &value : cases["groups"].toArray()) {
        const auto entry = value.toObject();
        groups.append(QJsonObject{{"name", entry["name"]}, {"blocks", sources(layout::groupParagraphs(tokens(entry["tokens"].toArray())))}});
    }
    result["groups"] = groups;
    QJsonArray joins;
    for (const auto &value : cases["joins"].toArray()) {
        const auto entry = value.toArray();
        joins.append(layout::joinParagraphLine(entry[0].toString(), entry[1].toString()));
    }
    result["joins"] = joins;
    QJsonArray colors;
    for (const auto &value : cases["colors"].toArray()) {
        const auto entry = value.toObject();
        const auto sampled = layout::sampleBlockColors(image(entry), rect(entry["rect"]), entry["line_height"].toDouble());
        colors.append(QJsonObject{{"name", entry["name"]}, {"background", rgba(sampled.background)}, {"foreground", rgba(sampled.foreground)}});
    }
    result["colors"] = colors;
    QJsonArray layouts;
    for (const auto &value : cases["layouts"].toArray()) {
        const auto entry = value.toObject();
        const auto blocks = layout::groupParagraphs(tokens(entry["tokens"].toArray()));
        QVector<QString> texts;
        for (const auto &text : entry["texts"].toArray()) {
            texts.append(text.toString());
        }
        const auto fitted = layout::layoutTranslatedBlocks(image(entry), blocks, texts, QFont("Noto Sans"));
        const auto shifted = layout::offsetBlocks(fitted, QPointF(-15.0, 7.0), QRectF(0, 0, 100, 100));
        layouts.append(QJsonObject{{"name", entry["name"]}, {"blocks", translated(fitted)}, {"text", layout::translatedText(fitted)}, {"shifted", translated(shifted)}});
    }
    result["layouts"] = layouts;
    QJsonArray exchanges;
    for (const auto &value : cases["exchanges"].toArray()) {
        const auto entry = value.toObject();
        const auto blocks = layout::groupParagraphs(tokens(entry["tokens"].toArray()));
        const auto output = QJsonDocument(entry["output"].toObject()).toJson(QJsonDocument::Compact);
        const auto texts = layout::translatedBlockTexts(output, blocks, QSize(400, 200));
        QJsonArray array;
        for (const auto &text : texts) {
            array.append(text);
        }
        exchanges.append(QJsonObject{{"name", entry["name"]}, {"input", QJsonDocument::fromJson(layout::translationInputJson(blocks, "English")).object()}, {"texts", array}});
    }
    result["exchanges"] = exchanges;
    QFile output(argv[2]);
    if (!output.open(QIODevice::WriteOnly)) {
        return 3;
    }
    output.write(QJsonDocument(result).toJson(QJsonDocument::Compact));
    return 0;
}
