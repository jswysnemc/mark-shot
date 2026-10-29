#pragma once

#include <QByteArray>
#include <QJsonDocument>
#include <QString>
#include <QStringList>
#include <QVector>

namespace markshot::marketplace {

enum class PluginIndexIssueSeverity {
    Error,
    Warning,
};

struct PluginIndexIssue {
    PluginIndexIssueSeverity severity = PluginIndexIssueSeverity::Error;
    QString path;
    QString message;
};

struct PluginIndexCapability {
    QString type;
    QString providerId;
    QString displayName;
};

struct PluginIndexAsset {
    QString platform;
    QString architecture;
    QString fileName;
    QString libraryFileName;
    QString downloadUrl;
    QString sha256;
    qint64 size = 0;
    /// 构建插件时使用的 Qt 版本；为空表示未声明，按兼容处理
    QString qtVersion;
};

struct PluginIndexEntry {
    QString id;
    QString name;
    QString version;
    QString vendor;
    QString description;
    QString markShotMinVersion;
    QString homepage;
    QString license;
    QVector<PluginIndexCapability> capabilities;
    QVector<PluginIndexAsset> assets;
};

struct PluginIndex {
    int schemaVersion = 0;
    QString generatedAt;
    QVector<PluginIndexEntry> plugins;
};

struct PluginIndexParseResult {
    PluginIndex index;
    QVector<PluginIndexIssue> issues;

    /**
     * 判断索引是否通过校验。
     * @return 没有错误时返回 true。
     */
    bool ok() const;

    /**
     * 读取所有错误消息。
     * @return 错误消息列表。
     */
    QStringList errorMessages() const;
};

/**
 * 解析插件市场索引 JSON 字节流。
 * @param json 索引 JSON 内容。
 * @return 解析结果和校验问题。
 */
PluginIndexParseResult parsePluginIndex(const QByteArray &json);

/**
 * 解析插件市场索引 JSON 文档。
 * @param document 索引 JSON 文档。
 * @return 解析结果和校验问题。
 */
PluginIndexParseResult parsePluginIndexDocument(const QJsonDocument &document);

/**
 * 读取当前平台在插件索引中的平台标识。
 * @return `windows`、`linux` 或 `macos`。
 */
QString currentPlatformKey();

/**
 * 把 CPU 架构名称标准化为插件索引使用的架构标识。
 *
 * `amd64` 归一为 `x86_64`，`arm64` 归一为 `aarch64`，其余名称只转为小写。
 * @param architecture 原始架构名称。
 * @return 标准化后的架构标识。
 */
QString normalizeArchitectureKey(const QString &architecture);

/**
 * 读取当前 CPU 架构在插件索引中的架构标识。
 * @return 标准化后的架构标识。
 */
QString currentArchitectureKey();

/**
 * 筛选当前平台可安装的插件资产。
 *
 * 除平台与架构外，还按资产声明的 qtVersion 排除当前 Qt 无法加载的动态库。
 * @param entry 插件索引条目。
 * @return 当前平台匹配的资产列表。
 */
QVector<PluginIndexAsset> assetsForCurrentPlatform(const PluginIndexEntry &entry);

/**
 * 按指定运行时 Qt 版本筛选当前平台可安装的插件资产。
 * @param entry 插件索引条目。
 * @param runtimeQtVersion 运行时 Qt 版本。
 * @return 平台、架构与 Qt 版本均兼容的资产列表。
 */
QVector<PluginIndexAsset> assetsForCurrentPlatform(const PluginIndexEntry &entry,
                                                   const QString &runtimeQtVersion);

}  // namespace markshot::marketplace
