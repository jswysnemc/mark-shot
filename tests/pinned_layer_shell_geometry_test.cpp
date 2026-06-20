#include "pinned_window/pinned_layer_shell_geometry.h"

#include <QtTest/QtTest>

class PinnedLayerShellGeometryTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证左上越界时保留负边距。
     * @return 无返回值。
     */
    void marginsPreserveNegativeOffsets()
    {
        const QMargins margins =
            markshot::shot::pinnedLayerShellMargins(QRect(-40, -20, 240, 120), QRect(0, 0, 1920, 1080));

        QCOMPARE(margins.left(), -40);
        QCOMPARE(margins.top(), -20);
        QCOMPARE(margins.right(), 0);
        QCOMPARE(margins.bottom(), 0);
    }

    /**
     * 验证左上越界时 surface 保持在屏幕内并使用内容偏移裁剪。
     * @return 无返回值。
     */
    void placementClipsLeftAndTopEdges()
    {
        const markshot::shot::PinnedLayerShellPlacement placement =
            markshot::shot::pinnedLayerShellPlacement(QRect(-40, -20, 240, 120),
                                                      QRect(0, 0, 1920, 1080),
                                                      QSize(24, 24));

        QCOMPARE(placement.logicalGeometry, QRect(-40, -20, 240, 120));
        QCOMPARE(placement.visibleGeometry, QRect(0, 0, 200, 100));
        QCOMPARE(placement.contentOffset, QPoint(-40, -20));
        QCOMPARE(placement.margins, QMargins(0, 0, 0, 0));
        QCOMPARE(placement.desiredSize, QSize(200, 100));
    }

    /**
     * 验证右下越界时 surface 只保留屏幕内区域,避免 compositor 压缩整图。
     * @return 无返回值。
     */
    void placementClipsRightAndBottomEdges()
    {
        const markshot::shot::PinnedLayerShellPlacement placement =
            markshot::shot::pinnedLayerShellPlacement(QRect(1800, 1000, 240, 120),
                                                      QRect(0, 0, 1920, 1080),
                                                      QSize(24, 24));

        QCOMPARE(placement.logicalGeometry, QRect(1800, 1000, 240, 120));
        QCOMPARE(placement.visibleGeometry, QRect(1800, 1000, 120, 80));
        QCOMPARE(placement.contentOffset, QPoint(0, 0));
        QCOMPARE(placement.margins, QMargins(1800, 1000, 0, 0));
        QCOMPARE(placement.desiredSize, QSize(120, 80));
    }

    /**
     * 验证拖拽过远时至少保留指定尺寸在屏幕内。
     * @return 无返回值。
     */
    void placementKeepsMinimumVisibleArea()
    {
        const markshot::shot::PinnedLayerShellPlacement placement =
            markshot::shot::pinnedLayerShellPlacement(QRect(2200, 1200, 240, 120),
                                                      QRect(0, 0, 1920, 1080),
                                                      QSize(24, 24));

        QCOMPARE(placement.logicalGeometry, QRect(1896, 1056, 240, 120));
        QCOMPARE(placement.visibleGeometry, QRect(1896, 1056, 24, 24));
        QCOMPARE(placement.desiredSize, QSize(24, 24));
    }

    /**
     * 验证多屏时选择可见面积最大的屏幕。
     * @return 无返回值。
     */
    void screenSelectionUsesLargestIntersection()
    {
        const QVector<QRect> screens = {
            QRect(0, 0, 1920, 1080),
            QRect(1920, 0, 1920, 1080),
        };

        QCOMPARE(markshot::shot::bestPinnedLayerShellScreenIndex(QRect(1850, 100, 300, 200), screens), 1);
    }

    /**
     * 验证完全离屏时选择距离最近的屏幕。
     * @return 无返回值。
     */
    void screenSelectionFallsBackToNearestScreen()
    {
        const QVector<QRect> screens = {
            QRect(0, 0, 1920, 1080),
            QRect(1920, 0, 1920, 1080),
        };

        QCOMPARE(markshot::shot::bestPinnedLayerShellScreenIndex(QRect(-500, 100, 200, 120), screens), 0);
        QCOMPARE(markshot::shot::bestPinnedLayerShellScreenIndex(QRect(4200, 100, 200, 120), screens), 1);
    }
};

QTEST_APPLESS_MAIN(PinnedLayerShellGeometryTest)

#include "pinned_layer_shell_geometry_test.moc"
