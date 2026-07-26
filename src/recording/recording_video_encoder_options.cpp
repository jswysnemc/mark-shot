#include "recording/recording_video_encoder_options.h"

#include "recording/recording_encoder_probe.h"

#include <QtGlobal>

namespace markshot::recording {
namespace {

/**
 * 【录制】【软件编码】创建指定的软件编码候选。
 * @param id FFmpeg 编码器名称。
 * @return 软件编码配置。
 */
RecordingVideoEncoderOptions softwareEncoder(const QString &id)
{
    return {
        id,
        id,
        false,
    };
}

/**
 * 创建硬件编码候选配置。
 * @param id FFmpeg 编码器名称。
 * @return 硬件编码配置。
 */
RecordingVideoEncoderOptions hardwareEncoder(const QString &id)
{
    return {id, id, true};
}

/**
 * 判断是否禁用硬件编码候选。
 * @return 环境变量要求仅软件编码时返回 true。
 */
bool hardwareEncodersDisabled()
{
    return qEnvironmentVariableIsSet("MARK_SHOT_RECORDING_SW_ENCODER");
}

/**
 * 【录制】【硬件编码】追加通过探测的硬件编码候选。
 * @param candidates 候选列表。
 * @param id FFmpeg 编码器名称。
 * @param deviceAvailable 对应硬件节点是否存在。
 * @return 无返回值。
 */
void appendHardwareCandidate(QVector<RecordingVideoEncoderOptions> &candidates,
                             const QString &id,
                             bool deviceAvailable)
{
    if (!deviceAvailable || !recordingEncoderImplementationAvailable(id)) {
        return;
    }
    candidates.append(hardwareEncoder(id));
}

/**
 * 【录制】【硬件编码】按平台追加硬件编码候选。
 * @param candidates 候选列表。
 * @return 无返回值。
 */
void appendPlatformHardwareCandidates(QVector<RecordingVideoEncoderOptions> &candidates)
{
    const bool nvidia = recordingNvidiaDeviceAvailable();
    const bool renderNode = recordingRenderNodeAvailable();

#if defined(Q_OS_WIN)
    Q_UNUSED(renderNode)
    // Windows 上三种硬件编码都接收系统内存帧，按厂商覆盖面排序
    appendHardwareCandidate(candidates, QStringLiteral("h264_nvenc"), nvidia);
    appendHardwareCandidate(candidates, QStringLiteral("h264_amf"), true);
    appendHardwareCandidate(candidates, QStringLiteral("h264_qsv"), true);
    appendHardwareCandidate(candidates, QStringLiteral("h264_mf"), true);
#else
    // 1. NVIDIA 走专有节点，只有驱动在位才尝试
    appendHardwareCandidate(candidates, QStringLiteral("h264_nvenc"), nvidia);
    // 2. VAAPI 覆盖 Intel 与 AMD，是 Linux 上适用面最广的硬件编码
    appendHardwareCandidate(candidates, QStringLiteral("h264_vaapi"), renderNode);
    // 3. QSV 依赖 Intel 专有运行时，作为 VAAPI 之后的补充
    appendHardwareCandidate(candidates, QStringLiteral("h264_qsv"), renderNode);
#endif
}

}  // namespace

QVector<RecordingVideoEncoderOptions> recordingVideoEncoderCandidates(const RecordingOptions &options,
                                                                      int fps)
{
    Q_UNUSED(options)
    Q_UNUSED(fps)
    QVector<RecordingVideoEncoderOptions> candidates;
    if (!hardwareEncodersDisabled()) {
        appendPlatformHardwareCandidates(candidates);
    }
    // 4. 优先使用画质更好的 libx264
    candidates.append(softwareEncoder(QStringLiteral("libx264")));
    // 5. mpeg4 是 FFmpeg 原生编码器，Fedora ffmpeg-free 默认提供该候选
    candidates.append(softwareEncoder(QStringLiteral("mpeg4")));
    return candidates;
}

}  // namespace markshot::recording
