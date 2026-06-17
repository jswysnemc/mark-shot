#pragma once

#include <QString>
#include <QVector>

namespace markshot {

struct TranslationLanguageOption {
    QString value;
    QString label;
};

/// @brief 返回内置翻译目标语言选项
/// @return 语言选项列表,value 是传给翻译器的稳定英文名称,label 是当前界面语言展示文本
QVector<TranslationLanguageOption> translationLanguageOptions();

/// @brief 按展示文本或实际值解析目标语言
/// @param text 用户选择或输入的语言文本
/// @return 实际传给翻译器的目标语言名称
QString translationLanguageValueFromText(QString text);

/// @brief 保存翻译目标语言到应用配置
/// @param targetLanguage 实际传给翻译器的目标语言名称
/// @param error 保存失败时的错误信息
/// @return 保存成功时返回 true
bool saveTranslationTargetLanguage(QString targetLanguage, QString *error = nullptr);

}  // namespace markshot
