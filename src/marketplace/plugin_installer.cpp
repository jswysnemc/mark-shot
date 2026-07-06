#include "marketplace/plugin_installer.h"

#include "providers/provider_plugin_paths.h"

#include <QCryptographicHash>
#include <QDir>
#include <QFile>
#include <QFileInfo>

namespace markshot::marketplace {
namespace {

/**
 * 判断文件名是否不包含路径信息。
 * @param fileName 文件名。
 * @return 安全时返回 true。
 */
bool isSafeFileName(const QString &fileName)
{
    return !fileName.isEmpty()
        && fileName != QStringLiteral(".")
        && fileName != QStringLiteral("..")
        && !fileName.contains(QLatin1Char('/'))
        && !fileName.contains(QLatin1Char('\\'));
}

/**
 * 计算文件 SHA-256 摘要。
 * @param path 文件路径。
 * @param error 错误说明输出。
 * @return 摘要十六进制字符串。
 */
QString fileSha256(const QString &path, QString *error)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        if (error != nullptr) {
            *error = file.errorString();
        }
        return QString();
    }

    QCryptographicHash hash(QCryptographicHash::Sha256);
    while (!file.atEnd()) {
        hash.addData(file.read(1024 * 1024));
    }
    return QString::fromLatin1(hash.result().toHex());
}

/**
 * 构造失败的安装结果。
 * @param error 错误说明。
 * @return 失败结果。
 */
PluginInstallResult failedResult(const QString &error)
{
    PluginInstallResult result;
    result.error = error;
    return result;
}

}  // namespace

bool isSupportedPluginLibraryFile(const QString &fileName)
{
    const QString lower = QFileInfo(fileName).fileName().toLower();
    return lower.endsWith(QStringLiteral(".dll"))
        || lower.endsWith(QStringLiteral(".dylib"))
        || lower.endsWith(QStringLiteral(".so"))
        || lower.contains(QStringLiteral(".so."));
}

PluginInstallResult installPluginAsset(const PluginInstallRequest &request)
{
    const QFileInfo sourceInfo(request.sourcePath);
    if (!sourceInfo.exists() || !sourceInfo.isFile()) {
        return failedResult(QStringLiteral("Plugin source file does not exist"));
    }

    const QString fileName = request.fileName.trimmed().isEmpty()
        ? sourceInfo.fileName()
        : request.fileName.trimmed();
    if (!isSafeFileName(fileName)) {
        return failedResult(QStringLiteral("Plugin file name must not contain path separators"));
    }
    if (!isSupportedPluginLibraryFile(fileName)) {
        return failedResult(QStringLiteral("Plugin asset must be a platform dynamic library"));
    }

    // 1. 安装前再次校验本地文件摘要，确保下载与索引一致
    if (!request.expectedSha256.trimmed().isEmpty()) {
        QString error;
        const QString actualSha256 = fileSha256(sourceInfo.absoluteFilePath(), &error);
        if (!error.isEmpty()) {
            return failedResult(error);
        }
        if (actualSha256 != request.expectedSha256.trimmed().toLower()) {
            return failedResult(QStringLiteral("Plugin asset SHA-256 mismatch"));
        }
    }

    const QString destinationDirectory = request.destinationDirectory.trimmed().isEmpty()
        ? markshot::providers::userPluginDirectory()
        : request.destinationDirectory.trimmed();
    QDir destinationDir(destinationDirectory);
    if (!destinationDir.exists() && !destinationDir.mkpath(QStringLiteral("."))) {
        return failedResult(QStringLiteral("Failed to create plugin destination directory"));
    }

    // 2. 复制到临时文件，再替换最终文件，降低中途失败留下半文件的概率
    const QString destinationPath = destinationDir.absoluteFilePath(fileName);
    const QString temporaryPath = destinationPath + QStringLiteral(".installing");
    QFile::remove(temporaryPath);
    if (!QFile::copy(sourceInfo.absoluteFilePath(), temporaryPath)) {
        return failedResult(QStringLiteral("Failed to copy plugin asset"));
    }
    QFile::remove(destinationPath);
    if (!QFile::rename(temporaryPath, destinationPath)) {
        QFile::remove(temporaryPath);
        return failedResult(QStringLiteral("Failed to move plugin asset into plugin directory"));
    }

    PluginInstallResult result;
    result.success = true;
    result.installedPath = destinationPath;
    return result;
}

}  // namespace markshot::marketplace
