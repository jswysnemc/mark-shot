#pragma once

#include <QString>

#include <optional>

namespace markshot::translate_anthropic {

struct AnthropicTranslateConfig {
    QString endpoint = QStringLiteral("https://api.anthropic.com/v1");
    QString apiKey;
    QString model = QStringLiteral("claude-haiku-4-5");
    QString systemPrompt;
    int maxTokens = 4096;
    // 未配置时不下发：Opus 4.7 及更新的模型传入 temperature 会返回 400
    std::optional<double> temperature;
    int timeoutMs = 60000;
};

/**
 * 读取 Anthropic Claude 翻译插件配置。
 * @return 合并应用配置、环境变量与默认值后的配置。
 */
AnthropicTranslateConfig readAnthropicTranslateConfig();

/**
 * 校验翻译配置是否具备发起请求的必要字段。
 * @param config 待校验配置。
 * @param error 输出错误信息。
 * @return 配置可用时返回 true。
 */
bool validateAnthropicTranslateConfig(const AnthropicTranslateConfig &config, QString *error);

/**
 * 读取默认翻译系统提示词。
 * @return 系统提示词。
 */
QString defaultSystemPrompt();

}  // namespace markshot::translate_anthropic
