#include "screen_capture_internal.h"

#ifdef MARK_SHOT_WITH_DBUS
// QtDBus 在 Windows 构建中不可用，D-Bus 专属头文件必须跟随开关引入
#include <QDBusConnectionInterface>
#include <QElapsedTimer>
#include <QMutex>
#include <QMutexLocker>
#endif

/// @brief Captures the screen using the grim utility.
/// @param request The capture request details such as source geometry and output name.
/// @return The result of the screen capture operation.
CaptureResult captureWithGrim(const CaptureRequest &request)
{
    const QStringList baseArguments{QStringLiteral("-t"), QStringLiteral("ppm")};
    auto grimArguments = [&request, &baseArguments] {
        QStringList arguments = baseArguments;
        if (request.includeCursor) {
            arguments << QStringLiteral("-c");
        }
        return arguments;
    };

    if (!request.allOutputs && !request.preferredOutputName.isEmpty()) {
        const QRect outputGeometry = screenGeometryForRequest(request);
        QStringList arguments = grimArguments();
        arguments << QStringLiteral("-o") << request.preferredOutputName << QStringLiteral("-");
        CaptureResult outputCapture = runGrim(arguments, request.preferredOutputName, outputGeometry, request.includeCursor);
        if (!outputCapture.image.isNull()) {
            if (!outputGeometry.isEmpty()) {
                return cropGrimFrameToRequest(std::move(outputCapture), outputGeometry, request);
            }
            if (!request.sourceGeometry.isValid() || request.sourceGeometry.isEmpty()) {
                return outputCapture;
            }
        }

        const QString outputError = outputCapture.error;
        const QRect fullGeometry = fullGrimSourceGeometry(request);
        QStringList fullArguments = grimArguments();
        fullArguments << QStringLiteral("-");
        CaptureResult fullCapture = runGrim(fullArguments, {}, fullGeometry, request.includeCursor);
        if (!fullCapture.image.isNull()) {
            fullCapture.outputName = request.preferredOutputName;
            return cropGrimFrameToRequest(std::move(fullCapture), fullGeometry, request);
        }

        if (!outputError.isEmpty() && !fullCapture.error.isEmpty()) {
            return {{},
                    QStringLiteral("%1\nFull-desktop grim fallback: %2")
                        .arg(outputError, fullCapture.error),
                    request.preferredOutputName,
                    request.sourceGeometry};
        }
        if (!outputGeometry.isValid() || outputGeometry.isEmpty()) {
            const QString fallbackError = fullCapture.error.isEmpty()
                ? QStringLiteral("full-desktop grim fallback was not usable")
                : fullCapture.error;
            return {{},
                    QStringLiteral("grim output capture has no Qt output geometry for local crop\nFull-desktop grim fallback: %1")
                        .arg(fallbackError),
                    request.preferredOutputName,
                    request.sourceGeometry};
        }
        return outputCapture.image.isNull() ? fullCapture : outputCapture;
    }

    const QRect frameGeometry = fullGrimSourceGeometry(request);
    QStringList arguments = grimArguments();
    arguments << QStringLiteral("-");
    CaptureResult fullCapture = runGrim(arguments, {}, frameGeometry, request.includeCursor);
    return cropGrimFrameToRequest(std::move(fullCapture), frameGeometry, request);
}

#ifdef MARK_SHOT_WITH_DBUS

namespace {

// KWin 可用性探测的缓存时长。D-Bus 服务列表在会话内基本不变，而滚动
// 捕获每个 tick 都会询问；短 TTL 只用于消化 KWin 重启等罕见变化。
constexpr int kKWinProbeCacheMs = 5000;

/**
 * 读取带 TTL 的 KWin ScreenShot2 可用性缓存。
 * @param available 探测到的新结果，空值表示只读缓存。
 * @return 缓存有效期内的可用性，过期且未更新时返回空值。
 */
std::optional<bool> kwinProbeCache(std::optional<bool> available)
{
    static QMutex mutex;
    static QElapsedTimer timer;
    static bool cached = false;

    QMutexLocker locker(&mutex);
    if (available.has_value()) {
        cached = *available;
        timer.restart();
        return available;
    }
    if (!timer.isValid() || timer.elapsed() >= kKWinProbeCacheMs) {
        return std::nullopt;
    }
    return cached;
}

}  // namespace

/// @brief 判断 KWin ScreenShot2 DBus 接口是否可用。
/// @return 接口存在时返回 true。
bool isKWinScreenShotAvailable()
{
    // 1. 命中缓存时跳过 D-Bus 往返，滚动捕获热路径不再逐帧探测
    if (const std::optional<bool> cached = kwinProbeCache(std::nullopt)) {
        return *cached;
    }

    // 2. 用服务注册表判断，避免 QDBusInterface 构造触发同步 Introspect 往返
    const bool available =
        QDBusConnection::sessionBus().interface()->isServiceRegistered(
            QStringLiteral("org.kde.KWin.ScreenShot2"));
    kwinProbeCache(available);
    return available;
}

/// @brief 使用 KWin ScreenShot2 接口截取指定 Wayland 区域。
/// @param request 捕获请求，包含源区域、输出名称和鼠标包含策略。
/// @return 捕获成功时返回图像，失败时返回错误信息供后续回退链路使用。
CaptureResult captureWithKWinScreenShot(const CaptureRequest &request)
{
    const QRect geometry = request.sourceGeometry.normalized();
    if (geometry.isEmpty()) {
        return {{}, QStringLiteral("KWin ScreenShot2 requires a non-empty geometry"), {}, request.sourceGeometry};
    }

    QDBusInterface kwin(QStringLiteral("org.kde.KWin.ScreenShot2"),
                        QStringLiteral("/org/kde/KWin/ScreenShot2"),
                        QStringLiteral("org.kde.KWin.ScreenShot2"),
                        QDBusConnection::sessionBus());
    if (!kwin.isValid()) {
        return {{}, QStringLiteral("org.kde.KWin.ScreenShot2 interface is not available"), {}, request.sourceGeometry};
    }

    int fds[2];
    if (::pipe2(fds, O_CLOEXEC) != 0) {
        return {{}, QStringLiteral("failed to create pipe for KWin ScreenShot2"), {}, request.sourceGeometry};
    }

    QVariantMap options;
    options.insert(QStringLiteral("include-cursor"), request.includeCursor);
    // native-resolution keeps device pixels on HiDPI instead of downscaling to
    // logical size, so the stitched result stays sharp.
    options.insert(QStringLiteral("native-resolution"), true);
    // KWin ScreenShot2 defaults to hide-caller-windows=true, which drops pinned
    // mark-shot windows. Pass false when the user wants own windows included.
    if (!request.hideOwnWindows) {
        options.insert(QStringLiteral("hide-caller-windows"), false);
    }
    // KWin sends the D-Bus reply with the buffer metadata first, then writes the
    // pixels to the pipe, so this synchronous call does not deadlock even when
    // the image is larger than the pipe buffer.
    QDBusReply<QVariantMap> reply =
        kwin.call(QStringLiteral("CaptureArea"),
                  geometry.x(), geometry.y(),
                  static_cast<uint>(geometry.width()), static_cast<uint>(geometry.height()),
                  options,
                  QVariant::fromValue(QDBusUnixFileDescriptor(fds[1])));
    ::close(fds[1]);

    if (!reply.isValid()) {
        ::close(fds[0]);
        markshot::debugLog("kwin", "capture-area-error geom=%d,%d %dx%d name=%s msg=%s",
                           geometry.x(), geometry.y(), geometry.width(), geometry.height(),
                           reply.error().name().toUtf8().constData(),
                           reply.error().message().toUtf8().constData());
        return {{},
                QStringLiteral("KWin ScreenShot2 CaptureArea failed: %1: %2")
                    .arg(reply.error().name(), reply.error().message()),
                {},
                request.sourceGeometry};
    }

    const QVariantMap results = reply.value();
    const int width = results.value(QStringLiteral("width")).toInt();
    const int height = results.value(QStringLiteral("height")).toInt();
    const int stride = results.value(QStringLiteral("stride")).toInt();
    const uint format = results.value(QStringLiteral("format")).toUInt();
    if (width <= 0 || height <= 0 || stride < width * 4) {
        ::close(fds[0]);
        return {{},
                QStringLiteral("KWin ScreenShot2 returned invalid buffer metadata (%1x%2 stride=%3)")
                    .arg(width).arg(height).arg(stride),
                {},
                request.sourceGeometry};
    }

    const qulonglong total = static_cast<qulonglong>(stride) * static_cast<qulonglong>(height);
    QByteArray buffer(static_cast<int>(total), Qt::Uninitialized);
    qulonglong received = 0;
    while (received < total) {
        struct pollfd pfd { fds[0], POLLIN, 0 };
        const int polled = ::poll(&pfd, 1, 2000);
        if (polled <= 0) {
            break;  // timeout or poll error
        }
        const ssize_t bytes = ::read(fds[0], buffer.data() + received, total - received);
        if (bytes <= 0) {
            break;  // EOF or read error
        }
        received += static_cast<qulonglong>(bytes);
    }
    ::close(fds[0]);

    if (received < total) {
        markshot::debugLog("kwin", "short-read got=%llu want=%llu %dx%d stride=%d",
                           received, total, width, height, stride);
        return {{},
                QStringLiteral("KWin ScreenShot2 delivered a truncated frame (%1/%2 bytes)")
                    .arg(received).arg(total),
                {},
                request.sourceGeometry};
    }

    const QImage::Format imageFormat =
        format != 0 ? static_cast<QImage::Format>(format) : QImage::Format_ARGB32_Premultiplied;
    const QImage view(reinterpret_cast<const uchar *>(buffer.constData()),
                      width, height, stride, imageFormat);
    if (view.isNull()) {
        return {{}, QStringLiteral("KWin ScreenShot2 frame could not be wrapped as an image"), {}, request.sourceGeometry};
    }

    markshot::debugLog("kwin", "capture-area-ok geom=%d,%d %dx%d -> frame=%dx%d stride=%d format=%u",
                       geometry.x(), geometry.y(), geometry.width(), geometry.height(),
                       width, height, stride, format);
    // Detach from the soon-to-be-freed buffer and normalize the format.
    return {view.copy().convertToFormat(QImage::Format_ARGB32_Premultiplied),
            {},
            request.allOutputs ? QString() : request.preferredOutputName,
            request.sourceGeometry,
            request.includeCursor};
}

#endif  // MARK_SHOT_WITH_DBUS
