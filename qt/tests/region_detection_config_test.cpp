#include "region_detection/region_detection_config.h"

#include <QJsonArray>
#include <QtTest>

class RegionDetectionConfigTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【截图测试】【区域识别】验证旧配置默认启用区域分析，无参数和返回值。 */
    void defaultsToEnabled()
    {
        using markshot::region_detection::regionDetectionEnabledFromConfigRoot;
        QVERIFY(regionDetectionEnabledFromConfigRoot({}));
        QVERIFY(regionDetectionEnabledFromConfigRoot({
            {QStringLiteral("windowDetection"), QJsonObject{{QStringLiteral("enabled"), false}}}}));
    }

    /** 【截图测试】【区域识别】提供有效开关和无效配置数据，无参数和返回值。 */
    void readsEnabled_data()
    {
        QTest::addColumn<QJsonValue>("value");
        QTest::addColumn<bool>("expected");
        QTest::newRow("enabled") << QJsonValue(true) << true;
        QTest::newRow("disabled") << QJsonValue(false) << false;
        QTest::newRow("null") << QJsonValue(QJsonValue::Null) << true;
        QTest::newRow("undefined") << QJsonValue(QJsonValue::Undefined) << true;
        QTest::newRow("invalid-string") << QJsonValue(QStringLiteral("invalid")) << true;
        QTest::newRow("object") << QJsonValue(QJsonObject{}) << true;
        QTest::newRow("array") << QJsonValue(QJsonArray{}) << true;
    }

    /** 【截图测试】【区域识别】验证嵌套开关解析及无效值回退，无参数和返回值。 */
    void readsEnabled()
    {
        QFETCH(QJsonValue, value);
        QFETCH(bool, expected);
        const QJsonObject root{{QStringLiteral("capture"), QJsonObject{
            {QStringLiteral("regionDetection"), QJsonObject{{QStringLiteral("enabled"), value}}}}}};
        QCOMPARE(markshot::region_detection::regionDetectionEnabledFromConfigRoot(root), expected);
    }
};

QTEST_APPLESS_MAIN(RegionDetectionConfigTest)
#include "region_detection_config_test.moc"
