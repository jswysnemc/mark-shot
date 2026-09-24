#pragma once

#include <QJsonObject>
#include <QPair>
#include <QString>
#include <QStringList>
#include <QVector>

namespace markshot::settings {

/**
 * 云翻译服务凭据。
 *
 * 各厂商插件按 translation.<vendor> 子节读取同名字段，设置界面在此集中维护。
 */
struct CloudTranslateSettings {
    QString tencentSecretId;
    QString tencentSecretKey;
    QString tencentRegion = QStringLiteral("ap-guangzhou");
    QString baiduAppId;
    QString baiduAppKey;
    QString youdaoAppKey;
    QString youdaoAppSecret;
    QString geminiApiKey;
    QString geminiModel;
    QString geminiEndpoint;
    QString geminiSystemPrompt;
    QString anthropicApiKey;
    QString anthropicModel;
    QString anthropicEndpoint;
    QString anthropicSystemPrompt;
};

/**
 * 从 translation 节读取云翻译凭据。
 * @param translation 应用配置中的 translation 对象。
 * @return 云翻译凭据，缺失字段保留默认值。
 */
CloudTranslateSettings readCloudTranslateSettings(const QJsonObject &translation);

/**
 * 展开云翻译凭据对应的配置路径与取值。
 * @param settings 云翻译凭据。
 * @return 路径与取值组成的写入条目，路径以 translation 开头。
 */
QVector<QPair<QStringList, QString>> cloudTranslateConfigEntries(const CloudTranslateSettings &settings);

}  // namespace markshot::settings
