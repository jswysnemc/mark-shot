#pragma once

#include "providers/provider_task.h"
#include "providers/translate/translate_segments.h"

#include <QTimer>

class QNetworkAccessManager;
class QNetworkReply;

namespace markshot::providers {

/**
 * 内置 openai-compatible 翻译任务。
 *
 * 直接通过 QNetworkAccessManager 调用 chat/completions 接口，
 * 不依赖 Python 环境；配置读取与旧 helper 完全一致。
 */
class TranslateOpenAiTask final : public ProviderTask {
    Q_OBJECT

public:
    /**
     * 创建内置翻译任务。
     * @param inputJson {tokens, targetLanguage} 输入。
     * @param targetLanguage 目标语言，空串时取输入 JSON 内的值。
     * @param configPath 应用配置文件路径。
     * @param parent 父对象。
     */
    TranslateOpenAiTask(QByteArray inputJson,
                        QString targetLanguage,
                        QString configPath,
                        QObject *parent = nullptr);

    void start(int timeoutMs) override;
    void cancel() override;

private:
    /**
     * 处理 HTTP 响应并生成输出 tokens JSON。
     * @return 无返回值。
     */
    void handleReply();

    /**
     * 以失败形态结束任务，错误文本写入标准 errors JSON。
     * @param message 错误信息。
     * @return 无返回值。
     */
    void failWith(const QString &message);

    QByteArray m_inputJson;
    QString m_targetLanguage;
    QString m_configPath;
    QNetworkAccessManager *m_network = nullptr;
    QNetworkReply *m_reply = nullptr;
    QTimer m_timeoutTimer;
    QVector<TranslateSourceSegment> m_segments;
};

}  // namespace markshot::providers
