#include "screen_capture_cursor.h"

#include <QCursor>
#include <QGuiApplication>
#include <QPainter>
#include <QPainterPath>
#include <QPixmap>

#include <algorithm>

namespace {

/// @brief 根据截图几何计算从全局坐标到图像像素的缩放比例。
/// @param image 截图图像。
/// @param sourceGeometry 截图覆盖的全局逻辑坐标范围。
/// @return 横向和纵向缩放比例。
QPointF captureScale(const QImage &image, const QRect &sourceGeometry)
{
    return QPointF(static_cast<qreal>(image.width()) / std::max(1, sourceGeometry.width()),
                   static_cast<qreal>(image.height()) / std::max(1, sourceGeometry.height()));
}

/// @brief 获取当前 Qt 覆盖光标，缺失时返回箭头光标。
/// @return 当前光标对象。
QCursor currentCursor()
{
    if (const QCursor *cursor = QGuiApplication::overrideCursor()) {
        return *cursor;
    }
    return QCursor(Qt::ArrowCursor);
}

/// @brief 创建当前鼠标帧描述。
/// @return 当前鼠标的图像、热点和全局位置。
markshot::capture::CursorFrame currentCursorFrame()
{
    const QCursor cursor = currentCursor();
    QImage image;
    QPoint hotSpot = cursor.hotSpot();
    const QPixmap pixmap = cursor.pixmap();
    if (!pixmap.isNull()) {
        image = pixmap.toImage().convertToFormat(QImage::Format_ARGB32_Premultiplied);
        image.setDevicePixelRatio(1.0);
    }
    if (image.isNull()) {
        image = markshot::capture::fallbackCursorImage();
        hotSpot = QPoint(0, 0);
    }
    if (hotSpot.x() < 0 || hotSpot.y() < 0) {
        hotSpot = QPoint(0, 0);
    }
    return {image, hotSpot, QCursor::pos()};
}

}  // namespace

namespace markshot::capture {

QImage fallbackCursorImage()
{
    QImage image(28, 32, QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::transparent);

    QPainter painter(&image);
    painter.setRenderHint(QPainter::Antialiasing, true);

    QPainterPath body;
    body.moveTo(1.5, 1.5);
    body.lineTo(1.5, 24.0);
    body.lineTo(7.5, 18.0);
    body.lineTo(11.5, 29.0);
    body.lineTo(16.5, 27.0);
    body.lineTo(12.5, 16.0);
    body.lineTo(21.5, 16.0);
    body.closeSubpath();

    painter.setPen(QPen(QColor(17, 24, 39, 235), 3.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(QColor(255, 255, 255, 245));
    painter.drawPath(body);
    painter.setPen(QPen(QColor(255, 255, 255, 225), 1.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(body);
    painter.end();

    return image;
}

bool paintCursorFrameIntoCapture(CaptureResult *capture, const CursorFrame &cursor)
{
    if (!capture || capture->image.isNull() || cursor.image.isNull()
        || !capture->sourceGeometry.isValid() || capture->sourceGeometry.isEmpty()) {
        return false;
    }

    const QRect sourceGeometry = capture->sourceGeometry.normalized();
    const QPointF scale = captureScale(capture->image, sourceGeometry);
    const QPointF cursorPosition((cursor.globalPosition.x() - sourceGeometry.x()) * scale.x(),
                                 (cursor.globalPosition.y() - sourceGeometry.y()) * scale.y());
    const QSizeF cursorSize(cursor.image.width() * scale.x(),
                            cursor.image.height() * scale.y());
    const QPointF hotSpot(cursor.hotSpot.x() * scale.x(), cursor.hotSpot.y() * scale.y());
    const QRectF targetRect(cursorPosition - hotSpot, cursorSize);
    if (!targetRect.intersects(QRectF(capture->image.rect()))) {
        return false;
    }

    QPainter painter(&capture->image);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
    painter.drawImage(targetRect, cursor.image);
    painter.end();
    return true;
}

bool paintCurrentCursorIntoCapture(CaptureResult *capture)
{
    return paintCursorFrameIntoCapture(capture, currentCursorFrame());
}

}  // namespace markshot::capture
