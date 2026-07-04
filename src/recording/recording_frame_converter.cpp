#include "recording/recording_frame_converter.h"

#include <cstring>

namespace markshot::recording {

RecordingBgraFrame RecordingFrameConverter::convertToBgra(const QImage &frame,
                                                          QSize targetSize,
                                                          QString *error)
{
    if (error) {
        error->clear();
    }
    if (frame.isNull() || targetSize.isEmpty()) {
        if (error) {
            *error = QStringLiteral("Cannot convert an empty recording frame");
        }
        return {};
    }

    QImage image = frame;
    if (image.size() != targetSize) {
        image = image.scaled(targetSize, Qt::IgnoreAspectRatio, Qt::SmoothTransformation);
    }
    if (!isBgraCompatible(image.format())) {
        image = image.convertToFormat(QImage::Format_ARGB32);
    }

    m_converted = image;
    const int rowBytes = targetSize.width() * 4;
    return contiguousView(m_converted, rowBytes);
}

bool RecordingFrameConverter::isBgraCompatible(QImage::Format format)
{
    return format == QImage::Format_ARGB32
        || format == QImage::Format_ARGB32_Premultiplied
        || format == QImage::Format_RGB32;
}

RecordingBgraFrame RecordingFrameConverter::contiguousView(const QImage &image, int rowBytes)
{
    const qsizetype frameBytes = static_cast<qsizetype>(rowBytes) * image.height();
    if (image.bytesPerLine() == rowBytes) {
        return {reinterpret_cast<const char *>(image.constBits()), frameBytes, rowBytes};
    }

    m_buffer.resize(frameBytes);
    char *destination = m_buffer.data();
    for (int y = 0; y < image.height(); ++y) {
        std::memcpy(destination + static_cast<qsizetype>(y) * rowBytes,
                    image.constScanLine(y),
                    static_cast<size_t>(rowBytes));
    }
    return {m_buffer.constData(), m_buffer.size(), rowBytes};
}

}  // namespace markshot::recording
