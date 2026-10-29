#include "marketplace/plugin_index_parser.h"
#include "marketplace/plugin_qt_compat.h"

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
     * 验证索引资产使用 arm64、amd64 等架构别名时仍能匹配当前平台。
     * @return 无返回值。
     */
    void matchesArchitectureAliases()
    {
        QCOMPARE(markshot::marketplace::normalizeArchitectureKey(QStringLiteral("arm64")), QStringLiteral("aarch64"));
        QCOMPARE(markshot::marketplace::normalizeArchitectureKey(QStringLiteral(" AArch64 ")), QStringLiteral("aarch64"));
        QCOMPARE(markshot::marketplace::normalizeArchitectureKey(QStringLiteral("AMD64")), QStringLiteral("x86_64"));
        QCOMPARE(markshot::marketplace::normalizeArchitectureKey(QStringLiteral("riscv64")), QStringLiteral("riscv64"));

        // 1. 用当前架构的别名写资产，模拟发布流程按文件名写入 arm64 的索引
        const QString current = markshot::marketplace::currentArchitectureKey();
        QString alias;
        if (current == QStringLiteral("aarch64")) {
            alias = QStringLiteral("arm64");
        } else if (current == QStringLiteral("x86_64")) {
            alias = QStringLiteral("amd64");
        } else {
            QSKIP("当前架构没有别名");
        }
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
                                                           {QStringLiteral("architecture"), alias},
                                                           {QStringLiteral("fileName"), QStringLiteral("alias.so")},
                                                           {QStringLiteral("downloadUrl"),
                                                            QStringLiteral("https://github.com/example/sample/releases/download/v0.1.0/alias.so")},
                                                           {QStringLiteral("sha256"),
                                                            QStringLiteral("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")},
                                                           {QStringLiteral("size"), 1024}}}}}}}};

        // 2. 别名资产必须出现在当前平台的可安装列表中
        const markshot::marketplace::PluginIndexParseResult result =
            markshot::marketplace::parsePluginIndexDocument(QJsonDocument(root));
        QVERIFY2(result.ok(), qPrintable(result.errorMessages().join(QStringLiteral("; "))));
        const QVector<markshot::marketplace::PluginIndexAsset> assets =
            markshot::marketplace::assetsForCurrentPlatform(result.index.plugins.first());
        QCOMPARE(assets.size(), 1);
        QCOMPARE(assets.first().fileName, QStringLiteral("alias.so"));
    }

    /**
     * 验证资产声明的构建 Qt 版本高于运行时版本时不再提供安装。
     * @return 无返回值。
     */
    void filtersAssetsBuiltWithNewerQt()
    {
        const auto asset = [](const QString &fileName, const QString &qtVersion) {
            QJsonObject object{{QStringLiteral("platform"), markshot::marketplace::currentPlatformKey()},
                               {QStringLiteral("architecture"), markshot::marketplace::currentArchitectureKey()},
                               {QStringLiteral("fileName"), fileName},
                               {QStringLiteral("downloadUrl"),
                                QStringLiteral("https://github.com/example/sample/releases/download/v0.1.0/") + fileName},
                               {QStringLiteral("sha256"),
                                QStringLiteral("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")},
                               {QStringLiteral("size"), 1024}};
            if (!qtVersion.isEmpty()) {
                object.insert(QStringLiteral("qtVersion"), qtVersion);
            }
            return object;
        };
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
                                     QJsonArray{asset(QStringLiteral("newer.so"), QStringLiteral("6.12.0")),
                                                asset(QStringLiteral("older.so"), QStringLiteral("6.8.3")),
                                                asset(QStringLiteral("unversioned.so"), QString())}}}}}};

        const markshot::marketplace::PluginIndexParseResult result =
            markshot::marketplace::parsePluginIndexDocument(QJsonDocument(root));
        QVERIFY2(result.ok(), qPrintable(result.errorMessages().join(QStringLiteral("; "))));

        // 1. Fedora 44 的 Qt 6.11 不能加载 Qt 6.12 构建的插件，修订号不影响判断
        const QVector<markshot::marketplace::PluginIndexAsset> fedoraAssets =
            markshot::marketplace::assetsForCurrentPlatform(result.index.plugins.first(), QStringLiteral("6.11.2"));
        QCOMPARE(fedoraAssets.size(), 2);
        QCOMPARE(fedoraAssets.at(0).fileName, QStringLiteral("older.so"));
        QCOMPARE(fedoraAssets.at(1).fileName, QStringLiteral("unversioned.so"));

        // 2. 相同次版本、修订号更低的运行时仍可加载
        QCOMPARE(markshot::marketplace::assetsForCurrentPlatform(result.index.plugins.first(),
                                                                  QStringLiteral("6.12.0"))
                     .size(),
                 3);
        QVERIFY(markshot::marketplace::pluginQtVersionCompatible(QStringLiteral("6.12.3"), QStringLiteral("6.12.0")));
        QVERIFY(!markshot::marketplace::pluginQtVersionCompatible(QStringLiteral("6.2.4"), QStringLiteral("7.0.0")));
        QVERIFY(!markshot::marketplace::pluginQtVersionCompatible(QStringLiteral("bad"), QStringLiteral("6.12.0")));
    }

    /**
     * 验证非法的 qtVersion 字段会返回校验错误。
     * @return 无返回值。
     */
    void rejectsInvalidQtVersion()
    {
        QVERIFY(markshot::marketplace::isValidPluginQtVersion(QStringLiteral("6")));
        QVERIFY(markshot::marketplace::isValidPluginQtVersion(QStringLiteral("6.12.0")));
        QVERIFY(!markshot::marketplace::isValidPluginQtVersion(QStringLiteral("6.x")));
        QVERIFY(!markshot::marketplace::isValidPluginQtVersion(QStringLiteral("6.1.2.3")));
        QVERIFY(!markshot::marketplace::isValidPluginQtVersion(QString()));
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
