#include "platform/wayland/wlroots_screencopy_buffer_pool.h"
#include "support/process_memory_sample.h"

#include <QtTest/QtTest>

#include <cstring>
#include <wayland-client.h>

using namespace markshot::recording;

namespace {
struct ShmConnection {
    wl_display *display = nullptr;
    wl_registry *registry = nullptr;
    wl_shm *shm = nullptr;

    /** 在隔离显示器上绑定 wl_shm，无参数。 */
    ShmConnection()
    {
        display = wl_display_connect(nullptr);
        if (!display) return;
        registry = wl_display_get_registry(display);
        static const wl_registry_listener listener{
            [](void *data, wl_registry *registry, uint32_t name, const char *interface, uint32_t) {
                if (std::strcmp(interface, wl_shm_interface.name) == 0) {
                    static_cast<ShmConnection *>(data)->shm = static_cast<wl_shm *>(
                        wl_registry_bind(registry, name, &wl_shm_interface, 1));
                }
            },
            [](void *, wl_registry *, uint32_t) {}};
        wl_registry_add_listener(registry, &listener, this);
        wl_display_roundtrip(display);
    }

    /** 调用者先销毁池，再关闭协议连接，无参数。 */
    ~ShmConnection()
    {
        if (shm) wl_shm_destroy(shm);
        if (registry) wl_registry_destroy(registry);
        if (display) wl_display_disconnect(display);
    }
};
}  // namespace

class RecordingWlrootsBufferPoolTest final : public QObject {
    Q_OBJECT
private slots:
    /** 只允许在测试脚本创建的隔离 socket 内运行，无参数和返回值。 */
    void initTestCase()
    {
        const QString runtime = qEnvironmentVariable("MARK_SHOT_RECORDING_TEST_RUNTIME");
        QVERIFY(!runtime.isEmpty());
        QVERIFY(qEnvironmentVariable("WAYLAND_DISPLAY").startsWith(runtime + QLatin1Char('/')));
    }

    /** 同一映射的多份引用只计费一次；预算满时拒绝创建下一帧，无参数和返回值。 */
    void enforcesBudgetBeforeMapping()
    {
        ShmConnection connection;
        QVERIFY(connection.shm);
        WlrootsScreencopyBufferPool pool(3 * 4096);
        std::vector<std::shared_ptr<WlrootsScreencopyShmBuffer>> held;
        QString error;
        for (int i = 0; i < 3; ++i) {
            auto buffer = pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error, 0);
            QVERIFY2(buffer, qPrintable(error));
            held.push_back(buffer);
        }
        const auto shared = held.front();
        QCOMPARE(pool.retainedBytes(), quint64(3 * 4096));
        QVERIFY(!pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error, 0));
        QCOMPARE(error, QStringLiteral("wlroots screencopy buffer pool is temporarily full"));
        held.back().reset();
        QVERIFY(pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error, 1));
    }

    /** 没有新帧时也按真实时间回收闲置映射，借出帧归还后重新计算空闲期，无参数和返回值。 */
    void reclaimsIdleMappingsWithoutNewCapture()
    {
        ShmConnection connection;
        QVERIFY(connection.shm);
        WlrootsScreencopyBufferPool pool;
        QString error;
        auto held = pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error, 0);
        QVERIFY(held);
        auto idle = pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error, 0);
        QVERIFY(idle);
        idle.reset();
        pool.reclaimIdle(100);
        pool.reclaimIdle(2099);
        QCOMPARE(pool.retainedBytes(), quint64(8192));
        pool.reclaimIdle(2100);
        QCOMPARE(pool.retainedBytes(), quint64(4096));
        held.reset();
        pool.reclaimIdle(10000);
        QCOMPARE(pool.retainedBytes(), quint64(4096));
        pool.reclaimIdle(12000);
        QCOMPARE(pool.retainedBytes(), quint64(0));
    }

    /** 降低分辨率时清除空闲大帧，不覆盖还未归还的映射，无参数和返回值。 */
    void downscalesWhileOldFrameIsHeld()
    {
        ShmConnection connection;
        QVERIFY(connection.shm);
        WlrootsScreencopyBufferPool pool(4096);
        QString error;
        auto a = pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 64, 32, 256, &error, 0);
        auto b = pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 64, 32, 256, &error, 0);
        QVERIFY(a && b);
        QVERIFY(!pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 16, 16, 64, &error, 1));
        a.reset();
        auto small = pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 16, 16, 64, &error, 2);
        QVERIFY(small);
        QCOMPARE(pool.retainedBytes(), quint64(8192 + 1024));
        b.reset();
        QVERIFY(pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 16, 16, 64, &error, 3));
        QCOMPARE(pool.retainedBytes(), quint64(2048));
    }

    /** 协议参数溢出必须在建立 mmap 前被拒绝，无参数和返回值。 */
    void rejectsOverflowingProtocolLayouts()
    {
        ShmConnection connection;
        QVERIFY(connection.shm);
        WlrootsScreencopyBufferPool pool;
        QString error;
        QVERIFY(!pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888,
                              std::numeric_limits<int>::max(), 2, 128, &error));
        QVERIFY(!pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888,
                              32, std::numeric_limits<int>::max(), 128, &error));
        QCOMPARE(pool.retainedBytes(), quint64(0));
    }

    /** 实际触达 4K 映射页，检查默认预算和空闲释放；RSS 只记录，不作为平台相关的断言，无参数和返回值。 */
    void measuresFourKMappingBudgetAndReclamation()
    {
        ShmConnection connection;
        QVERIFY(connection.shm);
        WlrootsScreencopyBufferPool pool(96 * 1024 * 1024);
        std::vector<std::shared_ptr<WlrootsScreencopyShmBuffer>> held;
        QString error;
        const auto before = markshot::test::processMemorySample();
        constexpr quint64 frameBytes = 3840ULL * 2160 * 4;
        for (int i = 0; i < 3; ++i) {
            auto buffer = pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 3840, 2160, 15360, &error, 0);
            QVERIFY2(buffer, qPrintable(error));
            auto view = buffer->mappedBgraFrame(buffer, false, &error);
            QVERIFY(view.isValid());
            std::memset(const_cast<char *>(view.constData()), i + 1, view.byteSize());
            held.push_back(buffer);
        }
        for (int i = 0; i < 13; ++i) {
            QVERIFY(!pool.acquire(connection.shm, WL_SHM_FORMAT_XRGB8888, 3840, 2160, 15360, &error, 1));
        }
        QCOMPARE(pool.retainedBytes(), frameBytes * 3);
        const auto full = markshot::test::processMemorySample();
        held.resize(1);
        pool.reclaimIdle(100);
        pool.reclaimIdle(2100);
        QCOMPARE(pool.retainedBytes(), frameBytes);
        const auto idle = markshot::test::processMemorySample();
        qInfo("【录制测试】【4K映射预算】rss_before=%lld rss_full=%lld rss_idle=%lld pool_bytes=%llu",
              static_cast<long long>(before.resident), static_cast<long long>(full.resident),
              static_cast<long long>(idle.resident), static_cast<unsigned long long>(pool.retainedBytes()));
    }
};

QTEST_MAIN(RecordingWlrootsBufferPoolTest)
#include "recording_wlroots_buffer_pool_test.moc"
