#include "startup_config.h"

#include <QtTest/QtTest>

class StartupDefaultColorTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证内置默认颜色不会覆盖状态文件。
     * @return 无返回值。
     */
    void builtInColorDoesNotOverride()
    {
        QCOMPARE(markshot::shouldApplyDefaultColor(markshot::DefaultColorSource::BuiltIn, false), false);
        QCOMPARE(markshot::shouldApplyDefaultColor(markshot::DefaultColorSource::BuiltIn, true), false);
    }

    /**
     * 验证配置颜色只在没有状态文件时作为初始值生效。
     * @return 无返回值。
     */
    void configColorOnlySeedsMissingState()
    {
        QCOMPARE(markshot::shouldApplyDefaultColor(markshot::DefaultColorSource::Config, false), true);
        QCOMPARE(markshot::shouldApplyDefaultColor(markshot::DefaultColorSource::Config, true), false);
    }

    /**
     * 验证命令行颜色始终覆盖状态文件。
     * @return 无返回值。
     */
    void commandLineColorAlwaysOverrides()
    {
        QCOMPARE(markshot::shouldApplyDefaultColor(markshot::DefaultColorSource::CommandLine, false), true);
        QCOMPARE(markshot::shouldApplyDefaultColor(markshot::DefaultColorSource::CommandLine, true), true);
    }
};

QTEST_APPLESS_MAIN(StartupDefaultColorTest)

#include "startup_default_color_test.moc"
