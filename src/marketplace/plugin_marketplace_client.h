#pragma once

#include "marketplace/plugin_index_parser.h"

#include <QNetworkAccessManager>
#include <QObject>
#include <QUrl>

namespace markshot::marketplace {

class PluginMarketplaceClient final : public QObject {
    Q_OBJECT

public:
    /**
     * 构造插件市场客户端。
     * @param parent Qt 父对象。
     */
    explicit PluginMarketplaceClient(QObject *parent = nullptr);

    /**
     * 下载并解析插件市场索引。
     * @param indexUrl 插件索引 URL。
     * @return 无返回值。
     */
    void fetchIndex(const QUrl &indexUrl);

signals:
    /**
     * 插件索引解析成功后触发。
     * @param index 插件索引。
     * @return 无返回值。
     */
    void indexReady(const markshot::marketplace::PluginIndex &index);

    /**
     * 插件索引下载或解析失败后触发。
     * @param error 错误说明。
     * @return 无返回值。
     */
    void failed(const QString &error);

private:
    QNetworkAccessManager m_network;
};

}  // namespace markshot::marketplace
