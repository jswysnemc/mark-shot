#include "rapid_runtime_paths.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QLibraryInfo>

#ifdef Q_OS_WIN
#include <qt_windows.h>
#else
#include <dlfcn.h>
#endif

namespace markshot::ocr_rapid {
namespace {

/** 【OCR】【模块路径】通过本模块内的数据地址定位插件文件，失败返回空串，无参数。 */
QString modulePath()
{
    static const char anchor = 0;
#ifdef Q_OS_WIN
    HMODULE module = nullptr;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                               | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                           reinterpret_cast<LPCWSTR>(&anchor), &module)) {
        return {};
    }
    wchar_t path[32768];
    const DWORD size = GetModuleFileNameW(module, path, 32768);
    return size > 0 && size < 32768 ? QString::fromWCharArray(path, static_cast<int>(size)) : QString();
#else
    Dl_info info{};
    return dladdr(&anchor, &info) && info.dli_fname ? QString::fromLocal8Bit(info.dli_fname) : QString();
#endif
}

/** 【OCR】【路径候选】向 paths 追加 directory 中实际存在的 names，去重但保持优先级，无返回值。 */
void appendLibraries(QStringList *paths, const QString &directory, const QStringList &names)
{
    for (const QString &name : names) {
        const QFileInfo file(QDir(directory).filePath(name));
        if (!file.isFile()) continue;
        const QString path = file.canonicalFilePath();
        if (!path.isEmpty() && !paths->contains(path)) paths->append(path);
    }
}

}  // namespace

QStringList rapidOnnxRuntimeCandidates()
{
    const QString specified = qEnvironmentVariable("MARK_SHOT_RAPID_ONNX_RUNTIME").trimmed();
    if (!specified.isEmpty()) return {specified};

#ifdef Q_OS_WIN
    const QStringList names{QStringLiteral("onnxruntime.dll"), QStringLiteral("libonnxruntime.dll")};
#elif defined(Q_OS_MACOS)
    const QStringList names{QStringLiteral("libonnxruntime.1.dylib"), QStringLiteral("libonnxruntime.dylib")};
#else
    const QStringList names{QStringLiteral("libonnxruntime.so.1"), QStringLiteral("libonnxruntime.so")};
#endif
    QStringList paths;
    const QDir app(QCoreApplication::applicationDirPath());
    const QString ownPath = modulePath();
    // 1. 【OCR】【私有运行库】优先插件与应用随包提供的版本，避免误选系统同名旧库
    if (!ownPath.isEmpty()) {
        const QDir plugin = QFileInfo(ownPath).absoluteDir();
        for (const QString &directory : {plugin.absolutePath(), plugin.filePath(QStringLiteral("../lib")),
                                         plugin.filePath(QStringLiteral("../.."))}) {
            appendLibraries(&paths, directory, names);
        }
    }
    for (const QString &directory : {app.absolutePath(), app.filePath(QStringLiteral("../lib")),
                                     app.filePath(QStringLiteral("../Frameworks")),
                                     QLibraryInfo::path(QLibraryInfo::LibrariesPath)}) {
        appendLibraries(&paths, directory, names);
    }

    // 2. 【OCR】【系统回退】保留标准加载器搜索；若旧同名库不兼容，再尝试系统绝对路径
    paths.append(names);
#ifdef Q_OS_LINUX
    QStringList directories{QStringLiteral("/usr/lib64"), QStringLiteral("/usr/lib"),
                            QStringLiteral("/lib64"), QStringLiteral("/lib")};
    for (const QString &name : QDir(QStringLiteral("/usr/lib")).entryList(
             {QStringLiteral("*-linux-gnu"), QStringLiteral("*-linux-gnueabihf")},
             QDir::Dirs | QDir::NoDotAndDotDot)) {
        directories.append(QStringLiteral("/usr/lib/") + name);
    }
    for (const QString &directory : directories) appendLibraries(&paths, directory, names);
#endif
    return paths;
}

}  // namespace markshot::ocr_rapid
