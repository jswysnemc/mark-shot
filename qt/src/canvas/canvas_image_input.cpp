#include "canvas_image_input.h"

#include <QColorSpace>

#include <memory>
#include <utility>

namespace markshot::canvas {
namespace {

/** 【画布】【图像寿命】释放 owner 持有的源图引用，无返回值。 */
void releaseSource(void *owner)
{
    delete static_cast<QImage *>(owner);
}

}  // namespace

QImage canvasImageInput(QImage image)
{
    if (image.isNull()) return image;
    // 1. 【画布】【像素格式】不透明图和预乘图直接复用，其他格式优先就地转换
    if (image.format() != QImage::Format_ARGB32_Premultiplied && image.format() != QImage::Format_RGB32) {
        image = std::move(image).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    }
    if (image.devicePixelRatio() == 1.0) return image;
    if (image.isDetached()) {
        image.setDevicePixelRatio(1.0);
        return image;
    }

    // 2. 【画布】【元数据隔离】像素坐标绘制需要 DPR 为 1；仅替换元数据，不复制贴图仍持有的整帧
    auto owner = std::make_unique<QImage>(std::move(image));
    QImage view(owner->constBits(), owner->width(), owner->height(), owner->bytesPerLine(),
                owner->format(), releaseSource, owner.get());
    if (view.isNull()) {
        owner->setDevicePixelRatio(1.0);
        return std::move(*owner);
    }
    const QImage source = *owner.release();
    view.setColorSpace(source.colorSpace());
    view.setDotsPerMeterX(source.dotsPerMeterX());
    view.setDotsPerMeterY(source.dotsPerMeterY());
    view.setOffset(source.offset());
    for (const QString &key : source.textKeys()) view.setText(key, source.text(key));

    // 3. 【画布】【写入隔离】只读缓冲区在写入时复制；清理回调使源图存活到最后一个视图释放
    return view;
}

}  // namespace markshot::canvas
