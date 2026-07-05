#pragma once

#include <QHash>
#include <QString>

namespace markshot::translate_openai {

/**
 * 从模型回复文本解析翻译结果。
 * @param content 回复正文，支持裸 JSON 或被 Markdown 包裹的 JSON。
 * @param translations 输出分段 id 到译文的映射。
 * @param error 输出错误信息。
 * @return 解析成功时返回 true。
 */
bool parseTranslationContent(const QString &content, QHash<int, QString> *translations, QString *error);

}  // namespace markshot::translate_openai
