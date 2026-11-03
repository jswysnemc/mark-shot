// 【文字对照】【原版渲染】只编译原版排版与主题，不创建截图会话、不读写用户配置

#include "text_annotation_layout.h"
#include <QApplication>
#include <QFile>
#include <QFontInfo>
#include <QFontMetricsF>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPainter>
#include <QTextOption>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>
#include <cmath>

/** 【文字对照】【颜色输入】将 text 解析为 RGB 或 RGBA，fallback 提供默认色；返回有效 QColor。 */
QColor color(QString text, QColor fallback)
{
    if (text.size() == 9) {
        bool ok = false;
        const int alpha = text.mid(7, 2).toInt(&ok, 16);
        QColor result(text.left(7));
        if (ok && result.isValid()) { result.setAlpha(alpha); return result; }
    }
    QColor result(text);
    return result.isValid() ? result : fallback;
}

/** 【文字对照】【程序入口】argc/argv 提供 JSON 配置和输出前缀；输出原版边界、字体信息与 PNG，返回进程状态。 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    if (argc != 3) return 2;
    QFile input(QString::fromUtf8(argv[1]));
    if (!input.open(QIODevice::ReadOnly)) return 3;
    const auto config = QJsonDocument::fromJson(input.readAll()).object();
    markshot::shot::types::Annotation annotation;
    annotation.tool = markshot::shot::types::Tool::Text;
    annotation.text = config.value("text").toString("Mark Shot");
    annotation.width = config.value("points").toInt(20) - 19;
    annotation.fontFamily = config.value("family").toString("Sans Serif");
    annotation.fontWeight = config.value("bold").toBool(true) ? QFont::DemiBold : QFont::Normal;
    annotation.textItalic = config.value("italic").toBool(false);
    annotation.color = color(config.value("color").toString(), QColor(255,77,77));
    annotation.backgroundColor = color(config.value("background").toString(), QColor(0,0,0,0));
    annotation.rect = QRectF(16,16,config.value("wrap_width").toDouble(100000),1);
    const qreal scale = config.value("scale").toDouble(1.0);
    const QSizeF box = markshot::shot::textAnnotationBoxSize(annotation, scale);
    const QRectF bounds(QPointF(16 * scale,16 * scale),box);
    QFont font = markshot::theme::textFont(qRound((19.0 + annotation.width) * scale),annotation.fontWeight,annotation.fontFamily);
    font.setItalic(annotation.textItalic);
    QImage image(qCeil(bounds.right()+16*scale),qCeil(bounds.bottom()+16*scale),QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::transparent);
    QPainter painter(&image);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setFont(font);
    if (annotation.backgroundColor.alpha() > 0) {
        painter.setPen(Qt::NoPen); painter.setBrush(annotation.backgroundColor);
        painter.drawRoundedRect(bounds,4*scale,4*scale);
    }
    painter.setPen(annotation.color); painter.setBrush(Qt::NoBrush);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    option.setAlignment(Qt::AlignLeft|Qt::AlignTop);
    painter.drawText(bounds.adjusted(6*scale,4*scale,-6*scale,-4*scale),annotation.text,option);
    painter.end();
    const QString prefix = QString::fromUtf8(argv[2]);
    if (!image.save(prefix+".png")) return 4;
    const QFontInfo info(font);
    const QFontMetricsF metrics(font);
    QTextDocument document;
    document.setDocumentMargin(0);
    document.setDefaultFont(font);
    document.setDefaultTextOption(option);
    document.setPlainText(annotation.text);
    document.setTextWidth(markshot::shot::textAnnotationWraps(annotation)
        ? std::max(16.0 * scale, (annotation.rect.width() - 12.0) * scale) : -1.0);
    document.size();
    QJsonArray lines;
    for (auto block = document.begin(); block.isValid(); block = block.next()) {
        const auto *layout = block.layout();
        for (int index = 0; layout && index < layout->lineCount(); ++index) {
            const auto line = layout->lineAt(index);
            QJsonArray families;
            for (const auto &run : line.glyphRuns()) families.append(run.rawFont().familyName());
            lines.append(QJsonObject{{"text", block.text().mid(line.textStart(), line.textLength())},
                {"width",line.naturalTextWidth()},{"height",line.height()}, {"families",families}});
        }
    }
    const QJsonObject result{{"bounds",QJsonArray{bounds.x(),bounds.y(),bounds.width(),bounds.height()}},
        {"font_family",info.family()},{"font_pixels",info.pixelSize()},{"font_points",info.pointSizeF()},
        {"ascent",metrics.ascent()},{"descent",metrics.descent()},{"leading",metrics.leading()},
        {"line_height",metrics.height()},{"line_spacing",metrics.lineSpacing()},
        {"advance",metrics.horizontalAdvance(annotation.text)},
        {"lines",lines},
        {"wrapped",markshot::shot::textAnnotationWraps(annotation)}};
    QFile output(prefix+".json");
    if (!output.open(QIODevice::WriteOnly)) return 5;
    output.write(QJsonDocument(result).toJson());
    return 0;
}
