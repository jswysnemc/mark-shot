#include "autostart/autostart_manager.h"

#include "ui/i18n.h"

#include <QCoreApplication>

namespace markshot::autostart {
namespace platform {

/// @brief 判断 Linux 桌面自启动是否可用。
/// @return Linux 自启动可用返回 true。
bool linuxAutostartSupported();

/// @brief 查询 Linux 桌面自启动状态。
/// @return 已启用 Linux 桌面自启动返回 true。
bool linuxAutostartEnabled();

/// @brief 写入或删除 Linux 桌面自启动项。
/// @param enabled 是否启用自启动。
/// @param error 操作失败时输出错误信息。
/// @return 操作成功返回 true。
bool setLinuxAutostartEnabled(bool enabled, QString *error);

/// @brief 判断 Windows 注册表自启动是否可用。
/// @return Windows 自启动可用返回 true。
bool windowsAutostartSupported();

/// @brief 查询 Windows 注册表自启动状态。
/// @return 已启用 Windows 自启动返回 true。
bool windowsAutostartEnabled();

/// @brief 写入或删除 Windows 注册表自启动项。
/// @param enabled 是否启用自启动。
/// @param error 操作失败时输出错误信息。
/// @return 操作成功返回 true。
bool setWindowsAutostartEnabled(bool enabled, QString *error);

}  // namespace platform

namespace {

/// @brief 清空可选错误输出。
/// @param error 错误输出指针。
void clearError(QString *error)
{
    if (error) {
        error->clear();
    }
}

}  // namespace

bool isSupported()
{
#if defined(Q_OS_WIN)
    return platform::windowsAutostartSupported();
#elif defined(Q_OS_LINUX)
    return platform::linuxAutostartSupported();
#else
    return false;
#endif
}

bool isEnabled()
{
#if defined(Q_OS_WIN)
    return platform::windowsAutostartEnabled();
#elif defined(Q_OS_LINUX)
    return platform::linuxAutostartEnabled();
#else
    return false;
#endif
}

bool setEnabled(bool enabled, QString *error)
{
    clearError(error);
    if (!isSupported()) {
        if (enabled && error) {
            *error = MS_TR("Autostart is not supported on this platform.");
        }
        return !enabled;
    }

#if defined(Q_OS_WIN)
    return platform::setWindowsAutostartEnabled(enabled, error);
#elif defined(Q_OS_LINUX)
    return platform::setLinuxAutostartEnabled(enabled, error);
#else
    return false;
#endif
}

QString startupCommand()
{
    return QStringLiteral("\"%1\" --tray").arg(QCoreApplication::applicationFilePath());
}

}  // namespace markshot::autostart
