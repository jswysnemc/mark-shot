#include "annotation_launch.h"

#include "shot_window.h"
#include "platform/wayland/compositor_pointer.h"
#include "window_detection_session.h"

#include <QCursor>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QScreen>
#include <QTimer>

namespace markshot {

namespace {

/// @brief Retrieves the name of the currently focused output in the niri window manager.
/// @return The name of the focused output, or an empty string if not running on niri or on error.
QString niriFocusedOutputName()
{
#if defined(Q_OS_WIN)
    return {};
#else
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
#endif
}

/// @brief Retrieves the name of the currently focused output in the Umbriel compositor.
/// @return The name of the focused output, or an empty string if not running on Umbriel or on error.
QString umbrielFocusedOutputName()
{
#if defined(Q_OS_WIN)
    return {};
#else
    if (!qEnvironmentVariableIsSet("UMBRIEL_SOCKET")) {
        return {};
    }
    // `subscribe` emits the current state as its first JSON line and then keeps
    // streaming, so read one line and stop the process.
    QProcess umbriel;
    umbriel.setProgram(QStringLiteral("umbriel"));
    umbriel.setArguments({QStringLiteral("subscribe"), QStringLiteral("outputs")});
    umbriel.start(QIODevice::ReadOnly);
    if (!umbriel.waitForStarted(1000) || !umbriel.waitForReadyRead(1000)) {
        umbriel.kill();
        return {};
    }
    const QByteArray line = umbriel.readLine();
    umbriel.kill();
    umbriel.waitForFinished(1000);

    const QJsonDocument document = QJsonDocument::fromJson(line);
    const QJsonArray outputs = document.object().value(QStringLiteral("data")).toArray();
    for (const QJsonValue &value : outputs) {
        const QJsonObject output = value.toObject();
        if (output.value(QStringLiteral("focused")).toBool()) {
            return output.value(QStringLiteral("name")).toString();
        }
    }
    return {};
#endif
}

/// @brief Finds a QScreen object by its name.
/// @param name The name of the screen to find.
/// @return A pointer to the QScreen if found, or nullptr if not found or if the name is empty.
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

}  // namespace

QScreen *focusedScreen()
{
    // 1. 【截图会话】【屏幕选择】GNOME 全局快捷键启动时优先使用扩展提供的真实指针位置
    if (const auto pointer = wayland::compositorPointerPosition()) {
        if (QScreen *screen = QGuiApplication::screenAt(*pointer)) {
            return screen;
        }
    }
    // 2. 【截图会话】【屏幕选择】仅在 niri 会话询问聚焦输出，其他桌面不调用 niri
    const auto session = window_detection::detectSession(QProcessEnvironment::systemEnvironment());
    if (session.compositor == QStringLiteral("niri")) {
        if (QScreen *screen = screenByName(niriFocusedOutputName())) {
            return screen;
        }
    }
    // 2.1 【截图会话】【屏幕选择】Umbriel 会话询问聚焦输出
    if (QScreen *screen = screenByName(umbrielFocusedOutputName())) {
        return screen;
    }
    // 3. 【截图会话】【屏幕选择】保留 X11 和已收到客户端指针事件时的 Qt 回退
    if (QScreen *screen = QGuiApplication::screenAt(QCursor::pos())) {
        return screen;
    }
    return QGuiApplication::primaryScreen();
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
