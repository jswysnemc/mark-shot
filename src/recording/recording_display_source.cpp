#include "recording/recording_display_source.h"

#include "ui/i18n.h"

#include <QGuiApplication>
#include <QScreen>

namespace markshot::recording {
namespace {

/**
 * 计算全部显示器组成的虚拟桌面几何。
 * @return 虚拟桌面几何。
 */
QRect virtualScreensGeometry()
{
    QRect geometry;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen || screen->geometry().isEmpty()) {
            continue;
        }
        geometry = geometry.isNull() ? screen->geometry() : geometry.united(screen->geometry());
    }
    return geometry;
}

/**
 * 按屏幕对象构建录制来源。
 * @param screen 屏幕对象。
 * @param index 屏幕序号。
 * @return 录制来源。
 */
DisplaySource sourceFromScreen(QScreen *screen, int index)
{
    DisplaySource source;
    source.screenName = screen ? screen->name() : QString();
    source.outputName = source.screenName.isEmpty()
        ? QStringLiteral("display-%1").arg(index + 1)
        : source.screenName;
    source.title = source.screenName.isEmpty()
        ? MS_TR("Display %1").arg(index + 1)
        : source.screenName;
    source.geometry = screen ? screen->geometry() : QRect();
    return source;
}

}  // namespace

QVector<DisplaySource> availableDisplaySources()
{
    QVector<DisplaySource> sources;
    const QList<QScreen *> screens = QGuiApplication::screens();
    const QRect virtualGeometry = virtualScreensGeometry();
    if (screens.size() > 1 && !virtualGeometry.isEmpty()) {
        DisplaySource allDisplays;
        allDisplays.allOutputs = true;
        allDisplays.outputName = QStringLiteral("all-displays");
        allDisplays.title = MS_TR("All Displays");
        allDisplays.geometry = virtualGeometry;
        sources.append(allDisplays);
    }

    for (int i = 0; i < screens.size(); ++i) {
        QScreen *screen = screens.at(i);
        if (!screen || screen->geometry().isEmpty()) {
            continue;
        }
        sources.append(sourceFromScreen(screen, i));
    }

    return sources;
}

}  // namespace markshot::recording
