#include "screen_capture_pipewire_screencast.h"

#ifdef HAVE_PIPEWIRE

#include "debug_log.h"
#include "pipewire/pipewire_dmabuf_policy.h"

#include <cerrno>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <sys/mman.h>

namespace {

/**
 * 【录制】【PipeWire帧读取】读取 raw 像素格式每像素字节数。
 * @param format PipeWire 像素格式。
 * @return 每像素字节数，不支持时返回 0。
 */
int rawBytesPerPixel(spa_video_format format)
{
    switch (format) {
    case SPA_VIDEO_FORMAT_BGRA:
    case SPA_VIDEO_FORMAT_BGRx:
    case SPA_VIDEO_FORMAT_xBGR:
    case SPA_VIDEO_FORMAT_RGBA:
    case SPA_VIDEO_FORMAT_RGBx:
    case SPA_VIDEO_FORMAT_ARGB:
    case SPA_VIDEO_FORMAT_ABGR:
    case SPA_VIDEO_FORMAT_xRGB:
        return 4;
    case SPA_VIDEO_FORMAT_RGB:
    case SPA_VIDEO_FORMAT_BGR:
        return 3;
    default:
        return 0;
    }
}

/**
 * 【录制】【PipeWire帧读取】判断当前 buffer 是否可以尝试 fd 映射。
 * @param data PipeWire 数据块。
 * @return 具备 fd 和有效大小时返回 true。
 */
bool canTryFdMap(const spa_data &data)
{
    return data.fd >= 0 && data.maxsize > 0
        && data.type == SPA_DATA_MemFd;
}

/**
 * 【录制】【PipeWire帧读取】把 QImage 转成连续 BGRA 字节。
 * @param image 输入图像。
 * @param buffer 已通过预算准入的目标缓冲。
 * @param frame 输出 raw BGRA 帧。
 * @return 转换成功时返回 true。
 */
bool fillRawFrameFromImage(const QImage &image,
                           QByteArray *buffer,
                           PipeWireScreencastRawFrame *frame)
{
    if (image.isNull() || !frame || !buffer
        || buffer->size() != markshot::recording::recordingBgraFrameBytes(image.size())) {
        return false;
    }
    QImage bgra = image;
    if (bgra.format() != QImage::Format_ARGB32
        && bgra.format() != QImage::Format_ARGB32_Premultiplied
        && bgra.format() != QImage::Format_RGB32) {
        bgra = bgra.convertToFormat(QImage::Format_ARGB32);
    }
    if (bgra.isNull()) return false;

    const int rowBytes = bgra.width() * 4;
    for (int y = 0; y < bgra.height(); ++y) {
        std::memcpy(buffer->data() + static_cast<qsizetype>(y) * rowBytes,
                    bgra.constScanLine(y),
                    static_cast<size_t>(rowBytes));
    }
    frame->bgra = *buffer;
    frame->size = bgra.size();
    frame->stride = rowBytes;
    frame->yInverted = false;
    return true;
}

}  // namespace

bool PortalPipeWireScreencast::rawFrameFromBuffer(pw_buffer *pipewireBuffer,
                                                  PipeWireScreencastRawFrame *frame,
                                                  QString *error)
{
    if (error) {
        error->clear();
    }
    if (!frame) {
        if (error) {
            *error = QStringLiteral("PipeWire raw frame output is empty");
        }
        return false;
    }
    if (!pipewireBuffer || !pipewireBuffer->buffer || pipewireBuffer->buffer->n_datas == 0) {
        if (error) {
            *error = QStringLiteral("PipeWire delivered an empty buffer");
        }
        return false;
    }

    const spa_buffer *spaBuffer = pipewireBuffer->buffer;
    const spa_data &data = spaBuffer->datas[0];
    if (!data.chunk) {
        if (error) {
            *error = QStringLiteral("PipeWire buffer chunk is missing (data type %1)")
                         .arg(static_cast<uint>(data.type));
        }
        return false;
    }
    if (data.type == SPA_DATA_DmaBuf) {
        if (!readDmaBufRawFrame(spaBuffer, pipewireBuffer, frame, error)) {
            return false;
        }
    } else {
        const int width = static_cast<int>(m_videoInfo.size.width);
        const int height = static_cast<int>(m_videoInfo.size.height);
        if (!markshot::recording::recordingBgraFrameBytes(QSize(width, height))) {
            if (error) {
                *error = QStringLiteral("PipeWire frame size is invalid");
            }
            return false;
        }

        const int bytesPerPixel = rawBytesPerPixel(m_videoInfo.format);
        if (bytesPerPixel <= 0) {
            if (error) {
                *error = QStringLiteral("unsupported PipeWire video format %1")
                             .arg(static_cast<int>(m_videoInfo.format));
            }
            return false;
        }
        const qint64 rowStride = data.chunk->stride != 0
            ? std::abs(static_cast<qint64>(data.chunk->stride))
            : static_cast<qint64>(width) * bytesPerPixel;
        if (rowStride > std::numeric_limits<int>::max()
            || rowStride < static_cast<qint64>(width) * bytesPerPixel) {
            if (error) *error = QStringLiteral("PipeWire frame stride is invalid");
            return false;
        }
        const int stride = static_cast<int>(rowStride);
        void *mappedAddress = nullptr;
        size_t mappedSize = 0;
        const uchar *base = static_cast<const uchar *>(data.data);
        if (!base && canTryFdMap(data)) {
            mappedSize = data.maxsize;
            mappedAddress = ::mmap(nullptr,
                                   mappedSize,
                                   PROT_READ,
                                   MAP_SHARED,
                                   static_cast<int>(data.fd),
                                   static_cast<off_t>(data.mapoffset));
            if (mappedAddress == MAP_FAILED) {
                mappedAddress = nullptr;
                if (error) {
                    *error = QStringLiteral("failed to mmap PipeWire buffer fd: %1")
                                 .arg(QString::fromLocal8Bit(std::strerror(errno)));
                }
                return false;
            }
            base = static_cast<const uchar *>(mappedAddress);
        }
        if (!base) {
            if (error) {
                *error = QStringLiteral("PipeWire buffer is not CPU-mappable (data type %1 flags=0x%2)")
                             .arg(static_cast<uint>(data.type))
                             .arg(static_cast<uint>(data.flags), 0, 16);
            }
            return false;
        }

        const uint32_t dataOffset = data.maxsize > 0
            ? data.chunk->offset % data.maxsize
            : data.chunk->offset;
        const uchar *source = base + dataOffset;
        if (!source || stride < width * bytesPerPixel) {
            if (mappedAddress) {
                ::munmap(mappedAddress, mappedSize);
            }
            if (error) {
                *error = QStringLiteral("PipeWire frame stride is invalid");
            }
            return false;
        }

        const QRect streamGeometry = m_negotiatedStreamGeometry;
        const QRect sourceGeometry = streamGeometry.isEmpty()
            ? QRect(QPoint(0, 0), QSize(width, height))
            : streamGeometry;
        const QRect crop = m_rawRequestedGeometry.isEmpty()
            ? QRect(QPoint(0, 0), QSize(width, height))
            : markshot::capture::scaledCropRect(sourceGeometry,
                                                m_rawRequestedGeometry,
                                                QSize(width, height));
        if (crop.isEmpty()) {
            if (mappedAddress) {
                ::munmap(mappedAddress, mappedSize);
            }
            if (error) {
                *error = QStringLiteral("PipeWire raw frame does not cover requested geometry");
            }
            return false;
        }

        // 1. 【录制】【采集预算】颜色转换和像素复制前取得槽位；预算满时无错误地丢弃当前帧
        const qsizetype byteCount = markshot::recording::recordingBgraFrameBytes(crop.size());
        QByteArray *buffer = m_rawBufferPool.tryAcquire(byteCount);
        if (!buffer) {
            if (mappedAddress) ::munmap(mappedAddress, mappedSize);
            return false;
        }

        if (m_videoInfo.format == SPA_VIDEO_FORMAT_BGRA
            || m_videoInfo.format == SPA_VIDEO_FORMAT_BGRx) {
            // BGRA 快路径复用池化缓冲，避免每帧新分配整帧内存
            const int rowBytes = crop.width() * 4;
            for (int y = 0; y < crop.height(); ++y) {
                const uchar *row = source
                    + static_cast<qsizetype>(crop.y() + y) * stride
                    + static_cast<qsizetype>(crop.x()) * 4;
                std::memcpy(buffer->data() + static_cast<qsizetype>(y) * rowBytes,
                            row,
                            static_cast<size_t>(rowBytes));
            }
            frame->bgra = *buffer;
            frame->size = crop.size();
            frame->stride = rowBytes;
            frame->yInverted = false;
        } else {
            QImage image = imageFromBuffer(pipewireBuffer, error);
            if (!m_rawRequestedGeometry.isEmpty()) {
                image = markshot::capture::cropFrameToRequest(image, sourceGeometry, m_rawRequestedGeometry);
            }
            if (!fillRawFrameFromImage(image, buffer, frame)) {
                if (mappedAddress) {
                    ::munmap(mappedAddress, mappedSize);
                }
                if (error && error->isEmpty()) {
                    *error = QStringLiteral("failed to convert PipeWire frame to raw BGRA");
                }
                return false;
            }
        }
        if (mappedAddress) {
            ::munmap(mappedAddress, mappedSize);
        }
        frame->streamGeometry = sourceGeometry;
    }

    const qint64 frameTimeMs = QDateTime::currentMSecsSinceEpoch();
    if (m_rawBaseFrameTimeMs < 0) {
        m_rawBaseFrameTimeMs = frameTimeMs;
    }
    frame->timestampMs = std::max<qint64>(0, frameTimeMs - m_rawBaseFrameTimeMs);
    if (frame->streamGeometry.isEmpty()) {
        frame->streamGeometry = m_negotiatedStreamGeometry;
    }
    frame->outputName = m_rawOutputName;
    frame->cursorIncluded = m_cursorIncluded;
    return true;
}

bool PortalPipeWireScreencast::readDmaBufRawFrame(const spa_buffer *spaBuffer,
                                                  pw_buffer *pipewireBuffer,
                                                  PipeWireScreencastRawFrame *frame,
                                                  QString *error)
{
    const int width = static_cast<int>(m_videoInfo.size.width);
    const int height = static_cast<int>(m_videoInfo.size.height);
    if (!markshot::recording::recordingBgraFrameBytes(QSize(width, height))) {
        if (error) *error = QStringLiteral("PipeWire DMA-BUF frame size is invalid");
        return false;
    }
    const QRect streamGeometry = m_negotiatedStreamGeometry;
    const QRect sourceGeometry = streamGeometry.isEmpty()
        ? QRect(QPoint(0, 0), QSize(width, height)) : streamGeometry;
    const QRect crop = m_rawRequestedGeometry.isEmpty()
        ? QRect(QPoint(0, 0), QSize(width, height))
        : markshot::capture::scaledCropRect(sourceGeometry, m_rawRequestedGeometry, QSize(width, height));
    if (crop.isEmpty()) {
        if (error) *error = QStringLiteral("PipeWire DMA-BUF frame does not cover requested geometry");
        return false;
    }
    // 1. 【录制】【采集预算】GPU 读回或回退图像分配前准入，不能把预算耗尽记成驱动故障
    QByteArray *buffer = m_rawBufferPool.tryAcquire(
        markshot::recording::recordingBgraFrameBytes(crop.size()));
    if (!buffer) return false;
    if (!m_rawDmaBufDirectReadBroken) {
        if (!m_dmaBufImporter) {
            m_dmaBufImporter = std::make_unique<markshot::PipeWireDmaBufImporter>();
        }
        QString directError;
        if (m_dmaBufImporter->importBufferToBgra(spaBuffer, m_videoInfo, crop, buffer, &directError)) {
            frame->bgra = *buffer;
            frame->size = crop.size();
            frame->stride = crop.width() * 4;
            frame->yInverted = true;
            frame->streamGeometry = sourceGeometry;
            return true;
        }
        m_rawDmaBufDirectReadBroken = true;
        markshot::pipewire::markDmaBufImportBroken();
        markshot::debugLog("screencast", "【录制】【PipeWire DMA-BUF】direct-read fallback error=%s",
                           directError.toUtf8().constData());
    }

    // 2. 【录制】【PipeWire帧读取】驱动不支持直读时沿用图像回退，但复用同一个已准入缓冲
    QImage image = imageFromBuffer(pipewireBuffer, error);
    if (!m_rawRequestedGeometry.isEmpty()) {
        image = markshot::capture::cropFrameToRequest(image, sourceGeometry, m_rawRequestedGeometry);
    }
    if (!fillRawFrameFromImage(image, buffer, frame)) {
        if (error && error->isEmpty()) {
            *error = QStringLiteral("failed to convert PipeWire DMA-BUF frame to raw BGRA");
        }
        return false;
    }
    return true;
}

#endif
