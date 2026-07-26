#include "recording/ui/recording_overlay_layout.h"

#include <QtTest/QtTest>

class RecordingOverlayLayoutTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证控制条优先放在录制区域下方且水平居中。
     * @return 无返回值。
     */
    void placesBarBelowRegion()
    {
        const QRect screen(0, 0, 1920, 1080);
        const QRect region(400, 200, 800, 600);
        const QSize bar(220, 36);

        const QRect result = markshot::recording::ui::recordingControlBarRect(region, screen, bar);
        QCOMPARE(result.size(), bar);
        // 整数取中会有一个像素的偏差，这里只要求视觉居中
        QVERIFY(qAbs(result.center().x() - region.center().x()) <= 1);
        QVERIFY(result.top() > region.bottom());
        QVERIFY(screen.contains(result));
    }

    /**
     * 验证下方空间不足时改放到录制区域上方。
     * @return 无返回值。
     */
    void placesBarAboveWhenNoRoomBelow()
    {
        const QRect screen(0, 0, 1920, 1080);
        const QRect region(400, 400, 800, 670);
        const QSize bar(220, 36);

        const QRect result = markshot::recording::ui::recordingControlBarRect(region, screen, bar);
        QVERIFY(result.bottom() < region.top());
        QVERIFY(screen.contains(result));
    }

    /**
     * 验证控制条始终保持在屏幕内。
     * @return 无返回值。
     */
    void keepsBarInsideScreen()
    {
        const QRect screen(0, 0, 1920, 1080);
        const QRect region(1800, 100, 110, 90);
        const QSize bar(220, 36);

        const QRect result = markshot::recording::ui::recordingControlBarRect(region, screen, bar);
        QVERIFY(screen.contains(result));
    }

    /**
     * 验证整屏录制不显示边框与控制条。
     * @return 无返回值。
     */
    void hidesOverlayForWholeScreenRecording()
    {
        const QRect screen(0, 0, 1920, 1080);
        const auto placement = markshot::recording::ui::recordingOverlayPlacement(screen,
                                                                                  screen,
                                                                                  true,
                                                                                  QSize(220, 36));
        QVERIFY(!placement.showRegionFrame);
        QVERIFY(!placement.showControlBar);
    }

    /**
     * 验证控制条与录制区域相交时不显示，避免被录进画面。
     * @return 无返回值。
     */
    void hidesControlBarWhenItWouldBeRecorded()
    {
        const QRect screen(0, 0, 1920, 1080);
        // 区域几乎占满屏幕，控制条无处安放
        const QRect region(0, 0, 1920, 1070);
        const auto placement = markshot::recording::ui::recordingOverlayPlacement(region,
                                                                                  screen,
                                                                                  false,
                                                                                  QSize(220, 36));
        QVERIFY(placement.showRegionFrame);
        QVERIFY(!placement.showControlBar);
    }

    /**
     * 验证区域录制时边框与控制条都会显示。
     * @return 无返回值。
     */
    void showsOverlayForRegionRecording()
    {
        const QRect screen(0, 0, 1920, 1080);
        const QRect region(300, 200, 900, 500);
        const auto placement = markshot::recording::ui::recordingOverlayPlacement(region,
                                                                                  screen,
                                                                                  false,
                                                                                  QSize(220, 36));
        QVERIFY(placement.showRegionFrame);
        QVERIFY(placement.showControlBar);
        QVERIFY(!placement.controlBarRect.intersects(region));
    }
};

QTEST_APPLESS_MAIN(RecordingOverlayLayoutTest)

#include "recording_overlay_layout_test.moc"
