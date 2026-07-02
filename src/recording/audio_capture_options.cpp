#include "recording/audio_capture_options.h"

#include <QtGlobal>

namespace markshot::recording {

QStringList defaultAudioInputArguments()
{
#if defined(Q_OS_WIN)
    return {QStringLiteral("-f"), QStringLiteral("dshow"), QStringLiteral("-i"), QStringLiteral("audio=default")};
#elif defined(Q_OS_MACOS)
    return {QStringLiteral("-f"), QStringLiteral("avfoundation"), QStringLiteral("-i"), QStringLiteral(":default")};
#elif defined(Q_OS_LINUX)
    return {QStringLiteral("-f"), QStringLiteral("pulse"), QStringLiteral("-i"), QStringLiteral("default")};
#else
    return {};
#endif
}

}  // namespace markshot::recording
