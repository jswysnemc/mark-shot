#include "screen_capture.h"
#include "shot_window.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QCommandLineParser>
#include <QCursor>
#include <QFileInfo>
#include <QGuiApplication>
#include <QImageReader>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMessageBox>
#include <QProcess>
#include <QScreen>
#include <QTimer>

namespace {

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

} // namespace

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("mark-shot"));
    QApplication::setApplicationDisplayName(QStringLiteral("Mark Shot"));
    QApplication::setApplicationVersion(QStringLiteral("0.1.11"));
    // Matches mark-shot.desktop so the desktop portal can attribute screenshot
    // requests to a stable app id and persist a granted permission.
    QGuiApplication::setDesktopFileName(QStringLiteral("mark-shot"));

    markshot::i18n::initialize();

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Wayland screenshot selection and annotation tool for niri."));
    parser.addHelpOption();
    parser.addVersionOption();
    parser.addPositionalArgument(QStringLiteral("file"), QStringLiteral("Open an existing image file for annotation instead of capturing the screen."), QStringLiteral("[file]"));
    QCommandLineOption allOutputsOption(QStringLiteral("all-outputs"), QStringLiteral("Capture all outputs instead of the current Qt screen."));
    QCommandLineOption xdgWindowOption(QStringLiteral("xdg-window"), QStringLiteral("Use a regular fullscreen xdg window instead of layer-shell."));
    QCommandLineOption fullscreenAnnotationOption({QStringLiteral("fullscreen"), QStringLiteral("full-screen")},
                                                  QStringLiteral("Skip region selection and annotate the full captured frame."));
    parser.addOption(allOutputsOption);
    parser.addOption(xdgWindowOption);
    parser.addOption(fullscreenAnnotationOption);
    parser.process(app);

    const QStringList positionalArguments = parser.positionalArguments();
    if (positionalArguments.size() > 1) {
        QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), MS_TR("Only one image file can be opened at a time."));
        return 1;
    }

    QScreen *screen = focusedScreen();
    const QString imagePath = positionalArguments.isEmpty() ? QString() : positionalArguments.first();
    const bool fileMode = !imagePath.isEmpty();
    if (fileMode) {
        QFileInfo imageFile(imagePath);
        if (!imageFile.exists() || !imageFile.isFile()) {
            QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), MS_TR("Image file does not exist: %1").arg(imagePath));
            return 1;
        }

        QImageReader reader(imageFile.absoluteFilePath());
        reader.setAutoTransform(true);
        const QImage image = reader.read();
        if (image.isNull()) {
            QMessageBox::critical(nullptr,
                                  QStringLiteral("Mark Shot"),
                                  MS_TR("Failed to load image: %1\n%2").arg(imageFile.absoluteFilePath(), reader.errorString()));
            return 1;
        }

        ShotWindow *window = new ShotWindow(image, imageFile.fileName());
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
        return QApplication::exec();
    }

    const bool allOutputs = parser.isSet(allOutputsOption);
    const QRect captureGeometry = allOutputs ? virtualScreensGeometry() : (screen ? screen->geometry() : QRect());
    const QString outputName = (!allOutputs && screen) ? screen->name() : QString();
    CaptureResult capture = captureScreenFrame({outputName, captureGeometry, allOutputs});
    if (capture.image.isNull()) {
        QMessageBox::critical(nullptr, QStringLiteral("Mark Shot"), capture.error);
        return 1;
    }

    const QRect sourceGeometry = capture.sourceGeometry.isValid() && !capture.sourceGeometry.isEmpty()
        ? capture.sourceGeometry
        : captureGeometry;
    ShotWindow *window = new ShotWindow(capture.image, capture.outputName, sourceGeometry);
    if (screen && !allOutputs) {
        window->setScreen(screen);
    }

    const bool useRegularWindow = parser.isSet(xdgWindowOption);
    const bool layerShellReady = !allOutputs && !useRegularWindow && window->configureLayerShell(screen);
    if (layerShellReady) {
        window->show();
    } else {
        if (capture.sourceGeometry.isValid() && !capture.sourceGeometry.isEmpty()) {
            window->setGeometry(capture.sourceGeometry);
        }
        if (allOutputs) {
            window->show();
        } else {
            window->showFullScreen();
        }
        window->raise();
        window->activateWindow();
    }
    if (parser.isSet(fullscreenAnnotationOption)) {
        QTimer::singleShot(0, window, [window] {
            window->startFullscreenAnnotation();
        });
    }

    return QApplication::exec();
}
