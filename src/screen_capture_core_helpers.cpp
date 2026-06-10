#include "screen_capture_internal.h"

bool isWaylandSession()
{
    const QString sessionType = QProcessEnvironment::systemEnvironment().value(QStringLiteral("XDG_SESSION_TYPE")).toLower();
    return sessionType == QStringLiteral("wayland");
}

QString desktopEnvironmentText()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    return (env.value(QStringLiteral("XDG_CURRENT_DESKTOP")) + QLatin1Char(':')
            + env.value(QStringLiteral("XDG_SESSION_DESKTOP")) + QLatin1Char(':')
            + env.value(QStringLiteral("DESKTOP_SESSION")) + QLatin1Char(':')
            + env.value(QStringLiteral("WAYLAND_DISPLAY")))
        .toLower();
}

bool prefersGrim()
{
    const QString desktop = desktopEnvironmentText();
    return desktop.contains(QStringLiteral("sway"))
        || desktop.contains(QStringLiteral("hyprland"))
        || desktop.contains(QStringLiteral("niri"))
        || desktop.contains(QStringLiteral("river"))
        || desktop.contains(QStringLiteral("wayfire"))
        || desktop.contains(QStringLiteral("labwc"))
        || desktop.contains(QStringLiteral("wlroots"));
}

// KWin exposes org.kde.KWin.ScreenShot2, which can capture an exact pixel
// region server-side with no portal dialog. That is the right backend for live
// scrolling capture on Plasma; the portal ScreenCast path forces a source
// picker and reports a virtual-desktop geometry that does not match KWin's real
// outputs, so its crop math is unreliable here.
bool isKdePlasma()
{
    const QString desktop = desktopEnvironmentText();
    return desktop.contains(QStringLiteral("kde")) || desktop.contains(QStringLiteral("plasma"));
}

QRect virtualScreensGeometry()
{
    QRect geometry;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        geometry = geometry.isNull() ? screen->geometry() : geometry.united(screen->geometry());
    }
    return geometry;
}

QImage normalizeCaptureImage(QImage image)
{
    if (!image.isNull()) {
        image.setDevicePixelRatio(1.0);
    }
    return image;
}

CaptureResult captureAllScreensWithQScreen(const CaptureRequest &request)
{
    QRect frameGeometry = request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()
        ? request.sourceGeometry.normalized()
        : virtualScreensGeometry();
    if (frameGeometry.isEmpty()) {
        return {{}, QStringLiteral("no virtual screen geometry available for capture"), {}, {}};
    }

    QImage combined(frameGeometry.size(), QImage::Format_ARGB32_Premultiplied);
    combined.fill(Qt::transparent);

    QPainter painter(&combined);
    painter.setCompositionMode(QPainter::CompositionMode_Source);

    int capturedScreens = 0;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen) {
            continue;
        }

        const QRect screenGeometry = screen->geometry();
        const QRect overlap = screenGeometry.intersected(frameGeometry);
        if (overlap.isEmpty()) {
            continue;
        }

        const QPixmap pixmap = screen->grabWindow(0);
        if (pixmap.isNull()) {
            markshot::debugLog("capture",
                               "qscreen-all screen=%s returned null pixmap",
                               screen->name().toUtf8().constData());
            continue;
        }

        const QImage image = pixmap.toImage().convertToFormat(QImage::Format_ARGB32_Premultiplied);
        const QRect sourceRect = markshot::capture::scaledCropRect(screenGeometry, overlap, image.size());
        if (sourceRect.isEmpty()) {
            continue;
        }

        const QRect destinationRect(overlap.topLeft() - frameGeometry.topLeft(), overlap.size());
        painter.drawImage(destinationRect, image, sourceRect);
        ++capturedScreens;
    }
    painter.end();

    if (capturedScreens == 0) {
        return {{}, QStringLiteral("QScreen::grabWindow returned no usable screen pixmaps"), {}, frameGeometry};
    }

    markshot::debugLog("capture",
                       "qscreen-all screens=%d virtual_geom=%d,%d %dx%d result=%dx%d",
                       capturedScreens,
                       frameGeometry.x(), frameGeometry.y(),
                       frameGeometry.width(), frameGeometry.height(),
                       combined.width(), combined.height());
    return {combined, {}, {}, frameGeometry};
}

CaptureResult captureWithQScreen(const CaptureRequest &request)
{
    if (request.allOutputs) {
        return captureAllScreensWithQScreen(request);
    }

    QScreen *screen = nullptr;
    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        screen = QGuiApplication::screenAt(request.sourceGeometry.center());
    }
    if (!screen) {
        screen = QGuiApplication::primaryScreen();
    }
    if (!screen) {
        return {{}, QStringLiteral("no screen available for capture"), {}, {}};
    }

    QPixmap pixmap = screen->grabWindow(0);
    if (pixmap.isNull()) {
        return {{}, QStringLiteral("QScreen::grabWindow returned null pixmap"), {}, {}};
    }

    const QImage rawImage = pixmap.toImage();
    QImage image = rawImage.convertToFormat(QImage::Format_ARGB32_Premultiplied);

    QRect cropRect;
    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        const QRect requested = request.sourceGeometry.normalized();
        const QRect screenGeometry = screen->geometry();
        const QRect overlap = requested.intersected(screenGeometry);
        if (!overlap.isEmpty()) {
            const qreal scaleX = static_cast<qreal>(image.width()) / std::max(1, screenGeometry.width());
            const qreal scaleY = static_cast<qreal>(image.height()) / std::max(1, screenGeometry.height());
            const int left = static_cast<int>(std::lround((overlap.left() - screenGeometry.left()) * scaleX));
            const int top = static_cast<int>(std::lround((overlap.top() - screenGeometry.top()) * scaleY));
            const int right = static_cast<int>(std::lround((overlap.right() + 1 - screenGeometry.left()) * scaleX));
            const int bottom = static_cast<int>(std::lround((overlap.bottom() + 1 - screenGeometry.top()) * scaleY));
            cropRect = QRect(QPoint(left, top), QPoint(right - 1, bottom - 1)).intersected(image.rect());
        }
        if (cropRect.isEmpty()) {
            return {{}, QStringLiteral("QScreen capture does not cover requested geometry"), {}, request.sourceGeometry};
        }
        image = image.copy(cropRect);
    }

    markshot::debugLog("capture",
                       "qscreen frame screen=%s screen_geom=%d,%d %dx%d "
                       "pixmap=%dx%d pixmap_dpr=%.3f image=%dx%d requested=%d,%d %dx%d "
                       "crop=%d,%d %dx%d result=%dx%d",
                       screen->name().toUtf8().constData(),
                       screen->geometry().x(), screen->geometry().y(),
                       screen->geometry().width(), screen->geometry().height(),
                       pixmap.width(), pixmap.height(), pixmap.devicePixelRatioF(),
                       rawImage.width(), rawImage.height(),
                       request.sourceGeometry.x(), request.sourceGeometry.y(),
                       request.sourceGeometry.width(), request.sourceGeometry.height(),
                       cropRect.x(), cropRect.y(), cropRect.width(), cropRect.height(),
                       image.width(), image.height());
    return {image, {}, {}, request.sourceGeometry};
}

QRect screenGeometryForOutputName(const QString &outputName)
{
    if (outputName.isEmpty()) {
        return {};
    }

    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (screen && screen->name() == outputName) {
            return screen->geometry();
        }
    }
    return {};
}

QRect screenGeometryForRequest(const CaptureRequest &request)
{
    if (!request.preferredOutputName.isEmpty()) {
        const QRect outputGeometry = screenGeometryForOutputName(request.preferredOutputName);
        if (!outputGeometry.isEmpty()) {
            return outputGeometry;
        }
    }

    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        if (QScreen *screen = QGuiApplication::screenAt(request.sourceGeometry.center())) {
            return screen->geometry();
        }
    }
    return {};
}

QRect fullGrimSourceGeometry(const CaptureRequest &request)
{
    if (request.allOutputs && request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        return request.sourceGeometry.normalized();
    }

    const QRect virtualGeometry = virtualScreensGeometry();
    if (!virtualGeometry.isEmpty()) {
        return virtualGeometry;
    }
    if (request.sourceGeometry.isValid() && !request.sourceGeometry.isEmpty()) {
        return request.sourceGeometry.normalized();
    }
    return {};
}

CaptureResult runGrim(const QStringList &arguments, const QString &outputName, QRect sourceGeometry, bool cursorIncluded)
{
    QProcess grim;
    grim.setProgram(QStringLiteral("grim"));
    grim.setArguments(arguments);
    grim.start(QIODevice::ReadOnly);

    if (!grim.waitForStarted(3000)) {
        return {{}, QStringLiteral("failed to start grim; install grim and run under a Wayland compositor that supports screencopy"), outputName, sourceGeometry, false};
    }

    if (!grim.waitForFinished(8000)) {
        grim.kill();
        grim.waitForFinished(1000);
        return {{}, QStringLiteral("grim timed out while capturing the screen"), outputName, sourceGeometry, false};
    }

    const QByteArray png = grim.readAllStandardOutput();
    const QByteArray stderrText = grim.readAllStandardError();

    if (grim.exitStatus() != QProcess::NormalExit || grim.exitCode() != 0) {
        QString error = QString::fromLocal8Bit(stderrText).trimmed();
        if (error.isEmpty()) {
            error = QStringLiteral("grim failed with exit code %1").arg(grim.exitCode());
        }
        return {{}, error, outputName, sourceGeometry, false};
    }

    QImage image;
    if (!image.loadFromData(png, "PPM") || image.isNull()) {
        return {{}, QStringLiteral("grim returned invalid PPM data"), outputName, sourceGeometry, false};
    }

    return {image.convertToFormat(QImage::Format_ARGB32_Premultiplied), {}, outputName, sourceGeometry, cursorIncluded};
}

CaptureResult cropGrimFrameToRequest(CaptureResult capture, QRect frameGeometry, const CaptureRequest &request)
{
    if (capture.image.isNull()) {
        return capture;
    }

    if (frameGeometry.isValid() && !frameGeometry.isEmpty()) {
        capture.sourceGeometry = frameGeometry.normalized();
    }

    if (!request.sourceGeometry.isValid() || request.sourceGeometry.isEmpty() || request.allOutputs) {
        return capture;
    }

    if (capture.sourceGeometry.isEmpty()) {
        return {{},
                QStringLiteral("grim capture has no source geometry for local crop"),
                capture.outputName,
                request.sourceGeometry};
    }

    const QRect requested = request.sourceGeometry.normalized();
    const QRect overlap = requested.intersected(capture.sourceGeometry);
    if (overlap.isEmpty()) {
        return {{},
                QStringLiteral("grim capture does not cover requested geometry"),
                capture.outputName,
                request.sourceGeometry};
    }

    const QSize frameSize = capture.image.size();
    QImage cropped = markshot::capture::cropFrameToRequest(capture.image,
                                                           capture.sourceGeometry,
                                                           requested);
    if (cropped.isNull()) {
        return {{},
                QStringLiteral("grim local crop is empty"),
                capture.outputName,
                request.sourceGeometry};
    }

    capture.image = cropped.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const QSize croppedSize = capture.image.size();
    capture.sourceGeometry = overlap;
    markshot::debugLog("capture",
                       "grim-local-crop frame=%dx%d frame_geom=%d,%d %dx%d "
                       "requested=%d,%d %dx%d overlap=%d,%d %dx%d result=%dx%d",
                       frameSize.width(), frameSize.height(),
                       frameGeometry.x(), frameGeometry.y(),
                       frameGeometry.width(), frameGeometry.height(),
                       requested.x(), requested.y(), requested.width(), requested.height(),
                       overlap.x(), overlap.y(), overlap.width(), overlap.height(),
                       croppedSize.width(), croppedSize.height());
    return capture;
}
