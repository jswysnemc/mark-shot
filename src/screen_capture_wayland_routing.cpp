#include "screen_capture_internal.h"

#include "capture_own_windows_policy.h"
#include "capture_own_windows_guard.h"
#include "kde_capture_config.h"

#ifdef MARK_SHOT_WITH_DBUS

/**
 * 【Wayland捕获】【后端路由】按会话能力和请求策略选择采集后端。
 * @param request 区域、输出、交互授权和自身窗口策略。
 * @return 成功的采集帧，或全部后端失败时的错误信息。
 */
CaptureResult captureWaylandFrame(const CaptureRequest &request)
{
    std::optional<markshot::CaptureOwnWindowsGuard> ownWindowsGuard;
    if (request.hideOwnWindows && !request.preferScreencast) {
        ownWindowsGuard.emplace();
    }

    const bool grimPreferred = prefersGrim();
    const bool kdeSession = isKdePlasma();
    const bool kwinConfigured = markshot::configuredKdeKWinScreenshotEnabled();
    const bool kwinAvailable = kwinConfigured ? isKWinScreenShotAvailable() : false;
    if (markshot::debugEnabled()) {
        markshot::debugLog("capture",
                           "【Wayland捕获】【后端路由】wayland-frame geom=%d,%d %dx%d output=%s all_outputs=%d "
                           "prefer_screencast=%d allow_screencast=%d allow_interactive=%d allow_screenshot_fallback=%d "
                           "prefers_grim=%d kde=%d kwin=%d kwin_configured=%d desktop_file=%s desktop=%s",
                           request.sourceGeometry.x(), request.sourceGeometry.y(),
                           request.sourceGeometry.width(), request.sourceGeometry.height(),
                           request.preferredOutputName.toUtf8().constData(),
                           request.allOutputs ? 1 : 0, request.preferScreencast ? 1 : 0,
                           request.allowScreencast ? 1 : 0,
                           request.allowInteractivePortal ? 1 : 0,
                           request.allowPortalScreenshotFallback ? 1 : 0, grimPreferred ? 1 : 0,
                           kdeSession ? 1 : 0, kwinAvailable ? 1 : 0, kwinConfigured ? 1 : 0,
                           QGuiApplication::desktopFileName().toUtf8().constData(),
                           desktopEnvironmentText().toUtf8().constData());
    }

    if (isGnomeWaylandSession() && hasGnomeScrollHelper() && request.sourceGeometry.isValid()
        && !request.sourceGeometry.isEmpty() && !request.allOutputs) {
        markshot::debugLog("capture", "【Wayland捕获】【后端路由】route=gnome-scroll-helper");
        CaptureResult gnomeCapture = captureWithGnomeScrollHelper(request);
        if (!gnomeCapture.image.isNull()) {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】gnome-scroll-helper-ok frame=%dx%d",
                               gnomeCapture.image.width(), gnomeCapture.image.height());
            return gnomeCapture;
        }
        markshot::debugLog("capture", "【Wayland捕获】【后端路由】gnome-scroll-helper-failed (falling back) error=%s",
                           gnomeCapture.error.toUtf8().constData());
    }

    // 1. 【Wayland捕获】【KWin路由】流已停用时直接使用原生截图，健康流仍保持优先
    const bool preferScreencast = request.preferScreencast && request.allowScreencast;
    const bool kwinAllowedForRequest =
        markshot::kwinScreenShotSupportsOwnWindowPolicy(request.hideOwnWindows,
                                                         preferScreencast);
    const bool kwinEligible = kwinConfigured && (kdeSession || kwinAvailable)
        && request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty();
    if (kwinEligible && kwinAllowedForRequest) {
        markshot::debugLog("capture", "【Wayland捕获】【后端路由】route=kwin-screenshot kde=%d kwin=%d all_outputs=%d",
                           kdeSession ? 1 : 0, kwinAvailable ? 1 : 0,
                           request.allOutputs ? 1 : 0);
        CaptureResult kwinCapture = captureWithKWinScreenShot(request);
        if (!kwinCapture.image.isNull()) {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】kwin-screenshot-ok frame=%dx%d",
                               kwinCapture.image.width(), kwinCapture.image.height());
            return kwinCapture;
        }
        markshot::debugLog("capture", "【Wayland捕获】【后端路由】kwin-screenshot-failed (falling back) error=%s",
                           kwinCapture.error.toUtf8().constData());
    } else if (kdeSession || kwinAvailable) {
        markshot::debugLog("capture",
                           "【Wayland捕获】【KWin路由】skipped configured=%d hide_own_windows=%d "
                           "prefer_screencast=%d",
                           kwinConfigured ? 1 : 0,
                           request.hideOwnWindows ? 1 : 0,
                           request.preferScreencast ? 1 : 0);
    }

    if (preferScreencast) {
        markshot::debugLog("capture", "【Wayland捕获】【后端路由】route=screencast (preferScreencast)");
        CaptureResult screencastCapture = captureWithPortalScreencast(request);
        if (!screencastCapture.image.isNull()) {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】screencast-ok frame=%dx%d",
                               screencastCapture.image.width(), screencastCapture.image.height());
            return screencastCapture;
        }
        markshot::debugLog("capture", "【Wayland捕获】【后端路由】screencast-failed error=%s",
                           screencastCapture.error.toUtf8().constData());
        stopPortalScreencast();

        // 2. 【Wayland捕获】【流回退】优先尝试非交互原生后端，保持配置与自身窗口策略
        CaptureResult kwinCapture;
        if (kwinEligible) {
            markshot::debugLog("capture", "【Wayland捕获】【流回退】fallback=kwin-screenshot");
            kwinCapture = captureWithKWinScreenShot(request);
            kwinCapture.screencastFailed = true;
            if (!kwinCapture.image.isNull()) {
                return kwinCapture;
            }
            markshot::debugLog("capture", "【Wayland捕获】【流回退】kwin-failed error=%s",
                               kwinCapture.error.toUtf8().constData());
        }

        markshot::debugLog("capture", "【Wayland捕获】【流回退】fallback=grim");
        CaptureResult grimCapture = captureWithGrim(request);
        grimCapture.screencastFailed = true;
        if (!grimCapture.image.isNull()) {
            markshot::debugLog("capture", "【Wayland捕获】【流回退】grim-ok frame=%dx%d",
                               grimCapture.image.width(), grimCapture.image.height());
            return grimCapture;
        }
        markshot::debugLog("capture", "【Wayland捕获】【流回退】grim-failed error=%s",
                           grimCapture.error.toUtf8().constData());

        CaptureResult portalCapture;
        if (request.allowPortalScreenshotFallback) {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】fallback=portal-screenshot");
            portalCapture = captureWithPortalScreenshot(request);
            portalCapture.screencastFailed = true;
            if (!portalCapture.image.isNull()) {
                markshot::debugLog("capture", "【Wayland捕获】【后端路由】portal-screenshot-ok frame=%dx%d",
                                   portalCapture.image.width(), portalCapture.image.height());
                return portalCapture;
            }
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】portal-screenshot-failed error=%s",
                               portalCapture.error.toUtf8().constData());
        } else {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】portal-screenshot-fallback disabled");
        }

        // 3. 【Wayland捕获】【首次授权】所有非交互路径失败后，由调用方限制为首帧授权
        if (!request.allowInteractivePortal && request.allowInteractiveScreencastInit) {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】fallback=interactive-screencast-init");
            CaptureRequest interactiveRequest = request;
            interactiveRequest.allowInteractivePortal = true;
            CaptureResult interactiveCapture = captureWithPortalScreencast(interactiveRequest);
            if (!interactiveCapture.image.isNull()) {
                markshot::debugLog("capture", "【Wayland捕获】【后端路由】interactive-screencast-init-ok frame=%dx%d",
                                   interactiveCapture.image.width(),
                                   interactiveCapture.image.height());
                return interactiveCapture;
            }
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】interactive-screencast-init-failed error=%s",
                               interactiveCapture.error.toUtf8().constData());
            stopPortalScreencast();
            screencastCapture.error = interactiveCapture.error;
        }

        markshot::debugLog("capture", "【Wayland捕获】【后端路由】all-routes-exhausted");
        CaptureResult failure;
        failure.sourceGeometry = request.sourceGeometry;
        failure.screencastFailed = true;
        failure.error =
            QStringLiteral("%1\nPortal screenshot fallback: %2\nGrim fallback: %3")
                .arg(screencastCapture.error,
                     request.allowPortalScreenshotFallback
                         ? portalCapture.error
                         : QStringLiteral("disabled for live capture"),
                     grimCapture.error);
        if (!kwinCapture.error.isEmpty()) {
            failure.error += QStringLiteral("\nKWin fallback: %1").arg(kwinCapture.error);
        }
        return failure;
    }

    if (grimPreferred) {
        markshot::debugLog("capture", "【Wayland捕获】【后端路由】route=grim (prefersGrim)");
        CaptureResult grimCapture = captureWithGrim(request);
        if (!grimCapture.image.isNull()) {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】grim-ok frame=%dx%d",
                               grimCapture.image.width(), grimCapture.image.height());
            return grimCapture;
        }

        // 4. 【Wayland捕获】【截图回退】门户可能创建临时窗口，记录原生后端失败以便排查布局变化
        markshot::debugLog("capture",
                           "【Wayland捕获】【后端路由】grim-failed error=%s (portal fallback is slow and may reflow tiled windows)",
                           grimCapture.error.toUtf8().constData());

        if (!request.allowPortalScreenshotFallback) {
            markshot::debugLog("capture", "【Wayland捕获】【Portal截图回退】已禁用");
            return {{},
                    QStringLiteral("%1\nPortal fallback: disabled for live capture").arg(grimCapture.error),
                    {},
                    request.sourceGeometry};
        }

        markshot::debugLog("capture", "【Wayland捕获】【后端路由】fallback=portal-screenshot");
        CaptureResult portalCapture = captureWithPortalScreenshot(request);
        if (!portalCapture.image.isNull()) {
            markshot::debugLog("capture", "【Wayland捕获】【后端路由】portal-screenshot-ok frame=%dx%d",
                               portalCapture.image.width(), portalCapture.image.height());
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
