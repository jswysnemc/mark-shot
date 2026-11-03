#include "rapid_rec_decoder.h"

#include <QtTest>

#include <limits>

using namespace markshot::ocr_rapid;

class RapidRecDecoderTest final : public QObject {
    Q_OBJECT
private slots:
    /** 【OCR测试】【字符契约】重复、空白、尾部空格和字符跨度保持原解码语义，无参数和返回值。 */
    void preservesCtcRunsAndConfidence()
    {
        const QVector<int> labels{0, 1, 1, 0, 1, 2, 2, 3, 3, 0};
        const QVector<float> scores{1.0f, .8f, .6f, .9f, .75f, .5f, .9f, .6f, .8f, 1.0f};
        QVector<float> output(labels.size() * 4, 0.0f);
        for (qsizetype step = 0; step < labels.size(); ++step) output[step * 4 + labels[step]] = scores[step];
        RapidRecResult result;
        QString error;
        QVERIFY(decodeRapidRecognition(output.constData(), output.size(), {{1, 10, 4}},
                                       {QStringLiteral("A"), QStringLiteral("中")}, &result, &error));
        QVERIFY(error.isEmpty());
        QCOMPARE(result.text, QStringLiteral("AA中"));
        QCOMPARE(result.stepCount, 10);
        QCOMPARE(result.characters.size(), 4);
        QCOMPARE(result.characters[0].startStep, 1);
        QCOMPARE(result.characters[0].endStep, 2);
        QCOMPARE(result.characters[1].startStep, 4);
        QCOMPARE(result.characters[2].endStep, 6);
        QCOMPARE(result.characters[3].text, QStringLiteral(" "));
        const double a = (double(.8f) + double(.6f)) / 2;
        const double c = (double(.5f) + double(.9f)) / 2;
        QCOMPARE(result.confidence, (a + double(.75f) + c + a) / 4);
        output.fill(-1.0f);
        QCOMPARE(result.text, QStringLiteral("AA中"));
        QCOMPARE(result.characters[0].confidence, a);
    }

    /** 【OCR测试】【概率边界】相等概率保留最早类别，NaN 比较和空字典兼容原路径，无参数和返回值。 */
    void preservesTieAndBlankRules()
    {
        const float output[]{std::numeric_limits<float>::quiet_NaN(), .8f, .2f, 0, .5f, .5f};
        RapidRecResult result;
        QVERIFY(decodeRapidRecognition(output, 6, {{1, 2, 3}}, {QStringLiteral("A"), QStringLiteral("B")},
                                       &result, nullptr));
        QCOMPARE(result.text, QStringLiteral("A"));
        QCOMPARE(result.characters[0].startStep, 1);
        QVERIFY(decodeRapidRecognition(output, 6, {{1, 2, 3}}, {}, &result, nullptr));
        QVERIFY(result.text.isEmpty());
        QCOMPARE(result.characters.size(), 1);
    }

    /** 【OCR测试】【输入边界】拒绝越界形状、空数据和缺失输出，失败后清除旧结果，无参数和返回值。 */
    void rejectsInvalidShapes()
    {
        const float output[]{0, 1, 0, 1};
        const std::vector<TensorShape> shapes{
            {{}}, {{1, 4}}, {{2, 2, 2}}, {{1, 0, 2}}, {{1, -1, 2}}, {{1, 2, 1}},
            {{1, 3, 2}}, {{1, std::numeric_limits<int>::max(), std::numeric_limits<int>::max()}},
            {{1, std::numeric_limits<std::int64_t>::max(), 2}}};
        for (const auto &shape : shapes) {
            RapidRecResult result;
            result.text = QStringLiteral("stale");
            QString error;
            QVERIFY(!decodeRapidRecognition(output, 4, shape, {}, &result, &error));
            QVERIFY(!error.isEmpty());
            QVERIFY(result.text.isEmpty());
        }
        RapidRecResult result;
        QVERIFY(!decodeRapidRecognition(nullptr, 4, {{1, 2, 2}}, {}, &result, nullptr));
        QVERIFY(!decodeRapidRecognition(output, -1, {{1, 2, 2}}, {}, &result, nullptr));
        QVERIFY(!decodeRapidRecognition(output, 4, {{1, 2, 2}}, {}, nullptr, nullptr));
    }
};

QTEST_GUILESS_MAIN(RapidRecDecoderTest)
#include "rapid_rec_decoder_test.moc"
