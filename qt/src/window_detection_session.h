#pragma once

#include <QDir>
#include <QFileInfo>
#include <QProcessEnvironment>
#include <QString>
#include <QStringList>

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

/**
 * 在给定目录中查找内置窗口检测脚本。
 * @param command 配置中的命令。
 * @param directories 按优先级排列的安装目录和构建目录。
 * @return 找到可执行脚本时返回绝对路径，否则返回原命令。
 */
inline QString resolveBundledCommand(const QString &command, const QStringList &directories)
{
    if (!isBundledCommand(command)) {
        return command;
    }
    for (const QString &directory : directories) {
        if (directory.isEmpty()) {
            continue;
        }
        const QFileInfo file(QDir(directory).filePath(command));
#if defined(Q_OS_WIN)
        // Windows 用扩展名判断可执行，安装目录里的脚本没有 .exe 也要算找到
        const bool runnable = file.isFile();
#else
        const bool runnable = file.isFile() && file.isExecutable();
#endif
        if (runnable) {
            return file.absoluteFilePath();
        }
    }
    return command;
}

/**
 * 把单个路径包成 shell 单引号参数，避免安装路径含空格时被拆开。
 * @param word 需要原样传给 shell 的路径或命令。
 * @return 可放进 `sh -c` 的单个单词。
 */
inline QString quoteShellWord(const QString &word)
{
    QString escaped = word;
    escaped.replace(QLatin1Char('\''), QStringLiteral("'\\''"));
    return QLatin1Char('\'') + escaped + QLatin1Char('\'');
}

/**
 * 列出内置窗口检测脚本的查找目录。
 * 优先使用应用专用安装目录，并兼容本地构建树及旧版 bin 布局。
 * @param applicationDirectory 可执行文件所在目录。
 * @param homeDirectory 当前用户主目录。
 * @param currentDirectory 进程工作目录。
 * @return 按优先级排列的目录。
 */
inline QStringList bundledCommandSearchDirectories(const QString &applicationDirectory,
                                                   const QString &homeDirectory,
                                                   const QString &currentDirectory)
{
    QStringList directories;
    const auto append = [&directories](const QString &path) {
        const QString cleaned = QDir::cleanPath(path);
        if (!cleaned.isEmpty() && !directories.contains(cleaned)) {
            directories.append(cleaned);
        }
    };
    append(QDir(applicationDirectory).filePath(QStringLiteral("../libexec/mark-shot")));
    append(applicationDirectory);
    append(QDir(applicationDirectory).filePath(QStringLiteral("scripts")));
    append(QDir(applicationDirectory).filePath(QStringLiteral("../scripts")));
    append(QDir(applicationDirectory).filePath(QStringLiteral("../qt/scripts")));
    append(QDir(applicationDirectory).filePath(QStringLiteral("../lib/mark-shot")));
    append(QDir(currentDirectory).filePath(QStringLiteral("scripts")));
    append(QDir(homeDirectory).filePath(QStringLiteral(".local/bin")));
    append(QStringLiteral("/usr/local/bin"));
    append(QStringLiteral("/usr/bin"));
    return directories;
}

}  // namespace markshot::window_detection
