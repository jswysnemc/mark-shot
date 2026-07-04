#pragma once

#include <QString>

namespace markshot::recording {

/**
 * 把 FFmpeg 错误码转换为可读文本。
 * @param errorCode FFmpeg 返回的错误码。
 * @return 可读错误文本。
 */
QString libavErrorText(int errorCode);

}  // namespace markshot::recording
