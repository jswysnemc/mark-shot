#include "screen_capture_internal.h"

#include "kde_capture_config.h"

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

/// @brief 判断 KWin ScreenShot2 DBus 接口是否可用。
/// @return 接口存在时返回 true。
bool isKWinScreenShotAvailable()
{
    QDBusInterface kwin(QStringLiteral("org.kde.KWin.ScreenShot2"),
                        QStringLiteral("/org/kde/KWin/ScreenShot2"),
                        QStringLiteral("org.kde.KWin.ScreenShot2"),
                        QDBusConnection::sessionBus());
    return kwin.isValid();
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
    // 按隐藏自身窗口开关决定是否要求 KWin 排除调用者窗口
    // KWin ScreenShot2 默认会隐藏调用截图接口的窗口
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

CaptureResult captureWaylandFrame(const CaptureRequest &request)
{
    const bool grimPreferred = prefersGrim();
    const bool kdeSession = isKdePlasma();
    const bool kwinConfigured = markshot::configuredKdeKWinScreenshotEnabled();
    const bool kwinAvailable = kwinConfigured ? isKWinScreenShotAvailable() : false;
    markshot::debugLog("capture",
                       "wayland-frame geom=%d,%d %dx%d output=%s all_outputs=%d "
                       "prefer_screencast=%d allow_interactive=%d allow_screenshot_fallback=%d "
                       "prefers_grim=%d kde=%d kwin=%d kwin_configured=%d desktop_file=%s desktop=%s",
                       request.sourceGeometry.x(), request.sourceGeometry.y(),
                       request.sourceGeometry.width(), request.sourceGeometry.height(),
                       request.preferredOutputName.toUtf8().constData(),
                       request.allOutputs ? 1 : 0, request.preferScreencast ? 1 : 0,
                       request.allowInteractivePortal ? 1 : 0,
                       request.allowPortalScreenshotFallback ? 1 : 0, grimPreferred ? 1 : 0,
                       kdeSession ? 1 : 0, kwinAvailable ? 1 : 0, kwinConfigured ? 1 : 0,
                       QGuiApplication::desktopFileName().toUtf8().constData(),
                       desktopEnvironmentText().toUtf8().constData());

    if (isGnomeWaylandSession() && hasGnomeScrollHelper() && request.sourceGeometry.isValid()
        && !request.sourceGeometry.isEmpty() && !request.allOutputs) {
        markshot::debugLog("capture", "route=gnome-scroll-helper");
        CaptureResult gnomeCapture = captureWithGnomeScrollHelper(request);
        if (!gnomeCapture.image.isNull()) {
            markshot::debugLog("capture", "gnome-scroll-helper-ok frame=%dx%d",
                               gnomeCapture.image.width(), gnomeCapture.image.height());
            return gnomeCapture;
        }
        markshot::debugLog("capture", "gnome-scroll-helper-failed (falling back) error=%s",
                           gnomeCapture.error.toUtf8().constData());
    }

    // KDE 使用 ScreenShot2.CaptureArea 截取精确区域，失败后继续进入现有回退链路
    if (kwinConfigured && (kdeSession || kwinAvailable) && request.sourceGeometry.isValid()
        && !request.sourceGeometry.isEmpty()) {
        markshot::debugLog("capture", "route=kwin-screenshot kde=%d kwin=%d all_outputs=%d",
                           kdeSession ? 1 : 0, kwinAvailable ? 1 : 0,
                           request.allOutputs ? 1 : 0);
        CaptureResult kwinCapture = captureWithKWinScreenShot(request);
        if (!kwinCapture.image.isNull()) {
            markshot::debugLog("capture", "kwin-screenshot-ok frame=%dx%d",
                               kwinCapture.image.width(), kwinCapture.image.height());
            return kwinCapture;
        }
        markshot::debugLog("capture", "kwin-screenshot-failed (falling back) error=%s",
                           kwinCapture.error.toUtf8().constData());
    } else if (!kwinConfigured && (kdeSession || kwinAvailable)) {
        markshot::debugLog("capture", "kwin-screenshot-disabled-by-config");
    }

    if (request.preferScreencast) {
        markshot::debugLog("capture", "route=screencast (preferScreencast)");
        CaptureResult screencastCapture = captureWithPortalScreencast(request);
        if (!screencastCapture.image.isNull()) {
            markshot::debugLog("capture", "screencast-ok frame=%dx%d",
                               screencastCapture.image.width(), screencastCapture.image.height());
            return screencastCapture;
        }
        markshot::debugLog("capture", "screencast-failed error=%s",
                           screencastCapture.error.toUtf8().constData());
        stopPortalScreencast();

        CaptureResult portalCapture;
        if (request.allowPortalScreenshotFallback) {
            markshot::debugLog("capture", "fallback=portal-screenshot");
            portalCapture = captureWithPortalScreenshot(request);
            if (!portalCapture.image.isNull()) {
                markshot::debugLog("capture", "portal-screenshot-ok frame=%dx%d",
                                   portalCapture.image.width(), portalCapture.image.height());
                return portalCapture;
            }
            markshot::debugLog("capture", "portal-screenshot-failed error=%s",
                               portalCapture.error.toUtf8().constData());
        } else {
            markshot::debugLog("capture", "portal-screenshot-fallback disabled");
        }

        markshot::debugLog("capture", "fallback=grim");
        CaptureResult grimCapture = captureWithGrim(request);
        if (!grimCapture.image.isNull()) {
            markshot::debugLog("capture", "grim-ok frame=%dx%d",
                               grimCapture.image.width(), grimCapture.image.height());
            return grimCapture;
        }
        markshot::debugLog("capture", "grim-failed error=%s all-routes-exhausted",
                           grimCapture.error.toUtf8().constData());
        return {{},
                QStringLiteral("%1\nPortal screenshot fallback: %2\nGrim fallback: %3")
                    .arg(screencastCapture.error,
                         request.allowPortalScreenshotFallback
                             ? portalCapture.error
                             : QStringLiteral("disabled for live scrolling capture"),
                         grimCapture.error),
                {},
                request.sourceGeometry};
    }

    if (prefersGrim()) {
        CaptureResult grimCapture = captureWithGrim(request);
        if (!grimCapture.image.isNull()) {
            return grimCapture;
        }

        if (!request.allowPortalScreenshotFallback) {
            markshot::debugLog("capture", "【Wayland捕获】【Portal截图回退】已禁用");
            return {{},
                    QStringLiteral("%1\nPortal fallback: disabled for live capture").arg(grimCapture.error),
                    {},
                    request.sourceGeometry};
        }

        CaptureResult portalCapture = captureWithPortalScreenshot(request);
        if (!portalCapture.image.isNull()) {
            return portalCapture;
        }

        return {{}, QStringLiteral("%1\nPortal fallback: %2").arg(grimCapture.error, portalCapture.error), {}, request.sourceGeometry};
    }

    CaptureResult portalCapture;
    if (request.allowPortalScreenshotFallback) {
        portalCapture = captureWithPortalScreenshot(request);
        if (!portalCapture.image.isNull()) {
            return portalCapture;
        }
    } else {
        markshot::debugLog("capture", "【Wayland捕获】【Portal截图回退】已禁用");
    }

    CaptureResult grimCapture = captureWithGrim(request);
    if (!grimCapture.image.isNull()) {
        return grimCapture;
    }

    return {{},
            QStringLiteral("%1\nGrim fallback: %2")
                .arg(request.allowPortalScreenshotFallback
                         ? portalCapture.error
                         : QStringLiteral("Portal fallback disabled for live capture"),
                     grimCapture.error),
            {},
            request.sourceGeometry};
}

#endif  // MARK_SHOT_WITH_DBUS
