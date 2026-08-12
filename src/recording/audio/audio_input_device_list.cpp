#include "recording/audio/audio_input_device_list.h"

#include "debug_log.h"

#ifdef HAVE_PULSE_RECORDING
#include <pulse/context.h>
#include <pulse/introspect.h>
#include <pulse/mainloop.h>

#include <QElapsedTimer>
#endif

namespace markshot::recording {

#ifndef HAVE_PULSE_RECORDING

QVector<AudioInputDevice> listAudioInputDevices()
{
    return {};
}

#else

namespace {

/// @brief 枚举回调间共享的采集状态。
struct SourceEnumeration {
    QVector<AudioInputDevice> devices;
    bool done = false;
};

/**
 * 接收单个 source 信息的回调。
 * @param context PulseAudio 上下文。
 * @param info source 信息，列表结束时为空。
 * @param eol 非零表示列表结束。
 * @param userData 枚举状态。
 * @return 无返回值。
 */
void onSourceInfo(pa_context *context, const pa_source_info *info, int eol, void *userData)
{
    Q_UNUSED(context);
    auto *enumeration = static_cast<SourceEnumeration *>(userData);
    if (!enumeration) {
        return;
    }
    if (eol != 0 || !info) {
        enumeration->done = true;
        return;
    }
    AudioInputDevice device;
    device.name = QString::fromUtf8(info->name ? info->name : "");
    device.description = QString::fromUtf8(info->description ? info->description : "");
    device.isMonitor = info->monitor_of_sink != PA_INVALID_INDEX;
    if (!device.name.isEmpty()) {
        enumeration->devices.append(device);
    }
}

}  // namespace

QVector<AudioInputDevice> listAudioInputDevices()
{
    pa_mainloop *mainloop = pa_mainloop_new();
    if (!mainloop) {
        return {};
    }
    pa_context *context = pa_context_new(pa_mainloop_get_api(mainloop), "mark-shot");
    if (!context) {
        pa_mainloop_free(mainloop);
        return {};
    }

    QVector<AudioInputDevice> devices;
    if (pa_context_connect(context, nullptr, PA_CONTEXT_NOFLAGS, nullptr) >= 0) {
        // 同步驱动 mainloop：先等连接就绪，再等枚举回调收尾；整体限时避免
        // 声音服务无响应时卡住录制对话框。
        QElapsedTimer deadline;
        deadline.start();
        constexpr qint64 kTimeoutMs = 2000;
        pa_operation *operation = nullptr;
        SourceEnumeration enumeration;
        bool failed = false;
        while (!failed && deadline.elapsed() < kTimeoutMs) {
            if (pa_mainloop_iterate(mainloop, 0, nullptr) < 0) {
                break;
            }
            const pa_context_state_t state = pa_context_get_state(context);
            if (state == PA_CONTEXT_FAILED || state == PA_CONTEXT_TERMINATED) {
                failed = true;
                break;
            }
            if (!operation && state == PA_CONTEXT_READY) {
                operation = pa_context_get_source_info_list(context, &onSourceInfo, &enumeration);
                if (!operation) {
                    break;
                }
            }
            if (enumeration.done) {
                devices = enumeration.devices;
                break;
            }
        }
        if (operation) {
            pa_operation_unref(operation);
        }
        pa_context_disconnect(context);
    }
    pa_context_unref(context);
    pa_mainloop_free(mainloop);

    markshot::debugLog("recording",
                       "【录制】【音频设备】enumerated count=%lld",
                       static_cast<long long>(devices.size()));
    return devices;
}

#endif

}  // namespace markshot::recording
