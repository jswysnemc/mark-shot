#include "recording/libav/libav_video_scaler.h"

#include <QtTest/QtTest>

#include <QByteArray>

extern "C" {
#include <libavutil/frame.h>
#include <libavutil/imgutils.h>
}

namespace {

constexpr int kWidth = 640;
constexpr int kHeight = 480;

/**
 * 生成带有横向与纵向梯度的 BGRA 测试帧。
 * @param width 帧宽度。
 * @param height 帧高度。
 * @return BGRA 字节。
 */
QByteArray makeGradientFrame(int width, int height)
{
    QByteArray bytes(static_cast<qsizetype>(width) * height * 4, '\0');
    char *data = bytes.data();
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            const qsizetype offset = (static_cast<qsizetype>(y) * width + x) * 4;
            data[offset + 0] = static_cast<char>((x * 7 + y * 3) & 0xFF);
            data[offset + 1] = static_cast<char>((x * 3 + y * 11) & 0xFF);
            data[offset + 2] = static_cast<char>((x + y * 5) & 0xFF);
            data[offset + 3] = static_cast<char>(0xFF);
        }
    }
    return bytes;
}

/**
 * 分配一个可写的目标编码帧。
 * @param format 目标像素格式。
 * @return 编码帧，调用方负责释放。
 */
AVFrame *makeTargetFrame(AVPixelFormat format)
{
    AVFrame *frame = av_frame_alloc();
    frame->format = format;
    frame->width = kWidth;
    frame->height = kHeight;
    av_frame_get_buffer(frame, 32);
    return frame;
}

/**
 * 把编码帧的全部平面拷贝为连续字节，便于逐字节比较。
 * @param frame 编码帧。
 * @return 平面字节。
 */
QByteArray planeBytes(const AVFrame *frame)
{
    QByteArray bytes;
    for (int plane = 0; plane < 4 && frame->data[plane]; ++plane) {
        const int height = plane == 0 ? frame->height : (frame->height + 1) / 2;
        for (int y = 0; y < height; ++y) {
            bytes.append(reinterpret_cast<const char *>(frame->data[plane])
                             + static_cast<qsizetype>(frame->linesize[plane]) * y,
                         frame->linesize[plane]);
        }
    }
    return bytes;
}

/**
 * 用指定并行度转换一帧并返回结果字节。
 * @param source BGRA 源数据。
 * @param threads 并行切片数量上限。
 * @param yInverted 源数据是否自底向上。
 * @param sliceCount 输出实际切片数量。
 * @return 转换后的平面字节。
 */
QByteArray convertWithThreads(const QByteArray &source, int threads, bool yInverted, int *sliceCount)
{
    qputenv("MARK_SHOT_RECORDING_SCALE_THREADS", QByteArray::number(threads));
    markshot::recording::LibavVideoScaler scaler;
    QString error;
    if (!scaler.prepare(QSize(kWidth, kHeight), QSize(kWidth, kHeight), AV_PIX_FMT_YUV420P, &error)) {
        return {};
    }
    if (sliceCount) {
        *sliceCount = scaler.sliceCount();
    }

    AVFrame *frame = makeTargetFrame(AV_PIX_FMT_YUV420P);
    const bool ok = scaler.convert(source.constData(), kWidth * 4, kHeight, yInverted, frame, &error);
    QByteArray result;
    if (ok) {
        result = planeBytes(frame);
    }
    av_frame_free(&frame);
    return result;
}

}  // namespace

class LibavVideoScalerTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证并行切片转换与单线程转换结果完全一致。
     * @return 无返回值。
     */
    void parallelSlicesMatchSingleThread()
    {
        const QByteArray source = makeGradientFrame(kWidth, kHeight);

        int singleSlices = 0;
        const QByteArray single = convertWithThreads(source, 1, false, &singleSlices);
        QCOMPARE(singleSlices, 1);
        QVERIFY(!single.isEmpty());

        int parallelSlices = 0;
        const QByteArray parallel = convertWithThreads(source, 4, false, &parallelSlices);
        QVERIFY(parallelSlices > 1);
        QCOMPARE(parallel, single);
    }

    /**
     * 验证自底向上的帧在并行切片下同样得到一致结果。
     * @return 无返回值。
     */
    void invertedFramesMatchSingleThread()
    {
        const QByteArray source = makeGradientFrame(kWidth, kHeight);

        const QByteArray single = convertWithThreads(source, 1, true, nullptr);
        QVERIFY(!single.isEmpty());

        int parallelSlices = 0;
        const QByteArray parallel = convertWithThreads(source, 4, true, &parallelSlices);
        QVERIFY(parallelSlices > 1);
        QCOMPARE(parallel, single);
    }

    /**
     * 验证自底向上的帧确实按行翻转写出。
     * @return 无返回值。
     */
    void invertedFrameFlipsRows()
    {
        const QByteArray source = makeGradientFrame(kWidth, kHeight);
        const QByteArray upright = convertWithThreads(source, 4, false, nullptr);
        const QByteArray inverted = convertWithThreads(source, 4, true, nullptr);
        QVERIFY(!upright.isEmpty());
        QVERIFY(upright != inverted);
    }

    /**
     * 清理测试期间设置的并行度环境变量。
     * @return 无返回值。
     */
    void cleanupTestCase()
    {
        qunsetenv("MARK_SHOT_RECORDING_SCALE_THREADS");
    }
};

QTEST_APPLESS_MAIN(LibavVideoScalerTest)

#include "libav_video_scaler_test.moc"
