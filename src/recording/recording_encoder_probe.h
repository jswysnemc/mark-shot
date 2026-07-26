#pragma once

#include <QString>
#include <QStringList>

namespace markshot::recording {

/**
 * 【录制】【编码探测】判断 FFmpeg 是否编入了指定编码器实现。
 * @param encoderId FFmpeg 编码器名称。
 * @return 编码器可用时返回 true；未链接 FFmpeg 时返回 true 并交由运行时回退处理。
 */
bool recordingEncoderImplementationAvailable(const QString &encoderId);

/**
 * 【录制】【编码探测】判断是否存在可用于 VAAPI 的渲染节点。
 * @return 存在 /dev/dri 渲染节点时返回 true。
 */
bool recordingRenderNodeAvailable();

/**
 * 【录制】【编码探测】列出全部 DRM 渲染节点。
 * @return 渲染节点路径，按节点编号升序排列。
 */
QStringList recordingRenderNodePaths();

/**
 * 【录制】【编码探测】判断是否存在 NVIDIA 驱动节点。
 * @return 存在 NVIDIA 设备节点时返回 true。
 */
bool recordingNvidiaDeviceAvailable();

}  // namespace markshot::recording
