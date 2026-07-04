#include "recording/libav_error.h"

#ifdef HAVE_LIBAV_RECORDING
extern "C" {
#include <libavutil/error.h>
}
#endif

namespace markshot::recording {

/**
 * 把 FFmpeg 错误码转换为可读文本。
 * @param errorCode FFmpeg 返回的错误码。
 * @return 可读错误文本。
 */
QString libavErrorText(int errorCode)
{
#ifdef HAVE_LIBAV_RECORDING
    char buffer[AV_ERROR_MAX_STRING_SIZE] = {};
    av_strerror(errorCode, buffer, sizeof(buffer));
    return QString::fromLocal8Bit(buffer);
#else
    return QStringLiteral("FFmpeg libraries are not available: %1").arg(errorCode);
#endif
}

}  // namespace markshot::recording
