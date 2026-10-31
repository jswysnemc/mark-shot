#include "translation_layout/block_colors.h"
#include "translation_layout/block_layout.h"
#include "translation_layout/block_painter.h"
#include "translation_layout/translated_block.h"

#include <QFontMetricsF>
#include <QGuiApplication>
#include <QPainter>
#include <QtTest>

namespace {

namespace layout = markshot::translation_layout;

/**
 * 创建一个原文段落。
 * @param text 段落文字。
 * @param box 段落包围盒。
 * @param lineHeight 行高。
 * @param lineCount 行数，行距等于行高加 4px。
 * @return 原文段落。
 */
layout::SourceBlock sourceBlock(const QString &text, const QRectF &box, qreal lineHeight, int lineCount = 1)
{
    layout::SourceBlock block;
    block.text = text;
    block.box = box;
    block.lineHeight = lineHeight;
    block.linePitch = lineCount > 1 ? lineHeight + 4.0 : 0.0;
    for (int line = 0; line < lineCount; ++line) {
        block.lineBoxes.append(QRectF(box.left(), box.top() + line * (lineHeight + 4.0), box.width(), lineHeight));
    }
    return block;
}

/**
 * 创建一张底色图片，并在指定位置画上文字色块。
 * @param background 底色。
 * @param ink 文字色。
 * @param inkRects 文字色块。
 * @return 测试图片。
 */
QImage paintedImage(const QColor &background, const QColor &ink, const QVector<QRect> &inkRects)
{
    QImage image(400, 300, QImage::Format_ARGB32_Premultiplied);
    image.fill(background);
    QPainter painter(&image);
    for (const QRect &rect : inkRects) {
        painter.fillRect(rect, ink);
    }
    return image;
}

}  // namespace

/// @brief 检查译文取色、按段落排版、绘制和转贴图时的坐标换算
class TranslationBlockLayoutTest final : public QObject {
    Q_OBJECT

private slots:
    /** 验证背景取框外一圈的主色，文字颜色取框内与背景差别最大的像素，无参数和返回值。 */
    void samplesBackgroundAndInk()
    {
        const QImage image = paintedImage(QColor(30, 41, 59), QColor(250, 204, 21),
                                          {QRect(24, 24, 6, 12), QRect(40, 24, 6, 12), QRect(56, 24, 6, 12)});
        const layout::BlockColors colors = layout::sampleBlockColors(image, QRectF(20, 20, 50, 20), 20);
        QCOMPARE(colors.background, QColor(30, 41, 59));
        QCOMPARE(colors.foreground, QColor(250, 204, 21));
    }

    /** 验证文字与背景对比不足时按背景明暗改用深色字，无参数和返回值。 */
    void fallsBackToReadableInk()
    {
        const QImage image = paintedImage(QColor(240, 240, 240), QColor(225, 225, 225), {QRect(24, 24, 20, 12)});
        const layout::BlockColors colors = layout::sampleBlockColors(image, QRectF(20, 20, 50, 20), 20);
        QCOMPARE(colors.background, QColor(240, 240, 240));
        QCOMPARE(colors.foreground, QColor(17, 24, 39));
    }

    /** 验证短译文留在原文框内、单行垂直居中，并沿用原图配色，无参数和返回值。 */
    void placesShortTranslationInSourceBox()
    {
        const QImage image = paintedImage(Qt::white, Qt::black, {QRect(20, 20, 200, 20)});
        const layout::SourceBlock source = sourceBlock(QStringLiteral("Hello world"), QRectF(20, 20, 200, 20), 20);
        const QVector<layout::TranslatedBlock> blocks =
            layout::layoutTranslatedBlocks(image, {source}, {QStringLiteral("Hi")}, QFont());
        QCOMPARE(blocks.size(), 1);
        const layout::TranslatedBlock &block = blocks.first();
        QVERIFY(block.plateRect.contains(source.box));
        QCOMPARE(block.background, QColor(255, 255, 255));
        QCOMPARE(block.lines.size(), 1);
        QVERIFY(block.font.pixelSize() >= 9 && block.font.pixelSize() <= 20);
        QVERIFY(block.plateRect.contains(block.lines.first().rect.topLeft()));
        QVERIFY(qAbs(block.lines.first().rect.center().y() - source.box.center().y()) <= 1.0);
        QCOMPARE(layout::translatedText(blocks), QStringLiteral("Hi"));
    }

    /** 验证空译文的段落不排版，无参数和返回值。 */
    void skipsEmptyTranslations()
    {
        const QImage image = paintedImage(Qt::white, Qt::black, {});
        const QVector<layout::TranslatedBlock> blocks = layout::layoutTranslatedBlocks(
            image, {sourceBlock(QStringLiteral("a"), {10, 10, 50, 20}, 20)}, {QStringLiteral("  ")}, QFont());
        QVERIFY(blocks.isEmpty());
    }

    /** 验证长译文先缩小字号，底板向下延伸时不压到下一段，无参数和返回值。 */
    void longTranslationStopsAboveNextBlock()
    {
        const QImage image = paintedImage(Qt::white, Qt::black, {});
        const layout::SourceBlock first = sourceBlock(QStringLiteral("Short"), QRectF(20, 20, 120, 20), 20);
        const layout::SourceBlock second = sourceBlock(QStringLiteral("Next"), QRectF(20, 60, 120, 20), 20);
        const QString longText = QStringLiteral("This translation is much longer than the original text and "
                                                "cannot possibly fit into a single short line at the starting size");
        const QVector<layout::TranslatedBlock> blocks =
            layout::layoutTranslatedBlocks(image, {first, second}, {longText, QStringLiteral("Next")}, QFont());
        QCOMPARE(blocks.size(), 2);
        const layout::TranslatedBlock &block = blocks.first();
        QVERIFY(block.plateRect.bottom() < second.box.top());
        QCOMPARE(block.text, longText);
        QVERIFY(!block.lines.isEmpty());
        // 有可用字形时字号一定被缩小，行数受底板高度限制
        if (QFontMetricsF(block.font).horizontalAdvance(longText) > first.box.width()) {
            QCOMPARE(block.font.pixelSize(), 9);
            QVERIFY(block.lines.constLast().rect.bottom() <= block.plateRect.bottom() + 1.0);
        }
    }

    /** 验证绘制时底板用原图背景色盖住原文，无参数和返回值。 */
    void paintCoversSourceText()
    {
        QImage image = paintedImage(QColor(219, 234, 254), QColor(30, 64, 175), {QRect(22, 22, 196, 16)});
        const layout::SourceBlock source = sourceBlock(QStringLiteral("Hello world"), QRectF(20, 20, 200, 20), 20);
        const QVector<layout::TranslatedBlock> blocks =
            layout::layoutTranslatedBlocks(image, {source}, {QStringLiteral("Hi")}, QFont());
        QPainter painter(&image);
        layout::paintTranslatedBlocks(painter, blocks);
        painter.end();
        // 译文只占左侧一小段，原文色块的右端应被底板盖成背景色
        QCOMPARE(image.pixelColor(214, 30), QColor(219, 234, 254));
        QCOMPARE(blocks.first().foreground, QColor(30, 64, 175));
    }

    /** 验证译文行转成连续编号的 token，平移后去掉范围外的段落，无参数和返回值。 */
    void convertsLinesAndOffsetsBlocks()
    {
        layout::TranslatedBlock first;
        first.sourceBox = QRectF(10, 10, 100, 40);
        first.plateRect = QRectF(8, 8, 104, 44);
        first.text = QStringLiteral("one two");
        first.lines = {{QStringLiteral("one"), QRectF(10, 10, 30, 18), 24}, {QStringLiteral("two"), QRectF(10, 30, 30, 18), 44}};
        layout::TranslatedBlock second = first;
        second.sourceBox.translate(0, 200);
        second.plateRect.translate(0, 200);
        second.lines = {{QStringLiteral("far"), QRectF(10, 210, 30, 18), 224}};

        const QVector<markshot::ocr::Token> tokens = layout::translatedLineTokens({first, second});
        QCOMPARE(tokens.size(), 3);
        QCOMPARE(tokens.at(1).text, QStringLiteral("two"));
        QCOMPARE(tokens.at(1).line, 1);
        QCOMPARE(tokens.at(2).line, 2);

        const QVector<layout::TranslatedBlock> shifted =
            layout::offsetBlocks({first, second}, QPointF(-5, -5), QRectF(0, 0, 150, 100));
        QCOMPARE(shifted.size(), 1);
        QCOMPARE(shifted.first().plateRect, QRectF(3, 3, 104, 44));
        QCOMPARE(shifted.first().lines.at(1).rect, QRectF(5, 25, 30, 18));
        QCOMPARE(shifted.first().lines.at(1).baseline, 39.0);
    }
};

QTEST_MAIN(TranslationBlockLayoutTest)
#include "translation_block_layout_test.moc"
