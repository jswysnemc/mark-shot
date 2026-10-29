#include "marketplace/plugin_qt_compat.h"

#include <QStringList>

namespace markshot::marketplace {
namespace {

struct QtMajorMinor {
    int major = -1;
    int minor = 0;
};

/**
 * 解析版本号中的主版本与次版本。
 * @param version 版本号字符串。
 * @param parsed 输出解析结果。
 * @return 格式合法时返回 true。
 */
bool parseMajorMinor(const QString &version, QtMajorMinor *parsed)
{
    const QStringList parts = version.trimmed().split(QLatin1Char('.'));
    if (parts.isEmpty() || parts.size() > 3) {
        return false;
    }
    int values[3] = {0, 0, 0};
    for (int index = 0; index < parts.size(); ++index) {
        bool ok = false;
        values[index] = parts.at(index).toInt(&ok);
        if (!ok || values[index] < 0) {
            return false;
        }
    }
    parsed->major = values[0];
    parsed->minor = values[1];
    return true;
}

}  // namespace

bool isValidPluginQtVersion(const QString &version)
{
    QtMajorMinor parsed;
    return parseMajorMinor(version, &parsed);
}

bool pluginQtVersionCompatible(const QString &assetQtVersion, const QString &runtimeQtVersion)
{
    // 1. 旧索引没有声明构建版本，保持原有行为
    if (assetQtVersion.trimmed().isEmpty()) {
        return true;
    }
    QtMajorMinor asset;
    QtMajorMinor runtime;
    if (!parseMajorMinor(assetQtVersion, &asset) || !parseMajorMinor(runtimeQtVersion, &runtime)) {
        return false;
    }
    // 2. 主版本必须一致，插件次版本不能高于运行时，与 QPluginLoader 的拒绝条件相同
    return asset.major == runtime.major && asset.minor <= runtime.minor;
}

}  // namespace markshot::marketplace
