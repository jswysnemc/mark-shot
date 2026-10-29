#include "clipboard_image_mime_data.h"

#include <QBuffer>
#include <QImage>

#include <utility>

namespace markshot {
namespace {

const QString kPngMimeType = QStringLiteral("image/png");
const QString kBmpMimeType = QStringLiteral("image/bmp");

}  // namespace

ClipboardImageMimeData::ClipboardImageMimeData(QByteArray png)
    : m_png(std::move(png))
{
}

QByteArray ClipboardImageMimeData::png() const
{
    return m_png;
}

QStringList ClipboardImageMimeData::formats() const
{
    // 1. 不声明 application/x-qt-image，避免 Qt 把每次请求都改为解码后重新编码
    return {kPngMimeType, kBmpMimeType};
}

QVariant ClipboardImageMimeData::retrieveData(const QString &mimeType, QMetaType preferredType) const
{
    Q_UNUSED(preferredType);
    if (mimeType == kPngMimeType) {
        return m_png;
    }
    if (mimeType == kBmpMimeType) {
        return bmpFromPng(m_png);
    }
    return {};
}

QByteArray bmpFromPng(const QByteArray &png)
{
    // 1. 解码 PNG，失败时不提供 BMP
    QImage image;
    if (!image.loadFromData(png, "PNG")) {
        return {};
    }
    // 2. 转为不带 Alpha 的 32 位格式，Qt 的 BMP 写出器会输出 24 位位图
    image = image.convertToFormat(QImage::Format_RGB32);
    QByteArray bmp;
    QBuffer buffer(&bmp);
    if (!buffer.open(QIODevice::WriteOnly) || !image.save(&buffer, "BMP")) {
        return {};
    }
    return bmp;
}

}  // namespace markshot
