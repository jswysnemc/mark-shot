#pragma once

#include <QJsonObject>
#include <QJsonValue>
#include <QString>
#include <QStringList>

namespace markshot {

/// @brief Reads the application config root object from the configured JSON file.
/// @param ok Optional output flag that is true when the file was read successfully.
/// @return Application config root object, or an empty object on failure.
QJsonObject readAppConfigRoot(bool *ok = nullptr);

/// @brief Writes the application config root object to the configured JSON file.
/// @param root Application config root object to persist.
/// @param error Optional output error message when writing fails.
/// @return True when the config file was written successfully.
bool writeAppConfigRoot(const QJsonObject &root, QString *error = nullptr);

/// @brief Writes a nested value in the application config JSON file.
/// @param path Nested key path from the root object to the value.
/// @param value JSON value to write at the nested path.
/// @param error Optional output error message when writing fails.
/// @return True when the value was persisted successfully.
bool writeAppConfigValue(const QStringList &path, const QJsonValue &value, QString *error = nullptr);

}  // namespace markshot
