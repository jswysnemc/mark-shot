#include "providers/translate/translate_segments.h"

#include <QtTest/QtTest>

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

using namespace markshot::providers;

namespace {

/**
 * 构建翻译输入 JSON。
 * @param targetLanguage 目标语言。
 * @return 两行文本的输入 JSON。
 */
QByteArray sampleInputJson(const QString &targetLanguage)
{
    QJsonArray tokens;
    tokens.append(QJsonObject{{"text", "Hello"},
                              {"box", QJsonArray{0, 0, 50, 20}},
                              {"line", 0},
                              {"index", 0},
                              {"confidence", 0.9}});
    tokens.append(QJsonObject{{"text", "world"},
                              {"box", QJsonArray{60, 0, 50, 20}},
                              {"line", 0},
                              {"index", 1},
                              {"confidence", 0.9}});
    tokens.append(QJsonObject{{"text", "第二行"},
                              {"box", QJsonArray{0, 30, 80, 20}},
                              {"line", 1},
                              {"index", 0},
                              {"confidence", 0.9}});
    QJsonObject root;
    root.insert(QStringLiteral("targetLanguage"), targetLanguage);
    root.insert(QStringLiteral("tokens"), tokens);
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

}  // namespace

class TranslateSegmentsTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证输入 JSON 按行合并为分段并解析目标语言。
     * @return 无返回值。
     */
    void buildsSegmentsFromInput()
    {
        QString targetLanguage;
        const QVector<TranslateSourceSegment> segments =
            translateSegmentsFromInputJson(sampleInputJson(QStringLiteral("English")), &targetLanguage);

        QCOMPARE(targetLanguage, QStringLiteral("English"));
        QCOMPARE(segments.size(), 2);
        QCOMPARE(segments.at(0).text, QStringLiteral("Hello world"));
        QCOMPARE(segments.at(1).text, QStringLiteral("第二行"));
        // 行内 box 取包围盒
        QCOMPARE(segments.at(0).box, QRectF(0, 0, 110, 20));
    }

    /**
     * 验证译文输出 JSON 契约与缺失译文回退原文。
     * @return 无返回值。
     */
    void buildsOutputTokensJson()
    {
        QString targetLanguage;
        const QVector<TranslateSourceSegment> segments =
            translateSegmentsFromInputJson(sampleInputJson(QStringLiteral("English")), &targetLanguage);

        QHash<int, QString> translations;
        translations.insert(segments.at(0).id, QStringLiteral("你好 世界"));
        const QByteArray output =
            translateTokensJson(segments, translations, QStringLiteral("test-backend"));

        const QJsonObject root = QJsonDocument::fromJson(output).object();
        QCOMPARE(root.value(QStringLiteral("backend")).toString(), QStringLiteral("test-backend"));
        const QJsonArray tokens = root.value(QStringLiteral("tokens")).toArray();
        QCOMPARE(tokens.size(), 2);
        QCOMPARE(tokens.at(0).toObject().value(QStringLiteral("text")).toString(),
                 QStringLiteral("你好 世界"));
        // 第二段无译文时保留原文
        QCOMPARE(tokens.at(1).toObject().value(QStringLiteral("text")).toString(),
                 QStringLiteral("第二行"));
    }
};

QTEST_APPLESS_MAIN(TranslateSegmentsTest)
#include "translate_segments_test.moc"
