#include "cli/headless_capture.h"

#include "screen_capture.h"

#include <QDateTime>
#include <QDir>
#include <QFileInfo>
#include <QGuiApplication>
#include <QImageWriter>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QScreen>
#include <QTextStream>

#include <cstdio>
#include <optional>

namespace markshot::cli {
namespace {

// Parses "x,y,w,h" into a QRect. Returns nullopt when malformed or empty.
std::optional<QRect> parseRegion(const QString &value)
{
    const QStringList parts = value.split(QLatin1Char(','));
    if (parts.size() != 4) {
        return std::nullopt;
    }
    bool xOk = false;
    bool yOk = false;
    bool wOk = false;
    bool hOk = false;
    const int x = parts.at(0).trimmed().toInt(&xOk);
    const int y = parts.at(1).trimmed().toInt(&yOk);
    const int w = parts.at(2).trimmed().toInt(&wOk);
    const int h = parts.at(3).trimmed().toInt(&hOk);
    if (!xOk || !yOk || !wOk || !hOk || w <= 0 || h <= 0) {
        return std::nullopt;
    }
    return QRect(x, y, w, h);
}

QByteArray displaysJson()
{
    QJsonArray displays;
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (!screen) {
            continue;
        }
        QJsonObject entry;
        entry.insert(QStringLiteral("name"), screen->name());
        const QRect geometry = screen->geometry();
        entry.insert(QStringLiteral("x"), geometry.x());
        entry.insert(QStringLiteral("y"), geometry.y());
        entry.insert(QStringLiteral("width"), geometry.width());
        entry.insert(QStringLiteral("height"), geometry.height());
        entry.insert(QStringLiteral("dpr"), screen->devicePixelRatio());
        entry.insert(QStringLiteral("primary"), screen == QGuiApplication::primaryScreen());
        displays.append(entry);
    }
    QJsonObject root;
    root.insert(QStringLiteral("displays"), displays);
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

// Resolves the final output path. When the user passes a directory, a
// timestamped file name is generated inside it.
QString resolveOutputPath(const QString &captureTo, const QString &outputName, QString *error)
{
    QFileInfo info(captureTo);
    if (info.isDir()) {
        const QString stamp =
            QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-HHmmss"));
        QString fileName = QStringLiteral("mark-shot-%1.png").arg(stamp);
        if (!outputName.isEmpty()) {
            fileName = QStringLiteral("mark-shot-%1-%2.png").arg(outputName, stamp);
        }
        return QDir(info.absoluteFilePath()).filePath(fileName);
    }
    if (info.absoluteFilePath().isEmpty()) {
        if (error) {
            *error = QStringLiteral("empty output path");
        }
        return QString();
    }
    return info.absoluteFilePath();
}

} // namespace

void addHeadlessCaptureOptions(QCommandLineParser *parser)
{
    QCommandLineOption captureToOption(QStringLiteral("capture-to"),
                                       QStringLiteral("Capture the screen and write it to the given file or directory without showing the UI."),
                                       QStringLiteral("path"));
    QCommandLineOption regionOption(QStringLiteral("region"),
                                    QStringLiteral("Capture only the region x,y,width,height in logical screen coordinates."),
                                    QStringLiteral("x,y,w,h"));
    QCommandLineOption displayOption(QStringLiteral("display"),
                                     QStringLiteral("Capture a specific output by monitor name."),
                                     QStringLiteral("name"));
    QCommandLineOption includeCursorOption(QStringLiteral("include-cursor"),
                                           QStringLiteral("Draw the mouse cursor into the captured image."));
    QCommandLineOption listDisplaysOption(QStringLiteral("list-displays"),
                                          QStringLiteral("Print the available outputs as JSON and exit."));
    QCommandLineOption outputNameOption(QStringLiteral("output-name"),
                                        QStringLiteral("Base file name (without extension) used when the capture path is a directory."),
                                        QStringLiteral("name"));
    parser->addOption(captureToOption);
    parser->addOption(regionOption);
    parser->addOption(displayOption);
    parser->addOption(includeCursorOption);
    parser->addOption(listDisplaysOption);
    parser->addOption(outputNameOption);
}

int runHeadlessCaptureIfRequested(const QCommandLineParser &parser)
{
    QTextStream out(stdout);
    QTextStream err(stderr);

    const bool wantListDisplays = parser.isSet(QStringLiteral("list-displays"));
    const bool wantCapture = parser.isSet(QStringLiteral("capture-to"));
    if (!wantListDisplays && !wantCapture) {
        return -1;
    }

    if (wantListDisplays) {
        out << displaysJson() << '\n';
        out.flush();
        if (wantCapture) {
            err << "--list-displays and --capture-to cannot be combined.\n";
            return 1;
        }
        return 0;
    }

    if (parser.positionalArguments().size() > 0) {
        err << "--capture-to cannot be combined with an image file argument.\n";
        return 1;
    }

    CaptureRequest request;
    request.allOutputs = parser.isSet(QStringLiteral("all-outputs"));
    request.includeCursor = parser.isSet(QStringLiteral("include-cursor"));
    request.allowInteractivePortal = false;
    request.hideOwnWindows = false;

    if (parser.isSet(QStringLiteral("region"))) {
        const std::optional<QRect> region = parseRegion(parser.value(QStringLiteral("region")));
        if (!region.has_value()) {
            err << "--region expects a comma-separated rectangle x,y,width,height.\n";
            return 1;
        }
        if (request.allOutputs) {
            err << "--region cannot be combined with --all-outputs.\n";
            return 1;
        }
        request.sourceGeometry = region.value();
    }

    const QString displayName = parser.value(QStringLiteral("display")).trimmed();
    if (!displayName.isEmpty()) {
        request.preferredOutputName = displayName;
        // When no explicit --region is given, crop to the named output so
        // portal-based backends capture that monitor instead of the whole
        // virtual desktop.
        if (!request.sourceGeometry.isValid()) {
            const QList<QScreen *> screens = QGuiApplication::screens();
            for (QScreen *screen : screens) {
                if (screen && screen->name() == displayName) {
                    request.sourceGeometry = screen->geometry();
                    break;
                }
            }
        }
    }

    const CaptureResult result = captureScreenFrame(request);

    if (result.image.isNull()) {
        err << result.error << '\n';
        out << QJsonDocument(QJsonObject{{QStringLiteral("path"), QJsonValue::Null},
                                         {QStringLiteral("width"), 0},
                                         {QStringLiteral("height"), 0},
                                         {QStringLiteral("output"), QJsonValue::Null},
                                         {QStringLiteral("error"), result.error}})
                   .toJson(QJsonDocument::Compact)
            << '\n';
        out.flush();
        return 1;
    }

    QString resolveError;
    const QString baseName = parser.value(QStringLiteral("output-name")).trimmed();
    const QString outputPath = resolveOutputPath(parser.value(QStringLiteral("capture-to")),
                                                 baseName.isEmpty() ? result.outputName : baseName,
                                                 &resolveError);
    if (outputPath.isEmpty()) {
        err << resolveError << '\n';
        return 1;
    }

    QImageWriter writer(outputPath, QByteArrayLiteral("png"));
    if (!writer.write(result.image)) {
        const QString writeError = writer.errorString();
        err << "failed to write capture to " << outputPath << ": " << writeError << '\n';
        out << QJsonDocument(QJsonObject{{QStringLiteral("path"), QJsonValue::Null},
                                         {QStringLiteral("width"), 0},
                                         {QStringLiteral("height"), 0},
                                         {QStringLiteral("output"), QJsonValue::Null},
                                         {QStringLiteral("error"), writeError}})
                   .toJson(QJsonDocument::Compact)
            << '\n';
        out.flush();
        return 1;
    }

    out << QJsonDocument(QJsonObject{{QStringLiteral("path"), outputPath},
                                     {QStringLiteral("width"), result.image.width()},
                                     {QStringLiteral("height"), result.image.height()},
                                     {QStringLiteral("output"), result.outputName.isEmpty() ? QJsonValue::Null : QJsonValue(result.outputName)},
                                     {QStringLiteral("error"), QJsonValue::Null}})
               .toJson(QJsonDocument::Compact)
        << '\n';
    out.flush();
    return 0;
}

} // namespace markshot::cli
