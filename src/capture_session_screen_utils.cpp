#include "capture_session_screen_utils.h"

#include "debug_log.h"

#include <QGuiApplication>
#include <QScreen>

#include <cmath>
#include <optional>

namespace markshot::capture_session {

const char *freezeScopeDebugName(CaptureFreezeScope scope)
{
    switch (scope) {
    case CaptureFreezeScope::CursorScreen:
        return "cursor-screen";
    case CaptureFreezeScope::AllScreens:
        return "all-screens";
    }
    return "unknown";
}

bool isWaylandPlatform()
{
    return QGuiApplication::platformName().compare(QStringLiteral("wayland"),
                                                   Qt::CaseInsensitive) == 0;
}

bool hasMixedDevicePixelRatios(const QList<QScreen *> &screens)
{
    std::optional<qreal> firstRatio;
    for (QScreen *screen : screens) {
        if (!screen || screen->geometry().isEmpty()) {
            continue;
        }

        const qreal ratio = screen->devicePixelRatio();
        if (!firstRatio.has_value()) {
            firstRatio = ratio;
            continue;
        }
        if (std::abs(*firstRatio - ratio) > 0.01) {
            return true;
        }
    }
    return false;
}

bool shouldCaptureScreensIndividually(const QList<QScreen *> &screens)
{
    return isWaylandPlatform() && screens.size() > 1;
}

void logCaptureSessionScreens(const QList<QScreen *> &screens)
{
    int index = 0;
    for (QScreen *screen : screens) {
        if (!screen) {
            markshot::debugLog("capture-session",
                               "【截图会话】【缩放诊断】screen index=%d null=1",
                               index++);
            continue;
        }

        const QRect geometry = screen->geometry();
        const QRect available = screen->availableGeometry();
        markshot::debugLog("capture-session",
                           "【截图会话】【缩放诊断】screen index=%d name=%s geom=%d,%d %dx%d "
                           "available=%d,%d %dx%d dpr=%.3f logical_dpi=%.3fx%.3f "
                           "physical_dpi=%.3fx%.3f refresh=%.3f",
                           index++,
                           screen->name().toUtf8().constData(),
                           geometry.x(), geometry.y(), geometry.width(), geometry.height(),
                           available.x(), available.y(), available.width(), available.height(),
                           screen->devicePixelRatio(),
                           screen->logicalDotsPerInchX(), screen->logicalDotsPerInchY(),
                           screen->physicalDotsPerInchX(), screen->physicalDotsPerInchY(),
                           screen->refreshRate());
    }
}

QRect virtualScreensGeometry()
{
    QRect geometry;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen) {
            continue;
        }
        geometry = geometry.isNull() ? screen->geometry() : geometry.united(screen->geometry());
    }
    return geometry;
}

QScreen *screenByName(const QString &screenName)
{
    if (screenName.isEmpty()) {
        return nullptr;
    }
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (screen && screen->name() == screenName) {
            return screen;
        }
    }
    return nullptr;
}

}  // namespace markshot::capture_session
