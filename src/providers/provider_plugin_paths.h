#pragma once

#include <QString>
#include <QStringList>

namespace markshot::providers {

/**
 * 读取用户级插件安装目录。
 * @return 用户级插件目录的绝对路径。
 */
QString userPluginDirectory();

/**
 * 读取插件搜索目录列表。
 * @return 插件搜索目录的绝对路径列表。
 */
QStringList pluginSearchDirs();

}  // namespace markshot::providers
