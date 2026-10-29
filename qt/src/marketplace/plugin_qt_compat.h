#pragma once

#include <QString>

namespace markshot::marketplace {

/**
 * 【插件市场】【Qt兼容】判断字符串是否为合法的 Qt 版本号。
 *
 * 接受 `6`、`6.12`、`6.12.0` 三种写法，各段均为非负整数。
 * @param version 待检查的版本号。
 * @return 格式合法时返回 true。
 */
bool isValidPluginQtVersion(const QString &version);

/**
 * 【插件市场】【Qt兼容】判断按 assetQtVersion 构建的插件能否被 runtimeQtVersion 加载。
 *
 * 与 QPluginLoader 的规则一致：主版本必须相同，插件构建时的次版本不能高于运行时；
 * 修订号不参与比较。assetQtVersion 为空表示索引未声明，按兼容处理以保持旧索引可用。
 * @param assetQtVersion 插件资产声明的构建 Qt 版本。
 * @param runtimeQtVersion 当前进程运行的 Qt 版本，例如 qVersion() 的返回值。
 * @return 可以加载时返回 true。
 */
bool pluginQtVersionCompatible(const QString &assetQtVersion, const QString &runtimeQtVersion);

}  // namespace markshot::marketplace
