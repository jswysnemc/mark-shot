#include "ocr_result_window_geometry.h"

#include <QtTest/QtTest>

class OcrResultWindowGeometryTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证 OCR 结果窗口以截图所在的次屏为中心。
     * @return 无返回值。
     */
    void placementUsesCaptureScreen()
    {
        const QRect captureScreen(1707, 0, 2560, 1440);
        const QRect primaryScreen(0, 0, 1707, 1067);

        const markshot::shot::OcrResultWindowPlacement placement =
            markshot::shot::ocrResultWindowPlacement(captureScreen, primaryScreen);

        QRect expectedGeometry(QPoint(0, 0), QSize(420, 520));
        expectedGeometry.moveCenter(captureScreen.center());
        QCOMPARE(placement.size, expectedGeometry.size());
        QCOMPARE(placement.topLeft, expectedGeometry.topLeft());
        QVERIFY(captureScreen.contains(expectedGeometry.center()));
    }

    /**
     * 验证截图屏幕信息缺失时回退到主屏幕。
     * @return 无返回值。
     */
    void placementFallsBackToPrimaryScreen()
    {
        const QRect primaryScreen(0, 0, 1920, 1080);

        const markshot::shot::OcrResultWindowPlacement placement =
            markshot::shot::ocrResultWindowPlacement({}, primaryScreen);

        QRect expectedGeometry(QPoint(0, 0), QSize(420, 520));
        expectedGeometry.moveCenter(primaryScreen.center());
        QCOMPARE(placement.topLeft, expectedGeometry.topLeft());
    }
};

QTEST_APPLESS_MAIN(OcrResultWindowGeometryTest)

#include "ocr_result_window_geometry_test.moc"
