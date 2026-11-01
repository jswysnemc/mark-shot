#include "rapid_image_input.h"

#include <QColorSpace>

#include <memory>

namespace markshot::ocr_rapid {
namespace {

/** 【OCR】【透明位校验】检查 image 的有效 RGB32 像素是否全部不透明，不检查行填充，返回是否可共享。 */
bool hasOpaquePixels(const QImage &image)
{
    for (int y = 0; y < image.height(); ++y) {
        const auto *row = reinterpret_cast<const QRgb *>(image.constScanLine(y));
        for (int x = 0; x < image.width(); ++x) {
            if (qAlpha(row[x]) != 255) return false;
        }
    }
    return true;
}

/** 【OCR】【输入释放】释放 owner 指向的源图共享引用，无返回值。 */
void releaseSource(void *owner)
{
    delete static_cast<QImage *>(owner);
}

}  // namespace

QImage rapidImageInput(const QImage &image)
{
    if (image.isNull() || image.format() != QImage::Format_RGB32 || !hasOpaquePixels(image)) {
        return image.convertToFormat(QImage::Format_ARGB32);
    }

    // 1. 【OCR】【输入像素】维持 ARGB32 的缩放分支；直接保留 RGB32 会改变识别结果
    auto owner = std::make_unique<QImage>(image);
    QImage view(owner->constBits(), image.width(), image.height(), image.bytesPerLine(),
                QImage::Format_ARGB32, releaseSource, owner.get());
    if (view.isNull()) return image.convertToFormat(QImage::Format_ARGB32);
    owner.release();

    // 2. 【OCR】【输入寿命】只读外部缓冲使结果写入时复制；源图共享引用保证原像素不会提前释放
    view.setDevicePixelRatio(image.devicePixelRatio());
    view.setColorSpace(image.colorSpace());
    return view;
}

}  // namespace markshot::ocr_rapid
