#include "platform/wayland/wlroots_screencopy_capture_stream.h"

#include "recording/recording_frame_handoff.h"

#include <QProcessEnvironment>

#include <memory>
#include <utility>

#ifdef HAVE_WLROOTS_SCREENCOPY
#include "platform/wayland/wlroots_screencopy_worker.h"

#include <QMetaObject>
#include <QPointer>
#include <QThread>
#endif

namespace markshot::recording {
namespace {

#ifdef HAVE_WLROOTS_SCREENCOPY

class WlrootsScreencopyCaptureStream final : public RecordingCaptureStream {
public:
    /**
     * 创建 wlroots screencopy 采集流。
     * @param options 录制配置。
     * @param parent 父对象。
     */
    explicit WlrootsScreencopyCaptureStream(RecordingOptions options, QObject *parent = nullptr)
        : RecordingCaptureStream(parent)
        , m_options(std::move(options))
    {
        qRegisterMetaType<RecordingFrameSample>("markshot::recording::RecordingFrameSample");
    }

    /**
     * 销毁 wlroots screencopy 采集流。
     */
    ~WlrootsScreencopyCaptureStream() override
    {
        stop();
    }

    /**
     * 启动采集流。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(QString *error) override
    {
        if (error) {
            error->clear();
        }
        if (m_thread.isRunning()) {
            return true;
        }
        m_handoff = std::make_unique<RecordingFrameHandoff>(this);
        connect(m_handoff.get(), &RecordingFrameHandoff::frameReady,
                this, &RecordingCaptureStream::frameReady);
        connect(m_handoff.get(), &RecordingFrameHandoff::failed,
                this, &RecordingCaptureStream::failed);
        m_worker = new WlrootsScreencopyWorker(m_options);
        m_worker->moveToThread(&m_thread);
        connect(&m_thread, &QThread::finished, m_worker, &QObject::deleteLater);
        connect(m_worker,
                &WlrootsScreencopyWorker::frameReady,
                m_handoff.get(),
                [publish = m_handoff->publisher()](const RecordingFrameSample &sample) { publish(sample); },
                Qt::DirectConnection);
        connect(m_worker,
                &WlrootsScreencopyWorker::failed,
                m_handoff.get(),
                [fail = m_handoff->errorPublisher()](const QString &error) { fail(error); },
                Qt::DirectConnection);
        m_thread.start();

        QString workerError;
        bool ok = false;
        QMetaObject::invokeMethod(
            m_worker,
            [this, &workerError, &ok] {
                ok = m_worker->start(&workerError);
            },
            Qt::BlockingQueuedConnection);
        if (!ok) {
            if (error) {
                *error = workerError;
            }
            stop();
            return false;
        }
        return true;
    }

    /**
     * 停止采集流。
     * @return 无返回值。
     */
    void stop() override
    {
        // 1. 【录制】【帧交接】先归还尚未交付的映射帧，再等待采集线程释放 Wayland 对象
        if (m_handoff) {
            m_handoff->close();
        }
        if (m_thread.isRunning() && m_worker) {
            QMetaObject::invokeMethod(m_worker,
                                      [this] { m_worker->stop(); },
                                      Qt::BlockingQueuedConnection);
        }
        if (m_thread.isRunning()) {
            m_thread.quit();
            m_thread.wait();
        }
        m_worker = nullptr;
    }

    /**
     * 设置写出背压状态。
     * @param active 队列繁忙时为 true。
     * @return 无返回值。
     */
    void setBackpressureActive(bool active) override
    {
        if (m_handoff) {
            m_handoff->setEnabled(!active);
        }
        if (!m_thread.isRunning() || !m_worker) {
            return;
        }
        QMetaObject::invokeMethod(
            m_worker,
            [worker = QPointer<WlrootsScreencopyWorker>(m_worker), active] {
                if (worker) {
                    worker->setBackpressureActive(active);
                }
            },
            Qt::QueuedConnection);
    }

private:
    RecordingOptions m_options;
    QThread m_thread;
    WlrootsScreencopyWorker *m_worker = nullptr;
    std::unique_ptr<RecordingFrameHandoff> m_handoff;
};

#endif

/**
 * 判断当前会话是否值得尝试 wlroots screencopy。
 * @return 可尝试时返回 true。
 */
bool shouldTryWlrootsScreencopy()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    if (env.value(QStringLiteral("XDG_SESSION_TYPE")).toLower() != QStringLiteral("wayland")) {
        return false;
    }
    const QString desktop = (env.value(QStringLiteral("XDG_CURRENT_DESKTOP")) + QLatin1Char(':')
                             + env.value(QStringLiteral("XDG_SESSION_DESKTOP")) + QLatin1Char(':')
                             + env.value(QStringLiteral("DESKTOP_SESSION")))
                                .toLower();
    return desktop.contains(QStringLiteral("niri"))
        || desktop.contains(QStringLiteral("sway"))
        || desktop.contains(QStringLiteral("hyprland"))
        || desktop.contains(QStringLiteral("river"))
        || desktop.contains(QStringLiteral("wayfire"))
        || desktop.contains(QStringLiteral("labwc"))
        || desktop.contains(QStringLiteral("wlroots"));
}

}  // namespace

std::unique_ptr<RecordingCaptureStream> createWlrootsScreencopyCaptureStream(const RecordingOptions &options,
                                                                             QObject *parent)
{
#ifdef HAVE_WLROOTS_SCREENCOPY
    // GIF 与视频共用同一条采集链路，写出层按目标帧率筛除多余帧
    if (!shouldTryWlrootsScreencopy()) {
        return nullptr;
    }
    return std::make_unique<WlrootsScreencopyCaptureStream>(options, parent);
#else
    Q_UNUSED(options)
    Q_UNUSED(parent)
    return nullptr;
#endif
}

}  // namespace markshot::recording
