#include "window_detection_bundled_script.h"

#include "debug_log.h"
#include "window_detection_session.h"

#include <QCoreApplication>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QFileInfo>
#include <QGlobalStatic>
#include <QMutex>
#include <QMutexLocker>
#include <QSaveFile>
#include <QStandardPaths>
#include <QTemporaryDir>

namespace {

struct EmbeddedWindowScripts {
    QTemporaryDir directory;
    bool ready = false;
    QMutex mutex;
};

Q_GLOBAL_STATIC(EmbeddedWindowScripts, embeddedScripts)

/**
 * 【窗口识别】【脚本回退】将资源中的脚本和共享模块释放到进程专用临时目录。
 * @param command 当前桌面的内置脚本名称。
 * @return 脚本副本路径，资源缺失或写入失败时返回空字符串。
 */
QString materializeEmbeddedScript(const QString &command)
{
    const QString resourceRoot = QStringLiteral(":/window-detection");
    if (!QFile::exists(resourceRoot + QLatin1Char('/') + command)) {
        return {};
    }
    auto *bundle = embeddedScripts();
    QMutexLocker locker(&bundle->mutex);
    if (!bundle->directory.isValid()) {
        return {};
    }

    // 1. 【窗口识别】【脚本回退】一次性复制整个包，niri 的共享模块保持相同相对路径
    if (!bundle->ready) {
        QDirIterator resources(resourceRoot, QDir::Files, QDirIterator::Subdirectories);
        while (resources.hasNext()) {
            const QString resourcePath = resources.next();
            const QString relativePath = resourcePath.mid(resourceRoot.size() + 1);
            const QString outputPath = bundle->directory.filePath(relativePath);
            if (!QDir().mkpath(QFileInfo(outputPath).absolutePath())) {
                return {};
            }
            QFile resource(resourcePath);
            QSaveFile output(outputPath);
            if (!resource.open(QIODevice::ReadOnly) || !output.open(QIODevice::WriteOnly)) {
                return {};
            }
            const QByteArray contents = resource.readAll();
            if (output.write(contents) != contents.size() || !output.commit()
                || !QFile::setPermissions(outputPath, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                                        | QFileDevice::ExeOwner)) {
                return {};
            }
        }
        bundle->ready = true;
    }

    // 2. 【窗口识别】【脚本回退】只返回实际存在的资源副本，不为未知命令生成文件
    const QString path = bundle->directory.filePath(command);
    return QFileInfo(path).isFile() ? path : QString();
}

}  // namespace

namespace markshot::window_detection {

QString resolveBundledWindowDetectionScript(const QString &command, const QStringList &directories)
{
    if (!isBundledCommand(command)) {
        return command;
    }
    // 1. 【窗口识别】【内置脚本】保留安装目录和用户 PATH 中的脚本优先级
    const QString resolved = resolveBundledCommand(command, directories);
    if (resolved != command) {
        return resolved;
    }
    const QString onPath = QStandardPaths::findExecutable(command);
    if (!onPath.isEmpty()) {
        return onPath;
    }
    // 2. 【窗口识别】【内置脚本】独立二进制或缺失安装脚本时回退到随程序携带的资源
    const QString embedded = materializeEmbeddedScript(command);
    return embedded.isEmpty() ? command : embedded;
}

QString windowDetectionInvocation(const QString &command)
{
#if defined(Q_OS_WIN)
    return command;
#else
    if (!isBundledCommand(command)) {
        return command;
    }
    const QString resolved = resolveBundledWindowDetectionScript(
        command, bundledCommandSearchDirectories(QCoreApplication::applicationDirPath(),
                                                 QDir::homePath(), QDir::currentPath()));
    if (resolved == command) {
        markshot::debugLog("window-detection", "【窗口识别】【内置脚本】script-not-found dir=%s command=%s",
                           QCoreApplication::applicationDirPath().toUtf8().constData(),
                           command.toUtf8().constData());
        return command;
    }
    markshot::debugLog("window-detection", "【窗口识别】【内置脚本】script-resolved path=%s",
                       resolved.toUtf8().constData());
    // 1. 【窗口识别】【脚本调用】临时目录可能设置 noexec，资源副本通过 Python 解释器读取
    if (embeddedScripts.exists() && QFileInfo(resolved).absolutePath() == embeddedScripts()->directory.path()) {
        const QString python = QStandardPaths::findExecutable(QStringLiteral("python3"));
        return quoteShellWord(python.isEmpty() ? QStringLiteral("python3") : python)
            + QLatin1Char(' ') + quoteShellWord(resolved);
    }
    return quoteShellWord(resolved);
#endif
}

}  // namespace markshot::window_detection
