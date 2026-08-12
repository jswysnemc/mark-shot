#pragma once

#include <QString>
#include <QVector>

namespace markshot::recording {

/// @brief 一个可用的音频输入设备。
struct AudioInputDevice {
    /// @brief 后端设备标识（PulseAudio source 名），传给采集器使用。
    QString name;
    /// @brief 展示给用户的可读描述。
    QString description;
    /// @brief 是否为输出回环（monitor）源，即“录制系统正在播放的声音”。
    bool isMonitor = false;
};

/**
 * 枚举当前可用的音频输入设备。
 *
 * Linux 上通过 PulseAudio/PipeWire-Pulse 的 introspection 接口列出全部
 * source（含 monitor 回环源）；查询是同步的，内部带短超时，服务不可用时
 * 返回空列表。其他平台或未链接 PulseAudio 时返回空列表，调用方应只展示
 * “系统默认”一项。
 *
 * @return 设备列表；空列表表示仅系统默认设备可用。
 */
QVector<AudioInputDevice> listAudioInputDevices();

}  // namespace markshot::recording
