#pragma once

#include <QString>
#include <QStringList>

namespace markshot::window_detection {

/**
 * 【窗口识别】【内置脚本】定位指定内置窗口检测脚本。
 * @param command 已配置的脚本命令，自定义命令保持原样。
 * @param directories 按优先级排列的安装目录和构建目录。
 * @return 可用脚本的路径，未找到时返回原命令。
 */
QString resolveBundledWindowDetectionScript(const QString &command, const QStringList &directories);

/**
 * 【窗口识别】【脚本调用】构造安全引用路径的窗口检测命令。
 * @param command 用户配置中的命令。
 * @return 可交给统一 shell 执行入口的命令。
 */
QString windowDetectionInvocation(const QString &command);

}  // namespace markshot::window_detection
