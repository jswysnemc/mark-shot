#pragma once

#include <QImage>
#include <QRect>
#include <QSize>

namespace markshot::capture {

QRect scaledCropRect(QRect sourceGeometry, QRect requestedGeometry, QSize imageSize);
QImage cropFrameToRequest(const QImage &frame, QRect streamGeometry, QRect requestedGeometry);
QRect imageRectFromGeometry(QRect geometry, QRect sourceGeometry, QSize imageSize);
QRect geometryFromImageRect(QRect imageRect, QRect sourceGeometry, QSize imageSize);

} // namespace markshot::capture
