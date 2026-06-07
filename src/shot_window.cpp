#include "shot_window.h"

#include "capture_geometry.h"
#include "clipboard_image.h"
#include "layer_shell_runtime.h"
#include "screen_capture.h"
#include "scroll/scroll_session_window.h"
#include "scroll/stitcher.h"
#include "ui/color_picker.h"
#include "ui/i18n.h"
#include "ui/icons.h"
#include "ui/theme.h"
#include "window_detection.h"
#include "windows_integration.h"

#include <QAbstractItemView>
#include <QAction>
#include <QApplication>
#include <QBoxLayout>
#include <QComboBox>
#include <QBrush>
#include <QClipboard>
#include <QContextMenuEvent>
#include <QCoreApplication>
#include <QCursor>
#include <QDateTime>
#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#endif
#include <QDir>
#include <QDirIterator>
#include <QEvent>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFontDatabase>
#include <QFrame>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QKeyEvent>
#include <QKeySequence>
#include <QLabel>
#include <QLayout>
#include <QLineF>
#include <QListWidget>
#include <QLineEdit>
#include <QMenu>
#include <QMessageBox>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPointer>
#include <QPixmap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QScrollBar>
#include <QScreen>
#include <QShortcut>
#include <QShowEvent>
#include <QSignalBlocker>
#include <QSlider>
#include <QStandardPaths>
#include <QStyle>
#include <QTemporaryFile>
#include <QTextEdit>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>
#include <QTextOption>
#include <QThread>
#include <QTimer>
#include <QTransform>
#include <QUrl>
#include <QVBoxLayout>
#include <QWindow>
#include <QWheelEvent>
#include <QtConcurrent>

#include <algorithm>
#include <array>
#include <cmath>
#include <limits>
#include <utility>
#include <vector>

namespace {

constexpr qreal kMinSelectionSize = 8.0;
constexpr qreal kToolbarMargin = 10.0;
constexpr qreal kMinStrokeWidth = 1.0;
constexpr qreal kMaxStrokeWidth = 24.0;
constexpr qreal kMinNumberWidth = 1.0;
constexpr qreal kMaxNumberWidth = 72.0;
constexpr qreal kMinMosaicBlockSize = 4.0;
constexpr qreal kMaxMosaicBlockSize = 48.0;
constexpr qreal kMinLaserWidth = 4.0;
constexpr qreal kMaxLaserWidth = 48.0;
constexpr qint64 kLaserLifetimeMs = 1800;
constexpr qreal kTextBackgroundPaddingX = 6.0;
constexpr qreal kTextBackgroundPaddingY = 4.0;
constexpr qreal kMinImageZoom = 0.25;
constexpr qreal kMaxImageZoom = 64.0;
constexpr qint64 kCtrlDoubleTapMs = 360;
constexpr int kPinnedOcrTimeoutMs = 30000;
constexpr int kPinnedTranslationTimeoutMs = 60000;
constexpr int kImageScrollBarExtent = 14;
constexpr int kImageWindowMinimumToolbarPadding = 24;
constexpr qsizetype kMaxSharpViewportPixels = 50'000'000;
constexpr double kSharpKernelRadius = 2.5;
constexpr int kMinSharpRowsPerThread = 64;
constexpr qreal kMinStartupColorLoupeSize = 72.0;
constexpr qreal kMaxStartupColorLoupeSize = 260.0;
constexpr qreal kDefaultMagnifierScale = 2.75;
constexpr qreal kMinMagnifierScale = 1.25;
constexpr qreal kMaxMagnifierScale = 6.0;
constexpr qreal kMinMagnifierDiameter = 72.0;
constexpr qreal kMaxMagnifierDiameter = 320.0;
constexpr qreal kMagnifierDragScale = 1.05;
constexpr qreal kMinMagnifierDragDistance = 64.0;

qreal clampedMagnifierScale(qreal scale)
{
    return std::clamp(scale, kMinMagnifierScale, kMaxMagnifierScale);
}

int magnifierScaleSliderValue(qreal scale)
{
    return qRound(clampedMagnifierScale(scale) * 100.0);
}

qreal magnifierScaleFromSliderValue(int value)
{
    return clampedMagnifierScale(static_cast<qreal>(value) / 100.0);
}

QString magnifierScaleText(qreal scale)
{
    return QStringLiteral("%1x").arg(QString::number(clampedMagnifierScale(scale), 'f', 2));
}

qreal normalizedRotationDegrees(qreal degrees)
{
    degrees = std::fmod(degrees, 360.0);
    if (degrees < -180.0) {
        degrees += 360.0;
    } else if (degrees > 180.0) {
        degrees -= 360.0;
    }
    return degrees;
}

std::optional<bool> boolFromText(QString value)
{
    value = value.trimmed().toLower();
    if (value.isEmpty()) {
        return std::nullopt;
    }
    if (value == QStringLiteral("1") || value == QStringLiteral("true")
        || value == QStringLiteral("yes") || value == QStringLiteral("on")) {
        return true;
    }
    if (value == QStringLiteral("0") || value == QStringLiteral("false")
        || value == QStringLiteral("no") || value == QStringLiteral("off")) {
        return false;
    }
    return std::nullopt;
}

std::optional<bool> boolFromConfigValue(const QJsonValue &value)
{
    if (value.isBool()) {
        return value.toBool();
    }
    if (value.isString()) {
        return boolFromText(value.toString());
    }
    return std::nullopt;
}

std::optional<bool> envFlagValue(const QProcessEnvironment &env, const QStringList &names)
{
    for (const QString &name : names) {
        if (!env.contains(name)) {
            continue;
        }
        if (const std::optional<bool> value = boolFromText(env.value(name))) {
            return value;
        }
    }
    return std::nullopt;
}

bool sendDesktopNotification(const QString &summary, const QString &body, int timeoutMs = 2500)
{
#ifdef MARK_SHOT_WITH_DBUS
    QDBusInterface notifications(QStringLiteral("org.freedesktop.Notifications"),
                                 QStringLiteral("/org/freedesktop/Notifications"),
                                 QStringLiteral("org.freedesktop.Notifications"),
                                 QDBusConnection::sessionBus());
    if (!notifications.isValid()) {
        return false;
    }

    QDBusMessage reply = notifications.call(QStringLiteral("Notify"),
                                            QStringLiteral("mark-shot"),
                                            static_cast<uint>(0),
                                            QString(),
                                            summary,
                                            body,
                                            QStringList(),
                                            QVariantMap(),
                                            timeoutMs);
    return reply.type() != QDBusMessage::ErrorMessage;
#else
    Q_UNUSED(summary);
    Q_UNUSED(body);
    Q_UNUSED(timeoutMs);
    return false;
#endif
}



QColor propertyIconInkForFill(const QColor &fillColor)
{
    if (!fillColor.isValid() || fillColor.alpha() < 48) {
        return QColor(229, 231, 235);
    }
    const int luma =
        (fillColor.red() * 299 + fillColor.green() * 587 + fillColor.blue() * 114) / 1000;
    return luma > 150 ? QColor(15, 23, 42) : QColor(248, 250, 252);
}

QString colorHexRgb(QColor color)
{
    return color.name(QColor::HexRgb).toUpper();
}

QString colorHexRgba(QColor color)
{
    return QStringLiteral("#%1%2%3%4")
        .arg(color.red(), 2, 16, QLatin1Char('0'))
        .arg(color.green(), 2, 16, QLatin1Char('0'))
        .arg(color.blue(), 2, 16, QLatin1Char('0'))
        .arg(color.alpha(), 2, 16, QLatin1Char('0'))
        .toUpper();
}

QString alphaText(QColor color)
{
    if (color.alpha() == 255) {
        return QStringLiteral("1");
    }
    if (color.alpha() == 0) {
        return QStringLiteral("0");
    }
    return QString::number(color.alphaF(), 'f', 3);
}

QString normalizedColorChannel(int value)
{
    return QString::number(static_cast<qreal>(value) / 255.0, 'f', 3);
}

int colorHueOrZero(int hue)
{
    return hue < 0 ? 0 : hue;
}

int rulerTickStep(qreal pixels)
{
    if (pixels <= 0.0) {
        return 1;
    }

    const qreal raw = pixels / 8.0;
    const qreal magnitude = std::pow(10.0, std::floor(std::log10(std::max<qreal>(1.0, raw))));
    const qreal normalized = raw / magnitude;
    qreal nice = 10.0;
    if (normalized <= 1.0) {
        nice = 1.0;
    } else if (normalized <= 2.0) {
        nice = 2.0;
    } else if (normalized <= 5.0) {
        nice = 5.0;
    }
    return std::max(1, qRound(nice * magnitude));
}

void drawRoundedLabel(QPainter &painter, QRectF rect, const QString &text, QColor fill = QColor(8, 13, 19, 225))
{
    painter.save();
    painter.setPen(Qt::NoPen);
    painter.setBrush(fill);
    painter.drawRoundedRect(rect, 9.0, 9.0);
    painter.setPen(QColor(229, 231, 235));
    painter.drawText(rect, Qt::AlignCenter, text);
    painter.restore();
}

QRectF normalizedRect(QPointF a, QPointF b)
{
    return QRectF(a, b).normalized();
}

bool ellipseContainsPoint(QRectF ellipse, QPointF point, qreal tolerance)
{
    ellipse = ellipse.normalized().adjusted(-tolerance, -tolerance, tolerance, tolerance);
    if (ellipse.width() <= 0.0 || ellipse.height() <= 0.0) {
        return false;
    }

    const QPointF center = ellipse.center();
    const qreal radiusX = ellipse.width() / 2.0;
    const qreal radiusY = ellipse.height() / 2.0;
    const qreal dx = (point.x() - center.x()) / radiusX;
    const qreal dy = (point.y() - center.y()) / radiusY;
    return dx * dx + dy * dy <= 1.0;
}

QRect windowGeometryToImageRect(QRect windowGeometry, QRect sourceGeometry, QSize imageSize)
{
    return markshot::capture::imageRectFromGeometry(windowGeometry, sourceGeometry, imageSize);
}

// Builds a smoothed stroke path from raw sample points. Each sample point is
// used as the control point of a quadratic segment whose endpoints are the
// midpoints of adjacent samples, so the curve passes through every midpoint
// and rounds off the corners that appear when the pointer moves fast and the
// samples are sparse. The original points are left untouched; this only
// affects rendering.
QPainterPath smoothedStrokePath(const QVector<QPointF> &points)
{
    QPainterPath path;
    if (points.isEmpty()) {
        return path;
    }
    if (points.size() < 3) {
        path.moveTo(points.first());
        for (int i = 1; i < points.size(); ++i) {
            path.lineTo(points.at(i));
        }
        return path;
    }

    path.moveTo(points.first());
    path.lineTo((points.at(0) + points.at(1)) / 2.0);
    for (int i = 1; i < points.size() - 1; ++i) {
        const QPointF midpoint = (points.at(i) + points.at(i + 1)) / 2.0;
        path.quadTo(points.at(i), midpoint);
    }
    path.lineTo(points.last());
    return path;
}

qreal imageNavigationWheelFactor(const QWheelEvent *event)
{
    const QPoint angleDelta = event->angleDelta();
    if (angleDelta.y() != 0) {
        return std::pow(1.12, static_cast<qreal>(angleDelta.y()) / 120.0);
    }

    const QPoint pixelDelta = event->pixelDelta();
    if (pixelDelta.y() != 0) {
        return std::pow(1.12, static_cast<qreal>(pixelDelta.y()) / 80.0);
    }

    return 1.0;
}

struct AxisSample {
    int index = 0;
    double weight = 0.0;
};

struct AxisTable {
    std::vector<std::vector<AxisSample>> samples;
    int first = 0;
    int last = -1;
};

double sharpKernel(double distance)
{
    const double x = std::abs(distance);
    if (x > kSharpKernelRadius) {
        return 0.0;
    }
    if (x <= 0.5) {
        return 17.0 / 16.0 - 7.0 / 4.0 * x * x;
    }
    if (x <= 1.5) {
        return x * x - 11.0 / 4.0 * x + 7.0 / 4.0;
    }
    return -1.0 / 8.0 * x * x + 5.0 / 8.0 * x - 25.0 / 32.0;
}

AxisTable buildAxisTable(int inputSize, int outputSize, double sourceStart, double scale)
{
    AxisTable table;
    table.samples.resize(outputSize);
    table.first = inputSize;

    const double filterScale = std::min(scale, 1.0);
    const double radius = kSharpKernelRadius / filterScale;
    for (int output = 0; output < outputSize; ++output) {
        const double center = sourceStart + (static_cast<double>(output) + 0.5) / scale - 0.5;
        const int first = std::max(0, static_cast<int>(std::floor(center - radius)));
        const int last = std::min(inputSize - 1, static_cast<int>(std::ceil(center + radius)));

        double sum = 0.0;
        std::vector<AxisSample> samples;
        samples.reserve(std::max(0, last - first + 1));
        for (int input = first; input <= last; ++input) {
            const double weight = sharpKernel((static_cast<double>(input) - center) * filterScale);
            if (weight == 0.0) {
                continue;
            }
            samples.push_back({input, weight});
            sum += weight;
        }

        if (samples.empty() || qFuzzyIsNull(sum)) {
            const int nearest = std::clamp(static_cast<int>(std::round(center)), 0, inputSize - 1);
            samples.push_back({nearest, 1.0});
            table.first = std::min(table.first, nearest);
            table.last = std::max(table.last, nearest);
        } else {
            for (AxisSample &sample : samples) {
                sample.weight /= sum;
                table.first = std::min(table.first, sample.index);
                table.last = std::max(table.last, sample.index);
            }
        }

        table.samples[output] = std::move(samples);
    }

    if (table.first > table.last) {
        table.first = 0;
        table.last = 0;
    }
    return table;
}

int byteFromWeightedSum(double value)
{
    return std::clamp(static_cast<int>(std::lround(value)), 0, 255);
}

QRgb premultipliedPixel(double red, double green, double blue, double alpha)
{
    const int a = byteFromWeightedSum(alpha);
    const int r = std::min(byteFromWeightedSum(red), a);
    const int g = std::min(byteFromWeightedSum(green), a);
    const int b = std::min(byteFromWeightedSum(blue), a);
    return qRgba(r, g, b, a);
}

template <typename Function>
void forRowRanges(int rowCount, Function function)
{
    if (rowCount <= 0) {
        return;
    }

    const int idealThreads = std::max(1, QThread::idealThreadCount());
    const int threadCount = std::clamp(rowCount / kMinSharpRowsPerThread, 1, idealThreads);
    if (threadCount == 1) {
        function(0, rowCount);
        return;
    }

    std::vector<std::pair<int, int>> ranges;
    ranges.reserve(threadCount);
    const int rowsPerThread = rowCount / threadCount;
    int begin = 0;
    for (int i = 0; i < threadCount; ++i) {
        const int end = i == threadCount - 1 ? rowCount : begin + rowsPerThread;
        ranges.push_back({begin, end});
        begin = end;
    }

    QtConcurrent::blockingMap(ranges, [&function](const std::pair<int, int> &range) {
        function(range.first, range.second);
    });
}

QImage renderSharpViewport(const QImage &source, const QRectF &sourceRect, const QSize &targetSize)
{
    if (source.isNull() || sourceRect.isEmpty() || targetSize.isEmpty()) {
        return {};
    }

    const QImage src = source.format() == QImage::Format_ARGB32_Premultiplied
        ? source
        : source.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const QRectF sourceBounds(QPointF(0.0, 0.0), QSizeF(src.size()));
    const QRectF clampedSourceRect = sourceRect.intersected(sourceBounds);
    if (clampedSourceRect.isEmpty()) {
        return {};
    }

    const double scaleX = static_cast<double>(targetSize.width()) / clampedSourceRect.width();
    const double scaleY = static_cast<double>(targetSize.height()) / clampedSourceRect.height();
    const AxisTable xTable =
        buildAxisTable(src.width(), targetSize.width(), clampedSourceRect.left(), scaleX);
    const AxisTable yTable =
        buildAxisTable(src.height(), targetSize.height(), clampedSourceRect.top(), scaleY);

    const int intermediateHeight = yTable.last - yTable.first + 1;
    QImage horizontal(targetSize.width(), intermediateHeight, QImage::Format_ARGB32_Premultiplied);
    const uchar *sourceBits = src.constBits();
    const int sourceStride = src.bytesPerLine();
    uchar *horizontalBits = horizontal.bits();
    const int horizontalStride = horizontal.bytesPerLine();

    forRowRanges(intermediateHeight, [&](int begin, int end) {
        for (int row = begin; row < end; ++row) {
            const int y = yTable.first + row;
            const auto *sourceLine =
                reinterpret_cast<const QRgb *>(sourceBits + y * sourceStride);
            auto *targetLine =
                reinterpret_cast<QRgb *>(horizontalBits + row * horizontalStride);
            for (int x = 0; x < targetSize.width(); ++x) {
                double a = 0.0;
                double r = 0.0;
                double g = 0.0;
                double b = 0.0;
                for (const AxisSample &sample : xTable.samples[x]) {
                    const QRgb pixel = sourceLine[sample.index];
                    a += sample.weight * qAlpha(pixel);
                    r += sample.weight * qRed(pixel);
                    g += sample.weight * qGreen(pixel);
                    b += sample.weight * qBlue(pixel);
                }
                targetLine[x] = premultipliedPixel(r, g, b, a);
            }
        }
    });

    QImage target(targetSize, QImage::Format_ARGB32_Premultiplied);
    const uchar *horizontalReadBits = horizontal.constBits();
    const int horizontalReadStride = horizontal.bytesPerLine();
    uchar *targetBits = target.bits();
    const int targetStride = target.bytesPerLine();

    forRowRanges(targetSize.height(), [&](int begin, int end) {
        for (int y = begin; y < end; ++y) {
            auto *targetLine = reinterpret_cast<QRgb *>(targetBits + y * targetStride);
            for (int x = 0; x < targetSize.width(); ++x) {
                double a = 0.0;
                double r = 0.0;
                double g = 0.0;
                double b = 0.0;
                for (const AxisSample &sample : yTable.samples[y]) {
                    const auto *sourceLine = reinterpret_cast<const QRgb *>(
                        horizontalReadBits + (sample.index - yTable.first) * horizontalReadStride);
                    const QRgb pixel = sourceLine[x];
                    a += sample.weight * qAlpha(pixel);
                    r += sample.weight * qRed(pixel);
                    g += sample.weight * qGreen(pixel);
                    b += sample.weight * qBlue(pixel);
                }
                targetLine[x] = premultipliedPixel(r, g, b, a);
            }
        }
    });
    return target;
}

// Stylesheets, palette presets, action names, and toolbar icons now live in
// src/ui/theme.{h,cpp} and src/ui/icons.{h,cpp}.

QString desktopEntryValue(const QStringList &lines, const QString &key)
{
    bool inDesktopEntry = false;
    const QString prefix = key + QLatin1Char('=');
    for (const QString &line : lines) {
        const QString trimmed = line.trimmed();
        if (trimmed.isEmpty() || trimmed.startsWith(QLatin1Char('#'))) {
            continue;
        }
        if (trimmed.startsWith(QLatin1Char('[')) && trimmed.endsWith(QLatin1Char(']'))) {
            inDesktopEntry = trimmed == QStringLiteral("[Desktop Entry]");
            continue;
        }
        if (inDesktopEntry && trimmed.startsWith(prefix)) {
            return trimmed.mid(prefix.size()).trimmed();
        }
    }
    return {};
}

bool desktopEntryBool(const QStringList &lines, const QString &key)
{
    const QString value = desktopEntryValue(lines, key).toLower();
    return value == QStringLiteral("true") || value == QStringLiteral("1");
}

bool desktopEntrySupportsImage(const QStringList &lines)
{
    const QStringList mimeTypes = desktopEntryValue(lines, QStringLiteral("MimeType"))
        .split(QLatin1Char(';'), Qt::SkipEmptyParts);
    for (const QString &mimeType : mimeTypes) {
        const QString normalized = mimeType.trimmed().toLower();
        if (normalized == QStringLiteral("image/png") || normalized == QStringLiteral("image/*")
            || normalized.startsWith(QStringLiteral("image/"))) {
            return true;
        }
    }
    return false;
}

QStringList desktopSearchDirs()
{
#if defined(Q_OS_WIN)
    return {};
#else
    QStringList dataDirs;
    dataDirs << QDir::home().filePath(QStringLiteral(".local/share"));

    const QString envDataDirs = QProcessEnvironment::systemEnvironment().value(
        QStringLiteral("XDG_DATA_DIRS"),
        QStringLiteral("/usr/local/share:/usr/share"));
    dataDirs << envDataDirs.split(QLatin1Char(':'), Qt::SkipEmptyParts);
    dataDirs.removeDuplicates();

    QStringList appDirs;
    for (const QString &dataDir : dataDirs) {
        appDirs << QDir(dataDir).filePath(QStringLiteral("applications"));
    }
    appDirs.removeDuplicates();
    return appDirs;
#endif
}

QString markShotPicturesDir()
{
    QString pictures = QStandardPaths::writableLocation(QStandardPaths::PicturesLocation);
    if (pictures.isEmpty()) {
        pictures = QDir::homePath();
    }

    const QString markShotDir = QDir(pictures).filePath(QStringLiteral("mark-shot"));
    QDir dir(markShotDir);
    if (dir.exists() || dir.mkpath(QStringLiteral("."))) {
        return markShotDir;
    }
    return pictures;
}

QString markShotConfigDir()
{
    return markshot::markShotConfigDir();
}

QString extensionCommandsConfigPath()
{
    return QDir(markShotConfigDir()).filePath(QStringLiteral("extensions.json"));
}

QString appConfigPath()
{
    return markshot::appConfigPath();
}

QString markShotDataDir()
{
    QString dataDir = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    if (!dataDir.isEmpty()) {
        return dataDir;
    }

    const QString genericDataDir = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    if (!genericDataDir.isEmpty()) {
        return QDir(genericDataDir).filePath(QStringLiteral("mark-shot"));
    }
    return QDir::home().filePath(QStringLiteral(".local/share/mark-shot"));
}

QString slurpGeometry(QRect geometry)
{
    geometry = geometry.normalized();
    return QStringLiteral("%1,%2 %3x%4")
        .arg(geometry.x())
        .arg(geometry.y())
        .arg(geometry.width())
        .arg(geometry.height());
}

QString expandUserPath(const QString &path)
{
    if (path == QStringLiteral("~")) {
        return QDir::homePath();
    }
    if (path.startsWith(QStringLiteral("~/"))) {
        return QDir::home().filePath(path.mid(2));
    }
    return path;
}

QString shellQuote(QString value)
{
#if defined(Q_OS_WIN)
    if (value.isEmpty()) {
        return QStringLiteral("\"\"");
    }
    value.replace(QStringLiteral("^"), QStringLiteral("^^"));
    value.replace(QStringLiteral("%"), QStringLiteral("^%"));
    value.replace(QStringLiteral("\""), QStringLiteral("^\""));
    return QStringLiteral("\"") + value + QStringLiteral("\"");
#else
    if (value.isEmpty()) {
        return QStringLiteral("''");
    }
    value.replace(QStringLiteral("'"), QStringLiteral("'\"'\"'"));
    return QStringLiteral("'") + value + QStringLiteral("'");
#endif
}

QString commandShellProgram()
{
#if defined(Q_OS_WIN)
    const QString comspec = QProcessEnvironment::systemEnvironment().value(QStringLiteral("COMSPEC"));
    return comspec.isEmpty() ? QStringLiteral("cmd.exe") : comspec;
#else
    QString shell = QProcessEnvironment::systemEnvironment().value(QStringLiteral("SHELL"),
                                                                   QStringLiteral("/bin/sh"));
    return shell.isEmpty() ? QStringLiteral("/bin/sh") : shell;
#endif
}

QStringList commandShellArguments(const QString &commandLine)
{
#if defined(Q_OS_WIN)
    return {QStringLiteral("/D"), QStringLiteral("/V:OFF"), QStringLiteral("/S"), QStringLiteral("/C"), commandLine};
#else
    return {QStringLiteral("-c"), commandLine};
#endif
}

void setShellCommand(QProcess *process, const QString &commandLine)
{
    if (!process) {
        return;
    }
    process->setProgram(commandShellProgram());
    process->setArguments(commandShellArguments(commandLine));
}

bool extensionCommandUsesImagePlaceholder(const QString &command)
{
    return command.contains(QStringLiteral("{image}"))
        || command.contains(QStringLiteral("{imagePath}"))
        || command.contains(QStringLiteral("{imageUrl}"));
}

bool replaceExtensionImagePlaceholders(QString *command, const QString &imagePath)
{
    if (!command) {
        return false;
    }

    bool replaced = false;
    const QString quotedPath = shellQuote(imagePath);
    if (command->contains(QStringLiteral("{image}"))) {
        command->replace(QStringLiteral("{image}"), quotedPath);
        replaced = true;
    }
    if (command->contains(QStringLiteral("{imagePath}"))) {
        command->replace(QStringLiteral("{imagePath}"), quotedPath);
        replaced = true;
    }
    if (command->contains(QStringLiteral("{imageUrl}"))) {
        command->replace(QStringLiteral("{imageUrl}"), shellQuote(QUrl::fromLocalFile(imagePath).toString()));
        replaced = true;
    }
    return replaced;
}

bool replaceExtensionSlurpPlaceholder(QString *command, const QString &geometry)
{
    if (!command || !command->contains(QStringLiteral("{slurp}"))) {
        return false;
    }

    command->replace(QStringLiteral("{slurp}"), shellQuote(geometry));
    return true;
}

void replaceShellPlaceholder(QString *command, const QString &placeholder, const QString &value, bool *replaced = nullptr)
{
    if (!command || !command->contains(placeholder)) {
        return;
    }
    command->replace(placeholder, shellQuote(value));
    if (replaced) {
        *replaced = true;
    }
}

QStringList expandDesktopExec(const ShotWindow::DesktopApp &app, const QString &imagePath)
{
    QStringList command = QProcess::splitCommand(app.exec);
    if (command.isEmpty()) {
        return {};
    }

    const QString fileUrl = QUrl::fromLocalFile(imagePath).toString();
    bool usedFileField = false;
    QStringList expanded;
    for (QString argument : command) {
        if (argument == QStringLiteral("%i")) {
            continue;
        }

        bool appendArgument = true;
        if (argument.contains(QLatin1Char('%'))) {
            if (argument.contains(QStringLiteral("%f")) || argument.contains(QStringLiteral("%F"))) {
                argument.replace(QStringLiteral("%f"), imagePath);
                argument.replace(QStringLiteral("%F"), imagePath);
                usedFileField = true;
            }
            if (argument.contains(QStringLiteral("%u")) || argument.contains(QStringLiteral("%U"))) {
                argument.replace(QStringLiteral("%u"), fileUrl);
                argument.replace(QStringLiteral("%U"), fileUrl);
                usedFileField = true;
            }
            argument.replace(QStringLiteral("%c"), app.name);
            argument.replace(QStringLiteral("%k"), app.desktopPath);
            argument.replace(QStringLiteral("%%"), QStringLiteral("%"));

            static const QStringList unsupportedFields = {
                QStringLiteral("%d"), QStringLiteral("%D"), QStringLiteral("%n"), QStringLiteral("%N"),
                QStringLiteral("%v"), QStringLiteral("%m"),
            };
            for (const QString &field : unsupportedFields) {
                argument.remove(field);
            }
            appendArgument = !argument.trimmed().isEmpty();
        }

        if (appendArgument) {
            expanded.append(argument);
        }
    }

    if (!usedFileField) {
        expanded.append(imagePath);
    }
    return expanded;
}

QString helperProgramPath(const QString &programName)
{
    const QString appDir = QCoreApplication::applicationDirPath();
    const QString configDir = markShotConfigDir();
    const QString dataDir = markShotDataDir();
#if defined(Q_OS_WIN)
    QStringList programNames{programName};
    if (QFileInfo(programName).suffix().isEmpty()) {
        programNames.prepend(programName + QStringLiteral(".bat"));
        programNames.prepend(programName + QStringLiteral(".cmd"));
        programNames.prepend(programName + QStringLiteral(".exe"));
    }

    QStringList candidates;
    const QStringList candidateDirs = {
        appDir,
        QDir(appDir).filePath(QStringLiteral("scripts")),
        QDir(appDir).filePath(QStringLiteral("../scripts")),
        QDir::current().filePath(QStringLiteral("scripts")),
        configDir,
        QDir(configDir).filePath(QStringLiteral("scripts")),
        dataDir,
        QDir(dataDir).filePath(QStringLiteral("scripts")),
    };
    for (const QString &dir : candidateDirs) {
        for (const QString &name : programNames) {
            candidates.append(QDir(dir).filePath(name));
        }
    }
#else
    const QStringList candidates = {
        QDir(appDir).filePath(programName),
        QDir(appDir).filePath(QStringLiteral("../scripts/%1").arg(programName)),
        QDir(appDir).filePath(QStringLiteral("../libexec/mark-shot/%1").arg(programName)),
        QDir(appDir).filePath(QStringLiteral("../lib/mark-shot/%1").arg(programName)),
        QDir::current().filePath(QStringLiteral("scripts/%1").arg(programName)),
        QDir(configDir).filePath(programName),
        QDir(configDir).filePath(QStringLiteral("scripts/%1").arg(programName)),
        QDir(dataDir).filePath(programName),
        QDir(dataDir).filePath(QStringLiteral("scripts/%1").arg(programName)),
        QDir::home().filePath(QStringLiteral(".local/bin/%1").arg(programName)),
        QStringLiteral("/usr/local/bin/%1").arg(programName),
        QStringLiteral("/usr/bin/%1").arg(programName),
    };
#endif

    for (const QString &candidate : candidates) {
        QFileInfo fileInfo(expandUserPath(QDir::cleanPath(candidate)));
        if (fileInfo.isExecutable()) {
            return fileInfo.absoluteFilePath();
        }
    }

    return programName;
}

struct PinnedWindowConfig {
    bool ocrEnabled = true;
    bool autoOcr = false;
    QString ocrBackend = QStringLiteral("auto");
    QString ocrCommand;
    int ocrTimeoutMs = kPinnedOcrTimeoutMs;
    QString translationCommand;
    QString translationTargetLanguage = QStringLiteral("Simplified Chinese");
    int translationTimeoutMs = kPinnedTranslationTimeoutMs;
    bool autoTranslateAfterOcr = false;
    bool borderEnabled = true;
    QColor borderColor = markshot::theme::kAccent;
    qreal borderWidth = 2.0;
};

QJsonObject objectValue(const QJsonObject &object, const QString &key)
{
    const QJsonValue value = object.value(key);
    return value.isObject() ? value.toObject() : QJsonObject();
}

QJsonObject objectValue(const QJsonObject &object, const QStringList &keys)
{
    for (const QString &key : keys) {
        const QJsonValue value = object.value(key);
        if (value.isObject()) {
            return value.toObject();
        }
    }
    return {};
}

QJsonValue valueForKeys(const QJsonObject &object, const QStringList &keys)
{
    for (const QString &key : keys) {
        const QJsonValue value = object.value(key);
        if (!value.isUndefined()) {
            return value;
        }
    }
    return {};
}

std::optional<bool> boolValue(const QJsonObject &object, const QStringList &keys)
{
    for (const QString &key : keys) {
        const QJsonValue value = object.value(key);
        if (value.isBool()) {
            return value.toBool();
        }
    }
    return std::nullopt;
}

QString normalizedConfigKey(QString key)
{
    key = key.trimmed().toLower();
    key.remove(QLatin1Char(' '));
    key.remove(QLatin1Char('_'));
    key.remove(QLatin1Char('-'));
    key.remove(QLatin1Char('.'));
    return key;
}

bool isHexColorDigits(const QString &text)
{
    if (text.isEmpty()) {
        return false;
    }
    for (const QChar ch : text) {
        if (!ch.isDigit()
            && (ch < QLatin1Char('a') || ch > QLatin1Char('f'))
            && (ch < QLatin1Char('A') || ch > QLatin1Char('F'))) {
            return false;
        }
    }
    return true;
}

std::optional<QColor> colorFromConfigString(QString value)
{
    QString text = value.trimmed();
    if (text.isEmpty()) {
        return std::nullopt;
    }
    if (text.startsWith(QStringLiteral("0x"), Qt::CaseInsensitive)) {
        text = text.mid(2);
    }
    if (!text.startsWith(QLatin1Char('#')) && (text.size() == 6 || text.size() == 8)
        && isHexColorDigits(text)) {
        text.prepend(QLatin1Char('#'));
    }
    if (text.startsWith(QLatin1Char('#')) && text.size() == 9 && isHexColorDigits(text.mid(1))) {
        bool ok = false;
        const int red = text.mid(1, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        const int green = text.mid(3, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        const int blue = text.mid(5, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        const int alpha = text.mid(7, 2).toInt(&ok, 16);
        if (!ok) return std::nullopt;
        return QColor(red, green, blue, alpha);
    }

    const QColor color(text);
    return color.isValid() ? std::optional<QColor>(color) : std::nullopt;
}

std::optional<QColor> colorFromConfigValue(const QJsonValue &value)
{
    if (value.isString()) {
        return colorFromConfigString(value.toString());
    }
    if (!value.isObject()) {
        return std::nullopt;
    }

    const QJsonObject object = value.toObject();
    for (const QString &key : {QStringLiteral("value"), QStringLiteral("color"), QStringLiteral("hex")}) {
        if (object.value(key).isString()) {
            return colorFromConfigString(object.value(key).toString());
        }
    }
    if (!object.value(QStringLiteral("r")).isDouble()
        || !object.value(QStringLiteral("g")).isDouble()
        || !object.value(QStringLiteral("b")).isDouble()) {
        return std::nullopt;
    }
    const int alpha = object.value(QStringLiteral("a")).isDouble()
        ? std::clamp(object.value(QStringLiteral("a")).toInt(), 0, 255)
        : 255;
    return QColor(std::clamp(object.value(QStringLiteral("r")).toInt(), 0, 255),
                  std::clamp(object.value(QStringLiteral("g")).toInt(), 0, 255),
                  std::clamp(object.value(QStringLiteral("b")).toInt(), 0, 255),
                  alpha);
}

std::optional<qreal> realFromConfigValue(const QJsonValue &value)
{
    if (value.isDouble()) {
        return value.toDouble();
    }
    if (value.isString()) {
        bool ok = false;
        const qreal number = value.toString().trimmed().toDouble(&ok);
        if (ok) {
            return number;
        }
    }
    return std::nullopt;
}

std::optional<int> pixelDistanceFromConfigValue(const QJsonValue &value,
                                                int minimum,
                                                int maximum)
{
    if (const std::optional<qreal> distance = realFromConfigValue(value)) {
        return std::clamp(static_cast<int>(std::lround(*distance)), minimum, maximum);
    }
    return std::nullopt;
}

using ActionShortcuts = std::array<QKeySequence, static_cast<int>(ShotWindow::Action::Cancel) + 1>;
using ToolShortcuts = std::array<QKeySequence, static_cast<int>(ShotWindow::Tool::Laser) + 1>;

struct ShortcutConfig {
    ActionShortcuts actions;
    ToolShortcuts tools;
    QKeySequence startupColorPicker;
    QKeySequence startupRuler;
};

int actionIndex(ShotWindow::Action action)
{
    return static_cast<int>(action);
}

int toolIndex(ShotWindow::Tool tool)
{
    return static_cast<int>(tool);
}

ActionShortcuts defaultActionShortcuts()
{
    ActionShortcuts shortcuts{};
    shortcuts[actionIndex(ShotWindow::Action::ToggleCaptureScope)] = QKeySequence(Qt::Key_F);
    shortcuts[actionIndex(ShotWindow::Action::Pin)] = QKeySequence(Qt::CTRL | Qt::Key_P);
    shortcuts[actionIndex(ShotWindow::Action::Copy)] = QKeySequence(Qt::CTRL | Qt::Key_C);
    shortcuts[actionIndex(ShotWindow::Action::Save)] = QKeySequence(Qt::CTRL | Qt::Key_S);
    shortcuts[actionIndex(ShotWindow::Action::Cancel)] = QKeySequence(Qt::Key_Escape);
    shortcuts[actionIndex(ShotWindow::Action::Undo)] = QKeySequence(Qt::CTRL | Qt::Key_Z);
    shortcuts[actionIndex(ShotWindow::Action::Redo)] = QKeySequence(Qt::CTRL | Qt::Key_Y);
    return shortcuts;
}

ToolShortcuts defaultToolShortcuts()
{
    ToolShortcuts shortcuts{};
    shortcuts[toolIndex(ShotWindow::Tool::Move)] = QKeySequence(Qt::Key_V);
    shortcuts[toolIndex(ShotWindow::Tool::Select)] = QKeySequence(Qt::Key_S);
    shortcuts[toolIndex(ShotWindow::Tool::Pen)] = QKeySequence(Qt::Key_P);
    shortcuts[toolIndex(ShotWindow::Tool::Line)] = QKeySequence(Qt::Key_L);
    shortcuts[toolIndex(ShotWindow::Tool::Highlighter)] = QKeySequence(Qt::Key_H);
    shortcuts[toolIndex(ShotWindow::Tool::Rectangle)] = QKeySequence(Qt::Key_R);
    shortcuts[toolIndex(ShotWindow::Tool::Ellipse)] = QKeySequence(Qt::Key_E);
    shortcuts[toolIndex(ShotWindow::Tool::Arrow)] = QKeySequence(Qt::Key_A);
    shortcuts[toolIndex(ShotWindow::Tool::Text)] = QKeySequence(Qt::Key_T);
    shortcuts[toolIndex(ShotWindow::Tool::Number)] = QKeySequence(Qt::Key_N);
    shortcuts[toolIndex(ShotWindow::Tool::Mosaic)] = QKeySequence(Qt::Key_M);
    shortcuts[toolIndex(ShotWindow::Tool::Laser)] = QKeySequence(Qt::Key_G);
    return shortcuts;
}

std::optional<QKeySequence> keySequenceFromConfigValue(const QJsonValue &value)
{
    if (!value.isString()) {
        return std::nullopt;
    }

    const QString text = value.toString().trimmed();
    if (text.isEmpty()) {
        return std::nullopt;
    }

    QKeySequence sequence(text, QKeySequence::PortableText);
    if (sequence.isEmpty()) {
        sequence = QKeySequence(text, QKeySequence::NativeText);
    }
    return sequence.isEmpty() ? std::nullopt : std::optional<QKeySequence>(sequence);
}

std::optional<ShotWindow::Action> actionFromConfigName(QString name)
{
    const QString key = normalizedConfigKey(std::move(name));
    if (key == QStringLiteral("scope") || key == QStringLiteral("togglecapturescope")
        || key == QStringLiteral("fullscreen")) {
        return ShotWindow::Action::ToggleCaptureScope;
    }
    if (key == QStringLiteral("layout") || key == QStringLiteral("toggletoolbarlayout")) {
        return ShotWindow::Action::ToggleToolbarLayout;
    }
    if (key == QStringLiteral("clear")) return ShotWindow::Action::Clear;
    if (key == QStringLiteral("undo")) return ShotWindow::Action::Undo;
    if (key == QStringLiteral("redo")) return ShotWindow::Action::Redo;
    if (key == QStringLiteral("open") || key == QStringLiteral("openwith")) return ShotWindow::Action::OpenWith;
    if (key == QStringLiteral("extension") || key == QStringLiteral("extensions")) return ShotWindow::Action::Extensions;
    if (key == QStringLiteral("pin")) return ShotWindow::Action::Pin;
    if (key == QStringLiteral("scroll") || key == QStringLiteral("scrollcapture")) return ShotWindow::Action::ScrollCapture;
    if (key == QStringLiteral("ocr") || key == QStringLiteral("ocrcopy")) return ShotWindow::Action::OcrCopy;
    if (key == QStringLiteral("magnifier") || key == QStringLiteral("magnify")
        || key == QStringLiteral("loupe") || key == QStringLiteral("zoom")) return ShotWindow::Action::ToolMagnifier;
    if (key == QStringLiteral("copy")) return ShotWindow::Action::Copy;
    if (key == QStringLiteral("save") || key == QStringLiteral("saveas")) return ShotWindow::Action::Save;
    if (key == QStringLiteral("cancel") || key == QStringLiteral("close") || key == QStringLiteral("escape")) {
        return ShotWindow::Action::Cancel;
    }
    return std::nullopt;
}

void applyToolShortcutObject(const QJsonObject &object, ToolShortcuts *shortcuts)
{
    if (!shortcuts) {
        return;
    }
    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        const std::optional<ShotWindow::Tool> tool = ShotWindow::toolFromName(it.key());
        const std::optional<QKeySequence> sequence = keySequenceFromConfigValue(it.value());
        if (tool.has_value() && sequence.has_value()) {
            (*shortcuts)[toolIndex(*tool)] = *sequence;
        }
    }
}

void applyActionShortcutObject(const QJsonObject &object, ActionShortcuts *shortcuts)
{
    if (!shortcuts) {
        return;
    }
    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        const std::optional<ShotWindow::Action> action = actionFromConfigName(it.key());
        const std::optional<QKeySequence> sequence = keySequenceFromConfigValue(it.value());
        if (action.has_value() && sequence.has_value()) {
            (*shortcuts)[actionIndex(*action)] = *sequence;
        }
    }
}

void applyStartupShortcutObject(const QJsonObject &object, ShortcutConfig *config)
{
    if (!config) {
        return;
    }

    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        const QString key = normalizedConfigKey(it.key());
        const std::optional<QKeySequence> sequence = keySequenceFromConfigValue(it.value());
        if (!sequence.has_value()) {
            continue;
        }
        if (key == QStringLiteral("colorpicker") || key == QStringLiteral("color")
            || key == QStringLiteral("pickcolor")) {
            config->startupColorPicker = *sequence;
        } else if (key == QStringLiteral("ruler") || key == QStringLiteral("measure")) {
            config->startupRuler = *sequence;
        }
    }
}

void applyShortcutObject(const QJsonObject &object, ShortcutConfig *config)
{
    if (!config || object.isEmpty()) {
        return;
    }

    applyToolShortcutObject(objectValue(object, {QStringLiteral("tools"),
                                                 QStringLiteral("tool"),
                                                 QStringLiteral("toolShortcuts")}),
                            &config->tools);
    applyActionShortcutObject(objectValue(object, {QStringLiteral("actions"),
                                                   QStringLiteral("action"),
                                                   QStringLiteral("actionShortcuts")}),
                              &config->actions);
    applyStartupShortcutObject(objectValue(object, {QStringLiteral("startup"),
                                                    QStringLiteral("startupTools"),
                                                    QStringLiteral("selection")}),
                               config);

    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        if (it.value().isObject()) {
            continue;
        }
        const std::optional<QKeySequence> sequence = keySequenceFromConfigValue(it.value());
        if (!sequence.has_value()) {
            continue;
        }
        if (const std::optional<ShotWindow::Tool> tool = ShotWindow::toolFromName(it.key())) {
            config->tools[toolIndex(*tool)] = *sequence;
            continue;
        }
        if (const std::optional<ShotWindow::Action> action = actionFromConfigName(it.key())) {
            config->actions[actionIndex(*action)] = *sequence;
            continue;
        }
        QJsonObject startupShortcut;
        startupShortcut.insert(it.key(), it.value());
        applyStartupShortcutObject(startupShortcut, config);
    }
}

ShortcutConfig configuredShortcuts()
{
    ShortcutConfig config{defaultActionShortcuts(),
                          defaultToolShortcuts(),
                          QKeySequence(Qt::Key_C),
                          QKeySequence(Qt::Key_R)};

    QFile file(appConfigPath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return config;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return config;
    }

    const QJsonObject root = document.object();
    const QJsonObject annotation = objectValue(root, QStringLiteral("annotation"));
    for (const QJsonObject &object : {
             objectValue(root, QStringLiteral("shortcuts")),
             objectValue(root, QStringLiteral("hotkeys")),
             objectValue(annotation, QStringLiteral("shortcuts")),
             objectValue(annotation, QStringLiteral("hotkeys")),
         }) {
        applyShortcutObject(object, &config);
    }
    return config;
}

QStringList jsonStringList(const QJsonValue &value)
{
    QStringList items;
    if (value.isString()) {
        items.append(value.toString());
    } else if (value.isArray()) {
        for (const QJsonValue &item : value.toArray()) {
            if (item.isString()) {
                items.append(item.toString());
            }
        }
    }
    return items;
}

bool ocrErrorLooksLikeMissingBackend(QString errorText)
{
    const QString text = errorText.toLower();
    return text.contains(QStringLiteral("no module named"))
        || text.contains(QStringLiteral("modulenotfounderror"))
        || text.contains(QStringLiteral("importerror"))
        || text.contains(QStringLiteral("not installed"))
        || text.contains(QStringLiteral("not found"))
        || text.contains(QStringLiteral("no such file or directory"));
}

bool ocrOutputReportsMissingBackend(const QByteArray &stdoutData,
                                    const QByteArray &stderrData,
                                    const QString &configuredBackend)
{
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(stdoutData, &parseError);
    QStringList errors;
    QString reportedBackend;
    QJsonArray tokenArray;
    if (parseError.error == QJsonParseError::NoError && document.isObject()) {
        const QJsonObject root = document.object();
        reportedBackend = root.value(QStringLiteral("backend")).toString().trimmed();
        errors = jsonStringList(root.value(QStringLiteral("errors")));
        tokenArray = root.value(QStringLiteral("tokens")).toArray();
    }

    if (!stderrData.trimmed().isEmpty()) {
        errors.append(QString::fromUtf8(stderrData).trimmed());
    }

    if (tokenArray.isEmpty() && reportedBackend.isEmpty() && !errors.isEmpty()) {
        return true;
    }

    const QString backend = configuredBackend.trimmed().toLower();
    if (backend == QStringLiteral("auto")) {
        return false;
    }

    for (const QString &error : std::as_const(errors)) {
        if (ocrErrorLooksLikeMissingBackend(error)) {
            return true;
        }
    }

    return tokenArray.isEmpty() && !errors.isEmpty()
        && reportedBackend.compare(backend, Qt::CaseInsensitive) == 0;
}

PinnedWindowConfig pinnedWindowConfig()
{
    PinnedWindowConfig config;

    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonObject root = document.object();
            const QJsonObject ocr = objectValue(root, QStringLiteral("ocr"));
            if (ocr.value(QStringLiteral("enabled")).isBool()) {
                config.ocrEnabled = ocr.value(QStringLiteral("enabled")).toBool();
            }
            if (const std::optional<bool> autoOcr =
                    boolValue(ocr,
                              {QStringLiteral("autoOnPin"),
                               QStringLiteral("autoPinned"),
                               QStringLiteral("autoOcr"),
                               QStringLiteral("autoOCR"),
                               QStringLiteral("background")});
                autoOcr.has_value()) {
                config.autoOcr = *autoOcr;
            }
            config.ocrBackend = ocr.value(QStringLiteral("backend")).toString(config.ocrBackend).trimmed();
            config.ocrCommand = ocr.value(QStringLiteral("command")).toString().trimmed();
            if (ocr.value(QStringLiteral("timeoutMs")).isDouble()) {
                config.ocrTimeoutMs = std::max(1000, ocr.value(QStringLiteral("timeoutMs")).toInt(config.ocrTimeoutMs));
            }

            const QJsonObject translation = objectValue(root, QStringLiteral("translation"));
            config.translationCommand = translation.value(QStringLiteral("command")).toString().trimmed();
            config.translationTargetLanguage = translation.value(QStringLiteral("targetLanguage"))
                                                   .toString(config.translationTargetLanguage)
                                                   .trimmed();
            if (const std::optional<bool> autoTranslate =
                    boolValue(translation,
                              {QStringLiteral("autoAfterOcr"),
                               QStringLiteral("autoAfterOCR"),
                               QStringLiteral("autoTranslateAfterOcr"),
                               QStringLiteral("autoTranslateAfterOCR"),
                               QStringLiteral("auto")});
                autoTranslate.has_value()) {
                config.autoTranslateAfterOcr = *autoTranslate;
            }
            if (translation.value(QStringLiteral("timeoutMs")).isDouble()) {
                config.translationTimeoutMs = std::max(3000, translation.value(QStringLiteral("timeoutMs")).toInt(config.translationTimeoutMs));
            }

            const QJsonObject pinned = objectValue(root,
                                                   {QStringLiteral("pinnedWindow"),
                                                    QStringLiteral("pinned"),
                                                    QStringLiteral("pin")});
            if (!pinned.isEmpty()) {
                if (const std::optional<bool> autoOcr =
                        boolValue(pinned,
                                  {QStringLiteral("autoOcr"),
                                   QStringLiteral("autoOCR"),
                                   QStringLiteral("autoRecognizeText"),
                                   QStringLiteral("autoTextRecognition"),
                                   QStringLiteral("ocrOnPin"),
                                   QStringLiteral("backgroundOcr"),
                                   QStringLiteral("backgroundOCR")});
                    autoOcr.has_value()) {
                    config.autoOcr = *autoOcr;
                }
                const QJsonValue border = pinned.value(QStringLiteral("border"));
                if (border.isBool()) {
                    config.borderEnabled = border.toBool();
                } else if (border.isObject()) {
                    const QJsonObject borderObject = border.toObject();
                    if (borderObject.value(QStringLiteral("enabled")).isBool()) {
                        config.borderEnabled = borderObject.value(QStringLiteral("enabled")).toBool();
                    } else {
                        config.borderEnabled = true;
                    }
                    if (const std::optional<QColor> color = colorFromConfigValue(borderObject)) {
                        config.borderColor = *color;
                    }
                    if (const std::optional<qreal> width = realFromConfigValue(borderObject.value(QStringLiteral("width")))) {
                        config.borderWidth = std::clamp(*width, 1.0, 12.0);
                    }
                }
                if (pinned.value(QStringLiteral("borderEnabled")).isBool()) {
                    config.borderEnabled = pinned.value(QStringLiteral("borderEnabled")).toBool();
                }
                if (const std::optional<QColor> color =
                        colorFromConfigValue(pinned.value(QStringLiteral("borderColor")))) {
                    config.borderColor = *color;
                }
                if (const std::optional<qreal> width =
                        realFromConfigValue(pinned.value(QStringLiteral("borderWidth")))) {
                    config.borderWidth = std::clamp(*width, 1.0, 12.0);
                }
            }
        }
    }

    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString disabled = env.value(QStringLiteral("MARK_SHOT_OCR_DISABLED")).trimmed().toLower();
    if (disabled == QStringLiteral("1") || disabled == QStringLiteral("true") || disabled == QStringLiteral("yes")) {
        config.ocrEnabled = false;
    }
    const QString autoOcr = env.value(QStringLiteral("MARK_SHOT_PINNED_AUTO_OCR")).trimmed().toLower();
    if (autoOcr == QStringLiteral("1") || autoOcr == QStringLiteral("true") || autoOcr == QStringLiteral("yes")) {
        config.autoOcr = true;
    } else if (autoOcr == QStringLiteral("0") || autoOcr == QStringLiteral("false") || autoOcr == QStringLiteral("no")) {
        config.autoOcr = false;
    }
    const QString envOcrBackend = env.value(QStringLiteral("MARK_SHOT_OCR_BACKEND")).trimmed();
    if (!envOcrBackend.isEmpty()) {
        config.ocrBackend = envOcrBackend;
    }
    const QString envOcrCommand = env.value(QStringLiteral("MARK_SHOT_OCR_COMMAND")).trimmed();
    if (!envOcrCommand.isEmpty()) {
        config.ocrCommand = envOcrCommand;
    }
    const QString envTargetLanguage = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_TARGET_LANGUAGE")).trimmed();
    if (!envTargetLanguage.isEmpty()) {
        config.translationTargetLanguage = envTargetLanguage;
    }
    const QString envTranslationCommand = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_COMMAND")).trimmed();
    if (!envTranslationCommand.isEmpty()) {
        config.translationCommand = envTranslationCommand;
    }
    const QString autoTranslate = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_AUTO_AFTER_OCR")).trimmed().toLower();
    if (autoTranslate == QStringLiteral("1") || autoTranslate == QStringLiteral("true")
        || autoTranslate == QStringLiteral("yes")) {
        config.autoTranslateAfterOcr = true;
    } else if (autoTranslate == QStringLiteral("0") || autoTranslate == QStringLiteral("false")
               || autoTranslate == QStringLiteral("no")) {
        config.autoTranslateAfterOcr = false;
    }

    if (config.ocrBackend.isEmpty()) {
        config.ocrBackend = QStringLiteral("auto");
    }
    if (config.translationTargetLanguage.isEmpty()) {
        config.translationTargetLanguage = QStringLiteral("Simplified Chinese");
    }
    return config;
}

std::optional<bool> boolFromResultPanelValue(const QJsonValue &value)
{
    if (const std::optional<bool> result = boolFromConfigValue(value)) {
        return result;
    }
    if (!value.isObject()) {
        return std::nullopt;
    }
    return boolValue(value.toObject(),
                     {QStringLiteral("enabled"),
                      QStringLiteral("show"),
                      QStringLiteral("visible"),
                      QStringLiteral("use")});
}

bool ocrResultPanelEnabled()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    if (const std::optional<bool> envValue =
            envFlagValue(env,
                         {QStringLiteral("MARK_SHOT_OCR_RESULT_PANEL"),
                          QStringLiteral("MARK_SHOT_OCR_RESULT_WINDOW")})) {
        return *envValue;
    }

    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonObject root = document.object();
            const QStringList ocrKeys = {
                QStringLiteral("resultPanel"),
                QStringLiteral("resultWindow"),
                QStringLiteral("resultPanelEnabled"),
                QStringLiteral("resultWindowEnabled"),
                QStringLiteral("ocrResultPanel"),
                QStringLiteral("ocrResultWindow"),
                QStringLiteral("showResultPanel"),
                QStringLiteral("showResultWindow"),
                QStringLiteral("useResultPanel"),
                QStringLiteral("useResultWindow"),
                QStringLiteral("newResultPanel"),
                QStringLiteral("newResultWindow"),
            };

            const QJsonObject ocr = objectValue(root, QStringLiteral("ocr"));
            if (const std::optional<bool> ocrValue =
                    boolFromResultPanelValue(valueForKeys(ocr, ocrKeys))) {
                return *ocrValue;
            }

            if (const std::optional<bool> rootValue =
                    boolFromResultPanelValue(valueForKeys(root,
                                                          {QStringLiteral("ocrResultPanel"),
                                                           QStringLiteral("ocrResultWindow"),
                                                           QStringLiteral("ocrResultPanelEnabled"),
                                                           QStringLiteral("ocrResultWindowEnabled")}))) {
                return *rootValue;
            }
        }
    }

    return true;
}

void applyScrollFrameConfig(const QJsonValue &value, markshot::scroll::ScrollSessionUiConfig *config)
{
    if (!config || value.isUndefined()) {
        return;
    }

    if (value.isBool()) {
        config->frameEnabled = value.toBool();
        return;
    }
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(value, 0, 256)) {
        config->frameEnabled = true;
        config->frameGap = *gap;
        return;
    }
    if (!value.isObject()) {
        return;
    }

    const QJsonObject object = value.toObject();
    if (const std::optional<bool> enabled =
            boolValue(object,
                      {QStringLiteral("enabled"),
                       QStringLiteral("visible"),
                       QStringLiteral("show"),
                       QStringLiteral("showFrame"),
                       QStringLiteral("showBorder")});
        enabled.has_value()) {
        config->frameEnabled = *enabled;
    } else {
        config->frameEnabled = true;
    }

    const QJsonValue gapValue =
        valueForKeys(object,
                     {QStringLiteral("gap"),
                      QStringLiteral("distance"),
                      QStringLiteral("offset"),
                      QStringLiteral("padding"),
                      QStringLiteral("margin"),
                      QStringLiteral("value"),
                      QStringLiteral("px")});
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(gapValue, 0, 256)) {
        config->frameGap = *gap;
    }
}

void applyScrollPreviewConfig(const QJsonValue &value, markshot::scroll::ScrollSessionUiConfig *config)
{
    if (!config || value.isUndefined()) {
        return;
    }
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(value, 0, 512)) {
        config->previewGap = *gap;
        return;
    }
    if (!value.isObject()) {
        return;
    }

    const QJsonObject object = value.toObject();
    if (const std::optional<bool> hideDuringCapture =
            boolValue(object,
                      {QStringLiteral("hideDuringCapture"),
                       QStringLiteral("hideWhileCapturing"),
                       QStringLiteral("hidePreviewDuringCapture"),
                       QStringLiteral("hidePreviewWhileCapturing"),
                       QStringLiteral("hidePanelDuringCapture"),
                       QStringLiteral("hidePanelWhileCapturing"),
                       QStringLiteral("hideUiDuringCapture"),
                       QStringLiteral("hideUIDuringCapture"),
                       QStringLiteral("hideUiWhileCapturing"),
                       QStringLiteral("hideUIWhileCapturing")});
        hideDuringCapture.has_value()) {
        config->hidePreviewDuringCapture = *hideDuringCapture;
    }

    const QJsonValue gapValue =
        valueForKeys(object,
                     {QStringLiteral("gap"),
                      QStringLiteral("distance"),
                      QStringLiteral("offset"),
                      QStringLiteral("margin"),
                      QStringLiteral("value"),
                      QStringLiteral("px")});
    if (const std::optional<int> gap = pixelDistanceFromConfigValue(gapValue, 0, 512)) {
        config->previewGap = *gap;
    }
}

markshot::scroll::ScrollSessionUiConfig scrollSessionUiConfig()
{
    markshot::scroll::ScrollSessionUiConfig config;

    QFile file(appConfigPath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return config;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return config;
    }

    const QJsonObject root = document.object();
    const QJsonObject scroll =
        objectValue(root,
                    {QStringLiteral("scrollCapture"),
                     QStringLiteral("scrollingCapture"),
                     QStringLiteral("scroll")});
    if (scroll.isEmpty()) {
        return config;
    }

    applyScrollFrameConfig(valueForKeys(scroll,
                                        {QStringLiteral("frame"),
                                         QStringLiteral("captureFrame"),
                                         QStringLiteral("border"),
                                         QStringLiteral("outline")}),
                           &config);

    if (const std::optional<bool> frameEnabled =
            boolValue(scroll,
                      {QStringLiteral("frameEnabled"),
                       QStringLiteral("borderEnabled"),
                       QStringLiteral("showFrame"),
                       QStringLiteral("showBorder")});
        frameEnabled.has_value()) {
        config.frameEnabled = *frameEnabled;
    }
    if (const std::optional<int> frameGap =
            pixelDistanceFromConfigValue(valueForKeys(scroll,
                                                      {QStringLiteral("frameGap"),
                                                       QStringLiteral("frameDistance"),
                                                       QStringLiteral("frameOffset"),
                                                       QStringLiteral("borderGap"),
                                                       QStringLiteral("borderDistance"),
                                                       QStringLiteral("borderOffset")}),
                                         0,
                                         256)) {
        config.frameGap = *frameGap;
    }

    applyScrollPreviewConfig(valueForKeys(scroll,
                                          {QStringLiteral("preview"),
                                           QStringLiteral("previewUi"),
                                           QStringLiteral("previewUI"),
                                           QStringLiteral("panel")}),
                             &config);
    if (const std::optional<int> previewGap =
            pixelDistanceFromConfigValue(valueForKeys(scroll,
                                                      {QStringLiteral("previewGap"),
                                                       QStringLiteral("previewDistance"),
                                                       QStringLiteral("previewOffset"),
                                                       QStringLiteral("panelGap"),
                                                       QStringLiteral("panelDistance"),
                                                       QStringLiteral("panelOffset")}),
                                         0,
                                         512)) {
        config.previewGap = *previewGap;
    }
    if (const std::optional<bool> hidePreviewDuringCapture =
            boolValue(scroll,
                      {QStringLiteral("hidePreviewDuringCapture"),
                       QStringLiteral("hidePreviewWhileCapturing"),
                       QStringLiteral("hidePanelDuringCapture"),
                       QStringLiteral("hidePanelWhileCapturing"),
                       QStringLiteral("hideUiDuringCapture"),
                       QStringLiteral("hideUIDuringCapture"),
                       QStringLiteral("hideUiWhileCapturing"),
                       QStringLiteral("hideUIWhileCapturing"),
                       QStringLiteral("alwaysHidePreview"),
                       QStringLiteral("forceHidePreview")});
        hidePreviewDuringCapture.has_value()) {
        config.hidePreviewDuringCapture = *hidePreviewDuringCapture;
    }

    return config;
}

class OcrResultWindow final : public QWidget {
public:
    explicit OcrResultWindow(QString text)
        : m_config(pinnedWindowConfig())
    {
        setWindowTitle(MS_TR("OCR Result"));
        setAttribute(Qt::WA_DeleteOnClose);
        setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
        setFocusPolicy(Qt::StrongFocus);
        setMouseTracking(true);
        setObjectName(QStringLiteral("extensionPanel"));
        setStyleSheet(markshot::theme::openWithPanelStyleSheet());

        auto *layout = new QVBoxLayout(this);
        layout->setContentsMargins(12, 12, 12, 12);
        layout->setSpacing(10);

        m_titleBar = new QWidget(this);
        m_titleBar->setCursor(Qt::SizeAllCursor);
        m_titleBar->setMinimumHeight(30);
        auto *titleLayout = new QHBoxLayout(m_titleBar);
        titleLayout->setContentsMargins(0, 0, 0, 0);
        titleLayout->setSpacing(0);
        m_titleLabel = new QLabel(MS_TR("OCR Result"), m_titleBar);
        m_titleLabel->setObjectName(QStringLiteral("ocrResultTitle"));
        m_titleLabel->setAlignment(Qt::AlignVCenter | Qt::AlignLeft);
        m_titleLabel->setCursor(Qt::SizeAllCursor);
        titleLayout->addWidget(m_titleLabel);
        m_titleBar->installEventFilter(this);
        m_titleLabel->installEventFilter(this);
        layout->addWidget(m_titleBar);

        m_hintLabel = new QLabel(MS_TR("Review or edit the recognized text before copying."), this);
        m_hintLabel->setWordWrap(true);
        layout->addWidget(m_hintLabel);

        m_editor = new QTextEdit(this);
        m_editor->setAcceptRichText(false);
        m_editor->setPlaceholderText(MS_TR("OCR text appears here"));
        m_editor->setMinimumHeight(220);
        m_editor->setPlainText(std::move(text));
        m_editor->setContextMenuPolicy(Qt::CustomContextMenu);
        connect(m_editor, &QTextEdit::customContextMenuRequested, this, [this](const QPoint &position) {
            showEditorContextMenu(m_editor->mapToGlobal(position));
        });
        layout->addWidget(m_editor);

        auto *actions = new QHBoxLayout();
        actions->setSpacing(8);
        auto *copyButton = new QPushButton(MS_TR("Copy"), this);
        m_translateButton = new QPushButton(MS_TR("Translate"), this);
        auto *closeButton = new QPushButton(MS_TR("Close"), this);
        for (QPushButton *button : {copyButton, m_translateButton, closeButton}) {
            button->setObjectName(QStringLiteral("ocrPanelButton"));
            button->setStyleSheet(markshot::theme::ocrPanelButtonStyleSheet());
        }
        connect(copyButton, &QPushButton::clicked, this, [this] {
            markshot::copyTextToClipboard(m_editor->toPlainText());
            showToast(MS_TR("OCR text copied"));
        });
        connect(m_translateButton, &QPushButton::clicked, this, [this] {
            startTranslation();
        });
        connect(closeButton, &QPushButton::clicked, this, &QWidget::close);
        actions->addWidget(copyButton);
        actions->addWidget(m_translateButton);
        actions->addWidget(closeButton);
        layout->addLayout(actions);

        resize(initialWindowSize());
        centerOnPrimaryScreen();
        m_editor->setFocus(Qt::MouseFocusReason);
    }

    ~OcrResultWindow() override
    {
        cancelTranslation();
    }

protected:
    void mousePressEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton && event->position().y() <= 44.0) {
            beginWindowDrag(event);
            return;
        }
        QWidget::mousePressEvent(event);
    }

    void mouseMoveEvent(QMouseEvent *event) override
    {
        if (updateWindowDrag(event)) {
            return;
        }
        QWidget::mouseMoveEvent(event);
    }

    void mouseReleaseEvent(QMouseEvent *event) override
    {
        if (finishWindowDrag(event)) {
            return;
        }
        QWidget::mouseReleaseEvent(event);
    }

    void keyPressEvent(QKeyEvent *event) override
    {
        if (event->key() == Qt::Key_Escape) {
            close();
            event->accept();
            return;
        }
        QWidget::keyPressEvent(event);
    }

    bool eventFilter(QObject *watched, QEvent *event) override
    {
        if (watched == m_titleBar || watched == m_titleLabel) {
            if (event->type() == QEvent::MouseButtonPress) {
                auto *mouseEvent = static_cast<QMouseEvent *>(event);
                if (mouseEvent->button() == Qt::LeftButton) {
                    return beginWindowDrag(mouseEvent);
                }
            } else if (event->type() == QEvent::MouseMove && m_dragging) {
                return updateWindowDrag(static_cast<QMouseEvent *>(event));
            } else if (event->type() == QEvent::MouseButtonRelease && m_dragging) {
                return finishWindowDrag(static_cast<QMouseEvent *>(event));
            }
        }
        return QWidget::eventFilter(watched, event);
    }

private:
    bool beginWindowDrag(QMouseEvent *event)
    {
        if (!event || event->button() != Qt::LeftButton) {
            return false;
        }

        if (QWindow *window = windowHandle()) {
            if (window->startSystemMove()) {
                event->accept();
                return true;
            }
        }

        m_dragging = true;
        m_dragOffset = event->globalPosition().toPoint() - frameGeometry().topLeft();
        setCursor(Qt::SizeAllCursor);
        grabMouse();
        event->accept();
        return true;
    }

    bool updateWindowDrag(QMouseEvent *event)
    {
        if (!event || !m_dragging) {
            return false;
        }

        move(event->globalPosition().toPoint() - m_dragOffset);
        event->accept();
        return true;
    }

    bool finishWindowDrag(QMouseEvent *event)
    {
        if (!event || event->button() != Qt::LeftButton || !m_dragging) {
            return false;
        }

        m_dragging = false;
        if (QWidget::mouseGrabber() == this) {
            releaseMouse();
        }
        unsetCursor();
        event->accept();
        return true;
    }

    QSize initialWindowSize() const
    {
        QSize size(420, 520);
        if (QScreen *screen = QApplication::primaryScreen()) {
            const QSize available = screen->availableGeometry().size();
            size.setWidth(std::min(size.width(), std::max(320, qRound(available.width() * 0.9))));
            size.setHeight(std::min(size.height(), std::max(260, qRound(available.height() * 0.9))));
        }
        return size;
    }

    void centerOnPrimaryScreen()
    {
        if (QScreen *screen = QApplication::primaryScreen()) {
            move(screen->availableGeometry().center() - rect().center());
        }
    }

    void showToast(const QString &text, int durationMs = 2000)
    {
        auto *label = new QLabel(text, this);
        label->setAlignment(Qt::AlignCenter);
        label->setFont(markshot::theme::uiFont(12, QFont::DemiBold));
        label->setStyleSheet(QStringLiteral(
            "background: rgba(8, 13, 19, 220);"
            "color: rgba(204, 251, 241, 238);"
            "border-radius: 14px;"
            "padding: 8px 22px;"));
        label->adjustSize();
        label->move((width() - label->width()) / 2, height() - label->height() - 24);
        label->show();
        QTimer::singleShot(durationMs, label, &QObject::deleteLater);
    }

    template <typename Callback>
    QAction *addEditorMenuAction(QMenu *menu,
                                 const QString &text,
                                 const QKeySequence &shortcut,
                                 bool enabled,
                                 Callback callback)
    {
        QAction *action = menu->addAction(text, this, callback);
        action->setShortcut(shortcut);
        action->setShortcutVisibleInContextMenu(true);
        action->setEnabled(enabled);
        return action;
    }

    void showEditorContextMenu(const QPoint &globalPosition)
    {
        if (!m_editor) {
            return;
        }

        QMenu menu(this);
        const QTextCursor cursor = m_editor->textCursor();
        const bool readOnly = m_editor->isReadOnly();
        const bool hasSelection = cursor.hasSelection();
        const bool hasDocumentText = !m_editor->document()->isEmpty();
        const bool hasClipboardText = !QApplication::clipboard()->text().isEmpty();

        addEditorMenuAction(&menu,
                            MS_TR("Undo"),
                            QKeySequence::Undo,
                            !readOnly && m_editor->document()->isUndoAvailable(),
                            [this] { m_editor->undo(); });
        addEditorMenuAction(&menu,
                            MS_TR("Redo"),
                            QKeySequence::Redo,
                            !readOnly && m_editor->document()->isRedoAvailable(),
                            [this] { m_editor->redo(); });
        menu.addSeparator();
        addEditorMenuAction(&menu,
                            MS_TR("Cut"),
                            QKeySequence::Cut,
                            !readOnly && hasSelection,
                            [this] { m_editor->cut(); });
        addEditorMenuAction(&menu,
                            MS_TR("Copy"),
                            QKeySequence::Copy,
                            hasSelection,
                            [this] { m_editor->copy(); });
        addEditorMenuAction(&menu,
                            MS_TR("Paste"),
                            QKeySequence::Paste,
                            !readOnly && hasClipboardText,
                            [this] { m_editor->paste(); });
        addEditorMenuAction(&menu,
                            MS_TR("Delete"),
                            QKeySequence(Qt::Key_Delete),
                            !readOnly && hasSelection,
                            [this] {
                                QTextCursor selection = m_editor->textCursor();
                                selection.removeSelectedText();
                                m_editor->setTextCursor(selection);
                            });
        menu.addSeparator();
        addEditorMenuAction(&menu,
                            MS_TR("Select All"),
                            QKeySequence::SelectAll,
                            hasDocumentText,
                            [this] { m_editor->selectAll(); });

        menu.exec(globalPosition);
    }

    void startTranslation()
    {
        if (!m_editor || !m_translateButton || m_translationProcess) {
            return;
        }

        const QString text = m_editor->toPlainText().trimmed();
        if (text.isEmpty()) {
            showToast(MS_TR("No text to translate"));
            return;
        }

        QJsonArray tokens;
        const QStringList lines = text.split(QLatin1Char('\n'));
        int lineIndex = 0;
        for (const QString &rawLine : lines) {
            const QString line = rawLine.trimmed();
            if (line.isEmpty()) {
                ++lineIndex;
                continue;
            }

            QJsonObject object;
            object.insert(QStringLiteral("text"), line);
            object.insert(QStringLiteral("box"),
                          QJsonArray{0, static_cast<double>(lineIndex) * 24.0, 1000.0, 20.0});
            object.insert(QStringLiteral("line"), lineIndex);
            object.insert(QStringLiteral("index"), 0);
            object.insert(QStringLiteral("confidence"), 1.0);
            tokens.append(object);
            ++lineIndex;
        }

        if (tokens.isEmpty()) {
            showToast(MS_TR("No text to translate"));
            return;
        }

        QJsonObject root;
        root.insert(QStringLiteral("targetLanguage"), m_config.translationTargetLanguage);
        root.insert(QStringLiteral("tokens"), tokens);

        QTemporaryFile inputFile(
            QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                     ? QDir::tempPath()
                     : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                .filePath(QStringLiteral("mark-shot-ocr-result-translate-XXXXXX.json")));
        inputFile.setAutoRemove(false);
        if (!inputFile.open()) {
            showToast(MS_TR("Translation failed"));
            return;
        }

        m_translationInputPath = inputFile.fileName();
        inputFile.write(QJsonDocument(root).toJson(QJsonDocument::Compact));
        inputFile.close();

        auto *process = new QProcess(this);
        m_translationProcess = process;
        process->setProcessEnvironment(QProcessEnvironment::systemEnvironment());

        if (!m_config.translationCommand.isEmpty()) {
            QString commandLine = m_config.translationCommand;
            bool replaced = false;
            replaceShellPlaceholder(&commandLine, QStringLiteral("{input}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{inputPath}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine,
                                    QStringLiteral("{targetLanguage}"),
                                    m_config.translationTargetLanguage,
                                    &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{config}"), appConfigPath(), &replaced);
            if (!replaced) {
                commandLine += QLatin1Char(' ');
                commandLine += shellQuote(m_translationInputPath);
            }

            setShellCommand(process, commandLine);
        } else {
            process->setProgram(helperProgramPath(QStringLiteral("mark-shot-translate")));
            process->setArguments({QStringLiteral("--input"),
                                   m_translationInputPath,
                                   QStringLiteral("--target-language"),
                                   m_config.translationTargetLanguage,
                                   QStringLiteral("--config"),
                                   appConfigPath()});
        }

        m_hintLabel->setText(MS_TR("Translating edited OCR text. Keep this window open."));
        m_translateButton->setEnabled(false);
        m_translateButton->setText(MS_TR("Translating..."));

        connect(process, &QProcess::errorOccurred, this, [this, process] {
            if (process == m_translationProcess && process->state() == QProcess::NotRunning) {
                finishTranslation(process, QByteArray());
            }
        });
        connect(process,
                QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
                this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
                    const QByteArray output = exitStatus == QProcess::NormalExit && exitCode == 0
                        ? process->readAllStandardOutput()
                        : QByteArray();
                    finishTranslation(process, output);
                });
        QTimer::singleShot(m_config.translationTimeoutMs, process, [process] {
            if (process->state() != QProcess::NotRunning) {
                process->kill();
            }
        });

        process->start();
    }

    void finishTranslation(QProcess *process, const QByteArray &output)
    {
        if (process != m_translationProcess) {
            return;
        }

        QStringList translatedLines;
        if (!output.isEmpty()) {
            QJsonParseError parseError;
            const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
            if (parseError.error == QJsonParseError::NoError && document.isObject()) {
                const QJsonArray tokenArray =
                    document.object().value(QStringLiteral("tokens")).toArray();
                translatedLines.reserve(tokenArray.size());
                for (const QJsonValue &value : tokenArray) {
                    if (!value.isObject()) {
                        continue;
                    }
                    translatedLines.append(value.toObject()
                                               .value(QStringLiteral("text"))
                                               .toString()
                                               .trimmed());
                }
            }
        }

        const QString translatedText = translatedLines.join(QLatin1Char('\n')).trimmed();
        if (!translatedText.isEmpty() && m_editor) {
            m_editor->setPlainText(translatedText);
        } else {
            showToast(MS_TR("Translation failed"));
        }

        finishTranslationCleanup(process);
    }

    void cancelTranslation()
    {
        if (m_translationProcess) {
            disconnect(m_translationProcess, nullptr, this, nullptr);
            if (m_translationProcess->state() != QProcess::NotRunning) {
                m_translationProcess->kill();
            }
            QProcess *process = m_translationProcess;
            m_translationProcess = nullptr;
            process->deleteLater();
        }

        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        resetTranslationUi();
    }

    void finishTranslationCleanup(QProcess *process)
    {
        m_translationProcess = nullptr;
        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        resetTranslationUi();
        process->deleteLater();
    }

    void resetTranslationUi()
    {
        if (m_translateButton) {
            m_translateButton->setEnabled(true);
            m_translateButton->setText(MS_TR("Translate"));
        }
        if (m_hintLabel) {
            m_hintLabel->setText(MS_TR("Review or edit the recognized text before copying."));
        }
    }

    QWidget *m_titleBar = nullptr;
    QLabel *m_titleLabel = nullptr;
    QTextEdit *m_editor = nullptr;
    QLabel *m_hintLabel = nullptr;
    QPushButton *m_translateButton = nullptr;
    QProcess *m_translationProcess = nullptr;
    QString m_translationInputPath;
    QPoint m_dragOffset;
    PinnedWindowConfig m_config;
    bool m_dragging = false;
};

class PinnedImageWindow final : public QWidget {
public:
    struct OcrToken {
        QString text;
        QRectF imageRect;
        int line = 0;
        int index = 0;
        qreal confidence = 0.0;
    };

    explicit PinnedImageWindow(QImage image)
        : m_pixmap(QPixmap::fromImage(std::move(image)))
        , m_imageSize(m_pixmap.size())
        , m_config(pinnedWindowConfig())
    {
        setWindowTitle(MS_TR("Pinned Mark Shot"));
        setAttribute(Qt::WA_DeleteOnClose);
        setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
        setFocusPolicy(Qt::StrongFocus);
        setMouseTracking(true);
        setCursor(Qt::OpenHandCursor);

        QSize targetSize = m_imageSize;
        if (QScreen *screen = QApplication::primaryScreen()) {
            const QSize maxSize = screen->availableGeometry().size() * 0.9;
            if (targetSize.width() > maxSize.width() || targetSize.height() > maxSize.height()) {
                targetSize.scale(maxSize, Qt::KeepAspectRatio);
            }
            m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_imageSize.width());
            setFixedSize(targetSize);
            move(screen->availableGeometry().center() - rect().center());
        } else {
            setFixedSize(targetSize);
        }

        if (m_config.autoOcr) {
            QTimer::singleShot(0, this, [this] { startOcr(); });
        }
    }

    ~PinnedImageWindow() override
    {
        cancelTranslation();
        cancelOcr();
    }

protected:
    void paintEvent(QPaintEvent *) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        painter.drawPixmap(rect(), m_pixmap);

        if (m_translationActive) {
            drawTranslationOverlay(painter);
        }

        auto drawBorder = [this, &painter] {
            if (!m_config.borderEnabled || !m_config.borderColor.isValid() || m_config.borderWidth <= 0.0) {
                return;
            }
            painter.save();
            painter.setRenderHint(QPainter::Antialiasing, false);
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(m_config.borderColor, m_config.borderWidth));
            const qreal inset = m_config.borderWidth / 2.0;
            painter.drawRect(QRectF(rect()).adjusted(inset, inset, -inset, -inset));
            painter.restore();
        };

        if (!hasTextSelection()) {
            drawBorder();
            return;
        }

        painter.setRenderHint(QPainter::Antialiasing, false);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(72, 132, 245, 96));
        const auto [first, last] = selectionRange();
        const QVector<OcrToken> &tokens = activeTokens();
        for (int i = first; i <= last; ++i) {
            painter.drawRect(imageToWidget(tokens.at(i).imageRect).intersected(QRectF(rect())));
        }
        drawBorder();
    }

    void mousePressEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (const std::optional<int> token = tokenAt(widgetToImage(event->position()))) {
                m_selectingText = true;
                m_selectionAnchor = *token;
                m_selectionFocus = *token;
                setCursor(Qt::IBeamCursor);
                update();
                event->accept();
                return;
            }

            clearTextSelection();
            m_dragOffset = event->globalPosition().toPoint() - frameGeometry().topLeft();
            setCursor(Qt::ClosedHandCursor);
            if (QWindow *window = windowHandle()) {
                if (window->startSystemMove()) {
                    event->accept();
                    return;
                }
            }
            event->accept();
            return;
        }
        QWidget::mousePressEvent(event);
    }

    void mouseMoveEvent(QMouseEvent *event) override
    {
        if (m_selectingText) {
            const QPointF imagePoint = widgetToImage(event->position());
            const std::optional<int> token = tokenAt(imagePoint).has_value()
                ? tokenAt(imagePoint)
                : closestToken(imagePoint);
            if (token && m_selectionFocus != *token) {
                m_selectionFocus = *token;
                update();
            }
            event->accept();
            return;
        }

        if (event->buttons().testFlag(Qt::LeftButton)) {
            move(event->globalPosition().toPoint() - m_dragOffset);
            event->accept();
            return;
        }

        updateCursorForPosition(event->position());
        QWidget::mouseMoveEvent(event);
    }

    void mouseReleaseEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (m_selectingText) {
                m_selectingText = false;
                updateCursorForPosition(event->position());
                event->accept();
                return;
            }

            updateCursorForPosition(event->position());
            event->accept();
            return;
        }
        QWidget::mouseReleaseEvent(event);
    }

    void wheelEvent(QWheelEvent *event) override
    {
        const QPoint delta = event->angleDelta();
        if (delta.y() == 0) {
            QWidget::wheelEvent(event);
            return;
        }

        const qreal factor = std::pow(1.12, static_cast<qreal>(delta.y()) / 120.0);
        resizeByScale(m_scale * factor, event->globalPosition().toPoint(), event->position());
        event->accept();
    }

    void mouseDoubleClickEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (const std::optional<int> token = tokenAt(widgetToImage(event->position()))) {
                m_selectionAnchor = *token;
                m_selectionFocus = *token;
                update();
                event->accept();
                return;
            }

            close();
            event->accept();
            return;
        }
        QWidget::mouseDoubleClickEvent(event);
    }

    void contextMenuEvent(QContextMenuEvent *event) override
    {
        class LeftClickMenuFilter final : public QObject {
        public:
            explicit LeftClickMenuFilter(QObject *parent = nullptr) : QObject(parent) {}
        protected:
            bool eventFilter(QObject *obj, QEvent *event) override
            {
                if (event->type() == QEvent::MouseButtonPress || event->type() == QEvent::MouseButtonRelease || event->type() == QEvent::MouseButtonDblClick) {
                    QMouseEvent *me = static_cast<QMouseEvent *>(event);
                    if (me->button() != Qt::LeftButton) {
                        QWidget *widget = qobject_cast<QWidget *>(obj);
                        if (widget && widget->rect().contains(me->position().toPoint())) {
                            return true;
                        }
                    }
                }
                return QObject::eventFilter(obj, event);
            }
        };

        QMenu menu(this);
        LeftClickMenuFilter filter(&menu);
        menu.installEventFilter(&filter);

        menu.addAction(MS_TR("Rotate Left"), this, [this] { rotateImage(-90); });
        menu.addAction(MS_TR("Rotate Right"), this, [this] { rotateImage(90); });
        menu.addSeparator();
        menu.addAction(MS_TR("Zoom In"), this, [this, event] {
            resizeByScale(m_scale * 1.18, event->globalPos(), rect().center());
        });
        menu.addAction(MS_TR("Zoom Out"), this, [this, event] {
            resizeByScale(m_scale / 1.18, event->globalPos(), rect().center());
        });
        menu.addAction(MS_TR("Reset Size"), this, [this] {
            resizeByScale(1.0, frameGeometry().center(), QPointF(width() / 2.0, height() / 2.0));
        });
        menu.addSeparator();
        menu.addAction(MS_TR("Copy"), this, [this] {
            markshot::copyImageToClipboard(m_pixmap.toImage());
        });
        QAction *copySelectedTextAction = menu.addAction(MS_TR("Copy Selected Text"), this, [this] {
            markshot::copyTextToClipboard(selectedText());
        });
        copySelectedTextAction->setEnabled(hasTextSelection());
        QAction *copyTextAction = menu.addAction(MS_TR("Copy Image Text"), this, [this] {
            copyImageText();
        });
        copyTextAction->setEnabled(m_config.ocrEnabled);
        QAction *translateAction = menu.addAction(MS_TR("Translate"), this, [this] {
            requestTranslation();
        });
        translateAction->setEnabled(canRequestTranslation());
        QAction *toggleTranslationAction = menu.addAction(
            m_translationActive ? MS_TR("Show Original Text") : MS_TR("Show Translated Text"),
            this,
            [this] { setTranslationActive(!m_translationActive); });
        toggleTranslationAction->setEnabled(!m_translationOverlayTokens.isEmpty() && !m_translationProcess);
        menu.addAction(MS_TR("Save As"), this, [this] { saveImageAs(); });
        menu.addSeparator();
        menu.addAction(MS_TR("Close"), this, &QWidget::close);
        menu.exec(event->globalPos());
    }

    void keyPressEvent(QKeyEvent *event) override
    {
        if (event->matches(QKeySequence::Copy) && hasTextSelection()) {
            markshot::copyTextToClipboard(selectedText());
            event->accept();
            return;
        }

        if (event->key() == Qt::Key_Escape) {
            close();
            return;
        }
        QWidget::keyPressEvent(event);
    }

private:
    void rotateImage(qreal degrees)
    {
        const QPoint center = frameGeometry().center();
        m_pixmap = m_pixmap.transformed(QTransform().rotate(degrees), Qt::SmoothTransformation);
        m_imageSize = m_pixmap.size();
        clearTextSelection();
        m_ocrTokens.clear();
        m_translatedTokens.clear();
        m_translationOverlayTokens.clear();
        m_translationActive = false;
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
        cancelTranslation();
        resizeByScale(m_scale, center, QPointF(width() / 2.0, height() / 2.0));
        update();
        if (m_config.autoOcr) {
            startOcr();
        }
    }

    void saveImageAs()
    {
        const QString filename = QStringLiteral("mark-shot-pin-%1.png").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
        const QString path = QFileDialog::getSaveFileName(this,
                                                          MS_TR("Save Pinned Image"),
                                                          QDir(markShotPicturesDir()).filePath(filename),
                                                          MS_TR("PNG Images (*.png)"));
        if (!path.isEmpty()) {
            m_pixmap.save(path, "PNG");
        }
    }

    void resizeByScale(qreal scale, QPoint globalAnchor, QPointF localAnchor)
    {
        scale = std::clamp(scale, 0.1, 6.0);
        QSize targetSize(qMax(24, qRound(m_imageSize.width() * scale)),
                         qMax(24, qRound(m_imageSize.height() * scale)));
        targetSize.scale(targetSize, Qt::KeepAspectRatio);

        const qreal xRatio = width() > 0 ? localAnchor.x() / width() : 0.5;
        const qreal yRatio = height() > 0 ? localAnchor.y() / height() : 0.5;
        const QPoint topLeft(globalAnchor.x() - qRound(targetSize.width() * xRatio),
                             globalAnchor.y() - qRound(targetSize.height() * yRatio));

        m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_imageSize.width());
        setFixedSize(targetSize);
        move(topLeft);
    }

    void startOcr()
    {
        cancelOcr();

        if (!m_config.ocrEnabled) {
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
            return;
        }

        QTemporaryFile tempFile(QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                                    ? QDir::tempPath()
                                    : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                                    .filePath(QStringLiteral("mark-shot-pin-ocr-XXXXXX.png")));
        tempFile.setAutoRemove(false);
        if (!tempFile.open()) {
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
            return;
        }

        m_ocrTempPath = tempFile.fileName();
        if (!m_pixmap.save(&tempFile, "PNG")) {
            tempFile.close();
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
            return;
        }
        tempFile.close();

        auto *process = new QProcess(this);
        m_ocrProcess = process;

        if (!m_config.ocrCommand.isEmpty()) {
            QString commandLine = m_config.ocrCommand;
            const bool replaced = replaceExtensionImagePlaceholders(&commandLine, m_ocrTempPath);
            if (!replaced) {
                commandLine += QLatin1Char(' ');
                commandLine += shellQuote(m_ocrTempPath);
            }

            setShellCommand(process, commandLine);
        } else {
            process->setProgram(defaultOcrHelperProgram());
            process->setArguments({QStringLiteral("--format"),
                                   QStringLiteral("json"),
                                   QStringLiteral("--backend"),
                                   m_config.ocrBackend,
                                   m_ocrTempPath});
        }

        connect(process, &QProcess::errorOccurred, this, [this, process](QProcess::ProcessError error) {
            if (process == m_ocrProcess && process->state() == QProcess::NotRunning) {
                finishOcr(process,
                          QByteArray(),
                          process->readAllStandardError(),
                          -1,
                          QProcess::CrashExit,
                          error);
            }
        });
        connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            finishOcr(process,
                      process->readAllStandardOutput(),
                      process->readAllStandardError(),
                      exitCode,
                      exitStatus,
                      process->error());
        });
        QTimer::singleShot(m_config.ocrTimeoutMs, process, [process] {
            if (process->state() != QProcess::NotRunning) {
                process->kill();
            }
        });

        process->start();
    }

    QString defaultOcrHelperProgram() const
    {
        return helperProgramPath(QStringLiteral("mark-shot-ocr"));
    }

    void cancelOcr()
    {
        if (m_ocrProcess) {
            disconnect(m_ocrProcess, nullptr, this, nullptr);
            if (m_ocrProcess->state() != QProcess::NotRunning) {
                m_ocrProcess->kill();
            }
            m_ocrProcess->deleteLater();
            m_ocrProcess = nullptr;
        }

        if (!m_ocrTempPath.isEmpty()) {
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
        }
    }

    void finishOcr(QProcess *process,
                   const QByteArray &output,
                   const QByteArray &errorOutput,
                   int exitCode,
                   QProcess::ExitStatus exitStatus,
                   QProcess::ProcessError processError)
    {
        if (process != m_ocrProcess) {
            return;
        }

        const bool success = exitStatus == QProcess::NormalExit && exitCode == 0;
        if (success && !output.isEmpty()) {
            applyOcrOutput(output, errorOutput);
        } else if (processError == QProcess::FailedToStart && m_config.ocrCommand.isEmpty()) {
            notifyMissingOcrBackend();
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
        } else if (m_config.ocrCommand.isEmpty()
                   && ocrOutputReportsMissingBackend(output, errorOutput, m_config.ocrBackend)) {
            notifyMissingOcrBackend();
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
        } else {
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
        }

        m_ocrProcess = nullptr;
        if (!m_ocrTempPath.isEmpty()) {
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
        }
        process->deleteLater();
    }

    void applyOcrOutput(const QByteArray &output, const QByteArray &errorOutput)
    {
        const QVector<OcrToken> tokens = tokensFromJsonOutput(output);
        if (tokens.isEmpty()) {
            if (m_config.ocrCommand.isEmpty()
                && ocrOutputReportsMissingBackend(output, errorOutput, m_config.ocrBackend)) {
                notifyMissingOcrBackend();
            }
            m_translateAfterOcr = false;
            m_copyTextAfterOcr = false;
            return;
        }

        m_ocrTokens = tokens;
        m_translatedTokens.clear();
        m_translationOverlayTokens.clear();
        m_translationActive = false;
        const bool translateAfterOcr = m_translateAfterOcr;
        const bool copyTextAfterOcr = m_copyTextAfterOcr;
        m_translateAfterOcr = false;
        m_copyTextAfterOcr = false;
        updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        if (copyTextAfterOcr) {
            markshot::copyTextToClipboard(allText());
        }
        if (translateAfterOcr) {
            startTranslation(true);
        } else if (m_config.autoTranslateAfterOcr) {
            startTranslation(false, false);
        } else {
            update();
        }
    }

    void notifyMissingOcrBackend()
    {
        if (m_ocrBackendWarningShown) {
            return;
        }
        m_ocrBackendWarningShown = true;
        sendDesktopNotification(QStringLiteral("Mark Shot"),
                                MS_TR("OCR backend not installed. Install rapidocr or tesseract."));
    }

    QVector<OcrToken> tokensFromJsonOutput(const QByteArray &output) const
    {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
        if (parseError.error != QJsonParseError::NoError) {
            return {};
        }

        QJsonArray tokenArray;
        if (document.isArray()) {
            tokenArray = document.array();
        } else if (document.isObject()) {
            tokenArray = document.object().value(QStringLiteral("tokens")).toArray();
        }

        QVector<OcrToken> tokens;
        tokens.reserve(tokenArray.size());
        int fallbackIndex = 0;
        for (const QJsonValue &value : tokenArray) {
            if (!value.isObject()) {
                continue;
            }

            const QJsonObject object = value.toObject();
            OcrToken token;
            token.text = object.value(QStringLiteral("text")).toString().trimmed();
            if (token.text.isEmpty()) {
                continue;
            }

            const std::optional<QRectF> rect = ocrRect(object);
            if (!rect) {
                continue;
            }

            token.imageRect = rect->normalized().intersected(QRectF(QPointF(0.0, 0.0), QSizeF(m_imageSize)));
            if (token.imageRect.isEmpty()) {
                continue;
            }

            token.line = object.value(QStringLiteral("line")).toInt(0);
            token.index = object.value(QStringLiteral("index")).toInt(fallbackIndex++);
            token.confidence = object.value(QStringLiteral("confidence")).toDouble(0.0);
            tokens.append(token);
        }

        std::stable_sort(tokens.begin(), tokens.end(), [](const OcrToken &left, const OcrToken &right) {
            if (left.line != right.line) {
                return left.line < right.line;
            }
            if (left.index != right.index) {
                return left.index < right.index;
            }
            if (!qFuzzyCompare(left.imageRect.top(), right.imageRect.top())) {
                return left.imageRect.top() < right.imageRect.top();
            }
            return left.imageRect.left() < right.imageRect.left();
        });

        return tokens;
    }

    void startTranslation(bool activateWhenFinished = true, bool showBusyCursor = true)
    {
        if (m_ocrTokens.isEmpty()) {
            return;
        }

        cancelTranslation();
        clearTextSelection();
        m_activateTranslationWhenFinished = activateWhenFinished;

        QTemporaryFile inputFile(QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                                     ? QDir::tempPath()
                                     : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                                     .filePath(QStringLiteral("mark-shot-translate-XXXXXX.json")));
        inputFile.setAutoRemove(false);
        if (!inputFile.open()) {
            m_activateTranslationWhenFinished = true;
            return;
        }

        m_translationInputPath = inputFile.fileName();
        inputFile.write(translationInputJson());
        inputFile.close();

        auto *process = new QProcess(this);
        m_translationProcess = process;

        if (!m_config.translationCommand.isEmpty()) {
            QString commandLine = m_config.translationCommand;
            bool replaced = false;
            replaceShellPlaceholder(&commandLine, QStringLiteral("{input}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{inputPath}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{targetLanguage}"), m_config.translationTargetLanguage, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{config}"), appConfigPath(), &replaced);
            if (!replaced) {
                commandLine += QLatin1Char(' ');
                commandLine += shellQuote(m_translationInputPath);
            }

            setShellCommand(process, commandLine);
        } else {
            process->setProgram(defaultTranslationHelperProgram());
            process->setArguments({QStringLiteral("--input"),
                                   m_translationInputPath,
                                   QStringLiteral("--target-language"),
                                   m_config.translationTargetLanguage,
                                   QStringLiteral("--config"),
                                   appConfigPath()});
        }

        connect(process, &QProcess::errorOccurred, this, [this, process] {
            if (process == m_translationProcess && process->state() == QProcess::NotRunning) {
                finishTranslation(process, QByteArray());
            }
        });
        connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            const QByteArray output = exitStatus == QProcess::NormalExit && exitCode == 0
                ? process->readAllStandardOutput()
                : QByteArray();
            finishTranslation(process, output);
        });
        QTimer::singleShot(m_config.translationTimeoutMs, process, [process] {
            if (process->state() != QProcess::NotRunning) {
                process->kill();
            }
        });

        if (showBusyCursor) {
            setTranslationBusyCursor(true);
        }
        process->start();
        update();
    }

    QByteArray translationInputJson() const
    {
        QJsonArray tokens;
        for (const OcrToken &token : m_ocrTokens) {
            QJsonObject object;
            object.insert(QStringLiteral("text"), token.text);
            object.insert(QStringLiteral("box"), rectToJson(token.imageRect));
            object.insert(QStringLiteral("line"), token.line);
            object.insert(QStringLiteral("index"), token.index);
            object.insert(QStringLiteral("confidence"), token.confidence);
            tokens.append(object);
        }

        QJsonObject root;
        root.insert(QStringLiteral("targetLanguage"), m_config.translationTargetLanguage);
        root.insert(QStringLiteral("tokens"), tokens);
        return QJsonDocument(root).toJson(QJsonDocument::Compact);
    }

    QJsonArray rectToJson(QRectF rect) const
    {
        QJsonArray array;
        array.append(rect.x());
        array.append(rect.y());
        array.append(rect.width());
        array.append(rect.height());
        return array;
    }

    QString defaultTranslationHelperProgram() const
    {
        return helperProgramPath(QStringLiteral("mark-shot-translate"));
    }

    void cancelTranslation()
    {
        if (m_translationProcess) {
            disconnect(m_translationProcess, nullptr, this, nullptr);
            if (m_translationProcess->state() != QProcess::NotRunning) {
                m_translationProcess->kill();
            }
            m_translationProcess->deleteLater();
            m_translationProcess = nullptr;
        }

        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        m_activateTranslationWhenFinished = true;
        setTranslationBusyCursor(false);
    }

    void finishTranslation(QProcess *process, const QByteArray &output)
    {
        if (process != m_translationProcess) {
            return;
        }

        if (!output.isEmpty()) {
            const QVector<OcrToken> tokens = tokensFromJsonOutput(output);
            if (!tokens.isEmpty()) {
                m_translationOverlayTokens = tokens;
                m_translatedTokens = selectableTranslationTokens(tokens);
                m_translationActive = m_activateTranslationWhenFinished;
                clearTextSelection();
                updateCursorForPosition(mapFromGlobal(QCursor::pos()));
                update();
            }
        }

        m_translationProcess = nullptr;
        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        setTranslationBusyCursor(false);
        m_activateTranslationWhenFinished = true;
        process->deleteLater();
    }

    std::optional<QRectF> ocrRect(const QJsonObject &object) const
    {
        if (object.contains(QStringLiteral("box"))) {
            return rectFromJsonValue(object.value(QStringLiteral("box")));
        }
        if (object.contains(QStringLiteral("bbox"))) {
            return rectFromJsonValue(object.value(QStringLiteral("bbox")));
        }
        if (object.contains(QStringLiteral("points"))) {
            return rectFromJsonValue(object.value(QStringLiteral("points")));
        }

        if (object.contains(QStringLiteral("x")) && object.contains(QStringLiteral("y"))) {
            return QRectF(object.value(QStringLiteral("x")).toDouble(),
                          object.value(QStringLiteral("y")).toDouble(),
                          object.value(QStringLiteral("width")).toDouble(),
                          object.value(QStringLiteral("height")).toDouble());
        }

        if (object.contains(QStringLiteral("left")) && object.contains(QStringLiteral("top"))) {
            return QRectF(object.value(QStringLiteral("left")).toDouble(),
                          object.value(QStringLiteral("top")).toDouble(),
                          object.value(QStringLiteral("width")).toDouble(),
                          object.value(QStringLiteral("height")).toDouble());
        }

        return std::nullopt;
    }

    std::optional<QRectF> rectFromJsonValue(const QJsonValue &value) const
    {
        if (!value.isArray()) {
            return std::nullopt;
        }

        const QJsonArray array = value.toArray();
        if (array.size() == 4 && array.at(0).isDouble()) {
            return QRectF(array.at(0).toDouble(),
                          array.at(1).toDouble(),
                          array.at(2).toDouble(),
                          array.at(3).toDouble());
        }

        if (array.size() < 2 || !array.at(0).isArray()) {
            return std::nullopt;
        }

        QRectF bounds;
        bool initialized = false;
        for (const QJsonValue &pointValue : array) {
            if (!pointValue.isArray()) {
                continue;
            }
            const QJsonArray point = pointValue.toArray();
            if (point.size() < 2) {
                continue;
            }
            const QPointF p(point.at(0).toDouble(), point.at(1).toDouble());
            bounds = initialized ? bounds.united(QRectF(p, QSizeF(0.0, 0.0))) : QRectF(p, QSizeF(0.0, 0.0));
            initialized = true;
        }

        if (!initialized) {
            return std::nullopt;
        }
        return bounds;
    }

    QPointF widgetToImage(QPointF point) const
    {
        if (width() <= 0 || height() <= 0 || m_imageSize.isEmpty()) {
            return {};
        }
        return QPointF(point.x() * static_cast<qreal>(m_imageSize.width()) / static_cast<qreal>(width()),
                       point.y() * static_cast<qreal>(m_imageSize.height()) / static_cast<qreal>(height()));
    }

    QRectF imageToWidget(QRectF imageRect) const
    {
        if (m_imageSize.isEmpty()) {
            return {};
        }
        const qreal sx = static_cast<qreal>(width()) / static_cast<qreal>(m_imageSize.width());
        const qreal sy = static_cast<qreal>(height()) / static_cast<qreal>(m_imageSize.height());
        return QRectF(imageRect.left() * sx,
                      imageRect.top() * sy,
                      imageRect.width() * sx,
                      imageRect.height() * sy);
    }

    std::optional<int> tokenAt(QPointF imagePoint) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        for (int i = 0; i < tokens.size(); ++i) {
            const QRectF hitRect = tokens.at(i).imageRect.adjusted(-2.0, -2.0, 2.0, 2.0);
            if (hitRect.contains(imagePoint)) {
                return i;
            }
        }
        return std::nullopt;
    }

    std::optional<int> closestToken(QPointF imagePoint) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        if (tokens.isEmpty()) {
            return std::nullopt;
        }

        int bestIndex = 0;
        qreal bestDistance = std::numeric_limits<qreal>::max();
        for (int i = 0; i < tokens.size(); ++i) {
            const QRectF rect = tokens.at(i).imageRect;
            const qreal dx = imagePoint.x() < rect.left()
                ? rect.left() - imagePoint.x()
                : imagePoint.x() > rect.right() ? imagePoint.x() - rect.right() : 0.0;
            const qreal dy = imagePoint.y() < rect.top()
                ? rect.top() - imagePoint.y()
                : imagePoint.y() > rect.bottom() ? imagePoint.y() - rect.bottom() : 0.0;
            const qreal distance = dx * dx + dy * dy;
            if (distance < bestDistance) {
                bestDistance = distance;
                bestIndex = i;
            }
        }
        return bestIndex;
    }

    void updateCursorForPosition(QPointF widgetPoint)
    {
        if (tokenAt(widgetToImage(widgetPoint))) {
            setCursor(Qt::IBeamCursor);
        } else {
            setCursor(Qt::OpenHandCursor);
        }
    }

    bool hasTextSelection() const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        return m_selectionAnchor >= 0
            && m_selectionFocus >= 0
            && m_selectionAnchor < tokens.size()
            && m_selectionFocus < tokens.size();
    }

    std::pair<int, int> selectionRange() const
    {
        const int first = std::min(m_selectionAnchor, m_selectionFocus);
        const int last = std::max(m_selectionAnchor, m_selectionFocus);
        return {first, last};
    }

    void clearTextSelection()
    {
        if (m_selectionAnchor < 0 && m_selectionFocus < 0) {
            return;
        }
        m_selectionAnchor = -1;
        m_selectionFocus = -1;
        update();
    }

    QString selectedText() const
    {
        if (!hasTextSelection()) {
            return {};
        }

        const auto [first, last] = selectionRange();
        return tokenRangeText(first, last);
    }

    QString allText() const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        if (tokens.isEmpty()) {
            return {};
        }
        return tokenRangeText(0, tokens.size() - 1);
    }

    void copyImageText()
    {
        if (!m_config.ocrEnabled) {
            return;
        }

        if (!activeTokens().isEmpty()) {
            markshot::copyTextToClipboard(allText());
            return;
        }

        m_copyTextAfterOcr = true;
        if (!m_ocrProcess) {
            startOcr();
        }
    }

    QString tokenRangeText(int first, int last) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        QString text;
        int currentLine = -1;
        QRectF previousRect;
        QString previousText;
        for (int i = first; i <= last; ++i) {
            const OcrToken &token = tokens.at(i);
            if (currentLine != token.line) {
                if (!text.isEmpty()) {
                    text += QLatin1Char('\n');
                }
                currentLine = token.line;
            } else if (shouldInsertSpace(previousText, token.text, previousRect, token.imageRect)) {
                text += QLatin1Char(' ');
            }
            text += token.text;
            previousText = token.text;
            previousRect = token.imageRect;
        }
        return text;
    }

    const QVector<OcrToken> &activeTokens() const
    {
        return m_translationActive ? m_translatedTokens : m_ocrTokens;
    }

    bool canRequestTranslation() const
    {
        return m_config.ocrEnabled && !m_translationProcess;
    }

    void requestTranslation()
    {
        if (!canRequestTranslation()) {
            return;
        }

        if (!m_translationOverlayTokens.isEmpty()) {
            setTranslationActive(true);
            return;
        }

        if (m_ocrTokens.isEmpty()) {
            m_translateAfterOcr = true;
            if (!m_ocrProcess) {
                startOcr();
            }
            return;
        }

        m_translateAfterOcr = false;
        startTranslation(true);
    }

    void setTranslationActive(bool active)
    {
        if (active && m_translationOverlayTokens.isEmpty()) {
            return;
        }

        m_translationActive = active;
        clearTextSelection();
        updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        update();
    }

    void setTranslationBusyCursor(bool active)
    {
        if (m_translationBusyCursor == active) {
            return;
        }

        m_translationBusyCursor = active;
        if (active) {
            QApplication::setOverrideCursor(Qt::WaitCursor);
        } else {
            QApplication::restoreOverrideCursor();
            updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        }
    }

    QVector<OcrToken> selectableTranslationTokens(const QVector<OcrToken> &tokens) const
    {
        QVector<OcrToken> selectableTokens;
        int selectableIndex = 0;
        for (const OcrToken &token : tokens) {
            const QVector<OcrToken> splitTokens = splitTokenForSelection(token);
            for (OcrToken splitToken : splitTokens) {
                splitToken.index = selectableIndex++;
                selectableTokens.append(splitToken);
            }
        }
        return selectableTokens;
    }

    QVector<OcrToken> splitTokenForSelection(const OcrToken &token) const
    {
        QVector<OcrToken> splitTokens;
        if (token.text.size() <= 1) {
            splitTokens.append(token);
            return splitTokens;
        }

        qreal totalWeight = 0.0;
        QVector<qreal> weights;
        weights.reserve(token.text.size());
        for (const QChar ch : token.text) {
            const qreal weight = selectionCharacterWeight(ch);
            weights.append(weight);
            totalWeight += weight;
        }

        if (totalWeight <= 0.0 || token.imageRect.width() <= 0.0) {
            splitTokens.append(token);
            return splitTokens;
        }

        qreal offset = 0.0;
        for (int i = 0; i < token.text.size(); ++i) {
            const qreal nextOffset = offset + token.imageRect.width() * weights.at(i) / totalWeight;
            OcrToken splitToken = token;
            splitToken.text = token.text.mid(i, 1);
            splitToken.imageRect = QRectF(token.imageRect.left() + offset,
                                          token.imageRect.top(),
                                          std::max<qreal>(1.0, nextOffset - offset),
                                          token.imageRect.height());
            splitTokens.append(splitToken);
            offset = nextOffset;
        }

        return splitTokens;
    }

    qreal selectionCharacterWeight(QChar ch) const
    {
        if (ch.isSpace()) {
            return 0.45;
        }
        if (isNoLeadingSpacePunctuation(ch)) {
            return 0.65;
        }
        if (ch.unicode() < 0x80) {
            return 0.75;
        }
        return 1.0;
    }

    void drawTranslationOverlay(QPainter &painter) const
    {
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setRenderHint(QPainter::TextAntialiasing, true);

        for (const OcrToken &token : m_translationOverlayTokens) {
            const QRectF textRect = imageToWidget(token.imageRect).adjusted(-3.0, -2.0, 3.0, 2.0);
            if (textRect.isEmpty()) {
                continue;
            }

            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(255, 255, 255, 232));
            painter.drawRoundedRect(textRect, 2.0, 2.0);

            QFont font = painter.font();
            const int pixelSize = std::clamp(qRound(textRect.height() * 0.62), 8, 28);
            font.setPixelSize(pixelSize);

            QTextDocument document;
            document.setDefaultFont(font);
            document.setDocumentMargin(1.0);
            document.setTextWidth(textRect.width());
            QTextOption option;
            option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
            document.setDefaultTextOption(option);
            document.setPlainText(token.text);

            painter.save();
            painter.setClipRect(textRect);
            painter.translate(textRect.topLeft());
            document.drawContents(&painter, QRectF(QPointF(0.0, 0.0), textRect.size()));
            painter.restore();
        }

        painter.restore();
    }

    bool shouldInsertSpace(const QString &previousText, const QString &currentText, QRectF previousRect, QRectF currentRect) const
    {
        if (previousText.isEmpty() || currentText.isEmpty()) {
            return false;
        }
        const QChar currentFirst = currentText.front();
        if (isNoLeadingSpacePunctuation(currentFirst)) {
            return false;
        }
        const qreal gap = currentRect.left() - previousRect.right();
        const qreal threshold = std::max<qreal>(3.0, std::min(previousRect.height(), currentRect.height()) * 0.28);
        return gap > threshold;
    }

    bool isNoLeadingSpacePunctuation(QChar ch) const
    {
        switch (ch.unicode()) {
        case '.':
        case ',':
        case ';':
        case ':':
        case '!':
        case '?':
        case ')':
        case ']':
        case '}':
        case 0x3001:
        case 0x3002:
        case 0x300B:
        case 0x3011:
        case 0xFF01:
        case 0xFF09:
        case 0xFF0C:
        case 0xFF1A:
        case 0xFF1B:
        case 0xFF1F:
            return true;
        default:
            return false;
        }
    }

    QPixmap m_pixmap;
    QSize m_imageSize;
    qreal m_scale = 1.0;
    QPoint m_dragOffset;
    PinnedWindowConfig m_config;
    QVector<OcrToken> m_ocrTokens;
    QVector<OcrToken> m_translatedTokens;
    QVector<OcrToken> m_translationOverlayTokens;
    QProcess *m_ocrProcess = nullptr;
    QProcess *m_translationProcess = nullptr;
    QString m_ocrTempPath;
    QString m_translationInputPath;
    int m_selectionAnchor = -1;
    int m_selectionFocus = -1;
    bool m_selectingText = false;
    bool m_translationActive = false;
    bool m_translateAfterOcr = false;
    bool m_copyTextAfterOcr = false;
    bool m_translationBusyCursor = false;
    bool m_activateTranslationWhenFinished = true;
    bool m_ocrBackendWarningShown = false;
};

} // namespace

std::optional<ShotWindow::Tool> ShotWindow::toolFromName(QString name)
{
    QString key = name.trimmed().toLower();
    key.replace(QLatin1Char('_'), QLatin1Char('-'));

    if (key == QStringLiteral("move")) {
        return Tool::Move;
    }
    if (key == QStringLiteral("select") || key == QStringLiteral("selection") || key == QStringLiteral("cursor")) {
        return Tool::Select;
    }
    if (key == QStringLiteral("pen")) {
        return Tool::Pen;
    }
    if (key == QStringLiteral("line")) {
        return Tool::Line;
    }
    if (key == QStringLiteral("highlighter") || key == QStringLiteral("highlight")) {
        return Tool::Highlighter;
    }
    if (key == QStringLiteral("rectangle") || key == QStringLiteral("rect")) {
        return Tool::Rectangle;
    }
    if (key == QStringLiteral("ellipse") || key == QStringLiteral("oval") || key == QStringLiteral("circle")) {
        return Tool::Ellipse;
    }
    if (key == QStringLiteral("arrow")) {
        return Tool::Arrow;
    }
    if (key == QStringLiteral("text")) {
        return Tool::Text;
    }
    if (key == QStringLiteral("number") || key == QStringLiteral("counter")) {
        return Tool::Number;
    }
    if (key == QStringLiteral("magnifier") || key == QStringLiteral("magnify")
        || key == QStringLiteral("loupe") || key == QStringLiteral("zoom")) {
        return Tool::Magnifier;
    }
    if (key == QStringLiteral("mosaic") || key == QStringLiteral("blur")) {
        return Tool::Mosaic;
    }
    if (key == QStringLiteral("laser")) {
        return Tool::Laser;
    }
    return std::nullopt;
}

QStringList ShotWindow::supportedToolNames()
{
    return {
        QStringLiteral("move"),
        QStringLiteral("select"),
        QStringLiteral("pen"),
        QStringLiteral("line"),
        QStringLiteral("highlighter"),
        QStringLiteral("rectangle"),
        QStringLiteral("ellipse"),
        QStringLiteral("arrow"),
        QStringLiteral("text"),
        QStringLiteral("number"),
        QStringLiteral("magnifier"),
        QStringLiteral("mosaic"),
        QStringLiteral("laser"),
    };
}

ShotWindow::ShotWindow(QImage frozenFrame,
                       QString outputName,
                       QRect sourceGeometry,
                       QVector<QRect> windowGeometries,
                       bool windowDetectionEnabled,
                       QWidget *parent)
    : QWidget(parent)
    , m_frozenFrame(std::move(frozenFrame))
    , m_outputName(std::move(outputName))
    , m_sourceGeometry(sourceGeometry)
{
    const ShortcutConfig shortcutConfig = configuredShortcuts();
    m_actionShortcuts = shortcutConfig.actions;
    m_toolShortcuts = shortcutConfig.tools;
    m_startupColorPickerShortcut = shortcutConfig.startupColorPicker;
    m_startupRulerShortcut = shortcutConfig.startupRuler;

    setWindowTitle(MS_TR("Mark Shot"));
    setCursor(Qt::CrossCursor);
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
    markshot::windows::setExcludedFromCapture(this);
    if (m_frozenFrame.format() != QImage::Format_ARGB32_Premultiplied) {
        m_frozenFrame = m_frozenFrame.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    }
    // Annotation and selection geometry are stored in image pixels. Clear any
    // high-DPI metadata from screen grabs so Qt painting does not apply a second
    // device-pixel-ratio scale on top of our explicit image-to-widget mapping.
    m_frozenFrame.setDevicePixelRatio(1.0);

    m_toolbar = new QWidget(this);
    m_toolbar->setObjectName(QStringLiteral("shotToolbar"));
    m_toolbar->setStyleSheet(markshot::theme::panelStyleSheet());
    m_toolbar->installEventFilter(this);

    m_toolbarLayout = new QHBoxLayout(m_toolbar);
    m_toolbarLayout->setContentsMargins(6, 6, 6, 6);
    m_toolbarLayout->setSpacing(3);

    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMove, shortcutText(Tool::Move)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolSelect, shortcutText(Tool::Select)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolPen, shortcutText(Tool::Pen)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolLine, shortcutText(Tool::Line)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolHighlighter, shortcutText(Tool::Highlighter)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolRectangle, shortcutText(Tool::Rectangle)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolEllipse, shortcutText(Tool::Ellipse)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolArrow, shortcutText(Tool::Arrow)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolText, shortcutText(Tool::Text)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolNumber, shortcutText(Tool::Number)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMosaic, shortcutText(Tool::Mosaic)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMagnifier, shortcutText(Tool::Magnifier)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolLaser, shortcutText(Tool::Laser)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Clear, shortcutText(Action::Clear, QStringLiteral("Clear"))));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Undo, shortcutText(Action::Undo)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Redo, shortcutText(Action::Redo, QStringLiteral("Ctrl+Shift+Z"))));
    for (Action action : {Action::ToggleCaptureScope,
                          Action::ToggleToolbarLayout,
                          Action::OpenWith,
                          Action::Extensions,
                          Action::ScrollCapture,
                          Action::Pin,
                          Action::OcrCopy,
                          Action::Copy,
                          Action::Save,
                          Action::Cancel}) {
        const QString shortcut = action == Action::OpenWith ? shortcutText(action, QStringLiteral("Open"))
            : action == Action::Extensions           ? shortcutText(action, QStringLiteral("Ext"))
            : action == Action::ScrollCapture        ? shortcutText(action, QStringLiteral("Scroll"))
            : action == Action::Pin                  ? shortcutText(action)
            : action == Action::OcrCopy              ? shortcutText(action, QStringLiteral("OCR"))
            : action == Action::Copy                 ? shortcutText(action)
            : action == Action::Save                 ? shortcutText(action, QStringLiteral("Save As"))
            : action == Action::ToggleToolbarLayout  ? shortcutText(action, QStringLiteral("Layout"))
            : action == Action::ToggleCaptureScope   ? shortcutText(action)
                                                     : shortcutText(action);
        QPushButton *button = addToolbarButton(action, shortcut);
        button->hide();
        m_fullscreenActionButtons.append(button);
        m_toolbarLayout->addWidget(button);
    }
    m_toolbar->hide();

    m_horizontalImageScrollBar = new QScrollBar(Qt::Horizontal, this);
    m_horizontalImageScrollBar->setFocusPolicy(Qt::NoFocus);
    m_horizontalImageScrollBar->hide();
    m_verticalImageScrollBar = new QScrollBar(Qt::Vertical, this);
    m_verticalImageScrollBar->setFocusPolicy(Qt::NoFocus);
    m_verticalImageScrollBar->hide();
    const QString imageScrollBarStyle = QStringLiteral(
        "QScrollBar { background: rgba(8,13,19,190); border: 0; }"
        "QScrollBar:horizontal { height: 14px; }"
        "QScrollBar:vertical { width: 14px; }"
        "QScrollBar::handle { background: rgba(45,212,191,180); border-radius: 6px; min-width: 28px; min-height: 28px; }"
        "QScrollBar::handle:hover { background: rgba(94,234,212,220); }"
        "QScrollBar::add-line, QScrollBar::sub-line { width: 0; height: 0; }"
        "QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }");
    m_horizontalImageScrollBar->setStyleSheet(imageScrollBarStyle);
    m_verticalImageScrollBar->setStyleSheet(imageScrollBarStyle);
    connect(m_horizontalImageScrollBar, &QScrollBar::valueChanged, this, [this] {
        setImageCenterFromScrollBars();
    });
    connect(m_verticalImageScrollBar, &QScrollBar::valueChanged, this, [this] {
        setImageCenterFromScrollBars();
    });

    m_actionToolbar = new QWidget(this);
    m_actionToolbar->setObjectName(QStringLiteral("actionToolbar"));
    m_actionToolbar->setStyleSheet(m_toolbar->styleSheet()
        + QStringLiteral(
              "QWidget#actionToolbar QPushButton {"
              " border-radius: 6px;"
              " min-width: 28px;"
              " min-height: 28px;"
              " max-width: 28px;"
              " max-height: 28px;"
              "}"));
    auto *actionLayout = new QVBoxLayout(m_actionToolbar);
    actionLayout->setContentsMargins(4, 4, 4, 4);
    actionLayout->setSpacing(2);
    for (QPushButton *button : {
             addToolbarButton(Action::ToggleCaptureScope, shortcutText(Action::ToggleCaptureScope), m_actionToolbar),
             addToolbarButton(Action::OpenWith, shortcutText(Action::OpenWith, QStringLiteral("Open")), m_actionToolbar),
             addToolbarButton(Action::Extensions, shortcutText(Action::Extensions, QStringLiteral("Ext")), m_actionToolbar),
             addToolbarButton(Action::ScrollCapture, shortcutText(Action::ScrollCapture, QStringLiteral("Scroll")), m_actionToolbar),
             addToolbarButton(Action::Pin, shortcutText(Action::Pin), m_actionToolbar),
             addToolbarButton(Action::OcrCopy, shortcutText(Action::OcrCopy, QStringLiteral("OCR")), m_actionToolbar),
             addToolbarButton(Action::Copy, shortcutText(Action::Copy), m_actionToolbar),
             addToolbarButton(Action::Save, shortcutText(Action::Save, QStringLiteral("Save As")), m_actionToolbar),
             addToolbarButton(Action::Cancel, shortcutText(Action::Cancel), m_actionToolbar),
         }) {
        button->setIconSize(QSize(20, 20));
        actionLayout->addWidget(button);
    }
    m_actionToolbar->hide();

    auto shortcutBlockedByTextInput = [this] {
        if (m_textEditor && m_textEditor->isVisible()) {
            return true;
        }
        QWidget *focusWidget = QApplication::focusWidget();
        return qobject_cast<QLineEdit *>(focusWidget) != nullptr
            || qobject_cast<QTextEdit *>(focusWidget) != nullptr;
    };
    auto addPlainShortcut = [this, shortcutBlockedByTextInput](const QKeySequence &sequence, auto callback) {
        if (sequence.isEmpty()) {
            return;
        }
        auto *shortcut = new QShortcut(sequence, this);
        shortcut->setContext(Qt::WindowShortcut);
        shortcut->setAutoRepeat(false);
        connect(shortcut, &QShortcut::activated, this, [this, shortcutBlockedByTextInput, callback] {
            if (shortcutBlockedByTextInput()) {
                return;
            }
            callback();
        });
    };
    auto addToolShortcut = [this, addPlainShortcut](Tool tool) {
        const QKeySequence sequence = this->shortcutForTool(tool);
        addPlainShortcut(sequence, [this, tool, sequence] {
            if (m_mode == Mode::Selecting && sequence == m_startupColorPickerShortcut) {
                setStartupTool(StartupTool::ColorPicker);
                return;
            }
            if (m_mode == Mode::Selecting && sequence == m_startupRulerShortcut) {
                setStartupTool(StartupTool::Ruler);
                return;
            }
            setTool(tool);
        });
    };
    for (Tool tool : {Tool::Move,
                      Tool::Select,
                      Tool::Pen,
                      Tool::Line,
                      Tool::Highlighter,
                      Tool::Rectangle,
                      Tool::Ellipse,
                      Tool::Arrow,
                      Tool::Text,
                      Tool::Number,
                      Tool::Mosaic,
                      Tool::Magnifier,
                      Tool::Laser}) {
        addToolShortcut(tool);
    }
    auto sequenceUsedByTool = [this](const QKeySequence &sequence) {
        for (const QKeySequence &toolSequence : m_toolShortcuts) {
            if (!sequence.isEmpty() && sequence == toolSequence) {
                return true;
            }
        }
        return false;
    };
    if (!sequenceUsedByTool(m_startupColorPickerShortcut)) {
        addPlainShortcut(m_startupColorPickerShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                setStartupTool(StartupTool::ColorPicker);
            }
        });
    }
    if (!sequenceUsedByTool(m_startupRulerShortcut)) {
        addPlainShortcut(m_startupRulerShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                setStartupTool(StartupTool::Ruler);
            }
        });
    }
    auto addActionShortcut = [this, addPlainShortcut](Action action, auto callback) {
        addPlainShortcut(this->shortcutForAction(action), callback);
    };
    addActionShortcut(Action::ToggleCaptureScope, [this] { toggleCaptureScope(); });
    addActionShortcut(Action::Pin, [this] { pinSelection(); });
    addActionShortcut(Action::Copy, [this] {
        commitTextEditor();
        copySelection();
    });
    addActionShortcut(Action::Save, [this] {
        commitTextEditor();
        saveSelection();
    });
    addActionShortcut(Action::Undo, [this] { undoAnnotationEdit(); });
    addActionShortcut(Action::Redo, [this] { redoAnnotation(); });
    addActionShortcut(Action::OpenWith, [this] { toggleOpenWithPanel(); });
    addActionShortcut(Action::Extensions, [this] { toggleExtensionPanel(); });
    addActionShortcut(Action::ScrollCapture, [this] { startScrollCapture(); });
    addActionShortcut(Action::OcrCopy, [this] { ocrCopySelection(); });
    addActionShortcut(Action::Clear, [this] { clearAnnotations(); });
    addActionShortcut(Action::ToggleToolbarLayout, [this] { toggleToolbarLayout(); });
    addActionShortcut(Action::Cancel, [this] {
        if (m_mode == Mode::Selecting && m_startupTool != StartupTool::None) {
            leaveStartupTool();
            return;
        }
        emit sessionCancelRequested();
        close();
    });

    m_annotationPropertyPanel = new QWidget(this);
    m_annotationPropertyPanel->setObjectName(QStringLiteral("annotationPropertyPanel"));
    m_annotationPropertyPanel->setStyleSheet(m_toolbar->styleSheet());
    auto *propertyLayout = new QHBoxLayout(m_annotationPropertyPanel);
    propertyLayout->setContentsMargins(8, 6, 8, 6);
    propertyLayout->setSpacing(6);
    auto addPropertyGlyph = [this, propertyLayout](markshot::ui::PropertyIcon icon, const QString &tooltip) {
        auto *label = new QLabel(m_annotationPropertyPanel);
        label->setObjectName(QStringLiteral("propertyGlyph"));
        label->setAlignment(Qt::AlignCenter);
        label->setPixmap(markshot::ui::makePropertyIcon(icon).pixmap(QSize(18, 18)));
        label->setToolTip(tooltip);
        propertyLayout->addWidget(label);
        return label;
    };
    auto configurePropertyValueLabel = [](QLabel *label, int width, const QString &tooltip) {
        label->setObjectName(QStringLiteral("propertyValue"));
        label->setAlignment(Qt::AlignCenter);
        label->setFixedWidth(width);
        label->setToolTip(tooltip);
    };

    m_annotationPropertyTitle = new QLabel(QStringLiteral("Object"), m_annotationPropertyPanel);
    m_annotationPropertyTitle->setObjectName(QStringLiteral("propertyTitle"));
    m_annotationPropertyTitle->setAlignment(Qt::AlignCenter);
    m_annotationPropertyTitle->setMinimumWidth(58);
    m_annotationPropertyTitle->setToolTip(MS_TR("Selected object type"));
    propertyLayout->addWidget(m_annotationPropertyTitle);
    propertyLayout->addSpacing(2);
    addPropertyGlyph(markshot::ui::PropertyIcon::StrokeWidth, MS_TR("Selected object width or size"));
    m_propertyWidthLabel = new QLabel(QStringLiteral("2"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyWidthLabel, 34, MS_TR("Selected object width or size"));
    propertyLayout->addWidget(m_propertyWidthLabel);
    m_propertyWidthSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyWidthSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyWidthSlider->setFixedWidth(88);
    m_propertyWidthSlider->setToolTip(MS_TR("Selected object width or size"));
    connect(m_propertyWidthSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationWidth(value); });
    propertyLayout->addWidget(m_propertyWidthSlider);
    propertyLayout->addSpacing(2);
    addPropertyGlyph(markshot::ui::PropertyIcon::Opacity, MS_TR("Selected object opacity"));
    m_propertyOpacityLabel = new QLabel(QStringLiteral("100%"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyOpacityLabel, 36, MS_TR("Selected object opacity"));
    propertyLayout->addWidget(m_propertyOpacityLabel);
    m_propertyOpacitySlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyOpacitySlider->setFocusPolicy(Qt::NoFocus);
    m_propertyOpacitySlider->setRange(0, 100);
    m_propertyOpacitySlider->setFixedWidth(76);
    m_propertyOpacitySlider->setToolTip(MS_TR("Selected object opacity"));
    connect(m_propertyOpacitySlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationOpacity(value); });
    propertyLayout->addWidget(m_propertyOpacitySlider);
    propertyLayout->addSpacing(2);
    m_propertyColorButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyColorButton->setFocusPolicy(Qt::NoFocus);
    m_propertyColorButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Color));
    m_propertyColorButton->setIconSize(QSize(18, 18));
    m_propertyColorButton->setToolTip(MS_TR("Change selected object color"));
    m_propertyColorButton->setAccessibleName(MS_TR("Change selected object color"));
    connect(m_propertyColorButton, &QPushButton::clicked, this, [this] { openSelectedAnnotationColorPalette(); });
    propertyLayout->addWidget(m_propertyColorButton);
    m_propertyTextBackgroundButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyTextBackgroundButton->setFocusPolicy(Qt::NoFocus);
    m_propertyTextBackgroundButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::TextBackground));
    m_propertyTextBackgroundButton->setIconSize(QSize(18, 18));
    m_propertyTextBackgroundButton->setToolTip(MS_TR("Text background color"));
    m_propertyTextBackgroundButton->setAccessibleName(MS_TR("Text background color"));
    connect(m_propertyTextBackgroundButton, &QPushButton::clicked, this, [this] { openSelectedTextBackgroundColorPalette(); });
    propertyLayout->addWidget(m_propertyTextBackgroundButton);
    m_propertyFillButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyFillButton->setCheckable(true);
    m_propertyFillButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(false));
    m_propertyFillButton->setIconSize(QSize(20, 20));
    m_propertyFillButton->setToolTip(MS_TR("Toggle shape fill"));
    m_propertyFillButton->setAccessibleName(MS_TR("Toggle shape fill"));
    connect(m_propertyFillButton, &QPushButton::toggled, this, [this](bool checked) {
        m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(checked));
        setSelectedAnnotationFilled(checked);
    });
    propertyLayout->addWidget(m_propertyFillButton);
    m_propertyRadiusGlyphLabel = addPropertyGlyph(markshot::ui::PropertyIcon::CornerRadius, MS_TR("Rectangle corner radius"));
    m_propertyRadiusLabel = new QLabel(QStringLiteral("0"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyRadiusLabel, 24, MS_TR("Rectangle corner radius"));
    propertyLayout->addWidget(m_propertyRadiusLabel);
    m_propertyRadiusSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyRadiusSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyRadiusSlider->setRange(0, 48);
    m_propertyRadiusSlider->setFixedWidth(72);
    m_propertyRadiusSlider->setToolTip(MS_TR("Rectangle corner radius"));
    connect(m_propertyRadiusSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationCornerRadius(value); });
    propertyLayout->addWidget(m_propertyRadiusSlider);
    m_propertyArrowStyleCombo = new QComboBox(m_annotationPropertyPanel);
    m_propertyArrowStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_propertyArrowStyleCombo->addItem(MS_TR("Fletched"), static_cast<int>(ArrowStyle::Fletched));
    m_propertyArrowStyleCombo->addItem(MS_TR("KDE"), static_cast<int>(ArrowStyle::Kde));
    m_propertyArrowStyleCombo->setToolTip(MS_TR("Arrow style"));
    m_propertyArrowStyleCombo->setAccessibleName(MS_TR("Arrow style"));
    connect(m_propertyArrowStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_propertyArrowStyleCombo) {
            return;
        }
        setSelectedAnnotationArrowStyle(static_cast<ArrowStyle>(m_propertyArrowStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_propertyArrowStyleCombo);
    m_propertyHighlighterStyleCombo = new QComboBox(m_annotationPropertyPanel);
    m_propertyHighlighterStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_propertyHighlighterStyleCombo->addItem(MS_TR("Pen"), static_cast<int>(HighlighterStyle::Freehand));
    m_propertyHighlighterStyleCombo->addItem(MS_TR("Line"), static_cast<int>(HighlighterStyle::StraightLine));
    m_propertyHighlighterStyleCombo->setToolTip(MS_TR("Highlighter style"));
    m_propertyHighlighterStyleCombo->setAccessibleName(MS_TR("Highlighter style"));
    connect(m_propertyHighlighterStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_propertyHighlighterStyleCombo) {
            return;
        }
        setSelectedHighlighterStyle(
            static_cast<HighlighterStyle>(m_propertyHighlighterStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_propertyHighlighterStyleCombo);
    m_propertyMagnifierScaleGlyphLabel =
        addPropertyGlyph(markshot::ui::PropertyIcon::MagnifierScale, MS_TR("Magnifier scale"));
    m_propertyMagnifierScaleLabel = new QLabel(magnifierScaleText(kDefaultMagnifierScale), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyMagnifierScaleLabel, 48, MS_TR("Magnifier scale"));
    propertyLayout->addWidget(m_propertyMagnifierScaleLabel);
    m_propertyMagnifierScaleSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyMagnifierScaleSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyMagnifierScaleSlider->setRange(magnifierScaleSliderValue(kMinMagnifierScale),
                                             magnifierScaleSliderValue(kMaxMagnifierScale));
    m_propertyMagnifierScaleSlider->setFixedWidth(84);
    m_propertyMagnifierScaleSlider->setToolTip(MS_TR("Magnifier scale"));
    connect(m_propertyMagnifierScaleSlider, &QSlider::valueChanged, this, [this](int value) {
        setSelectedMagnifierScale(value);
    });
    propertyLayout->addWidget(m_propertyMagnifierScaleSlider);
    m_propertyFontButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyFontButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFontButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Font));
    m_propertyFontButton->setIconSize(QSize(20, 20));
    m_propertyFontButton->setToolTip(MS_TR("Text font"));
    m_propertyFontButton->setAccessibleName(MS_TR("Text font"));
    connect(m_propertyFontButton, &QPushButton::clicked, this, [this] { toggleSelectedTextFontPanel(); });
    propertyLayout->addWidget(m_propertyFontButton);
    m_propertyEditTextButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyEditTextButton->setFocusPolicy(Qt::NoFocus);
    m_propertyEditTextButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::EditText));
    m_propertyEditTextButton->setIconSize(QSize(20, 20));
    m_propertyEditTextButton->setToolTip(MS_TR("Edit selected text"));
    m_propertyEditTextButton->setAccessibleName(MS_TR("Edit selected text"));
    connect(m_propertyEditTextButton, &QPushButton::clicked, this, [this] { beginEditingSelectedTextAnnotation(); });
    propertyLayout->addWidget(m_propertyEditTextButton);
    m_annotationPropertyPanel->hide();

    m_propertyColorDialogPanel = new QWidget(this);
    m_propertyColorDialogPanel->setObjectName(QStringLiteral("propertyColorDialogPanel"));
    m_propertyColorDialogPanel->setStyleSheet(markshot::theme::propertyColorDialogPanelStyleSheet());
    auto *propertyColorLayout = new QVBoxLayout(m_propertyColorDialogPanel);
    propertyColorLayout->setContentsMargins(8, 8, 8, 8);
    propertyColorLayout->setSpacing(0);
    m_propertyColorPicker = new markshot::ui::ColorPicker(m_propertyColorDialogPanel);
    m_propertyColorPicker->setColor(m_currentColor);
    connect(m_propertyColorPicker, &markshot::ui::ColorPicker::colorChanged, this,
            [this](const QColor &color) { applyPropertyColor(color); });
    propertyColorLayout->addWidget(m_propertyColorPicker);
    m_propertyColorDialogPanel->hide();

    m_propertyFontPanel = new QWidget(this);
    m_propertyFontPanel->setObjectName(QStringLiteral("propertyFontPanel"));
    m_propertyFontPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *fontPanelLayout = new QVBoxLayout(m_propertyFontPanel);
    fontPanelLayout->setContentsMargins(6, 6, 6, 6);
    fontPanelLayout->setSpacing(0);
    m_propertyFontList = new QListWidget(m_propertyFontPanel);
    m_propertyFontList->setFocusPolicy(Qt::NoFocus);
    m_propertyFontList->setUniformItemSizes(true);
    m_propertyFontList->setMinimumHeight(84);
    m_propertyFontList->setMaximumHeight(260);
    m_propertyFontList->setVerticalScrollBarPolicy(Qt::ScrollBarAsNeeded);
    m_propertyFontList->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    for (const QString &family : QFontDatabase::families()) {
        auto *item = new QListWidgetItem(family, m_propertyFontList);
        item->setData(Qt::UserRole, family);
        item->setFont(QFont(family, 12));
    }
    connect(m_propertyFontList, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }
        setSelectedTextFontFamily(item->data(Qt::UserRole).toString());
        if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
        }
    });
    fontPanelLayout->addWidget(m_propertyFontList);
    m_propertyFontPanel->hide();

    m_openWithPanel = new QWidget(this);
    m_openWithPanel->setObjectName(QStringLiteral("openWithPanel"));
    m_openWithPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *openLayout = new QVBoxLayout(m_openWithPanel);
    openLayout->setContentsMargins(8, 8, 8, 8);
    openLayout->setSpacing(4);
    m_openWithPanel->hide();

    m_extensionPanel = new QWidget(this);
    m_extensionPanel->setObjectName(QStringLiteral("extensionPanel"));
    m_extensionPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *extensionLayout = new QVBoxLayout(m_extensionPanel);
    extensionLayout->setContentsMargins(8, 8, 8, 8);
    extensionLayout->setSpacing(4);
    m_extensionPanel->hide();

    m_colorPalette = new QWidget(this);
    m_colorPalette->setObjectName(QStringLiteral("colorPalette"));
    m_colorPalette->setStyleSheet(markshot::theme::colorPaletteStyleSheet());
    for (const QColor &color : markshot::theme::paletteColors()) {
        auto *button = new QPushButton(m_colorPalette);
        button->setFocusPolicy(Qt::NoFocus);
        button->setStyleSheet(QStringLiteral("background: %1;").arg(color.name()));
        connect(button, &QPushButton::clicked, this, [this, color] { setCurrentColor(color); });
    }
    m_colorPalettePreview = new QWidget(m_colorPalette);
    m_colorPalettePreview->setObjectName(QStringLiteral("colorPalettePreview"));
    m_colorPalette->installEventFilter(this);
    m_colorPalette->hide();
    updateColorPalettePreview();

    m_textEditor = new QTextEdit(this);
    m_textEditor->setObjectName(QStringLiteral("textEditor"));
    m_textEditor->setPlaceholderText(MS_TR("Type text"));
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(QColor(94, 234, 212), QColor(0, 0, 0, 0), 24));
    m_textEditor->setAcceptRichText(false);
    m_textEditor->setTabChangesFocus(false);
    m_textEditor->setFrameShape(QFrame::NoFrame);
    m_textEditor->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->viewport()->setAutoFillBackground(false);
    m_textEditor->setToolTip(MS_TR("Enter inserts newline, click outside commits, Esc cancels"));
    m_textEditor->hide();
    m_textEditor->installEventFilter(this);

    m_laserClock.start();
    m_laserTimer = new QTimer(this);
    m_laserTimer->setInterval(33);
    connect(m_laserTimer, &QTimer::timeout, this, [this] { cleanupLaserStrokes(); });

    if (windowDetectionEnabled) {
        if (windowGeometries.isEmpty()) {
#if defined(Q_OS_WIN)
            windowGeometries = markshot::windows::enumerateWindowGeometries();
#else
            windowGeometries = enumerateX11WindowGeometries();
#endif
        }
        for (const QRect &windowGeometry : std::as_const(windowGeometries)) {
            const QRect imageRect = windowGeometryToImageRect(windowGeometry,
                                                              m_sourceGeometry,
                                                              m_frozenFrame.size());
            if (imageRect.width() > 1 && imageRect.height() > 1) {
                m_windowRects.append(imageRect);
            }
        }
    }
}

bool ShotWindow::configureLayerShell(QScreen *screen)
{
    const QSize desiredSize = m_sourceGeometry.isValid() && !m_sourceGeometry.isEmpty()
        ? m_sourceGeometry.size()
        : m_frozenFrame.size();
    if (!desiredSize.isEmpty()) {
        resize(desiredSize);
    }

    if (screen) {
        setScreen(screen);
    }

    return markshot::layershell::configureOverlay(
        this,
        screen,
        {QStringLiteral("mark-shot"),
         markshot::layershell::KeyboardInteractivity::Exclusive,
         true,
         true});
}

void ShotWindow::startFullscreenAnnotation()
{
    enterFullscreenAnnotation(true);
}

void ShotWindow::setImageNavigationEnabled(bool enabled)
{
    m_imageNavigationEnabled = enabled;
    if (!enabled) {
        m_imageZoom = 1.0;
        m_imageCenterInitialized = false;
        m_imageSelected = false;
        m_imagePanning = false;
    }
    updateMinimumImageWindowSize();
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::setDefaultTool(Tool tool)
{
    setDefaultTools(tool, tool);
}

void ShotWindow::setDefaultTools(Tool tool, Tool fullscreenTool)
{
    m_defaultTool = tool;
    m_fullscreenDefaultTool = fullscreenTool;
}

void ShotWindow::setDefaultColor(QColor color)
{
    setCurrentColor(color);
}

QKeySequence ShotWindow::shortcutForAction(Action action) const
{
    return m_actionShortcuts.at(actionIndex(action));
}

QKeySequence ShotWindow::shortcutForTool(Tool tool) const
{
    return m_toolShortcuts.at(toolIndex(tool));
}

QString ShotWindow::shortcutText(Action action, const QString &fallback) const
{
    const QKeySequence sequence = shortcutForAction(action);
    if (sequence.isEmpty()) {
        return fallback;
    }
    return sequence.toString(QKeySequence::NativeText);
}

QString ShotWindow::shortcutText(Tool tool) const
{
    const QKeySequence sequence = shortcutForTool(tool);
    return sequence.isEmpty() ? QString() : sequence.toString(QKeySequence::NativeText);
}

bool shortcutMatchesEvent(const QKeySequence &sequence, const QKeyEvent *event)
{
    if (!event || sequence.isEmpty() || event->key() == Qt::Key_unknown) {
        return false;
    }
    return QKeySequence(event->keyCombination()).matches(sequence) == QKeySequence::ExactMatch;
}

bool ShotWindow::eventMatchesShortcut(const QKeyEvent *event, Action action) const
{
    return shortcutMatchesEvent(shortcutForAction(action), event);
}

bool ShotWindow::eventMatchesShortcut(const QKeyEvent *event, Tool tool) const
{
    return shortcutMatchesEvent(shortcutForTool(tool), event);
}

bool ShotWindow::eventMatchesStartupShortcut(const QKeyEvent *event, StartupTool tool) const
{
    if (tool == StartupTool::ColorPicker) {
        return shortcutMatchesEvent(m_startupColorPickerShortcut, event);
    }
    if (tool == StartupTool::Ruler) {
        return shortcutMatchesEvent(m_startupRulerShortcut, event);
    }
    return false;
}

bool ShotWindow::handleConfiguredActionShortcut(QKeyEvent *event)
{
    if (eventMatchesShortcut(event, Action::Cancel)) {
        emit sessionCancelRequested();
        close();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Copy)) {
        commitTextEditor();
        copySelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Save)) {
        commitTextEditor();
        saveSelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Pin)) {
        pinSelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Undo)) {
        undoAnnotationEdit();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Redo) || event->matches(QKeySequence::Redo)) {
        redoAnnotation();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ToggleCaptureScope)) {
        toggleCaptureScope();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ToggleToolbarLayout)) {
        toggleToolbarLayout();
        return true;
    }
    if (eventMatchesShortcut(event, Action::OpenWith)) {
        toggleOpenWithPanel();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Extensions)) {
        toggleExtensionPanel();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ScrollCapture)) {
        startScrollCapture();
        return true;
    }
    if (eventMatchesShortcut(event, Action::OcrCopy)) {
        ocrCopySelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Clear)) {
        clearAnnotations();
        return true;
    }
    return false;
}

bool ShotWindow::handleConfiguredToolShortcut(QKeyEvent *event)
{
    const std::array<Tool, static_cast<int>(Tool::Laser) + 1> tools = {
        Tool::Move,
        Tool::Select,
        Tool::Pen,
        Tool::Line,
        Tool::Highlighter,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Arrow,
        Tool::Text,
        Tool::Number,
        Tool::Mosaic,
        Tool::Magnifier,
        Tool::Laser,
    };
    for (Tool tool : tools) {
        if (eventMatchesShortcut(event, tool)) {
            setTool(tool);
            return true;
        }
    }
    return false;
}

void ShotWindow::enterFullscreenAnnotation(bool resetAnnotations)
{
    commitTextEditor();
    emit selectionActivated(this);
    if (m_colorPalette) {
        m_colorPalette->hide();
    }

    if (!m_fullscreenAnnotation && hasUsableSelection()) {
        m_selectionBeforeFullscreenAnnotation = normalizedSelection();
    }
    m_mode = Mode::Editing;
    m_dragging = false;
    m_fullscreenAnnotation = true;
    applyToolbarLayout();
    updateMinimumImageWindowSize();
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_selection = QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size()));
    if (resetAnnotations) {
        m_annotations.clear();
        m_undoStack.clear();
        m_redoStack.clear();
        m_laserStrokes.clear();
        m_laserDraft.reset();
    }
    m_draft.reset();
    setSelectedAnnotations({});
    if (resetAnnotations) {
        m_nextNumber = 1;
        m_nextAnnotationId = 1;
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    setTool(defaultEditingTool());
    if (m_toolbar) {
        setFullscreenActionButtonsVisible(true);
        m_toolbar->show();
    }
    updateMinimumImageWindowSize();
    if (m_actionToolbar) {
        m_actionToolbar->hide();
    }
    updateToolbarGeometry();
    updateToolbarState();
    update();
}

void ShotWindow::leaveFullscreenAnnotation()
{
    commitTextEditor();
    m_dragging = false;
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_fullscreenAnnotation = false;
    m_toolbarVerticalLayout = false;
    applyToolbarLayout();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_draft.reset();
    m_laserDraft.reset();

    if (m_selectionBeforeFullscreenAnnotation.has_value()) {
        m_selection = *m_selectionBeforeFullscreenAnnotation;
    } else {
        resetImageZoom();
        m_mode = Mode::Selecting;
        m_selection = {};
        if (m_toolbar) {
            m_toolbar->hide();
        }
        if (m_actionToolbar) {
            m_actionToolbar->hide();
        }
        setFullscreenActionButtonsVisible(false);
        updateToolbarState();
        update();
        return;
    }

    m_mode = Mode::Editing;
    setFullscreenActionButtonsVisible(false);
    if (m_toolbar) {
        m_toolbar->show();
    }
    if (m_actionToolbar) {
        m_actionToolbar->show();
    }
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateToolbarState();
    update();
}

void ShotWindow::toggleCaptureScope()
{
    resetImageZoom();
    if (m_fullscreenAnnotation) {
        leaveFullscreenAnnotation();
    } else {
        enterFullscreenAnnotation(false);
    }
}

void ShotWindow::toggleToolbarLayout()
{
    m_toolbarVerticalLayout = !m_toolbarVerticalLayout;
    m_toolbarUserPlaced = false;
    applyToolbarLayout();
    updateToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateToolbarState();
}

void ShotWindow::applyToolbarLayout()
{
    if (!m_toolbarLayout) {
        return;
    }

    m_toolbarLayout->setDirection(m_toolbarVerticalLayout ? QBoxLayout::TopToBottom : QBoxLayout::LeftToRight);
    m_toolbar->setFixedSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX);
    m_toolbar->adjustSize();
}

QPushButton *ShotWindow::addToolbarButton(Action action, const QString &shortcutText, QWidget *parentToolbar)
{
    QWidget *toolbar = parentToolbar ? parentToolbar : m_toolbar;
    auto *button = new QPushButton(toolbar);
    button->setIcon(markshot::ui::makeToolIcon(action));
    button->setIconSize(QSize(22, 22));
    button->setFocusPolicy(Qt::NoFocus);
    button->setToolTip(QStringLiteral("%1 (%2)").arg(markshot::i18n::translate(markshot::ui::actionName(action)), shortcutText));
    button->setProperty("action", markshot::ui::actionName(action));
    if (action == Action::ScrollCapture && isGnomeWaylandSession() && !hasGnomeScrollHelper()) {
        button->setEnabled(false);
        button->setToolTip(MS_TR("Scroll capture is not supported on GNOME Wayland."));
    }
    if (!parentToolbar && action == Action::ToolMove) {
        button->installEventFilter(this);
    }
    if (action == Action::Save) {
        button->setProperty("role", QStringLiteral("primary"));
    } else if (action == Action::Cancel) {
        button->setProperty("role", QStringLiteral("danger"));
    } else if (action == Action::OpenWith || action == Action::Extensions || action == Action::Pin || action == Action::OcrCopy || action == Action::Copy || action == Action::ScrollCapture) {
        button->setProperty("role", QStringLiteral("secondary"));
    }

    if (action == Action::ToolMove) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Move); });
    } else if (action == Action::ToolSelect) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Select); });
    } else if (action == Action::ToolPen) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Pen); });
    } else if (action == Action::ToolLine) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Line); });
    } else if (action == Action::ToolHighlighter) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Highlighter); });
    } else if (action == Action::ToolRectangle) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Rectangle); });
    } else if (action == Action::ToolEllipse) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Ellipse); });
    } else if (action == Action::ToolArrow) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Arrow); });
    } else if (action == Action::ToolText) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Text); });
    } else if (action == Action::ToolNumber) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Number); });
    } else if (action == Action::ToolMosaic) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Mosaic); });
    } else if (action == Action::ToolMagnifier) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Magnifier); });
    } else if (action == Action::ToolLaser) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Laser); });
    } else if (action == Action::ToggleCaptureScope) {
        connect(button, &QPushButton::clicked, this, [this] { toggleCaptureScope(); });
    } else if (action == Action::ToggleToolbarLayout) {
        connect(button, &QPushButton::clicked, this, [this] { toggleToolbarLayout(); });
    } else if (action == Action::Clear) {
        connect(button, &QPushButton::clicked, this, [this] { clearAnnotations(); });
    } else if (action == Action::Undo) {
        connect(button, &QPushButton::clicked, this, [this] { undoAnnotationEdit(); });
    } else if (action == Action::Redo) {
        connect(button, &QPushButton::clicked, this, [this] { redoAnnotation(); });
    } else if (action == Action::OpenWith) {
        connect(button, &QPushButton::clicked, this, [this] { toggleOpenWithPanel(); });
    } else if (action == Action::Extensions) {
        connect(button, &QPushButton::clicked, this, [this] { toggleExtensionPanel(); });
    } else if (action == Action::Pin) {
        connect(button, &QPushButton::clicked, this, [this] { pinSelection(); });
    } else if (action == Action::ScrollCapture) {
        connect(button, &QPushButton::clicked, this, [this] { startScrollCapture(); });
    } else if (action == Action::OcrCopy) {
        connect(button, &QPushButton::clicked, this, [this] { ocrCopySelection(); });
    } else if (action == Action::Copy) {
        connect(button, &QPushButton::clicked, this, [this] { copySelection(); });
    } else if (action == Action::Save) {
        connect(button, &QPushButton::clicked, this, [this] { saveSelectionAs(); });
    } else if (action == Action::Cancel) {
        connect(button, &QPushButton::clicked, this, [this] { close(); });
    }

    return button;
}

QVector<ShotWindow::DesktopApp> ShotWindow::imageDesktopApps() const
{
    QVector<DesktopApp> apps;
    QStringList seenPaths;

    for (const QString &appDir : desktopSearchDirs()) {
        if (!QDir(appDir).exists()) {
            continue;
        }

        QDirIterator iterator(appDir, {QStringLiteral("*.desktop")}, QDir::Files, QDirIterator::Subdirectories);
        while (iterator.hasNext()) {
            const QString desktopPath = iterator.next();
            if (seenPaths.contains(desktopPath)) {
                continue;
            }
            seenPaths.append(desktopPath);

            QFile file(desktopPath);
            if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
                continue;
            }

            const QStringList lines = QString::fromUtf8(file.readAll()).split(QLatin1Char('\n'));
            if (desktopEntryValue(lines, QStringLiteral("Type")) != QStringLiteral("Application")) {
                continue;
            }
            if (desktopEntryBool(lines, QStringLiteral("Hidden"))
                || desktopEntryBool(lines, QStringLiteral("NoDisplay"))
                || !desktopEntrySupportsImage(lines)) {
                continue;
            }

            const QString exec = desktopEntryValue(lines, QStringLiteral("Exec"));
            const QString name = desktopEntryValue(lines, QStringLiteral("Name"));
            const QString icon = desktopEntryValue(lines, QStringLiteral("Icon"));
            if (exec.isEmpty() || name.isEmpty()) {
                continue;
            }

            apps.append({name, desktopPath, exec, icon});
        }
    }

    std::sort(apps.begin(), apps.end(), [](const DesktopApp &left, const DesktopApp &right) {
        return QString::localeAwareCompare(left.name, right.name) < 0;
    });
    return apps;
}

QVector<ShotWindow::ExtensionCommand> ShotWindow::extensionCommands(QString *errorMessage) const
{
    if (errorMessage) {
        errorMessage->clear();
    }

    const QString configPath = extensionCommandsConfigPath();
    QFile file(configPath);
    if (!file.exists()) {
        return {};
    }
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Cannot read %1").arg(configPath);
        }
        return {};
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Invalid JSON at offset %1: %2").arg(parseError.offset).arg(parseError.errorString());
        }
        return {};
    }

    QJsonArray commandArray;
    if (document.isArray()) {
        commandArray = document.array();
    } else if (document.isObject()) {
        const QJsonObject root = document.object();
        if (root.value(QStringLiteral("commands")).isArray()) {
            commandArray = root.value(QStringLiteral("commands")).toArray();
        } else if (root.value(QStringLiteral("command")).isString()) {
            commandArray.append(root);
        }
    } else {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Expected a JSON array, a command object, or an object with a commands array");
        }
        return {};
    }

    if (commandArray.isEmpty()) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("No extension commands found");
        }
        return {};
    }

    QVector<ExtensionCommand> commands;
    for (const QJsonValue &value : commandArray) {
        if (!value.isObject()) {
            continue;
        }

        const QJsonObject object = value.toObject();
        ExtensionCommand command;
        command.name = object.value(QStringLiteral("name")).toString().trimmed();
        command.command = object.value(QStringLiteral("command")).toString().trimmed();
        command.workingDirectory = object.value(QStringLiteral("workingDirectory"))
                                       .toString(object.value(QStringLiteral("cwd")).toString())
                                       .trimmed();
        command.description = object.value(QStringLiteral("description")).toString().trimmed();
        command.saveImage = extensionCommandUsesImagePlaceholder(command.command)
            || object.value(QStringLiteral("saveImage")).toBool(false)
            || object.value(QStringLiteral("needsImage")).toBool(false);
        if (object.value(QStringLiteral("closeOnStart")).isBool()) {
            command.closeOnStart = object.value(QStringLiteral("closeOnStart")).toBool();
        }

        if (command.name.isEmpty() || command.command.isEmpty()) {
            continue;
        }
        commands.append(command);
    }

    return commands;
}

bool ShotWindow::eventFilter(QObject *watched, QEvent *event)
{
    if (event->type() == QEvent::MouseButtonPress || event->type() == QEvent::KeyPress) {
        clearWheelPreview();
    }

    const bool isFullscreenMoveButton = m_fullscreenAnnotation
        && watched->property("action").toString() == markshot::ui::actionName(Action::ToolMove);
    if (isFullscreenMoveButton) {
        if (event->type() == QEvent::MouseButtonPress) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                auto *eventWidget = qobject_cast<QWidget *>(watched);
                if (!eventWidget) {
                    return false;
                }
                m_dragging = true;
                m_toolbarDragging = true;
                m_toolbarDragStart = eventWidget->mapTo(this, mouseEvent->pos());
                m_toolbarBeforeDrag = m_toolbar->geometry();
                setCursor(Qt::SizeAllCursor);
                return true;
            }
        } else if (event->type() == QEvent::MouseMove && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            auto *eventWidget = qobject_cast<QWidget *>(watched);
            if (!eventWidget) {
                return false;
            }
            const QPoint delta = eventWidget->mapTo(this, mouseEvent->pos()) - m_toolbarDragStart;
            m_toolbarUserPlaced = true;
            m_toolbar->setGeometry(clampedToolbarGeometry(m_toolbarBeforeDrag.translated(delta)));
            updateOpenWithPanelGeometry();
            updateExtensionPanelGeometry();
            updateAnnotationPropertyPanelGeometry();
            return true;
        } else if (event->type() == QEvent::MouseButtonRelease && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                m_dragging = false;
                m_toolbarDragging = false;
                updateCursor();
                updateOpenWithPanelGeometry();
                updateExtensionPanelGeometry();
                updateAnnotationPropertyPanelGeometry();
                return true;
            }
        }
    }

    if (watched == m_textEditor && event->type() == QEvent::KeyPress) {
        auto *keyEvent = static_cast<QKeyEvent *>(event);
        if (imageNavigationAvailable() && keyEvent->key() == Qt::Key_Control && !keyEvent->isAutoRepeat()) {
            if (m_ctrlTapTimer.isValid() && m_ctrlTapTimer.elapsed() <= kCtrlDoubleTapMs) {
                resetImageZoom();
                m_ctrlTapTimer.invalidate();
            } else {
                m_ctrlTapTimer.restart();
            }
            return true;
        }
        if (keyEvent->key() == Qt::Key_Escape) {
            m_draft.reset();
            m_editingTextAnnotationId.reset();
            m_textEditor->hide();
            m_textEditor->clear();
            setFocus(Qt::OtherFocusReason);
            update();
            return true;
        }
    }

    if (watched == m_colorPalette && event->type() == QEvent::MouseButtonPress) {
        m_colorPalette->hide();
        update();
        return true;
    }

    return QWidget::eventFilter(watched, event);
}

void ShotWindow::setFullscreenActionButtonsVisible(bool visible)
{
    for (QPushButton *button : std::as_const(m_fullscreenActionButtons)) {
        if (button) {
            button->setVisible(visible);
        }
    }
}

void ShotWindow::setStartupTool(StartupTool tool)
{
    if (m_mode != Mode::Selecting) {
        return;
    }

    if (m_startupTool == tool) {
        leaveStartupTool();
        return;
    }

    m_startupTool = tool;
    m_dragging = false;
    m_hoveredWindowRect.reset();
    m_startupHoverValid = false;
    m_startupRulerDragging = false;
    m_startupRulerHasMeasure = false;
    if (m_startupColorPanel) {
        m_startupColorPanel->hide();
    }
    setCursor(tool == StartupTool::ColorPicker ? Qt::CrossCursor : Qt::SizeAllCursor);
    update();
}

void ShotWindow::leaveStartupTool()
{
    m_startupTool = StartupTool::None;
    m_startupHoverValid = false;
    m_startupRulerDragging = false;
    m_startupRulerHasMeasure = false;
    m_dragging = false;
    if (m_startupColorPanel) {
        m_startupColorPanel->hide();
    }
    setCursor(Qt::CrossCursor);
    update();
}

QColor ShotWindow::sampledImageColor(QPointF imagePoint) const
{
    if (m_frozenFrame.isNull()) {
        return {};
    }

    const QPointF clamped = clampImagePoint(imagePoint);
    const int x = std::clamp(qRound(clamped.x()), 0, std::max(0, m_frozenFrame.width() - 1));
    const int y = std::clamp(qRound(clamped.y()), 0, std::max(0, m_frozenFrame.height() - 1));
    return m_frozenFrame.pixelColor(x, y);
}

void ShotWindow::showStartupColorDialog(QColor color, QPoint anchor)
{
    if (!color.isValid()) {
        return;
    }

    if (m_startupColorPanel) {
        m_startupColorPanel->deleteLater();
        m_startupColorPanel = nullptr;
    }

    m_startupColorPanel = new QWidget(this);
    m_startupColorPanel->setObjectName(QStringLiteral("startupColorInspector"));
    m_startupColorPanel->setAttribute(Qt::WA_DeleteOnClose, true);
    m_startupColorPanel->setStyleSheet(QStringLiteral(
        "QWidget#startupColorInspector {"
        " background: rgba(229, 231, 235, 238);"
        " border: 1px solid rgba(15, 23, 42, 55);"
        " border-radius: 16px;"
        "}"
        "QLabel { color: #172033; font-size: 12px; }"
        "QLabel#formatName { font-weight: 700; color: #475569; min-width: 76px; }"
        "QLabel#formatValue {"
        " font-family: %1;"
        " font-weight: 700;"
        " color: #172033;"
        "}"
        "QFrame#formatRow {"
        " background: rgba(248, 250, 252, 228);"
        " border-radius: 9px;"
        "}"
        "QPushButton {"
        " background: rgba(148, 163, 184, 70);"
        " border: 0;"
        " border-radius: 7px;"
        " padding: 5px 9px;"
        " color: #334155;"
        " font-weight: 700;"
        "}"
        "QPushButton:hover { background: rgba(45, 212, 191, 150); color: #042F2E; }")
            .arg(markshot::theme::monospaceFontFamilyCss()));

    auto *layout = new QVBoxLayout(m_startupColorPanel);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(9);

    const QString hex = colorHexRgb(color);
    auto *preview = new QLabel(hex, m_startupColorPanel);
    preview->setAlignment(Qt::AlignCenter);
    preview->setMinimumSize(300, 88);
    preview->setStyleSheet(QStringLiteral(
        "QLabel {"
        " background: %1;"
        " border-radius: 13px;"
        " color: %2;"
        " font-size: 20px;"
        " font-weight: 800;"
        "}").arg(hex, propertyIconInkForFill(color).name()));
    layout->addWidget(preview);

    struct FormatRow {
        QString name;
        QString value;
    };
    const QVector<FormatRow> rows = {
        {QStringLiteral("HEX"), hex},
        {QStringLiteral("hex lower"), hex.toLower()},
        {QStringLiteral("RGBA hex"), colorHexRgba(color)},
        {QStringLiteral("RGB"), QStringLiteral("rgb(%1, %2, %3)").arg(color.red()).arg(color.green()).arg(color.blue())},
        {QStringLiteral("RGBA"), QStringLiteral("rgba(%1, %2, %3, %4)").arg(color.red()).arg(color.green()).arg(color.blue()).arg(alphaText(color))},
        {QStringLiteral("HSL"), QStringLiteral("hsl(%1, %2%, %3%)")
             .arg(colorHueOrZero(color.hslHue()))
             .arg(qRound(color.hslSaturationF() * 100.0))
             .arg(qRound(color.lightnessF() * 100.0))},
        {QStringLiteral("HSV"), QStringLiteral("hsv(%1, %2%, %3%)")
             .arg(colorHueOrZero(color.hsvHue()))
             .arg(qRound(color.hsvSaturationF() * 100.0))
             .arg(qRound(color.valueF() * 100.0))},
        {QStringLiteral("Qt"), QStringLiteral("Qt.rgba(%1, %2, %3, %4)")
             .arg(normalizedColorChannel(color.red()),
                  normalizedColorChannel(color.green()),
                  normalizedColorChannel(color.blue()),
                  alphaText(color))},
    };

    for (const FormatRow &row : rows) {
        auto *frame = new QFrame(m_startupColorPanel);
        frame->setObjectName(QStringLiteral("formatRow"));
        auto *rowLayout = new QHBoxLayout(frame);
        rowLayout->setContentsMargins(10, 7, 8, 7);
        rowLayout->setSpacing(8);

        auto *nameLabel = new QLabel(row.name, frame);
        nameLabel->setObjectName(QStringLiteral("formatName"));
        rowLayout->addWidget(nameLabel);

        auto *valueLabel = new QLabel(row.value, frame);
        valueLabel->setObjectName(QStringLiteral("formatValue"));
        valueLabel->setTextInteractionFlags(Qt::TextSelectableByMouse);
        rowLayout->addWidget(valueLabel, 1);

        auto *copyButton = new QPushButton(MS_TR("Copy"), frame);
        copyButton->setFocusPolicy(Qt::NoFocus);
        connect(copyButton, &QPushButton::clicked, this, [this, value = row.value] {
            if (!markshot::copyTextToClipboard(value)) {
                return;
            }
            QTimer::singleShot(180, this, [this] {
                emit sessionCancelRequested();
                close();
            });
        });
        rowLayout->addWidget(copyButton);
        layout->addWidget(frame);
    }

    m_startupColorPanel->adjustSize();
    const QSize panelSize = m_startupColorPanel->sizeHint();
    int x = anchor.x() + 22;
    int y = anchor.y() + 22;
    if (x + panelSize.width() > width() - 12) {
        x = anchor.x() - panelSize.width() - 22;
    }
    if (y + panelSize.height() > height() - 12) {
        y = anchor.y() - panelSize.height() - 22;
    }
    x = std::clamp(x, 12, std::max(12, width() - panelSize.width() - 12));
    y = std::clamp(y, 12, std::max(12, height() - panelSize.height() - 12));
    m_startupColorPanel->setGeometry(QRect(QPoint(x, y), panelSize));
    m_startupColorPanel->show();
    m_startupColorPanel->raise();
}

void ShotWindow::drawStartupColorLoupe(QPainter &painter, QPointF imagePoint) const
{
    if (!m_startupHoverValid || m_frozenFrame.isNull()) {
        return;
    }

    const QColor color = sampledImageColor(imagePoint);
    const QPointF widgetPoint = imageToWidget(imagePoint);
    QRectF loupe(widgetPoint.x() + 22.0,
                 widgetPoint.y() + 22.0,
                 m_startupColorLoupeSize,
                 m_startupColorLoupeSize);
    if (loupe.right() > width() - 12.0) {
        loupe.moveRight(widgetPoint.x() - 22.0);
    }
    if (loupe.bottom() > height() - 12.0) {
        loupe.moveBottom(widgetPoint.y() - 22.0);
    }
    loupe.moveLeft(std::clamp(loupe.left(), 12.0, std::max(12.0, width() - loupe.width() - 12.0)));
    loupe.moveTop(std::clamp(loupe.top(), 12.0, std::max(12.0, height() - loupe.height() - 12.0)));

    const QPoint center = clampImagePoint(imagePoint).toPoint();
    const QRect sourceCandidate(center.x() - 6, center.y() - 6, 13, 13);
    const QRect sourceRect = sourceCandidate.intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));

    painter.save();
    QPainterPath clip;
    clip.addEllipse(loupe);
    painter.setClipPath(clip);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, false);
    painter.drawImage(loupe, m_frozenFrame.copy(sourceRect));
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(59, 40, 46), 3.0));
    painter.setBrush(Qt::NoBrush);
    painter.drawEllipse(loupe);

    const QPointF c = loupe.center();
    painter.setPen(QPen(QColor(96, 165, 250), 2.0));
    painter.drawLine(QPointF(c.x() - 12.0, c.y()), QPointF(c.x() + 12.0, c.y()));
    painter.drawLine(QPointF(c.x(), c.y() - 12.0), QPointF(c.x(), c.y() + 12.0));

    painter.setFont(markshot::theme::uiFont(11, QFont::DemiBold));
    const QString hex = colorHexRgb(color);
    const QFontMetrics metrics(painter.font());
    const QRectF label(loupe.center().x() - (metrics.horizontalAdvance(hex) + 20.0) / 2.0,
                       loupe.bottom() - metrics.height() - 11.0,
                       metrics.horizontalAdvance(hex) + 20.0,
                       metrics.height() + 7.0);
    drawRoundedLabel(painter, label, hex, QColor(8, 13, 19, 210));
    painter.restore();
}

void ShotWindow::drawStartupRuler(QPainter &painter) const
{
    if (!m_startupHoverValid && !m_startupRulerHasMeasure && !m_startupRulerDragging) {
        return;
    }

    painter.save();
    painter.setFont(markshot::theme::uiFont(10, QFont::DemiBold));
    painter.setPen(QPen(QColor(45, 212, 191, 150), 1.0, Qt::DashLine));
    auto clampFloatingRect = [this](QRectF rect) {
        if (rect.right() > width() - 8.0) {
            rect.moveRight(width() - 8.0);
        }
        if (rect.left() < 8.0) {
            rect.moveLeft(8.0);
        }
        if (rect.bottom() > height() - 8.0) {
            rect.moveBottom(height() - 8.0);
        }
        if (rect.top() < 8.0) {
            rect.moveTop(8.0);
        }
        return rect;
    };

    std::optional<QRectF> hoverLabelRect;
    if (m_startupHoverValid) {
        const QPointF hover = imageToWidget(m_startupHoverImagePoint);
        painter.drawLine(QPointF(m_frozenImageRect.left(), hover.y()), QPointF(m_frozenImageRect.right(), hover.y()));
        painter.drawLine(QPointF(hover.x(), m_frozenImageRect.top()), QPointF(hover.x(), m_frozenImageRect.bottom()));

        const QColor color = sampledImageColor(m_startupHoverImagePoint);
        const QString hoverText = QStringLiteral("x %1  y %2  %3")
                                      .arg(qRound(m_startupHoverImagePoint.x()))
                                      .arg(qRound(m_startupHoverImagePoint.y()))
                                      .arg(colorHexRgb(color));
        const QFontMetrics metrics(painter.font());
        QRectF hoverLabel(hover.x() + 14.0,
                          hover.y() + 14.0,
                          metrics.horizontalAdvance(hoverText) + 18.0,
                          metrics.height() + 10.0);
        if (hoverLabel.right() > width() - 8.0) {
            hoverLabel.moveRight(hover.x() - 14.0);
        }
        if (hoverLabel.bottom() > height() - 8.0) {
            hoverLabel.moveBottom(hover.y() - 14.0);
        }
        hoverLabel = clampFloatingRect(hoverLabel);
        hoverLabelRect = hoverLabel;
        drawRoundedLabel(painter, hoverLabel, hoverText, QColor(8, 13, 19, 225));
    }

    if (!m_startupRulerHasMeasure && !m_startupRulerDragging) {
        painter.restore();
        return;
    }

    const QRectF imageRect = normalizedRect(m_startupRulerStart, m_startupRulerEnd);
    if (imageRect.width() < 1.0 && imageRect.height() < 1.0) {
        painter.restore();
        return;
    }

    const QRectF widgetRect = imageRectToWidget(imageRect);
    painter.setPen(QPen(QColor(45, 212, 191), 2.0));
    painter.setBrush(QColor(45, 212, 191, 26));
    painter.drawRoundedRect(widgetRect, 3.0, 3.0);

    const int widthPx = qRound(imageRect.width());
    const int heightPx = qRound(imageRect.height());
    const qreal diagonal = std::hypot(imageRect.width(), imageRect.height());
    const QString info = QStringLiteral("%1 x %2 px   diag %3 px   area %4 px")
                             .arg(widthPx)
                             .arg(heightPx)
                             .arg(qRound(diagonal))
                             .arg(widthPx * heightPx);
    const QFontMetrics metrics(painter.font());
    const QSizeF infoSize(metrics.horizontalAdvance(info) + 20.0, metrics.height() + 10.0);
    constexpr qreal kRulerFloatingGap = 8.0;
    constexpr qreal kRulerMajorTickLength = 13.0;
    constexpr qreal kRulerMinorTickLength = 6.0;
    constexpr qreal kRulerTopLabelOffset = 30.0;
    constexpr qreal kRulerTopLabelWidth = 68.0;
    constexpr qreal kRulerTickLabelHeight = 16.0;
    constexpr qreal kRulerLeftLabelOffset = 62.0;
    constexpr qreal kRulerLeftLabelWidth = 48.0;
    constexpr qreal kRulerTopScaleGap = kRulerTopLabelOffset + kRulerFloatingGap;
    constexpr qreal kRulerBottomScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
    constexpr qreal kRulerRightScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
    constexpr qreal kRulerLeftScaleGap = kRulerLeftLabelOffset + kRulerFloatingGap;
    const QVector<QRectF> infoCandidates = {
        QRectF(QPointF(widgetRect.left(), widgetRect.bottom() + kRulerBottomScaleGap), infoSize),
        QRectF(QPointF(widgetRect.left(), widgetRect.top() - infoSize.height() - kRulerTopScaleGap), infoSize),
        QRectF(QPointF(widgetRect.right() + kRulerRightScaleGap, widgetRect.top()), infoSize),
        QRectF(QPointF(widgetRect.left() - infoSize.width() - kRulerLeftScaleGap, widgetRect.top()), infoSize),
    };
    const QRectF rulerScaleArea = widgetRect.adjusted(-kRulerLeftScaleGap,
                                                      -kRulerTopScaleGap,
                                                      kRulerRightScaleGap,
                                                      kRulerBottomScaleGap);
    auto overlapsInfoObstacle = [&](const QRectF &rect) {
        if (rect.intersects(rulerScaleArea)) {
            return true;
        }
        if (!hoverLabelRect.has_value()) {
            return false;
        }
        return rect.intersects(hoverLabelRect->adjusted(-kRulerFloatingGap,
                                                        -kRulerFloatingGap,
                                                        kRulerFloatingGap,
                                                        kRulerFloatingGap));
    };

    QRectF infoRect = clampFloatingRect(infoCandidates.first());
    for (const QRectF &candidate : infoCandidates) {
        const QRectF clamped = clampFloatingRect(candidate);
        if (!overlapsInfoObstacle(clamped)) {
            infoRect = clamped;
            break;
        }
    }
    drawRoundedLabel(painter, infoRect, info);

    const int stepX = rulerTickStep(imageRect.width());
    const int stepY = rulerTickStep(imageRect.height());
    const int minorX = std::max(1, stepX / 5);
    const int minorY = std::max(1, stepY / 5);
    painter.setPen(QPen(QColor(204, 251, 241, 230), 1.0));
    auto drawXTick = [&](int tick, bool major) {
        if (imageRect.width() <= 0.0) {
            return;
        }
        const qreal x = widgetRect.left() + tick * widgetRect.width() / imageRect.width();
        const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
        painter.drawLine(QPointF(x, widgetRect.top()), QPointF(x, widgetRect.top() - length));
        painter.drawLine(QPointF(x, widgetRect.bottom()), QPointF(x, widgetRect.bottom() + length));
        if (major) {
            const QString text = QString::number(tick);
            painter.drawText(QRectF(x - kRulerTopLabelWidth / 2.0,
                                    widgetRect.top() - kRulerTopLabelOffset,
                                    kRulerTopLabelWidth,
                                    kRulerTickLabelHeight),
                             Qt::AlignCenter,
                             text);
        }
    };
    auto drawYTick = [&](int tick, bool major) {
        if (imageRect.height() <= 0.0) {
            return;
        }
        const qreal y = widgetRect.top() + tick * widgetRect.height() / imageRect.height();
        const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
        painter.drawLine(QPointF(widgetRect.left(), y), QPointF(widgetRect.left() - length, y));
        painter.drawLine(QPointF(widgetRect.right(), y), QPointF(widgetRect.right() + length, y));
        if (major) {
            const QString text = QString::number(tick);
            painter.drawText(QRectF(widgetRect.left() - kRulerLeftLabelOffset,
                                    y - kRulerTickLabelHeight / 2.0,
                                    kRulerLeftLabelWidth,
                                    kRulerTickLabelHeight),
                             Qt::AlignRight | Qt::AlignVCenter,
                             text);
        }
    };

    for (int x = 0; x <= widthPx; x += minorX) {
        drawXTick(x, x % stepX == 0);
    }
    for (int y = 0; y <= heightPx; y += minorY) {
        drawYTick(y, y % stepY == 0);
    }

    painter.restore();
}

void ShotWindow::drawStartupToolOverlay(QPainter &painter)
{
    if (m_startupTool == StartupTool::ColorPicker) {
        drawStartupColorLoupe(painter, m_startupHoverImagePoint);
    } else if (m_startupTool == StartupTool::Ruler) {
        drawStartupRuler(painter);
    }
}

void ShotWindow::paintEvent(QPaintEvent *)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.fillRect(rect(), QColor(0, 0, 0));
    const qreal imageScale = m_frozenFrame.isNull()
        ? 1.0
        : m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    const QRectF visibleImageRect = m_frozenImageRect.intersected(QRectF(rect()));
    const qreal dpr = devicePixelRatioF();
    const QSize sharpTargetSize(qMax(1, qRound(visibleImageRect.width() * dpr)),
                                qMax(1, qRound(visibleImageRect.height() * dpr)));
    const qsizetype sharpPixels =
        static_cast<qsizetype>(sharpTargetSize.width()) * sharpTargetSize.height();
    if (imageNavigationAvailable() && imageScale > 1.01 && !visibleImageRect.isEmpty()
        && sharpPixels <= kMaxSharpViewportPixels) {
        const QRectF sourceRect((visibleImageRect.left() - m_frozenImageRect.left()) / imageScale,
                                (visibleImageRect.top() - m_frozenImageRect.top()) / imageScale,
                                visibleImageRect.width() / imageScale,
                                visibleImageRect.height() / imageScale);
        const bool cacheHit = !m_sharpViewportCache.isNull()
            && m_sharpViewportCacheSourceRect == sourceRect
            && m_sharpViewportCacheTargetSize == sharpTargetSize
            && qFuzzyCompare(m_sharpViewportCacheDpr, dpr);
        QImage rendered = cacheHit
            ? m_sharpViewportCache
            : renderSharpViewport(m_frozenFrame, sourceRect, sharpTargetSize);
        if (!rendered.isNull()) {
            rendered.setDevicePixelRatio(dpr);
            if (!cacheHit) {
                m_sharpViewportCache = rendered;
                m_sharpViewportCacheSourceRect = sourceRect;
                m_sharpViewportCacheTargetSize = sharpTargetSize;
                m_sharpViewportCacheDpr = dpr;
            }
            painter.drawImage(visibleImageRect, rendered);
        } else {
            m_sharpViewportCache = {};
            painter.drawImage(m_frozenImageRect, m_frozenFrame);
        }
    } else {
        m_sharpViewportCache = {};
        painter.drawImage(m_frozenImageRect, m_frozenFrame);
    }

    const QRectF selection = normalizedSelection();
    QPainterPath dimPath;
    dimPath.addRect(rect());
    if (hasUsableSelection()) {
        dimPath.addRect(imageRectToWidget(selection));
        painter.fillPath(dimPath, QColor(2, 6, 12, 128));
    } else {
        painter.fillRect(rect(), QColor(2, 6, 12, 88));
    }

    if (hasUsableSelection()) {
        const QRectF widgetSelection = imageRectToWidget(selection);
        painter.save();
        for (const Annotation &annotation : m_annotations) {
            if (m_editingTextAnnotationId.has_value() && annotation.id == *m_editingTextAnnotationId) {
                continue;
            }
            drawAnnotation(painter, annotation, true);
        }
        if (m_draft.has_value()) {
            drawAnnotation(painter, *m_draft, true);
        }
        const qint64 now = m_laserClock.isValid() ? m_laserClock.elapsed() : 0;
        for (const LaserStroke &stroke : m_laserStrokes) {
            const qreal opacity = std::clamp(static_cast<qreal>(stroke.expiresAt - now) / kLaserLifetimeMs, 0.0, 1.0);
            if (opacity > 0.0) {
                drawLaserStroke(painter, stroke, true, opacity);
            }
        }
        if (m_laserDraft.has_value()) {
            drawLaserStroke(painter, *m_laserDraft, true, 1.0);
        }
        drawSelectedAnnotationFrame(painter);
        if (m_annotationSelectionBoxActive) {
            const QRectF box = imageRectToWidget(m_annotationSelectionBox.normalized());
            painter.setPen(QPen(QColor(45, 212, 191), 1.5, Qt::DashLine));
            painter.setBrush(QColor(45, 212, 191, 34));
            painter.drawRoundedRect(box, 4.0, 4.0);
        }
        painter.restore();

        painter.setPen(QPen(QColor(94, 234, 212), 2.0));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(widgetSelection, 3.0, 3.0);

        if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(94, 234, 212));
            const QVector<QPointF> handles = {
                widgetSelection.topLeft(), QPointF(widgetSelection.center().x(), widgetSelection.top()), widgetSelection.topRight(),
                QPointF(widgetSelection.left(), widgetSelection.center().y()), QPointF(widgetSelection.right(), widgetSelection.center().y()),
                widgetSelection.bottomLeft(), QPointF(widgetSelection.center().x(), widgetSelection.bottom()), widgetSelection.bottomRight(),
            };
            for (const QPointF &handle : handles) {
                painter.drawRoundedRect(QRectF(handle.x() - 4.0, handle.y() - 4.0, 8.0, 8.0), 2.0, 2.0);
            }
        }

        const bool selectionInfoVisible = m_selectionDrag != SelectionDrag::None
            || (m_showSelectionInfo && m_selectionInfoTimer.isValid() && m_selectionInfoTimer.elapsed() <= 1000);
        if (selectionInfoVisible) {
            const QString sizeText = QStringLiteral("%1 x %2").arg(qRound(selection.width())).arg(qRound(selection.height()));
            painter.setFont(markshot::theme::uiFont(11, QFont::DemiBold));
            const QFontMetrics metrics(painter.font());
            const QRectF labelRect(widgetSelection.left() + 10.0,
                                   widgetSelection.top() + 10.0,
                                   metrics.horizontalAdvance(sizeText) + 22.0,
                                   metrics.height() + 12.0);
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(8, 13, 19, 220));
            painter.drawRoundedRect(labelRect, 10.0, 10.0);
            painter.setPen(QColor(204, 251, 241, 238));
            painter.drawText(labelRect, Qt::AlignCenter, sizeText);
        } else if (m_showSelectionInfo) {
            m_showSelectionInfo = false;
        }
    }

    if (m_hoveredWindowRect.has_value() && m_mode == Mode::Selecting && m_startupTool == StartupTool::None) {
        const QRectF hoverWidget = imageRectToWidget(QRectF(*m_hoveredWindowRect));
        painter.setPen(QPen(QColor(94, 234, 212), 2.0));
        painter.setBrush(QColor(94, 234, 212, 32));
        painter.drawRect(hoverWidget);
    }

    drawStartupToolOverlay(painter);

    if (!hasUsableSelection() && m_startupTool == StartupTool::None) {
        const QString hint = MS_TR("Drag to select   C color picker   R ruler   Middle switches   Right/Esc cancels");
        painter.setFont(markshot::theme::uiFont(15, QFont::DemiBold));
        const QFontMetrics metrics(painter.font());
        const QRectF hintRect((width() - metrics.horizontalAdvance(hint) - 44.0) / 2.0,
                              (height() - metrics.height() - 24.0) / 2.0,
                              metrics.horizontalAdvance(hint) + 44.0,
                              metrics.height() + 24.0);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(8, 13, 19, 222));
        painter.drawRoundedRect(hintRect, 18.0, 18.0);
        painter.setPen(QColor(204, 251, 241, 240));
        painter.drawText(hintRect, Qt::AlignCenter, hint);
    }

    drawWheelPreview(painter);
}

void ShotWindow::resizeEvent(QResizeEvent *)
{
    updateFrozenImageRect();
    if (m_colorPalette && m_colorPalette->isVisible()) {
        updateColorPaletteGeometry(m_colorPaletteAnchor);
    }
    updateTextEditorGeometry();
    updateImageScrollBars();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
}

void ShotWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    markshot::windows::setExcludedFromCapture(this);
}

void ShotWindow::mousePressEvent(QMouseEvent *event)
{
    clearWheelPreview();

    if (m_mode == Mode::Selecting && m_startupTool != StartupTool::None) {
        if (event->button() == Qt::RightButton) {
            leaveStartupTool();
            event->accept();
            return;
        }
        if (event->button() != Qt::LeftButton) {
            event->accept();
            return;
        }
        if (!m_frozenImageRect.contains(event->position())) {
            event->accept();
            return;
        }

        const QPointF imagePoint = clampImagePoint(widgetToImage(event->position()));
        m_startupHoverImagePoint = imagePoint;
        m_startupHoverValid = true;
        if (m_startupTool == StartupTool::ColorPicker) {
            showStartupColorDialog(sampledImageColor(imagePoint), event->pos());
            update();
            event->accept();
            return;
        }
        if (m_startupTool == StartupTool::Ruler) {
            m_startupRulerDragging = true;
            m_startupRulerHasMeasure = true;
            m_startupRulerStart = imagePoint;
            m_startupRulerEnd = imagePoint;
            update();
            event->accept();
            return;
        }
    }

    if (event->button() != Qt::LeftButton) {
        if (m_mode == Mode::Selecting) {
            if (event->button() == Qt::RightButton) {
                emit sessionCancelRequested();
                close();
                event->accept();
                return;
            }
            if (event->button() == Qt::MiddleButton) {
                enterFullscreenAnnotation(true);
                event->accept();
                return;
            }
        }
        if (event->button() == Qt::MiddleButton && imageNavigationAvailable() && m_frozenImageRect.contains(event->position())) {
            commitTextEditor();
            m_dragging = false;
            m_annotationSelectionBoxActive = false;
            m_imagePanning = true;
            m_imagePanStartWidget = event->position();
            m_imagePanStartCenter = m_imageCenterInitialized
                ? m_imageCenter
                : QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
            updateCursor();
            event->accept();
            return;
        }
        if (event->button() == Qt::RightButton && m_mode == Mode::Editing) {
            setTool(Tool::Select);
            event->accept();
            return;
        }
        return;
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (m_openWithPanel && m_openWithPanel->isVisible()
        && !m_openWithPanel->geometry().contains(event->pos())
        && (!m_actionToolbar || !m_actionToolbar->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel && m_extensionPanel->isVisible()
        && !m_extensionPanel->geometry().contains(event->pos())
        && (!m_actionToolbar || !m_actionToolbar->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_extensionPanel->hide();
    }
    if (m_colorPalette && m_colorPalette->isVisible()
        && !m_colorPalette->geometry().contains(event->pos())) {
        m_colorPalette->hide();
        update();
    }
    if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()
        && !m_propertyColorDialogPanel->geometry().contains(event->pos())
        && (!m_annotationPropertyPanel || !m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel && m_propertyFontPanel->isVisible()
        && !m_propertyFontPanel->geometry().contains(event->pos())
        && (!m_annotationPropertyPanel || !m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_propertyFontPanel->hide();
    }
    if (m_textEditor && m_textEditor->isVisible() && !m_textEditor->geometry().contains(event->pos())) {
        commitTextEditor();
    }

    if (m_mode == Mode::Selecting) {
        if (m_colorPalette) {
            m_colorPalette->hide();
        }
        m_selectionClickStart = event->position();
        beginSelection(imagePoint);
        return;
    }

    if (!m_frozenImageRect.contains(event->position())) {
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
        m_selectionDrag = selectionDragAt(imagePoint);
        if (m_selectionDrag == SelectionDrag::None) {
            updateCursor();
            return;
        }
        m_dragging = true;
        m_dragStart = imagePoint;
        m_selectionBeforeDrag = normalizedSelection();
        revealSelectionInfo();
        updateCursor();
        update();
        return;
    }

    if (m_tool == Tool::Select) {
        if (selectedAnnotationDeleteButtonRect().contains(event->position())) {
            deleteSelectedAnnotation();
            return;
        }

        const QVector<int> selectedIds = selectedAnnotationIds();
        if (selectedIds.size() > 1) {
            const SelectionDrag drag = selectedAnnotationsDragAt(imagePoint);
            if (drag != SelectionDrag::None) {
                beginAnnotationDrag(selectedIds.first(), drag, imagePoint);
                return;
            }
        } else if (m_selectedAnnotationId.has_value()) {
            const SelectionDrag drag = annotationDragAt(imagePoint, *m_selectedAnnotationId);
            if (drag != SelectionDrag::None) {
                beginAnnotationDrag(*m_selectedAnnotationId, drag, imagePoint);
                return;
            }
        }

        const std::optional<int> hitAnnotationId = annotationAt(imagePoint);
        if (hitAnnotationId.has_value()) {
            const SelectionDrag drag = annotationDragAt(imagePoint, *hitAnnotationId);
            setSelectedAnnotations({*hitAnnotationId});
            beginAnnotationDrag(*hitAnnotationId, drag == SelectionDrag::None ? SelectionDrag::Move : drag, imagePoint);
            updateAnnotationPropertyPanel();
        } else if (m_imageNavigationEnabled) {
            beginAnnotationSelectionBox(imagePoint);
            m_imageSelected = true;
        } else {
            beginAnnotationSelectionBox(imagePoint);
        }
        return;
    }

    if (m_tool == Tool::Text) {
        commitTextEditor();
        beginTextAnnotation(imagePoint);
        return;
    }

    if (m_tool == Tool::Number) {
        Annotation annotation;
        annotation.tool = Tool::Number;
        annotation.points.append(clampImagePoint(imagePoint));
        annotation.points.append(clampImagePoint(imagePoint));
        annotation.number = m_nextNumber;
        annotation.color = m_currentColor;
        annotation.width = m_numberWidth;
        m_dragging = true;
        m_dragStart = annotation.points.first();
        m_draft = annotation;
        update();
        return;
    }

    if (m_tool == Tool::Magnifier) {
        const QPointF sourceCenter = clampImagePoint(imagePoint);
        Annotation annotation;
        annotation.tool = Tool::Magnifier;
        annotation.points.append(sourceCenter);
        annotation.points.append(sourceCenter);
        annotation.rect = QRectF(sourceCenter, sourceCenter);
        annotation.color = m_currentColor;
        annotation.width = currentToolWidth();
        annotation.magnifierScale = m_magnifierScale;
        m_dragging = true;
        m_dragStart = sourceCenter;
        m_draft = annotation;
        update();
        return;
    }

    if (m_tool == Tool::Laser) {
        beginLaserStroke(imagePoint);
        return;
    }

    m_dragging = true;
    m_dragStart = imagePoint;
    Annotation annotation;
    annotation.tool = m_tool;
    annotation.color = m_currentColor;
    annotation.width = currentToolWidth();
    annotation.filled = m_shapeFilled;
    annotation.cornerRadius = m_tool == Tool::Rectangle ? m_rectangleCornerRadius : 0.0;
    annotation.arrowStyle = m_arrowStyle;
    annotation.fontFamily = m_textFontFamily;
    annotation.rotationDegrees = 0.0;
    annotation.highlighterStyle = m_tool == Tool::Highlighter ? m_highlighterStyle : HighlighterStyle::Freehand;
    if (m_tool == Tool::Pen
        || (m_tool == Tool::Highlighter && annotation.highlighterStyle == HighlighterStyle::Freehand)) {
        annotation.points.append(imagePoint);
    } else if (m_tool == Tool::Mosaic) {
        annotation.width = m_mosaicBlockSize;
        annotation.rect = QRectF(imagePoint, imagePoint);
        annotation.points.append(imagePoint);
        annotation.points.append(imagePoint);
    } else {
        annotation.rect = QRectF(imagePoint, imagePoint);
        annotation.points.append(imagePoint);
        annotation.points.append(imagePoint);
    }
    m_draft = annotation;
    update();
}

void ShotWindow::mouseMoveEvent(QMouseEvent *event)
{
    if (m_imagePanning) {
        panImageTo(event->position());
        event->accept();
        return;
    }

    if (m_showWheelPreview && m_wheelPreviewTimer.isValid() && m_wheelPreviewTimer.elapsed() <= 900) {
        m_wheelPreviewPosition = event->position();
        update();
    } else if (m_showWheelPreview) {
        m_showWheelPreview = false;
        updateCursor();
        update();
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (m_mode == Mode::Selecting && m_startupTool != StartupTool::None) {
        if (m_frozenImageRect.contains(event->position()) || m_startupRulerDragging) {
            m_startupHoverImagePoint = clampImagePoint(imagePoint);
            m_startupHoverValid = true;
            if (m_startupTool == StartupTool::Ruler && m_startupRulerDragging) {
                m_startupRulerEnd = m_startupHoverImagePoint;
            }
        } else {
            m_startupHoverValid = false;
        }
        update();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting && !m_dragging) {
        std::optional<QRect> best;
        qint64 bestArea = std::numeric_limits<qint64>::max();
        const QPoint imgPt = imagePoint.toPoint();
        for (const QRect &r : std::as_const(m_windowRects)) {
            if (r.contains(imgPt)) {
                qint64 area = static_cast<qint64>(r.width()) * r.height();
                if (area < bestArea) {
                    bestArea = area;
                    best = r;
                }
            }
        }
        if (best != m_hoveredWindowRect) {
            m_hoveredWindowRect = best;
            update();
        }
    }
    if (m_mode == Mode::Selecting && m_dragging) {
        m_selection = normalizedRect(m_selectionStart, imagePoint);
        revealSelectionInfo();
        update();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && m_dragging && m_annotationDrag != SelectionDrag::None) {
        updateAnnotationDrag(imagePoint, event->modifiers().testFlag(Qt::ControlModifier));
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && m_dragging && m_annotationSelectionBoxActive) {
        updateAnnotationSelectionBox(imagePoint);
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && !m_dragging) {
        if (selectedAnnotationIds().size() > 1) {
            m_annotationDrag = selectedAnnotationsDragAt(imagePoint);
            if (m_annotationDrag != SelectionDrag::None) {
                updateCursor();
                return;
            }
        } else if (m_selectedAnnotationId.has_value()) {
            m_annotationDrag = annotationDragAt(imagePoint, *m_selectedAnnotationId);
            if (m_annotationDrag != SelectionDrag::None) {
                updateCursor();
                return;
            }
        }
        m_annotationDrag = annotationAt(imagePoint).has_value() ? SelectionDrag::Move : SelectionDrag::None;
        updateCursor();
        return;
    }

    if (m_fullscreenAnnotation && m_toolbarDragging) {
        const QPoint delta = event->pos() - m_toolbarDragStart;
        QRect toolbarGeometry = m_toolbarBeforeDrag.translated(delta);
        if (m_toolbar) {
            m_toolbarUserPlaced = true;
            m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        }
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Move && !m_fullscreenAnnotation && !m_dragging) {
        const SelectionDrag hoverDrag = selectionDragAt(imagePoint);
        switch (hoverDrag) {
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
            setCursor(Qt::SizeAllCursor);
            break;
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            break;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            break;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            break;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            break;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            break;
        case SelectionDrag::None:
            setCursor(Qt::CrossCursor);
            break;
        }
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Move && !m_fullscreenAnnotation && m_dragging && m_selectionDrag != SelectionDrag::None) {
        const QPointF clamped = clampImagePoint(imagePoint);
        const QRectF start = m_selectionBeforeDrag;
        const qreal maxWidth = m_frozenFrame.width();
        const qreal maxHeight = m_frozenFrame.height();
        qreal left = start.left();
        qreal top = start.top();
        qreal right = start.right();
        qreal bottom = start.bottom();

        if (m_selectionDrag == SelectionDrag::Move) {
            const QPointF delta = clamped - m_dragStart;
            left = std::clamp(start.left() + delta.x(), 0.0, std::max<qreal>(0.0, maxWidth - start.width()));
            top = std::clamp(start.top() + delta.y(), 0.0, std::max<qreal>(0.0, maxHeight - start.height()));
            right = left + start.width();
            bottom = top + start.height();
        } else {
            if (m_selectionDrag == SelectionDrag::Left || m_selectionDrag == SelectionDrag::TopLeft
                || m_selectionDrag == SelectionDrag::BottomLeft) {
                left = std::clamp(clamped.x(), 0.0, right - kMinSelectionSize);
            }
            if (m_selectionDrag == SelectionDrag::Right || m_selectionDrag == SelectionDrag::TopRight
                || m_selectionDrag == SelectionDrag::BottomRight) {
                right = std::clamp(clamped.x(), left + kMinSelectionSize, maxWidth);
            }
            if (m_selectionDrag == SelectionDrag::Top || m_selectionDrag == SelectionDrag::TopLeft
                || m_selectionDrag == SelectionDrag::TopRight) {
                top = std::clamp(clamped.y(), 0.0, bottom - kMinSelectionSize);
            }
            if (m_selectionDrag == SelectionDrag::Bottom || m_selectionDrag == SelectionDrag::BottomLeft
                || m_selectionDrag == SelectionDrag::BottomRight) {
                bottom = std::clamp(clamped.y(), top + kMinSelectionSize, maxHeight);
            }
        }

        m_selection = QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
        revealSelectionInfo();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateTextEditorGeometry();
        update();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Laser && m_dragging && m_laserDraft.has_value()) {
        updateLaserStroke(imagePoint);
        return;
    }

    if (m_mode != Mode::Editing || !m_dragging || !m_draft.has_value()) {
        return;
    }

    const QPointF clamped = clampImagePoint(imagePoint);
    if (m_draft->tool == Tool::Pen
        || (m_draft->tool == Tool::Highlighter
            && m_draft->highlighterStyle == HighlighterStyle::Freehand)) {
        m_draft->points.append(clamped);
    } else if (m_draft->tool == Tool::Magnifier) {
        const qreal dragDistance = QLineF(m_dragStart, clamped).length();
        if (dragDistance < kMinMagnifierDragDistance) {
            m_draft->rect = QRectF(m_dragStart, m_dragStart);
            m_draft->points[1] = clamped;
            update();
            return;
        }

        const qreal frameDiameter = std::min<qreal>(m_frozenFrame.width(), m_frozenFrame.height());
        const qreal maxDiameter = std::max<qreal>(4.0,
                                                  std::min(kMaxMagnifierDiameter,
                                                           frameDiameter));
        const qreal minDiameter = std::min(kMinMagnifierDiameter, maxDiameter);
        const qreal diameter = std::clamp(dragDistance * kMagnifierDragScale,
                                          minDiameter,
                                          maxDiameter);
        const QRectF lensRect = magnifierCircleRect(clamped, diameter);
        m_draft->rect = lensRect;
        m_draft->points[1] = lensRect.center();
    } else if (m_draft->tool == Tool::Number) {
        if (m_draft->points.size() < 2) {
            m_draft->points.append(clamped);
        } else {
            m_draft->points[1] = clamped;
        }
        m_draft->rect = QRectF(m_dragStart, clamped).normalized();
    } else {
        if ((m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse || m_draft->tool == Tool::Magnifier)
            && event->modifiers().testFlag(Qt::ControlModifier)) {
            m_draft->rect = constrainedRect(m_dragStart, clamped);
        } else {
            m_draft->rect = normalizedRect(m_dragStart, clamped);
        }
        if (m_draft->points.size() >= 2) {
            m_draft->points[1] = (m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse || m_draft->tool == Tool::Magnifier)
                ? m_draft->rect.bottomRight()
                : clamped;
        }
    }
    update();
}

void ShotWindow::mouseDoubleClickEvent(QMouseEvent *event)
{
    if (event->button() == Qt::RightButton && m_mode == Mode::Editing) {
        toggleColorPalette(event->pos());
        event->accept();
        return;
    }

    if (event->button() != Qt::LeftButton || m_mode != Mode::Editing) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (!m_frozenImageRect.contains(event->position())) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const auto annotationId = annotationAt(imagePoint);
    if (!annotationId) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const Annotation *annotation = annotationById(*annotationId);
    if (!annotation || annotation->tool != Tool::Text) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const int targetId = *annotationId;

    // The single click that preceded this double-click already executed
    // mousePressEvent in the active tool's branch. Roll back its side
    // effects so the user does not see a stray duplicate annotation when
    // we transition into text editing.
    switch (m_tool) {
    case Tool::Number:
        m_draft.reset();
        break;
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Line:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Arrow:
    case Tool::Mosaic:
    case Tool::Magnifier:
        // First press created an in-flight draft; discard it so the
        // upcoming mouseReleaseEvent (which still fires for the second
        // click of the double-click) does not commit a tiny stamp.
        m_draft.reset();
        break;
    case Tool::Laser:
        m_laserDraft.reset();
        break;
    case Tool::Text:
        // First press opened a fresh, empty text editor at the click point.
        // setTool(Select) below will call commitTextEditor() and tear it
        // down without producing an empty annotation.
        break;
    case Tool::Move:
    case Tool::Select:
        // No draft to discard.
        break;
    }

    m_dragging = false;
    m_annotationDrag = SelectionDrag::None;
    m_annotationHistoryCaptured = false;

    if (m_tool != Tool::Select) {
        setTool(Tool::Select);
    }
    setSelectedAnnotations({targetId});
    beginEditingSelectedTextAnnotation();
    update();
    event->accept();
}

void ShotWindow::mouseReleaseEvent(QMouseEvent *event)
{
    if (m_mode == Mode::Selecting
        && m_startupTool == StartupTool::Ruler
        && event->button() == Qt::LeftButton
        && m_startupRulerDragging) {
        m_startupRulerDragging = false;
        if (m_frozenImageRect.contains(event->position())) {
            m_startupRulerEnd = clampImagePoint(widgetToImage(event->position()));
            m_startupHoverImagePoint = m_startupRulerEnd;
            m_startupHoverValid = true;
        }
        update();
        event->accept();
        return;
    }

    if ((event->button() == Qt::LeftButton || event->button() == Qt::MiddleButton) && m_imagePanning) {
        m_imagePanning = false;
        updateCursor();
        event->accept();
        return;
    }

    if (event->button() != Qt::LeftButton || !m_dragging) {
        return;
    }

    m_dragging = false;
    if (m_toolbarDragging) {
        m_toolbarDragging = false;
        updateCursor();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_tool == Tool::Select && m_annotationDrag != SelectionDrag::None) {
        m_annotationDrag = SelectionDrag::None;
        m_annotationHistoryCaptured = false;
        updateAnnotationPropertyPanel();
        updateCursor();
        update();
        return;
    }

    if (m_tool == Tool::Select && m_annotationSelectionBoxActive) {
        if (m_imageNavigationEnabled && m_imageSelected) {
            const QRectF box = m_annotationSelectionBox.normalized();
            if (box.width() < kMinSelectionSize || box.height() < kMinSelectionSize) {
                m_annotationSelectionBoxActive = false;
                m_annotationSelectionBox = {};
                updateAnnotationPropertyPanel();
                updateCursor();
                update();
                return;
            }
            m_imageSelected = false;
        }
        commitAnnotationSelectionBox();
        updateCursor();
        update();
        return;
    }

    if (m_mode == Mode::Selecting) {
        const QPointF releasePos = event->position();
        const qreal clickDistance = QLineF(m_selectionClickStart, releasePos).length();
        if (clickDistance < 5.0 && m_hoveredWindowRect.has_value()) {
            m_selection = QRectF(*m_hoveredWindowRect);
            m_hoveredWindowRect.reset();
            m_dragging = false;
            if (!hasUsableSelection()) {
                m_selection = {};
                update();
                return;
            }
            m_mode = Mode::Editing;
            m_fullscreenAnnotation = false;
            m_toolbarUserPlaced = false;
            setTool(defaultEditingTool());
            setFullscreenActionButtonsVisible(false);
            m_toolbar->show();
            m_actionToolbar->show();
            emit selectionActivated(this);
            revealSelectionInfo();
            updateToolbarGeometry();
            updateActionToolbarGeometry();
            update();
            return;
        }
        m_hoveredWindowRect.reset();
        m_selection = normalizedSelection();
        if (!hasUsableSelection()) {
            m_selection = {};
            update();
            return;
        }
        m_mode = Mode::Editing;
        m_fullscreenAnnotation = false;
        m_toolbarUserPlaced = false;
        setTool(defaultEditingTool());
        setFullscreenActionButtonsVisible(false);
        m_toolbar->show();
        m_actionToolbar->show();
        emit selectionActivated(this);
        revealSelectionInfo();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        update();
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation && m_selectionDrag != SelectionDrag::None) {
        m_selection = normalizedSelection();
        m_selectionDrag = SelectionDrag::None;
        revealSelectionInfo();
        updateCursor();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        update();
        return;
    }

    if (m_tool == Tool::Laser && m_laserDraft.has_value()) {
        commitLaserStroke();
        updateCursor();
        update();
        return;
    }

    commitDraft();
}

void ShotWindow::wheelEvent(QWheelEvent *event)
{
    if (m_mode == Mode::Selecting && m_startupTool == StartupTool::ColorPicker) {
        const int delta = event->angleDelta().y() != 0 ? event->angleDelta().y() : event->pixelDelta().y();
        if (delta == 0) {
            QWidget::wheelEvent(event);
            return;
        }

        const qreal factor = std::pow(1.12, static_cast<qreal>(delta) / 120.0);
        m_startupColorLoupeSize = std::clamp(m_startupColorLoupeSize * factor,
                                             kMinStartupColorLoupeSize,
                                             kMaxStartupColorLoupeSize);
        if (m_frozenImageRect.contains(event->position())) {
            m_startupHoverImagePoint = clampImagePoint(widgetToImage(event->position()));
            m_startupHoverValid = true;
        }
        event->accept();
        update();
        return;
    }

    const int steps = event->angleDelta().y() / 120;
    if (steps == 0 || m_mode != Mode::Editing) {
        QWidget::wheelEvent(event);
        return;
    }

    if (m_tool == Tool::Select && !selectedAnnotationIds().isEmpty()) {
        pushHistorySnapshot();
        for (int id : selectedAnnotationIds()) {
            if (Annotation *annotation = annotationById(id)) {
                if (annotation->tool == Tool::Mosaic) {
                    annotation->width = std::clamp(annotation->width + steps * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
                } else if (annotation->tool == Tool::Number) {
                    annotation->width = std::clamp(annotation->width + steps * 2.0, kMinNumberWidth, kMaxNumberWidth);
                } else if (annotation->tool == Tool::Text) {
                    const qreal oldWidth = annotation->width;
                    annotation->width = std::clamp(annotation->width + steps * 1.5, 1.0, 1000.0);
                    const qreal factor = ((19.0 + annotation->width) / (19.0 + oldWidth)) * 1.05;
                    annotation->rect.setWidth(annotation->rect.width() * factor);
                    annotation->rect = textContentRect(*annotation, false);
                    if (!annotation->points.isEmpty()) {
                        annotation->points[0] = annotation->rect.topLeft();
                    }
                } else {
                    annotation->width = std::clamp(annotation->width + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
                }
            }
        }
        updateColorPalettePreview();
        updateAnnotationPropertyPanel();
        event->accept();
        update();
        return;
    }

    if (wheelZoomsImage()) {
        const qreal factor = imageNavigationWheelFactor(event);
        if (qFuzzyCompare(factor, 1.0)) {
            QWidget::wheelEvent(event);
            return;
        }
        zoomImageAt(factor, event->position());
        m_showWheelPreview = true;
        m_wheelPreviewPosition = event->position();
        m_wheelPreviewTimer.restart();
        updateCursor();
        event->accept();
        update();
        return;
    }

    if (m_tool == Tool::Mosaic) {
        m_mosaicBlockSize = std::clamp(m_mosaicBlockSize + steps * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
    } else if (m_tool == Tool::Number) {
        m_numberWidth = std::clamp(m_numberWidth + steps * 2.0, kMinNumberWidth, kMaxNumberWidth);
    } else if (m_tool == Tool::Laser) {
        m_laserWidth = std::clamp(m_laserWidth + steps * 2.0, kMinLaserWidth, kMaxLaserWidth);
    } else if (m_tool == Tool::Pen || m_tool == Tool::Highlighter) {
        m_penWidth = std::clamp(m_penWidth + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
    } else if (m_tool == Tool::Text) {
        m_shapeWidth = std::clamp(m_shapeWidth + steps * 1.5, 1.0, 1000.0);
    } else {
        m_shapeWidth = std::clamp(m_shapeWidth + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
    }

    if (m_draft.has_value()) {
        m_draft->width = currentToolWidth();
    }
    m_showWheelPreview = true;
    m_wheelPreviewPosition = event->position();
    m_wheelPreviewTimer.restart();
    updateCursor();
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    event->accept();
    update();
}

void ShotWindow::keyPressEvent(QKeyEvent *event)
{
    clearWheelPreview();

    if (m_mode == Mode::Selecting
        && m_startupTool != StartupTool::None
        && eventMatchesShortcut(event, Action::Cancel)) {
        leaveStartupTool();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting) {
        if (eventMatchesStartupShortcut(event, StartupTool::ColorPicker)) {
            setStartupTool(StartupTool::ColorPicker);
            event->accept();
            return;
        }
        if (eventMatchesStartupShortcut(event, StartupTool::Ruler)) {
            setStartupTool(StartupTool::Ruler);
            event->accept();
            return;
        }
    }

    if (imageNavigationAvailable() && event->key() == Qt::Key_Control && !event->isAutoRepeat()) {
        if (m_ctrlTapTimer.isValid() && m_ctrlTapTimer.elapsed() <= kCtrlDoubleTapMs) {
            resetImageZoom();
            m_ctrlTapTimer.invalidate();
        } else {
            m_ctrlTapTimer.restart();
        }
        event->accept();
        return;
    }

    if (handleConfiguredActionShortcut(event)) {
        event->accept();
        return;
    }

    if ((event->key() == Qt::Key_Backspace || event->key() == Qt::Key_Delete)
        && m_mode == Mode::Editing
        && m_tool == Tool::Select
        && !selectedAnnotationIds().isEmpty()) {
        commitTextEditor();
        deleteSelectedAnnotation();
        return;
    }

    switch (event->key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
        saveSelection();
        break;
    default:
        if (!handleConfiguredToolShortcut(event)) {
            QWidget::keyPressEvent(event);
        } else {
            event->accept();
        }
        break;
    }
}

void ShotWindow::beginSelection(QPointF imagePoint)
{
    m_dragging = true;
    m_fullscreenAnnotation = false;
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_selectionDrag = SelectionDrag::None;
    m_selectionBeforeFullscreenAnnotation.reset();
    m_selectionStart = imagePoint;
    m_selection = QRectF(imagePoint, imagePoint);
    if (m_textEditor) {
        m_textEditor->hide();
        m_textEditor->clear();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    setFullscreenActionButtonsVisible(false);
    m_annotations.clear();
    m_undoStack.clear();
    m_redoStack.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    revealSelectionInfo();
    update();
}

void ShotWindow::commitDraft()
{
    if (!m_draft.has_value()) {
        return;
    }

    const bool highlighterLineDraft = m_draft->tool == Tool::Highlighter
        && m_draft->highlighterStyle == HighlighterStyle::StraightLine;
    const bool highlighterFreehandDraft = m_draft->tool == Tool::Highlighter
        && m_draft->highlighterStyle == HighlighterStyle::Freehand;

    if ((m_draft->tool == Tool::Pen || highlighterFreehandDraft) && m_draft->points.size() < 2) {
        m_draft.reset();
        update();
        return;
    }

    if ((m_draft->tool == Tool::Line || m_draft->tool == Tool::Arrow || highlighterLineDraft)
        && m_draft->points.size() >= 2
        && QLineF(m_draft->points.first(), m_draft->points.last()).length() < 2.0) {
        m_draft.reset();
        update();
        return;
    }

    if (m_draft->tool != Tool::Pen && !highlighterFreehandDraft && !highlighterLineDraft && m_draft->tool != Tool::Line
        && m_draft->tool != Tool::Arrow && m_draft->tool != Tool::Text && m_draft->tool != Tool::Number
        && (m_draft->rect.width() < 2.0 || m_draft->rect.height() < 2.0)) {
        m_draft.reset();
        update();
        return;
    }

    pushHistorySnapshot();
    if (m_draft->id == 0) {
        m_draft->id = m_nextAnnotationId++;
    }
    if (m_draft->tool == Tool::Number) {
        if (m_draft->number <= 0) {
            m_draft->number = m_nextNumber;
        }
        m_nextNumber = std::max(m_nextNumber, m_draft->number + 1);
    }
    m_annotations.append(*m_draft);
    m_draft.reset();
    update();
}

void ShotWindow::setTool(Tool tool)
{
    clearWheelPreview();
    commitTextEditor();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    if (tool != Tool::Laser) {
        m_laserDraft.reset();
    }
    m_tool = tool;
    if (m_tool != Tool::Select) {
        setSelectedAnnotations({});
        m_imageSelected = false;
        m_imagePanning = false;
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    updateToolbarState();
    update();
}

void ShotWindow::updateCursor()
{
    if (m_showWheelPreview && m_wheelPreviewTimer.isValid() && m_wheelPreviewTimer.elapsed() <= 900) {
        setCursor(Qt::BlankCursor);
        return;
    }

    if (m_imagePanning) {
        setCursor(Qt::ClosedHandCursor);
        return;
    }

    if (m_imageNavigationEnabled && m_tool == Tool::Select && m_imageSelected) {
        setCursor(m_imagePanning ? Qt::ClosedHandCursor : Qt::OpenHandCursor);
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
        switch (m_selectionDrag) {
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            return;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            return;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            return;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            return;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::None:
            setCursor(Qt::CrossCursor);
            return;
        }
    }

    if (m_tool == Tool::Select) {
        switch (m_annotationDrag) {
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            return;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            return;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            return;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            return;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::None:
            setCursor(Qt::ArrowCursor);
            return;
        }
    }

    setCursor(m_tool == Tool::Text ? Qt::IBeamCursor : Qt::CrossCursor);
}

void ShotWindow::clearWheelPreview()
{
    if (!m_showWheelPreview) {
        return;
    }

    m_showWheelPreview = false;
    m_wheelPreviewTimer.invalidate();
    updateCursor();
    update();
}

bool ShotWindow::hasUsableSelection() const
{
    const QRectF selection = normalizedSelection();
    return selection.width() >= kMinSelectionSize && selection.height() >= kMinSelectionSize;
}

bool ShotWindow::imageNavigationAvailable() const
{
    return m_imageNavigationEnabled || m_mode == Mode::Editing;
}

bool ShotWindow::wheelZoomsImage() const
{
    return m_imageNavigationEnabled || (m_mode == Mode::Editing && m_tool == Tool::Select);
}

qreal ShotWindow::annotationSizeScale(bool widgetCoordinates) const
{
    if (!widgetCoordinates || m_frozenFrame.isNull()) {
        return 1.0;
    }

    return !m_frozenImageRect.isEmpty()
        ? m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width())
        : 1.0;
}

ShotWindow::SelectionDrag ShotWindow::selectionDragAt(QPointF imagePoint) const
{
    const QRectF selection = normalizedSelection();
    if (selection.isEmpty() || m_frozenImageRect.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 10.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    if (!selection.adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance).contains(imagePoint)) {
        return SelectionDrag::None;
    }

    const bool nearLeft = std::abs(imagePoint.x() - selection.left()) <= imageTolerance;
    const bool nearRight = std::abs(imagePoint.x() - selection.right()) <= imageTolerance;
    const bool nearTop = std::abs(imagePoint.y() - selection.top()) <= imageTolerance;
    const bool nearBottom = std::abs(imagePoint.y() - selection.bottom()) <= imageTolerance;

    if (nearLeft && nearTop) {
        return SelectionDrag::TopLeft;
    }
    if (nearRight && nearTop) {
        return SelectionDrag::TopRight;
    }
    if (nearLeft && nearBottom) {
        return SelectionDrag::BottomLeft;
    }
    if (nearRight && nearBottom) {
        return SelectionDrag::BottomRight;
    }
    if (nearLeft) {
        return SelectionDrag::Left;
    }
    if (nearRight) {
        return SelectionDrag::Right;
    }
    if (nearTop) {
        return SelectionDrag::Top;
    }
    if (nearBottom) {
        return SelectionDrag::Bottom;
    }
    return selection.contains(imagePoint) ? SelectionDrag::Move : SelectionDrag::None;
}

ShotWindow::Annotation *ShotWindow::annotationById(int id)
{
    for (Annotation &annotation : m_annotations) {
        if (annotation.id == id) {
            return &annotation;
        }
    }
    return nullptr;
}

const ShotWindow::Annotation *ShotWindow::annotationById(int id) const
{
    for (const Annotation &annotation : m_annotations) {
        if (annotation.id == id) {
            return &annotation;
        }
    }
    return nullptr;
}

bool ShotWindow::annotationSupportsRotation(const Annotation &annotation) const
{
    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return false;
    case Tool::Pen:
    case Tool::Line:
    case Tool::Highlighter:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
    case Tool::Text:
    case Tool::Number:
    case Tool::Magnifier:
        return true;
    }
    return false;
}

bool ShotWindow::annotationSupportsLineControl(const Annotation &annotation) const
{
    return annotation.tool == Tool::Line
        || annotation.tool == Tool::Arrow
        || (annotation.tool == Tool::Highlighter
            && annotation.highlighterStyle == HighlighterStyle::StraightLine);
}

QPointF ShotWindow::annotationLineControlPoint(const Annotation &annotation) const
{
    if (!annotationSupportsLineControl(annotation) || annotation.points.size() < 2) {
        return {};
    }
    if (annotation.points.size() >= 3) {
        return annotation.points.at(2);
    }
    return (annotation.points.first() + annotation.points.at(1)) / 2.0;
}

QPointF ShotWindow::rotatedPoint(QPointF point, QPointF center, qreal degrees) const
{
    const qreal radians = degrees * M_PI / 180.0;
    const qreal c = std::cos(radians);
    const qreal s = std::sin(radians);
    const QPointF delta = point - center;
    return center + QPointF(delta.x() * c - delta.y() * s,
                            delta.x() * s + delta.y() * c);
}

QRectF ShotWindow::rotatedRectBounds(QRectF rect, qreal degrees) const
{
    rect = rect.normalized();
    if (rect.isEmpty() || qFuzzyIsNull(degrees)) {
        return rect;
    }

    const QPointF center = rect.center();
    const QVector<QPointF> points = {
        rotatedPoint(rect.topLeft(), center, degrees),
        rotatedPoint(rect.topRight(), center, degrees),
        rotatedPoint(rect.bottomLeft(), center, degrees),
        rotatedPoint(rect.bottomRight(), center, degrees),
    };

    qreal left = points.first().x();
    qreal right = left;
    qreal top = points.first().y();
    qreal bottom = top;
    for (const QPointF &point : points) {
        left = std::min(left, point.x());
        right = std::max(right, point.x());
        top = std::min(top, point.y());
        bottom = std::max(bottom, point.y());
    }
    return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
}

QRectF ShotWindow::annotationUnrotatedBounds(const Annotation &annotation) const
{
    auto pointsBounds = [&annotation] {
        if (annotation.points.isEmpty()) {
            return QRectF();
        }
        qreal left = annotation.points.first().x();
        qreal right = left;
        qreal top = annotation.points.first().y();
        qreal bottom = top;
        for (const QPointF &point : annotation.points) {
            left = std::min(left, point.x());
            right = std::max(right, point.x());
            top = std::min(top, point.y());
            bottom = std::max(bottom, point.y());
        }
        return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
    };

    QRectF bounds;
    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return {};
    case Tool::Pen:
    case Tool::Line:
    case Tool::Arrow:
        bounds = pointsBounds();
        bounds.adjust(-annotation.width, -annotation.width, annotation.width, annotation.width);
        break;
    case Tool::Highlighter:
        if (annotation.highlighterStyle == HighlighterStyle::StraightLine
            && annotation.points.size() >= 2) {
            bounds = annotation.points.size() >= 3
                ? pointsBounds()
                : QRectF(annotation.points.first(), annotation.points.at(1)).normalized();
        } else {
            bounds = pointsBounds();
        }
        bounds.adjust(-annotation.width, -annotation.width, annotation.width, annotation.width);
        break;
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
        bounds = annotation.rect.normalized();
        break;
    case Tool::Magnifier:
        bounds = annotation.rect.normalized().united(magnifierSourceRect(annotation));
        bounds.adjust(-annotation.width, -annotation.width, annotation.width, annotation.width);
        break;
    case Tool::Text:
        bounds = textContentRect(annotation, false);
        break;
    case Tool::Number: {
        if (annotation.points.isEmpty()) {
            return {};
        }
        const qreal radius = std::max<qreal>(13.0, 13.0 + annotation.width * 1.35);
        const QPointF center = annotation.points.first();
        const QPointF tip = annotation.points.size() >= 2 ? annotation.points.last() : center;
        bounds = QRectF(center.x() - radius, center.y() - radius, radius * 2.0, radius * 2.0);
        bounds = bounds.united(QRectF(tip, QSizeF(0.0, 0.0)));
        break;
    }
    }

    return bounds.normalized().intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QPointF ShotWindow::annotationRotationCenter(const Annotation &annotation, bool widgetCoordinates) const
{
    const QRectF bounds = annotationUnrotatedBounds(annotation);
    const QPointF center = bounds.center();
    return widgetCoordinates ? imageToWidget(center) : center;
}

QPointF ShotWindow::annotationRotationHandlePoint(const Annotation &annotation, bool widgetCoordinates) const
{
    QRectF bounds = annotationUnrotatedBounds(annotation);
    if (bounds.isEmpty()) {
        return {};
    }

    if (widgetCoordinates) {
        bounds = imageRectToWidget(bounds);
    }
    const QPointF center = bounds.center();
    const qreal angle = annotation.rotationDegrees;
    const QPointF topCenter = rotatedPoint(QPointF(bounds.center().x(), bounds.top()), center, angle);
    QPointF direction = topCenter - center;
    const qreal length = QLineF(center, topCenter).length();
    if (length <= 0.1) {
        direction = QPointF(0.0, -1.0);
    } else {
        direction /= length;
    }
    const qreal handleGap = widgetCoordinates
        ? 26.0
        : 26.0 / std::max<qreal>(0.001, annotationSizeScale(true));
    return topCenter + direction * handleGap;
}

QPointF ShotWindow::selectionRotationHandlePoint(QRectF imageBounds, bool widgetCoordinates) const
{
    imageBounds = imageBounds.normalized();
    if (imageBounds.isEmpty()) {
        return {};
    }

    QRectF bounds = widgetCoordinates ? imageRectToWidget(imageBounds) : imageBounds;
    const QPointF center = bounds.center();
    const QPointF topCenter(bounds.center().x(), bounds.top());
    QPointF direction = topCenter - center;
    const qreal length = QLineF(center, topCenter).length();
    if (length <= 0.1) {
        direction = QPointF(0.0, -1.0);
    } else {
        direction /= length;
    }

    const qreal handleGap = widgetCoordinates
        ? 26.0
        : 26.0 / std::max<qreal>(0.001, annotationSizeScale(true));
    return topCenter + direction * handleGap;
}

QRectF ShotWindow::annotationBounds(const Annotation &annotation) const
{
    const QRectF bounds = annotationUnrotatedBounds(annotation);
    if (!annotationSupportsRotation(annotation)) {
        return bounds;
    }
    return rotatedRectBounds(bounds, annotation.rotationDegrees)
        .intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QVector<int> ShotWindow::selectedAnnotationIds() const
{
    QVector<int> ids;
    for (int id : m_selectedAnnotationIds) {
        if (annotationById(id) && !ids.contains(id)) {
            ids.append(id);
        }
    }
    if (m_selectedAnnotationId.has_value() && annotationById(*m_selectedAnnotationId) && !ids.contains(*m_selectedAnnotationId)) {
        ids.append(*m_selectedAnnotationId);
    }
    return ids;
}

void ShotWindow::setSelectedAnnotations(QVector<int> annotationIds)
{
    QVector<int> validIds;
    for (int id : annotationIds) {
        if (annotationById(id) && !validIds.contains(id)) {
            validIds.append(id);
        }
    }
    m_selectedAnnotationIds = validIds;
    m_selectedAnnotationId = validIds.size() == 1
        ? std::optional<int>(validIds.first())
        : std::nullopt;
    if (!validIds.isEmpty()) {
        m_imageSelected = false;
        m_imagePanning = false;
    }
}

QRectF ShotWindow::selectedAnnotationsBounds() const
{
    QRectF bounds;
    for (int id : selectedAnnotationIds()) {
        const Annotation *annotation = annotationById(id);
        if (!annotation) {
            continue;
        }
        const QRectF annotationRect = annotationBounds(*annotation);
        if (annotationRect.isEmpty()) {
            continue;
        }
        bounds = bounds.isEmpty() ? annotationRect : bounds.united(annotationRect);
    }
    return bounds.normalized();
}

QVector<int> ShotWindow::annotationsInRect(QRectF imageRect) const
{
    imageRect = imageRect.normalized();
    QVector<int> ids;
    if (imageRect.width() < 2.0 || imageRect.height() < 2.0) {
        return ids;
    }
    for (const Annotation &annotation : m_annotations) {
        const QRectF bounds = annotationBounds(annotation);
        if (!bounds.isEmpty() && imageRect.intersects(bounds)) {
            ids.append(annotation.id);
        }
    }
    return ids;
}

ShotWindow::SelectionDrag ShotWindow::annotationBoundsDragAt(QPointF imagePoint, QRectF bounds) const
{
    bounds = bounds.normalized();
    if (bounds.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 10.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    const bool nearLeft = std::abs(imagePoint.x() - bounds.left()) <= imageTolerance;
    const bool nearRight = std::abs(imagePoint.x() - bounds.right()) <= imageTolerance;
    const bool nearTop = std::abs(imagePoint.y() - bounds.top()) <= imageTolerance;
    const bool nearBottom = std::abs(imagePoint.y() - bounds.bottom()) <= imageTolerance;

    if (nearLeft && nearTop) {
        return SelectionDrag::TopLeft;
    }
    if (nearRight && nearTop) {
        return SelectionDrag::TopRight;
    }
    if (nearLeft && nearBottom) {
        return SelectionDrag::BottomLeft;
    }
    if (nearRight && nearBottom) {
        return SelectionDrag::BottomRight;
    }
    if (nearLeft) {
        return SelectionDrag::Left;
    }
    if (nearRight) {
        return SelectionDrag::Right;
    }
    if (nearTop) {
        return SelectionDrag::Top;
    }
    if (nearBottom) {
        return SelectionDrag::Bottom;
    }
    return bounds.adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance).contains(imagePoint)
        ? SelectionDrag::Move
        : SelectionDrag::None;
}

ShotWindow::SelectionDrag ShotWindow::selectedAnnotationsDragAt(QPointF imagePoint) const
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    const QRectF bounds = selectedAnnotationsBounds();
    if (selectedIds.size() > 1) {
        const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
        const QPointF rotationHandle = selectionRotationHandlePoint(bounds, false);
        if (!rotationHandle.isNull() && QLineF(imagePoint, rotationHandle).length() <= imageTolerance * 1.4) {
            return SelectionDrag::Rotate;
        }
    }

    return annotationBoundsDragAt(imagePoint, bounds);
}

ShotWindow::SelectionDrag ShotWindow::magnifierDragAt(const Annotation &annotation, QPointF imagePoint) const
{
    if (annotation.tool != Tool::Magnifier) {
        return SelectionDrag::None;
    }

    const QRectF lensRect = annotation.rect.normalized();
    const QRectF sourceRect = magnifierSourceRect(annotation);
    if (lensRect.isEmpty() || sourceRect.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    if (ellipseContainsPoint(sourceRect, imagePoint, imageTolerance)) {
        return SelectionDrag::MagnifierSource;
    }
    if (ellipseContainsPoint(lensRect, imagePoint, imageTolerance)) {
        return SelectionDrag::MagnifierLens;
    }
    return SelectionDrag::None;
}

QVector<QPointF> ShotWindow::selectionHandlePoints(QRectF rect) const
{
    rect = rect.normalized();
    return {
        rect.topLeft(), QPointF(rect.center().x(), rect.top()), rect.topRight(),
        QPointF(rect.left(), rect.center().y()), QPointF(rect.right(), rect.center().y()),
        rect.bottomLeft(), QPointF(rect.center().x(), rect.bottom()), rect.bottomRight(),
    };
}

QRectF ShotWindow::selectedAnnotationDeleteButtonRect() const
{
    constexpr qreal buttonSize = 20.0;
    constexpr qreal margin = 8.0;
    auto clampedButtonRect = [this](QPointF center) {
        constexpr qreal buttonSize = 20.0;
        constexpr qreal margin = 8.0;
        const qreal x = std::clamp(center.x() - buttonSize / 2.0,
                                   margin,
                                   std::max<qreal>(margin, width() - buttonSize - margin));
        const qreal y = std::clamp(center.y() - buttonSize / 2.0,
                                   margin,
                                   std::max<qreal>(margin, height() - buttonSize - margin));
        return QRectF(x, y, buttonSize, buttonSize);
    };

    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() == 1) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotationSupportsRotation(*annotation)) {
            const QRectF localBounds = imageRectToWidget(annotationUnrotatedBounds(*annotation));
            if (!localBounds.isEmpty()) {
                const QPointF center = localBounds.center();
                const QPointF corner = rotatedPoint(localBounds.topRight(), center, annotation->rotationDegrees);
                QPointF direction = corner - center;
                const qreal length = QLineF(center, corner).length();
                if (length <= 0.1) {
                    direction = QPointF(1.0, -1.0);
                } else {
                    direction /= length;
                }
                return clampedButtonRect(corner + direction * (buttonSize * 1.2));
            }
        }
    }

    const QRectF bounds = imageRectToWidget(selectedAnnotationsBounds());
    if (bounds.isEmpty()) {
        return {};
    }
    return clampedButtonRect(QPointF(bounds.right() + buttonSize / 2.0 + margin,
                                     bounds.top() - buttonSize / 2.0 - margin));
}

QRectF ShotWindow::resizedBounds(QRectF start, SelectionDrag drag, QPointF imagePoint, bool keepAspectRatio) const
{
    start = start.normalized();
    const QPointF clamped = clampImagePoint(imagePoint);
    qreal left = start.left();
    qreal top = start.top();
    qreal right = start.right();
    qreal bottom = start.bottom();
    const qreal maxWidth = m_frozenFrame.width();
    const qreal maxHeight = m_frozenFrame.height();

    if (keepAspectRatio && drag != SelectionDrag::Move && start.width() > 0.0 && start.height() > 0.0) {
        const qreal minScale = std::max(kMinSelectionSize / start.width(), kMinSelectionSize / start.height());

        auto boundedScale = [minScale](qreal rawScale, qreal maxScale) {
            maxScale = std::max<qreal>(0.0, maxScale);
            const qreal lower = std::min(minScale, maxScale);
            return std::clamp(rawScale, lower, maxScale);
        };

        auto rectFromCorner = [&](QPointF anchor, qreal xSign, qreal ySign) {
            const qreal xDistance = std::abs(clamped.x() - anchor.x());
            const qreal yDistance = std::abs(clamped.y() - anchor.y());
            const qreal rawScale = std::max(xDistance / start.width(), yDistance / start.height());
            const qreal maxXScale = (xSign > 0.0 ? maxWidth - anchor.x() : anchor.x()) / start.width();
            const qreal maxYScale = (ySign > 0.0 ? maxHeight - anchor.y() : anchor.y()) / start.height();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            return QRectF(anchor,
                          QPointF(anchor.x() + xSign * start.width() * scale,
                                  anchor.y() + ySign * start.height() * scale)).normalized();
        };

        auto rectFromHorizontalEdge = [&](qreal anchorX, qreal xSign, qreal centerY) {
            const qreal rawScale = std::abs(clamped.x() - anchorX) / start.width();
            const qreal maxXScale = (xSign > 0.0 ? maxWidth - anchorX : anchorX) / start.width();
            const qreal maxYScale = (2.0 * std::min(centerY, maxHeight - centerY)) / start.height();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            const qreal newWidth = start.width() * scale;
            const qreal newHeight = start.height() * scale;
            return QRectF(QPointF(anchorX, centerY - newHeight / 2.0),
                          QPointF(anchorX + xSign * newWidth, centerY + newHeight / 2.0)).normalized();
        };

        auto rectFromVerticalEdge = [&](qreal anchorY, qreal ySign, qreal centerX) {
            const qreal rawScale = std::abs(clamped.y() - anchorY) / start.height();
            const qreal maxYScale = (ySign > 0.0 ? maxHeight - anchorY : anchorY) / start.height();
            const qreal maxXScale = (2.0 * std::min(centerX, maxWidth - centerX)) / start.width();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            const qreal newWidth = start.width() * scale;
            const qreal newHeight = start.height() * scale;
            return QRectF(QPointF(centerX - newWidth / 2.0, anchorY),
                          QPointF(centerX + newWidth / 2.0, anchorY + ySign * newHeight)).normalized();
        };

        switch (drag) {
        case SelectionDrag::TopLeft:
            return rectFromCorner(start.bottomRight(), -1.0, -1.0);
        case SelectionDrag::TopRight:
            return rectFromCorner(start.bottomLeft(), 1.0, -1.0);
        case SelectionDrag::BottomLeft:
            return rectFromCorner(start.topRight(), -1.0, 1.0);
        case SelectionDrag::BottomRight:
            return rectFromCorner(start.topLeft(), 1.0, 1.0);
        case SelectionDrag::Left:
            return rectFromHorizontalEdge(start.right(), -1.0, start.center().y());
        case SelectionDrag::Right:
            return rectFromHorizontalEdge(start.left(), 1.0, start.center().y());
        case SelectionDrag::Top:
            return rectFromVerticalEdge(start.bottom(), -1.0, start.center().x());
        case SelectionDrag::Bottom:
            return rectFromVerticalEdge(start.top(), 1.0, start.center().x());
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
        case SelectionDrag::Move:
        case SelectionDrag::None:
            break;
        }
    }

    if (drag == SelectionDrag::Left || drag == SelectionDrag::TopLeft || drag == SelectionDrag::BottomLeft) {
        left = std::clamp(clamped.x(), 0.0, right - kMinSelectionSize);
    }
    if (drag == SelectionDrag::Right || drag == SelectionDrag::TopRight || drag == SelectionDrag::BottomRight) {
        right = std::clamp(clamped.x(), left + kMinSelectionSize, maxWidth);
    }
    if (drag == SelectionDrag::Top || drag == SelectionDrag::TopLeft || drag == SelectionDrag::TopRight) {
        top = std::clamp(clamped.y(), 0.0, bottom - kMinSelectionSize);
    }
    if (drag == SelectionDrag::Bottom || drag == SelectionDrag::BottomLeft || drag == SelectionDrag::BottomRight) {
        bottom = std::clamp(clamped.y(), top + kMinSelectionSize, maxHeight);
    }

    return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
}

ShotWindow::SelectionDrag ShotWindow::annotationDragAt(QPointF imagePoint, int annotationId) const
{
    const Annotation *annotation = annotationById(annotationId);
    if (!annotation) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    if (annotationSupportsRotation(*annotation)) {
        const QPointF rotationHandle = annotationRotationHandlePoint(*annotation, false);
        if (!rotationHandle.isNull() && QLineF(imagePoint, rotationHandle).length() <= imageTolerance * 1.4) {
            return SelectionDrag::Rotate;
        }
    }

    QRectF localBounds;
    QPointF localPoint = imagePoint;
    if (annotationSupportsRotation(*annotation)) {
        localBounds = annotationUnrotatedBounds(*annotation);
        if (!localBounds.isEmpty()) {
            localPoint = rotatedPoint(imagePoint, localBounds.center(), -annotation->rotationDegrees);
        }
    }

    if (annotation->tool == Tool::Magnifier) {
        const SelectionDrag magnifierDrag = magnifierDragAt(*annotation, localPoint);
        if (magnifierDrag != SelectionDrag::None) {
            return magnifierDrag;
        }
    }

    if (annotationSupportsLineControl(*annotation) && annotation->points.size() >= 2) {
        const QPointF controlPoint = annotationLineControlPoint(*annotation);
        if (QLineF(localPoint, controlPoint).length() <= imageTolerance * 1.4) {
            return SelectionDrag::LineControl;
        }
    }

    if (annotationSupportsRotation(*annotation) && !localBounds.isEmpty()) {
        return annotationBoundsDragAt(localPoint, localBounds);
    }

    const QRectF bounds = annotationBounds(*annotation);
    if (bounds.isEmpty()) {
        return SelectionDrag::None;
    }

    return annotationBoundsDragAt(imagePoint, bounds);
}

std::optional<int> ShotWindow::annotationAt(QPointF imagePoint) const
{
    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    for (int i = m_annotations.size() - 1; i >= 0; --i) {
        const Annotation &annotation = m_annotations.at(i);
        QPointF localPoint = imagePoint;
        if (annotationSupportsRotation(annotation)) {
            const QRectF localBounds = annotationUnrotatedBounds(annotation);
            if (!localBounds.isEmpty()) {
                localPoint = rotatedPoint(imagePoint, localBounds.center(), -annotation.rotationDegrees);
            }
        }
        if (annotation.tool == Tool::Magnifier) {
            if (magnifierDragAt(annotation, localPoint) != SelectionDrag::None) {
                return annotation.id;
            }
        }
        const QRectF bounds = annotationBounds(annotation).adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance);
        if (bounds.contains(imagePoint)) {
            return annotation.id;
        }
    }
    return std::nullopt;
}

void ShotWindow::drawSelectedAnnotationFrame(QPainter &painter) const
{
    if (m_imageNavigationEnabled && m_tool == Tool::Select && m_imageSelected && selectedAnnotationIds().isEmpty()) {
        painter.save();
        painter.setPen(QPen(QColor(45, 212, 191), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(m_frozenImageRect.adjusted(2.0, 2.0, -2.0, -2.0), 6.0, 6.0);
        painter.restore();
        return;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }

    const QRectF bounds = imageRectToWidget(selectedAnnotationsBounds());
    if (bounds.isEmpty()) {
        return;
    }

    painter.save();
    const Annotation *singleSelectedAnnotation = selectedIds.size() == 1
        ? annotationById(selectedIds.first())
        : nullptr;
    const bool rotatedSingleSelection =
        singleSelectedAnnotation && annotationSupportsRotation(*singleSelectedAnnotation);
    if (selectedIds.size() > 1) {
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(255, 255, 255, 190), 3.0, Qt::DashLine));
        for (int id : selectedIds) {
            if (const Annotation *annotation = annotationById(id)) {
                painter.drawRoundedRect(imageRectToWidget(annotationBounds(*annotation)), 3.0, 3.0);
            }
        }
        painter.setPen(QPen(QColor(251, 146, 60, 170), 1.0, Qt::DashLine));
        for (int id : selectedIds) {
            if (const Annotation *annotation = annotationById(id)) {
                painter.drawRoundedRect(imageRectToWidget(annotationBounds(*annotation)), 3.0, 3.0);
            }
        }
    }
    if (rotatedSingleSelection) {
        const QRectF localBounds = imageRectToWidget(annotationUnrotatedBounds(*singleSelectedAnnotation));
        const QPointF center = localBounds.center();
        const qreal angle = singleSelectedAnnotation->rotationDegrees;
        const QVector<QPointF> corners = {
            rotatedPoint(localBounds.topLeft(), center, angle),
            rotatedPoint(localBounds.topRight(), center, angle),
            rotatedPoint(localBounds.bottomRight(), center, angle),
            rotatedPoint(localBounds.bottomLeft(), center, angle),
        };

        QPolygonF frame;
        for (const QPointF &corner : corners) {
            frame.append(corner);
        }
        painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawPolygon(frame);

        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(251, 146, 60));
        for (const QPointF &handle : selectionHandlePoints(localBounds)) {
            const QPointF rotatedHandle = rotatedPoint(handle, center, angle);
            painter.drawRoundedRect(QRectF(rotatedHandle.x() - 4.5,
                                           rotatedHandle.y() - 4.5,
                                           9.0,
                                           9.0),
                                    2.0,
                                    2.0);
        }

        if (annotationSupportsLineControl(*singleSelectedAnnotation)
            && singleSelectedAnnotation->points.size() >= 2) {
            const QPointF control =
                rotatedPoint(imageToWidget(annotationLineControlPoint(*singleSelectedAnnotation)), center, angle);
            painter.setBrush(QColor(255, 255, 255));
            painter.setPen(QPen(QColor(251, 146, 60), 2.0));
            painter.drawEllipse(QRectF(control.x() - 5.5,
                                       control.y() - 5.5,
                                       11.0,
                                       11.0));
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(251, 146, 60));
        }

        const QPointF topCenter =
            rotatedPoint(QPointF(localBounds.center().x(), localBounds.top()), center, angle);
        const QPointF rotateHandle = annotationRotationHandlePoint(*singleSelectedAnnotation, true);
        painter.setPen(QPen(QColor(251, 146, 60), 1.8, Qt::SolidLine, Qt::RoundCap));
        painter.drawLine(topCenter, rotateHandle);
        painter.setBrush(QColor(251, 146, 60));
        painter.setPen(QPen(QColor(255, 255, 255), 1.5));
        painter.drawEllipse(QRectF(rotateHandle.x() - 6.0,
                                   rotateHandle.y() - 6.0,
                                   12.0,
                                   12.0));
    } else {
        if (selectedIds.size() > 1) {
            painter.setPen(QPen(QColor(255, 255, 255, 230), 4.0, Qt::DashLine));
            painter.setBrush(Qt::NoBrush);
            painter.drawRoundedRect(bounds, 4.0, 4.0);
        }
        painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(bounds, 4.0, 4.0);
        for (const QPointF &handle : selectionHandlePoints(bounds)) {
            if (selectedIds.size() > 1) {
                painter.setPen(QPen(QColor(255, 255, 255), 1.5));
                painter.setBrush(QColor(255, 255, 255));
                painter.drawRoundedRect(QRectF(handle.x() - 5.8, handle.y() - 5.8, 11.6, 11.6), 2.6, 2.6);
            }
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(251, 146, 60));
            painter.drawRoundedRect(QRectF(handle.x() - 4.5, handle.y() - 4.5, 9.0, 9.0), 2.0, 2.0);
        }
        if (selectedIds.size() > 1) {
            const QPointF topCenter(bounds.center().x(), bounds.top());
            const QPointF rotateHandle = selectionRotationHandlePoint(selectedAnnotationsBounds(), true);
            if (!rotateHandle.isNull()) {
                painter.setPen(QPen(QColor(251, 146, 60), 1.8, Qt::SolidLine, Qt::RoundCap));
                painter.drawLine(topCenter, rotateHandle);
                painter.setBrush(QColor(251, 146, 60));
                painter.setPen(QPen(QColor(255, 255, 255), 1.5));
                painter.drawEllipse(QRectF(rotateHandle.x() - 6.0,
                                           rotateHandle.y() - 6.0,
                                           12.0,
                                           12.0));
            }
        }
    }
    if (selectedIds.size() == 1 && !rotatedSingleSelection) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotation->tool == Tool::Magnifier) {
            const QRectF sourceRect = imageRectToWidget(magnifierSourceRect(*annotation));
            const QRectF lensRect = imageRectToWidget(annotation->rect.normalized());
            painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
            painter.setBrush(Qt::NoBrush);
            painter.drawEllipse(sourceRect);
            painter.drawEllipse(lensRect);
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(251, 146, 60));
            for (const QPointF &center : {sourceRect.center(), lensRect.center()}) {
                painter.drawEllipse(QRectF(center.x() - 5.0,
                                           center.y() - 5.0,
                                           10.0,
                                           10.0));
            }
        }
    }
    const QRectF deleteButton = selectedAnnotationDeleteButtonRect();
    if (!deleteButton.isEmpty()) {
        painter.setBrush(QColor(239, 68, 68));
        painter.setPen(QPen(QColor(255, 255, 255), 2.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        painter.drawEllipse(deleteButton);
        painter.drawLine(deleteButton.center() + QPointF(-4.5, -4.5), deleteButton.center() + QPointF(4.5, 4.5));
        painter.drawLine(deleteButton.center() + QPointF(4.5, -4.5), deleteButton.center() + QPointF(-4.5, 4.5));
    }
    painter.restore();
}

void ShotWindow::moveAnnotation(Annotation &annotation, QPointF delta) const
{
    annotation.rect.translate(delta);
    for (QPointF &point : annotation.points) {
        point = clampImagePoint(point + delta);
    }
}

void ShotWindow::transformAnnotation(Annotation &annotation, QRectF oldBounds, QRectF newBounds) const
{
    oldBounds = oldBounds.normalized();
    newBounds = newBounds.normalized();
    if (oldBounds.width() <= 0.0 || oldBounds.height() <= 0.0) {
        moveAnnotation(annotation, newBounds.topLeft() - oldBounds.topLeft());
        return;
    }

    auto mapPoint = [this, oldBounds, newBounds](QPointF point) {
        const qreal xRatio = (point.x() - oldBounds.left()) / oldBounds.width();
        const qreal yRatio = (point.y() - oldBounds.top()) / oldBounds.height();
        return clampImagePoint(QPointF(newBounds.left() + xRatio * newBounds.width(),
                                      newBounds.top() + yRatio * newBounds.height()));
    };
    const qreal scaleFactor = std::max(newBounds.width() / oldBounds.width(),
                                       newBounds.height() / oldBounds.height());

    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return;
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        break;
    case Tool::Magnifier:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        for (QPointF &point : annotation.points) {
            point = mapPoint(point);
        }
        break;
    case Tool::Text:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        if (m_annotationDrag == SelectionDrag::TopLeft ||
            m_annotationDrag == SelectionDrag::BottomRight ||
            m_annotationDrag == SelectionDrag::TopRight ||
            m_annotationDrag == SelectionDrag::BottomLeft) {
            annotation.width = std::clamp((19.0 + annotation.width) * scaleFactor - 19.0, 1.0, 1000.0);
            if (!annotation.points.isEmpty()) {
                annotation.points[0] = annotation.rect.topLeft();
            }
        }
        break;
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Line:
    case Tool::Arrow:
        for (QPointF &point : annotation.points) {
            point = mapPoint(point);
        }
        break;
    case Tool::Number:
        if (!annotation.points.isEmpty()) {
            for (QPointF &point : annotation.points) {
                point = mapPoint(point);
            }
            annotation.width = std::clamp(annotation.width * scaleFactor, kMinNumberWidth, kMaxNumberWidth);
        }
        break;
    }
}

void ShotWindow::beginAnnotationDrag(int annotationId, SelectionDrag drag, QPointF imagePoint)
{
    Annotation *annotation = annotationById(annotationId);
    if (!annotation || drag == SelectionDrag::None) {
        return;
    }
    if (!selectedAnnotationIds().contains(annotationId)) {
        setSelectedAnnotations({annotationId});
    }
    m_annotationDrag = drag;
    m_annotationBeforeDrag = *annotation;
    m_annotationsBeforeDrag.clear();
    for (int id : selectedAnnotationIds()) {
        if (const Annotation *selected = annotationById(id)) {
            m_annotationsBeforeDrag.append(*selected);
        }
    }
    m_annotationBoundsBeforeDrag = selectedAnnotationsBounds();
    m_dragStart = imagePoint;
    m_dragging = true;
    m_annotationHistoryCaptured = false;
    updateCursor();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::updateAnnotationDrag(QPointF imagePoint, bool keepAspectRatio)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty() || m_annotationDrag == SelectionDrag::None) {
        return;
    }
    if (!m_annotationHistoryCaptured) {
        pushHistorySnapshot();
        m_annotationHistoryCaptured = true;
    }

    for (const Annotation &before : m_annotationsBeforeDrag) {
        if (Annotation *annotation = annotationById(before.id)) {
            *annotation = before;
        }
    }

    if (m_annotationDrag == SelectionDrag::Rotate) {
        const QPointF center = selectedIds.size() == 1
            ? annotationRotationCenter(m_annotationBeforeDrag, false)
            : m_annotationBoundsBeforeDrag.center();
        const QPointF startVector = m_dragStart - center;
        const QPointF currentVector = clampImagePoint(imagePoint) - center;
        if (QLineF(QPointF(0, 0), startVector).length() <= 0.1
            || QLineF(QPointF(0, 0), currentVector).length() <= 0.1) {
            return;
        }
        const qreal startAngle = std::atan2(startVector.y(), startVector.x());
        const qreal currentAngle = std::atan2(currentVector.y(), currentVector.x());
        const qreal deltaDegrees = (currentAngle - startAngle) * 180.0 / M_PI;
        if (selectedIds.size() == 1) {
            Annotation *annotation = annotationById(selectedIds.first());
            if (!annotation || !annotationSupportsRotation(m_annotationBeforeDrag)) {
                return;
            }
            annotation->rotationDegrees =
                normalizedRotationDegrees(m_annotationBeforeDrag.rotationDegrees + deltaDegrees);
        } else {
            for (const Annotation &before : m_annotationsBeforeDrag) {
                Annotation *annotation = annotationById(before.id);
                if (!annotation || !annotationSupportsRotation(before)) {
                    continue;
                }
                const QRectF beforeBounds = annotationUnrotatedBounds(before);
                if (beforeBounds.isEmpty()) {
                    continue;
                }
                const QPointF beforeCenter = beforeBounds.center();
                const QPointF rotatedCenter = rotatedPoint(beforeCenter, center, deltaDegrees);
                moveAnnotation(*annotation, rotatedCenter - beforeCenter);
                annotation->rotationDegrees = normalizedRotationDegrees(before.rotationDegrees + deltaDegrees);
            }
        }
    } else if (selectedIds.size() == 1
        && (m_annotationDrag == SelectionDrag::MagnifierSource
            || m_annotationDrag == SelectionDrag::MagnifierLens)) {
        Annotation *annotation = annotationById(selectedIds.first());
        if (!annotation || annotation->tool != Tool::Magnifier
            || m_annotationBeforeDrag.tool != Tool::Magnifier) {
            return;
        }

        const QRectF beforeLensRect = m_annotationBeforeDrag.rect.normalized();
        if (beforeLensRect.isEmpty()) {
            return;
        }

        const qreal lensDiameter = std::min(beforeLensRect.width(), beforeLensRect.height());
        const QRectF beforeSourceRect = magnifierSourceRect(m_annotationBeforeDrag);
        const qreal magnifierScale = clampedMagnifierScale(m_annotationBeforeDrag.magnifierScale);
        const QPointF delta = clampImagePoint(imagePoint) - m_dragStart;
        if (m_annotationDrag == SelectionDrag::MagnifierSource) {
            const qreal sourceDiameter = lensDiameter / magnifierScale;
            const QPointF sourceCenter =
                clampedMagnifierCircleCenter(beforeSourceRect.center() + delta, sourceDiameter);
            if (annotation->points.isEmpty()) {
                annotation->points.append(sourceCenter);
            } else {
                annotation->points[0] = sourceCenter;
            }
            if (annotation->points.size() < 2) {
                annotation->points.append(beforeLensRect.center());
            }
        } else {
            const QRectF lensRect = magnifierCircleRect(beforeLensRect.center() + delta,
                                                       lensDiameter);
            annotation->rect = lensRect;
            if (annotation->points.isEmpty()) {
                annotation->points.append(beforeSourceRect.center());
            }
            if (annotation->points.size() < 2) {
                annotation->points.append(lensRect.center());
            } else {
                annotation->points[1] = lensRect.center();
            }
        }
    } else if (selectedIds.size() == 1 && m_annotationDrag == SelectionDrag::LineControl) {
        Annotation *annotation = annotationById(selectedIds.first());
        if (!annotation || !annotationSupportsLineControl(m_annotationBeforeDrag)
            || m_annotationBeforeDrag.points.size() < 2) {
            return;
        }
        const QRectF beforeBounds = annotationUnrotatedBounds(m_annotationBeforeDrag);
        const QPointF localPoint = beforeBounds.isEmpty()
            ? clampImagePoint(imagePoint)
            : rotatedPoint(clampImagePoint(imagePoint),
                           beforeBounds.center(),
                           -m_annotationBeforeDrag.rotationDegrees);
        while (annotation->points.size() < 3) {
            annotation->points.append(annotationLineControlPoint(m_annotationBeforeDrag));
        }
        annotation->points[2] = clampImagePoint(localPoint);
        annotation->rotationDegrees = m_annotationBeforeDrag.rotationDegrees;
    } else if (m_annotationDrag == SelectionDrag::Move) {
        const QRectF startBounds = m_annotationBoundsBeforeDrag;
        QPointF delta = clampImagePoint(imagePoint) - m_dragStart;
        delta.setX(std::clamp(delta.x(), -startBounds.left(), m_frozenFrame.width() - startBounds.right()));
        delta.setY(std::clamp(delta.y(), -startBounds.top(), m_frozenFrame.height() - startBounds.bottom()));
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                moveAnnotation(*annotation, delta);
            }
        }
    } else if (selectedIds.size() == 1 && annotationSupportsRotation(m_annotationBeforeDrag)) {
        Annotation *annotation = annotationById(selectedIds.first());
        if (!annotation) {
            return;
        }
        bool lockAspectRatio = keepAspectRatio || annotation->tool == Tool::Magnifier;
        const QRectF oldBounds = annotationUnrotatedBounds(m_annotationBeforeDrag);
        const QPointF localPoint =
            rotatedPoint(clampImagePoint(imagePoint),
                         oldBounds.center(),
                         -m_annotationBeforeDrag.rotationDegrees);
        const QRectF newBounds = resizedBounds(oldBounds,
                                               m_annotationDrag,
                                               localPoint,
                                               lockAspectRatio);
        transformAnnotation(*annotation, oldBounds, newBounds);
        annotation->rotationDegrees = m_annotationBeforeDrag.rotationDegrees;
    } else {
        bool lockAspectRatio = keepAspectRatio;
        if (selectedIds.size() == 1) {
            const Annotation *annotation = annotationById(selectedIds.first());
            lockAspectRatio = lockAspectRatio
                || (annotation && annotation->tool == Tool::Magnifier);
        }
        const QRectF newBounds = resizedBounds(m_annotationBoundsBeforeDrag,
                                               m_annotationDrag,
                                               imagePoint,
                                               lockAspectRatio);
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                transformAnnotation(*annotation, m_annotationBoundsBeforeDrag, newBounds);
            }
        }
    }
    update();
}

void ShotWindow::beginAnnotationSelectionBox(QPointF imagePoint)
{
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = true;
    m_dragging = true;
    m_dragStart = clampImagePoint(imagePoint);
    m_annotationSelectionBox = QRectF(m_dragStart, m_dragStart);
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::updateAnnotationSelectionBox(QPointF imagePoint)
{
    m_annotationSelectionBox = QRectF(m_dragStart, clampImagePoint(imagePoint)).normalized();
    update();
}

void ShotWindow::commitAnnotationSelectionBox()
{
    m_annotationSelectionBoxActive = false;
    setSelectedAnnotations(annotationsInRect(m_annotationSelectionBox));
    m_annotationSelectionBox = {};
    updateAnnotationPropertyPanel();
}

ShotWindow::HistorySnapshot ShotWindow::currentHistorySnapshot() const
{
    return {m_annotations, m_selectedAnnotationId, selectedAnnotationIds(), m_nextNumber, m_nextAnnotationId};
}

void ShotWindow::restoreHistorySnapshot(const HistorySnapshot &snapshot)
{
    m_annotations = snapshot.annotations;
    setSelectedAnnotations(snapshot.selectedAnnotationIds.isEmpty()
                               ? (snapshot.selectedAnnotationId.has_value() ? QVector<int>{*snapshot.selectedAnnotationId} : QVector<int>{})
                               : snapshot.selectedAnnotationIds);
    m_nextNumber = snapshot.nextNumber;
    m_nextAnnotationId = snapshot.nextAnnotationId;
    m_draft.reset();
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_annotationHistoryCaptured = false;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::pushHistorySnapshot()
{
    m_undoStack.append(currentHistorySnapshot());
    if (m_undoStack.size() > 100) {
        m_undoStack.removeFirst();
    }
    m_redoStack.clear();
}

void ShotWindow::undoAnnotationEdit()
{
    if (m_undoStack.isEmpty()) {
        return;
    }
    const HistorySnapshot current = currentHistorySnapshot();
    const HistorySnapshot previous = m_undoStack.takeLast();
    m_redoStack.append(current);
    restoreHistorySnapshot(previous);
}

qreal ShotWindow::currentToolWidth() const
{
    switch (m_tool) {
    case Tool::Move:
    case Tool::Select:
        return m_shapeWidth;
    case Tool::Pen:
        return m_penWidth;
    case Tool::Highlighter:
        return m_penWidth * 2.0;
    case Tool::Line:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Text:
    case Tool::Magnifier:
        return m_shapeWidth;
    case Tool::Number:
        return m_numberWidth;
    case Tool::Mosaic:
        return m_mosaicBlockSize;
    case Tool::Laser:
        return m_laserWidth;
    }

    return m_shapeWidth;
}

qreal ShotWindow::currentToolPreviewSize() const
{
    const qreal scale = annotationSizeScale(true);

    switch (m_tool) {
    case Tool::Move:
    case Tool::Select:
        return 8.0;
    case Tool::Pen:
    case Tool::Line:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Magnifier:
        return std::max<qreal>(1.5, currentToolWidth() * scale);
    case Tool::Highlighter:
        return std::max<qreal>(6.0, currentToolWidth() * scale);
    case Tool::Text:
        return std::max<qreal>(10.0, (19.0 + currentToolWidth()) * scale);
    case Tool::Number:
        return std::max<qreal>(26.0, (13.0 + currentToolWidth() * 1.35) * scale * 2.0);
    case Tool::Mosaic:
        return std::max<qreal>(2.0, currentToolWidth() * scale);
    case Tool::Laser:
        return std::max<qreal>(8.0, currentToolWidth() * scale);
    }

    return std::max<qreal>(1.5, currentToolWidth() * scale);
}

void ShotWindow::setCurrentColor(QColor color)
{
    if (!color.isValid()) {
        return;
    }

    m_currentColor = color;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (m_tool == Tool::Select && !selectedIds.isEmpty()) {
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color = color;
            }
        }
        updateAnnotationPropertyPanel();
    }
    if (m_draft.has_value()) {
        m_draft->color = color;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
        m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(m_currentColor, m_textBackgroundColor, qRound(20.0 + m_shapeWidth)));
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::revealSelectionInfo()
{
    m_showSelectionInfo = true;
    m_selectionInfoTimer.restart();
    QTimer::singleShot(1000, this, [this] {
        if (m_selectionDrag == SelectionDrag::None
            && m_selectionInfoTimer.isValid()
            && m_selectionInfoTimer.elapsed() >= 1000) {
            m_showSelectionInfo = false;
            update();
        }
    });
}

QRectF ShotWindow::normalizedSelection() const
{
    return m_selection.normalized().intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QRect ShotWindow::selectionGlobalRect() const
{
    if (!hasUsableSelection()) {
        return {};
    }

    const QRect sourceBounds(QPoint(0, 0), m_frozenFrame.size());
    const QRectF selection = normalizedSelection();
    QRect selectionRect = selection.toAlignedRect().intersected(sourceBounds);
    if (selectionRect.isEmpty()) {
        return {};
    }

    if (m_sourceGeometry.isValid() && !m_sourceGeometry.isEmpty()) {
        const QSize imageSize = m_frozenFrame.size();
        if (imageSize.width() <= 0 || imageSize.height() <= 0) {
            return {};
        }

        // Wayland captures can be in output pixels while backend geometry is
        // logical. Map the image-space selection back before follow-up captures
        // and extension geometry placeholders use it.
        selectionRect = markshot::capture::geometryFromImageRect(selectionRect,
                                                                 m_sourceGeometry,
                                                                 imageSize);
        if (selectionRect.isEmpty()) {
            return {};
        }
    }
    return selectionRect;
}

QString ShotWindow::slurpSelectionGeometry() const
{
    const QRect selectionRect = selectionGlobalRect();
    if (selectionRect.isEmpty()) {
        return {};
    }
    return slurpGeometry(selectionRect);
}

QPointF ShotWindow::widgetToImage(QPointF point) const
{
    if (m_frozenImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return {};
    }

    const qreal x = (point.x() - m_frozenImageRect.left()) * m_frozenFrame.width() / m_frozenImageRect.width();
    const qreal y = (point.y() - m_frozenImageRect.top()) * m_frozenFrame.height() / m_frozenImageRect.height();
    return clampImagePoint({x, y});
}

QPointF ShotWindow::imageToWidget(QPointF point) const
{
    if (m_frozenImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return {};
    }

    const qreal x = m_frozenImageRect.left() + point.x() * m_frozenImageRect.width() / m_frozenFrame.width();
    const qreal y = m_frozenImageRect.top() + point.y() * m_frozenImageRect.height() / m_frozenFrame.height();
    return {x, y};
}

QPointF ShotWindow::clampImagePoint(QPointF point) const
{
    return {
        std::clamp(point.x(), 0.0, static_cast<qreal>(std::max(0, m_frozenFrame.width() - 1))),
        std::clamp(point.y(), 0.0, static_cast<qreal>(std::max(0, m_frozenFrame.height() - 1))),
    };
}

QString ShotWindow::currentToolName() const
{
    switch (m_tool) {
    case Tool::Move:
        return QStringLiteral("Move");
    case Tool::Select:
        return QStringLiteral("Select");
    case Tool::Pen:
        return QStringLiteral("Pen");
    case Tool::Line:
        return QStringLiteral("Line");
    case Tool::Highlighter:
        return QStringLiteral("Highlighter");
    case Tool::Rectangle:
        return QStringLiteral("Rect");
    case Tool::Ellipse:
        return QStringLiteral("Ellipse");
    case Tool::Arrow:
        return QStringLiteral("Arrow");
    case Tool::Text:
        return QStringLiteral("Text");
    case Tool::Number:
        return QStringLiteral("Number");
    case Tool::Mosaic:
        return QStringLiteral("Mosaic");
    case Tool::Magnifier:
        return QStringLiteral("Magnifier");
    case Tool::Laser:
        return QStringLiteral("Laser");
    }

    return QStringLiteral("Tool");
}

ShotWindow::Tool ShotWindow::defaultEditingTool() const
{
    const Tool tool = m_fullscreenAnnotation ? m_fullscreenDefaultTool : m_defaultTool;
    if (m_fullscreenAnnotation && tool == Tool::Move) {
        return Tool::Select;
    }
    return tool;
}

QImage ShotWindow::mosaicImage(QRect sourceRect, int blockSize) const
{
    sourceRect = sourceRect.normalized().intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
    if (sourceRect.isEmpty()) {
        return {};
    }

    blockSize = std::clamp(blockSize, 2, 96);
    const QImage source = m_frozenFrame.copy(sourceRect).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QImage output(source.size(), QImage::Format_ARGB32_Premultiplied);
    output.fill(Qt::transparent);

    QPainter blockPainter(&output);
    blockPainter.setPen(Qt::NoPen);
    blockPainter.setRenderHint(QPainter::Antialiasing, false);

    for (int y = 0; y < source.height(); y += blockSize) {
        const int blockHeight = std::min(blockSize, source.height() - y);
        for (int x = 0; x < source.width(); x += blockSize) {
            const int blockWidth = std::min(blockSize, source.width() - x);
            quint64 red = 0;
            quint64 green = 0;
            quint64 blue = 0;
            quint64 alpha = 0;
            for (int py = y; py < y + blockHeight; ++py) {
                const QRgb *line = reinterpret_cast<const QRgb *>(source.constScanLine(py));
                for (int px = x; px < x + blockWidth; ++px) {
                    const QRgb pixel = line[px];
                    red += qRed(pixel);
                    green += qGreen(pixel);
                    blue += qBlue(pixel);
                    alpha += qAlpha(pixel);
                }
            }

            const int count = blockWidth * blockHeight;
            QColor average(qRound(static_cast<double>(red) / count),
                           qRound(static_cast<double>(green) / count),
                           qRound(static_cast<double>(blue) / count),
                           qRound(static_cast<double>(alpha) / count));
            blockPainter.setBrush(average);
            blockPainter.drawRect(QRect(x, y, blockWidth, blockHeight));
        }
    }

    blockPainter.end();
    return output;
}

QRectF ShotWindow::imageRectToWidget(QRectF rect) const
{
    const QPointF topLeft = imageToWidget(rect.topLeft());
    const QPointF bottomRight = imageToWidget(rect.bottomRight());
    return QRectF(topLeft, bottomRight).normalized();
}

QPointF ShotWindow::clampedMagnifierCircleCenter(QPointF center, qreal diameter) const
{
    const qreal radius = std::max<qreal>(0.0, diameter / 2.0);
    const qreal frameWidth = m_frozenFrame.width();
    const qreal frameHeight = m_frozenFrame.height();
    if (frameWidth <= diameter) {
        center.setX(frameWidth / 2.0);
    } else {
        center.setX(std::clamp(center.x(), radius, frameWidth - radius));
    }
    if (frameHeight <= diameter) {
        center.setY(frameHeight / 2.0);
    } else {
        center.setY(std::clamp(center.y(), radius, frameHeight - radius));
    }
    return center;
}

QRectF ShotWindow::magnifierCircleRect(QPointF center, qreal diameter) const
{
    const QPointF clampedCenter = clampedMagnifierCircleCenter(center, diameter);
    return QRectF(clampedCenter.x() - diameter / 2.0,
                  clampedCenter.y() - diameter / 2.0,
                  diameter,
                  diameter);
}

QRectF ShotWindow::magnifierSourceRect(const Annotation &annotation) const
{
    const QRectF lensRect = annotation.rect.normalized();
    if (lensRect.isEmpty()) {
        return {};
    }

    const qreal diameter = std::min(lensRect.width(), lensRect.height())
        / clampedMagnifierScale(annotation.magnifierScale);
    const QPointF requestedCenter = annotation.points.isEmpty()
        ? lensRect.center()
        : annotation.points.first();
    return magnifierCircleRect(requestedCenter, diameter);
}

QRectF ShotWindow::textContentRect(const Annotation &annotation, bool widgetCoordinates) const
{
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const QRectF baseRect = annotation.rect.isEmpty()
        ? QRectF(annotation.points.value(0), QSizeF(360.0, 140.0))
        : annotation.rect.normalized();
    const QPointF topLeft = widgetCoordinates ? imageToWidget(baseRect.topLeft()) : baseRect.topLeft();
    const qreal wrapWidth = std::max<qreal>(16.0, baseRect.width() * scale - kTextBackgroundPaddingX * 2.0 * scale);

    QFont font = markshot::theme::textFont(qRound((19.0 + annotation.width) * scale),
                                           QFont::DemiBold,
                                           annotation.fontFamily);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    option.setAlignment(Qt::AlignLeft | Qt::AlignTop);

    QTextDocument document;
    document.setDocumentMargin(0.0);
    document.setDefaultFont(font);
    document.setDefaultTextOption(option);
    document.setPlainText(annotation.text);
    document.setTextWidth(wrapWidth);

    const QSizeF documentSize = document.size();
    qreal textWidth = 0.0;
    qreal textHeight = 0.0;
    for (QTextBlock block = document.begin(); block.isValid(); block = block.next()) {
        const QTextLayout *layout = block.layout();
        if (!layout) {
            continue;
        }
        for (int i = 0; i < layout->lineCount(); ++i) {
            const QTextLine line = layout->lineAt(i);
            textWidth = std::max(textWidth, line.naturalTextWidth());
            textHeight = std::max(textHeight, layout->position().y() + line.y() + line.height());
        }
    }
    if (textWidth <= 0.0 || textHeight <= 0.0) {
        textWidth = documentSize.width();
        textHeight = documentSize.height();
    }

    const qreal rectWidth = std::max<qreal>(1.0, std::ceil(textWidth + kTextBackgroundPaddingX * 2.0 * scale));
    const qreal rectHeight = std::max<qreal>(1.0, std::ceil(textHeight + kTextBackgroundPaddingY * 2.0 * scale));
    return QRectF(topLeft, QSizeF(rectWidth, rectHeight));
}

QRectF ShotWindow::constrainedRect(QPointF start, QPointF end) const
{
    const qreal dx = end.x() - start.x();
    const qreal dy = end.y() - start.y();
    const qreal side = std::max(std::abs(dx), std::abs(dy));
    const QPointF constrainedEnd(start.x() + std::copysign(side, dx == 0.0 ? 1.0 : dx),
                                 start.y() + std::copysign(side, dy == 0.0 ? 1.0 : dy));
    return normalizedRect(start, clampImagePoint(constrainedEnd));
}

void ShotWindow::updateFrozenImageRect()
{
    if (m_frozenFrame.isNull()) {
        m_frozenImageRect = {};
        m_imageCenterInitialized = false;
        return;
    }

    QSizeF frameSize = m_frozenFrame.size();
    frameSize.scale(size(), Qt::KeepAspectRatio);
    if (!imageNavigationAvailable()) {
        const QPointF topLeft((width() - frameSize.width()) / 2.0, (height() - frameSize.height()) / 2.0);
        m_frozenImageRect = QRectF(topLeft, frameSize);
        return;
    }

    const qreal fitScale = frameSize.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    const qreal scale = fitScale * m_imageZoom;
    frameSize = QSizeF(m_frozenFrame.width() * scale, m_frozenFrame.height() * scale);
    if (!m_imageCenterInitialized) {
        m_imageCenter = QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
        m_imageCenterInitialized = true;
    }

    if (frameSize.width() <= width()) {
        m_imageCenter.setX(m_frozenFrame.width() / 2.0);
    } else {
        const qreal halfVisibleWidth = width() / (2.0 * scale);
        m_imageCenter.setX(std::clamp(m_imageCenter.x(), halfVisibleWidth, m_frozenFrame.width() - halfVisibleWidth));
    }
    if (frameSize.height() <= height()) {
        m_imageCenter.setY(m_frozenFrame.height() / 2.0);
    } else {
        const qreal halfVisibleHeight = height() / (2.0 * scale);
        m_imageCenter.setY(std::clamp(m_imageCenter.y(), halfVisibleHeight, m_frozenFrame.height() - halfVisibleHeight));
    }

    const QPointF widgetCenter(width() / 2.0, height() / 2.0);
    const QPointF topLeft(widgetCenter.x() - m_imageCenter.x() * scale,
                          widgetCenter.y() - m_imageCenter.y() * scale);
    m_frozenImageRect = QRectF(topLeft, frameSize);
}

void ShotWindow::zoomImageAt(qreal factor, QPointF widgetAnchor)
{
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty() || factor <= 0.0) {
        return;
    }

    const QPointF anchorImage = m_frozenImageRect.contains(widgetAnchor)
        ? widgetToImage(widgetAnchor)
        : (m_imageCenterInitialized ? m_imageCenter : QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0));
    m_imageZoom = std::clamp(m_imageZoom * factor, kMinImageZoom, kMaxImageZoom);

    QSizeF fitSize = m_frozenFrame.size();
    fitSize.scale(size(), Qt::KeepAspectRatio);
    const qreal scale = fitSize.width() / std::max<qreal>(1.0, m_frozenFrame.width()) * m_imageZoom;
    const QPointF widgetCenter(width() / 2.0, height() / 2.0);
    m_imageCenter = QPointF(anchorImage.x() - (widgetAnchor.x() - widgetCenter.x()) / scale,
                            anchorImage.y() - (widgetAnchor.y() - widgetCenter.y()) / scale);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::resetImageZoom()
{
    if (m_frozenFrame.isNull()) {
        return;
    }

    m_imageZoom = 1.0;
    m_imageCenter = QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::panImageTo(QPointF widgetPosition)
{
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        return;
    }

    const QPointF delta = widgetPosition - m_imagePanStartWidget;
    m_imageCenter = m_imagePanStartCenter - delta / scale;
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::updateImageScrollBars()
{
    if (!m_horizontalImageScrollBar || !m_verticalImageScrollBar) {
        return;
    }
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        m_horizontalImageScrollBar->hide();
        m_verticalImageScrollBar->hide();
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        m_horizontalImageScrollBar->hide();
        m_verticalImageScrollBar->hide();
        return;
    }

    const int scaledWidth = qRound(m_frozenFrame.width() * scale);
    const int scaledHeight = qRound(m_frozenFrame.height() * scale);
    const int maxX = std::max(0, scaledWidth - width());
    const int maxY = std::max(0, scaledHeight - height());
    const bool showHorizontal = maxX > 0;
    const bool showVertical = maxY > 0;

    const int horizontalWidth = std::max(0, width() - (showVertical ? kImageScrollBarExtent : 0));
    const int verticalHeight = std::max(0, height() - (showHorizontal ? kImageScrollBarExtent : 0));
    m_horizontalImageScrollBar->setGeometry(0,
                                            height() - kImageScrollBarExtent,
                                            horizontalWidth,
                                            kImageScrollBarExtent);
    m_verticalImageScrollBar->setGeometry(width() - kImageScrollBarExtent,
                                          0,
                                          kImageScrollBarExtent,
                                          verticalHeight);

    const QSignalBlocker blockHorizontal(m_horizontalImageScrollBar);
    const QSignalBlocker blockVertical(m_verticalImageScrollBar);
    m_syncingImageScrollBars = true;
    m_horizontalImageScrollBar->setRange(0, maxX);
    m_horizontalImageScrollBar->setPageStep(std::max(1, width()));
    m_horizontalImageScrollBar->setSingleStep(std::max(1, width() / 12));
    m_horizontalImageScrollBar->setValue(std::clamp(qRound(-m_frozenImageRect.left()), 0, maxX));
    m_verticalImageScrollBar->setRange(0, maxY);
    m_verticalImageScrollBar->setPageStep(std::max(1, height()));
    m_verticalImageScrollBar->setSingleStep(std::max(1, height() / 12));
    m_verticalImageScrollBar->setValue(std::clamp(qRound(-m_frozenImageRect.top()), 0, maxY));
    m_syncingImageScrollBars = false;

    m_horizontalImageScrollBar->setVisible(showHorizontal);
    m_verticalImageScrollBar->setVisible(showVertical);
    if (showHorizontal) {
        m_horizontalImageScrollBar->raise();
    }
    if (showVertical) {
        m_verticalImageScrollBar->raise();
    }
}

void ShotWindow::setImageCenterFromScrollBars()
{
    if (m_syncingImageScrollBars || !imageNavigationAvailable()
        || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        return;
    }

    const bool hasHorizontal = m_horizontalImageScrollBar && m_horizontalImageScrollBar->maximum() > 0;
    const bool hasVertical = m_verticalImageScrollBar && m_verticalImageScrollBar->maximum() > 0;
    const qreal centerX = hasHorizontal
        ? (m_horizontalImageScrollBar->value() + width() / 2.0) / scale
        : m_frozenFrame.width() / 2.0;
    const qreal centerY = hasVertical
        ? (m_verticalImageScrollBar->value() + height() / 2.0) / scale
        : m_frozenFrame.height() / 2.0;

    m_imageCenter = QPointF(centerX, centerY);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::updateMinimumImageWindowSize()
{
    if (!m_imageNavigationEnabled || !m_toolbar) {
        setMinimumSize(QSize(0, 0));
        return;
    }

    m_toolbar->adjustSize();
    const int minWidth = m_toolbar->sizeHint().width() + kImageWindowMinimumToolbarPadding;
    setMinimumWidth(minWidth);
    if (width() < minWidth) {
        resize(minWidth, height());
    }
}

void ShotWindow::refreshViewGeometry()
{
    updateTextEditorGeometry();
    updateImageScrollBars();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
}

QRect ShotWindow::clampedToolbarGeometry(QRect toolbarGeometry) const
{
    toolbarGeometry.moveLeft(std::clamp(toolbarGeometry.left(), 8, std::max(8, width() - toolbarGeometry.width() - 8)));
    toolbarGeometry.moveTop(std::clamp(toolbarGeometry.top(), 8, std::max(8, height() - toolbarGeometry.height() - 8)));
    return toolbarGeometry;
}

void ShotWindow::updateToolbarGeometry()
{
    if (!m_toolbar || !hasUsableSelection()) {
        return;
    }

    m_toolbar->adjustSize();
    if (m_fullscreenAnnotation && m_toolbarUserPlaced) {
        const QSize toolbarSize = m_toolbar->sizeHint();
        QRect toolbarGeometry = m_toolbar->geometry();
        toolbarGeometry.setSize(toolbarSize);
        m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }
    if (m_imageNavigationEnabled && m_fullscreenAnnotation) {
        const QSize toolbarSize = m_toolbar->sizeHint();
        const QRect toolbarGeometry(QPoint(qRound((width() - toolbarSize.width()) / 2.0), 12), toolbarSize);
        m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_toolbar->sizeHint();
    int x = qRound(selection.center().x() - toolbarSize.width() / 2.0);
    int y = qRound(selection.bottom() + kToolbarMargin);

    x = std::clamp(x, 8, std::max(8, width() - toolbarSize.width() - 8));
    if (y + toolbarSize.height() > height() - 8) {
        y = qRound(selection.top() - toolbarSize.height() - kToolbarMargin);
    }
    y = std::clamp(y, 8, std::max(8, height() - toolbarSize.height() - 8));
    m_toolbar->setGeometry(x, y, toolbarSize.width(), toolbarSize.height());
    updateAnnotationPropertyPanelGeometry();
}

void ShotWindow::updateActionToolbarGeometry()
{
    if (!m_actionToolbar || !hasUsableSelection() || m_fullscreenAnnotation) {
        return;
    }

    m_actionToolbar->adjustSize();
    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_actionToolbar->sizeHint();
    const QRect selectionRect = selection.toAlignedRect();
    const QRect toolbarRect = m_toolbar && m_toolbar->isVisible() ? m_toolbar->geometry() : QRect();
    const QRect propertyRect = m_annotationPropertyPanel && m_annotationPropertyPanel->isVisible()
        ? m_annotationPropertyPanel->geometry()
        : QRect();

    auto clamped = [this, toolbarSize](QPoint topLeft) {
        const int x = std::clamp(topLeft.x(), 8, std::max(8, width() - toolbarSize.width() - 8));
        const int y = std::clamp(topLeft.y(), 8, std::max(8, height() - toolbarSize.height() - 8));
        return QRect(QPoint(x, y), toolbarSize);
    };
    auto clearOfPanels = [toolbarRect, propertyRect, selectionRect](const QRect &candidate) {
        const QRect padded = candidate.adjusted(-4, -4, 4, 4);
        return (toolbarRect.isNull() || !padded.intersects(toolbarRect))
            && (propertyRect.isNull() || !padded.intersects(propertyRect))
            && !padded.intersects(selectionRect);
    };

    const int selectionCenterY = qRound(selection.center().y() - toolbarSize.height() / 2.0);
    QVector<QRect> candidates = {
        clamped(QPoint(qRound(selection.right() + kToolbarMargin), selectionCenterY)),
        clamped(QPoint(qRound(selection.left() - toolbarSize.width() - kToolbarMargin), selectionCenterY)),
    };
    if (!toolbarRect.isNull()) {
        candidates.append(clamped(QPoint(toolbarRect.right() + kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.left() - toolbarSize.width() - kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.bottom() + kToolbarMargin)));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.top() - toolbarSize.height() - kToolbarMargin)));
    }

    for (const QRect &candidate : candidates) {
        if (clearOfPanels(candidate)) {
            m_actionToolbar->setGeometry(candidate);
            return;
        }
    }
    m_actionToolbar->setGeometry(candidates.first());
}

void ShotWindow::updateAnnotationPropertyPanel()
{
    if (!m_annotationPropertyPanel) {
        return;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    const Annotation *annotation = selectedIds.size() == 1
        ? annotationById(selectedIds.first())
        : nullptr;
    const Annotation *firstSelectedAnnotation = !selectedIds.isEmpty()
        ? annotationById(selectedIds.first())
        : nullptr;
    const bool groupSelection = m_tool == Tool::Select && selectedIds.size() > 1;
    const bool editingAnnotation = m_tool == Tool::Select && !selectedIds.isEmpty();
    const bool editingTool = m_mode == Mode::Editing
        && m_tool != Tool::Move
        && m_tool != Tool::Select;
    if (!editingAnnotation && !editingTool) {
        m_annotationPropertyPanel->hide();
        if (m_propertyColorDialogPanel) {
            m_propertyColorDialogPanel->hide();
        }
        if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
        }
        return;
    }

    QString title = QStringLiteral("Object");
    const Tool panelTool = groupSelection ? Tool::Select : (annotation ? annotation->tool : m_tool);
    const QColor panelColor = firstSelectedAnnotation ? firstSelectedAnnotation->color : m_currentColor;
    const QColor panelTextBackgroundColor = annotation && annotation->tool == Tool::Text
        ? annotation->backgroundColor
        : m_textBackgroundColor;
    const qreal panelWidth = firstSelectedAnnotation ? firstSelectedAnnotation->width : currentToolWidth();
    const int panelOpacity = qRound(panelColor.alphaF() * 100.0);
    const bool panelFilled = annotation ? annotation->filled : m_shapeFilled;
    const qreal panelRadius = annotation ? annotation->cornerRadius : m_rectangleCornerRadius;
    const qreal panelMagnifierScale =
        annotation && annotation->tool == Tool::Magnifier
            ? annotation->magnifierScale
            : m_magnifierScale;
    const HighlighterStyle panelHighlighterStyle =
        annotation && annotation->tool == Tool::Highlighter
            ? annotation->highlighterStyle
            : m_highlighterStyle;
    const QString panelFontFamily = annotation ? annotation->fontFamily : m_textFontFamily;

    switch (panelTool) {
    case Tool::Move:
    case Tool::Select:
        title = QStringLiteral("Object");
        break;
    case Tool::Pen:
        title = QStringLiteral("Pen");
        break;
    case Tool::Highlighter:
        title = QStringLiteral("Highlighter");
        break;
    case Tool::Line:
        title = QStringLiteral("Line");
        break;
    case Tool::Rectangle:
        title = QStringLiteral("Rect");
        break;
    case Tool::Ellipse:
        title = QStringLiteral("Ellipse");
        break;
    case Tool::Arrow:
        title = QStringLiteral("Arrow");
        break;
    case Tool::Text:
        title = QStringLiteral("Text");
        break;
    case Tool::Number:
        title = QStringLiteral("Number");
        break;
    case Tool::Mosaic:
        title = QStringLiteral("Mosaic");
        break;
    case Tool::Magnifier:
        title = QStringLiteral("Magnifier");
        break;
    case Tool::Laser:
        title = QStringLiteral("Laser");
        break;
    }

    if (m_annotationPropertyTitle) {
        m_annotationPropertyTitle->setText(groupSelection
                                               ? MS_TR("Group %1").arg(selectedIds.size())
                                               : markshot::i18n::translate(title));
    }
    if (m_propertyEditTextButton) {
        m_propertyEditTextButton->setVisible(!groupSelection && editingAnnotation && panelTool == Tool::Text);
    }
    if (m_propertyFontButton) {
        m_propertyFontButton->setVisible(!groupSelection && panelTool == Tool::Text);
        if (!groupSelection && panelTool == Tool::Text) {
            const QString family = panelFontFamily.isEmpty() ? markshot::theme::textFontFamily() : panelFontFamily;
            m_propertyFontButton->setToolTip(family);
            if (m_propertyFontList) {
                const auto matches = m_propertyFontList->findItems(family, Qt::MatchExactly);
                if (!matches.isEmpty()) {
                    m_propertyFontList->setCurrentItem(matches.first());
                    m_propertyFontList->scrollToItem(matches.first(), QAbstractItemView::PositionAtCenter);
                }
            }
        } else if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
            m_propertyFontButton->setToolTip(MS_TR("Text font"));
        }
    }
    if (m_propertyFillButton) {
        const bool supportsFill = !groupSelection && (panelTool == Tool::Rectangle || panelTool == Tool::Ellipse);
        m_propertyFillButton->setVisible(supportsFill);
        const QSignalBlocker blocker(m_propertyFillButton);
        m_propertyFillButton->setChecked(panelFilled);
        m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(panelFilled));
    }
    if (m_propertyRadiusGlyphLabel) {
        m_propertyRadiusGlyphLabel->setVisible(!groupSelection && panelTool == Tool::Rectangle);
    }
    if (m_propertyRadiusLabel) {
        m_propertyRadiusLabel->setVisible(!groupSelection && panelTool == Tool::Rectangle);
        m_propertyRadiusLabel->setText(QString::number(qRound(panelRadius)));
    }
    if (m_propertyRadiusSlider) {
        m_propertyRadiusSlider->setVisible(!groupSelection && panelTool == Tool::Rectangle);
        const QSignalBlocker blocker(m_propertyRadiusSlider);
        m_propertyRadiusSlider->setValue(qRound(panelRadius));
    }
    if (m_propertyArrowStyleCombo) {
        const bool supportsArrowStyle = !groupSelection && panelTool == Tool::Arrow;
        m_propertyArrowStyleCombo->setVisible(supportsArrowStyle);
        if (supportsArrowStyle) {
            const ArrowStyle panelArrowStyle = annotation ? annotation->arrowStyle : m_arrowStyle;
            const QSignalBlocker blocker(m_propertyArrowStyleCombo);
            m_propertyArrowStyleCombo->setCurrentIndex(panelArrowStyle == ArrowStyle::Kde ? 1 : 0);
        }
    }
    if (m_propertyHighlighterStyleCombo) {
        const bool supportsHighlighterStyle = !groupSelection && panelTool == Tool::Highlighter;
        m_propertyHighlighterStyleCombo->setVisible(supportsHighlighterStyle);
        if (supportsHighlighterStyle) {
            const QSignalBlocker blocker(m_propertyHighlighterStyleCombo);
            m_propertyHighlighterStyleCombo->setCurrentIndex(
                panelHighlighterStyle == HighlighterStyle::StraightLine ? 1 : 0);
        }
    }
    const bool supportsMagnifierScale = !groupSelection && panelTool == Tool::Magnifier;
    if (m_propertyMagnifierScaleGlyphLabel) {
        m_propertyMagnifierScaleGlyphLabel->setVisible(supportsMagnifierScale);
    }
    if (m_propertyMagnifierScaleLabel) {
        m_propertyMagnifierScaleLabel->setVisible(supportsMagnifierScale);
        m_propertyMagnifierScaleLabel->setText(magnifierScaleText(panelMagnifierScale));
    }
    if (m_propertyMagnifierScaleSlider) {
        m_propertyMagnifierScaleSlider->setVisible(supportsMagnifierScale);
        const QSignalBlocker blocker(m_propertyMagnifierScaleSlider);
        m_propertyMagnifierScaleSlider->setValue(magnifierScaleSliderValue(panelMagnifierScale));
    }
    if (m_propertyWidthLabel) {
        m_propertyWidthLabel->setText(QString::number(qRound(panelWidth)));
    }
    if (m_propertyWidthSlider) {
        const QSignalBlocker blocker(m_propertyWidthSlider);
        if (panelTool == Tool::Mosaic) {
            m_propertyWidthSlider->setRange(qRound(kMinMosaicBlockSize), qRound(kMaxMosaicBlockSize));
        } else if (panelTool == Tool::Number) {
            m_propertyWidthSlider->setRange(qRound(kMinNumberWidth), qRound(kMaxNumberWidth));
        } else if (panelTool == Tool::Laser) {
            m_propertyWidthSlider->setRange(qRound(kMinLaserWidth), qRound(kMaxLaserWidth));
        } else if (panelTool == Tool::Text) {
            m_propertyWidthSlider->setRange(1, 1000);
        } else {
            m_propertyWidthSlider->setRange(qRound(kMinStrokeWidth), qRound(kMaxStrokeWidth));
        }
        m_propertyWidthSlider->setValue(qRound(panelWidth));
    }
    if (m_propertyOpacityLabel) {
        m_propertyOpacityLabel->setText(QStringLiteral("%1%").arg(panelOpacity));
    }
    if (m_propertyOpacitySlider) {
        const QSignalBlocker blocker(m_propertyOpacitySlider);
        m_propertyOpacitySlider->setValue(panelOpacity);
    }
    if (m_propertyColorButton) {
        m_propertyColorButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelColor));
        m_propertyColorButton->setIcon(markshot::ui::makePropertyIcon(
            markshot::ui::PropertyIcon::Color, propertyIconInkForFill(panelColor)));
        m_propertyColorButton->setVisible(panelTool != Tool::Mosaic);
        if (panelTool == Tool::Mosaic && m_propertyColorDialogPanel) {
            m_propertyColorDialogPanel->hide();
        }
    }
    if (m_propertyTextBackgroundButton) {
        const bool supportsTextBackground = !groupSelection && panelTool == Tool::Text;
        m_propertyTextBackgroundButton->setVisible(supportsTextBackground);
        m_propertyTextBackgroundButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelTextBackgroundColor));
        m_propertyTextBackgroundButton->setIcon(markshot::ui::makePropertyIcon(
            markshot::ui::PropertyIcon::TextBackground, propertyIconInkForFill(panelTextBackgroundColor)));
        if (!supportsTextBackground && m_propertyColorDialogPanel && m_propertyColorEditingTextBackground) {
            m_propertyColorDialogPanel->hide();
        }
    }
    if (m_propertyColorPicker && m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(m_propertyColorEditingTextBackground ? panelTextBackgroundColor : panelColor);
    }

    m_annotationPropertyPanel->show();
    if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
        panelLayout->activate();
    }
    updateAnnotationPropertyPanelGeometry();
    m_annotationPropertyPanel->raise();
    if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        updatePropertyColorDialogGeometry();
        m_propertyColorDialogPanel->raise();
    }
}

void ShotWindow::updateAnnotationPropertyPanelGeometry()
{
    if (!m_annotationPropertyPanel) {
        return;
    }

    m_annotationPropertyPanel->adjustSize();
    const QSize panelSize = m_annotationPropertyPanel->sizeHint();
    const QRect toolbarRect = m_toolbar && m_toolbar->isVisible()
        ? m_toolbar->geometry()
        : QRect(8, 8, 0, 0);
    int x = toolbarRect.left();
    int y = toolbarRect.bottom() + kToolbarMargin;
    if (y + panelSize.height() > height() - 8) {
        y = toolbarRect.top() - panelSize.height() - kToolbarMargin;
    }
    if (x + panelSize.width() > width() - 8) {
        x = toolbarRect.right() - panelSize.width();
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_annotationPropertyPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
    updatePropertyColorDialogGeometry();
    updatePropertyFontPanelGeometry();
}

void ShotWindow::updatePropertyColorDialogGeometry()
{
    if (!m_propertyColorDialogPanel || !m_annotationPropertyPanel) {
        return;
    }

    m_propertyColorDialogPanel->adjustSize();
    QSize panelSize = m_propertyColorDialogPanel->sizeHint();
    panelSize.setWidth(std::min(panelSize.width(), std::max(160, width() - 16)));
    panelSize.setHeight(std::min(panelSize.height(), std::max(180, height() - 16)));

    // Anchor on the color button's centre so the picker stays put when the
    // property panel resizes (e.g. fill/radius/font slots showing or hiding).
    // Falling back to the property panel keeps geometry valid when the
    // button is hidden (mosaic case).
    QPoint anchor;
    if (m_propertyColorEditingTextBackground && m_propertyTextBackgroundButton && m_propertyTextBackgroundButton->isVisible()) {
        anchor = m_propertyTextBackgroundButton->mapTo(this, m_propertyTextBackgroundButton->rect().center());
    } else if (m_propertyColorButton && m_propertyColorButton->isVisible()) {
        anchor = m_propertyColorButton->mapTo(this, m_propertyColorButton->rect().center());
    } else {
        const QRect propertyRect = m_annotationPropertyPanel->geometry();
        anchor = QPoint(propertyRect.center().x(), propertyRect.bottom());
    }

    int x = anchor.x() - panelSize.width() / 2;
    int y = anchor.y() + 14;

    const QRect propertyRect = m_annotationPropertyPanel->geometry();
    if (y + panelSize.height() > height() - 8) {
        // Place above the property panel instead.
        y = propertyRect.top() - panelSize.height() - 8;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_propertyColorDialogPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::updatePropertyFontPanelGeometry()
{
    if (!m_propertyFontPanel || !m_annotationPropertyPanel || !m_propertyFontButton) {
        return;
    }

    const int visibleRows = std::min(10, m_propertyFontList ? std::max(1, m_propertyFontList->count()) : 1);
    const int rowHeight = m_propertyFontList ? std::max(24, m_propertyFontList->sizeHintForRow(0)) : 28;
    QSize panelSize(260, std::min(280, visibleRows * rowHeight + 18));
    panelSize.setWidth(std::min(panelSize.width(), std::max(180, width() - 16)));
    panelSize.setHeight(std::min(panelSize.height(), std::max(120, height() - 16)));

    QPoint anchor = m_propertyFontButton->mapTo(this, QPoint(m_propertyFontButton->width() / 2,
                                                            m_propertyFontButton->height()));
    int x = anchor.x() - panelSize.width() / 2;
    int y = anchor.y() + 10;
    const QRect propertyRect = m_annotationPropertyPanel->geometry();
    if (y + panelSize.height() > height() - 8) {
        y = propertyRect.top() - panelSize.height() - 8;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    if (m_propertyFontList) {
        m_propertyFontList->setFixedHeight(std::max(80, panelSize.height() - 16));
    }
    m_propertyFontPanel->setFixedSize(panelSize);
    m_propertyFontPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::adjustSelectedAnnotationWidth(qreal delta)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }

    pushHistorySnapshot();
    for (int id : selectedIds) {
        if (Annotation *annotation = annotationById(id)) {
            if (annotation->tool == Tool::Mosaic) {
                annotation->width = std::clamp(annotation->width + delta * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
            } else if (annotation->tool == Tool::Number) {
                annotation->width = std::clamp(annotation->width + delta * 2.0, kMinNumberWidth, kMaxNumberWidth);
            } else {
                annotation->width = std::clamp(annotation->width + delta, kMinStrokeWidth, kMaxStrokeWidth);
            }
        }
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationWidth(int width)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && qRound(annotation->width) != width) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                if (annotation->tool == Tool::Mosaic) {
                    annotation->width = std::clamp<qreal>(width, kMinMosaicBlockSize, kMaxMosaicBlockSize);
                } else if (annotation->tool == Tool::Number) {
                    annotation->width = std::clamp<qreal>(width, kMinNumberWidth, kMaxNumberWidth);
                } else if (annotation->tool == Tool::Text) {
                    const qreal oldWidth = annotation->width;
                    annotation->width = std::clamp<qreal>(width, 1.0, 1000.0);
                    const qreal factor = ((19.0 + annotation->width) / (19.0 + oldWidth)) * 1.05;
                    annotation->rect.setWidth(annotation->rect.width() * factor);
                    annotation->rect = textContentRect(*annotation, false);
                    if (!annotation->points.isEmpty()) {
                        annotation->points[0] = annotation->rect.topLeft();
                    }
                } else {
                    annotation->width = std::clamp<qreal>(width, kMinStrokeWidth, kMaxStrokeWidth);
                }
            }
        }
    } else {
        switch (m_tool) {
        case Tool::Pen:
        case Tool::Highlighter:
            m_penWidth = width;
            break;
        case Tool::Mosaic:
            m_mosaicBlockSize = width;
            break;
        case Tool::Laser:
            m_laserWidth = std::clamp<qreal>(width, kMinLaserWidth, kMaxLaserWidth);
            break;
        case Tool::Move:
        case Tool::Select:
            return;
        case Tool::Line:
        case Tool::Rectangle:
        case Tool::Ellipse:
        case Tool::Arrow:
        case Tool::Magnifier:
            m_shapeWidth = width;
            break;
        case Tool::Text:
            m_shapeWidth = std::clamp<qreal>(width, 1.0, 1000.0);
            break;
        case Tool::Number:
            m_numberWidth = std::clamp<qreal>(width, kMinNumberWidth, kMaxNumberWidth);
            break;
        }
    }
    updateAnnotationPropertyPanel();
    updateColorPalettePreview();
    update();
}

void ShotWindow::setSelectedAnnotationOpacity(int opacity)
{
    opacity = std::clamp(opacity, 0, 100);
    const int alpha = qRound(opacity * 255.0 / 100.0);
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->color.alpha() != alpha) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color.setAlpha(alpha);
            }
        }
    } else {
        if (m_currentColor.alpha() == alpha) {
            return;
        }
        m_currentColor.setAlpha(alpha);
    }

    if (m_draft.has_value()) {
        m_draft->color.setAlpha(alpha);
    }
    if (m_laserDraft.has_value()) {
        m_laserDraft->color.setAlpha(alpha);
    }
    if (m_propertyColorPicker && m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(selectedIds.isEmpty() ? m_currentColor : annotationById(selectedIds.first())->color);
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationFilled(bool filled)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->filled == filled) {
            return;
        }
        if (annotation->tool != Tool::Rectangle && annotation->tool != Tool::Ellipse) {
            return;
        }
        pushHistorySnapshot();
        annotation->filled = filled;
    } else {
        if (m_tool != Tool::Rectangle && m_tool != Tool::Ellipse) {
            return;
        }
        m_shapeFilled = filled;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationCornerRadius(int radius)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Rectangle || qRound(annotation->cornerRadius) == radius) {
            return;
        }
        pushHistorySnapshot();
        annotation->cornerRadius = radius;
    } else {
        if (m_tool != Tool::Rectangle || qRound(m_rectangleCornerRadius) == radius) {
            return;
        }
        m_rectangleCornerRadius = radius;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationArrowStyle(ArrowStyle style)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Arrow || annotation->arrowStyle == style) {
            return;
        }
        pushHistorySnapshot();
        annotation->arrowStyle = style;
    } else {
        if (m_tool != Tool::Arrow || m_arrowStyle == style) {
            return;
        }
        m_arrowStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedHighlighterStyle(HighlighterStyle style)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Highlighter
                && annotation->highlighterStyle != style) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Highlighter) {
                annotation->highlighterStyle = style;
            }
        }
    } else {
        if (m_tool != Tool::Highlighter || m_highlighterStyle == style) {
            return;
        }
        m_highlighterStyle = style;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Highlighter) {
        m_draft->highlighterStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedMagnifierScale(int scaleValue)
{
    const qreal scale = magnifierScaleFromSliderValue(scaleValue);
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Magnifier
                && !qFuzzyCompare(clampedMagnifierScale(annotation->magnifierScale), scale)) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Magnifier) {
                annotation->magnifierScale = scale;
            }
        }
    } else {
        if (m_tool != Tool::Magnifier || qFuzzyCompare(m_magnifierScale, scale)) {
            return;
        }
        m_magnifierScale = scale;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Magnifier) {
        m_draft->magnifierScale = scale;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::deleteSelectedAnnotation()
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }
    pushHistorySnapshot();
    for (int i = m_annotations.size() - 1; i >= 0; --i) {
        if (selectedIds.contains(m_annotations.at(i).id)) {
            m_annotations.removeAt(i);
        }
    }
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::openSelectedAnnotationColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = false;

    if (m_propertyColorDialogPanel->isVisible() && !wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_currentColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        if (const Annotation *annotation = annotationById(selectedIds.first())) {
            color = annotation->color;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::openSelectedTextBackgroundColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = true;

    if (m_propertyColorDialogPanel->isVisible() && wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_textBackgroundColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() == 1) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotation->tool == Tool::Text) {
            color = annotation->backgroundColor;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::toggleSelectedTextFontPanel()
{
    if (!m_propertyFontPanel || !m_propertyFontList || !m_propertyFontButton) {
        return;
    }

    if (m_propertyFontPanel->isVisible()) {
        m_propertyFontPanel->hide();
        return;
    }

    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    updateAnnotationPropertyPanel();
    if (QLayout *fontLayout = m_propertyFontPanel->layout()) {
        fontLayout->activate();
    }
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->show();
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->raise();
}

void ShotWindow::applyPropertyColor(QColor color)
{
    if (!color.isValid()) {
        return;
    }
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (m_propertyColorEditingTextBackground) {
        if (!selectedIds.isEmpty()) {
            if (!m_propertyColorEditHistoryCaptured) {
                pushHistorySnapshot();
                m_propertyColorEditHistoryCaptured = true;
            }
            for (int id : selectedIds) {
                if (Annotation *annotation = annotationById(id);
                    annotation && annotation->tool == Tool::Text) {
                    annotation->backgroundColor = color;
                }
            }
        } else if (m_tool == Tool::Text) {
            m_textBackgroundColor = color;
        }
    } else if (!selectedIds.isEmpty()) {
        if (!m_propertyColorEditHistoryCaptured) {
            pushHistorySnapshot();
            m_propertyColorEditHistoryCaptured = true;
        }
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color = color;
            }
        }
    } else {
        m_currentColor = color;
    }
    if (m_draft.has_value()) {
        m_draft->color = color;
    }
    if (m_textEditor && m_textEditor->isVisible()) {
        QColor editorColor = m_currentColor;
        QColor editorBackgroundColor = m_textBackgroundColor;
        qreal editorWidth = m_shapeWidth;
        if (m_editingTextAnnotationId.has_value()) {
            if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
                editorColor = annotation->color;
                editorBackgroundColor = annotation->backgroundColor;
                editorWidth = annotation->width;
            }
        }
        m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(editorColor, editorBackgroundColor, qRound(20.0 + editorWidth)));
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::clearAnnotations()
{
    commitTextEditor();
    if (m_annotations.isEmpty() && !m_draft.has_value() && m_laserStrokes.isEmpty() && !m_laserDraft.has_value()) {
        return;
    }

    pushHistorySnapshot();
    m_annotations.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_annotationHistoryCaptured = false;
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::setSelectedTextFontFamily(const QString &fontFamily)
{
    if (fontFamily.isEmpty()) {
        return;
    }

    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Text || annotation->fontFamily == fontFamily) {
            return;
        }
        pushHistorySnapshot();
        annotation->fontFamily = fontFamily;
    } else {
        if (m_tool != Tool::Text || m_textFontFamily == fontFamily) {
            return;
        }
        m_textFontFamily = fontFamily;
        if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
            m_textEditor->setFont(markshot::theme::textFont(qRound(20.0 + m_shapeWidth),
                                                            QFont::DemiBold,
                                                            m_textFontFamily));
        }
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::toggleOpenWithPanel()
{
    commitTextEditor();
    if (!m_openWithPanel || !hasUsableSelection()) {
        return;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }

    if (m_openWithPanel->isVisible()) {
        m_openWithPanel->hide();
        return;
    }

    updateOpenWithPanel();
    updateOpenWithPanelGeometry();
    m_openWithPanel->show();
    m_openWithPanel->raise();
}

void ShotWindow::updateOpenWithPanel()
{
    if (!m_openWithPanel) {
        return;
    }

    QLayout *layout = m_openWithPanel->layout();
    while (QLayoutItem *item = layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            delete widget;
        }
        delete item;
    }

    auto *title = new QLabel(MS_TR("Open with"), m_openWithPanel);
    layout->addWidget(title);

    const QVector<DesktopApp> apps = imageDesktopApps();
    if (apps.isEmpty()) {
        auto *empty = new QLabel(MS_TR("No image desktop entries found"), m_openWithPanel);
        empty->setWordWrap(true);
        layout->addWidget(empty);
        m_openWithPanel->adjustSize();
        return;
    }

    auto *list = new QListWidget(m_openWithPanel);
    list->setFocusPolicy(Qt::NoFocus);
    list->setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    list->setIconSize(QSize(22, 22));
    for (const DesktopApp &app : apps) {
        auto *item = new QListWidgetItem(app.name, list);
        item->setToolTip(app.desktopPath);
        item->setData(Qt::UserRole, app.desktopPath);
        item->setData(Qt::UserRole + 1, app.exec);
        item->setData(Qt::UserRole + 2, app.icon);
        QIcon icon;
        if (!app.icon.isEmpty()) {
            if (app.icon.startsWith(QLatin1Char('/')) && QFile::exists(app.icon)) {
                icon = QIcon(app.icon);
            } else {
                icon = QIcon::fromTheme(app.icon);
            }
        }
        if (!icon.isNull()) {
            item->setIcon(icon);
        }
    }
    list->setFixedHeight(std::min(360, std::max(48, static_cast<int>(apps.size()) * 36)));
    connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }
        DesktopApp app;
        app.name = item->text();
        app.desktopPath = item->data(Qt::UserRole).toString();
        app.exec = item->data(Qt::UserRole + 1).toString();
        app.icon = item->data(Qt::UserRole + 2).toString();
        openSelectionWithDesktop(app);
    });
    layout->addWidget(list);

    m_openWithPanel->adjustSize();
}

void ShotWindow::updateOpenWithPanelGeometry()
{
    if (!m_openWithPanel) {
        return;
    }

    m_openWithPanel->adjustSize();
    const QSize panelSize(std::min(340, std::max(280, m_openWithPanel->sizeHint().width())),
                          std::min(540, std::max(80, m_openWithPanel->sizeHint().height())));
    const QRect toolbarRect = m_fullscreenAnnotation && m_toolbar
        ? m_toolbar->geometry()
        : (m_actionToolbar ? m_actionToolbar->geometry() : QRect(width() - 64, height() / 2 - 80, 56, 160));
    int x = toolbarRect.left() - panelSize.width() - kToolbarMargin;
    int y = toolbarRect.top();
    if (x < 8) {
        x = toolbarRect.right() + kToolbarMargin;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_openWithPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::toggleExtensionPanel()
{
    commitTextEditor();
    if (!m_extensionPanel || !hasUsableSelection()) {
        return;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }

    if (m_extensionPanel->isVisible()) {
        m_extensionPanel->hide();
        return;
    }

    updateExtensionPanel();
    updateExtensionPanelGeometry();
    m_extensionPanel->show();
    m_extensionPanel->raise();
}

void ShotWindow::updateExtensionPanel()
{
    if (!m_extensionPanel) {
        return;
    }

    QLayout *layout = m_extensionPanel->layout();
    while (QLayoutItem *item = layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            delete widget;
        }
        delete item;
    }

    auto *title = new QLabel(MS_TR("Extensions"), m_extensionPanel);
    layout->addWidget(title);

    QString errorMessage;
    const QVector<ExtensionCommand> commands = extensionCommands(&errorMessage);
    if (!errorMessage.isEmpty()) {
        auto *error = new QLabel(errorMessage, m_extensionPanel);
        error->setWordWrap(true);
        layout->addWidget(error);
        m_extensionPanel->adjustSize();
        return;
    }

    if (commands.isEmpty()) {
        auto *empty = new QLabel(MS_TR("No extension commands configured.\nCreate %1").arg(extensionCommandsConfigPath()),
                                 m_extensionPanel);
        empty->setWordWrap(true);
        layout->addWidget(empty);
        m_extensionPanel->adjustSize();
        return;
    }

    auto *list = new QListWidget(m_extensionPanel);
    list->setFocusPolicy(Qt::NoFocus);
    list->setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    for (const ExtensionCommand &command : commands) {
        auto *item = new QListWidgetItem(command.name, list);
        const QString tooltip = command.description.isEmpty()
            ? command.command
            : QStringLiteral("%1\n%2").arg(command.description, command.command);
        item->setToolTip(tooltip);
        item->setData(Qt::UserRole, command.command);
        item->setData(Qt::UserRole + 1, command.workingDirectory);
        item->setData(Qt::UserRole + 2, command.description);
        item->setData(Qt::UserRole + 3, command.saveImage);
        item->setData(Qt::UserRole + 4, command.closeOnStart);
    }
    list->setFixedHeight(std::min(360, std::max(48, static_cast<int>(commands.size()) * 36)));
    connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }

        ExtensionCommand command;
        command.name = item->text();
        command.command = item->data(Qt::UserRole).toString();
        command.workingDirectory = item->data(Qt::UserRole + 1).toString();
        command.description = item->data(Qt::UserRole + 2).toString();
        command.saveImage = item->data(Qt::UserRole + 3).toBool();
        command.closeOnStart = item->data(Qt::UserRole + 4).toBool();
        runExtensionCommand(command);
    });
    layout->addWidget(list);

    m_extensionPanel->adjustSize();
}

void ShotWindow::updateExtensionPanelGeometry()
{
    if (!m_extensionPanel) {
        return;
    }

    m_extensionPanel->adjustSize();
    const QSize panelSize(std::min(380, std::max(300, m_extensionPanel->sizeHint().width())),
                          std::min(540, std::max(80, m_extensionPanel->sizeHint().height())));
    const QRect toolbarRect = m_fullscreenAnnotation && m_toolbar
        ? m_toolbar->geometry()
        : (m_actionToolbar ? m_actionToolbar->geometry() : QRect(width() - 64, height() / 2 - 80, 56, 160));
    int x = toolbarRect.left() - panelSize.width() - kToolbarMargin;
    int y = toolbarRect.top();
    if (x < 8) {
        x = toolbarRect.right() + kToolbarMargin;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_extensionPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::toggleColorPalette(QPoint position)
{
    commitTextEditor();
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (!m_colorPalette) {
        return;
    }

    m_colorPaletteAnchor = position;
    if (m_colorPalette->isVisible()) {
        m_colorPalette->hide();
    } else {
        updateColorPaletteGeometry(position);
        m_colorPalette->show();
        m_colorPalette->raise();
    }
    update();
}

void ShotWindow::updateColorPaletteGeometry(QPoint anchor)
{
    if (!m_colorPalette) {
        return;
    }

    const QSize paletteSize(178, 178);
    int x = anchor.x() - paletteSize.width() / 2;
    int y = anchor.y() - paletteSize.height() / 2;
    x = std::clamp(x, 8, std::max(8, width() - paletteSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - paletteSize.height() - 8));
    m_colorPalette->setGeometry(x, y, paletteSize.width(), paletteSize.height());

    const QPoint center(paletteSize.width() / 2, paletteSize.height() / 2);
    const qreal radius = 68.0;
    const auto buttons = m_colorPalette->findChildren<QPushButton *>(QString(), Qt::FindDirectChildrenOnly);
    for (int i = 0; i < buttons.size(); ++i) {
        const qreal angle = -M_PI / 2.0 + (2.0 * M_PI * i / std::max<qsizetype>(1, buttons.size()));
        const QPoint pos(qRound(center.x() + std::cos(angle) * radius - 15.0),
                         qRound(center.y() + std::sin(angle) * radius - 15.0));
        buttons.at(i)->setGeometry(QRect(pos, QSize(30, 30)));
    }
    updateColorPalettePreview();
}

void ShotWindow::updateColorPalettePreview()
{
    if (!m_colorPalettePreview) {
        return;
    }

    const int size = std::clamp(qRound(currentToolPreviewSize()), 8, 34);
    const QPoint center(89, 89);
    m_colorPalettePreview->setGeometry(center.x() - size / 2, center.y() - size / 2, size, size);
    m_colorPalettePreview->setStyleSheet(QStringLiteral(
        "QWidget#colorPalettePreview {"
        " background: %1;"
        " border: 0;"
        " border-radius: 3px;"
        "}").arg(m_currentColor.name()));
}

void ShotWindow::updateTextEditorGeometry()
{
    if (!m_textEditor || !m_textEditor->isVisible()) {
        return;
    }
    if (m_editingTextAnnotationId.has_value()) {
        if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            QRect editorRect = textContentRect(*annotation, true).toAlignedRect().adjusted(0, 0, 1, 1);
            editorRect.moveLeft(std::clamp(editorRect.left(), 8, std::max(8, width() - editorRect.width() - 8)));
            editorRect.moveTop(std::clamp(editorRect.top(), 8, std::max(8, height() - editorRect.height() - 8)));
            m_textEditor->setGeometry(editorRect);
        }
        return;
    }

    const QPointF topLeft = imageToWidget(m_textEditorImagePoint);
    const QRectF selection = imageRectToWidget(normalizedSelection());
    constexpr int kMinTextEditorWidth = 96;
    constexpr int kMinTextEditorHeight = 38;
    const int availableRight = std::max(kMinTextEditorWidth, qRound(selection.right() - topLeft.x() - 12));
    const int availableBottom = std::max(kMinTextEditorHeight, qRound(selection.bottom() - topLeft.y() - 12));
    const int editorWidth = std::clamp(220, kMinTextEditorWidth, availableRight);
    const int editorHeight = std::clamp(m_textEditor->fontMetrics().height() + 18, kMinTextEditorHeight, availableBottom);
    QRect editorRect(qRound(topLeft.x()), qRound(topLeft.y()), editorWidth, editorHeight);
    editorRect.moveLeft(std::clamp(editorRect.left(), 8, std::max(8, width() - editorRect.width() - 8)));
    editorRect.moveTop(std::clamp(editorRect.top(), 8, std::max(8, height() - editorRect.height() - 8)));
    m_textEditor->setGeometry(editorRect);
}

void ShotWindow::redoAnnotation()
{
    if (m_redoStack.isEmpty()) {
        return;
    }

    const HistorySnapshot current = currentHistorySnapshot();
    const HistorySnapshot next = m_redoStack.takeLast();
    m_undoStack.append(current);
    restoreHistorySnapshot(next);
}

void ShotWindow::updateToolbarState()
{
    if (!m_toolbar) {
        return;
    }

    const QString active = currentToolName();
    const QString scopeAction = markshot::ui::actionName(Action::ToggleCaptureScope);
    const QString layoutAction = markshot::ui::actionName(Action::ToggleToolbarLayout);
    const auto buttons = m_toolbar->findChildren<QPushButton *>();
    for (QPushButton *button : buttons) {
        const QString action = button->property("action").toString();
        const bool isActiveTool = action == active
            || (action == scopeAction && m_fullscreenAnnotation)
            || (action == layoutAction && m_toolbarVerticalLayout);
        button->setProperty("active", isActiveTool);
        button->style()->unpolish(button);
        button->style()->polish(button);
        button->update();
    }
}

void ShotWindow::drawAnnotation(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const
{
    auto mapPoint = [this, widgetCoordinates](QPointF point) {
        return widgetCoordinates ? imageToWidget(point) : point;
    };

    auto mapRect = [this, widgetCoordinates](QRectF rect) {
        return widgetCoordinates ? imageRectToWidget(rect) : rect;
    };

    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal penWidth = std::max<qreal>(1.5, annotation.width * scale);

    painter.save();
    if (annotationSupportsRotation(annotation) && !qFuzzyIsNull(annotation.rotationDegrees)) {
        const QPointF center = annotationRotationCenter(annotation, widgetCoordinates);
        painter.translate(center);
        painter.rotate(annotation.rotationDegrees);
        painter.translate(-center);
    }

    QPen pen(annotation.color, penWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
    painter.setPen(pen);
    painter.setBrush(Qt::NoBrush);

    auto drawLinePath = [this, &painter, &mapPoint](const Annotation &lineAnnotation) {
        if (lineAnnotation.points.size() < 2) {
            return;
        }
        QPainterPath path(mapPoint(lineAnnotation.points.first()));
        if (annotationSupportsLineControl(lineAnnotation) && lineAnnotation.points.size() >= 3) {
            path.quadTo(mapPoint(lineAnnotation.points.at(2)), mapPoint(lineAnnotation.points.at(1)));
        } else {
            path.lineTo(mapPoint(lineAnnotation.points.at(1)));
        }
        painter.drawPath(path);
    };

    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        break;
    case Tool::Pen: {
        if (annotation.points.size() < 2) {
            break;
        }
        QVector<QPointF> mapped;
        mapped.reserve(annotation.points.size());
        for (const QPointF &point : annotation.points) {
            mapped.append(mapPoint(point));
        }
        painter.drawPath(smoothedStrokePath(mapped));
        break;
    }
    case Tool::Highlighter: {
        if (annotation.points.size() < 2) {
            break;
        }
        QColor color = annotation.color;
        color.setAlpha(qRound(annotation.color.alphaF() * 120.0));
        painter.save();
        painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
        painter.setPen(QPen(color, std::max<qreal>(6.0, penWidth), Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        if (annotation.highlighterStyle == HighlighterStyle::StraightLine) {
            drawLinePath(annotation);
        } else {
            QVector<QPointF> mapped;
            mapped.reserve(annotation.points.size());
            for (const QPointF &point : annotation.points) {
                mapped.append(mapPoint(point));
            }
            painter.drawPath(smoothedStrokePath(mapped));
        }
        painter.restore();
        break;
    }
    case Tool::Line:
        if (annotation.points.size() >= 2) {
            drawLinePath(annotation);
        }
        break;
    case Tool::Rectangle: {
        painter.setBrush(annotation.filled ? QBrush(annotation.color) : QBrush(Qt::NoBrush));
        const QRectF rect = mapRect(annotation.rect);
        const qreal radius = annotation.cornerRadius * scale;
        if (radius > 0.0) {
            painter.drawRoundedRect(rect, radius, radius);
        } else {
            painter.drawRect(rect);
        }
        break;
    }
    case Tool::Ellipse:
        painter.setBrush(annotation.filled ? QBrush(annotation.color) : QBrush(Qt::NoBrush));
        painter.drawEllipse(mapRect(annotation.rect));
        break;
    case Tool::Arrow:
        if (annotation.points.size() >= 2) {
            const std::optional<QPointF> controlPoint = annotation.points.size() >= 3
                ? std::optional<QPointF>(mapPoint(annotation.points.at(2)))
                : std::nullopt;
            drawArrow(painter,
                      mapPoint(annotation.points.first()),
                      mapPoint(annotation.points.at(1)),
                      penWidth,
                      annotation.arrowStyle,
                      controlPoint);
        }
        break;
    case Tool::Text: {
        QFont font = markshot::theme::textFont(qRound((19.0 + annotation.width) * scale),
                                               QFont::DemiBold,
                                               annotation.fontFamily);
        QRectF backgroundRect = textContentRect(annotation, widgetCoordinates);
        QRectF textRect = backgroundRect.adjusted(kTextBackgroundPaddingX * scale,
                                                  kTextBackgroundPaddingY * scale,
                                                  -kTextBackgroundPaddingX * scale,
                                                  -kTextBackgroundPaddingY * scale);
        QTextOption option;
        option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
        option.setAlignment(Qt::AlignLeft | Qt::AlignTop);
        painter.save();
        painter.setFont(font);
        if (annotation.backgroundColor.alpha() > 0) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(annotation.backgroundColor);
            painter.drawRoundedRect(backgroundRect, 4.0 * scale, 4.0 * scale);
        }
        painter.setPen(annotation.color);
        painter.setBrush(Qt::NoBrush);
        painter.drawText(textRect, annotation.text, option);
        painter.restore();
        break;
    }
    case Tool::Number:
        if (!annotation.points.isEmpty()) {
            const QPointF bubblePoint = annotation.points.first();
            const QPointF tipPoint = annotation.points.size() >= 2 ? annotation.points.last() : bubblePoint;
            drawNumber(painter, tipPoint, bubblePoint, annotation.number, annotation.color, annotation.width, widgetCoordinates);
        }
        break;
    case Tool::Mosaic:
        painter.setOpacity(annotation.color.alphaF());
        drawMosaic(painter, annotation.rect, annotation.width, widgetCoordinates);
        break;
    case Tool::Magnifier:
        drawMagnifier(painter, annotation, widgetCoordinates);
        break;
    }
    painter.restore();
}

void ShotWindow::drawArrow(QPainter &painter,
                           QPointF start,
                           QPointF end,
                           qreal width,
                           ArrowStyle style,
                           std::optional<QPointF> controlPoint) const
{
    const QLineF line(start, end);
    const qreal L = line.length();
    if (L < 1.0) {
        return;
    }

    const QColor color = painter.pen().color();

    if (controlPoint.has_value()) {
        QPainterPath curve(start);
        curve.quadTo(*controlPoint, end);

        auto curvePoint = [start, end, control = *controlPoint](qreal t) {
            const qreal u = 1.0 - t;
            return start * (u * u) + control * (2.0 * u * t) + end * (t * t);
        };

        qreal curveLength = 0.0;
        QPointF previous = start;
        for (int i = 1; i <= 24; ++i) {
            const QPointF current = curvePoint(static_cast<qreal>(i) / 24.0);
            curveLength += QLineF(previous, current).length();
            previous = current;
        }

        QPointF tangent = end - *controlPoint;
        if (QLineF(QPointF(0, 0), tangent).length() < 1.0) {
            tangent = end - start;
        }
        const qreal tangentLength = QLineF(QPointF(0, 0), tangent).length();
        if (tangentLength < 1.0) {
            return;
        }
        const QPointF direction(tangent.x() / tangentLength, tangent.y() / tangentLength);
        const QPointF normal(-direction.y(), direction.x());

        if (style == ArrowStyle::Kde) {
            qreal headLength = std::clamp(curveLength * 0.32, width * 2.6, width * 6.0);
            if (headLength > curveLength) {
                headLength = curveLength;
            }
            const qreal headHalfWidth = headLength * 0.62;
            const QPointF headBase = end - direction * headLength;
            const QPointF leftWing = headBase + normal * headHalfWidth;
            const QPointF rightWing = headBase - normal * headHalfWidth;

            painter.save();
            painter.setBrush(Qt::NoBrush);
            painter.setPen(QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.setRenderHint(QPainter::Antialiasing, true);
            painter.drawPath(curve);
            QPainterPath head;
            head.moveTo(leftWing);
            head.lineTo(end);
            head.lineTo(rightWing);
            painter.drawPath(head);
            painter.restore();
            return;
        }

        qreal headLength = curveLength * 0.18;
        headLength = std::clamp(headLength, width * 5.0, width * 9.0);
        if (headLength > curveLength * 0.62) {
            headLength = curveLength * 0.62;
        }
        headLength = std::clamp(headLength, 12.0, 60.0);
        if (headLength > curveLength * 0.62) {
            headLength = curveLength * 0.62;
        }

        const qreal bodyHalfWidth = width * 0.5;
        qreal headHalfWidth = headLength * 0.28;
        const qreal minHeadHalfWidth = bodyHalfWidth * 1.5;
        if (headHalfWidth < minHeadHalfWidth) {
            headHalfWidth = minHeadHalfWidth;
        }

        const QPointF headBase = end - direction * headLength;
        QPainterPath head;
        head.moveTo(headBase + normal * headHalfWidth);
        head.lineTo(end);
        head.lineTo(headBase - normal * headHalfWidth);
        head.closeSubpath();

        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setPen(QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        painter.setBrush(Qt::NoBrush);
        painter.drawPath(curve);
        painter.setPen(Qt::NoPen);
        painter.setBrush(color);
        painter.drawPath(head);
        painter.restore();
        return;
    }

    // 1. Calculate normalized direction and normal vectors
    const QPointF direction = QPointF(line.dx() / L, line.dy() / L);
    const QPointF normal(-direction.y(), direction.x());

    if (style == ArrowStyle::Kde) {
        // Uniform round-capped shaft topped by an open V head, matching the
        // KDE/Spectacle arrow: the stroke width stays constant end to end.
        qreal headLength = std::clamp(L * 0.32, width * 2.6, width * 6.0);
        if (headLength > L) {
            headLength = L;
        }
        const qreal headHalfWidth = headLength * 0.62;
        const QPointF headBase = end - direction * headLength;
        const QPointF leftWing = headBase + normal * headHalfWidth;
        const QPointF rightWing = headBase - normal * headHalfWidth;

        painter.save();
        QPen pen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(pen);
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.drawLine(start, end);
        QPainterPath head;
        head.moveTo(leftWing);
        head.lineTo(end);
        head.lineTo(rightWing);
        painter.drawPath(head);
        painter.restore();
        return;
    }

    // 2. Compute physical body half-width (perfectly aligned with pen brush width)
    const qreal bodyHalfWidth = width * 0.5;

    // 3. Compute adaptive arrow head length based on L (golden stretch ratio)
    qreal headLength = L * 0.18;
    headLength = std::clamp(headLength, width * 5.0, width * 9.0);
    if (headLength > L * 0.62) {
        headLength = L * 0.62;
    }
    headLength = std::clamp(headLength, 12.0, 60.0);
    if (headLength > L * 0.62) {
        headLength = L * 0.62;
    }

    // 4. Compute head half-width (sleeker, sharper aerodynamic 28% stretch ratio for acute angle nose)
    qreal headHalfWidth = headLength * 0.28;
    const qreal minHeadHalfWidth = bodyHalfWidth * 1.5;
    if (headHalfWidth < minHeadHalfWidth) {
        headHalfWidth = minHeadHalfWidth;
    }

    // 5. Locate headBase position
    const QPointF headBase = end - direction * headLength;

    // 6. Construct the elegant 6-vertex classic pointy-tailed gradient triangle polygon path
    QPainterPath arrow;
    arrow.moveTo(start);                              // 1. Pointy start (sharp tail converges to 0 width)
    arrow.lineTo(headBase + normal * bodyHalfWidth);  // 2. Left side body (gradient shaft)
    arrow.lineTo(headBase + normal * headHalfWidth);  // 3. Left wing base (vertical fold-out)
    arrow.lineTo(end);                                // 4. Arrow tip (aerodynamic nose)
    arrow.lineTo(headBase - normal * headHalfWidth);  // 5. Right wing base (aerodynamic nose)
    arrow.lineTo(headBase - normal * bodyHalfWidth);  // 6. Right side body (vertical fold-in)
    arrow.closeSubpath();                             // 7. Close back to pointy start

    // 7. Render the gorgeous hard-line polygon with anti-aliasing
    painter.save();
    painter.setPen(Qt::NoPen);
    painter.setBrush(color);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.drawPath(arrow);
    painter.restore();
}

void ShotWindow::drawWheelPreview(QPainter &painter)
{
    if (!m_showWheelPreview || !m_wheelPreviewTimer.isValid() || m_wheelPreviewTimer.elapsed() > 900) {
        m_showWheelPreview = false;
        updateCursor();
        return;
    }

    if (wheelZoomsImage()) {
        const QString zoomText = QStringLiteral("%1%").arg(qRound(m_imageZoom * 100.0));
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setFont(markshot::theme::uiFont(12, QFont::DemiBold));
        const QFontMetrics metrics(painter.font());
        const QRectF textBounds = metrics.boundingRect(zoomText);
        QRectF bubble(m_wheelPreviewPosition.x() + 14.0,
                      m_wheelPreviewPosition.y() + 14.0,
                      textBounds.width() + 24.0,
                      textBounds.height() + 14.0);
        bubble.moveLeft(std::min<qreal>(bubble.left(), width() - bubble.width() - 8.0));
        bubble.moveTop(std::min<qreal>(bubble.top(), height() - bubble.height() - 8.0));
        bubble.moveLeft(std::max<qreal>(8.0, bubble.left()));
        bubble.moveTop(std::max<qreal>(8.0, bubble.top()));

        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(8, 13, 19, 230));
        painter.drawRoundedRect(bubble, 10.0, 10.0);
        painter.setPen(QColor(204, 251, 241, 245));
        painter.drawText(bubble, Qt::AlignCenter, zoomText);
        painter.restore();
        return;
    }

    const qreal size = std::clamp(currentToolPreviewSize(), 2.0, 96.0);
    QRectF preview(m_wheelPreviewPosition.x() - size / 2.0,
                   m_wheelPreviewPosition.y() - size / 2.0,
                   size,
                   size);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing,
                          m_tool == Tool::Number || m_tool == Tool::Magnifier);
    painter.setPen(Qt::NoPen);
    painter.setBrush(m_currentColor);
    if (m_tool == Tool::Number || m_tool == Tool::Magnifier) {
        painter.drawEllipse(preview);
    } else {
        painter.drawRect(preview);
    }
    painter.restore();
}

void ShotWindow::drawLaserStroke(QPainter &painter, const LaserStroke &stroke, bool widgetCoordinates, qreal opacity) const
{
    if (stroke.points.size() < 2 || opacity <= 0.0) {
        return;
    }

    auto mapPoint = [this, widgetCoordinates](QPointF point) {
        return widgetCoordinates ? imageToWidget(point) : point;
    };
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal width = std::max<qreal>(3.0, stroke.width * scale);

    QPainterPath path(mapPoint(stroke.points.first()));
    for (int i = 1; i < stroke.points.size(); ++i) {
        path.lineTo(mapPoint(stroke.points.at(i)));
    }

    const qreal configuredOpacity = stroke.color.alphaF();
    QColor glow = stroke.color;
    glow.setAlpha(qRound(80 * opacity * configuredOpacity));
    QColor core = stroke.color;
    core.setAlpha(qRound(230 * opacity * configuredOpacity));
    QColor hot(255, 255, 255, qRound(170 * opacity * configuredOpacity));

    painter.save();
    painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
    painter.setPen(QPen(glow, width * 2.6, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.setPen(QPen(core, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.setPen(QPen(hot, std::max<qreal>(1.4, width * 0.22), Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.restore();
}

void ShotWindow::beginLaserStroke(QPointF imagePoint)
{
    m_dragging = true;
    m_dragStart = imagePoint;
    LaserStroke stroke;
    stroke.points.append(clampImagePoint(imagePoint));
    stroke.color = m_currentColor;
    stroke.width = m_laserWidth;
    stroke.expiresAt = m_laserClock.elapsed() + kLaserLifetimeMs;
    m_laserDraft = stroke;
    update();
}

void ShotWindow::updateLaserStroke(QPointF imagePoint)
{
    if (!m_laserDraft.has_value()) {
        return;
    }
    m_laserDraft->points.append(clampImagePoint(imagePoint));
    update();
}

void ShotWindow::commitLaserStroke()
{
    if (!m_laserDraft.has_value()) {
        return;
    }
    if (m_laserDraft->points.size() >= 2) {
        m_laserDraft->expiresAt = m_laserClock.elapsed() + kLaserLifetimeMs;
        m_laserStrokes.append(*m_laserDraft);
        if (m_laserTimer && !m_laserTimer->isActive()) {
            m_laserTimer->start();
        }
    }
    m_laserDraft.reset();
    update();
}

void ShotWindow::cleanupLaserStrokes()
{
    const qint64 now = m_laserClock.elapsed();
    for (int i = m_laserStrokes.size() - 1; i >= 0; --i) {
        if (m_laserStrokes.at(i).expiresAt <= now) {
            m_laserStrokes.removeAt(i);
        }
    }
    if (m_laserStrokes.isEmpty() && m_laserTimer) {
        m_laserTimer->stop();
    }
    update();
}

void ShotWindow::drawNumber(QPainter &painter,
                            QPointF tipPoint,
                            QPointF bubblePoint,
                            int number,
                            QColor color,
                            qreal width,
                            bool widgetCoordinates) const
{
    const QPointF tip = widgetCoordinates ? imageToWidget(tipPoint) : tipPoint;
    const QPointF center = widgetCoordinates ? imageToWidget(bubblePoint) : bubblePoint;
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal radius = std::max<qreal>(13.0, (13.0 + width * 1.35) * scale);
    const QRectF bubble(center.x() - radius, center.y() - radius, radius * 2.0, radius * 2.0);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    const QLineF leader(center, tip);
    if (leader.length() > radius * 0.45) {
        const QPointF direction(leader.dx() / leader.length(), leader.dy() / leader.length());
        const QPointF normal(-direction.y(), direction.x());
        const qreal tailHalfWidth = std::clamp(radius * 0.46, 8.0, 38.0);
        const QPointF baseCenter = center + direction * (radius * 0.82);
        QPainterPath tail;
        tail.moveTo(tip);
        tail.lineTo(baseCenter + normal * tailHalfWidth);
        tail.lineTo(baseCenter - normal * tailHalfWidth);
        tail.closeSubpath();
        painter.setPen(Qt::NoPen);
        painter.setBrush(color);
        painter.drawPath(tail);
    }

    painter.setPen(QPen(QColor(255, 255, 255), std::clamp(width * 0.22 * scale, 2.0, 9.0)));
    painter.setBrush(color);
    painter.drawEllipse(bubble);

    QFont font = markshot::theme::uiFont(qRound(std::clamp(radius * 0.92, 12.0, 54.0)), QFont::Black);
    painter.setFont(font);
    painter.setPen(Qt::white);
    painter.drawText(bubble, Qt::AlignCenter, QString::number(number));
    painter.restore();
}

void ShotWindow::drawMosaic(QPainter &painter, QRectF imageRect, qreal blockSize, bool widgetCoordinates) const
{
    QRect sourceRect = imageRect.normalized().toAlignedRect().intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
    if (sourceRect.isEmpty()) {
        return;
    }

    const QImage mosaic = mosaicImage(sourceRect, qRound(blockSize));
    if (mosaic.isNull()) {
        return;
    }

    painter.save();
    painter.setRenderHint(QPainter::SmoothPixmapTransform, false);
    painter.drawImage(widgetCoordinates ? imageRectToWidget(sourceRect) : QRectF(sourceRect), mosaic);
    painter.restore();
}

void ShotWindow::drawMagnifier(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const
{
    const QRectF lensImageRect = annotation.rect.normalized();
    const QRectF sourceImageRect = magnifierSourceRect(annotation);
    if (lensImageRect.width() < 4.0 || lensImageRect.height() < 4.0
        || sourceImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return;
    }

    const QRectF lensRect = widgetCoordinates ? imageRectToWidget(lensImageRect) : lensImageRect;
    const QRectF sourceRect = widgetCoordinates ? imageRectToWidget(sourceImageRect) : sourceImageRect;
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal borderWidth = std::clamp(annotation.width * scale, 1.5, 18.0);
    const QColor borderColor = annotation.color;

    QPainterPath lensPath;
    lensPath.addEllipse(lensRect);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    const QPointF sourceCenter = sourceRect.center();
    const QPointF lensCenter = lensRect.center();
    const QLineF centerLine(sourceCenter, lensCenter);
    const qreal sourceRadius = sourceRect.width() / 2.0;
    const qreal lensRadius = lensRect.width() / 2.0;
    if (centerLine.length() > lensRadius + sourceRadius * 0.65) {
        const QPointF towardLens(centerLine.dx() / centerLine.length(),
                                 centerLine.dy() / centerLine.length());
        const QPointF normal(-towardLens.y(), towardLens.x());
        constexpr qreal connectorAngle = 34.0 * M_PI / 180.0;
        const qreal along = std::cos(connectorAngle);
        const qreal across = std::sin(connectorAngle);
        const QPointF sourceUpper = sourceCenter
            + towardLens * (sourceRadius * along)
            + normal * (sourceRadius * across);
        const QPointF sourceLower = sourceCenter
            + towardLens * (sourceRadius * along)
            - normal * (sourceRadius * across);
        const QPointF towardSource = -towardLens;
        const QPointF lensUpper = lensCenter
            + towardSource * (lensRadius * along)
            + normal * (lensRadius * across);
        const QPointF lensLower = lensCenter
            + towardSource * (lensRadius * along)
            - normal * (lensRadius * across);

        painter.setPen(QPen(borderColor,
                            borderWidth,
                            Qt::SolidLine,
                            Qt::RoundCap,
                            Qt::RoundJoin));
        painter.drawLine(sourceUpper, lensUpper);
        painter.drawLine(sourceLower, lensLower);
    }

    painter.setClipPath(lensPath);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
    painter.drawImage(lensRect, m_frozenFrame, sourceImageRect);
    painter.setClipping(false);

    painter.setPen(QPen(borderColor, borderWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(Qt::NoBrush);
    painter.drawEllipse(lensRect.adjusted(borderWidth / 2.0,
                                          borderWidth / 2.0,
                                          -borderWidth / 2.0,
                                          -borderWidth / 2.0));
    painter.drawEllipse(sourceRect.adjusted(borderWidth / 2.0,
                                            borderWidth / 2.0,
                                            -borderWidth / 2.0,
                                            -borderWidth / 2.0));
    painter.restore();
}

void ShotWindow::beginTextAnnotation(QPointF imagePoint)
{
    m_editingTextAnnotationId.reset();
    m_textEditorImagePoint = imagePoint;
    m_draft.reset();
    m_textEditor->clear();
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(m_currentColor, m_textBackgroundColor, qRound(20.0 + m_shapeWidth)));
    m_textEditor->setFont(markshot::theme::textFont(qRound(20.0 + m_shapeWidth),
                                                    QFont::DemiBold,
                                                    m_textFontFamily));
    m_textEditor->show();
    m_textEditor->raise();
    updateTextEditorGeometry();
    m_textEditor->setFocus(Qt::MouseFocusReason);
    update();
}

void ShotWindow::beginEditingSelectedTextAnnotation()
{
    if (!m_selectedAnnotationId.has_value()) {
        return;
    }
    Annotation *annotation = annotationById(*m_selectedAnnotationId);
    if (!annotation || annotation->tool != Tool::Text) {
        return;
    }

    m_editingTextAnnotationId = annotation->id;
    m_textEditorImagePoint = annotation->rect.normalized().topLeft();
    m_draft.reset();
    m_textEditor->setPlainText(annotation->text);
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(annotation->color, annotation->backgroundColor, qRound(20.0 + annotation->width)));
    m_textEditor->setFont(markshot::theme::textFont(qRound(20.0 + annotation->width),
                                                    QFont::DemiBold,
                                                    annotation->fontFamily));
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    m_textEditor->show();
    m_textEditor->raise();
    const QRectF widgetRect = textContentRect(*annotation, true);
    m_textEditor->setGeometry(widgetRect.toAlignedRect().adjusted(0, 0, 1, 1));
    m_textEditor->setFocus(Qt::MouseFocusReason);
    update();
}

void ShotWindow::commitTextEditor()
{
    if (m_committingText || !m_textEditor || !m_textEditor->isVisible()) {
        return;
    }

    m_committingText = true;
    const QString text = m_textEditor->toPlainText().trimmed();
    const QRect editorGeometry = m_textEditor->geometry();
    m_textEditor->hide();
    m_textEditor->clear();
    setFocus(Qt::OtherFocusReason);

    if (m_editingTextAnnotationId.has_value()) {
        if (Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            pushHistorySnapshot();
            annotation->text = text;
            annotation->rect = QRectF(widgetToImage(editorGeometry.topLeft()),
                                      widgetToImage(editorGeometry.bottomRight())).normalized();
            annotation->fontFamily = m_textEditor->font().family();
            annotation->rect = textContentRect(*annotation, false);
            if (!annotation->points.isEmpty()) {
                annotation->points[0] = annotation->rect.topLeft();
            }
        }
        m_editingTextAnnotationId.reset();
        m_committingText = false;
        updateAnnotationPropertyPanel();
        update();
        return;
    }

    if (!text.isEmpty()) {
        pushHistorySnapshot();
        Annotation annotation;
        annotation.id = m_nextAnnotationId++;
        annotation.tool = Tool::Text;
        annotation.points.append(m_textEditorImagePoint);
        annotation.rect = QRectF(widgetToImage(editorGeometry.topLeft()),
                                 widgetToImage(editorGeometry.bottomRight())).normalized();
        annotation.text = text;
        annotation.color = m_currentColor;
        annotation.backgroundColor = m_textBackgroundColor;
        annotation.width = m_shapeWidth;
        annotation.fontFamily = m_textEditor->font().family();
        annotation.rect = textContentRect(annotation, false);
        m_textFontFamily = annotation.fontFamily;
        m_annotations.append(annotation);
    }

    m_committingText = false;
    update();
}

QString ShotWindow::saveSelectionToTempFile() const
{
    if (!hasUsableSelection()) {
        return {};
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return {};
    }

    const QString tempDir = QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
        ? QDir::tempPath()
        : QStandardPaths::writableLocation(QStandardPaths::TempLocation);
    const QString filename = QStringLiteral("mark-shot-open-%1.png")
                                 .arg(QDateTime::currentDateTimeUtc().toString(QStringLiteral("yyyyMMdd-hhmmss-zzz")));
    const QString path = QDir(tempDir).filePath(filename);
    return output.save(path, "PNG") ? path : QString();
}

void ShotWindow::openSelectionWithDesktop(const DesktopApp &app)
{
    commitTextEditor();
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }

    const QString imagePath = saveSelectionToTempFile();
    if (imagePath.isEmpty()) {
        return;
    }

    QStringList command = expandDesktopExec(app, imagePath);
    if (command.isEmpty()) {
        return;
    }

    const QString program = command.takeFirst();
    if (QProcess::startDetached(program, command)) {
        close();
    }
}

void ShotWindow::runExtensionCommand(const ExtensionCommand &command)
{
    commitTextEditor();
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }

    QString commandLine = command.command;
    if (commandLine.contains(QStringLiteral("{slurp}"))) {
        const QString geometry = slurpSelectionGeometry();
        if (geometry.isEmpty()) {
            return;
        }
        replaceExtensionSlurpPlaceholder(&commandLine, geometry);
    }

    bool replacedImagePlaceholder = false;
    QString imagePath;
    if (command.saveImage) {
        imagePath = saveSelectionToTempFile();
        if (imagePath.isEmpty()) {
            return;
        }
        replacedImagePlaceholder = replaceExtensionImagePlaceholders(&commandLine, imagePath);
        if (!replacedImagePlaceholder) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(imagePath);
        }
    }

    if (commandLine.trimmed().isEmpty()) {
        return;
    }

    const QString workingDirectory = command.workingDirectory.isEmpty()
        ? QString()
        : expandUserPath(command.workingDirectory);

    if (command.closeOnStart) {
        hide();
        QApplication::processEvents();
    }

    const bool started = QProcess::startDetached(commandShellProgram(),
                                                 commandShellArguments(commandLine),
                                                 workingDirectory);
    if (started && command.closeOnStart) {
        close();
        return;
    }

    if (!started && command.closeOnStart) {
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateExtensionPanelGeometry();
    }
}

void ShotWindow::startScrollCapture()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QRect geometry = selectionGlobalRect();
    if (geometry.isEmpty()) {
        return;
    }

    if (isGnomeWaylandSession() && !hasGnomeScrollHelper()) {
        QMessageBox::information(
            this,
            MS_TR("Scroll Capture"),
            MS_TR("Scroll capture is not supported on GNOME Wayland."));
        return;
    }

    const QString outputName = m_outputName;
    const markshot::scroll::ScrollSessionUiConfig uiConfig = scrollSessionUiConfig();
    QScreen *targetScreen = screen();
    QPointer<ShotWindow> self(this);

    // On X11, QScreen::grabWindow captures visible top-level windows. Hide the
    // selection UI and give the compositor one repaint before seeding the scroll
    // stitcher, otherwise the first frame can contain our own toolbar/overlay.
    hide();
    QApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
    QTimer::singleShot(120, qApp, [self, geometry, outputName, targetScreen, uiConfig] {
        auto *window =
            new markshot::scroll::ScrollSessionWindow(geometry, outputName, targetScreen, uiConfig);
        window->show();
        window->raise();
        window->activateWindow();
        if (self) {
            self->close();
        }
    });
}

void ShotWindow::pinSelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    auto *window = new PinnedImageWindow(output);
    window->show();
    window->raise();
    window->activateWindow();
    close();
}

void ShotWindow::ocrCopySelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QString tempPath = saveSelectionToTempFile();
    if (tempPath.isEmpty()) {
        return;
    }

    const PinnedWindowConfig config = pinnedWindowConfig();
    if (!config.ocrEnabled) {
        QFile::remove(tempPath);
        return;
    }

    QApplication::setOverrideCursor(Qt::WaitCursor);

    QProcess process;
    if (!config.ocrCommand.isEmpty()) {
        QString commandLine = config.ocrCommand;
        const bool replaced = replaceExtensionImagePlaceholders(&commandLine, tempPath);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(tempPath);
        }
        setShellCommand(&process, commandLine);
    } else {
        process.setProgram(helperProgramPath(QStringLiteral("mark-shot-ocr")));
        process.setArguments({QStringLiteral("--format"),
                              QStringLiteral("json"),
                              QStringLiteral("--backend"),
                              config.ocrBackend,
                              tempPath});
    }
    process.start();
    if (!process.waitForStarted(3000)) {
        QFile::remove(tempPath);
        QApplication::restoreOverrideCursor();
        showToast(config.ocrCommand.isEmpty()
                      ? MS_TR("OCR helper not found")
                      : MS_TR("OCR failed"));
        return;
    }
    if (!process.waitForFinished(config.ocrTimeoutMs)) {
        process.kill();
        process.waitForFinished(1000);
        QFile::remove(tempPath);
        QApplication::restoreOverrideCursor();
        showToast(MS_TR("OCR timed out"));
        return;
    }

    QFile::remove(tempPath);
    QApplication::restoreOverrideCursor();

    const QByteArray output = process.readAllStandardOutput();
    const QByteArray errorOutput = process.readAllStandardError();
    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0) {
        showToast(config.ocrCommand.isEmpty()
                      && ocrOutputReportsMissingBackend(output, errorOutput, config.ocrBackend)
                      ? MS_TR("OCR backend not installed. Install rapidocr or tesseract.")
                      : MS_TR("OCR failed"));
        return;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
    if (parseError.error != QJsonParseError::NoError) {
        showToast(config.ocrCommand.isEmpty()
                      && ocrOutputReportsMissingBackend(output, errorOutput, config.ocrBackend)
                      ? MS_TR("OCR backend not installed. Install rapidocr or tesseract.")
                      : MS_TR("OCR failed"));
        return;
    }

    QJsonArray tokenArray;
    if (document.isArray()) {
        tokenArray = document.array();
    } else if (document.isObject()) {
        tokenArray = document.object().value(QStringLiteral("tokens")).toArray();
    }

    auto rectFromJson = [](const QJsonValue &value) -> std::optional<QRectF> {
        if (!value.isArray()) {
            return std::nullopt;
        }
        const QJsonArray array = value.toArray();
        if (array.size() == 4 && array.at(0).isDouble()) {
            return QRectF(array.at(0).toDouble(), array.at(1).toDouble(),
                          array.at(2).toDouble(), array.at(3).toDouble());
        }
        if (array.size() < 2 || !array.at(0).isArray()) {
            return std::nullopt;
        }
        QRectF bounds;
        bool initialized = false;
        for (const QJsonValue &pv : array) {
            if (!pv.isArray()) continue;
            const QJsonArray pt = pv.toArray();
            if (pt.size() < 2) continue;
            const QPointF p(pt.at(0).toDouble(), pt.at(1).toDouble());
            bounds = initialized ? bounds.united(QRectF(p, QSizeF(0, 0))) : QRectF(p, QSizeF(0, 0));
            initialized = true;
        }
        return initialized ? std::optional(bounds) : std::nullopt;
    };

    auto ocrRect = [&](const QJsonObject &obj) -> std::optional<QRectF> {
        for (const auto &key : {QStringLiteral("box"), QStringLiteral("bbox"), QStringLiteral("points")}) {
            if (obj.contains(key)) return rectFromJson(obj.value(key));
        }
        if (obj.contains(QStringLiteral("x")) && obj.contains(QStringLiteral("y"))) {
            return QRectF(obj.value(QStringLiteral("x")).toDouble(), obj.value(QStringLiteral("y")).toDouble(),
                          obj.value(QStringLiteral("width")).toDouble(), obj.value(QStringLiteral("height")).toDouble());
        }
        if (obj.contains(QStringLiteral("left")) && obj.contains(QStringLiteral("top"))) {
            return QRectF(obj.value(QStringLiteral("left")).toDouble(), obj.value(QStringLiteral("top")).toDouble(),
                          obj.value(QStringLiteral("width")).toDouble(), obj.value(QStringLiteral("height")).toDouble());
        }
        return std::nullopt;
    };

    auto isNoSpacePunctuation = [](QChar ch) {
        switch (ch.unicode()) {
        case '.': case ',': case ';': case ':': case '!': case '?':
        case ')': case ']': case '}':
        case 0x3001: case 0x3002: case 0x300B: case 0x3011:
        case 0xFF01: case 0xFF09: case 0xFF0C: case 0xFF1A: case 0xFF1B: case 0xFF1F:
            return true;
        default:
            return false;
        }
    };

    struct LineToken {
        int line;
        int index;
        QString text;
        QRectF rect;
    };
    QVector<LineToken> tokens;
    int fallbackIndex = 0;
    for (const QJsonValue &value : tokenArray) {
        if (!value.isObject()) continue;
        const QJsonObject object = value.toObject();
        const QString text = object.value(QStringLiteral("text")).toString().trimmed();
        if (text.isEmpty()) continue;
        const auto rect = ocrRect(object);
        if (!rect) continue;
        LineToken token;
        token.text = text;
        token.line = object.value(QStringLiteral("line")).toInt(0);
        token.index = object.value(QStringLiteral("index")).toInt(fallbackIndex++);
        token.rect = rect->normalized();
        tokens.append(token);
    }

    if (tokens.isEmpty()) {
        showToast(config.ocrCommand.isEmpty()
                      && ocrOutputReportsMissingBackend(output, errorOutput, config.ocrBackend)
                      ? MS_TR("OCR backend not installed. Install rapidocr or tesseract.")
                      : MS_TR("No text recognized"));
        return;
    }
    std::stable_sort(tokens.begin(), tokens.end(), [](const LineToken &a, const LineToken &b) {
        return a.line != b.line ? a.line < b.line : a.index < b.index;
    });

    QString result;
    int currentLine = -1;
    QRectF prevRect;
    QString prevText;
    for (const LineToken &token : tokens) {
        if (currentLine != token.line) {
            if (!result.isEmpty()) {
                result += QLatin1Char('\n');
            }
            currentLine = token.line;
        } else if (!prevText.isEmpty() && !token.text.isEmpty()
                   && !isNoSpacePunctuation(token.text.front())) {
            const qreal gap = token.rect.left() - prevRect.right();
            const qreal threshold = std::max<qreal>(3.0, std::min(prevRect.height(), token.rect.height()) * 0.28);
            if (gap > threshold) {
                result += QLatin1Char(' ');
            }
        }
        result += token.text;
        prevText = token.text;
        prevRect = token.rect;
    }

    if (ocrResultPanelEnabled()) {
        auto *window = new OcrResultWindow(result);
        window->show();
        window->raise();
        window->activateWindow();
        close();
        return;
    }

    markshot::copyTextToClipboard(result);
    if (!sendDesktopNotification(QStringLiteral("Mark Shot"), MS_TR("OCR text copied"), 2500)) {
        showToast(MS_TR("OCR text copied"));
    }
    QTimer::singleShot(150, this, [this] { close(); });
}

void ShotWindow::showToast(const QString &text, int durationMs)
{
    auto *label = new QLabel(text, this);
    label->setAlignment(Qt::AlignCenter);
    label->setFont(markshot::theme::uiFont(12, QFont::DemiBold));
    label->setStyleSheet(QStringLiteral(
        "background: rgba(8, 13, 19, 220);"
        "color: rgba(204, 251, 241, 238);"
        "border-radius: 14px;"
        "padding: 8px 22px;"));
    label->adjustSize();
    label->move((width() - label->width()) / 2, height() - label->height() - 80);
    label->show();
    QTimer::singleShot(durationMs, label, &QObject::deleteLater);
}

QImage ShotWindow::renderedSelection() const
{
    const QRect sourceBounds(QPoint(0, 0), m_frozenFrame.size());
    const QRect selectionRect = normalizedSelection().toAlignedRect().intersected(sourceBounds);
    if (selectionRect.isEmpty()) {
        return {};
    }

    QImage output = m_frozenFrame.copy(selectionRect).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QPainter painter(&output);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.translate(-selectionRect.topLeft());
    for (const Annotation &annotation : m_annotations) {
        drawAnnotation(painter, annotation, false);
    }
    painter.end();
    return output;
}

QString ShotWindow::defaultSavePath() const
{
    const QString filename = QStringLiteral("mark-shot-%1.png").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
    return QDir(markShotPicturesDir()).filePath(filename);
}

void ShotWindow::saveSelection()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    const QString path = defaultSavePath();
    if (output.save(path, "PNG")) {
        const QString message = MS_TR("Saved to %1").arg(path);
        // Keyboard save should finish without another dialog round-trip.
        if (!sendDesktopNotification(QStringLiteral("Mark Shot"), message, 3000)) {
            showToast(message, 2500);
        }
        QTimer::singleShot(150, this, [this] { close(); });
        return;
    }

    showToast(MS_TR("Save failed"), 2500);
}

void ShotWindow::saveSelectionAs()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }

    hide();

    auto *dialog = new QFileDialog(nullptr, MS_TR("Save Screenshot"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->setAcceptMode(QFileDialog::AcceptSave);
    dialog->setFileMode(QFileDialog::AnyFile);
    dialog->setNameFilter(MS_TR("PNG Images (*.png)"));
    dialog->setDefaultSuffix(QStringLiteral("png"));
    dialog->setOption(QFileDialog::DontUseNativeDialog, true);
    dialog->selectFile(defaultSavePath());

    connect(dialog, &QFileDialog::accepted, this, [this, dialog, output] {
        const QStringList files = dialog->selectedFiles();
        if (!files.isEmpty() && output.save(files.first(), "PNG")) {
            const QString message = MS_TR("Saved to %1").arg(files.first());
            // Prefer desktop notifications because the window may close immediately after saving.
            if (!sendDesktopNotification(QStringLiteral("Mark Shot"), message, 3000)) {
                showToast(message, 2500);
            }
            QTimer::singleShot(150, this, [this] { close(); });
            return;
        }
        showToast(MS_TR("Save failed"), 2500);
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
    });
    connect(dialog, &QFileDialog::rejected, this, [this] {
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
    });
    dialog->open();
}

void ShotWindow::copySelection()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    markshot::copyImageToClipboard(output);

    close();
}
