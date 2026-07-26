#include "recording/recording_encoder_probe.h"

#include <QDir>
#include <QFileInfo>
#include <QtGlobal>

#ifdef HAVE_LIBAV_RECORDING
#include <QByteArray>

extern "C" {
#include <libavcodec/avcodec.h>
}
#endif

namespace markshot::recording {

bool recordingEncoderImplementationAvailable(const QString &encoderId)
{
#ifdef HAVE_LIBAV_RECORDING
    const QByteArray name = encoderId.trimmed().toUtf8();
    if (name.isEmpty()) {
        return false;
    }
    return avcodec_find_encoder_by_name(name.constData()) != nullptr;
#else
    // 未链接 FFmpeg 时无法探测，保留候选并交由运行时打开失败后回退
    Q_UNUSED(encoderId)
    return true;
#endif
}

QStringList recordingRenderNodePaths()
{
#ifdef Q_OS_WIN
    return {};
#else
    // 1. VAAPI 与 QSV 都通过 DRM 渲染节点访问 GPU
    const QDir driDirectory(QStringLiteral("/dev/dri"));
    if (!driDirectory.exists()) {
        return {};
    }

    QStringList paths;
    const QStringList nodes = driDirectory.entryList({QStringLiteral("renderD*")},
                                                     QDir::System,
                                                     QDir::Name);
    paths.reserve(nodes.size());
    for (const QString &node : nodes) {
        paths.append(driDirectory.filePath(node));
    }
    return paths;
#endif
}

bool recordingRenderNodeAvailable()
{
    return !recordingRenderNodePaths().isEmpty();
}

bool recordingNvidiaDeviceAvailable()
{
#ifdef Q_OS_WIN
    return true;
#else
    // 2. NVENC 走专有驱动节点，节点不存在时没有必要尝试
    return QFileInfo::exists(QStringLiteral("/dev/nvidiactl"))
        || QFileInfo::exists(QStringLiteral("/dev/nvidia0"))
        || QFileInfo::exists(QStringLiteral("/proc/driver/nvidia/version"));
#endif
}

}  // namespace markshot::recording
