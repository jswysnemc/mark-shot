#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>

namespace markshot::providers {

class ProviderTask;

struct TranslateTaskRequest {
    QByteArray inputJson;
    QString inputPath;
    QString targetLanguage;
    QString configPath;
    QString provider = QStringLiteral("auto");
    QString commandLine;
    QString helperProgram;
};

/**
 * 按优先链创建翻译任务。
 *
 * 优先链：自定义命令 > 显式 provider > auto（插件 > 内置 openai-compatible >
 * helper 兜底）。
 * @param request 翻译请求。
 * @param parent 任务父对象。
 * @return 任务实例，调用方负责 start。
 */
ProviderTask *createTranslateTask(const TranslateTaskRequest &request, QObject *parent = nullptr);

/**
 * 解析当前配置实际会使用的翻译 provider 名称。
 * @param request 翻译请求（输入可为空）。
 * @return provider 展示名，供设置页显示。
 */
QString resolvedTranslateProviderName(const TranslateTaskRequest &request);

}  // namespace markshot::providers
