#include "capture_own_windows_policy.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class CaptureOwnWindowsPolicyTest : public QObject {
    Q_OBJECT

private slots:
    void configRootReadsNestedCaptureValue()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("hideOwnWindows"), false);
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::hideOwnWindowsDuringCaptureFromConfigRoot(root), false);
    }

    void configRootReadsAliases()
    {
        QJsonObject screenshot;
        screenshot.insert(QStringLiteral("hideOwnWindowsDuringCapture"), true);
        QJsonObject root;
        root.insert(QStringLiteral("screenshot"), screenshot);

        QCOMPARE(markshot::hideOwnWindowsDuringCaptureFromConfigRoot(root), true);
    }

    void configRootDefaultsToTrue()
    {
        QCOMPARE(markshot::defaultHideOwnWindowsDuringCapture(), true);
        QCOMPARE(markshot::hideOwnWindowsDuringCaptureFromConfigRoot(QJsonObject()), true);
    }

    void invalidValueDefaultsToTrue()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("hideOwnWindows"), QJsonObject());
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::hideOwnWindowsDuringCaptureFromConfigRoot(root), true);
    }

    /**
     * 验证静态截图在隐藏/保留自身窗口两种策略下都可走 KWin。
     * 保留自身窗口时由 hide-caller-windows=false 表达策略。
     * @return 无返回值。
     */
    void kwinScreenShotIsAllowedForStillCaptureWithEitherOwnWindowPolicy()
    {
        QCOMPARE(markshot::kwinScreenShotSupportsOwnWindowPolicy(false), true);
        QCOMPARE(markshot::kwinScreenShotSupportsOwnWindowPolicy(true), true);
    }

    /**
     * 验证实时流请求绕过一次性 KWin 截图后端。
     * @return 无返回值。
     */
    void kwinScreenShotIsSkippedForReusableScreencast()
    {
        QCOMPARE(markshot::kwinScreenShotSupportsOwnWindowPolicy(true, true), false);
        QCOMPARE(markshot::kwinScreenShotSupportsOwnWindowPolicy(false, true), false);
    }
};

QTEST_MAIN(CaptureOwnWindowsPolicyTest)
#include "capture_own_windows_policy_test.moc"
