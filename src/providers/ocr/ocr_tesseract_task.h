#pragma once

#include "providers/provider_task.h"

#include <QElapsedTimer>
#include <QStringList>

namespace markshot::providers {

class ProviderProcessTask;

/**
 * 内置 tesseract OCR 任务。
 *
 * 直接调用 tesseract CLI 输出 TSV 并转换为标准 tokens JSON，
 * 不依赖 Python 环境；主语言失败时自动回退英文。
 */
class OcrTesseractTask final : public ProviderTask {
    Q_OBJECT

public:
    /**
     * 创建 tesseract OCR 任务。
     * @param imagePath 输入图像路径。
     * @param parent 父对象。
     */
    explicit OcrTesseractTask(QString imagePath, QObject *parent = nullptr);

    void start(int timeoutMs) override;
    void cancel() override;

    /**
     * 判断本机 tesseract 是否可用。
     * @return PATH 中存在 tesseract 可执行文件时返回 true。
     */
    static bool available();

    /**
     * 把 tesseract TSV 输出转换为标准 tokens JSON。
     * @param tsv TSV 文本。
     * @return {backend, tokens, errors} JSON 字节。
     */
    static QByteArray tsvToTokensJson(const QString &tsv);

private:
    /**
     * 以当前候选语言发起一次识别尝试。
     * @return 无返回值。
     */
    void startAttempt();

    QString m_imagePath;
    QStringList m_languages;
    QElapsedTimer m_elapsed;
    ProviderProcessTask *m_currentAttempt = nullptr;
    QByteArray m_lastErrorOutput;
    int m_attempt = 0;
    int m_timeoutMs = 0;
};

}  // namespace markshot::providers
