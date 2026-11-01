#include "recording/libav/libav_gif_frame_cache.h"

#include <QtTest/QtTest>

using namespace markshot::recording;

class LibavGifFrameCacheTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【录制测试】【GIF复用】验证引用共享、原帧释放和时间属性独立，无参数和返回值。 */
    void sharesPixelsWithoutOwningInputFrame()
    {
        LibavGifFrameCache cache;
        QVERIFY(!cache.find(1));
        AVFrame *input = av_frame_alloc();
        QVERIFY(input);
        input->format = AV_PIX_FMT_PAL8;
        input->width = 32;
        input->height = 24;
        input->pts = 10;
        QVERIFY(av_frame_get_buffer(input, 32) >= 0);
        input->data[0][0] = 42;
        cache.remember(input, 1);
        AVFrame *cached = cache.find(1);
        QVERIFY(cached);
        QCOMPARE(cached->data[0], input->data[0]);
        QCOMPARE(cached->data[1], input->data[1]);
        cached->pts = 20;
        QCOMPARE(input->pts, 10);
        QVERIFY(!cache.find(2));
        QVERIFY(!cache.find(0));
        av_frame_free(&input);
        QCOMPARE(cached->data[0][0], 42);
        cache.clear();
        QVERIFY(!cache.find(1));
    }

    /** 【录制测试】【GIF复用】验证未知序号和无引用载荷不会建立缓存，无参数和返回值。 */
    void ignoresUnknownOrUnownedFrames()
    {
        LibavGifFrameCache cache;
        AVFrame *input = av_frame_alloc();
        QVERIFY(input);
        input->format = AV_PIX_FMT_RGB8;
        input->width = 32;
        input->height = 24;
        cache.remember(input, 2);
        QVERIFY(!cache.find(2));
        QVERIFY(av_frame_get_buffer(input, 32) >= 0);
        cache.remember(input, 1);
        QVERIFY(!cache.find(1));
        av_frame_unref(input);
        input->format = AV_PIX_FMT_PAL8;
        input->width = 32;
        input->height = 24;
        QVERIFY(av_frame_get_buffer(input, 32) >= 0);
        for (qint64 sequence : {1, 0, -1, 2}) {
            cache.remember(input, sequence);
            QCOMPARE(cache.find(sequence) != nullptr, sequence > 0);
            QVERIFY(!cache.find(sequence + 1));
        }
        cache.remember(nullptr, 2);
        QVERIFY(!cache.find(2));
        av_frame_free(&input);
    }
};

QTEST_APPLESS_MAIN(LibavGifFrameCacheTest)
#include "libav_gif_frame_cache_test.moc"
