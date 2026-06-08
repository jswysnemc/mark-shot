#include "app_config_store.h"

#include "window_detection.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonParseError>
#include <QSaveFile>

namespace {

/// @brief Assigns a JSON value at a nested object path.
/// @param object JSON object to update.
/// @param path Nested key path.
/// @param value JSON value to assign.
void setNestedValue(QJsonObject *object, const QStringList &path, const QJsonValue &value)
{
    if (!object || path.isEmpty()) {
        return;
    }

    const QString key = path.first();
    if (path.size() == 1) {
        object->insert(key, value);
        return;
    }

    QJsonObject child = object->value(key).isObject() ? object->value(key).toObject() : QJsonObject();
    setNestedValue(&child, path.mid(1), value);
    object->insert(key, child);
}

/// @brief Writes an optional error message.
/// @param error Optional output error message.
/// @param message Error message to write.
/// @return Always false for compact failure returns.
bool fail(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
    return false;
}

}  // namespace

namespace markshot {

QJsonObject readAppConfigRoot(bool *ok)
{
    if (ok) {
        *ok = false;
    }

    if (!ensureAppConfigFile()) {
        return {};
    }

    QFile file(appConfigPath());
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return {};
    }

    if (ok) {
        *ok = true;
    }
    return document.object();
}

bool writeAppConfigRoot(const QJsonObject &root, QString *error)
{
    const QString path = appConfigPath();
    if (path.isEmpty()) {
        return fail(error, QStringLiteral("Config path is empty"));
    }

    QDir dir(QFileInfo(path).absolutePath());
    if (!dir.exists() && !dir.mkpath(QStringLiteral("."))) {
        return fail(error, QStringLiteral("Cannot create config directory"));
    }

    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        return fail(error, file.errorString());
    }

    file.write(QJsonDocument(root).toJson(QJsonDocument::Indented));
    file.write("\n");
    if (!file.commit()) {
        return fail(error, file.errorString());
    }
    return true;
}

bool writeAppConfigValue(const QStringList &path, const QJsonValue &value, QString *error)
{
    if (path.isEmpty()) {
        return fail(error, QStringLiteral("Config value path is empty"));
    }

    bool ok = false;
    QJsonObject root = readAppConfigRoot(&ok);
    if (!ok) {
        return fail(error, QStringLiteral("Cannot read application config"));
    }

    setNestedValue(&root, path, value);
    return writeAppConfigRoot(root, error);
}

}  // namespace markshot
