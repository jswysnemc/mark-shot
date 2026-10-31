#include "translation_layout/paragraph_blocks.h"
#include "translation_layout/translation_exchange.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QtTest>

namespace {

namespace layout = markshot::translation_layout;

/**
 * 创建一个 OCR token。
 * @param text 文字。
 * @param rect 文字框，图片像素坐标。
 * @param line 行号。
 * @param index 行内序号。
 * @return OCR token。
 */
markshot::ocr::Token token(const QString &text, const QRectF &rect, int line, int index = 0)
{
    return {text, rect, line, index, 0.9};
}

}  // namespace

/// @brief 检查识别结果合并段落、段落拼接与翻译输入输出的对应
class TranslationParagraphBlocksTest final : public QObject {
    Q_OBJECT

private slots:
    /** 验证换行的英文句子合并成一段，行尾连字符断开的单词拼回，无参数和返回值。 */
    void mergesWrappedSentence()
    {
        const QVector<layout::SourceBlock> blocks = layout::groupParagraphs({
            token(QStringLiteral("The quick brown fox jumps over the lazy dog and keeps run-"), {10, 10, 400, 20}, 0),
            token(QStringLiteral("ning until the end of the paragraph, where it finally"), {10, 34, 390, 20}, 1),
            token(QStringLiteral("stops."), {10, 58, 50, 20}, 2),
        });
        QCOMPARE(blocks.size(), 1);
        QCOMPARE(blocks.first().text,
                 QStringLiteral("The quick brown fox jumps over the lazy dog and keeps running until the end of the "
                                "paragraph, where it finally stops."));
        QCOMPARE(blocks.first().box, QRectF(10, 10, 400, 68));
        QCOMPARE(blocks.first().lineBoxes.size(), 3);
        QCOMPARE(blocks.first().lineHeight, 20.0);
        QCOMPARE(blocks.first().linePitch, 24.0);
    }

    /** 验证菜单一类的短行各自成段，无参数和返回值。 */
    void keepsShortLinesApart()
    {
        const QVector<layout::SourceBlock> blocks = layout::groupParagraphs({
            token(QStringLiteral("File"), {10, 10, 40, 20}, 0),
            token(QStringLiteral("Edit"), {10, 34, 40, 20}, 1),
            token(QStringLiteral("View"), {10, 58, 44, 20}, 2),
        });
        QCOMPARE(blocks.size(), 3);
        QCOMPARE(blocks.at(1).text, QStringLiteral("Edit"));
    }

    /** 验证句末结束、列表项、字号变化和段间空白都会另起一段，无参数和返回值。 */
    void splitsAtParagraphBoundaries()
    {
        const QVector<layout::SourceBlock> blocks = layout::groupParagraphs({
            token(QStringLiteral("Settings overview for the current workspace profile"), {10, 0, 420, 32}, 0),
            token(QStringLiteral("This sentence is long enough to fill the whole first line."), {10, 40, 400, 20}, 1),
            token(QStringLiteral("A new sentence starts on the next line of the same column"), {10, 64, 400, 20}, 2),
            token(QStringLiteral("1. First item in the numbered list that follows the text"), {10, 88, 400, 20}, 3),
            token(QStringLiteral("Another paragraph after a blank line between the blocks"), {10, 140, 400, 20}, 4),
        });
        QCOMPARE(blocks.size(), 5);
    }

    /** 验证同一行号里相隔很远的两段文字拆成两栏，无参数和返回值。 */
    void splitsDistantColumnsOnOneRow()
    {
        const QVector<layout::SourceBlock> blocks = layout::groupParagraphs({
            token(QStringLiteral("Name"), {10, 10, 50, 20}, 0, 0),
            token(QStringLiteral("Value"), {300, 10, 60, 20}, 0, 1),
        });
        QCOMPARE(blocks.size(), 2);
        QCOMPARE(blocks.at(0).text, QStringLiteral("Name"));
        QCOMPARE(blocks.at(1).text, QStringLiteral("Value"));
    }

    /** 验证左右两栏的行交错出现时，各栏的行仍合并到自己的段落，无参数和返回值。 */
    void mergesInterleavedColumns()
    {
        const QString left = QStringLiteral("Left column text that is wide enough to");
        const QString right = QStringLiteral("Right column text that is wide enough");
        const QVector<layout::SourceBlock> blocks = layout::groupParagraphs({
            token(left, {10, 10, 280, 20}, 0),
            token(right, {400, 10, 280, 20}, 1),
            token(QStringLiteral("continue below."), {10, 34, 120, 20}, 2),
            token(QStringLiteral("to continue too."), {400, 34, 130, 20}, 3),
        });
        QCOMPARE(blocks.size(), 2);
        QCOMPARE(blocks.at(0).text, left + QStringLiteral(" continue below."));
        QCOMPARE(blocks.at(1).text, right + QStringLiteral(" to continue too."));
    }

    /** 验证中日韩文字跨行直接相连，西文补空格，大写开头不拼连字符，无参数和返回值。 */
    void joinsLinesByScript()
    {
        QCOMPARE(layout::joinParagraphLine(QStringLiteral("这是第一行"), QStringLiteral("第二行")),
                 QStringLiteral("这是第一行第二行"));
        QCOMPARE(layout::joinParagraphLine(QStringLiteral("中文，"), QStringLiteral("English")),
                 QStringLiteral("中文，English"));
        QCOMPARE(layout::joinParagraphLine(QStringLiteral("hello"), QStringLiteral("world")),
                 QStringLiteral("hello world"));
        QCOMPARE(layout::joinParagraphLine(QStringLiteral("well-"), QStringLiteral("Known")),
                 QStringLiteral("well- Known"));
        QCOMPARE(layout::joinParagraphLine(QString(), QStringLiteral("only")), QStringLiteral("only"));
    }

    /** 验证翻译输入每段一个 token，输出按行号或框对应回段落，无参数和返回值。 */
    void exchangesBlocksWithTranslator()
    {
        const QVector<layout::SourceBlock> blocks = layout::groupParagraphs({
            token(QStringLiteral("First"), {10, 10, 60, 20}, 0),
            token(QStringLiteral("Second"), {10, 80, 70, 20}, 1),
        });
        QCOMPARE(blocks.size(), 2);
        const QJsonObject input = QJsonDocument::fromJson(layout::translationInputJson(blocks, QStringLiteral("Japanese")))
                                      .object();
        QCOMPARE(input.value(QStringLiteral("targetLanguage")).toString(), QStringLiteral("Japanese"));
        const QJsonArray tokens = input.value(QStringLiteral("tokens")).toArray();
        QCOMPARE(tokens.size(), 2);
        QCOMPARE(tokens.at(1).toObject().value(QStringLiteral("line")).toInt(), 1);
        QCOMPARE(tokens.at(1).toObject().value(QStringLiteral("text")).toString(), QStringLiteral("Second"));

        // 行号被翻译器改乱时按框对应；与任何段落都不相交的译文丢弃
        const QByteArray output = QJsonDocument(QJsonObject{
            {QStringLiteral("tokens"),
             QJsonArray{
                 QJsonObject{{QStringLiteral("text"), QStringLiteral("Zweite")},
                             {QStringLiteral("box"), QJsonArray{10, 80, 70, 20}},
                             {QStringLiteral("line"), 0}},
                 QJsonObject{{QStringLiteral("text"), QStringLiteral("Erste")},
                             {QStringLiteral("box"), QJsonArray{10, 10, 60, 20}},
                             {QStringLiteral("line"), 0}},
                 QJsonObject{{QStringLiteral("text"), QStringLiteral("Stray")},
                             {QStringLiteral("box"), QJsonArray{300, 300, 20, 20}},
                             {QStringLiteral("line"), 7}},
             }}}).toJson(QJsonDocument::Compact);
        const QVector<QString> texts = layout::translatedBlockTexts(output, blocks, QSize(400, 400));
        QCOMPARE(texts, (QVector<QString>{QStringLiteral("Erste"), QStringLiteral("Zweite")}));
    }
};

QTEST_APPLESS_MAIN(TranslationParagraphBlocksTest)
#include "translation_paragraph_blocks_test.moc"
