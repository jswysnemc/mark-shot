#include "screen_capture_pipewire_screencast.h"

#include <QTemporaryDir>
#include <QTemporaryFile>
#include <QtTest/QtTest>

#include <array>
#include <chrono>
#include <thread>

namespace {
enum class Scenario { Bgra, MemFd, Converted, DmaBuf, Maintenance };
Scenario g_scenario = Scenario::Bgra;
bool g_quotaRejected = false;
bool g_resumed = false;
QString g_quotaError;
int g_imageReads = 0;
int g_readsAtLimit = 0;
bool g_directBrokenAtLimit = false;
bool g_leakedInputMapping = false;
bool g_pixelsPreserved = true;
std::atomic_bool g_loopBlocked{false};
std::atomic_bool g_releaseLoop{false};
std::atomic_bool *g_pending = nullptr;
markshot::recording::RecordingBgraBufferPool *g_pool = nullptr;

/** 断言提前返回时也解除测试循环阻塞，避免清理线程挂起。 */
struct ReleaseLoop {
    ~ReleaseLoop() { g_releaseLoop = true; }
};
}  // namespace

/** 测试不向真实桌面或门户注册应用，无参数和返回值。 */
void registerHostPortalApplication() {}

/** 用固定像素模拟非 BGRA 来源，buffer 不使用，error 为错误输出，返回源图像。 */
QImage PortalPipeWireScreencast::imageFromBuffer(pw_buffer *buffer, QString *error)
{
    Q_UNUSED(buffer)
    Q_UNUSED(error)
    ++g_imageReads;
    QImage image(16, 16, QImage::Format_RGB888);
    image.fill(Qt::blue);
    return image;
}

/** 替换门户协商，不替换 raw 读取、缓冲池、回收或停止实现；返回测试会话是否建立。 */
bool PortalPipeWireScreencast::startWithDbusPortal(bool includeCursor, QString *error)
{
    Q_UNUSED(includeCursor)
    if (g_scenario == Scenario::Maintenance) {
        m_loop = pw_thread_loop_new("recording-memory-test", nullptr);
        if (!m_loop || pw_thread_loop_start(m_loop) < 0) return false;
        g_pending = &m_rawReclaimPending;
        g_pool = &m_rawBufferPool;
        m_rawBufferPool.tryAcquire(1024, 0);
        m_rawBufferPool.reclaimIdle(1);
        return pw_loop_invoke(pw_thread_loop_get_loop(m_loop),
            [](spa_loop *, bool, uint32_t, const void *, size_t, void *) {
                g_loopBlocked = true;
                while (!g_releaseLoop) std::this_thread::sleep_for(std::chrono::milliseconds(1));
                return 0;
            }, SPA_ID_INVALID, nullptr, 0, false, nullptr) >= 0;
    }

    m_videoInfo.size.width = 16;
    m_videoInfo.size.height = 16;
    m_videoInfo.format = g_scenario == Scenario::Converted ? SPA_VIDEO_FORMAT_RGB : SPA_VIDEO_FORMAT_BGRA;
    m_negotiatedStreamGeometry = QRect(0, 0, 16, 16);
    m_rawRequestedGeometry = {};
    QByteArray pixels(16 * 16 * 4, char(0x45));
    QByteArray expected = pixels;
    if (g_scenario == Scenario::Converted || g_scenario == Scenario::DmaBuf) {
        QImage blue(16, 16, QImage::Format_ARGB32);
        blue.fill(Qt::blue);
        expected = QByteArray(reinterpret_cast<const char *>(blue.constBits()), blue.sizeInBytes());
    }
    QTemporaryFile input;
    spa_chunk chunk{};
    chunk.stride = g_scenario == Scenario::Converted ? 48 : 64;
    chunk.size = chunk.stride * 16;
    spa_data data{};
    data.type = g_scenario == Scenario::DmaBuf ? SPA_DATA_DmaBuf : SPA_DATA_MemPtr;
    data.data = pixels.data();
    data.maxsize = pixels.size();
    data.chunk = &chunk;
    if (g_scenario == Scenario::MemFd) {
        if (!input.open() || input.write(pixels) != pixels.size() || !input.flush()) return false;
        data.type = SPA_DATA_MemFd;
        data.fd = input.handle();
        data.data = nullptr;
    }
    spa_buffer spaBuffer{};
    spaBuffer.n_datas = 1;
    spaBuffer.datas = &data;
    pw_buffer buffer{};
    buffer.buffer = &spaBuffer;
    std::array<PipeWireScreencastRawFrame, 4> held;
    for (auto &frame : held) {
        if (g_scenario == Scenario::DmaBuf) {
            auto *slot = m_rawBufferPool.tryAcquire(16 * 16 * 4);
            if (!slot) return false;
            frame.bgra = *slot;
        } else if (!rawFrameFromBuffer(&buffer, &frame, error)) {
            return false;
        } else {
            g_pixelsPreserved = g_pixelsPreserved && frame.bgra == expected;
        }
    }
    g_readsAtLimit = g_imageReads;
    PipeWireScreencastRawFrame excess;
    g_quotaRejected = !rawFrameFromBuffer(&buffer, &excess, &g_quotaError);
    g_directBrokenAtLimit = m_rawDmaBufDirectReadBroken;
    held[0] = {};
    g_resumed = rawFrameFromBuffer(&buffer, &excess, error);
    g_pixelsPreserved = g_pixelsPreserved && excess.bgra == expected;
    if (g_scenario == Scenario::MemFd) {
        QFile mappings(QStringLiteral("/proc/self/maps"));
        if (!mappings.open(QIODevice::ReadOnly)) return false;
        g_leakedInputMapping = mappings.readAll().contains(input.fileName().toUtf8());
    }
    return true;
}

class PipeWireRecordingMemoryTest final : public QObject {
    Q_OBJECT
private slots:
    /** 初始化本地 PipeWire 事件循环库，不连接 daemon，无参数和返回值。 */
    void initTestCase() { pw_init(nullptr, nullptr); }
    /** 所有测试会话已停止后清理库，无参数和返回值。 */
    void cleanupTestCase() { pw_deinit(); }

    /** 覆盖直接 BGRA、颜色转换和 DMA-BUF 准入路径，无参数和返回值。 */
    void quotaIsBackpressureNotFailure_data()
    {
        QTest::addColumn<bool>("converted");
        QTest::addColumn<bool>("dmabuf");
        QTest::addColumn<bool>("memfd");
        QTest::newRow("bgra") << false << false << false;
        QTest::newRow("memfd") << false << false << true;
        QTest::newRow("color-conversion") << true << false << false;
        QTest::newRow("dmabuf-before-import") << true << true << false;
    }

    /** 超限帧不进入转换、不返回故障；归还槽位后能够继续读取，无参数和返回值。 */
    void quotaIsBackpressureNotFailure()
    {
        QFETCH(bool, converted);
        QFETCH(bool, dmabuf);
        QFETCH(bool, memfd);
        g_scenario = memfd ? Scenario::MemFd : dmabuf ? Scenario::DmaBuf
            : converted ? Scenario::Converted : Scenario::Bgra;
        g_imageReads = 0;
        g_quotaError.clear();
        g_leakedInputMapping = false;
        g_pixelsPreserved = true;
        PortalPipeWireScreencast capture;
        CaptureRequest request;
        request.allowInteractivePortal = true;
        QString error;
        QVERIFY2(capture.startRawStream(request, [](PipeWireScreencastRawFrame) {}, {}, &error), qPrintable(error));
        QVERIFY(g_quotaRejected);
        QVERIFY(g_quotaError.isEmpty());
        QVERIFY(g_resumed);
        QVERIFY(!g_leakedInputMapping);
        QVERIFY(g_pixelsPreserved);
        QVERIFY(!g_directBrokenAtLimit);
        QCOMPARE(g_readsAtLimit, converted && !dmabuf ? 4 : 0);
        QCOMPARE(g_imageReads, dmabuf ? 1 : converted ? 5 : 0);
    }

    /** 回收请求在繁忙循环前合并，界面调用立即返回，循环恢复后释放空闲缓冲，无参数和返回值。 */
    void maintenanceIsAsynchronousAndCoalesced()
    {
        g_scenario = Scenario::Maintenance;
        g_loopBlocked = false;
        g_releaseLoop = false;
        PortalPipeWireScreencast capture;
        ReleaseLoop release;
        CaptureRequest request;
        request.allowInteractivePortal = true;
        QString error;
        QVERIFY2(capture.startRawStream(request, [](PipeWireScreencastRawFrame) {}, {}, &error), qPrintable(error));
        QTRY_VERIFY(g_loopBlocked.load());
        QElapsedTimer elapsed;
        elapsed.start();
        for (int i = 0; i < 100; ++i) capture.reclaimRawBuffers();
        QVERIFY(elapsed.elapsed() < 100);
        QVERIFY(g_pending->load());
        g_releaseLoop = true;
        QTRY_VERIFY(!g_pending->load());
        QCOMPARE(g_pool->retainedCapacity(), qsizetype(0));
        capture.stop();
        capture.reclaimRawBuffers();
    }
};

QTEST_GUILESS_MAIN(PipeWireRecordingMemoryTest)
#include "pipewire_recording_memory_test.moc"
