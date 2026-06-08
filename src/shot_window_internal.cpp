#include "shot_window_module.h"

namespace cfg = markshot::config;

namespace markshot::shot {

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

/// @brief Creates the high-contrast cross cursor used during capture.
/// @return Cursor used for selection and annotation drawing modes.
QCursor captureCrossCursor()
{
    QPixmap pixmap(33, 33);
    pixmap.fill(Qt::transparent);

    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setPen(QPen(QColor(15, 23, 42, 235), 5, Qt::SolidLine, Qt::SquareCap));
    painter.drawLine(16, 0, 16, 32);
    painter.drawLine(0, 16, 32, 16);
    painter.setPen(QPen(QColor(255, 255, 255, 245), 3, Qt::SolidLine, Qt::SquareCap));
    painter.drawLine(16, 0, 16, 32);
    painter.drawLine(0, 16, 32, 16);
    painter.setPen(QPen(QColor(45, 212, 191, 255), 1, Qt::SolidLine, Qt::SquareCap));
    painter.drawLine(16, 0, 16, 32);
    painter.drawLine(0, 16, 32, 16);
    painter.end();

    return QCursor(pixmap, 16, 16);
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

bool sendDesktopNotification(const QString &summary, const QString &body, int timeoutMs)
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

void drawRoundedLabel(QPainter &painter, QRectF rect, const QString &text, QColor fill)
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
/// @brief Builds a smoothed stroke path from raw sample points.
/// @param points The raw sample points.
/// @return A QPainterPath containing the smoothed path.
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

/// @brief A single resampling sample representing an input pixel weight.
struct AxisSample {
    /// @brief The input index of the sample.
    int index = 0;
    /// @brief The weight contribution of the sample.
    double weight = 0.0;
};

/// @brief Resampling lookup table for a single coordinate axis.
struct AxisTable {
    /// @brief Precalculated kernel samples for each output coordinate.
    std::vector<std::vector<AxisSample>> samples;
    /// @brief The lowest input coordinate index referenced in the table.
    int first = 0;
    /// @brief The highest input coordinate index referenced in the table.
    int last = -1;
};

/// @brief Evaluates a sharpening filter kernel.
/// @param distance The distance from the center.
/// @return The filter weight at the given distance.
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

/// @brief Builds an AxisTable for 1D image scaling.
/// @param inputSize The size of the input axis.
/// @param outputSize The size of the output axis.
/// @param sourceStart The starting coordinate in the source coordinate system.
/// @param scale The scaling factor.
/// @return The populated AxisTable.
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

/// @brief Converts a weighted double value into a byte.
/// @param value The double value to convert.
/// @return The converted integer value clamped between 0 and 255.
int byteFromWeightedSum(double value)
{
    return std::clamp(static_cast<int>(std::lround(value)), 0, 255);
}

/// @brief Blends and packs red, green, blue, and alpha components into a premultiplied QRgb value.
/// @param red The red color component.
/// @param green The green color component.
/// @param blue The blue color component.
/// @param alpha The alpha opacity component.
/// @return The premultiplied QRgb value.
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

    /// @brief The ideal number of concurrent threads to use.
    const int idealThreads = std::max(1, QThread::idealThreadCount());
    /// @brief The number of threads selected for processing.
    const int threadCount = std::clamp(rowCount / kMinSharpRowsPerThread, 1, idealThreads);
    if (threadCount == 1) {
        function(0, rowCount);
        return;
    }

    /// @brief The ranges of rows to be processed by each thread.
    std::vector<std::pair<int, int>> ranges;
    ranges.reserve(threadCount);
    /// @brief The number of rows allocated to each thread.
    const int rowsPerThread = rowCount / threadCount;
    /// @brief The starting row index of the current range.
    int begin = 0;
    /// @brief The loop counter for iterating thread ranges.
    for (int i = 0; i < threadCount; ++i) {
        /// @brief The ending row index of the current range.
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

void replaceShellPlaceholder(QString *command, const QString &placeholder, const QString &value, bool *replaced)
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



}  // namespace markshot::shot
