#include "marketplace/plugin_asset_downloader.h"

#include <QDir>
#include <QFileInfo>
#include <QNetworkReply>
#include <QNetworkRequest>

namespace markshot::marketplace {

PluginAssetDownloader::PluginAssetDownloader(QObject *parent)
    : QObject(parent)
{
}

bool PluginAssetDownloader::isRunning() const
{
    return !m_reply.isNull();
}

void PluginAssetDownloader::download(const PluginAssetDownloadRequest &request)
{
    if (isRunning()) {
        emit failed(QStringLiteral("Another plugin asset download is already running"));
        return;
    }
    if (!request.url.isValid() || request.url.scheme() != QStringLiteral("https")) {
        emit failed(QStringLiteral("Plugin asset URL must be a valid HTTPS URL"));
        return;
    }
    if (request.destinationPath.trimmed().isEmpty()) {
        emit failed(QStringLiteral("Plugin asset destination path is empty"));
        return;
    }

    const QFileInfo destinationInfo(request.destinationPath);
    QDir().mkpath(destinationInfo.absolutePath());
    m_destinationPath = destinationInfo.absoluteFilePath();
    m_temporaryPath = m_destinationPath + QStringLiteral(".download");
    m_expectedSha256 = request.sha256.trimmed().toLower();
    m_hash.reset();

    m_file = std::make_unique<QFile>(m_temporaryPath);
    if (!m_file->open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        const QString error = m_file->errorString();
        reset();
        emit failed(error);
        return;
    }

    // 1. 使用同一目录下的临时文件，校验通过后再替换目标文件
    QNetworkRequest networkRequest(request.url);
    networkRequest.setAttribute(QNetworkRequest::RedirectPolicyAttribute,
                                QNetworkRequest::NoLessSafeRedirectPolicy);
    networkRequest.setTransferTimeout(60000);
    QNetworkReply *reply = m_network.get(networkRequest);
    m_reply = reply;

    connect(reply, &QNetworkReply::readyRead, this, &PluginAssetDownloader::readAvailableData);
    connect(reply, &QNetworkReply::downloadProgress, this, &PluginAssetDownloader::progress);
    connect(reply, &QNetworkReply::finished, this, &PluginAssetDownloader::finishDownload);
}

void PluginAssetDownloader::reset()
{
    if (m_file) {
        m_file->close();
        m_file.reset();
    }
    if (!m_reply.isNull()) {
        m_reply->deleteLater();
        m_reply.clear();
    }
    m_hash.reset();
    m_expectedSha256.clear();
    m_destinationPath.clear();
    m_temporaryPath.clear();
}

void PluginAssetDownloader::readAvailableData()
{
    if (m_reply.isNull() || !m_file) {
        return;
    }

    const QByteArray data = m_reply->readAll();
    if (data.isEmpty()) {
        return;
    }
    m_hash.addData(data);
    if (m_file->write(data) != data.size()) {
        const QString error = m_file->errorString();
        QFile::remove(m_temporaryPath);
        reset();
        emit failed(error);
    }
}

void PluginAssetDownloader::finishDownload()
{
    if (m_reply.isNull() || !m_file) {
        return;
    }

    const QNetworkReply::NetworkError networkError = m_reply->error();
    const QString networkErrorString = m_reply->errorString();
    readAvailableData();
    m_file->close();

    if (networkError != QNetworkReply::NoError) {
        QFile::remove(m_temporaryPath);
        reset();
        emit failed(networkErrorString);
        return;
    }

    // 2. 校验 GitHub Release 资产摘要，避免下载损坏或被替换的文件
    const QString actualSha256 = QString::fromLatin1(m_hash.result().toHex());
    if (!m_expectedSha256.isEmpty() && actualSha256 != m_expectedSha256) {
        QFile::remove(m_temporaryPath);
        reset();
        emit failed(QStringLiteral("Plugin asset SHA-256 mismatch"));
        return;
    }

    QFile::remove(m_destinationPath);
    if (!QFile::rename(m_temporaryPath, m_destinationPath)) {
        QFile::remove(m_temporaryPath);
        reset();
        emit failed(QStringLiteral("Failed to move plugin asset into destination path"));
        return;
    }

    const QString finalPath = m_destinationPath;
    reset();
    emit finished(finalPath);
}

}  // namespace markshot::marketplace
