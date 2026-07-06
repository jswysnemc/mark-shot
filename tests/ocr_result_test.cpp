#include "ocr_result.h"

#include <QtTest/QtTest>

/// @brief Test suite for validating OCR result parsing and token sorting.
class OcrResultTest : public QObject
{
    Q_OBJECT

private slots:
    void parsesArrayAndSortsTokens()
    {
        const QByteArray output = R"([
            {"text":"world","box":[60,0,40,20],"line":0,"index":1,"confidence":0.8},
            {"text":"Hello","box":[0,0,40,20],"line":0,"index":0,"confidence":0.9},
            {"text":"next","box":[0,30,40,20],"line":1,"index":0}
        ])";

        const QVector<markshot::ocr::Token> tokens = markshot::ocr::tokensFromJsonOutput(output);

        QCOMPARE(tokens.size(), 3);
        QCOMPARE(tokens.at(0).text, QStringLiteral("Hello"));
        QCOMPARE(tokens.at(1).text, QStringLiteral("world"));
        QCOMPARE(tokens.at(2).text, QStringLiteral("next"));
        QCOMPARE(markshot::ocr::tokensText(tokens), QStringLiteral("Hello world\nnext"));
    }

    void parsesObjectOutputAndPointBounds()
    {
        const QByteArray output = R"({
            "tokens": [
                {"text":"inside","points":[[5,5],[25,5],[25,15],[5,15]],"line":0,"index":0},
                {"text":"outside","box":[100,100,20,20],"line":0,"index":1}
            ]
        })";

        const QVector<markshot::ocr::Token> tokens = markshot::ocr::tokensFromJsonOutput(output, QSize(40, 40));

        QCOMPARE(tokens.size(), 1);
        QCOMPARE(tokens.first().text, QStringLiteral("inside"));
        QCOMPARE(tokens.first().imageRect, QRectF(5, 5, 20, 10));
    }

    void avoidsSpacesBeforePunctuation()
    {
        QVector<markshot::ocr::Token> tokens;
        tokens.append({QStringLiteral("Hello"), QRectF(0, 0, 40, 20), 0, 0, 1.0});
        tokens.append({QStringLiteral(","), QRectF(45, 0, 5, 20), 0, 1, 1.0});
        tokens.append({QStringLiteral("world"), QRectF(64, 0, 40, 20), 0, 2, 1.0});

        QCOMPARE(markshot::ocr::tokensText(tokens), QStringLiteral("Hello, world"));
    }

    void avoidsSpacesBetweenChineseTokens()
    {
        QVector<markshot::ocr::Token> tokens;
        tokens.append({QStringLiteral("源"), QRectF(0, 0, 10, 20), 0, 0, 1.0});
        tokens.append({QStringLiteral("文"), QRectF(18, 0, 10, 20), 0, 1, 1.0});
        tokens.append({QStringLiteral("件"), QRectF(36, 0, 10, 20), 0, 2, 1.0});

        QCOMPARE(markshot::ocr::tokensText(tokens), QStringLiteral("源文件"));
    }

    void avoidsSpacesAfterChinesePunctuation()
    {
        QVector<markshot::ocr::Token> tokens;
        tokens.append({QStringLiteral("完成"), QRectF(0, 0, 20, 20), 0, 0, 1.0});
        tokens.append({QStringLiteral("，"), QRectF(30, 0, 10, 20), 0, 1, 1.0});
        tokens.append({QStringLiteral("源"), QRectF(48, 0, 10, 20), 0, 2, 1.0});

        QCOMPARE(markshot::ocr::tokensText(tokens), QStringLiteral("完成，源"));
    }

    void keepsSpacesBetweenChineseAndLatinWhenGapIsLarge()
    {
        QVector<markshot::ocr::Token> tokens;
        tokens.append({QStringLiteral("文件"), QRectF(0, 0, 24, 20), 0, 0, 1.0});
        tokens.append({QStringLiteral("SHA-256"), QRectF(42, 0, 60, 20), 0, 1, 1.0});
        tokens.append({QStringLiteral("一致"), QRectF(120, 0, 24, 20), 0, 2, 1.0});

        QCOMPARE(markshot::ocr::tokensText(tokens), QStringLiteral("文件 SHA-256 一致"));
    }

    void reportsInvalidJson()
    {
        const markshot::ocr::ParsedOutput parsed = markshot::ocr::parseOutput(QByteArray("{"));

        QVERIFY(!parsed.validJson);
        QVERIFY(parsed.tokens.isEmpty());
    }
};

/// @brief Main entry point for the OCR result test suite.
QTEST_APPLESS_MAIN(OcrResultTest)

#include "ocr_result_test.moc"
