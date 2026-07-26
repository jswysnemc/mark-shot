#include "recording/libav/libav_video_scaler.h"

#ifdef HAVE_LIBAV_RECORDING

#include <QtGlobal>

#include <algorithm>
#include <condition_variable>
#include <cstdint>
#include <functional>
#include <mutex>
#include <thread>

extern "C" {
#include <libavutil/pixdesc.h>
}

namespace markshot::recording {
namespace {

/**
 * 向调用方写入错误文本。
 * @param error 输出错误信息。
 * @param text 错误文本。
 * @return 固定返回 false，便于调用处直接返回。
 */
bool failWith(QString *error, const QString &text)
{
    if (error) {
        *error = text;
    }
    return false;
}

/**
 * 【录制】【像素转换】读取转换并行度上限。
 * @return 允许的最大切片数量。
 */
int maximumSliceCount()
{
    // 环境变量便于排查并行转换引入的问题，设为 1 即退回单线程
    bool ok = false;
    const int configured = qEnvironmentVariableIntValue("MARK_SHOT_RECORDING_SCALE_THREADS", &ok);
    if (ok && configured > 0) {
        return std::min(configured, 8);
    }
    const int cores = static_cast<int>(std::thread::hardware_concurrency());
    if (cores <= 2) {
        return 1;
    }
    // 转换受内存带宽限制，切片过多只会与编码器争抢 CPU
    return std::clamp(cores / 2, 2, 4);
}

}  // namespace

/**
 * 【录制】【像素转换】切片转换用的常驻线程池。
 *
 * 每帧转换都要唤醒固定数量的工作线程，使用常驻线程避免逐帧创建线程的开销。
 */
class LibavScalerThreadPool final {
public:
    /**
     * 创建常驻线程池。
     * @param workerCount 除调用线程外的工作线程数量。
     */
    explicit LibavScalerThreadPool(int workerCount)
    {
        m_workers.reserve(static_cast<size_t>(std::max(0, workerCount)));
        for (int i = 0; i < workerCount; ++i) {
            m_workers.emplace_back([this, index = i + 1] { workerLoop(index); });
        }
    }

    /**
     * 停止全部工作线程。
     */
    ~LibavScalerThreadPool()
    {
        {
            std::lock_guard<std::mutex> lock(m_mutex);
            m_stopped = true;
        }
        m_startCondition.notify_all();
        for (std::thread &worker : m_workers) {
            if (worker.joinable()) {
                worker.join();
            }
        }
    }

    LibavScalerThreadPool(const LibavScalerThreadPool &) = delete;
    LibavScalerThreadPool &operator=(const LibavScalerThreadPool &) = delete;

    /**
     * 并行执行编号任务并等待全部完成。
     * @param task 任务体，参数为任务编号。
     * @param count 任务数量，编号 0 由调用线程执行。
     * @return 无返回值。
     */
    void run(const std::function<void(int)> &task, int count)
    {
        if (count <= 0) {
            return;
        }
        if (count == 1 || m_workers.empty()) {
            for (int i = 0; i < count; ++i) {
                task(i);
            }
            return;
        }

        // 1. 发布任务并唤醒需要参与的工作线程
        {
            std::lock_guard<std::mutex> lock(m_mutex);
            m_task = &task;
            m_taskCount = count;
            m_pending = count - 1;
            ++m_generation;
        }
        m_startCondition.notify_all();

        // 2. 调用线程承担编号 0，避免空等
        task(0);

        // 3. 等待其余切片完成后清理任务指针
        std::unique_lock<std::mutex> lock(m_mutex);
        m_doneCondition.wait(lock, [this] { return m_pending == 0; });
        m_task = nullptr;
        m_taskCount = 0;
    }

private:
    /**
     * 工作线程主循环。
     * @param index 该线程负责的任务编号。
     * @return 无返回值。
     */
    void workerLoop(int index)
    {
        uint64_t seenGeneration = 0;
        for (;;) {
            const std::function<void(int)> *task = nullptr;
            {
                std::unique_lock<std::mutex> lock(m_mutex);
                m_startCondition.wait(lock, [this, &seenGeneration] {
                    return m_stopped || m_generation != seenGeneration;
                });
                if (m_stopped) {
                    return;
                }
                seenGeneration = m_generation;
                if (index < m_taskCount) {
                    task = m_task;
                }
            }

            if (!task) {
                continue;
            }
            (*task)(index);

            bool finished = false;
            {
                std::lock_guard<std::mutex> lock(m_mutex);
                finished = (--m_pending == 0);
            }
            if (finished) {
                m_doneCondition.notify_one();
            }
        }
    }

    std::vector<std::thread> m_workers;
    std::mutex m_mutex;
    std::condition_variable m_startCondition;
    std::condition_variable m_doneCondition;
    const std::function<void(int)> *m_task = nullptr;
    uint64_t m_generation = 0;
    int m_taskCount = 0;
    int m_pending = 0;
    bool m_stopped = false;
};

LibavVideoScaler::LibavVideoScaler() = default;

LibavVideoScaler::~LibavVideoScaler()
{
    reset();
}

bool LibavVideoScaler::prepare(QSize sourceSize,
                               QSize targetSize,
                               AVPixelFormat targetFormat,
                               QString *error)
{
    if (error) {
        error->clear();
    }
    if (sourceSize.isEmpty() || targetSize.isEmpty()) {
        return failWith(error, QStringLiteral("Cannot prepare libav scaler with an empty frame size"));
    }

    reset();
    m_sourceSize = sourceSize;
    m_targetSize = targetSize;
    m_targetFormat = targetFormat;
    return buildSlices(error);
}

bool LibavVideoScaler::buildSlices(QString *error)
{
    const AVPixFmtDescriptor *descriptor = av_pix_fmt_desc_get(m_targetFormat);
    const int verticalShift = descriptor ? descriptor->log2_chroma_h : 1;
    const int alignment = 1 << std::max(0, verticalShift);

    // 1. 只有等尺寸转换才切片，缩放需要垂直方向的整帧上下文
    const bool unscaled = m_sourceSize == m_targetSize;
    int sliceCount = 1;
    if (unscaled) {
        const int maxByHeight = std::max(1, m_targetSize.height() / 128);
        sliceCount = std::clamp(std::min(maximumSliceCount(), maxByHeight), 1, 8);
    }

    // 2. 按色度子采样对齐切分目标高度
    const int height = m_targetSize.height();
    int baseHeight = height / sliceCount;
    baseHeight -= baseHeight % alignment;
    if (baseHeight <= 0) {
        sliceCount = 1;
        baseHeight = height;
    }

    m_slices.clear();
    m_slices.reserve(static_cast<size_t>(sliceCount));
    for (int index = 0; index < sliceCount; ++index) {
        const int startY = index * baseHeight;
        const int sliceHeight = index == sliceCount - 1 ? height - startY : baseHeight;
        if (sliceHeight <= 0) {
            break;
        }

        SwsContext *context = sws_getContext(m_sourceSize.width(),
                                             sliceHeight,
                                             AV_PIX_FMT_BGRA,
                                             m_targetSize.width(),
                                             sliceHeight,
                                             m_targetFormat,
                                             unscaled ? SWS_POINT : SWS_FAST_BILINEAR,
                                             nullptr,
                                             nullptr,
                                             nullptr);
        if (!context) {
            reset();
            return failWith(error, QStringLiteral("Failed to create libav scale context"));
        }
        m_slices.push_back({context, startY, sliceHeight});
    }

    if (m_slices.empty()) {
        return failWith(error, QStringLiteral("Failed to create libav scale context"));
    }

    // 3. 切片多于一个时才创建工作线程
    if (m_slices.size() > 1) {
        m_pool = std::make_unique<LibavScalerThreadPool>(static_cast<int>(m_slices.size()) - 1);
    }
    return true;
}

bool LibavVideoScaler::convert(const char *source,
                               int stride,
                               int sourceDataHeight,
                               bool yInverted,
                               AVFrame *destination,
                               QString *error)
{
    if (error) {
        error->clear();
    }
    if (!source || !destination || m_slices.empty()) {
        return failWith(error, QStringLiteral("libav scaler is not prepared"));
    }

    // 1. 自底向上的帧从末行起以负跨度读取，交给 sws 完成翻转
    const char *first = source;
    int effectiveStride = stride > 0 ? stride : m_sourceSize.width() * 4;
    if (yInverted) {
        const int dataHeight = sourceDataHeight > 0 ? sourceDataHeight : m_sourceSize.height();
        first += static_cast<qsizetype>(effectiveStride) * (dataHeight - 1);
        effectiveStride = -effectiveStride;
    }

    // 2. 单切片直接转换，多切片交给常驻线程池并行
    if (m_slices.size() == 1 || !m_pool) {
        convertSlice(m_slices.front(), first, effectiveStride, destination);
        return true;
    }

    m_pool->run(
        [this, first, effectiveStride, destination](int index) {
            convertSlice(m_slices.at(static_cast<size_t>(index)), first, effectiveStride, destination);
        },
        static_cast<int>(m_slices.size()));
    return true;
}

void LibavVideoScaler::convertSlice(const SliceContext &slice,
                                    const char *source,
                                    int stride,
                                    AVFrame *destination) const
{
    const AVPixFmtDescriptor *descriptor = av_pix_fmt_desc_get(m_targetFormat);
    const int verticalShift = descriptor ? descriptor->log2_chroma_h : 1;

    uint8_t *destinationData[4] = {nullptr, nullptr, nullptr, nullptr};
    int destinationLineSize[4] = {0, 0, 0, 0};
    for (int plane = 0; plane < 4; ++plane) {
        destinationLineSize[plane] = destination->linesize[plane];
        if (!destination->data[plane]) {
            continue;
        }
        // 色度平面按子采样比例折算切片起始行
        const int shift = (plane == 1 || plane == 2) ? verticalShift : 0;
        destinationData[plane] = destination->data[plane]
            + static_cast<qsizetype>(destination->linesize[plane]) * (slice.startY >> shift);
    }

    const uint8_t *sourceData[4] = {
        reinterpret_cast<const uint8_t *>(source + static_cast<qsizetype>(stride) * slice.startY),
        nullptr,
        nullptr,
        nullptr,
    };
    const int sourceLineSize[4] = {stride, 0, 0, 0};
    sws_scale(slice.context,
              sourceData,
              sourceLineSize,
              0,
              slice.height,
              destinationData,
              destinationLineSize);
}

void LibavVideoScaler::reset()
{
    m_pool.reset();
    for (SliceContext &slice : m_slices) {
        if (slice.context) {
            sws_freeContext(slice.context);
            slice.context = nullptr;
        }
    }
    m_slices.clear();
}

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
