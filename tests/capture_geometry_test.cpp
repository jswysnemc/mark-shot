#include "capture_geometry.h"

#include <QtTest/QtTest>

/// @brief Test class for verifying capture geometry calculations.
class CaptureGeometryTest : public QObject
{
    /// @brief Qt meta-object declaration for this class.
    Q_OBJECT

private slots:
    void scaledCropRectMapsLogicalGeometryToImagePixels()
    {
        const QRect source(-1920, 0, 3840, 1080);
        const QRect requested(0, 0, 1920, 1080);
        const QSize imageSize(7680, 2160);

        const QRect crop = markshot::capture::scaledCropRect(source, requested, imageSize);

        QCOMPARE(crop, QRect(3840, 0, 3840, 2160));
    }

    void scaledCropRectClipsRequestsOutsideSource()
    {
        const QRect source(0, 0, 100, 100);
        const QRect requested(50, 25, 100, 75);
        const QSize imageSize(200, 200);

        const QRect crop = markshot::capture::scaledCropRect(source, requested, imageSize);

        QCOMPARE(crop, QRect(100, 50, 100, 150));
    }

    void scaledCropRectReturnsEmptyForMiss()
    {
        const QRect crop = markshot::capture::scaledCropRect(QRect(0, 0, 100, 100),
                                                             QRect(120, 0, 50, 50),
                                                             QSize(200, 200));

        QVERIFY(crop.isEmpty());
    }

    void cropFrameToRequestUsesFrameAsFallbackGeometry()
    {
        QImage frame(120, 80, QImage::Format_ARGB32_Premultiplied);
        frame.fill(Qt::red);

        const QImage cropped = markshot::capture::cropFrameToRequest(frame,
                                                                     QRect(),
                                                                     QRect(10, 20, 30, 40));

        QCOMPARE(cropped.size(), QSize(30, 40));
    }

    void cropFrameToRequestMapsOutputGeometryToPhysicalFrame()
    {
        QImage frame(400, 100, QImage::Format_ARGB32_Premultiplied);
        frame.fill(Qt::red);
        for (int y = 0; y < frame.height(); ++y) {
            for (int x = 200; x < frame.width(); ++x) {
                frame.setPixelColor(x, y, Qt::blue);
            }
        }

        const QImage cropped = markshot::capture::cropFrameToRequest(frame,
                                                                     QRect(0, 0, 200, 50),
                                                                     QRect(100, 0, 100, 50));

        QCOMPARE(cropped.size(), QSize(200, 100));
        QCOMPARE(cropped.pixelColor(0, 0), QColor(Qt::blue));
        QCOMPARE(cropped.pixelColor(cropped.width() - 1, cropped.height() - 1), QColor(Qt::blue));
    }

    void cropFrameToRequestKeepsPhysicalPixelsForScaledOutput()
    {
        QImage image(400, 200, QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::black);

        const QImage cropped = markshot::capture::cropFrameToRequest(image,
                                                                     QRect(0, 0, 200, 100),
                                                                     QRect(0, 0, 200, 100));

        QCOMPARE(cropped.size(), QSize(400, 200));
        QCOMPARE(cropped.devicePixelRatio(), 1.0);
    }

    void resizeFrameToGeometrySizeMatchesLogicalBounds()
    {
        QImage image(400, 200, QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::black);
        image.setDevicePixelRatio(2.0);

        const QImage resized =
            markshot::capture::resizeFrameToGeometrySize(image, QRect(10, 20, 200, 100));

        QCOMPARE(resized.size(), QSize(200, 100));
        QCOMPARE(resized.devicePixelRatio(), 1.0);
    }

    void cropAndResizeMatchesScrollCaptureSelectionBounds()
    {
        QImage image(400, 200, QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::black);

        const QRect sourceGeometry(0, 0, 200, 100);
        const QRect selectedGeometry(50, 25, 100, 50);
        const QImage cropped =
            markshot::capture::cropFrameToRequest(image, sourceGeometry, selectedGeometry);
        const QImage resized =
            markshot::capture::resizeFrameToGeometrySize(cropped, selectedGeometry);

        QCOMPARE(cropped.size(), QSize(200, 100));
        QCOMPARE(resized.size(), selectedGeometry.size());
        QCOMPARE(resized.devicePixelRatio(), 1.0);
    }

    void imageRectAndGeometryRoundTrip()
    {
        const QRect source(100, 50, 800, 400);
        const QSize imageSize(1600, 800);
        const QRect geometry(300, 150, 200, 100);

        const QRect imageRect = markshot::capture::imageRectFromGeometry(geometry, source, imageSize);
        const QRect roundTrip = markshot::capture::geometryFromImageRect(imageRect, source, imageSize);

        QCOMPARE(imageRect, QRect(400, 200, 400, 200));
        QCOMPARE(roundTrip, geometry);
    }

    void perScreenCropRectHorizontalMixedDprWithGap()
    {
        const QList<markshot::capture::ScreenLayoutEntry> screens = {
            {QRect(0, 0, 1920, 1080), 2.0},    // HDMI-1 4K@200%
            {QRect(3840, 0, 1920, 1080), 1.0}, // DP-1 1080p@100%
        };
        const QSize rawImageSize(5760, 2160);

        const QRect cropHdmi = markshot::capture::perScreenCropRect(screens, QRect(0, 0, 1920, 1080), rawImageSize);
        QCOMPARE(cropHdmi, QRect(0, 0, 3840, 2160));

        const QRect cropDp = markshot::capture::perScreenCropRect(screens, QRect(3840, 0, 1920, 1080), rawImageSize);
        QCOMPARE(cropDp, QRect(3840, 0, 1920, 1080));
    }

    void perScreenCropRectHorizontalMixedDprAdjacent()
    {
        const QList<markshot::capture::ScreenLayoutEntry> screens = {
            {QRect(0, 0, 1920, 1080), 2.0},
            {QRect(1920, 0, 1920, 1080), 1.0},
        };
        const QSize rawImageSize(5760, 2160);

        const QRect cropFirst = markshot::capture::perScreenCropRect(screens, QRect(0, 0, 1920, 1080), rawImageSize);
        QCOMPARE(cropFirst, QRect(0, 0, 3840, 2160));

        const QRect cropSecond = markshot::capture::perScreenCropRect(screens, QRect(1920, 0, 1920, 1080), rawImageSize);
        QCOMPARE(cropSecond, QRect(3840, 0, 1920, 1080));
    }

    void perScreenCropRectVerticalMixedDpr()
    {
        const QList<markshot::capture::ScreenLayoutEntry> screens = {
            {QRect(0, 0, 1920, 1080), 2.0},
            {QRect(0, 1080, 1920, 1080), 1.0},
        };
        const QSize rawImageSize(3840, 3240);

        const QRect cropTop = markshot::capture::perScreenCropRect(screens, QRect(0, 0, 1920, 1080), rawImageSize);
        QCOMPARE(cropTop, QRect(0, 0, 3840, 2160));

        const QRect cropBottom = markshot::capture::perScreenCropRect(screens, QRect(0, 1080, 1920, 1080), rawImageSize);
        QCOMPARE(cropBottom, QRect(0, 2160, 1920, 1080));
    }

    void perScreenCropRectUniformDprReturnsEmpty()
    {
        const QList<markshot::capture::ScreenLayoutEntry> screens = {
            {QRect(0, 0, 1920, 1080), 2.0},
            {QRect(1920, 0, 1920, 1080), 2.0},
        };
        const QSize rawImageSize(7680, 2160);

        const QRect crop = markshot::capture::perScreenCropRect(screens, QRect(0, 0, 1920, 1080), rawImageSize);
        QVERIFY(crop.isEmpty());
    }

    void perScreenCropRectTargetNotFoundReturnsEmpty()
    {
        const QList<markshot::capture::ScreenLayoutEntry> screens = {
            {QRect(0, 0, 1920, 1080), 2.0},
            {QRect(3840, 0, 1920, 1080), 1.0},
        };
        const QSize rawImageSize(5760, 2160);

        const QRect crop = markshot::capture::perScreenCropRect(screens, QRect(100, 100, 500, 500), rawImageSize);
        QVERIFY(crop.isEmpty());
    }

    void perScreenCropRectRawSizeMismatchReturnsEmpty()
    {
        const QList<markshot::capture::ScreenLayoutEntry> screens = {
            {QRect(0, 0, 1920, 1080), 2.0},
            {QRect(3840, 0, 1920, 1080), 1.0},
        };
        const QSize rawImageSize(9999, 9999);

        const QRect crop = markshot::capture::perScreenCropRect(screens, QRect(0, 0, 1920, 1080), rawImageSize);
        QVERIFY(crop.isEmpty());
    }
};

/// @brief Main entry point for the CaptureGeometryTest test suite.
/// @param argc The number of command-line arguments.
/// @param argv The array of command-line arguments.
/// @return The exit status of the test suite.
QTEST_APPLESS_MAIN(CaptureGeometryTest)

#include "capture_geometry_test.moc"
