#include "marketplace/plugin_index_parser.h"

#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QtTest/QtTest>

class PluginIndexParserTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证插件市场索引 fixture 可以被解析。
     * @return 无返回值。
     */
    void parsesFixture()
    {
        QFile file(QStringLiteral(MARK_SHOT_TEST_SOURCE_DIR "/tests/fixtures/plugin-index-valid.json"));
        QVERIFY2(file.open(QIODevice::ReadOnly), qPrintable(file.errorString()));

        const markshot::marketplace::PluginIndexParseResult result =
            markshot::marketplace::parsePluginIndex(file.readAll());

        QVERIFY2(result.ok(), qPrintable(result.errorMessages().join(QStringLiteral("; "))));
        QCOMPARE(result.index.schemaVersion, 1);
        QCOMPARE(result.index.plugins.size(), 1);
        QCOMPARE(result.index.plugins.first().id, QStringLiteral("mark-shot-sample-ocr"));
        QCOMPARE(result.index.plugins.first().capabilities.first().providerId, QStringLiteral("sample-ocr"));
    }

    /**
     * 验证当前平台资产筛选逻辑。
     * @return 无返回值。
     */
    void filtersCurrentPlatformAssets()
    {
        const QJsonObject root{{QStringLiteral("schemaVersion"), 1},
                               {QStringLiteral("plugins"),
                                QJsonArray{QJsonObject{
                                    {QStringLiteral("id"), QStringLiteral("mark-shot-sample-ocr")},
                                    {QStringLiteral("name"), QStringLiteral("mark-shot-sample-ocr")},
                                    {QStringLiteral("version"), QStringLiteral("0.1.0")},
                                    {QStringLiteral("vendor"), QStringLiteral("example")},
                                    {QStringLiteral("markShotMinVersion"), QStringLiteral("0.1.37")},
                                    {QStringLiteral("capabilities"),
                                     QJsonArray{QJsonObject{{QStringLiteral("type"), QStringLiteral("ocr")},
                                                            {QStringLiteral("providerId"), QStringLiteral("sample-ocr")},
                                                            {QStringLiteral("displayName"), QStringLiteral("Sample OCR")}}}},
                                    {QStringLiteral("assets"),
                                     QJsonArray{QJsonObject{{QStringLiteral("platform"),
                                                            markshot::marketplace::currentPlatformKey()},
                                                           {QStringLiteral("architecture"),
                                                            markshot::marketplace::currentArchitectureKey()},
                                                           {QStringLiteral("fileName"),
                                                            QStringLiteral("mark-shot-sample-ocr.dll")},
                                                           {QStringLiteral("downloadUrl"),
                                                            QStringLiteral("https://github.com/example/mark-shot-sample-ocr/releases/download/v0.1.0/mark-shot-sample-ocr.dll")},
                                                           {QStringLiteral("sha256"),
                                                            QStringLiteral("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")},
                                                           {QStringLiteral("size"), 1024}},
                                                QJsonObject{{QStringLiteral("platform"), QStringLiteral("linux")},
                                                            {QStringLiteral("architecture"), QStringLiteral("riscv64")},
                                                            {QStringLiteral("fileName"),
                                                             QStringLiteral("libmark-shot-sample-ocr.so")},
                                                            {QStringLiteral("downloadUrl"),
                                                             QStringLiteral("https://github.com/example/mark-shot-sample-ocr/releases/download/v0.1.0/libmark-shot-sample-ocr.so")},
                                                            {QStringLiteral("sha256"),
                                                             QStringLiteral("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")},
                                                            {QStringLiteral("size"), 1024}}}}}}}};

        const markshot::marketplace::PluginIndexParseResult result =
            markshot::marketplace::parsePluginIndexDocument(QJsonDocument(root));

        QVERIFY2(result.ok(), qPrintable(result.errorMessages().join(QStringLiteral("; "))));
        const QVector<markshot::marketplace::PluginIndexAsset> assets =
            markshot::marketplace::assetsForCurrentPlatform(result.index.plugins.first());
        QCOMPARE(assets.size(), 1);
        QCOMPARE(assets.first().architecture, markshot::marketplace::currentArchitectureKey());
    }

    /**
     * 验证非法资产字段会返回校验错误。
     * @return 无返回值。
     */
    void rejectsInvalidAsset()
    {
        const QJsonObject root{{QStringLiteral("schemaVersion"), 1},
                               {QStringLiteral("plugins"),
                                QJsonArray{QJsonObject{
                                    {QStringLiteral("id"), QStringLiteral("bad-plugin")},
                                    {QStringLiteral("name"), QStringLiteral("bad-plugin")},
                                    {QStringLiteral("version"), QStringLiteral("0.1.0")},
                                    {QStringLiteral("vendor"), QStringLiteral("example")},
                                    {QStringLiteral("markShotMinVersion"), QStringLiteral("0.1.37")},
                                    {QStringLiteral("capabilities"),
                                     QJsonArray{QJsonObject{{QStringLiteral("type"), QStringLiteral("ocr")},
                                                            {QStringLiteral("providerId"), QStringLiteral("bad-plugin")},
                                                            {QStringLiteral("displayName"), QStringLiteral("Bad Plugin")}}}},
                                    {QStringLiteral("assets"),
                                     QJsonArray{QJsonObject{{QStringLiteral("platform"), QStringLiteral("windows")},
                                                           {QStringLiteral("architecture"), QStringLiteral("x86_64")},
                                                           {QStringLiteral("fileName"), QStringLiteral("../bad.dll")},
                                                           {QStringLiteral("downloadUrl"), QStringLiteral("file:///tmp/bad.dll")},
                                                           {QStringLiteral("sha256"), QStringLiteral("bad")},
                                                           {QStringLiteral("size"), 0}}}}}}}};

        const markshot::marketplace::PluginIndexParseResult result =
            markshot::marketplace::parsePluginIndexDocument(QJsonDocument(root));

        QVERIFY(!result.ok());
        const QString errors = result.errorMessages().join(QStringLiteral("; "));
        QVERIFY(errors.contains(QStringLiteral("fileName")));
        QVERIFY(errors.contains(QStringLiteral("downloadUrl")));
        QVERIFY(errors.contains(QStringLiteral("sha256")));
        QVERIFY(errors.contains(QStringLiteral("size")));
    }
};

QTEST_APPLESS_MAIN(PluginIndexParserTest)

#include "plugin_index_parser_test.moc"
