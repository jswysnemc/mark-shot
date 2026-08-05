#pragma once

#include <QProcessEnvironment>
#include <QString>

namespace markshot::window_detection {

/**
 * 描述窗口检测所处的显示会话。
 */
struct Session {
    bool wayland = false;
    QString compositor;
};

/**
 * 根据进程环境识别当前显示会话和已支持的 Wayland 合成器。
 * @param environment 用于识别会话的进程环境。
 * @return 会话类型和已识别的合成器名称。
 */
inline Session detectSession(const QProcessEnvironment &environment)
{
    Session session;
    session.wayland = environment.value(QStringLiteral("XDG_SESSION_TYPE"))
                           .compare(QStringLiteral("wayland"), Qt::CaseInsensitive) == 0;
    if (!session.wayland) {
        return session;
    }

    const QString desktop = (environment.value(QStringLiteral("XDG_CURRENT_DESKTOP"))
        + QLatin1Char(':') + environment.value(QStringLiteral("XDG_SESSION_DESKTOP"))
        + QLatin1Char(':') + environment.value(QStringLiteral("DESKTOP_SESSION")))
        .toLower();
    if (desktop.contains(QStringLiteral("gnome"))) {
        session.compositor = QStringLiteral("gnome");
    } else if (desktop.contains(QStringLiteral("kde"))
               || desktop.contains(QStringLiteral("plasma"))) {
        session.compositor = QStringLiteral("kde");
    } else if (desktop.contains(QStringLiteral("hyprland"))) {
        session.compositor = QStringLiteral("hyprland");
    } else if (desktop.contains(QStringLiteral("niri"))) {
        session.compositor = QStringLiteral("niri");
    }
    return session;
}

/**
 * 判断命令是否为 Mark Shot 提供的确切内置窗口检测脚本名。
 * @param command 已配置的窗口检测命令。
 * @return 命令为四个内置脚本之一时返回 true。
 */
inline bool isBundledCommand(const QString &command)
{
    return command == QStringLiteral("mark-shot-window-detection-gnome")
        || command == QStringLiteral("mark-shot-window-detection-kde")
        || command == QStringLiteral("mark-shot-window-detection-hyprland")
        || command == QStringLiteral("mark-shot-window-detection-niri");
}

/**
 * 判断配置命令是否适合当前显示会话。
 * @param command 用户配置中的窗口检测命令。
 * @param session 当前显示会话。
 * @return 应保留配置命令时返回 true。
 */
inline bool commandMatchesSession(const QString &command, const Session &session)
{
    if (!session.wayland) {
        return true;
    }
    if (command.isEmpty()) {
        return false;
    }
    if (!isBundledCommand(command)) {
        return true;
    }
    return !session.compositor.isEmpty()
        && command == QStringLiteral("mark-shot-window-detection-") + session.compositor;
}

/**
 * 返回当前会话应使用的内置窗口检测命令。
 * @param session 当前显示会话。
 * @return 已支持 Wayland 合成器的脚本名，其他会话返回空字符串。
 */
inline QString defaultCommand(const Session &session)
{
    return session.wayland && !session.compositor.isEmpty()
        ? QStringLiteral("mark-shot-window-detection-") + session.compositor
        : QString();
}

}  // namespace markshot::window_detection
