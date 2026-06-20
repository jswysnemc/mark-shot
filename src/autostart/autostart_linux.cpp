#include "autostart/autostart_manager.h"

#include "ui/i18n.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QStandardPaths>
#include <QString>
#include <QTextStream>

namespace markshot::autostart::platform {
namespace {

/// @brief 返回 Linux 桌面自启动目录路径。
/// @return XDG autostart 目录路径。
QString autostartDirPath()
{
    const QString configDir = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation);
    return configDir.isEmpty() ? QString() : QDir(configDir).filePath(QStringLiteral("autostart"));
}

/// @brief 返回 Mark Shot 桌面自启动文件路径。
/// @return mark-shot.desktop 完整路径。
QString autostartDesktopPath()
{
    const QString dirPath = autostartDirPath();
    return dirPath.isEmpty() ? QString() : QDir(dirPath).filePath(QStringLiteral("mark-shot.desktop"));
}

/// @brief 转义 desktop Exec 字段中的路径。
/// @param value 原始路径。
/// @return 可放入双引号中的 Exec 字段片段。
QString desktopQuoted(QString value)
{
    value.replace(QLatin1Char('\\'), QStringLiteral("\\\\"));
    value.replace(QLatin1Char('"'), QStringLiteral("\\\""));
    value.replace(QLatin1Char('$'), QStringLiteral("\\$"));
    value.replace(QLatin1Char('`'), QStringLiteral("\\`"));
    return QStringLiteral("\"%1\"").arg(value);
}

/// @brief 写入 Linux desktop 自启动文件。
/// @param path 目标 desktop 文件路径。
/// @param error 操作失败时输出错误信息。
/// @return 写入成功返回 true。
bool writeDesktopFile(const QString &path, QString *error)
{
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text | QIODevice::Truncate)) {
        if (error) {
            *error = MS_TR("Cannot write autostart desktop file: %1").arg(file.errorString());
        }
        return false;
    }

    QTextStream stream(&file);
    stream << QStringLiteral("[Desktop Entry]\n");
    stream << QStringLiteral("Type=Application\n");
    stream << QStringLiteral("Name=Mark Shot\n");
    stream << QStringLiteral("Comment=Start Mark Shot in the system tray\n");
    stream << QStringLiteral("Exec=%1 --tray\n").arg(desktopQuoted(QCoreApplication::applicationFilePath()));
    stream << QStringLiteral("Terminal=false\n");
    stream << QStringLiteral("X-GNOME-Autostart-enabled=true\n");
    return true;
}

}  // namespace

bool linuxAutostartSupported()
{
    return !autostartDirPath().isEmpty();
}

bool linuxAutostartEnabled()
{
    return QFile::exists(autostartDesktopPath());
}

bool setLinuxAutostartEnabled(bool enabled, QString *error)
{
    if (error) {
        error->clear();
    }

    const QString filePath = autostartDesktopPath();
    if (filePath.isEmpty()) {
        if (enabled && error) {
            *error = MS_TR("Cannot resolve autostart directory.");
        }
        return !enabled;
    }

    if (!enabled) {
        if (!QFile::exists(filePath)) {
            return true;
        }
        if (!QFile::remove(filePath)) {
            if (error) {
                *error = MS_TR("Cannot remove autostart desktop file.");
            }
            return false;
        }
        return true;
    }

    const QString dirPath = autostartDirPath();
    QDir dir(dirPath);
    if (!dir.exists() && !dir.mkpath(QStringLiteral("."))) {
        if (error) {
            *error = MS_TR("Cannot create autostart directory: %1").arg(dirPath);
        }
        return false;
    }
    return writeDesktopFile(filePath, error);
}

}  // namespace markshot::autostart::platform
