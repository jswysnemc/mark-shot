#include "settings/settings_cloud_translate_card.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QFormLayout>
#include <QFrame>
#include <QLineEdit>
#include <QPlainTextEdit>

namespace markshot::settings {
namespace {

/**
 * 添加密钥输入项，输入后以掩码显示。
 * @param form 表单布局。
 * @param label 标签文本。
 * @param placeholder 占位文本。
 * @return 文本输入控件。
 */
QLineEdit *addSecretRow(QFormLayout *form, const QString &label, const QString &placeholder)
{
    QLineEdit *edit = addTextRow(form, label, placeholder);
    edit->setEchoMode(QLineEdit::PasswordEchoOnEdit);
    return edit;
}

}  // namespace

QFrame *createCloudTranslateCard(QWidget *parent, CloudTranslateCardWidgets *widgets)
{
    QFrame *card = createAdvancedSettingsCard(
        MS_TR("Cloud Translation Credentials"),
        MS_TR("Credentials for the Tencent, Baidu, Youdao, Gemini, and Anthropic translation plugins. "
              "Leave a service empty to keep reading its credentials from environment variables."),
        parent);
    if (!widgets) {
        return card;
    }

    QFormLayout *form = settingsCardForm(card);

    // 1. 腾讯云机器翻译，密钥可改用 TENCENTCLOUD_SECRET_ID 等环境变量
    widgets->tencentSecretId =
        addTextRow(form, MS_TR("Tencent SecretId"), QStringLiteral("AKID..."));
    widgets->tencentSecretKey =
        addSecretRow(form, MS_TR("Tencent SecretKey"), QStringLiteral("TENCENTCLOUD_SECRET_KEY"));
    widgets->tencentRegion =
        addTextRow(form, MS_TR("Tencent Region"), QStringLiteral("ap-guangzhou"));

    // 2. 百度翻译开放平台
    widgets->baiduAppId = addTextRow(form, MS_TR("Baidu AppID"), QStringLiteral("20150630..."));
    widgets->baiduAppKey =
        addSecretRow(form, MS_TR("Baidu Secret Key"), QStringLiteral("MARK_SHOT_BAIDU_APP_KEY"));

    // 3. 网易有道智云
    widgets->youdaoAppKey = addTextRow(form, MS_TR("Youdao AppKey"), QStringLiteral("MARK_SHOT_YOUDAO_APP_KEY"));
    widgets->youdaoAppSecret =
        addSecretRow(form, MS_TR("Youdao App Secret"), QStringLiteral("MARK_SHOT_YOUDAO_APP_SECRET"));

    // 4. Google Gemini
    widgets->geminiApiKey =
        addSecretRow(form, MS_TR("Gemini API Key"), QStringLiteral("GEMINI_API_KEY"));
    widgets->geminiModel =
        addTextRow(form, MS_TR("Gemini Model"), QStringLiteral("gemini-3.5-flash-lite"));
    widgets->geminiEndpoint =
        addTextRow(form, MS_TR("Gemini Endpoint"), QStringLiteral("https://generativelanguage.googleapis.com"));

    // 5. Anthropic Claude
    widgets->anthropicApiKey =
        addSecretRow(form, MS_TR("Anthropic API Key"), QStringLiteral("ANTHROPIC_API_KEY"));
    widgets->anthropicModel =
        addTextRow(form, MS_TR("Anthropic Model"), QStringLiteral("claude-haiku-4-5"));
    widgets->anthropicEndpoint =
        addTextRow(form, MS_TR("Anthropic Endpoint"), QStringLiteral("https://api.anthropic.com"));
    return card;
}

void applyCloudTranslateSettings(const CloudTranslateCardWidgets &widgets,
                                 const CloudTranslateSettings &settings)
{
    if (widgets.tencentSecretId) {
        widgets.tencentSecretId->setText(settings.tencentSecretId);
    }
    if (widgets.tencentSecretKey) {
        widgets.tencentSecretKey->setText(settings.tencentSecretKey);
    }
    if (widgets.tencentRegion) {
        widgets.tencentRegion->setText(settings.tencentRegion);
    }
    if (widgets.baiduAppId) {
        widgets.baiduAppId->setText(settings.baiduAppId);
    }
    if (widgets.baiduAppKey) {
        widgets.baiduAppKey->setText(settings.baiduAppKey);
    }
    if (widgets.youdaoAppKey) {
        widgets.youdaoAppKey->setText(settings.youdaoAppKey);
    }
    if (widgets.youdaoAppSecret) {
        widgets.youdaoAppSecret->setText(settings.youdaoAppSecret);
    }
    if (widgets.geminiApiKey) {
        widgets.geminiApiKey->setText(settings.geminiApiKey);
    }
    if (widgets.geminiModel) {
        widgets.geminiModel->setText(settings.geminiModel);
    }
    if (widgets.geminiEndpoint) {
        widgets.geminiEndpoint->setText(settings.geminiEndpoint);
    }
    if (widgets.geminiSystemPrompt) {
        widgets.geminiSystemPrompt->setPlainText(settings.geminiSystemPrompt);
    }
    if (widgets.anthropicApiKey) {
        widgets.anthropicApiKey->setText(settings.anthropicApiKey);
    }
    if (widgets.anthropicModel) {
        widgets.anthropicModel->setText(settings.anthropicModel);
    }
    if (widgets.anthropicEndpoint) {
        widgets.anthropicEndpoint->setText(settings.anthropicEndpoint);
    }
    if (widgets.anthropicSystemPrompt) {
        widgets.anthropicSystemPrompt->setPlainText(settings.anthropicSystemPrompt);
    }
}

void collectCloudTranslateSettings(const CloudTranslateCardWidgets &widgets,
                                   CloudTranslateSettings *settings)
{
    if (!settings) {
        return;
    }
    if (widgets.tencentSecretId) {
        settings->tencentSecretId = widgets.tencentSecretId->text().trimmed();
    }
    if (widgets.tencentSecretKey) {
        settings->tencentSecretKey = widgets.tencentSecretKey->text().trimmed();
    }
    if (widgets.tencentRegion) {
        settings->tencentRegion = widgets.tencentRegion->text().trimmed();
    }
    if (widgets.baiduAppId) {
        settings->baiduAppId = widgets.baiduAppId->text().trimmed();
    }
    if (widgets.baiduAppKey) {
        settings->baiduAppKey = widgets.baiduAppKey->text().trimmed();
    }
    if (widgets.youdaoAppKey) {
        settings->youdaoAppKey = widgets.youdaoAppKey->text().trimmed();
    }
    if (widgets.youdaoAppSecret) {
        settings->youdaoAppSecret = widgets.youdaoAppSecret->text().trimmed();
    }
    if (widgets.geminiApiKey) {
        settings->geminiApiKey = widgets.geminiApiKey->text().trimmed();
    }
    if (widgets.geminiModel) {
        settings->geminiModel = widgets.geminiModel->text().trimmed();
    }
    if (widgets.geminiEndpoint) {
        settings->geminiEndpoint = widgets.geminiEndpoint->text().trimmed();
    }
    if (widgets.geminiSystemPrompt) {
        settings->geminiSystemPrompt = widgets.geminiSystemPrompt->toPlainText().trimmed();
    }
    if (widgets.anthropicApiKey) {
        settings->anthropicApiKey = widgets.anthropicApiKey->text().trimmed();
    }
    if (widgets.anthropicModel) {
        settings->anthropicModel = widgets.anthropicModel->text().trimmed();
    }
    if (widgets.anthropicEndpoint) {
        settings->anthropicEndpoint = widgets.anthropicEndpoint->text().trimmed();
    }
    if (widgets.anthropicSystemPrompt) {
        settings->anthropicSystemPrompt = widgets.anthropicSystemPrompt->toPlainText().trimmed();
    }
}

}  // namespace markshot::settings
