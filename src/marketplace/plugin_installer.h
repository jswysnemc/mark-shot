#pragma once

#include <QString>

namespace markshot::marketplace {

struct PluginInstallRequest {
    QString sourcePath;
    QString fileName;
    QString destinationDirectory;
    QString expectedSha256;
};

struct PluginInstallResult {
    bool success = false;
    QString installedPath;
    QString error;
};

/**
 * 判断文件名是否为受支持的插件动态库。
 * @param fileName 文件名。
 * @return 支持时返回 true。
 */
bool isSupportedPluginLibraryFile(const QString &fileName);

/**
 * 安装本地插件动态库资产。
 * @param request 安装请求。
 * @return 安装结果。
 */
PluginInstallResult installPluginAsset(const PluginInstallRequest &request);

}  // namespace markshot::marketplace
