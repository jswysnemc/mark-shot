#pragma once

#include <QCryptographicHash>
#include <QFile>
#include <QNetworkAccessManager>
#include <QObject>
#include <QPointer>
#include <QUrl>

#include <memory>

class QNetworkReply;

namespace markshot::marketplace {

struct PluginAssetDownloadRequest {
    QUrl url;
    QString sha256;
    QString destinationPath;
};

class PluginAssetDownloader final : public QObject {
    Q_OBJECT

public:
    /**
     * 构造插件资产下载器。
     * @param parent Qt 父对象。
     */
    explicit PluginAssetDownloader(QObject *parent = nullptr);

    /**
     * 判断当前是否正在下载。
     * @return 正在下载时返回 true。
     */
    bool isRunning() const;

    /**
     * 下载插件资产并校验 SHA-256。
     * @param request 下载请求。
     * @return 无返回值。
     */
    void download(const PluginAssetDownloadRequest &request);

signals:
    /**
     * 下载进度变更后触发。
     * @param received 已接收字节数。
     * @param total 总字节数。
     * @return 无返回值。
     */
    void progress(qint64 received, qint64 total);

    /**
     * 下载成功后触发。
     * @param destinationPath 最终文件路径。
     * @return 无返回值。
     */
    void finished(const QString &destinationPath);

    /**
     * 下载失败后触发。
     * @param error 错误说明。
     * @return 无返回值。
     */
    void failed(const QString &error);

private:
    /**
     * 清理当前下载状态。
     * @return 无返回值。
     */
    void reset();

    /**
     * 处理网络响应可读数据。
     * @return 无返回值。
     */
    void readAvailableData();

    /**
     * 处理网络响应结束。
     * @return 无返回值。
     */
    void finishDownload();

    QNetworkAccessManager m_network;
    QPointer<QNetworkReply> m_reply;
    std::unique_ptr<QFile> m_file;
    QCryptographicHash m_hash{QCryptographicHash::Sha256};
    QString m_expectedSha256;
    QString m_destinationPath;
    QString m_temporaryPath;
};

}  // namespace markshot::marketplace
