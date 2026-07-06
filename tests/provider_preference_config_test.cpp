#include "settings/provider_preference_config.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class ProviderPreferenceConfigTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证默认 provider 偏好为自动模式。
     * @return 无返回值。
     */
    void defaultsToAuto()
    {
        const markshot::settings::ProviderPreferenceConfig config =
            markshot::settings::providerPreferenceConfigFromRoot(QJsonObject());

        QCOMPARE(config.ocrProvider, QStringLiteral("auto"));
        QCOMPARE(config.translationProvider, QStringLiteral("auto"));
        QCOMPARE(config.codeScanProvider, QStringLiteral("auto"));
    }

    /**
     * 验证从标准配置路径读取 provider 偏好。
     * @return 无返回值。
     */
    void readsProviderValues()
    {
        QJsonObject root;
        root.insert(QStringLiteral("ocr"), QJsonObject{{QStringLiteral("provider"), QStringLiteral("plugin:rapid-onnx")}});
        root.insert(QStringLiteral("translation"),
                    QJsonObject{{QStringLiteral("provider"), QStringLiteral("plugin:openai-compatible")}});
        root.insert(QStringLiteral("codeScan"), QJsonObject{{QStringLiteral("provider"), QStringLiteral("plugin:zxing-cpp")}});

        const markshot::settings::ProviderPreferenceConfig config =
            markshot::settings::providerPreferenceConfigFromRoot(root);

        QCOMPARE(config.ocrProvider, QStringLiteral("plugin:rapid-onnx"));
        QCOMPARE(config.translationProvider, QStringLiteral("plugin:openai-compatible"));
        QCOMPARE(config.codeScanProvider, QStringLiteral("plugin:zxing-cpp"));
    }

    /**
     * 验证扫码 provider 读取兼容旧别名对象。
     * @return 无返回值。
     */
    void readsCodeScanAliases()
    {
        QJsonObject root;
        root.insert(QStringLiteral("barcodeScanner"), QJsonObject{{QStringLiteral("provider"), QStringLiteral("builtin")}});

        const markshot::settings::ProviderPreferenceConfig config =
            markshot::settings::providerPreferenceConfigFromRoot(root);

        QCOMPARE(config.codeScanProvider, QStringLiteral("builtin"));
    }

    /**
     * 验证 provider 偏好会写回标准配置路径。
     * @return 无返回值。
     */
    void writesProviderValues()
    {
        QJsonObject root;
        markshot::settings::writeProviderPreferenceConfig(
            &root,
            markshot::settings::ProviderPreferenceConfig{QStringLiteral("plugin:rapid-onnx"),
                                                         QStringLiteral("plugin:openai-compatible"),
                                                         QStringLiteral("plugin:zxing-cpp")});

        QCOMPARE(root.value(QStringLiteral("ocr")).toObject().value(QStringLiteral("provider")).toString(),
                 QStringLiteral("plugin:rapid-onnx"));
        QCOMPARE(root.value(QStringLiteral("translation")).toObject().value(QStringLiteral("provider")).toString(),
                 QStringLiteral("plugin:openai-compatible"));
        QCOMPARE(root.value(QStringLiteral("codeScan")).toObject().value(QStringLiteral("provider")).toString(),
                 QStringLiteral("plugin:zxing-cpp"));
    }
};

QTEST_APPLESS_MAIN(ProviderPreferenceConfigTest)

#include "provider_preference_config_test.moc"
