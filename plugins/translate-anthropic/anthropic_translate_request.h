#pragma once

#include "anthropic_translate_config.h"
#include "markshot/translate_provider_plugin.h"

#include <QByteArray>
#include <QHash>
#include <QString>
#include <QUrl>
#include <QVector>

namespace markshot::translate_anthropic {

/**
 * 构造 Anthropic messages 端点 URL。
 * @param endpoint 基础端点。
 * @return /v1/messages 完整 URL。
 */
QUrl buildAnthropicUrl(QString endpoint);

/**
 * 构造 Anthropic /v1/messages 请求体。
 * @param config 配置。
 * @param segments 待翻译分段。
 * @param targetLanguage 目标语言。
 * @return JSON 字节数组。
 */
QByteArray buildAnthropicPayload(const AnthropicTranslateConfig &config,
                                const QVector<markshot::plugin::TranslateSegment> &segments,
                                const QString &targetLanguage);

/**
 * 解析 Anthropic 响应体。
 * @param responseBody HTTP 响应体。
 * @param translations 输出 id 到译文的映射。
 * @param error 输出错误信息。
 * @return 解析成功返回 true。
 */
bool parseAnthropicResponse(const QByteArray &responseBody,
                           QHash<int, QString> *translations,
                           QString *error);

/**
 * 同步发送 Anthropic 翻译请求。
 * @param config 配置。
 * @param payload 请求体。
 * @param responseBody 输出响应体。
 * @param error 输出错误信息。
 * @return 请求成功且获得响应时返回 true。
 */
bool sendAnthropicRequest(const AnthropicTranslateConfig &config,
                         const QByteArray &payload,
                         QByteArray *responseBody,
                         QString *error);

}  // namespace markshot::translate_anthropic
