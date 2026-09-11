#include "shot_window_module.h"

#include "notifications/app_notifications.h"

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
    return markshot::notifications::sendDesktopNotification(summary, body, timeoutMs);
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
