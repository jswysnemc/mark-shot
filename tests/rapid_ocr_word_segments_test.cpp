#include "rapid_ocr_word_segments.h"

#include <QtTest/QtTest>

using namespace markshot::ocr_rapid;

namespace {

/**
 * 构造一个 CTC 解码字符。
 * @param text 字符文本。
 * @param startStep 起始时间步。
 * @param endStep 结束时间步。
 * @param confidence 字符置信度。
 * @return CTC 解码字符。
 */
RapidRecCharacter makeCharacter(const QString &text,
                                int startStep,
                                int endStep,
                                qreal confidence = 0.9)
{
    RapidRecCharacter character;
    character.text = text;
    character.startStep = startStep;
    character.endStep = endStep;
    character.confidence = confidence;
    return character;
}

}  // namespace

class RapidOcrWordSegmentsTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证英文与数字按空格切分为词。
     * @return 无返回值。
     */
    void splitsLatinWordsBySpaces()
    {
        RapidRecResult result;
        result.text = QStringLiteral("HELLO 123");
        result.confidence = 0.9;
        result.stepCount = 10;
        result.characters = {
            makeCharacter(QStringLiteral("H"), 0, 0),
            makeCharacter(QStringLiteral("E"), 1, 1),
            makeCharacter(QStringLiteral("L"), 2, 2),
            makeCharacter(QStringLiteral("L"), 3, 3),
            makeCharacter(QStringLiteral("O"), 4, 4),
            makeCharacter(QStringLiteral(" "), 5, 5),
            makeCharacter(QStringLiteral("1"), 6, 6),
            makeCharacter(QStringLiteral("2"), 7, 7),
            makeCharacter(QStringLiteral("3"), 8, 8),
        };

        const QVector<RapidOcrWordSegment> segments =
            buildRapidOcrWordSegments(result, QRectF(10.0, 20.0, 100.0, 30.0));

        QCOMPARE(segments.size(), 2);
        QCOMPARE(segments.at(0).text, QStringLiteral("HELLO"));
        QCOMPARE(segments.at(1).text, QStringLiteral("123"));
        QVERIFY(segments.at(0).box.left() >= 10.0);
        QVERIFY(segments.at(0).box.right() <= segments.at(1).box.left());
        QVERIFY(segments.at(1).box.right() <= 110.0);
    }

    /**
     * 验证中文无空格文本按字符切分。
     * @return 无返回值。
     */
    void splitsChineseTextIntoCharacters()
    {
        RapidRecResult result;
        result.text = QStringLiteral("你好世界");
        result.confidence = 0.9;
        result.stepCount = 4;
        result.characters = {
            makeCharacter(QStringLiteral("你"), 0, 0),
            makeCharacter(QStringLiteral("好"), 1, 1),
            makeCharacter(QStringLiteral("世"), 2, 2),
            makeCharacter(QStringLiteral("界"), 3, 3),
        };

        const QVector<RapidOcrWordSegment> segments =
            buildRapidOcrWordSegments(result, QRectF(0.0, 0.0, 80.0, 20.0));

        QCOMPARE(segments.size(), 4);
        QCOMPARE(segments.at(0).text, QStringLiteral("你"));
        QCOMPARE(segments.at(1).text, QStringLiteral("好"));
        QCOMPARE(segments.at(2).text, QStringLiteral("世"));
        QCOMPARE(segments.at(3).text, QStringLiteral("界"));
        QVERIFY(segments.at(0).box.right() <= segments.at(1).box.left());
        QVERIFY(segments.at(3).box.right() <= 80.0);
    }

    /**
     * 验证中文与 ASCII 混排时按显示列宽分配 token 框。
     * @return 无返回值。
     */
    void usesDisplayColumnsForMixedChineseAndAscii()
    {
        RapidRecResult result;
        result.text = QStringLiteral("中A");
        result.confidence = 0.9;
        result.stepCount = 2;
        result.characters = {
            makeCharacter(QStringLiteral("中"), 0, 0),
            makeCharacter(QStringLiteral("A"), 1, 1),
        };

        const QVector<RapidOcrWordSegment> segments =
            buildRapidOcrWordSegments(result, QRectF(0.0, 0.0, 30.0, 20.0));

        QCOMPARE(segments.size(), 2);
        QCOMPARE(segments.at(0).text, QStringLiteral("中"));
        QCOMPARE(segments.at(1).text, QStringLiteral("A"));
        QCOMPARE(qRound(segments.at(0).box.width()), 20);
        QCOMPARE(qRound(segments.at(1).box.left()), 20);
        QCOMPARE(qRound(segments.at(1).box.width()), 10);
    }

    /**
     * 验证缺少字符跨度时不生成词段。
     * @return 无返回值。
     */
    void returnsEmptyWhenCharactersAreMissing()
    {
        RapidRecResult result;
        result.text = QStringLiteral("HELLO");
        result.confidence = 0.8;
        result.stepCount = 0;

        const QVector<RapidOcrWordSegment> segments =
            buildRapidOcrWordSegments(result, QRectF(0.0, 0.0, 80.0, 20.0));

        QVERIFY(segments.isEmpty());
    }
};

QTEST_MAIN(RapidOcrWordSegmentsTest)
#include "rapid_ocr_word_segments_test.moc"
