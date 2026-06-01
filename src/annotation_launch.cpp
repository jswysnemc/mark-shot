#include "annotation_launch.h"

#include "shot_window.h"

#include <QCursor>
#include <QGuiApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QScreen>
#include <QTimer>

namespace markshot {

namespace {

QString niriFocusedOutputName()
{
    QProcess niri;
    niri.setProgram(QStringLiteral("niri"));
    niri.setArguments({QStringLiteral("msg"), QStringLiteral("-j"), QStringLiteral("focused-output")});
    niri.start(QIODevice::ReadOnly);
    if (!niri.waitForStarted(1000) || !niri.waitForFinished(1000)) {
        return {};
    }
    if (niri.exitStatus() != QProcess::NormalExit || niri.exitCode() != 0) {
        return {};
    }

    const QJsonDocument document = QJsonDocument::fromJson(niri.readAllStandardOutput());
    if (!document.isObject()) {
        return {};
    }
    return document.object().value(QStringLiteral("name")).toString();
}

QScreen *screenByName(const QString &name)
{
    if (name.isEmpty()) {
        return nullptr;
    }
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (screen && screen->name() == name) {
            return screen;
        }
    }
    return nullptr;
}

QRect centeredImageWindowGeometry(const QSize &imageSize, QScreen *screen)
{
    if (imageSize.isEmpty()) {
        return {};
    }

    const QRect availableGeometry = screen
        ? screen->availableGeometry()
        : QRect(QPoint(0, 0), imageSize);
    QSize targetSize = imageSize;
    const QSize maxSize(qMax(320, qRound(availableGeometry.width() * 0.9)),
                        qMax(240, qRound(availableGeometry.height() * 0.9)));
    if (targetSize.width() > maxSize.width() || targetSize.height() > maxSize.height()) {
        targetSize.scale(maxSize, Qt::KeepAspectRatio);
    }

    const QPoint topLeft(availableGeometry.center().x() - targetSize.width() / 2,
                         availableGeometry.center().y() - targetSize.height() / 2);
    return QRect(topLeft, targetSize);
}

}  // namespace

QScreen *focusedScreen()
{
    if (QScreen *screen = screenByName(niriFocusedOutputName())) {
        return screen;
    }
    if (QScreen *screen = QGuiApplication::screenAt(QCursor::pos())) {
        return screen;
    }
    return QGuiApplication::primaryScreen();
}

ShotWindow *openImageForAnnotation(const QImage &image, const QString &name)
{
    QScreen *screen = focusedScreen();
    auto *window = new ShotWindow(image, name);
    if (screen) {
        window->setScreen(screen);
    }
    window->setWindowFlags(Qt::Window);
    const QRect windowGeometry = centeredImageWindowGeometry(image.size(), screen);
    if (windowGeometry.isValid() && !windowGeometry.isEmpty()) {
        window->setGeometry(windowGeometry);
    }
    window->setImageNavigationEnabled(true);
    window->show();
    window->raise();
    window->activateWindow();
    QTimer::singleShot(0, window, [window] {
        window->startFullscreenAnnotation();
    });
    return window;
}

}  // namespace markshot
