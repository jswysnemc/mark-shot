#include "platform/wayland/wlroots_screencopy_shm_buffer.h"

#ifdef HAVE_WLROOTS_SCREENCOPY
#include <wayland-client.h>
#endif

#include <QImage>

#include <cerrno>
#include <cstring>

#ifdef HAVE_WLROOTS_SCREENCOPY
#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>
#endif

namespace markshot::recording {
namespace {

#ifdef HAVE_WLROOTS_SCREENCOPY
/**
 * 创建匿名共享内存文件。
 * @param size 文件大小。
 * @param error 输出错误信息。
 * @return 文件描述符，失败时返回 -1。
 */
int createSharedMemoryFile(qsizetype size, QString *error)
{
    char name[] = "/tmp/mark-shot-wlroots-shm-XXXXXX";
    const int fd = ::mkstemp(name);
    if (fd < 0) {
        if (error) {
            *error = QStringLiteral("failed to create wl_shm file: %1")
                         .arg(QString::fromLocal8Bit(std::strerror(errno)));
        }
        return -1;
    }
    ::unlink(name);
    if (::ftruncate(fd, size) < 0) {
        if (error) {
            *error = QStringLiteral("failed to resize wl_shm file: %1")
                         .arg(QString::fromLocal8Bit(std::strerror(errno)));
        }
        ::close(fd);
        return -1;
    }
    return fd;
}

/**
 * 判断像素格式是否可以直接包装为 QImage。
 * @param format wl_shm 像素格式。
 * @return 支持时返回 true。
 */
bool isSupportedFormat(std::uint32_t format)
{
    return format == WL_SHM_FORMAT_ARGB8888
        || format == WL_SHM_FORMAT_XRGB8888;
}
#endif

}  // namespace

WlrootsScreencopyShmBuffer::~WlrootsScreencopyShmBuffer()
{
    reset();
}

bool WlrootsScreencopyShmBuffer::ensure(wl_shm *shm,
                                        std::uint32_t format,
                                        int width,
                                        int height,
                                        int stride,
                                        QString *error)
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_WLROOTS_SCREENCOPY
    Q_UNUSED(shm)
    Q_UNUSED(format)
    Q_UNUSED(width)
    Q_UNUSED(height)
    Q_UNUSED(stride)
    if (error) {
        *error = QStringLiteral("wlroots screencopy support is not enabled");
    }
    return false;
#else
    if (!shm || width <= 0 || height <= 0 || stride < width * 4) {
        if (error) {
            *error = QStringLiteral("invalid wl_shm buffer parameters");
        }
        return false;
    }
    if (!isSupportedFormat(format)) {
        if (error) {
            *error = QStringLiteral("unsupported wl_shm format %1").arg(format);
        }
        return false;
    }
    const qsizetype size = static_cast<qsizetype>(stride) * height;
    if (m_buffer && m_format == format && m_width == width
        && m_height == height && m_stride == stride && m_size == size) {
        return true;
    }

    reset();
    const int fd = createSharedMemoryFile(size, error);
    if (fd < 0) {
        return false;
    }
    void *data = ::mmap(nullptr, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (data == MAP_FAILED) {
        if (error) {
            *error = QStringLiteral("failed to map wl_shm file: %1")
                         .arg(QString::fromLocal8Bit(std::strerror(errno)));
        }
        ::close(fd);
        return false;
    }

    wl_shm_pool *pool = wl_shm_create_pool(shm, fd, static_cast<int>(size));
    ::close(fd);
    if (!pool) {
        if (error) {
            *error = QStringLiteral("failed to create wl_shm pool");
        }
        ::munmap(data, size);
        return false;
    }
    m_buffer = wl_shm_pool_create_buffer(pool, 0, width, height, stride, format);
    wl_shm_pool_destroy(pool);
    if (!m_buffer) {
        if (error) {
            *error = QStringLiteral("failed to create wl_shm buffer");
        }
        ::munmap(data, size);
        return false;
    }

    m_data = data;
    m_size = size;
    m_format = format;
    m_width = width;
    m_height = height;
    m_stride = stride;
    return true;
#endif
}

wl_buffer *WlrootsScreencopyShmBuffer::buffer() const
{
    return m_buffer;
}

QImage WlrootsScreencopyShmBuffer::toImage(bool yInvert, QString *error) const
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_WLROOTS_SCREENCOPY
    Q_UNUSED(yInvert)
    if (error) {
        *error = QStringLiteral("wlroots screencopy support is not enabled");
    }
    return {};
#else
    if (!m_data || !m_buffer || m_width <= 0 || m_height <= 0) {
        if (error) {
            *error = QStringLiteral("wl_shm buffer is empty");
        }
        return {};
    }

    const QImage::Format imageFormat = m_format == WL_SHM_FORMAT_ARGB8888
        ? QImage::Format_ARGB32
        : QImage::Format_RGB32;
    QImage view(static_cast<const uchar *>(m_data),
                m_width,
                m_height,
                m_stride,
                imageFormat);
    if (view.isNull()) {
        if (error) {
            *error = QStringLiteral("failed to wrap wl_shm buffer");
        }
        return {};
    }
    QImage image = view.copy();
    if (yInvert) {
        image = image.mirrored(false, true);
    }
    image.setDevicePixelRatio(1.0);
    return image;
#endif
}

/**
 * 【录制】【wlroots采集】把缓冲内容复制为连续 raw BGRA 帧。
 * @param yInvert 内容是否上下翻转。
 * @param error 输出错误信息。
 * @return 连续 raw BGRA 帧。
 */
RecordingRawBgraFrame WlrootsScreencopyShmBuffer::copyBgraFrame(bool yInvert, QString *error) const
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_WLROOTS_SCREENCOPY
    Q_UNUSED(yInvert)
    if (error) {
        *error = QStringLiteral("wlroots screencopy support is not enabled");
    }
    return {};
#else
    if (!m_data || !m_buffer || m_width <= 0 || m_height <= 0) {
        if (error) {
            *error = QStringLiteral("wl_shm buffer is empty");
        }
        return {};
    }
    if (!isSupportedFormat(m_format)) {
        if (error) {
            *error = QStringLiteral("unsupported wl_shm format %1").arg(m_format);
        }
        return {};
    }

    const int rowBytes = m_width * 4;
    RecordingRawBgraFrame frame;
    frame.size = QSize(m_width, m_height);
    frame.stride = rowBytes;
    frame.bytes.resize(static_cast<qsizetype>(rowBytes) * m_height);

    // 1. 【录制】【wlroots采集】复制为连续 BGRA，避免后续 QImage 格式转换
    const auto *source = static_cast<const uchar *>(m_data);
    char *destination = frame.bytes.data();
    for (int y = 0; y < m_height; ++y) {
        const int sourceY = yInvert ? (m_height - 1 - y) : y;
        std::memcpy(destination + static_cast<qsizetype>(y) * rowBytes,
                    source + static_cast<qsizetype>(sourceY) * m_stride,
                    static_cast<size_t>(rowBytes));
    }
    return frame;
#endif
}

void WlrootsScreencopyShmBuffer::reset()
{
#ifdef HAVE_WLROOTS_SCREENCOPY
    if (m_buffer) {
        wl_buffer_destroy(m_buffer);
        m_buffer = nullptr;
    }
    if (m_data && m_size > 0) {
        ::munmap(m_data, m_size);
    }
#endif
    m_data = nullptr;
    m_size = 0;
    m_format = 0;
    m_width = 0;
    m_height = 0;
    m_stride = 0;
}

}  // namespace markshot::recording
