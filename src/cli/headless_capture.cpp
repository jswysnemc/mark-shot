#include "cli/headless_capture.h"

#include "screen_capture.h"

#include <QDateTime>
#include <QDir>
#include <QFile>
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

// Resolves the final output path. When the user passes a directory (or a path
// ending with a separator that does not yet exist), a timestamped file name is
// generated inside it.
QString resolveOutputPath(const QString &captureTo, const QString &outputName, QString *error)
{
    QFileInfo info(captureTo);
    const bool looksLikeDirectory = info.isDir()
        || (captureTo.endsWith(QLatin1Char('/')) || captureTo.endsWith(QLatin1Char('\\')));
    if (looksLikeDirectory) {
        QDir().mkpath(info.absoluteFilePath());
        const QString stamp =
            QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-HHmmss-zzz"));
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

// Returns true when captureTo looks like an existing directory (the only form
// supported for multi-display capture).
bool isDirectoryTarget(const QString &captureTo)
{
    return QFileInfo(captureTo).isDir();
}

// Builds the base CaptureRequest for headless mode. Shared by the single and
// multi-display paths.
CaptureRequest baseCaptureRequest(const QCommandLineParser &parser)
{
    CaptureRequest request;
    request.allOutputs = parser.isSet(QStringLiteral("all-outputs"));
    request.includeCursor = parser.isSet(QStringLiteral("include-cursor"));
    request.allowInteractivePortal = false;
    request.hideOwnWindows = false;
    return request;
}

// Returns the geometry of the named display, or a default-constructed QRect
// when no such display exists.
QRect displayGeometry(const QString &displayName)
{
    const QList<QScreen *> screens = QGuiApplication::screens();
    for (QScreen *screen : screens) {
        if (screen && screen->name() == displayName) {
            return screen->geometry();
        }
    }
    return {};
}

// Verifies that every requested display name exists. Returns the first
// offending name, or an empty string when all names are valid.
QString firstUnknownDisplay(const QStringList &displayNames)
{
    for (const QString &displayName : displayNames) {
        if (displayGeometry(displayName).isNull()) {
            return displayName;
        }
    }
    return {};
}

// Applies a display name: prefers that output and, when no explicit region is
// given, crops to the output geometry so portal backends capture that monitor
// instead of the whole virtual desktop.
void applyDisplayToRequest(CaptureRequest *request, const QString &displayName)
{
    if (displayName.isEmpty() || !request) {
        return;
    }
    request->preferredOutputName = displayName;
    if (!request->sourceGeometry.isValid()) {
        const QRect geometry = displayGeometry(displayName);
        if (!geometry.isNull()) {
            request->sourceGeometry = geometry;
        }
    }
}

// Captures one frame to a PNG and returns a compact JSON summary object.
// On failure an object with a null path and an error message is returned.
QJsonObject captureOneToFile(const CaptureRequest &request,
                             const QString &captureTo,
                             const QString &baseName,
                             QTextStream *err)
{
    const CaptureResult result = captureScreenFrame(request);

    if (result.image.isNull()) {
        if (err) {
            *err << result.error << '\n';
        }
        return {{QStringLiteral("path"), QJsonValue::Null},
                {QStringLiteral("width"), 0},
                {QStringLiteral("height"), 0},
                {QStringLiteral("output"), QJsonValue::Null},
                {QStringLiteral("error"), result.error}};
    }

    QString resolveError;
    const QString outputPath = resolveOutputPath(captureTo, baseName, &resolveError);
    if (outputPath.isEmpty()) {
        if (err) {
            *err << resolveError << '\n';
        }
        return {{QStringLiteral("path"), QJsonValue::Null},
                {QStringLiteral("width"), 0},
                {QStringLiteral("height"), 0},
                {QStringLiteral("output"), QJsonValue::Null},
                {QStringLiteral("error"), resolveError}};
    }

    QImageWriter writer(outputPath, QByteArrayLiteral("png"));
    if (!writer.write(result.image)) {
        const QString writeError = writer.errorString();
        QFile::remove(outputPath);
        if (err) {
            *err << "failed to write capture to " << outputPath << ": " << writeError << '\n';
        }
        return {{QStringLiteral("path"), QJsonValue::Null},
                {QStringLiteral("width"), 0},
                {QStringLiteral("height"), 0},
                {QStringLiteral("output"), QJsonValue::Null},
                {QStringLiteral("error"), writeError}};
    }

    // Propagate the requested display name when the backend did not fill it
    // (QScreen-based backends return an empty outputName).
    QString effectiveOutput = result.outputName;
    if (effectiveOutput.isEmpty() && !request.preferredOutputName.isEmpty()) {
        effectiveOutput = request.preferredOutputName;
    }

    return {{QStringLiteral("path"), outputPath},
            {QStringLiteral("width"), result.image.width()},
            {QStringLiteral("height"), result.image.height()},
            {QStringLiteral("output"), effectiveOutput.isEmpty() ? QJsonValue::Null : QJsonValue(effectiveOutput)},
            {QStringLiteral("error"), QJsonValue::Null}};
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
                                     QStringLiteral("Capture a specific output by monitor name. May be repeated to capture several monitors at once."),
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

    const QStringList displayNames = parser.values(QStringLiteral("display"));
    const QString captureTo = parser.value(QStringLiteral("capture-to"));
    const QString outputName = parser.value(QStringLiteral("output-name")).trimmed();
    const bool hasRegion = parser.isSet(QStringLiteral("region"));
    const bool allOutputs = parser.isSet(QStringLiteral("all-outputs"));

    // Reject semantically conflicting option combinations early so the user
    // gets a clear error instead of silently capturing the wrong pixels.
    if (allOutputs && !displayNames.isEmpty()) {
        err << "--all-outputs cannot be combined with --display.\n";
        return 1;
    }
    if (hasRegion && displayNames.size() > 1) {
        err << "--region cannot be combined with multiple --display options.\n";
        return 1;
    }
    if (displayNames.size() > 1 && !isDirectoryTarget(captureTo)) {
        err << "--capture-to must be an existing directory when capturing multiple displays.\n";
        return 1;
    }
    const QString unknownDisplay = firstUnknownDisplay(displayNames);
    if (!unknownDisplay.isEmpty()) {
        err << "unknown display: " << unknownDisplay << ". Use --list-displays to see valid names.\n";
        return 1;
    }

    CaptureRequest request = baseCaptureRequest(parser);
    request.allOutputs = allOutputs;

    if (hasRegion) {
        const std::optional<QRect> region = parseRegion(parser.value(QStringLiteral("region")));
        if (!region.has_value()) {
            err << "--region expects a comma-separated rectangle x,y,width,height.\n";
            return 1;
        }
        if (allOutputs) {
            err << "--region cannot be combined with --all-outputs.\n";
            return 1;
        }
        request.sourceGeometry = region.value();
    }

    // Multiple --display: capture each monitor to its own PNG and print a
    // {"captures":[...]} JSON array. This enables multi-screen selection for
    // agents and scripts.
    if (displayNames.size() > 1) {
        QJsonArray captures;
        bool anyFailed = false;
        for (const QString &displayName : displayNames) {
            CaptureRequest displayRequest = request;
            applyDisplayToRequest(&displayRequest, displayName);
            const QString baseName = outputName.isEmpty()
                ? displayName
                : QStringLiteral("%1-%2").arg(outputName, displayName);
            const QJsonObject one = captureOneToFile(displayRequest, captureTo, baseName, &err);
            if (one.value(QStringLiteral("error")).isString()) {
                anyFailed = true;
            }
            captures.append(one);
        }
        out << QJsonDocument(QJsonObject{{QStringLiteral("captures"), captures}})
                   .toJson(QJsonDocument::Compact)
            << '\n';
        out.flush();
        return anyFailed ? 1 : 0;
    }

    // Single display (or none): keep the original compact single-object JSON.
    if (!displayNames.isEmpty()) {
        applyDisplayToRequest(&request, displayNames.first());
    }

    const QJsonObject single = captureOneToFile(request, captureTo, outputName, &err);
    out << QJsonDocument(single).toJson(QJsonDocument::Compact) << '\n';
    out.flush();
    return single.value(QStringLiteral("error")).isString() ? 1 : 0;
}

} // namespace markshot::cli
