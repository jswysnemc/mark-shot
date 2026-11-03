#include "capture_geometry.h"
#include "geometry_bridge.h"
#include <vector>
namespace markshot::capture
{
/** 根据 sourceGeometry、requestedGeometry 和 imageSize 返回 Rust 换算的像素裁剪矩形。 */
QRect scaledCropRect(QRect sourceGeometry, QRect requestedGeometry, QSize imageSize)
{
    return rust::fromCore(shot_geometry_transform(0, rust::toCore(requestedGeometry),
                                                  rust::toCore(sourceGeometry), rust::toCore(imageSize)));
}
/** 根据 screens 布局在 rawImageSize 中定位 targetGeometry，返回混合缩放裁剪矩形。 */
QRect perScreenCropRect(const QList<ScreenLayoutEntry> &screens, const QRect &targetGeometry,
                        QSize rawImageSize)
{
    std::vector<ShotScreen> layout;
    layout.reserve(static_cast<size_t>(screens.size()));
    for (const auto &screen : screens)
    {
        layout.push_back({rust::toCore(screen.geometry), screen.dpr});
    }
    return rust::fromCore(shot_screen_crop(layout.data(), layout.size(), rust::toCore(targetGeometry),
                                           rust::toCore(rawImageSize)));
}
/** 从 frame 裁剪 streamGeometry 对应的 requestedGeometry；整帧共享像素，局部选区返回独立图像。 */
QImage cropFrameToRequest(const QImage &frame, QRect streamGeometry, QRect requestedGeometry)
{
    if (frame.isNull() || requestedGeometry.isEmpty())
    {
        return frame;
    }
    if (streamGeometry.isEmpty())
    {
        streamGeometry = QRect(QPoint(0, 0), frame.size());
    }
    const QRect crop = scaledCropRect(streamGeometry, requestedGeometry, frame.size());
    if (crop.isEmpty()) return {};
    // 1. 【截图】【整帧裁剪】保留 Qt 写时复制语义，避免全屏请求重复分配整张截图
    return crop == frame.rect() ? frame : frame.copy(crop);
}
/** 按 geometry 的逻辑尺寸缩放 frame，返回 DPR 为 1 的 Qt 展示图像。 */
QImage resizeFrameToGeometrySize(const QImage &frame, QRect geometry)
{
    if (frame.isNull() || !geometry.isValid())
    {
        return frame;
    }
    QImage result = frame.size() == geometry.size()
                        ? frame
                        : frame.scaled(geometry.size(), Qt::IgnoreAspectRatio, Qt::SmoothTransformation);
    result.setDevicePixelRatio(1.0);
    return result;
}
/** 将 geometry 从 sourceGeometry 换算至 imageSize，返回像素选区。 */
QRect imageRectFromGeometry(QRect geometry, QRect sourceGeometry, QSize imageSize)
{
    return rust::fromCore(shot_geometry_transform(1, rust::toCore(geometry), rust::toCore(sourceGeometry),
                                                  rust::toCore(imageSize)));
}
/** 将 imageRect 从 imageSize 换算至 sourceGeometry，返回逻辑选区。 */
QRect geometryFromImageRect(QRect imageRect, QRect sourceGeometry, QSize imageSize)
{
    return rust::fromCore(shot_geometry_transform(2, rust::toCore(imageRect), rust::toCore(sourceGeometry),
                                                  rust::toCore(imageSize)));
}
} // namespace markshot::capture
