#pragma once

#include "gemini_translate_config.h"
#include "markshot/translate_provider_plugin.h"

#include <QByteArray>
#include <QHash>
#include <QString>
#include <QUrl>
#include <QVector>

namespace markshot::translate_gemini {

/**
 * 构造 Gemini generateContent 端点 URL。
 * @param endpoint 基础端点。
 * @param model 模型名称。
 * @return generateContent 完整 URL。
 */
QUrl buildGeminiUrl(QString endpoint, const QString &model);

/**
 * 构造 Gemini generateContent 请求体。
 * @param config 配置。
 * @param segments 待翻译分段。
 * @param targetLanguage 目标语言。
 * @return JSON 字节数组。
 */
QByteArray buildGeminiPayload(const GeminiTranslateConfig &config,
                              const QVector<markshot::plugin::TranslateSegment> &segments,
                              const QString &targetLanguage);

/**
 * 解析 Gemini 响应体。
 * @param responseBody HTTP 响应体。
 * @param translations 输出 id 到译文的映射。
 * @param error 输出错误信息。
 * @return 解析成功返回 true。
 */
bool parseGeminiResponse(const QByteArray &responseBody,
                         QHash<int, QString> *translations,
                         QString *error);

/**
 * 同步发送 Gemini 翻译请求。
 * @param config 配置。
 * @param payload 请求体。
 * @param responseBody 输出响应体。
 * @param error 输出错误信息。
 * @return 请求成功且获得响应时返回 true。
 */
bool sendGeminiRequest(const GeminiTranslateConfig &config,
                       const QByteArray &payload,
                       QByteArray *responseBody,
                       QString *error);

}  // namespace markshot::translate_gemini
