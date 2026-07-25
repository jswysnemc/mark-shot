#include "debug_log.h"

#include <QtTest/QtTest>

class DebugLogTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 清理每个测试修改的调试环境变量。
     * @return 无返回值。
     */
    void cleanup()
    {
        qunsetenv("MARK_SHOT_DEBUG");
        qunsetenv("DEBUG");
    }

    /**
     * 验证 MARK_SHOT_DEBUG 可以启用调试日志。
     * @return 无返回值。
     */
    void markShotDebugEnablesLogging()
    {
        qputenv("MARK_SHOT_DEBUG", "1");
        qputenv("DEBUG", "0");

        QVERIFY(markshot::debugEnabled());
    }

    /**
     * 验证 MARK_SHOT_DEBUG 明确关闭时优先于兼容变量。
     * @return 无返回值。
     */
    void markShotDebugOverridesLegacyVariable()
    {
        qputenv("MARK_SHOT_DEBUG", "0");
        qputenv("DEBUG", "1");

        QVERIFY(!markshot::debugEnabled());
    }

    /**
     * 验证原有 DEBUG 环境变量继续生效。
     * @return 无返回值。
     */
    void legacyDebugVariableRemainsSupported()
    {
        qunsetenv("MARK_SHOT_DEBUG");
        qputenv("DEBUG", "1");

        QVERIFY(markshot::debugEnabled());
    }
};

QTEST_APPLESS_MAIN(DebugLogTest)

#include "debug_log_test.moc"
