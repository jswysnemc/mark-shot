#include "capture_geometry.h"

#include <algorithm>

namespace markshot::capture {

QRect scaledCropRect(QRect sourceGeometry, QRect requestedGeometry, QSize imageSize)
{
    if (sourceGeometry.isEmpty() || requestedGeometry.isEmpty() || imageSize.isEmpty()) {
        return {};
    }

    const QRect source = sourceGeometry.normalized();
    const QRect requested = requestedGeometry.normalized();
    const QRect overlap = requested.intersected(source);
    if (overlap.isEmpty()) {
        return {};
    }

    const qreal scaleX = static_cast<qreal>(imageSize.width()) / source.width();
    const qreal scaleY = static_cast<qreal>(imageSize.height()) / source.height();
    const int left = qRound((overlap.left() - source.left()) * scaleX);
    const int top = qRound((overlap.top() - source.top()) * scaleY);
    const int right = qRound((overlap.right() + 1 - source.left()) * scaleX);
    const int bottom = qRound((overlap.bottom() + 1 - source.top()) * scaleY);
    return QRect(left, top, std::max(1, right - left), std::max(1, bottom - top))
        .intersected(QRect(QPoint(0, 0), imageSize));
}

QImage cropFrameToRequest(const QImage &frame, QRect streamGeometry, QRect requestedGeometry)
{
    if (frame.isNull() || requestedGeometry.isEmpty()) {
        return frame;
    }

    if (streamGeometry.isNull() || streamGeometry.isEmpty()) {
        streamGeometry = QRect(QPoint(0, 0), frame.size());
    }

    const QRect crop = scaledCropRect(streamGeometry, requestedGeometry, frame.size());
    return crop.isEmpty() ? QImage() : frame.copy(crop);
}

QRect imageRectFromGeometry(QRect geometry, QRect sourceGeometry, QSize imageSize)
{
    if (geometry.isEmpty() || imageSize.isEmpty()) {
        return {};
    }

    geometry = geometry.normalized();
    const QRect imageBounds(QPoint(0, 0), imageSize);
    if (!sourceGeometry.isValid() || sourceGeometry.isEmpty()) {
        return geometry.intersected(imageBounds);
    }

    return scaledCropRect(sourceGeometry, geometry, imageSize);
}

QRect geometryFromImageRect(QRect imageRect, QRect sourceGeometry, QSize imageSize)
{
    if (imageRect.isEmpty() || sourceGeometry.isEmpty() || imageSize.isEmpty()) {
        return {};
    }

    const QRect source = sourceGeometry.normalized();
    imageRect = imageRect.normalized().intersected(QRect(QPoint(0, 0), imageSize));
    if (imageRect.isEmpty()) {
        return {};
    }

    const qreal scaleX = static_cast<qreal>(source.width()) / imageSize.width();
    const qreal scaleY = static_cast<qreal>(source.height()) / imageSize.height();
    const int left = source.left() + qRound(imageRect.left() * scaleX);
    const int top = source.top() + qRound(imageRect.top() * scaleY);
    const int right = source.left() + qRound((imageRect.left() + imageRect.width()) * scaleX);
    const int bottom = source.top() + qRound((imageRect.top() + imageRect.height()) * scaleY);
    return QRect(left, top, std::max(1, right - left), std::max(1, bottom - top))
        .intersected(source);
}

} // namespace markshot::capture
