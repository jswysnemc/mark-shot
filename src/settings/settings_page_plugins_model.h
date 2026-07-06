#pragma once

#include "providers/provider_plugin_info.h"

#include <QString>
#include <QStringList>
#include <QVector>

namespace markshot::settings {

struct ProviderOption {
    QString label;
    QString value;
};

struct PluginDiagnosticRow {
    QString capability;
    QString provider;
    QString status;
    QString path;
    QString details;
};

/**
 * 生成指定能力的 provider 选择项。
 * @param capability 插件能力。
 * @return 下拉框选择项。
 */
QVector<ProviderOption> providerOptionsForCapability(markshot::providers::ProviderPluginCapability capability);

/**
 * 生成插件诊断表格行。
 * @return 诊断表格行。
 */
QVector<PluginDiagnosticRow> pluginDiagnosticRows();

/**
 * 读取插件搜索目录列表。
 * @return 插件搜索目录。
 */
QStringList pluginSearchDirectoryRows();

/**
 * 读取用户级插件目录。
 * @return 用户级插件目录。
 */
QString userPluginDirectory();

}  // namespace markshot::settings
