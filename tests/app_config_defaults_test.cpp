#include "app_config_defaults.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class AppConfigDefaultsTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证新配置默认关闭 macOS 风格导出边框。
     * @return 无返回值。
     */
    void imageFrameIsDisabledByDefault()
    {
        const QJsonObject root = markshot::defaultAppConfigRoot(QStringLiteral("detector"));
        const QJsonObject imageFrame = root.value(QStringLiteral("export"))
                                           .toObject()
                                           .value(QStringLiteral("imageFrame"))
                                           .toObject();

        QCOMPARE(imageFrame.value(QStringLiteral("enabled")).toBool(true), false);
        QCOMPARE(root.value(QStringLiteral("windowDetection"))
                     .toObject()
                     .value(QStringLiteral("command"))
                     .toString(),
                 QStringLiteral("detector"));
    }
};

QTEST_APPLESS_MAIN(AppConfigDefaultsTest)

#include "app_config_defaults_test.moc"
