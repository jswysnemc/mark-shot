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
     * 验证保留自身窗口时跳过无法表达该策略的 KWin 截图后端。
     * @return 无返回值。
     */
    void kwinScreenShotIsSkippedWhenOwnWindowsMustRemainVisible()
    {
        QCOMPARE(markshot::kwinScreenShotSupportsOwnWindowPolicy(false), false);
        QCOMPARE(markshot::kwinScreenShotSupportsOwnWindowPolicy(true), true);
    }
};

QTEST_MAIN(CaptureOwnWindowsPolicyTest)
#include "capture_own_windows_policy_test.moc"
