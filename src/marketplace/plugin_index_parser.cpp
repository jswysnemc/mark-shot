#include "marketplace/plugin_index_parser.h"

#include <QJsonArray>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QRegularExpression>
#include <QSet>
#include <QSysInfo>
#include <QUrl>

#include <algorithm>

namespace markshot::marketplace {
namespace {

const QSet<QString> kAllowedCapabilities = {
    QStringLiteral("ocr"),
    QStringLiteral("translation"),
    QStringLiteral("code-scan"),
};

const QSet<QString> kAllowedPlatforms = {
    QStringLiteral("windows"),
    QStringLiteral("linux"),
    QStringLiteral("macos"),
};

/**
 * 添加校验问题。
 * @param result 解析结果。
 * @param severity 问题级别。
 * @param path JSON 路径。
 * @param message 问题说明。
 * @return 无返回值。
 */
void addIssue(PluginIndexParseResult *result,
              PluginIndexIssueSeverity severity,
              const QString &path,
              const QString &message)
{
    result->issues.append({severity, path, message});
}

/**
 * 添加校验错误。
 * @param result 解析结果。
 * @param path JSON 路径。
 * @param message 错误说明。
 * @return 无返回值。
 */
void addError(PluginIndexParseResult *result, const QString &path, const QString &message)
{
    addIssue(result, PluginIndexIssueSeverity::Error, path, message);
}

/**
 * 判断字符串是否匹配正则表达式。
 * @param value 待检查字符串。
 * @param pattern 正则表达式。
 * @return 匹配时返回 true。
 */
bool matchesPattern(const QString &value, const QString &pattern)
{
    return QRegularExpression(pattern).match(value).hasMatch();
}

/**
 * 判断字符串是否为 SemVer 形式。
 * @param value 待检查字符串。
 * @return 符合 SemVer 基础格式时返回 true。
 */
bool isSemVer(const QString &value)
{
    return matchesPattern(value, QStringLiteral(R"(^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$)"));
}

/**
 * 判断 provider 标识是否合法。
 * @param value 待检查标识。
 * @return 合法时返回 true。
 */
bool isProviderId(const QString &value)
{
    return matchesPattern(value, QStringLiteral(R"(^[a-z0-9][a-z0-9-]{1,62}[a-z0-9]$)"));
}

/**
 * 判断插件索引条目标识是否合法。
 * @param value 待检查标识。
 * @return 合法时返回 true。
 */
bool isPluginId(const QString &value)
{
    return matchesPattern(value, QStringLiteral(R"(^[a-z0-9][a-z0-9.-]{1,126}[a-z0-9]$)"));
}

/**
 * 判断 SHA-256 是否为 64 位十六进制字符串。
 * @param value 待检查摘要。
 * @return 合法时返回 true。
 */
bool isSha256(const QString &value)
{
    return matchesPattern(value, QStringLiteral(R"(^[0-9A-Fa-f]{64}$)"));
}

/**
 * 判断文件名是否不包含路径信息。
 * @param value 待检查文件名。
 * @return 安全时返回 true。
 */
bool isSafeFileName(const QString &value)
{
    return !value.isEmpty()
        && value != QStringLiteral(".")
        && value != QStringLiteral("..")
        && !value.contains(QLatin1Char('/'))
        && !value.contains(QLatin1Char('\\'));
}

/**
 * 判断 URL 是否为 HTTP 下载地址。
 * @param value 待检查 URL 字符串。
 * @return 合法时返回 true。
 */
bool isHttpUrl(const QString &value)
{
    const QUrl url(value);
    return url.isValid() && !url.host().isEmpty()
        && (url.scheme() == QStringLiteral("https") || url.scheme() == QStringLiteral("http"));
}

/**
 * 读取对象中的必填字符串字段。
 * @param object JSON 对象。
 * @param key 字段名。
 * @param path JSON 路径。
 * @param result 解析结果。
 * @return 字段值，非法时返回空字符串。
 */
QString requiredString(const QJsonObject &object,
                       const QString &key,
                       const QString &path,
                       PluginIndexParseResult *result)
{
    const QJsonValue value = object.value(key);
    if (!value.isString() || value.toString().trimmed().isEmpty()) {
        addError(result,
                 QStringLiteral("%1.%2").arg(path, key),
                 QStringLiteral("must be a non-empty string"));
        return QString();
    }
    return value.toString().trimmed();
}

/**
 * 读取对象中的可选字符串字段。
 * @param object JSON 对象。
 * @param key 字段名。
 * @return 字段值，不存在时返回空字符串。
 */
QString optionalString(const QJsonObject &object, const QString &key)
{
    const QJsonValue value = object.value(key);
    return value.isString() ? value.toString().trimmed() : QString();
}

/**
 * 解析 capability 对象。
 * @param object capability JSON 对象。
 * @param path JSON 路径。
 * @param result 解析结果。
 * @return capability 条目。
 */
PluginIndexCapability parseCapability(const QJsonObject &object,
                                      const QString &path,
                                      PluginIndexParseResult *result)
{
    PluginIndexCapability capability;
    capability.type = requiredString(object, QStringLiteral("type"), path, result);
    capability.providerId = requiredString(object, QStringLiteral("providerId"), path, result);
    capability.displayName = requiredString(object, QStringLiteral("displayName"), path, result);

    if (!capability.type.isEmpty() && !kAllowedCapabilities.contains(capability.type)) {
        addError(result,
                 QStringLiteral("%1.type").arg(path),
                 QStringLiteral("must be one of ocr, translation, code-scan"));
    }
    if (!capability.providerId.isEmpty() && !isProviderId(capability.providerId)) {
        addError(result,
                 QStringLiteral("%1.providerId").arg(path),
                 QStringLiteral("must use lowercase letters, numbers, and hyphens"));
    }
    return capability;
}

/**
 * 解析 asset 对象。
 * @param object asset JSON 对象。
 * @param path JSON 路径。
 * @param result 解析结果。
 * @return asset 条目。
 */
PluginIndexAsset parseAsset(const QJsonObject &object, const QString &path, PluginIndexParseResult *result)
{
    PluginIndexAsset asset;
    asset.platform = requiredString(object, QStringLiteral("platform"), path, result);
    asset.architecture = requiredString(object, QStringLiteral("architecture"), path, result);
    asset.fileName = requiredString(object, QStringLiteral("fileName"), path, result);
    asset.libraryFileName = optionalString(object, QStringLiteral("libraryFileName"));
    asset.downloadUrl = requiredString(object, QStringLiteral("downloadUrl"), path, result);
    asset.sha256 = requiredString(object, QStringLiteral("sha256"), path, result).toLower();

    const QJsonValue sizeValue = object.value(QStringLiteral("size"));
    if (!sizeValue.isDouble() || sizeValue.toDouble() <= 0) {
        addError(result, QStringLiteral("%1.size").arg(path), QStringLiteral("must be a positive number"));
    } else {
        asset.size = static_cast<qint64>(sizeValue.toDouble());
    }

    if (!asset.platform.isEmpty() && !kAllowedPlatforms.contains(asset.platform)) {
        addError(result,
                 QStringLiteral("%1.platform").arg(path),
                 QStringLiteral("must be one of windows, linux, macos"));
    }
    if (!isSafeFileName(asset.fileName)) {
        addError(result,
                 QStringLiteral("%1.fileName").arg(path),
                 QStringLiteral("must be a file name without path separators"));
    }
    if (!asset.libraryFileName.isEmpty() && !isSafeFileName(asset.libraryFileName)) {
        addError(result,
                 QStringLiteral("%1.libraryFileName").arg(path),
                 QStringLiteral("must be a file name without path separators"));
    }
    if (!asset.downloadUrl.isEmpty() && !isHttpUrl(asset.downloadUrl)) {
        addError(result,
                 QStringLiteral("%1.downloadUrl").arg(path),
                 QStringLiteral("must be an HTTP or HTTPS URL"));
    }
    if (!asset.sha256.isEmpty() && !isSha256(asset.sha256)) {
        addError(result,
                 QStringLiteral("%1.sha256").arg(path),
                 QStringLiteral("must be a SHA-256 hex digest"));
    }
    return asset;
}

/**
 * 解析插件条目对象。
 * @param object 插件条目 JSON 对象。
 * @param index 插件条目下标。
 * @param result 解析结果。
 * @return 插件条目。
 */
PluginIndexEntry parsePluginEntry(const QJsonObject &object, int index, PluginIndexParseResult *result)
{
    const QString path = QStringLiteral("plugins[%1]").arg(index);
    PluginIndexEntry entry;
    entry.id = requiredString(object, QStringLiteral("id"), path, result);
    entry.name = requiredString(object, QStringLiteral("name"), path, result);
    entry.version = requiredString(object, QStringLiteral("version"), path, result);
    entry.vendor = requiredString(object, QStringLiteral("vendor"), path, result);
    entry.markShotMinVersion = requiredString(object, QStringLiteral("markShotMinVersion"), path, result);
    entry.description = optionalString(object, QStringLiteral("description"));
    entry.homepage = optionalString(object, QStringLiteral("homepage"));
    entry.license = optionalString(object, QStringLiteral("license"));

    if (!entry.id.isEmpty() && !isPluginId(entry.id)) {
        addError(result, QStringLiteral("%1.id").arg(path), QStringLiteral("must be a stable lowercase plugin id"));
    }
    if (!entry.version.isEmpty() && !isSemVer(entry.version)) {
        addError(result, QStringLiteral("%1.version").arg(path), QStringLiteral("must look like SemVer"));
    }
    if (!entry.markShotMinVersion.isEmpty() && !isSemVer(entry.markShotMinVersion)) {
        addError(result,
                 QStringLiteral("%1.markShotMinVersion").arg(path),
                 QStringLiteral("must look like SemVer"));
    }
    if (!entry.homepage.isEmpty() && !isHttpUrl(entry.homepage)) {
        addError(result, QStringLiteral("%1.homepage").arg(path), QStringLiteral("must be an HTTP or HTTPS URL"));
    }

    const QJsonValue capabilitiesValue = object.value(QStringLiteral("capabilities"));
    if (!capabilitiesValue.isArray() || capabilitiesValue.toArray().isEmpty()) {
        addError(result, QStringLiteral("%1.capabilities").arg(path), QStringLiteral("must be a non-empty array"));
    } else {
        const QJsonArray capabilities = capabilitiesValue.toArray();
        for (int capabilityIndex = 0; capabilityIndex < capabilities.size(); ++capabilityIndex) {
            const QString capabilityPath =
                QStringLiteral("%1.capabilities[%2]").arg(path).arg(capabilityIndex);
            if (!capabilities.at(capabilityIndex).isObject()) {
                addError(result, capabilityPath, QStringLiteral("must be an object"));
                continue;
            }
            entry.capabilities.append(parseCapability(capabilities.at(capabilityIndex).toObject(),
                                                      capabilityPath,
                                                      result));
        }
    }

    const QJsonValue assetsValue = object.value(QStringLiteral("assets"));
    if (!assetsValue.isArray() || assetsValue.toArray().isEmpty()) {
        addError(result, QStringLiteral("%1.assets").arg(path), QStringLiteral("must be a non-empty array"));
    } else {
        const QJsonArray assets = assetsValue.toArray();
        for (int assetIndex = 0; assetIndex < assets.size(); ++assetIndex) {
            const QString assetPath = QStringLiteral("%1.assets[%2]").arg(path).arg(assetIndex);
            if (!assets.at(assetIndex).isObject()) {
                addError(result, assetPath, QStringLiteral("must be an object"));
                continue;
            }
            entry.assets.append(parseAsset(assets.at(assetIndex).toObject(), assetPath, result));
        }
    }
    return entry;
}

}  // namespace

bool PluginIndexParseResult::ok() const
{
    return std::none_of(issues.cbegin(), issues.cend(), [](const PluginIndexIssue &issue) {
        return issue.severity == PluginIndexIssueSeverity::Error;
    });
}

QStringList PluginIndexParseResult::errorMessages() const
{
    QStringList messages;
    for (const PluginIndexIssue &issue : issues) {
        if (issue.severity == PluginIndexIssueSeverity::Error) {
            messages.append(QStringLiteral("%1: %2").arg(issue.path, issue.message));
        }
    }
    return messages;
}

PluginIndexParseResult parsePluginIndex(const QByteArray &json)
{
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    PluginIndexParseResult result;
    if (parseError.error != QJsonParseError::NoError) {
        addError(&result, QStringLiteral("root"), parseError.errorString());
        return result;
    }
    return parsePluginIndexDocument(document);
}

PluginIndexParseResult parsePluginIndexDocument(const QJsonDocument &document)
{
    PluginIndexParseResult result;
    if (!document.isObject()) {
        addError(&result, QStringLiteral("root"), QStringLiteral("must be an object"));
        return result;
    }

    const QJsonObject root = document.object();
    const QJsonValue schemaVersionValue = root.value(QStringLiteral("schemaVersion"));
    if (!schemaVersionValue.isDouble() || schemaVersionValue.toInt() != 1) {
        addError(&result, QStringLiteral("schemaVersion"), QStringLiteral("must be 1"));
    } else {
        result.index.schemaVersion = 1;
    }
    result.index.generatedAt = optionalString(root, QStringLiteral("generatedAt"));

    const QJsonValue pluginsValue = root.value(QStringLiteral("plugins"));
    if (!pluginsValue.isArray()) {
        addError(&result, QStringLiteral("plugins"), QStringLiteral("must be an array"));
        return result;
    }

    const QJsonArray plugins = pluginsValue.toArray();
    for (int index = 0; index < plugins.size(); ++index) {
        if (!plugins.at(index).isObject()) {
            addError(&result, QStringLiteral("plugins[%1]").arg(index), QStringLiteral("must be an object"));
            continue;
        }
        result.index.plugins.append(parsePluginEntry(plugins.at(index).toObject(), index, &result));
    }
    return result;
}

QString currentPlatformKey()
{
#ifdef Q_OS_WIN
    return QStringLiteral("windows");
#elif defined(Q_OS_MACOS)
    return QStringLiteral("macos");
#else
    return QStringLiteral("linux");
#endif
}

QString currentArchitectureKey()
{
    const QString arch = QSysInfo::currentCpuArchitecture().toLower();
    if (arch == QStringLiteral("amd64") || arch == QStringLiteral("x86_64")) {
        return QStringLiteral("x86_64");
    }
    if (arch == QStringLiteral("arm64") || arch == QStringLiteral("aarch64")) {
        return QStringLiteral("aarch64");
    }
    return arch;
}

QVector<PluginIndexAsset> assetsForCurrentPlatform(const PluginIndexEntry &entry)
{
    QVector<PluginIndexAsset> assets;
    const QString platform = currentPlatformKey();
    const QString architecture = currentArchitectureKey();
    for (const PluginIndexAsset &asset : entry.assets) {
        if (asset.platform == platform && asset.architecture == architecture) {
            assets.append(asset);
        }
    }
    return assets;
}

}  // namespace markshot::marketplace
