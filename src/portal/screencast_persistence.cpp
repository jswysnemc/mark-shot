#include "portal/screencast_persistence.h"

#include "debug_log.h"

#include <QCryptographicHash>
#include <QJsonArray>
#include <QJsonDocument>

#include <utility>

namespace markshot::portal {

ScreenCastPersistence::ScreenCastPersistence(const CaptureRequest &request, bool recording, QString storePath)
    : m_store(std::move(storePath))
{
    const QJsonArray scope{
        qEnvironmentVariable("XDG_CURRENT_DESKTOP").trimmed().toLower(),
        request.preferredOutputName,
        request.allOutputs,
        request.includeCursor,
        recording,
    };
    m_scope = QString::fromLatin1(QCryptographicHash::hash(
        QJsonDocument(scope).toJson(QJsonDocument::Compact), QCryptographicHash::Sha256).toHex());
}

QVariantMap ScreenCastPersistence::prepare(uint portalVersion)
{
    m_supported = portalVersion >= 4;
    m_savedToken.clear();
    if (!m_supported) {
        return {};
    }

    QString error;
    const QString token = m_store.take(m_scope, &error);
    if (!error.isEmpty()) {
        debugLog("screencast", "【屏幕共享】【授权读取】failed error=%s", error.toUtf8().constData());
    }
    QVariantMap options{{QStringLiteral("persist_mode"), uint(2)}};
    if (!token.isEmpty()) {
        options.insert(QStringLiteral("restore_token"), token);
    }
    debugLog("screencast", "【屏幕共享】【授权恢复】requested=1 restore_available=%d", !token.isEmpty());
    return options;
}

void ScreenCastPersistence::accept(const QString &token)
{
    if (!m_supported || token.isEmpty()) {
        return;
    }
    QString error;
    if (!m_store.save(m_scope, token, &error)) {
        debugLog("screencast", "【屏幕共享】【授权保存】failed error=%s", error.toUtf8().constData());
        return;
    }
    m_savedToken = token;
    debugLog("screencast", "【屏幕共享】【授权保存】updated=1");
}

void ScreenCastPersistence::invalidate()
{
    if (m_savedToken.isEmpty()) {
        return;
    }
    QString error;
    if (!m_store.forget(m_scope, m_savedToken, &error)) {
        debugLog("screencast", "【屏幕共享】【授权失效】failed error=%s", error.toUtf8().constData());
    }
    m_savedToken.clear();
}

}  // namespace markshot::portal
