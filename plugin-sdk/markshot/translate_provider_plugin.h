#pragma once

#include <QString>
#include <QVector>
#include <QtPlugin>

namespace markshot::plugin {

struct TranslateSegment {
    int id = 0;
    QString text;
};

/**
 * 翻译 provider 插件接口。
 *
 * translate 在工作线程调用，实现必须线程安全且不得访问 GUI。
 */
class TranslateProviderPlugin {
public:
    virtual ~TranslateProviderPlugin() = default;

    /**
     * 读取 provider 唯一标识。
     * @return 稳定的小写短标识。
     */
    virtual QString providerId() const = 0;

    /**
     * 读取展示名称。
     * @return 用于设置页展示的名称。
     */
    virtual QString displayName() const = 0;

    /**
     * 判断 provider 当前是否可用。
     * @param error 输出不可用原因。
     * @return 可用时返回 true。
     */
    virtual bool isAvailable(QString *error) const = 0;

    /**
     * 翻译分段文本。
     * @param segments 输入分段，id 在返回值中保持不变。
     * @param targetLanguage 目标语言名称。
     * @param translations 输出译文分段，缺失的 id 视为翻译失败保留原文。
     * @param error 输出错误信息。
     * @return 翻译成功时返回 true。
     */
    virtual bool translate(const QVector<TranslateSegment> &segments,
                           const QString &targetLanguage,
                           QVector<TranslateSegment> *translations,
                           QString *error) = 0;
};

}  // namespace markshot::plugin

#define MARK_SHOT_TRANSLATE_PROVIDER_PLUGIN_IID "dev.mark-shot.TranslateProviderPlugin/1.0"

Q_DECLARE_INTERFACE(markshot::plugin::TranslateProviderPlugin, MARK_SHOT_TRANSLATE_PROVIDER_PLUGIN_IID)
