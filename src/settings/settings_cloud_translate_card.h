#pragma once

#include "settings/cloud_translate_settings.h"

class QFrame;
class QLineEdit;
class QPlainTextEdit;
class QWidget;

namespace markshot::settings {

/**
 * 云翻译凭据卡片持有的输入控件。
 */
struct CloudTranslateCardWidgets {
    QLineEdit *tencentSecretId = nullptr;
    QLineEdit *tencentSecretKey = nullptr;
    QLineEdit *tencentRegion = nullptr;
    QLineEdit *baiduAppId = nullptr;
    QLineEdit *baiduAppKey = nullptr;
    QLineEdit *youdaoAppKey = nullptr;
    QLineEdit *youdaoAppSecret = nullptr;
    QLineEdit *geminiApiKey = nullptr;
    QLineEdit *geminiModel = nullptr;
    QLineEdit *geminiEndpoint = nullptr;
    QPlainTextEdit *geminiSystemPrompt = nullptr;
    QLineEdit *anthropicApiKey = nullptr;
    QLineEdit *anthropicModel = nullptr;
    QLineEdit *anthropicEndpoint = nullptr;
    QPlainTextEdit *anthropicSystemPrompt = nullptr;
};

/**
 * 创建云翻译凭据设置卡片。
 * @param parent 父控件。
 * @param widgets 输出卡片内的输入控件指针。
 * @return 卡片控件。
 */
QFrame *createCloudTranslateCard(QWidget *parent, CloudTranslateCardWidgets *widgets);

/**
 * 把凭据填入卡片控件。
 * @param widgets 卡片控件。
 * @param settings 云翻译凭据。
 * @return 无返回值。
 */
void applyCloudTranslateSettings(const CloudTranslateCardWidgets &widgets,
                                 const CloudTranslateSettings &settings);

/**
 * 从卡片控件读取凭据。
 * @param widgets 卡片控件。
 * @param settings 输出云翻译凭据。
 * @return 无返回值。
 */
void collectCloudTranslateSettings(const CloudTranslateCardWidgets &widgets,
                                   CloudTranslateSettings *settings);

}  // namespace markshot::settings
