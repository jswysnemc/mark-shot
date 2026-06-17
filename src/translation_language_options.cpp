#include "translation_language_options.h"

#include "app_config_store.h"
#include "ui/i18n.h"

#include <QJsonValue>
#include <QStringList>

namespace markshot {

/// @brief 返回内置翻译目标语言选项
/// @return 语言选项列表,value 是传给翻译器的稳定英文名称,label 是当前界面语言展示文本
QVector<TranslationLanguageOption> translationLanguageOptions()
{
    return {
        {QStringLiteral("Simplified Chinese"), MS_TR("Simplified Chinese")},
        {QStringLiteral("Traditional Chinese"), MS_TR("Traditional Chinese")},
        {QStringLiteral("English"), MS_TR("English")},
        {QStringLiteral("Japanese"), MS_TR("Japanese")},
        {QStringLiteral("Korean"), MS_TR("Korean")},
        {QStringLiteral("French"), MS_TR("French")},
        {QStringLiteral("German"), MS_TR("German")},
        {QStringLiteral("Spanish"), MS_TR("Spanish")},
        {QStringLiteral("Russian"), MS_TR("Russian")},
    };
}

/// @brief 按展示文本或实际值解析目标语言
/// @param text 用户选择或输入的语言文本
/// @return 实际传给翻译器的目标语言名称
QString translationLanguageValueFromText(QString text)
{
    text = text.trimmed();
    for (const TranslationLanguageOption &option : translationLanguageOptions()) {
        if (text.compare(option.label, Qt::CaseInsensitive) == 0
            || text.compare(option.value, Qt::CaseInsensitive) == 0) {
            return option.value;
        }
    }
    return text;
}

/// @brief 保存翻译目标语言到应用配置
/// @param targetLanguage 实际传给翻译器的目标语言名称
/// @param error 保存失败时的错误信息
/// @return 保存成功时返回 true
bool saveTranslationTargetLanguage(QString targetLanguage, QString *error)
{
    targetLanguage = targetLanguage.trimmed();
    if (targetLanguage.isEmpty()) {
        if (error) {
            *error = QStringLiteral("Target language is empty");
        }
        return false;
    }

    return writeAppConfigValue({QStringLiteral("translation"), QStringLiteral("targetLanguage")},
                               QJsonValue(targetLanguage),
                               error);
}

}  // namespace markshot
