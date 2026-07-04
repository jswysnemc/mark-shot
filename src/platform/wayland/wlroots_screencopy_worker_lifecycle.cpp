#include "platform/wayland/wlroots_screencopy_worker.h"

#include "debug_log.h"

#include <QSocketNotifier>

#include <utility>

namespace markshot::recording {

#ifdef HAVE_WLROOTS_SCREENCOPY

/**
 * 【录制】【wlroots采集】创建 screencopy 工作对象。
 * @param options 录制配置。
 */
WlrootsScreencopyWorker::WlrootsScreencopyWorker(RecordingOptions options)
    : m_options(std::move(options))
{
}

/**
 * 【录制】【wlroots采集】启动 Wayland screencopy 会话。
 * @param error 输出错误信息。
 * @return 启动成功时返回 true。
 */
bool WlrootsScreencopyWorker::start(QString *error)
{
    if (error) {
        error->clear();
    }
    if (m_options.display.allOutputs) {
        if (error) {
            *error = QStringLiteral("wlroots screencopy does not support all-outputs recording");
        }
        return false;
    }

    // 1. 【录制】【wlroots采集】连接当前 Wayland display
    m_display = wl_display_connect(nullptr);
    if (!m_display) {
        if (error) {
            *error = QStringLiteral("failed to connect Wayland display");
        }
        return false;
    }

    // 2. 【录制】【wlroots采集】读取 registry，绑定 wl_shm、wl_output 和 screencopy manager
    m_registry = wl_display_get_registry(m_display);
    if (!m_registry) {
        cleanup();
        if (error) {
            *error = QStringLiteral("failed to get Wayland registry");
        }
        return false;
    }
    wl_registry_add_listener(m_registry, &s_registryListener, this);
    wl_display_roundtrip(m_display);
    wl_display_roundtrip(m_display);

    if (!m_shm || !m_screencopy || m_outputs.empty()) {
        const QString missing = !m_screencopy
            ? QStringLiteral("zwlr_screencopy_manager_v1 is not available")
            : !m_shm ? QStringLiteral("wl_shm is not available")
                     : QStringLiteral("no Wayland outputs are available");
        cleanup();
        if (error) {
            *error = missing;
        }
        return false;
    }

    m_selectedOutput = chooseOutput();
    if (!m_selectedOutput || !m_selectedOutput->output) {
        cleanup();
        if (error) {
            *error = QStringLiteral("failed to select Wayland output for recording");
        }
        return false;
    }

    // 3. 【录制】【wlroots采集】选择输出并注册 Qt 事件分发
    m_notifier = new QSocketNotifier(wl_display_get_fd(m_display), QSocketNotifier::Read, this);
    connect(m_notifier, &QSocketNotifier::activated, this, &WlrootsScreencopyWorker::dispatchEvents);
    m_running = true;
    m_clock.start();
    requestFrame();
    markshot::debugLog("recording",
                       "【录制】【wlroots采集】started output=%s geometry=%d,%d %dx%d",
                       m_selectedOutput->name.toUtf8().constData(),
                       m_selectedOutput->geometry.x(),
                       m_selectedOutput->geometry.y(),
                       m_selectedOutput->geometry.width(),
                       m_selectedOutput->geometry.height());
    return true;
}

/**
 * 【录制】【wlroots采集】停止 Wayland screencopy 会话。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::stop()
{
    m_running = false;
    cleanup();
}

/**
 * 【录制】【wlroots采集】设置写出背压状态。
 * @param active 队列繁忙时为 true。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::setBackpressureActive(bool active)
{
    m_backpressureActive = active;
    if (!m_backpressureActive) {
        requestFrame();
    }
}

/**
 * 【录制】【wlroots采集】分发 Wayland display 事件。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::dispatchEvents()
{
    if (!m_display || !m_running) {
        return;
    }
    if (wl_display_dispatch(m_display) < 0) {
        emit failed(QStringLiteral("wlroots screencopy Wayland dispatch failed"));
        stop();
        return;
    }
    wl_display_flush(m_display);
}

/**
 * 【录制】【wlroots采集】释放 Wayland 资源和共享内存缓冲。
 * @return 无返回值。
 */
void WlrootsScreencopyWorker::cleanup()
{
    // 1. 【录制】【wlroots采集】先停止 Qt 事件源，避免清理期间继续分发 Wayland 事件
    if (m_notifier) {
        m_notifier->setEnabled(false);
        delete m_notifier;
        m_notifier = nullptr;
    }
    // 2. 【录制】【wlroots采集】按依赖顺序释放 frame、buffer、output 和全局对象
    destroyCurrentFrame();
    m_buffer.reset();
    for (std::unique_ptr<WlrootsOutput> &output : m_outputs) {
        if (output->output) {
            wl_output_destroy(output->output);
            output->output = nullptr;
        }
    }
    m_outputs.clear();
    m_selectedOutput = nullptr;
    if (m_screencopy) {
        zwlr_screencopy_manager_v1_destroy(m_screencopy);
        m_screencopy = nullptr;
    }
    if (m_shm) {
        wl_shm_destroy(m_shm);
        m_shm = nullptr;
    }
    if (m_registry) {
        wl_registry_destroy(m_registry);
        m_registry = nullptr;
    }
    if (m_display) {
        wl_display_disconnect(m_display);
        m_display = nullptr;
    }
}

#endif

}  // namespace markshot::recording
