#include "pinned_window/pinned_layer_shell_screen_binding.h"

#include <QtTest/QtTest>

class PinnedLayerShellScreenBindingTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证贴图主体进入另一屏幕后要求重建 surface。
     * @return 无返回值。
     */
    void detectsTargetScreenChange()
    {
        const QVector<QRect> screens = {
            QRect(0, 0, 1920, 1080),
            QRect(1920, 0, 2560, 1440),
        };

        const markshot::shot::PinnedLayerShellScreenBinding binding =
            markshot::shot::resolvePinnedLayerShellScreenBinding(
                QRect(2100, 100, 600, 400), screens, 0);

        QCOMPARE(binding.targetScreenIndex, 1);
        QVERIFY(binding.screenChanged);
    }

    /**
     * 验证贴图仍以原屏幕为主要可见区域时不重建 surface。
     * @return 无返回值。
     */
    void keepsCurrentScreenWhileItHasLargestIntersection()
    {
        const QVector<QRect> screens = {
            QRect(0, 0, 1920, 1080),
            QRect(1920, 0, 2560, 1440),
        };

        const markshot::shot::PinnedLayerShellScreenBinding binding =
            markshot::shot::resolvePinnedLayerShellScreenBinding(
                QRect(1700, 100, 400, 300), screens, 0);

        QCOMPARE(binding.targetScreenIndex, 0);
        QVERIFY(!binding.screenChanged);
    }
};

QTEST_APPLESS_MAIN(PinnedLayerShellScreenBindingTest)

#include "pinned_layer_shell_screen_binding_test.moc"
