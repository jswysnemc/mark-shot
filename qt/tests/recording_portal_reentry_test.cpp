#include "recording/recording_display_source.h"
#include "recording/recording_start_gate.h"
#include "screen_capture_portal_guard.h"

#include <QtTest/QtTest>

class RecordingPortalReentryTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 全部显示器上的单屏区域应改绑到包含它的输出，不再走 all-outputs。
     * @return 无返回值。
     */
    void allDisplaysRegionBindsToContainingOutput()
    {
        const QVector<markshot::recording::RecordingScreenCandidate> screens = {
            {QStringLiteral("DP-1"), QRect(0, 0, 2560, 1440)},
            {QStringLiteral("HDMI-A-1"), QRect(2560, 0, 2560, 1440)},
        };

        markshot::recording::RecordingOptions options;
        options.scope = markshot::recording::RecordingScope::Region;
        options.display.allOutputs = true;
        options.display.outputName = QStringLiteral("all-displays");
        options.captureGeometry = QRect(3167, 239, 791, 742);

        markshot::recording::bindRegionRecordingDisplay(&options, screens);

        QVERIFY(!options.display.allOutputs);
        QCOMPARE(options.display.outputName, QStringLiteral("HDMI-A-1"));
        QCOMPARE(options.display.screenName, QStringLiteral("HDMI-A-1"));
        QCOMPARE(options.display.geometry, QRect(2560, 0, 2560, 1440));
    }

    /**
     * 【录制测试】【输出绑定】对话框默认选中 DP-1，但区域框在 HDMI-A-1 时必须改绑到 HDMI-A-1（issue #139）。
     * @return 无返回值。
     */
    void explicitDisplayRegionBindsToContainingOutput()
    {
        const QVector<markshot::recording::RecordingScreenCandidate> screens = {
            {QStringLiteral("DP-1"), QRect(0, 0, 2560, 1440)},
            {QStringLiteral("HDMI-A-1"), QRect(2560, 0, 2560, 1440)},
        };

        // 1. 【录制测试】【输出绑定】单屏区域落在另一块屏幕，输出名和几何都以区域所在屏幕为准
        markshot::recording::RecordingOptions options;
        options.scope = markshot::recording::RecordingScope::Region;
        options.display.screenName = QStringLiteral("DP-1");
        options.display.outputName = QStringLiteral("DP-1");
        options.display.geometry = QRect(0, 0, 2560, 1440);
        options.captureGeometry = QRect(3167, 239, 791, 742);
        markshot::recording::bindRegionRecordingDisplay(&options, screens);
        QVERIFY(!options.display.allOutputs);
        QCOMPARE(options.display.outputName, QStringLiteral("HDMI-A-1"));
        QCOMPARE(options.display.screenName, QStringLiteral("HDMI-A-1"));
        QCOMPARE(options.display.geometry, QRect(2560, 0, 2560, 1440));

        // 2. 【录制测试】【输出绑定】跨屏区域无法归属单屏，保留对话框选择
        markshot::recording::RecordingOptions spanning;
        spanning.scope = markshot::recording::RecordingScope::Region;
        spanning.display.screenName = QStringLiteral("DP-1");
        spanning.display.outputName = QStringLiteral("DP-1");
        spanning.display.geometry = QRect(0, 0, 2560, 1440);
        spanning.captureGeometry = QRect(2000, 100, 1200, 400);
        markshot::recording::bindRegionRecordingDisplay(&spanning, screens);
        QCOMPARE(spanning.display.outputName, QStringLiteral("DP-1"));
        QCOMPARE(spanning.display.geometry, QRect(0, 0, 2560, 1440));
    }

    /**
     * 跨屏区域和整屏全部显示器保持原样，不能假装成单输出。
     * @return 无返回值。
     */
    void spanningRegionAndVirtualDesktopStayUnbound()
    {
        const QVector<markshot::recording::RecordingScreenCandidate> screens = {
            {QStringLiteral("DP-1"), QRect(0, 0, 2560, 1440)},
            {QStringLiteral("HDMI-A-1"), QRect(2560, 0, 2560, 1440)},
        };

        markshot::recording::RecordingOptions spanning;
        spanning.scope = markshot::recording::RecordingScope::Region;
        spanning.display.allOutputs = true;
        spanning.display.outputName = QStringLiteral("all-displays");
        spanning.captureGeometry = QRect(2000, 100, 1200, 400);
        markshot::recording::bindRegionRecordingDisplay(&spanning, screens);
        QVERIFY(spanning.display.allOutputs);
        QCOMPARE(spanning.display.outputName, QStringLiteral("all-displays"));

        markshot::recording::RecordingOptions desktop;
        desktop.scope = markshot::recording::RecordingScope::Display;
        desktop.display.allOutputs = true;
        desktop.display.outputName = QStringLiteral("all-displays");
        desktop.captureGeometry = QRect(0, 0, 5120, 1440);
        markshot::recording::bindRegionRecordingDisplay(&desktop, screens);
        QVERIFY(desktop.display.allOutputs);
        QCOMPARE(desktop.display.outputName, QStringLiteral("all-displays"));
    }

    /**
     * 启动占位在释放前拒绝第二次进入，对应门户事件循环里的重入 start()。
     * @return 无返回值。
     */
    void startGateRejectsReentrantEnter()
    {
        markshot::recording::RecordingStartGate gate;
        QString error;
        QVERIFY(gate.tryEnter(&error));
        QVERIFY(error.isEmpty());
        QVERIFY(gate.active());

        QString nestedError;
        QVERIFY(!gate.tryEnter(&nestedError));
        QCOMPARE(nestedError, QStringLiteral("recording is already active"));
        QVERIFY(gate.active());

        gate.leave();
        QVERIFY(!gate.active());
        nestedError = QStringLiteral("stale");
        QVERIFY(gate.tryEnter(&nestedError));
        QVERIFY(nestedError.isEmpty());
        gate.leave();
    }

    /**
     * 已有 ScreenCast 选择请求时，第二次占用必须失败且不能覆盖第一次。
     * @return 无返回值。
     */
    void interactiveScreenCastRejectsSecondRequest()
    {
        QString error;
        QVERIFY(markshot::tryAcquireInteractiveScreenCast(&error));
        QVERIFY(markshot::interactiveScreenCastInProgress());

        QString nestedError;
        QVERIFY(!markshot::tryAcquireInteractiveScreenCast(&nestedError));
        QCOMPARE(nestedError, QStringLiteral("ScreenCast portal request is already in progress"));
        QVERIFY(markshot::interactiveScreenCastInProgress());

        markshot::releaseInteractiveScreenCast();
        QVERIFY(!markshot::interactiveScreenCastInProgress());
        nestedError = QStringLiteral("stale");
        QVERIFY(markshot::tryAcquireInteractiveScreenCast(&nestedError));
        QVERIFY(nestedError.isEmpty());
        markshot::releaseInteractiveScreenCast();
        QVERIFY(!markshot::interactiveScreenCastInProgress());
    }
};

QTEST_GUILESS_MAIN(RecordingPortalReentryTest)

#include "recording_portal_reentry_test.moc"
