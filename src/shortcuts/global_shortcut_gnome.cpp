#include "shortcuts/global_shortcut_gnome.h"

#include "debug_log.h"
#include "shortcuts/global_shortcut_keymap.h"
#include "ui/i18n.h"

#include <QProcess>
#include <QProcessEnvironment>
#include <QStandardPaths>

namespace markshot::shortcuts {
namespace {

/// @brief media-keys 插件的 gsettings schema。
const char kMediaKeysSchema[] = "org.gnome.settings-daemon.plugins.media-keys";
/// @brief 自定义快捷键列表键名。
const char kCustomKeybindingsKey[] = "custom-keybindings";
/// @brief 自定义快捷键条目的 relocatable schema。
const char kCustomKeybindingSchema[] = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
/// @brief 本应用写入的 keybinding 路径前缀，注销与清理遗留条目时按此识别。
const char kKeybindingPathPrefix[] =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/mark-shot-";

/**
 * 同步执行一次 gsettings 命令。
 *
 * @param arguments 命令参数。
 * @param standardOutput 可选，接收去除首尾空白的标准输出。
 * @param errorText 可选，接收失败原因。
 * @return 进程正常退出且退出码为 0 时返回 true。
 */
bool runGsettings(const QStringList &arguments, QString *standardOutput, QString *errorText)
{
    QProcess process;
    process.start(QStringLiteral("gsettings"), arguments);
    if (!process.waitForStarted(3000)) {
        if (errorText) {
            *errorText = QStringLiteral("failed to start gsettings");
        }
        return false;
    }
    if (!process.waitForFinished(3000)) {
        process.kill();
        process.waitForFinished(1000);
        if (errorText) {
            *errorText = QStringLiteral("gsettings timed out");
        }
        return false;
    }
    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0) {
        if (errorText) {
            const QString stderrText = QString::fromUtf8(process.readAllStandardError()).trimmed();
            *errorText = stderrText.isEmpty()
                ? QStringLiteral("gsettings exited with code %1").arg(process.exitCode())
                : stderrText;
        }
        return false;
    }
    if (standardOutput) {
        *standardOutput = QString::fromUtf8(process.readAllStandardOutput()).trimmed();
    }
    return true;
}

/**
 * 解析 gsettings 输出的字符串数组字面量。
 *
 * 输入形如 "@as []" 或 "['/path1/', '/path2/']"，提取每个单引号包裹的元素。
 *
 * @param raw gsettings get 的输出。
 * @return 数组元素列表。
 */
QStringList parseKeybindingList(const QString &raw)
{
    QStringList paths;
    int cursor = 0;
    while (true) {
        const int begin = raw.indexOf(QLatin1Char('\''), cursor);
        if (begin < 0) {
            break;
        }
        const int end = raw.indexOf(QLatin1Char('\''), begin + 1);
        if (end < 0) {
            break;
        }
        paths.append(raw.mid(begin + 1, end - begin - 1));
        cursor = end + 1;
    }
    return paths;
}

/**
 * 把路径列表序列化为 gsettings 接受的数组字面量。
 *
 * @param paths keybinding 路径列表。
 * @return 数组字面量文本。
 */
QString serializeKeybindingList(const QStringList &paths)
{
    if (paths.isEmpty()) {
        return QStringLiteral("@as []");
    }
    QStringList quoted;
    quoted.reserve(paths.size());
    for (const QString &path : paths) {
        quoted.append(QStringLiteral("'%1'").arg(path));
    }
    return QStringLiteral("[%1]").arg(quoted.join(QStringLiteral(", ")));
}

/**
 * 读取当前的自定义快捷键路径列表。
 *
 * @param paths 接收路径列表。
 * @param errorText 可选，接收失败原因。
 * @return 读取成功返回 true。
 */
bool readKeybindingList(QStringList *paths, QString *errorText)
{
    QString output;
    if (!runGsettings({QStringLiteral("get"),
                       QLatin1String(kMediaKeysSchema),
                       QLatin1String(kCustomKeybindingsKey)},
                      &output, errorText)) {
        return false;
    }
    *paths = parseKeybindingList(output);
    return true;
}

/**
 * 写回自定义快捷键路径列表。
 *
 * @param paths 路径列表。
 * @param errorText 可选，接收失败原因。
 * @return 写入成功返回 true。
 */
bool writeKeybindingList(const QStringList &paths, QString *errorText)
{
    return runGsettings({QStringLiteral("set"),
                         QLatin1String(kMediaKeysSchema),
                         QLatin1String(kCustomKeybindingsKey),
                         serializeKeybindingList(paths)},
                        nullptr, errorText);
}

/**
 * 设置一条 keybinding 的单个键值。
 *
 * @param path keybinding 路径。
 * @param key 键名（name/command/binding）。
 * @param value 键值。
 * @param errorText 可选，接收失败原因。
 * @return 写入成功返回 true。
 */
bool writeKeybindingValue(const QString &path,
                          const QString &key,
                          const QString &value,
                          QString *errorText)
{
    return runGsettings({QStringLiteral("set"),
                         QStringLiteral("%1:%2").arg(QLatin1String(kCustomKeybindingSchema), path),
                         key,
                         value},
                        nullptr, errorText);
}

/**
 * 清空一条 keybinding 在 dconf 中的全部键值。
 *
 * @param path keybinding 路径。
 * @return 无返回值；失败只记录日志。
 */
void resetKeybindingEntry(const QString &path)
{
    QString errorText;
    if (!runGsettings({QStringLiteral("reset-recursively"),
                       QStringLiteral("%1:%2").arg(QLatin1String(kCustomKeybindingSchema), path)},
                      nullptr, &errorText)) {
        markshot::debugLog("shortcuts",
                           "【全局快捷键】【GNOME】reset failed path=%s error=%s",
                           path.toUtf8().constData(),
                           errorText.toUtf8().constData());
    }
}

/**
 * 从列表中剔除本应用写入的 keybinding 路径。
 *
 * @param paths 原始路径列表。
 * @return 剔除后的列表。
 */
QStringList withoutMarkShotEntries(const QStringList &paths)
{
    QStringList filtered;
    filtered.reserve(paths.size());
    for (const QString &path : paths) {
        if (!path.startsWith(QLatin1String(kKeybindingPathPrefix))) {
            filtered.append(path);
        }
    }
    return filtered;
}

}  // namespace

GnomeGlobalShortcutBackend::~GnomeGlobalShortcutBackend()
{
    unregisterShortcuts();
}

bool GnomeGlobalShortcutBackend::isAvailable()
{
    const QString desktop = QProcessEnvironment::systemEnvironment()
                                .value(QStringLiteral("XDG_CURRENT_DESKTOP"));
    if (!desktop.contains(QStringLiteral("gnome"), Qt::CaseInsensitive)) {
        return false;
    }
    return !QStandardPaths::findExecutable(QStringLiteral("gsettings")).isEmpty();
}

bool GnomeGlobalShortcutBackend::registerShortcuts(const QList<Shortcut> &shortcuts)
{
    m_errorString.clear();
    unregisterShortcuts();

    // 1. 转换出可注册的条目；无命令行或无法表达的组合跳过
    struct Entry {
        QString path;
        QString name;
        QString command;
        QString binding;
    };
    QList<Entry> entries;
    for (const Shortcut &shortcut : shortcuts) {
        if (shortcut.id.isEmpty() || shortcut.commandLine.isEmpty()) {
            continue;
        }
        const QString binding = gnomeAcceleratorForSequence(shortcut.sequence);
        if (binding.isEmpty()) {
            markshot::debugLog("shortcuts",
                               "【全局快捷键】【GNOME】unsupported sequence id=%s sequence=%s",
                               shortcut.id.toUtf8().constData(),
                               shortcut.sequence.toString().toUtf8().constData());
            continue;
        }
        Entry entry;
        entry.path = QStringLiteral("%1%2/").arg(QLatin1String(kKeybindingPathPrefix), shortcut.id);
        entry.name = QStringLiteral("Mark Shot: %1").arg(
            shortcut.description.isEmpty() ? shortcut.id : shortcut.description);
        entry.command = shortcut.commandLine;
        entry.binding = binding;
        entries.append(entry);
    }
    if (entries.isEmpty()) {
        m_errorString = MS_TR("No global shortcuts could be registered through GNOME settings.");
        return false;
    }

    // 2. 合并进 custom-keybindings 列表；顺带清掉上次异常退出遗留的条目
    QStringList paths;
    QString errorText;
    if (!readKeybindingList(&paths, &errorText)) {
        m_errorString = errorText;
        return false;
    }
    paths = withoutMarkShotEntries(paths);
    for (const Entry &entry : entries) {
        paths.append(entry.path);
    }
    if (!writeKeybindingList(paths, &errorText)) {
        m_errorString = errorText;
        return false;
    }

    // 3. 写入每条快捷键的名称、命令与按键
    bool anyRegistered = false;
    for (const Entry &entry : entries) {
        if (!writeKeybindingValue(entry.path, QStringLiteral("name"), entry.name, &errorText)
            || !writeKeybindingValue(entry.path, QStringLiteral("command"), entry.command, &errorText)
            || !writeKeybindingValue(entry.path, QStringLiteral("binding"), entry.binding, &errorText)) {
            markshot::debugLog("shortcuts",
                               "【全局快捷键】【GNOME】write failed path=%s error=%s",
                               entry.path.toUtf8().constData(),
                               errorText.toUtf8().constData());
            resetKeybindingEntry(entry.path);
            continue;
        }
        m_registeredPaths.append(entry.path);
        anyRegistered = true;
        markshot::debugLog("shortcuts",
                           "【全局快捷键】【GNOME】registered path=%s binding=%s command=%s",
                           entry.path.toUtf8().constData(),
                           entry.binding.toUtf8().constData(),
                           entry.command.toUtf8().constData());
    }

    if (!anyRegistered) {
        // 写入全部失败时撤掉列表改动，避免留下指向空条目的路径
        QStringList cleanupPaths;
        if (readKeybindingList(&cleanupPaths, nullptr)) {
            writeKeybindingList(withoutMarkShotEntries(cleanupPaths), nullptr);
        }
        m_errorString = errorText.isEmpty()
            ? MS_TR("Failed to write GNOME custom keybindings.")
            : errorText;
        return false;
    }
    return true;
}

void GnomeGlobalShortcutBackend::unregisterShortcuts()
{
    if (m_registeredPaths.isEmpty()) {
        return;
    }

    QStringList paths;
    if (readKeybindingList(&paths, nullptr)) {
        writeKeybindingList(withoutMarkShotEntries(paths), nullptr);
    }
    for (const QString &path : m_registeredPaths) {
        resetKeybindingEntry(path);
    }
    m_registeredPaths.clear();
}

QString GnomeGlobalShortcutBackend::errorString() const
{
    return m_errorString;
}

QString GnomeGlobalShortcutBackend::backendName() const
{
    return QStringLiteral("gnome");
}

}  // namespace markshot::shortcuts
