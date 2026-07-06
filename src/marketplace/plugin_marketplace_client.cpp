#include "marketplace/plugin_marketplace_client.h"

#include <QNetworkReply>
#include <QNetworkRequest>

namespace markshot::marketplace {

PluginMarketplaceClient::PluginMarketplaceClient(QObject *parent)
    : QObject(parent)
{
}

void PluginMarketplaceClient::fetchIndex(const QUrl &indexUrl)
{
    if (!indexUrl.isValid() || indexUrl.scheme() != QStringLiteral("https")) {
        emit failed(QStringLiteral("Plugin index URL must be a valid HTTPS URL"));
        return;
    }

    // 1. 只允许安全重定向，避免 GitHub Release 或 raw 地址跳转到低安全地址
    QNetworkRequest request(indexUrl);
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute,
                         QNetworkRequest::NoLessSafeRedirectPolicy);
    request.setTransferTimeout(30000);

    QNetworkReply *reply = m_network.get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        if (reply->error() != QNetworkReply::NoError) {
            emit failed(reply->errorString());
            return;
        }

        // 2. 下载完成后立即使用内置解析器校验索引结构
        const PluginIndexParseResult result = parsePluginIndex(reply->readAll());
        if (!result.ok()) {
            emit failed(result.errorMessages().join(QStringLiteral("; ")));
            return;
        }
        emit indexReady(result.index);
    });
}

}  // namespace markshot::marketplace
