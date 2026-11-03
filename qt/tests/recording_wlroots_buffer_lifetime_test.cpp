#include "platform/wayland/wlroots_screencopy_buffer_pool.h"

#include <QtTest/QtTest>

#include <cstring>
#include <fcntl.h>
#include <sys/socket.h>
#include <thread>
#include <unistd.h>
#include <wayland-client.h>

using namespace markshot::recording;

namespace {

constexpr quint16 kWlBufferDestroyOpcode = 0;

/**
 * 【录制测试】【协议记录】用 socketpair 冒充 compositor，只记录客户端写出的 Wayland 请求，
 * 不需要真实桌面即可验证 wl_buffer 何时、由谁发送 destroy。
 */
struct WireRecorder {
    wl_display *display = nullptr;
    wl_registry *registry = nullptr;
    wl_shm *shm = nullptr;
    int serverFd = -1;
    QByteArray wire;

    /** 建立连接并绑定 wl_shm 全局对象，失败时 shm 为空，无参数。 */
    WireRecorder()
    {
        int fds[2] = {-1, -1};
        if (::socketpair(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0, fds) != 0) return;
        serverFd = fds[1];
        ::fcntl(serverFd, F_SETFL, O_NONBLOCK);
        // wl_display_connect_to_fd 在成功和失败时都会接管 fds[0]
        display = wl_display_connect_to_fd(fds[0]);
        if (!display) return;
        registry = wl_display_get_registry(display);
        shm = static_cast<wl_shm *>(wl_registry_bind(registry, 1, &wl_shm_interface, 1));
    }

    /** 若测试未显式断开则在此断开，并关闭模拟服务端，无参数。 */
    ~WireRecorder()
    {
        disconnect();
        if (serverFd >= 0) ::close(serverFd);
    }

    /** 把客户端缓冲的请求写入 socket 并读入 wire，无参数和返回值。 */
    void drain()
    {
        if (display) wl_display_flush(display);
        char chunk[4096];
        for (;;) {
            const ssize_t count = ::read(serverFd, chunk, sizeof(chunk));
            if (count <= 0) break;
            wire.append(chunk, static_cast<qsizetype>(count));
        }
    }

    /** 统计 objectId 上 opcode 请求的出现次数，返回次数。 */
    int requests(quint32 objectId, quint16 opcode) const
    {
        int matches = 0;
        qsizetype offset = 0;
        while (offset + 8 <= wire.size()) {
            quint32 header[2];
            std::memcpy(header, wire.constData() + offset, sizeof(header));
            const quint32 size = header[1] >> 16;
            if (size < 8) break;
            if (header[0] == objectId && (header[1] & 0xffff) == opcode) ++matches;
            offset += size;
        }
        return matches;
    }

    /** 模拟采集线程收尾：写出剩余请求后销毁全局对象并断开连接，无参数和返回值。 */
    void disconnect()
    {
        if (!display) return;
        drain();
        if (shm) wl_shm_destroy(shm);
        if (registry) wl_registry_destroy(registry);
        wl_display_disconnect(display);
        shm = nullptr;
        registry = nullptr;
        display = nullptr;
    }
};

/** 在 recorder 的连接上创建 32x32 XRGB 缓冲，返回绑定协议对象的缓冲，失败返回空。 */
std::shared_ptr<WlrootsScreencopyShmBuffer> makeBuffer(WireRecorder &recorder)
{
    auto buffer = std::make_shared<WlrootsScreencopyShmBuffer>();
    QString error;
    if (!buffer->ensure(recorder.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error)) return nullptr;
    return buffer;
}

}  // namespace

class RecordingWlrootsBufferLifetimeTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 【录制测试】【#145】控制器保留最后一帧，采集线程先按清理顺序关闭连接，主线程随后释放该帧。
     * 协议对象必须在断开前由采集线程销毁，之后释放帧只解除像素映射，无参数和返回值。
     */
    void retainedFrameOutlivesConnection()
    {
        RecordingRawBgraFrame retained;
        quint32 bufferId = 0;
        bool published = false;
        bool releasedBeforeDisconnect = false;
        int destroyRequests = -1;
        std::thread capture([&] {
            WireRecorder recorder;
            WlrootsScreencopyBufferPool pool(64 * 1024 * 1024);
            QString error;
            auto held = pool.acquire(recorder.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error, 0);
            if (!held) return;
            bufferId = wl_proxy_get_id(reinterpret_cast<wl_proxy *>(held->buffer()));
            // 1. 【录制测试】【#145】按 publishReadyFrame 的方式发布映射帧并写入可识别像素
            retained = held->mappedBgraFrame(held, false, &error);
            published = retained.isValid();
            if (published) std::memset(const_cast<char *>(retained.constData()), 0x5a, retained.byteSize());
            held.reset();
            // 2. 【录制测试】【#145】按 WlrootsScreencopyWorker::cleanup 的顺序先清空缓冲池再断开连接
            const auto lease = std::static_pointer_cast<const WlrootsScreencopyShmBuffer>(retained.mappedOwner);
            pool.reset();
            releasedBeforeDisconnect = lease && lease->buffer() == nullptr;
            recorder.disconnect();
            destroyRequests = recorder.requests(bufferId, kWlBufferDestroyOpcode);
        });
        capture.join();

        QVERIFY(bufferId != 0);
        QVERIFY(published);
        QVERIFY2(releasedBeforeDisconnect, "Pool reset must close the wl_buffer of a borrowed frame");
        QCOMPARE(destroyRequests, 1);
        // 3. 【录制测试】【#145】连接已关闭，主线程仍可读取像素，并在控制器析构时释放最后一帧
        QCOMPARE(retained.constData()[0], char(0x5a));
        QCOMPARE(retained.constData()[retained.byteSize() - 1], char(0x5a));
        const std::weak_ptr<const void> lease = retained.mappedOwner;
        retained = {};
        QVERIFY(lease.expired());
    }

    /** 提供最后一个引用所在线程，无参数和返回值。 */
    void destructorNeverSendsProtocolRequests_data()
    {
        QTest::addColumn<bool>("foreignThread");
        QTest::newRow("capture-thread") << false;
        QTest::newRow("foreign-thread") << true;
    }

    /**
     * 【录制测试】【映射寿命】缓冲析构只解除像素映射，无论最后一个引用在哪个线程释放都不编组
     * wl_buffer.destroy；协议对象仍由创建线程负责销毁，无参数和返回值。
     */
    void destructorNeverSendsProtocolRequests()
    {
        QFETCH(bool, foreignThread);
        WireRecorder recorder;
        QVERIFY(recorder.shm);
        auto buffer = makeBuffer(recorder);
        QVERIFY(buffer);
        wl_buffer *proxy = buffer->buffer();
        const quint32 bufferId = wl_proxy_get_id(reinterpret_cast<wl_proxy *>(proxy));

        if (foreignThread) {
            std::thread([last = std::move(buffer)]() mutable { last.reset(); }).join();
        } else {
            buffer.reset();
        }
        recorder.drain();
        QCOMPARE(recorder.requests(bufferId, kWlBufferDestroyOpcode), 0);

        // 1. 【录制测试】【映射寿命】确认记录器能观察到真正的 destroy，同时回收测试中的句柄
        wl_buffer_destroy(proxy);
        recorder.drain();
        QCOMPARE(recorder.requests(bufferId, kWlBufferDestroyOpcode), 1);
    }

    /** 非创建线程显式释放协议对象时只放弃句柄，不跨线程编组请求，无参数和返回值。 */
    void foreignReleaseIsRefused()
    {
        WireRecorder recorder;
        QVERIFY(recorder.shm);
        auto buffer = makeBuffer(recorder);
        QVERIFY(buffer);
        wl_buffer *proxy = buffer->buffer();
        const quint32 bufferId = wl_proxy_get_id(reinterpret_cast<wl_proxy *>(proxy));

        std::thread([&buffer] { buffer->releaseWaylandBuffer(); }).join();
        QVERIFY(buffer->buffer() == nullptr);
        recorder.drain();
        QCOMPARE(recorder.requests(bufferId, kWlBufferDestroyOpcode), 0);
        wl_buffer_destroy(proxy);
    }

    /** 空闲回收、布局切换和整体清空都必须在采集线程关闭被移除槽位的 wl_buffer，无参数和返回值。 */
    void poolDropPathsCloseProtocolBuffers()
    {
        WireRecorder recorder;
        QVERIFY(recorder.shm);
        WlrootsScreencopyBufferPool pool(64 * 1024 * 1024);
        QString error;

        // 1. 【录制测试】【空闲回收】归还后空闲满两秒的槽位被移除并发送 destroy
        auto idle = pool.acquire(recorder.shm, WL_SHM_FORMAT_XRGB8888, 32, 32, 128, &error, 0);
        QVERIFY2(idle, qPrintable(error));
        const quint32 idleId = wl_proxy_get_id(reinterpret_cast<wl_proxy *>(idle->buffer()));
        idle.reset();
        pool.reclaimIdle(0);
        pool.reclaimIdle(kRecordingCaptureIdleMs);
        QCOMPARE(pool.retainedBytes(), quint64(0));
        recorder.drain();
        QCOMPARE(recorder.requests(idleId, kWlBufferDestroyOpcode), 1);

        // 2. 【录制测试】【布局切换】新布局移除已归还的旧布局槽位并发送 destroy
        auto large = pool.acquire(recorder.shm, WL_SHM_FORMAT_XRGB8888, 64, 32, 256, &error, 0);
        QVERIFY2(large, qPrintable(error));
        const quint32 largeId = wl_proxy_get_id(reinterpret_cast<wl_proxy *>(large->buffer()));
        large.reset();
        auto small = pool.acquire(recorder.shm, WL_SHM_FORMAT_XRGB8888, 16, 16, 64, &error, 1);
        QVERIFY2(small, qPrintable(error));
        recorder.drain();
        QCOMPARE(recorder.requests(largeId, kWlBufferDestroyOpcode), 1);

        // 3. 【录制测试】【整体清空】借出中的槽位也在清空时关闭协议对象，像素映射留给借用方
        const quint32 smallId = wl_proxy_get_id(reinterpret_cast<wl_proxy *>(small->buffer()));
        pool.reset();
        QVERIFY(small->buffer() == nullptr);
        QVERIFY(small->byteSize() > 0);
        recorder.drain();
        QCOMPARE(recorder.requests(smallId, kWlBufferDestroyOpcode), 1);
    }
};

QTEST_GUILESS_MAIN(RecordingWlrootsBufferLifetimeTest)
#include "recording_wlroots_buffer_lifetime_test.moc"
