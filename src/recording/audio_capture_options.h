#pragma once

#include <QStringList>

namespace markshot::recording {

/**
 * 生成 FFmpeg 平台默认音频输入参数。
 * @return FFmpeg 参数列表，当前平台不支持时为空。
 */
QStringList defaultAudioInputArguments();

}  // namespace markshot::recording
