#include "platform/wayland/wlroots_screencopy_shm_buffer.h"
#include "debug_log.h"
#include "recording/recording_capture_memory_policy.h"

#ifdef HAVE_WLROOTS_SCREENCOPY
#include <wayland-client.h>
#endif

#include <QImage>

#include <cerrno>
#include <cstring>
#include <utility>

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

/**
 * 【录制】【映射寿命】销毁共享内存缓冲，只解除像素映射，永不发送 Wayland 请求。
 * 最后一个帧引用可能在任意线程、连接关闭之后释放，此时 wl_buffer 所属的 wl_display 可能已经失效。
 */
WlrootsScreencopyShmBuffer::~WlrootsScreencopyShmBuffer()
{
    // 1. 【录制】【映射寿命】协议对象应已由采集线程释放，残留句柄说明所有者违反约定，只能放弃
    if (m_buffer) {
        abandonWaylandBuffer("destroyed before the capture thread released it");
    }
    // 2. 【录制】【映射寿命】像素映射与 Wayland 连接无关，任意线程都可以解除
    unmapPixels();
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
    const qsizetype size = stride <= 0 ? 0 : recordingBgraFrameBytes(QSize(width, height), stride);
    if (!shm || size == 0) {
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

    // 【录制】【映射寿命】记录分发该连接事件的采集线程，之后只有它可以销毁 wl_buffer
    m_ownerThread = std::this_thread::get_id();
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

bool WlrootsScreencopyShmBuffer::matches(std::uint32_t format, int width, int height, int stride) const
{
    return m_buffer && m_format == format && m_width == width && m_height == height && m_stride == stride;
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
#if QT_VERSION >= QT_VERSION_CHECK(6, 9, 0)
        image = image.flipped(Qt::Vertical);
#else
        image = image.mirrored(false, true);
#endif
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

/**
 * 【录制】【wlroots采集】把缓冲内容包装为共享内存 raw BGRA 视图。
 * @param owner 维持缓冲生命周期的共享所有者。
 * @param yInvert 内容是否上下翻转。
 * @param error 输出错误信息。
 * @return raw BGRA 视图。
 */
RecordingRawBgraFrame WlrootsScreencopyShmBuffer::mappedBgraFrame(std::shared_ptr<const void> owner,
                                                                  bool yInvert,
                                                                  QString *error) const
{
    if (error) {
        error->clear();
    }
#ifndef HAVE_WLROOTS_SCREENCOPY
    Q_UNUSED(owner)
    Q_UNUSED(yInvert)
    if (error) {
        *error = QStringLiteral("wlroots screencopy support is not enabled");
    }
    return {};
#else
    if (!m_data || !m_buffer || m_width <= 0 || m_height <= 0 || !owner) {
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

    RecordingRawBgraFrame frame;
    frame.mappedOwner = std::move(owner);
    frame.mappedData = static_cast<const char *>(m_data);
    frame.mappedSize = m_size;
    frame.size = QSize(m_width, m_height);
    frame.stride = m_stride;
    frame.yInverted = yInvert;
    return frame;
#endif
}

/**
 * 【录制】【映射寿命】释放协议缓冲并解除像素映射，必须在创建线程且连接有效时调用。
 * @return 无返回值。
 */
void WlrootsScreencopyShmBuffer::reset()
{
    releaseWaylandBuffer();
    unmapPixels();
}

/**
 * 【录制】【映射寿命】在创建线程销毁 wl_buffer，其他线程只放弃句柄，保证不跨线程编组请求。
 * @return 无返回值。
 */
void WlrootsScreencopyShmBuffer::releaseWaylandBuffer()
{
    if (!m_buffer) {
        return;
    }
    // 1. 【录制】【映射寿命】其他线程无法确认所属连接和事件队列仍然有效，不能编组 destroy 请求
    if (std::this_thread::get_id() != m_ownerThread) {
        abandonWaylandBuffer("released outside the capture thread");
        return;
    }
#ifdef HAVE_WLROOTS_SCREENCOPY
    // 2. 【录制】【映射寿命】采集线程在 wl_display 断开之前销毁协议对象
    wl_buffer_destroy(m_buffer);
#endif
    m_buffer = nullptr;
}

/**
 * 【录制】【映射寿命】放弃无法安全销毁的 wl_buffer 句柄，不访问所属连接。
 * @param reason 放弃原因，写入错误日志。
 * @return 无返回值。
 */
void WlrootsScreencopyShmBuffer::abandonWaylandBuffer(const char *reason)
{
    markshot::debugLog("recording",
                       "【录制】【映射寿命】abandon wl_buffer without protocol request error=%s",
                       reason);
    m_buffer = nullptr;
}

/**
 * 【录制】【映射寿命】解除像素映射并清空布局参数，不涉及任何 Wayland 对象。
 * @return 无返回值。
 */
void WlrootsScreencopyShmBuffer::unmapPixels()
{
#ifdef HAVE_WLROOTS_SCREENCOPY
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
