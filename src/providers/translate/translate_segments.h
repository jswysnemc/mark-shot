#pragma once

#include <QByteArray>
#include <QHash>
#include <QRectF>
#include <QString>
#include <QVector>

namespace markshot::providers {

struct TranslateSourceSegment {
    int id = 0;
    QString text;
    QRectF box;
};

/**
 * 从翻译输入 JSON 提取按行合并的源文分段。
 * @param inputJson {tokens, targetLanguage} JSON 字节。
 * @param targetLanguage 输出 JSON 内的目标语言，可为空。
 * @return 分段列表，与旧 helper 的行合并与空格启发式一致。
 */
QVector<TranslateSourceSegment> translateSegmentsFromInputJson(const QByteArray &inputJson,
                                                               QString *targetLanguage);

/**
 * 用译文构建标准 tokens 输出 JSON。
 * @param segments 源文分段。
 * @param translations 分段 id 到译文的映射，缺失时保留原文。
 * @param backend 输出的 backend 标识。
 * @return {backend, tokens, errors} JSON 字节。
 */
QByteArray translateTokensJson(const QVector<TranslateSourceSegment> &segments,
                               const QHash<int, QString> &translations,
                               const QString &backend);

}  // namespace markshot::providers
