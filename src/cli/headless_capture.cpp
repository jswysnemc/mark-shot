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

/**
 * 解析逻辑坐标区域参数。
 * @param value 格式为 x,y,w,h 的区域字符串。
 * @return 合法区域；格式错误或尺寸无效时返回空值。
 */
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

/**
 * 序列化当前可用显示器信息。
 * @return 紧凑格式的 JSON 数据。
 */
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

/**
 * 解析截图最终输出路径，目录目标会生成带时间戳的文件名。
 * @param captureTo 用户指定的文件或目录路径。
 * @param outputName 目录目标使用的文件基础名称。
 * @param error 输出路径解析错误。
 * @return 最终输出文件路径；失败时返回空字符串。
 */
QString resolveOutputPath(const QString &captureTo, const QString &outputName, QString *error)
{
    QFileInfo info(captureTo);
    const bool looksLikeDirectory = info.isDir()
        || (captureTo.endsWith(QLatin1Char('/')) || captureTo.endsWith(QLatin1Char('\\')));
    if (looksLikeDirectory) {
        if (!QDir().mkpath(info.absoluteFilePath())) {
            if (error) {
                *error = QStringLiteral("failed to create output directory: %1")
                             .arg(info.absoluteFilePath());
            }
            return {};
        }
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

/**
 * 判断多显示器截图目标是否明确表示目录。
 * @param captureTo 用户指定的输出路径。
 * @return 路径为现有目录或以目录分隔符结尾时返回 true。
 */
bool isDirectoryTarget(const QString &captureTo)
{
    return QFileInfo(captureTo).isDir()
        || captureTo.endsWith(QLatin1Char('/'))
        || captureTo.endsWith(QLatin1Char('\\'));
}

/**
 * 根据命令行参数创建无界面截图基础请求。
 * @param parser 已完成参数解析的命令行解析器。
 * @return 单屏和多屏流程共用的截图请求。
 */
CaptureRequest baseCaptureRequest(const QCommandLineParser &parser)
{
    CaptureRequest request;
    request.allOutputs = parser.isSet(QStringLiteral("all-outputs"));
    request.includeCursor = parser.isSet(QStringLiteral("include-cursor"));
    request.allowInteractivePortal = false;
    request.hideOwnWindows = false;
    return request;
}

/**
 * 查找指定显示器的逻辑坐标范围。
 * @param displayName 显示器名称。
 * @return 显示器逻辑坐标；未找到时返回空矩形。
 */
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

/**
 * 查找请求中第一个不存在的显示器名称。
 * @param displayNames 请求的显示器名称列表。
 * @return 第一个无效名称；全部有效时返回空字符串。
 */
QString firstUnknownDisplay(const QStringList &displayNames)
{
    for (const QString &displayName : displayNames) {
        if (displayGeometry(displayName).isNull()) {
            return displayName;
        }
    }
    return {};
}

/**
 * 将显示器名称和逻辑坐标应用到截图请求。
 * @param request 需要更新的截图请求。
 * @param displayName 目标显示器名称。
 * @return 无返回值。
 */
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

/**
 * 创建统一的截图失败结果对象。
 * @param error 错误说明。
 * @return 路径和输出为空、尺寸为零的 JSON 结果。
 */
QJsonObject captureErrorObject(const QString &error)
{
    return {{QStringLiteral("path"), QJsonValue::Null},
            {QStringLiteral("width"), 0},
            {QStringLiteral("height"), 0},
            {QStringLiteral("output"), QJsonValue::Null},
            {QStringLiteral("error"), error}};
}

/**
 * 向标准输出和标准错误报告参数错误。
 * @param out 标准输出流。
 * @param err 标准错误流。
 * @param error 错误说明。
 * @return 固定的失败退出码 1。
 */
int reportUsageError(QTextStream *out, QTextStream *err, const QString &error)
{
    if (out) {
        *out << QJsonDocument(captureErrorObject(error)).toJson(QJsonDocument::Compact) << '\n';
        out->flush();
    }
    if (err) {
        *err << error << '\n';
        err->flush();
    }
    return 1;
}

/**
 * 捕获一帧并写入 PNG 文件。
 * @param request 截图请求。
 * @param captureTo 输出文件或目录路径。
 * @param baseName 目录目标使用的文件基础名称。
 * @param err 标准错误输出流。
 * @return 包含路径、尺寸、显示器和错误的 JSON 结果。
 */
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
        return captureErrorObject(result.error);
    }

    QString resolveError;
    const QString outputPath = resolveOutputPath(captureTo, baseName, &resolveError);
    if (outputPath.isEmpty()) {
        if (err) {
            *err << resolveError << '\n';
        }
        return captureErrorObject(resolveError);
    }

    QImageWriter writer(outputPath, QByteArrayLiteral("png"));
    if (!writer.write(result.image)) {
        const QString writeError = writer.errorString();
        QFile::remove(outputPath);
        if (err) {
            *err << "failed to write capture to " << outputPath << ": " << writeError << '\n';
        }
        return captureErrorObject(writeError);
    }

    // 1. 后端未返回显示器名称时保留用户请求的名称
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

    if (!parser.positionalArguments().isEmpty()) {
        return reportUsageError(
            &out,
            &err,
            QStringLiteral("headless options cannot be combined with an image file argument."));
    }
    if (wantListDisplays && wantCapture) {
        return reportUsageError(
            &out,
            &err,
            QStringLiteral("--list-displays and --capture-to cannot be combined."));
    }
    if (wantListDisplays) {
        out << displaysJson() << '\n';
        out.flush();
        return 0;
    }

    const QStringList displayNames = parser.values(QStringLiteral("display"));
    const QString captureTo = parser.value(QStringLiteral("capture-to"));
    const QString outputName = parser.value(QStringLiteral("output-name")).trimmed();
    const bool hasRegion = parser.isSet(QStringLiteral("region"));
    const bool allOutputs = parser.isSet(QStringLiteral("all-outputs"));

    // 1. 校验互斥参数和多显示器输出目录
    if (allOutputs && !displayNames.isEmpty()) {
        return reportUsageError(
            &out, &err, QStringLiteral("--all-outputs cannot be combined with --display."));
    }
    if (hasRegion && displayNames.size() > 1) {
        return reportUsageError(
            &out,
            &err,
            QStringLiteral("--region cannot be combined with multiple --display options."));
    }
    if (displayNames.size() > 1 && !isDirectoryTarget(captureTo)) {
        return reportUsageError(
            &out,
            &err,
            QStringLiteral("--capture-to must be a directory when capturing multiple displays."));
    }
    const QString unknownDisplay = firstUnknownDisplay(displayNames);
    if (!unknownDisplay.isEmpty()) {
        return reportUsageError(
            &out,
            &err,
            QStringLiteral("unknown display: %1. Use --list-displays to see valid names.")
                .arg(unknownDisplay));
    }

    CaptureRequest request = baseCaptureRequest(parser);
    request.allOutputs = allOutputs;

    if (hasRegion) {
        const std::optional<QRect> region = parseRegion(parser.value(QStringLiteral("region")));
        if (!region.has_value()) {
            return reportUsageError(
                &out,
                &err,
                QStringLiteral("--region expects a comma-separated rectangle x,y,width,height."));
        }
        if (allOutputs) {
            return reportUsageError(
                &out,
                &err,
                QStringLiteral("--region cannot be combined with --all-outputs."));
        }
        request.sourceGeometry = region.value();
    }

    // 2. 多显示器请求逐屏写入文件并汇总 JSON 结果
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

    // 3. 单显示器请求保持单对象 JSON 结果
    if (!displayNames.isEmpty()) {
        applyDisplayToRequest(&request, displayNames.first());
    }

    const QJsonObject single = captureOneToFile(request, captureTo, outputName, &err);
    out << QJsonDocument(single).toJson(QJsonDocument::Compact) << '\n';
    out.flush();
    return single.value(QStringLiteral("error")).isString() ? 1 : 0;
}

} // namespace markshot::cli
