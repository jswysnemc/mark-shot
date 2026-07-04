#include "screen_capture_pipewire_screencast.h"

#include "pipewire/pipewire_dmabuf_importer.h"

#ifdef HAVE_PIPEWIRE

#include <cerrno>
#include <sys/mman.h>

#if __has_include(<drm_fourcc.h>)
#include <drm_fourcc.h>
#elif __has_include(<libdrm/drm_fourcc.h>)
#include <libdrm/drm_fourcc.h>
#endif

namespace {

constexpr spa_video_format kSupportedRawFormats[] = {
    SPA_VIDEO_FORMAT_BGRA,
    SPA_VIDEO_FORMAT_BGRx,
    SPA_VIDEO_FORMAT_xBGR,
    SPA_VIDEO_FORMAT_RGBA,
    SPA_VIDEO_FORMAT_RGBx,
    SPA_VIDEO_FORMAT_ARGB,
    SPA_VIDEO_FORMAT_ABGR,
    SPA_VIDEO_FORMAT_xRGB,
    SPA_VIDEO_FORMAT_RGB,
    SPA_VIDEO_FORMAT_BGR,
};

constexpr std::uint64_t kDrmFormatModInvalid =
#ifdef DRM_FORMAT_MOD_INVALID
    DRM_FORMAT_MOD_INVALID;
#else
    0x00ffffffffffffffULL;
#endif

constexpr std::uint64_t kDrmFormatModLinear =
#ifdef DRM_FORMAT_MOD_LINEAR
    DRM_FORMAT_MOD_LINEAR;
#else
    0;
#endif

/**
 * 【录制】【PipeWire协商】读取 raw 像素格式每像素字节数。
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
 * 【录制】【PipeWire协商】写入基础 modifier 选择。
 * @param builder SPA POD 构造器。
 * @return 无返回值。
 */
void addReadableModifierChoice(spa_pod_builder *builder)
{
    spa_pod_frame frame;
    spa_pod_builder_push_choice(builder, &frame, SPA_CHOICE_Enum, 0);
    spa_pod_builder_long(builder, static_cast<int64_t>(kDrmFormatModInvalid));
    spa_pod_builder_long(builder, static_cast<int64_t>(kDrmFormatModInvalid));
    if (kDrmFormatModLinear != kDrmFormatModInvalid) {
        spa_pod_builder_long(builder, static_cast<int64_t>(kDrmFormatModLinear));
    }
    spa_pod_builder_pop(builder, &frame);
}

/**
 * 【录制】【PipeWire协商】构造单个 raw 格式参数。
 * @param builder SPA POD 构造器。
 * @param format PipeWire 像素格式。
 * @param withModifier 是否声明 DMA-BUF modifier 变体。
 * @return 构造完成的格式参数。
 */
const spa_pod *buildRawFormatParam(spa_pod_builder *builder,
                                   spa_video_format format,
                                   bool withModifier)
{
    spa_pod_frame frame;
    spa_pod_builder_push_object(builder, &frame, SPA_TYPE_OBJECT_Format, SPA_PARAM_EnumFormat);
    spa_pod_builder_add(builder,
                        SPA_FORMAT_mediaType, SPA_POD_Id(SPA_MEDIA_TYPE_video),
                        SPA_FORMAT_mediaSubtype, SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),
                        SPA_FORMAT_VIDEO_format, SPA_POD_Id(format),
                        0);
    if (withModifier) {
        spa_pod_builder_prop(builder,
                             SPA_FORMAT_VIDEO_modifier,
                             SPA_POD_PROP_FLAG_MANDATORY | SPA_POD_PROP_FLAG_DONT_FIXATE);
        addReadableModifierChoice(builder);
    }
    return static_cast<const spa_pod *>(spa_pod_builder_pop(builder, &frame));
}

/**
 * 【录制】【PipeWire协商】判断协商结果是否包含 modifier。
 * @param param PipeWire 格式参数。
 * @return 包含 modifier 时返回 true。
 */
bool formatHasModifier(const spa_pod *param)
{
    return spa_pod_find_prop(param, nullptr, SPA_FORMAT_VIDEO_modifier) != nullptr;
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

}  // namespace

bool PortalPipeWireScreencast::startPipeWire(int fd, QString *error)
{
    pw_init(nullptr, nullptr);

    m_loop = pw_thread_loop_new("mark-shot-screencast", nullptr);
    if (!m_loop) {
        ::close(fd);
        if (error) {
            *error = QStringLiteral("failed to create PipeWire loop");
        }
        return false;
    }

    if (pw_thread_loop_start(m_loop) < 0) {
        ::close(fd);
        if (error) {
            *error = QStringLiteral("failed to start PipeWire loop");
        }
        return false;
    }

    pw_thread_loop_lock(m_loop);
    m_context = pw_context_new(pw_thread_loop_get_loop(m_loop), nullptr, 0);
    if (!m_context) {
        pw_thread_loop_unlock(m_loop);
        ::close(fd);
        if (error) {
            *error = QStringLiteral("failed to create PipeWire context");
        }
        return false;
    }

    m_core = pw_context_connect_fd(m_context, fd, nullptr, 0);
    if (!m_core) {
        pw_thread_loop_unlock(m_loop);
        ::close(fd);
        if (error) {
            *error = QStringLiteral("failed to connect to PipeWire remote");
        }
        return false;
    }

    pw_properties *properties =
        pw_properties_new(PW_KEY_MEDIA_TYPE, "Video",
                          PW_KEY_MEDIA_CATEGORY, "Capture",
                          PW_KEY_MEDIA_ROLE, "Screen",
                          nullptr);
    if (!m_targetObject.isEmpty()) {
        pw_properties_set(properties, PW_KEY_TARGET_OBJECT, m_targetObject.toUtf8().constData());
    }

    m_stream = pw_stream_new(m_core, "mark-shot-screencast", properties);
    if (!m_stream) {
        pw_thread_loop_unlock(m_loop);
        if (error) {
            *error = QStringLiteral("failed to create PipeWire stream");
        }
        return false;
    }

    m_streamEvents = {};
    m_streamEvents.version = PW_VERSION_STREAM_EVENTS;
    m_streamEvents.state_changed = &PortalPipeWireScreencast::onStreamStateChanged;
    m_streamEvents.param_changed = &PortalPipeWireScreencast::onStreamParamChanged;
    m_streamEvents.process = &PortalPipeWireScreencast::onStreamProcess;
    pw_stream_add_listener(m_stream, &m_streamListener, &m_streamEvents, this);

    uint8_t buffer[16384];
    spa_pod_builder builder = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
    const spa_pod *params[std::size(kSupportedRawFormats) * 2];
    uint32_t paramCount = 0;
    for (spa_video_format format : kSupportedRawFormats) {
        // 1. 【录制】【PipeWire协商】优先声明 DMA-BUF，兼容只发布 modifier 格式的门户
        params[paramCount++] = buildRawFormatParam(&builder, format, true);
        // 2. 【录制】【PipeWire协商】同时声明共享内存，门户支持时避免 GPU 导入
        params[paramCount++] = buildRawFormatParam(&builder, format, false);
    }

    const pw_stream_flags flags = static_cast<pw_stream_flags>(
        PW_STREAM_FLAG_AUTOCONNECT | PW_STREAM_FLAG_MAP_BUFFERS);
    const int targetId = m_targetObject.isEmpty() ? static_cast<int>(m_nodeId) : PW_ID_ANY;
    const int result = pw_stream_connect(m_stream,
                                         PW_DIRECTION_INPUT,
                                         targetId,
                                         flags,
                                         params,
                                         paramCount);
    pw_thread_loop_unlock(m_loop);
    if (result < 0) {
        if (error) {
            *error = QStringLiteral("failed to connect PipeWire stream");
        }
        markshot::debugLog("screencast",
                           "pw-connect-failed result=%d node=%u target_id=%d target_object=%s",
                           result, m_nodeId, targetId,
                           m_targetObject.isEmpty() ? "<none>" : m_targetObject.toUtf8().constData());
        return false;
    }
    markshot::debugLog("screencast",
                       "pw-connect ok node=%u target_id=%d target_object=%s target_fps=%d params=%u flags=AUTOCONNECT|MAP_BUFFERS",
                       m_nodeId, targetId,
                       m_targetObject.isEmpty() ? "<none>" : m_targetObject.toUtf8().constData(),
                       m_targetFps,
                       paramCount);
    return true;
}

const char *PortalPipeWireScreencast::streamStateName(pw_stream_state state)
{
    switch (state) {
    case PW_STREAM_STATE_ERROR:
        return "error";
    case PW_STREAM_STATE_UNCONNECTED:
        return "unconnected";
    case PW_STREAM_STATE_CONNECTING:
        return "connecting";
    case PW_STREAM_STATE_PAUSED:
        return "paused";
    case PW_STREAM_STATE_STREAMING:
        return "streaming";
    }
    return "unknown";
}

void PortalPipeWireScreencast::onStreamStateChanged(void *data,
                                                    pw_stream_state old,
                                                    pw_stream_state state,
                                                    const char *error)
{
    auto *self = static_cast<PortalPipeWireScreencast *>(data);
    if (!self) {
        return;
    }
    markshot::debugLog("screencast", "pw-state %s -> %s%s%s",
                       streamStateName(old), streamStateName(state),
                       error ? " error=" : "", error ? error : "");
    if (state == PW_STREAM_STATE_ERROR && error) {
        QMutexLocker locker(&self->m_frameMutex);
        self->m_lastError = QString::fromUtf8(error);
        self->m_frameReady.wakeAll();
    }
}

void PortalPipeWireScreencast::onStreamParamChanged(void *data, uint32_t id, const spa_pod *param)
{
    auto *self = static_cast<PortalPipeWireScreencast *>(data);
    if (!self || id != SPA_PARAM_Format || !param) {
        return;
    }

    spa_video_info_raw info = {};
    if (spa_format_video_raw_parse(param, &info) < 0
        || info.size.width == 0 || info.size.height == 0) {
        markshot::debugLog("screencast",
                           "pw-param-changed rejected (parse failed or zero size)");
        return;
    }

    self->m_videoInfo = info;
    const bool hasModifier = formatHasModifier(param);
    const int bytesPerPixel = rawBytesPerPixel(info.format);
    if (bytesPerPixel <= 0) {
        markshot::debugLog("screencast",
                           "pw-param-changed rejected unsupported raw format=%d",
                           static_cast<int>(info.format));
        return;
    }
    const uint32_t stride = info.size.width * static_cast<uint32_t>(bytesPerPixel);
    const uint32_t size = stride * info.size.height;

    markshot::debugLog("screencast",
                       "pw-param-changed format=%d size=%ux%u bpp=%d stride=%u "
                       "framerate=%u/%u modifier=%d modifier_value=0x%llx",
                       static_cast<int>(info.format), info.size.width, info.size.height,
                       bytesPerPixel, stride, info.max_framerate.num, info.max_framerate.denom,
                       hasModifier ? 1 : 0,
                       static_cast<unsigned long long>(info.modifier));

    uint8_t buffer[1024];
    spa_pod_builder builder = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
    const spa_pod *params[1];
    const uint32_t dataTypes = hasModifier
        ? (1u << SPA_DATA_DmaBuf)
        : ((1u << SPA_DATA_MemPtr) | (1u << SPA_DATA_MemFd));
    params[0] = static_cast<const spa_pod *>(
        spa_pod_builder_add_object(&builder,
                                   SPA_TYPE_OBJECT_ParamBuffers, SPA_PARAM_Buffers,
                                   MARKSHOT_SPA_PARAM_BUFFERS_BUFFERS, SPA_POD_CHOICE_RANGE_Int(8, 2, 16),
                                   MARKSHOT_SPA_PARAM_BUFFERS_BLOCKS, SPA_POD_Int(1),
                                   MARKSHOT_SPA_PARAM_BUFFERS_SIZE, SPA_POD_Int(size),
                                   MARKSHOT_SPA_PARAM_BUFFERS_STRIDE, SPA_POD_Int(stride),
                                   MARKSHOT_SPA_PARAM_BUFFERS_DATA_TYPE,
                                   SPA_POD_CHOICE_FLAGS_Int(dataTypes)));
    pw_stream_update_params(self->m_stream, params, 1);
}

void PortalPipeWireScreencast::onStreamProcess(void *data)
{
    auto *self = static_cast<PortalPipeWireScreencast *>(data);
    if (!self || !self->m_stream) {
        return;
    }

    pw_buffer *buffer = nullptr;
    while (pw_buffer *next = pw_stream_dequeue_buffer(self->m_stream)) {
        if (buffer) {
            pw_stream_queue_buffer(self->m_stream, buffer);
        }
        buffer = next;
    }
    if (!buffer) {
        return;
    }

    const qint64 frameTimeMs = QDateTime::currentMSecsSinceEpoch();
    if (self->shouldDropIncomingFrame(frameTimeMs)) {
        pw_stream_queue_buffer(self->m_stream, buffer);
        return;
    }

    if (self->m_rawStreamMode) {
        // 1. 写出队列繁忙时直接归还 buffer，避免为将被丢弃的帧支付拷贝和 GPU 读回成本
        if (self->m_rawBackpressure.load(std::memory_order_relaxed)) {
            pw_stream_queue_buffer(self->m_stream, buffer);
            return;
        }
        QString frameError;
        PipeWireScreencastRawFrame frame;
        if (self->rawFrameFromBuffer(buffer, &frame, &frameError)) {
            {
                QMutexLocker locker(&self->m_frameMutex);
                self->m_latestFrameTimeMs = frameTimeMs;
                self->m_streamGeometry = frame.streamGeometry;
                self->m_frameCount += 1;
            }
            if (self->m_frameCount == 1 || self->m_frameCount % 100 == 0) {
                markshot::debugLog("screencast",
                                   "【录制】【PipeWire流回调】frame #%d raw=%dx%d stream_geom=%d,%d %dx%d",
                                   self->m_frameCount,
                                   frame.size.width(),
                                   frame.size.height(),
                                   frame.streamGeometry.x(),
                                   frame.streamGeometry.y(),
                                   frame.streamGeometry.width(),
                                   frame.streamGeometry.height());
            }
            if (self->m_rawFrameCallback) {
                self->m_rawFrameCallback(std::move(frame));
            }
        } else if (!frameError.isEmpty()) {
            {
                QMutexLocker locker(&self->m_frameMutex);
                self->m_lastError = frameError;
                self->m_frameErrorCount += 1;
            }
            if (self->m_rawErrorCallback) {
                self->m_rawErrorCallback(frameError);
            }
        }
        pw_stream_queue_buffer(self->m_stream, buffer);
        return;
    }

    QString imageError;
    QImage image = self->imageFromBuffer(buffer, &imageError);
    if (!image.isNull()) {
        QMutexLocker locker(&self->m_frameMutex);
        self->m_latestFrame = std::move(image);
        self->m_latestFrameTimeMs = frameTimeMs;
        self->m_streamGeometry = streamGeometryFromProperties(self->m_streamProperties, self->m_latestFrame.size());
        self->m_frameReady.wakeAll();
        self->m_frameCount += 1;
        if (self->m_frameCount == 1 || self->m_frameCount % 100 == 0) {
            markshot::debugLog("screencast",
                               "pw-frame #%d image=%dx%d stream_geom=%d,%d %dx%d",
                               self->m_frameCount,
                               self->m_latestFrame.width(), self->m_latestFrame.height(),
                               self->m_streamGeometry.x(), self->m_streamGeometry.y(),
                               self->m_streamGeometry.width(), self->m_streamGeometry.height());
        }
    } else if (!imageError.isEmpty()) {
        QMutexLocker locker(&self->m_frameMutex);
        self->m_lastError = imageError;
        self->m_frameReady.wakeAll();
        self->m_frameErrorCount += 1;
        if (self->m_frameErrorCount == 1 || self->m_frameErrorCount % 100 == 0) {
            markshot::debugLog("screencast", "pw-frame-error %s",
                               imageError.toUtf8().constData());
        }
    }
    pw_stream_queue_buffer(self->m_stream, buffer);
}

bool PortalPipeWireScreencast::shouldDropIncomingFrame(qint64 frameTimeMs)
{
    if (m_minFrameIntervalUs <= 0) {
        return false;
    }

    QMutexLocker locker(&m_frameMutex);
    if (m_latestFrameTimeMs <= 0) {
        return false;
    }
    if ((frameTimeMs - m_latestFrameTimeMs) * 1000 >= m_minFrameIntervalUs) {
        return false;
    }
    ++m_droppedFrameCount;
    if (m_droppedFrameCount == 1 || m_droppedFrameCount % 200 == 0) {
        markshot::debugLog("screencast",
                           "【录制】【PipeWire限帧】dropped=%d target_fps=%d",
                           m_droppedFrameCount,
                           m_targetFps);
    }
    return true;
}

QImage PortalPipeWireScreencast::imageFromBuffer(pw_buffer *pipewireBuffer, QString *error)
{
    if (!pipewireBuffer || !pipewireBuffer->buffer || pipewireBuffer->buffer->n_datas == 0) {
        if (error) {
            *error = QStringLiteral("PipeWire delivered an empty buffer");
        }
        return {};
    }
    const spa_buffer *spaBuffer = pipewireBuffer->buffer;
    const spa_data &data = spaBuffer->datas[0];
    if (!data.chunk) {
        if (error) {
            *error = QStringLiteral("PipeWire buffer chunk is missing (data type %1)")
                         .arg(static_cast<uint>(data.type));
        }
        return {};
    }
    if (data.type == SPA_DATA_DmaBuf) {
        if (!m_dmaBufImporter) {
            m_dmaBufImporter = std::make_unique<markshot::PipeWireDmaBufImporter>();
        }
        return m_dmaBufImporter->importBuffer(spaBuffer, m_videoInfo, error);
    }

    const int width = static_cast<int>(m_videoInfo.size.width);
    const int height = static_cast<int>(m_videoInfo.size.height);
    if (width <= 0 || height <= 0) {
        if (error) {
            *error = QStringLiteral("PipeWire frame size is invalid");
        }
        return {};
    }

    const int bytesPerPixel = rawBytesPerPixel(m_videoInfo.format);
    if (bytesPerPixel <= 0) {
        if (error) {
            *error = QStringLiteral("unsupported PipeWire video format %1")
                         .arg(static_cast<int>(m_videoInfo.format));
        }
        return {};
    }
    const int stride = data.chunk->stride != 0
        ? std::abs(static_cast<int>(data.chunk->stride))
        : width * bytesPerPixel;
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
            return {};
        }
        base = static_cast<const uchar *>(mappedAddress);
    }
    if (!base) {
        if (error) {
            *error = QStringLiteral("PipeWire buffer is not CPU-mappable (data type %1 flags=0x%2)")
                         .arg(static_cast<uint>(data.type))
                         .arg(static_cast<uint>(data.flags), 0, 16);
        }
        return {};
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
        return {};
    }

    QImage image;
    switch (m_videoInfo.format) {
    case SPA_VIDEO_FORMAT_BGRA:
        image = QImage(source, width, height, stride, QImage::Format_ARGB32).copy();
        break;
    case SPA_VIDEO_FORMAT_BGRx:
        image = QImage(source, width, height, stride, QImage::Format_RGB32).copy();
        break;
    case SPA_VIDEO_FORMAT_RGBA:
    case SPA_VIDEO_FORMAT_RGBx:
    case SPA_VIDEO_FORMAT_ARGB:
    case SPA_VIDEO_FORMAT_ABGR:
    case SPA_VIDEO_FORMAT_xRGB:
    case SPA_VIDEO_FORMAT_xBGR:
        image = convertFourByteFrame(source, width, height, stride, m_videoInfo.format);
        break;
    case SPA_VIDEO_FORMAT_RGB:
    case SPA_VIDEO_FORMAT_BGR:
        image = convertThreeByteFrame(source, width, height, stride, m_videoInfo.format);
        break;
    default:
        if (error) {
            *error = QStringLiteral("unsupported PipeWire video format %1")
                         .arg(static_cast<int>(m_videoInfo.format));
        }
        break;
    }
    if (mappedAddress) {
        ::munmap(mappedAddress, mappedSize);
    }
    return image;
}

QImage PortalPipeWireScreencast::convertFourByteFrame(const uchar *source,
                                                      int width,
                                                      int height,
                                                      int stride,
                                                      spa_video_format format)
{
    QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
    for (int y = 0; y < height; ++y) {
        const uchar *src = source + y * stride;
        QRgb *dst = reinterpret_cast<QRgb *>(image.scanLine(y));
        for (int x = 0; x < width; ++x) {
            const uchar *px = src + x * 4;
            int r = 0;
            int g = 0;
            int b = 0;
            int a = 255;
            if (format == SPA_VIDEO_FORMAT_ARGB) {
                a = px[0];
                r = px[1];
                g = px[2];
                b = px[3];
            } else if (format == SPA_VIDEO_FORMAT_ABGR) {
                a = px[0];
                b = px[1];
                g = px[2];
                r = px[3];
            } else if (format == SPA_VIDEO_FORMAT_xRGB) {
                r = px[1];
                g = px[2];
                b = px[3];
            } else if (format == SPA_VIDEO_FORMAT_xBGR) {
                b = px[1];
                g = px[2];
                r = px[3];
            } else {
                r = px[0];
                g = px[1];
                b = px[2];
                if (format == SPA_VIDEO_FORMAT_RGBA) {
                    a = px[3];
                }
            }
            dst[x] = qRgba(std::min(r, a), std::min(g, a), std::min(b, a), a);
        }
    }
    return image;
}

QImage PortalPipeWireScreencast::convertThreeByteFrame(const uchar *source,
                                                       int width,
                                                       int height,
                                                       int stride,
                                                       spa_video_format format)
{
    QImage image(width, height, QImage::Format_RGB32);
    for (int y = 0; y < height; ++y) {
        const uchar *src = source + y * stride;
        QRgb *dst = reinterpret_cast<QRgb *>(image.scanLine(y));
        for (int x = 0; x < width; ++x) {
            const uchar *px = src + x * 3;
            const int r = format == SPA_VIDEO_FORMAT_BGR ? px[2] : px[0];
            const int g = px[1];
            const int b = format == SPA_VIDEO_FORMAT_BGR ? px[0] : px[2];
            dst[x] = qRgb(r, g, b);
        }
    }
    return image;
}

#endif
