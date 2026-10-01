#include "screen_capture_internal.h"
#include "kde_capture_config.h"
#include "recording/recording_polling_capture_stream.h"

#include <QSignalSpy>
#include <QtTest/QtTest>

namespace {

bool g_kde = false;
bool g_kwinAvailable = false;
bool g_kwinConfigured = true;
bool g_kwinSucceeds = false;
bool g_screencastSucceeds = false;
bool g_interactiveScreencastSucceeds = false;
bool g_grimSucceeds = false;
bool g_portalSucceeds = false;
QStringList g_routes;
QVector<CaptureRequest> g_screencastRequests;
QVector<CaptureRequest> g_kwinRequests;
QVector<CaptureRequest> g_grimRequests;

/**
 * 【采集测试】【后端替身】返回指定区域的帧或后端错误。
 * @param request 实际路由传递的采集请求。
 * @param succeeds 是否模拟采集成功。
 * @param error 失败时返回的错误。
 * @return 测试帧或失败结果。
 */
CaptureResult backendResult(const CaptureRequest &request, bool succeeds, const QString &error)
{
    CaptureResult result;
    result.sourceGeometry = request.sourceGeometry;
    result.outputName = request.preferredOutputName;
    if (succeeds) {
        result.image = QImage(request.sourceGeometry.size(), QImage::Format_ARGB32_Premultiplied);
        result.image.fill(Qt::black);
    } else {
        result.error = error;
    }
    return result;
}

/**
 * 创建与副屏问题报告一致的采集请求。
 * @return 非交互的副屏区域请求。
 */
CaptureRequest secondaryOutputRequest()
{
    CaptureRequest request;
    request.sourceGeometry = QRect(2586, 767, 730, 542);
    request.preferredOutputName = QStringLiteral("HDMI-A-1");
    request.preferScreencast = true;
    request.allowInteractivePortal = false;
    request.allowPortalScreenshotFallback = false;
    request.hideOwnWindows = false;
    return request;
}

}  // namespace

/** @return 测试环境是否使用 KDE。 */
bool isKdePlasma() { return g_kde; }
/** @return 测试环境是否优先使用 grim。 */
bool prefersGrim() { return !g_kde; }
/** @return 测试环境是否提供 KWin 接口。 */
bool isKWinScreenShotAvailable() { return g_kwinAvailable; }
/** @return 固定禁用 GNOME 路由。 */
bool isGnomeWaylandSession() { return false; }
/** @return 固定禁用 GNOME 扩展。 */
bool hasGnomeScrollHelper() { return false; }
/** @return 测试桌面名称。 */
QString desktopEnvironmentText() { return QStringLiteral("test"); }
/** @return 测试配置是否启用 KWin 截图。 */
bool markshot::configuredKdeKWinScreenshotEnabled() { return g_kwinConfigured; }

/** @param request 采集请求。 @return 固定失败的 GNOME 结果。 */
CaptureResult captureWithGnomeScrollHelper(const CaptureRequest &request)
{
    return backendResult(request, false, QStringLiteral("GNOME helper unavailable"));
}

/** @param request 采集请求。 @return 模拟 KWin 成功或失败。 */
CaptureResult captureWithKWinScreenShot(const CaptureRequest &request)
{
    g_routes.append(QStringLiteral("kwin"));
    g_kwinRequests.append(request);
    return backendResult(request, g_kwinSucceeds, QStringLiteral("KWin capture failed"));
}

/** @param request 采集请求。 @return 模拟流不覆盖副屏或正常返回帧。 */
CaptureResult captureWithPortalScreencast(const CaptureRequest &request)
{
    g_routes.append(QStringLiteral("screencast"));
    g_screencastRequests.append(request);
    if (request.allowInteractivePortal && g_interactiveScreencastSucceeds) {
        g_screencastSucceeds = true;
    }
    return backendResult(request,
                         g_screencastSucceeds
                             || (request.allowInteractivePortal && g_interactiveScreencastSucceeds),
                         QStringLiteral("portal screencast frame does not cover requested geometry"));
}

/** @return 无返回值；记录失败会话的关闭。 */
void stopPortalScreencast() { g_routes.append(QStringLiteral("stop")); }

/** @param request 采集请求。 @return 模拟非交互的 grim 采集结果。 */
CaptureResult captureWithGrim(const CaptureRequest &request)
{
    g_routes.append(QStringLiteral("grim"));
    g_grimRequests.append(request);
    return backendResult(request, g_grimSucceeds, QStringLiteral("grim capture failed"));
}

/** @param request 采集请求。 @return 模拟需要逐次确认的门户截图结果。 */
CaptureResult captureWithPortalScreenshot(const CaptureRequest &request)
{
    g_routes.append(QStringLiteral("screenshot"));
    return backendResult(request, g_portalSucceeds, QStringLiteral("portal screenshot failed"));
}

/** @param request 录制流发出的真实请求。 @return 真实 Wayland 路由产生的结果。 */
CaptureResult captureScreenFrame(const CaptureRequest &request)
{
    return captureWaylandFrame(request);
}

class ScreenCaptureWaylandRoutingTest final : public QObject {
    Q_OBJECT

private slots:
    /** @return 无返回值；清空所有后端状态。 */
    void init()
    {
        g_kde = false;
        g_kwinAvailable = false;
        g_kwinConfigured = true;
        g_kwinSucceeds = false;
        g_screencastSucceeds = false;
        g_interactiveScreencastSucceeds = false;
        g_grimSucceeds = false;
        g_portalSucceeds = false;
        g_routes.clear();
        g_screencastRequests.clear();
        g_kwinRequests.clear();
        g_grimRequests.clear();
    }

    /** @return 无返回值；验证流失败后先尝试 KWin，再考虑门户交互。 */
    void kdeFallbackRecoversWithoutPrompt()
    {
        g_kde = true;
        g_kwinSucceeds = true;
        CaptureRequest request = secondaryOutputRequest();
        request.allowInteractiveScreencastInit = true;
        const CaptureResult result = captureWaylandFrame(request);
        QVERIFY2(!result.image.isNull(), qPrintable(result.error));
        QCOMPARE(g_routes, QStringList({"screencast", "stop", "kwin"}));
        QCOMPARE(g_kwinRequests.first().sourceGeometry, request.sourceGeometry);
        QVERIFY(!g_kwinRequests.first().hideOwnWindows);
    }

    /** @return 无返回值；验证可用的流仍然优先于 KWin。 */
    void healthyScreencastRemainsPreferred()
    {
        g_kde = true;
        g_kwinSucceeds = true;
        g_screencastSucceeds = true;
        QVERIFY(!captureWaylandFrame(secondaryOutputRequest()).image.isNull());
        QCOMPARE(g_routes, QStringList({"screencast"}));
    }

    /** @return 无返回值；验证用户禁用 KWin 后不会通过回退重新启用。 */
    void disabledKwinIsNotRetried()
    {
        g_kde = true;
        g_kwinConfigured = false;
        g_kwinSucceeds = true;
        QVERIFY(captureWaylandFrame(secondaryOutputRequest()).image.isNull());
        QVERIFY(g_kwinRequests.isEmpty());
    }

    /** @return 无返回值；验证 KWin 服务探测成功时不依赖桌面名称。 */
    void availableKwinRecoversOutsideKdeEnvironmentName()
    {
        g_kwinAvailable = true;
        g_kwinSucceeds = true;
        CaptureRequest request = secondaryOutputRequest();
        request.hideOwnWindows = true;
        QVERIFY(!captureWaylandFrame(request).image.isNull());
        QCOMPARE(g_kwinRequests.size(), 1);
        QVERIFY(g_kwinRequests.first().hideOwnWindows);
    }

    /** @return 无返回值；验证无效区域不会发送到 KWin。 */
    void emptyGeometrySkipsKwin()
    {
        g_kde = true;
        g_kwinSucceeds = true;
        CaptureRequest request = secondaryOutputRequest();
        request.sourceGeometry = {};
        QVERIFY(captureWaylandFrame(request).image.isNull());
        QVERIFY(g_kwinRequests.isEmpty());
    }

    /** @return 无返回值；验证 KWin 失败继续回退并保留诊断信息。 */
    void failedKwinContinuesFallback()
    {
        g_kde = true;
        const CaptureResult result = captureWaylandFrame(secondaryOutputRequest());
        QVERIFY(result.image.isNull());
        QCOMPARE(g_routes, QStringList({"screencast", "stop", "kwin", "grim"}));
        QVERIFY(result.error.contains(QStringLiteral("KWin capture failed")));
    }

    /** @return 无返回值；验证 grim 成功时无需请求交互式截图。 */
    void nativeFallbackPrecedesPortalScreenshot()
    {
        g_grimSucceeds = true;
        g_portalSucceeds = true;
        CaptureRequest request = secondaryOutputRequest();
        request.allowInteractivePortal = true;
        request.allowPortalScreenshotFallback = true;
        const CaptureResult result = captureWaylandFrame(request);
        QVERIFY(!result.image.isNull());
        QCOMPARE(g_routes, QStringList({"screencast", "stop", "grim"}));
        QCOMPARE(result.sourceGeometry, request.sourceGeometry);
        QCOMPARE(result.outputName, request.preferredOutputName);
    }

    /** @return 无返回值；覆盖 GIF 与视频两种轮询模式。 */
    void secondaryOutputPollingDoesNotReopenPortal_data()
    {
        QTest::addColumn<int>("mode");
        QTest::newRow("gif") << static_cast<int>(markshot::recording::RecordingMode::Gif);
        QTest::newRow("video") << static_cast<int>(markshot::recording::RecordingMode::Video);
    }

    /** @return 无返回值；验证副屏连续采集只尝试一次失败流且不弹截图门户。 */
    void secondaryOutputPollingDoesNotReopenPortal()
    {
        QFETCH(int, mode);
        g_grimSucceeds = true;
        g_portalSucceeds = true;
        markshot::recording::RecordingOptions options;
        options.mode = static_cast<markshot::recording::RecordingMode>(mode);
        options.fps = 30;
        options.captureGeometry = secondaryOutputRequest().sourceGeometry;
        options.display.outputName = QStringLiteral("HDMI-A-1");
        markshot::recording::RecordingPollingCaptureStream stream(options);
        QSignalSpy frames(&stream, &markshot::recording::RecordingCaptureStream::frameReady);
        QSignalSpy errors(&stream, &markshot::recording::RecordingCaptureStream::failed);
        QString error;
        QVERIFY(stream.start(&error));
        QTRY_VERIFY_WITH_TIMEOUT(frames.size() >= 3, 1000);
        stream.stop();
        QCOMPARE(errors.size(), 0);
        QCOMPARE(g_routes.count(QStringLiteral("screenshot")), 0);
        QCOMPARE(g_screencastRequests.size(), 1);
        QVERIFY(!g_screencastRequests.first().allowInteractivePortal);
        QCOMPARE(g_grimRequests.size(), frames.size());
        for (const CaptureRequest &request : std::as_const(g_grimRequests)) {
            QCOMPARE(request.sourceGeometry, options.captureGeometry);
            QCOMPARE(request.preferredOutputName, options.display.outputName);
        }

        // 1. 【录制】【会话重启】下一次录制允许重新探测流能力
        QVERIFY(stream.start(&error));
        QTRY_COMPARE_WITH_TIMEOUT(g_screencastRequests.size(), 2, 1000);
        stream.stop();
    }

    /** @return 无返回值；验证无原生后端时仅首次授权，后续复用已建立的流。 */
    void gifAuthorizesOnlyOnceAfterNativeBackendsFail()
    {
        g_interactiveScreencastSucceeds = true;
        markshot::recording::RecordingOptions options;
        options.fps = 30;
        options.captureGeometry = secondaryOutputRequest().sourceGeometry;
        markshot::recording::RecordingPollingCaptureStream stream(options);
        QSignalSpy frames(&stream, &markshot::recording::RecordingCaptureStream::frameReady);
        QString error;
        QVERIFY(stream.start(&error));
        QTRY_VERIFY_WITH_TIMEOUT(frames.size() >= 3, 1000);
        stream.stop();
        QCOMPARE(g_routes.count(QStringLiteral("screenshot")), 0);
        QCOMPARE(g_routes.count(QStringLiteral("grim")), 1);
        QCOMPARE(g_screencastRequests.size(), frames.size() + 1);
        QCOMPARE(std::count_if(g_screencastRequests.cbegin(), g_screencastRequests.cend(),
                               [](const CaptureRequest &request) { return request.allowInteractivePortal; }), 1);
    }

    /** @return 无返回值；验证视频模式无可用后端时失败，不请求交互授权。 */
    void videoFailsWithoutInteractiveFallback()
    {
        g_interactiveScreencastSucceeds = true;
        markshot::recording::RecordingOptions options;
        options.mode = markshot::recording::RecordingMode::Video;
        options.captureGeometry = secondaryOutputRequest().sourceGeometry;
        markshot::recording::RecordingPollingCaptureStream stream(options);
        QSignalSpy errors(&stream, &markshot::recording::RecordingCaptureStream::failed);
        QSignalSpy frames(&stream, &markshot::recording::RecordingCaptureStream::frameReady);
        QString error;
        QVERIFY(stream.start(&error));
        QTRY_COMPARE_WITH_TIMEOUT(errors.size(), 1, 1000);
        stream.stop();
        QCOMPARE(frames.size(), 0);
        QCOMPARE(g_screencastRequests.size(), 1);
        QVERIFY(!g_screencastRequests.first().allowInteractivePortal);
        QCOMPARE(g_routes.count(QStringLiteral("screenshot")), 0);
    }

    /** @return 无返回值；验证已出帧的流失效后只回退一次，不重新请求授权。 */
    void failedRunningStreamStaysOnNativeFallback()
    {
        g_screencastSucceeds = true;
        g_grimSucceeds = true;
        markshot::recording::RecordingOptions options;
        options.fps = 30;
        options.captureGeometry = secondaryOutputRequest().sourceGeometry;
        markshot::recording::RecordingPollingCaptureStream stream(options);
        connect(&stream, &markshot::recording::RecordingCaptureStream::frameReady,
                this, [] { g_screencastSucceeds = false; });
        QSignalSpy frames(&stream, &markshot::recording::RecordingCaptureStream::frameReady);
        QString error;
        QVERIFY(stream.start(&error));
        QTRY_VERIFY_WITH_TIMEOUT(frames.size() >= 4, 1000);
        stream.stop();
        QCOMPARE(g_screencastRequests.size(), 2);
        QVERIFY(!g_screencastRequests.last().allowInteractiveScreencastInit);
        QCOMPARE(g_grimRequests.size(), frames.size() - 1);
    }
};

QTEST_MAIN(ScreenCaptureWaylandRoutingTest)
#include "screen_capture_wayland_routing_test.moc"
