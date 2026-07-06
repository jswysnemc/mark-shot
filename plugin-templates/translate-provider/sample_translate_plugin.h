#pragma once

#include "markshot/translate_provider_plugin.h"

#include <QObject>

class SampleTranslatePlugin final : public QObject, public markshot::plugin::TranslateProviderPlugin {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_TRANSLATE_PROVIDER_PLUGIN_IID FILE "metadata.json")
    Q_INTERFACES(markshot::plugin::TranslateProviderPlugin)

public:
    /**
     * 读取 provider 唯一标识。
     * @return 小写短标识，用户配置会引用该值。
     */
    QString providerId() const override;

    /**
     * 读取设置页展示名称。
     * @return 展示名称。
     */
    QString displayName() const override;

    /**
     * 检查翻译 provider 当前是否可用。
     * @param error 不可用时输出原因。
     * @return 可用时返回 true。
     */
    bool isAvailable(QString *error) const override;

    /**
     * 翻译文本分段。
     * @param segments 输入文本分段。
     * @param targetLanguage 目标语言。
     * @param translations 输出翻译结果。
     * @param error 失败时输出错误信息。
     * @return 翻译流程成功时返回 true。
     */
    bool translate(const QVector<markshot::plugin::TranslateSegment> &segments,
                   const QString &targetLanguage,
                   QVector<markshot::plugin::TranslateSegment> *translations,
                   QString *error) override;
};
