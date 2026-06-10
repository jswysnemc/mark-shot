#include "screen_capture_cursor.h"

#include <QColor>
#include <QtTest/QtTest>

class ScreenCaptureCursorTest : public QObject {
    Q_OBJECT

private slots:
    void paintsCursorAtScaledPosition()
    {
        CaptureResult capture;
        capture.image = QImage(200, 100, QImage::Format_ARGB32_Premultiplied);
        capture.image.fill(Qt::transparent);
        capture.sourceGeometry = QRect(10, 20, 100, 50);

        markshot::capture::CursorFrame cursor;
        cursor.image = QImage(4, 4, QImage::Format_ARGB32_Premultiplied);
        cursor.image.fill(QColor(255, 0, 0, 255));
        cursor.hotSpot = QPoint(1, 1);
        cursor.globalPosition = QPoint(20, 30);

        QVERIFY(markshot::capture::paintCursorFrameIntoCapture(&capture, cursor));
        QCOMPARE(QColor(capture.image.pixel(18, 18)), QColor(255, 0, 0, 255));
    }

    void skipsCursorOutsideCapture()
    {
        CaptureResult capture;
        capture.image = QImage(100, 100, QImage::Format_ARGB32_Premultiplied);
        capture.image.fill(Qt::transparent);
        capture.sourceGeometry = QRect(0, 0, 100, 100);

        markshot::capture::CursorFrame cursor;
        cursor.image = QImage(4, 4, QImage::Format_ARGB32_Premultiplied);
        cursor.image.fill(QColor(255, 0, 0, 255));
        cursor.hotSpot = QPoint(0, 0);
        cursor.globalPosition = QPoint(200, 200);

        QVERIFY(!markshot::capture::paintCursorFrameIntoCapture(&capture, cursor));
    }
};

QTEST_MAIN(ScreenCaptureCursorTest)
#include "screen_capture_cursor_test.moc"
