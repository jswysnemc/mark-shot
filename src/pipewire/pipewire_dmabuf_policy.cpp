#include "pipewire/pipewire_dmabuf_policy.h"

#include <QDir>
#include <QFileInfo>
#include <QProcessEnvironment>
#include <QString>
#include <QStringList>
#include <QtGlobal>

namespace markshot::pipewire {
namespace {

/**
 * 读取桌面环境标识文本。
 * @return 小写的桌面环境标识拼接结果。
 */
QString desktopIdentityText()
{
    const QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
    return (environment.value(QStringLiteral("XDG_CURRENT_DESKTOP")) + QLatin1Char(':')
            + environment.value(QStringLiteral("XDG_SESSION_DESKTOP")) + QLatin1Char(':')
            + environment.value(QStringLiteral("DESKTOP_SESSION")))
        .toLower();
}

/**
 * 统计 DRM 渲染节点数量。
 * @return 渲染节点数量。
 */
int countRenderNodes()
{
#ifdef Q_OS_WIN
    return 0;
#else
    const QDir driDirectory(QStringLiteral("/dev/dri"));
    if (!driDirectory.exists()) {
        return 0;
    }
    return driDirectory.entryList({QStringLiteral("renderD*")}, QDir::System).size();
#endif
}

}  // namespace

bool shouldAvoidDmaBuf(const DmaBufEnvironment &environment)
{
    // 1. 显式强制优先级最高，便于在修复后验证 DMA-BUF 路径
    if (environment.forcedByEnvironment) {
        return false;
    }
    if (environment.disabledByEnvironment) {
        return true;
    }

    // 2. 已知失效组合：KWin 搭配 NVIDIA 专有驱动
    if (!environment.kdeSession || !environment.nvidiaProprietaryDriver) {
        return false;
    }

    // 3. 混合显卡机器上 KWin 通常渲染在集显，DMA-BUF 可用，不做规避
    return environment.renderNodeCount <= 1;
}

DmaBufEnvironment currentDmaBufEnvironment()
{
    const QString desktop = desktopIdentityText();

    DmaBufEnvironment environment;
    environment.kdeSession = desktop.contains(QStringLiteral("kde"))
        || desktop.contains(QStringLiteral("plasma"));
    environment.nvidiaProprietaryDriver =
        QFileInfo::exists(QStringLiteral("/proc/driver/nvidia/version"))
        || QFileInfo::exists(QStringLiteral("/dev/nvidiactl"));
    environment.renderNodeCount = countRenderNodes();
    environment.forcedByEnvironment = qEnvironmentVariableIsSet("MARK_SHOT_FORCE_DMABUF");
    environment.disabledByEnvironment = qEnvironmentVariableIsSet("MARK_SHOT_DISABLE_DMABUF");
    return environment;
}

bool shouldAvoidDmaBuf()
{
    return shouldAvoidDmaBuf(currentDmaBufEnvironment());
}

}  // namespace markshot::pipewire
