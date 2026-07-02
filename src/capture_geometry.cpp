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

QRect perScreenCropRect(const QList<ScreenLayoutEntry> &screens,
                        const QRect &targetGeometry,
                        QSize rawImageSize)
{
    if (screens.isEmpty() || targetGeometry.isEmpty() || rawImageSize.isEmpty()) {
        return {};
    }

    int targetIndex = -1;
    for (int i = 0; i < screens.size(); ++i) {
        if (screens[i].geometry == targetGeometry) {
            targetIndex = i;
            break;
        }
    }
    if (targetIndex < 0) {
        return {};
    }

    qreal firstDpr = screens.first().dpr;
    bool hasMixedDpr = false;
    for (const auto &entry : screens) {
        if (qAbs(entry.dpr - firstDpr) > 0.01) {
            hasMixedDpr = true;
            break;
        }
    }
    if (!hasMixedDpr) {
        return {};
    }

    QList<int> validIndices;
    for (int i = 0; i < screens.size(); ++i) {
        if (!screens[i].geometry.isEmpty() && screens[i].dpr > 0) {
            validIndices.append(i);
        }
    }
    if (validIndices.isEmpty()) {
        return {};
    }

    struct RowGroup {
        QList<int> indices;
        int minY = 0;
        int maxY = 0;
    };
    QList<RowGroup> rows;
    for (int idx : validIndices) {
        const QRect &g = screens[idx].geometry;
        bool placed = false;
        for (RowGroup &row : rows) {
            if (g.y() < row.maxY && row.minY < g.y() + g.height()) {
                row.indices.append(idx);
                row.minY = std::min(row.minY, g.y());
                row.maxY = std::max(row.maxY, g.y() + g.height());
                placed = true;
                break;
            }
        }
        if (!placed) {
            rows.append({{idx}, g.y(), g.y() + g.height()});
        }
    }

    std::sort(rows.begin(), rows.end(), [](const RowGroup &a, const RowGroup &b) {
        return a.minY < b.minY;
    });

    for (RowGroup &row : rows) {
        std::sort(row.indices.begin(), row.indices.end(), [&](int a, int b) {
            return screens[a].geometry.x() < screens[b].geometry.x();
        });
    }

    QList<QRect> physicalRects;
    physicalRects.resize(screens.size());
    int physYOffset = 0;
    int maxRowPhysWidth = 0;
    for (const RowGroup &row : rows) {
        int physXOffset = 0;
        int rowPhysHeight = 0;
        for (int idx : row.indices) {
            const auto &screen = screens[idx];
            const int physW = qRound(screen.geometry.width() * screen.dpr);
            const int physH = qRound(screen.geometry.height() * screen.dpr);
            physicalRects[idx] = QRect(physXOffset, physYOffset, physW, physH);
            physXOffset += physW;
            rowPhysHeight = std::max(rowPhysHeight, physH);
        }
        maxRowPhysWidth = std::max(maxRowPhysWidth, physXOffset);
        physYOffset += rowPhysHeight;
    }

    const int tolerance = 2;
    if (qAbs(maxRowPhysWidth - rawImageSize.width()) > tolerance ||
        qAbs(physYOffset - rawImageSize.height()) > tolerance) {
        return {};
    }

    return physicalRects[targetIndex].intersected(QRect(QPoint(0, 0), rawImageSize));
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

QImage resizeFrameToGeometrySize(const QImage &frame, QRect geometry)
{
    if (frame.isNull() || !geometry.isValid() || geometry.isEmpty()) {
        return frame;
    }

    const QSize targetSize = geometry.normalized().size();
    if (targetSize.isEmpty() || frame.size() == targetSize) {
        QImage unchanged = frame;
        unchanged.setDevicePixelRatio(1.0);
        return unchanged;
    }

    QImage resized = frame.scaled(targetSize, Qt::IgnoreAspectRatio, Qt::SmoothTransformation);
    resized.setDevicePixelRatio(1.0);
    return resized;
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
