#include "scroll/scroll_config.h"

#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QStandardPaths>

#include <optional>

namespace markshot::scroll {

namespace {

std::optional<StitchAlgorithm> g_override;

QString configFilePath()
{
    QString configDir = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    if (configDir.isEmpty()) {
        const QString generic = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
        configDir = generic.isEmpty()
            ? QDir::home().filePath(QStringLiteral(".config/mark-shot"))
            : QDir(generic).filePath(QStringLiteral("mark-shot"));
    }
    return QDir(configDir).filePath(QStringLiteral("config.json"));
}

}  // namespace

QString algorithmName(StitchAlgorithm algorithm)
{
    return algorithm == StitchAlgorithm::OpenCvOrb ? QStringLiteral("opencv-orb")
                                                   : QStringLiteral("col-sample");
}

StitchAlgorithm parseAlgorithmName(const QString &name, StitchAlgorithm fallback)
{
    const QString normalized = name.trimmed().toLower();
    if (normalized.isEmpty()) {
        return fallback;
    }
    if (normalized == QStringLiteral("col-sample") || normalized == QStringLiteral("colsample")
        || normalized == QStringLiteral("col") || normalized == QStringLiteral("cols")) {
        return StitchAlgorithm::ColSample;
    }
    if (normalized == QStringLiteral("opencv-orb") || normalized == QStringLiteral("opencv")
        || normalized == QStringLiteral("orb")) {
        return StitchAlgorithm::OpenCvOrb;
    }
    return fallback;
}

StitchAlgorithm configuredScrollAlgorithm()
{
    QFile file(configFilePath());
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return kDefaultAlgorithm;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return kDefaultAlgorithm;
    }

    const QJsonValue scroll = document.object().value(QStringLiteral("scroll"));
    if (!scroll.isObject()) {
        return kDefaultAlgorithm;
    }
    const QString name = scroll.toObject().value(QStringLiteral("algorithm")).toString();
    return parseAlgorithmName(name, kDefaultAlgorithm);
}

void setScrollAlgorithmOverride(StitchAlgorithm algorithm)
{
    g_override = algorithm;
}

void clearScrollAlgorithmOverride()
{
    g_override.reset();
}

StitchAlgorithm resolveScrollAlgorithm()
{
    if (g_override) {
        return *g_override;
    }
    return configuredScrollAlgorithm();
}

}  // namespace markshot::scroll
