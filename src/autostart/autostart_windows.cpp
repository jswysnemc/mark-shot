#include "autostart/autostart_manager.h"

#include "ui/i18n.h"

#include <QSettings>
#include <QString>

namespace markshot::autostart::platform {
namespace {

constexpr const char *kRunRegistryPath = "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
constexpr const char *kRunValueName = "Mark Shot";

/// @brief 返回 Windows Run 注册表项。
/// @return 当前用户 Run 注册表配置对象。
QSettings runRegistry()
{
    return QSettings(QString::fromLatin1(kRunRegistryPath), QSettings::NativeFormat);
}

}  // namespace

bool windowsAutostartSupported()
{
#if defined(Q_OS_WIN)
    return true;
#else
    return false;
#endif
}

bool windowsAutostartEnabled()
{
#if defined(Q_OS_WIN)
    QSettings settings = runRegistry();
    return settings.value(QString::fromLatin1(kRunValueName)).toString() == startupCommand();
#else
    return false;
#endif
}

bool setWindowsAutostartEnabled(bool enabled, QString *error)
{
    if (error) {
        error->clear();
    }

#if defined(Q_OS_WIN)
    QSettings settings = runRegistry();
    if (enabled) {
        settings.setValue(QString::fromLatin1(kRunValueName), startupCommand());
    } else {
        settings.remove(QString::fromLatin1(kRunValueName));
    }
    settings.sync();
    if (settings.status() != QSettings::NoError) {
        if (error) {
            *error = MS_TR("Cannot update Windows autostart registry.");
        }
        return false;
    }
    return true;
#else
    Q_UNUSED(enabled);
    if (error) {
        *error = MS_TR("Autostart is not supported on this platform.");
    }
    return false;
#endif
}

}  // namespace markshot::autostart::platform
