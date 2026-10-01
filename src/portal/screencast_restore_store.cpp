#include "portal/screencast_restore_store.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLockFile>
#include <QMutex>
#include <QMutexLocker>
#include <QSaveFile>
#include <QStandardPaths>

#include <optional>
#include <utility>

namespace {

QMutex g_storeMutex;

/** @param error 可选错误输出。 @param message 错误描述。 @return 固定返回 false。 */
bool fail(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
    return false;
}

/** @param path 文件路径。 @param error 错误输出。 @return 目录可用时返回 true。 */
bool prepareDirectory(const QString &path, QString *error)
{
    if (error) {
        error->clear();
    }
    if (path.isEmpty() || !QDir().mkpath(QFileInfo(path).absolutePath())) {
        return fail(error, QStringLiteral("Cannot create ScreenCast authorization directory"));
    }
    return true;
}

/** @param path 文件路径。 @param error 错误输出。 @return 记录对象，读取失败时返回空值。 */
std::optional<QJsonObject> readTokens(const QString &path, QString *error)
{
    QFile file(path);
    if (!file.exists()) {
        return QJsonObject();
    }
    if (!file.open(QIODevice::ReadOnly)) {
        fail(error, QStringLiteral("Cannot read ScreenCast authorization file"));
        return std::nullopt;
    }
    // 1. 【屏幕共享】【授权存储】损坏文件按无授权处理，新授权可重新建立有效记录
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll());
    return document.isObject() ? document.object() : QJsonObject();
}

/** @param path 文件路径。 @param tokens 授权记录。 @param error 错误输出。 @return 提交成功时返回 true。 */
bool writeTokens(const QString &path, const QJsonObject &tokens, QString *error)
{
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)
        || !file.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner)) {
        return fail(error, QStringLiteral("Cannot open private ScreenCast authorization file"));
    }
    // 1. 【屏幕共享】【授权存储】先限制访问权限再写入，完整提交后才允许使用或替换令牌
    const QByteArray data = QJsonDocument(tokens).toJson(QJsonDocument::Compact);
    if (file.write(data) != data.size() || !file.commit()) {
        return fail(error, QStringLiteral("Cannot commit ScreenCast authorization file"));
    }
    return true;
}

}  // namespace

namespace markshot::portal {

ScreenCastRestoreStore::ScreenCastRestoreStore(QString path)
    : m_path(std::move(path))
{
    if (m_path.isEmpty()) {
        const QString directory = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
        if (!directory.isEmpty()) {
            m_path = QDir(directory).filePath(QStringLiteral("portal/screencast-authorizations.json"));
        }
    }
}

QString ScreenCastRestoreStore::take(const QString &scope, QString *error) const
{
    QMutexLocker guard(&g_storeMutex);
    if (!prepareDirectory(m_path, error)) {
        return {};
    }
    QLockFile lock(m_path + QStringLiteral(".lock"));
    if (!lock.tryLock(100)) {
        fail(error, QStringLiteral("ScreenCast authorization file is busy"));
        return {};
    }
    auto tokens = readTokens(m_path, error);
    if (!tokens) {
        return {};
    }
    const QString token = tokens->take(scope).toString();
    if (token.isEmpty() || !writeTokens(m_path, *tokens, error)) {
        return {};
    }
    return token;
}

bool ScreenCastRestoreStore::save(const QString &scope, const QString &token, QString *error) const
{
    QMutexLocker guard(&g_storeMutex);
    if (!prepareDirectory(m_path, error)) {
        return false;
    }
    QLockFile lock(m_path + QStringLiteral(".lock"));
    if (!lock.tryLock(100)) {
        return fail(error, QStringLiteral("ScreenCast authorization file is busy"));
    }
    auto tokens = readTokens(m_path, error);
    if (!tokens) {
        return false;
    }
    if (scope.isEmpty() || token.isEmpty()) {
        return fail(error, QStringLiteral("ScreenCast authorization scope or token is empty"));
    }
    tokens->insert(scope, token);
    return writeTokens(m_path, *tokens, error);
}

bool ScreenCastRestoreStore::forget(const QString &scope, const QString &token, QString *error) const
{
    QMutexLocker guard(&g_storeMutex);
    if (!prepareDirectory(m_path, error)) {
        return false;
    }
    QLockFile lock(m_path + QStringLiteral(".lock"));
    if (!lock.tryLock(100)) {
        return fail(error, QStringLiteral("ScreenCast authorization file is busy"));
    }
    auto tokens = readTokens(m_path, error);
    if (!tokens) {
        return false;
    }
    if (tokens->value(scope).toString() != token || token.isEmpty()) {
        return true;
    }
    tokens->remove(scope);
    return writeTokens(m_path, *tokens, error);
}

}  // namespace markshot::portal
