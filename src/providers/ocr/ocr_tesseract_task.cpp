#include "providers/ocr/ocr_tesseract_task.h"

#include "providers/provider_process_task.h"

#include <QHash>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>

#include <algorithm>
#include <utility>

namespace markshot::providers {
namespace {

/**
 * 读取 tesseract 主识别语言。
 * @return 语言串，默认中英混合。
 */
QString primaryLanguage()
{
    const QString env = qEnvironmentVariable("MARK_SHOT_OCR_LANG").trimmed();
    return env.isEmpty() ? QStringLiteral("chi_sim+eng") : env;
}

/**
 * 读取 tesseract 页面分割模式。
 * @return psm 参数值，默认 6。
 */
QString pageSegmentationMode()
{
    const QString env = qEnvironmentVariable("MARK_SHOT_OCR_PSM").trimmed();
    return env.isEmpty() ? QStringLiteral("6") : env;
}

}  // namespace

OcrTesseractTask::OcrTesseractTask(QString imagePath, QObject *parent)
    : ProviderTask(QStringLiteral("tesseract"), parent)
    , m_imagePath(std::move(imagePath))
{
    // 1. 主语言失败时回退英文，与旧 helper 行为一致
    m_languages.append(primaryLanguage());
    if (!m_languages.contains(QStringLiteral("eng"))) {
        m_languages.append(QStringLiteral("eng"));
    }
}

bool OcrTesseractTask::available()
{
    return !QStandardPaths::findExecutable(QStringLiteral("tesseract")).isEmpty();
}

void OcrTesseractTask::start(int timeoutMs)
{
    m_timeoutMs = timeoutMs;
    m_elapsed.start();
    m_attempt = 0;
    startAttempt();
}

void OcrTesseractTask::cancel()
{
    if (m_currentAttempt) {
        m_currentAttempt->cancel();
        m_currentAttempt->deleteLater();
        m_currentAttempt = nullptr;
    }
}

void OcrTesseractTask::startAttempt()
{
    const QString executable = QStandardPaths::findExecutable(QStringLiteral("tesseract"));
    if (executable.isEmpty()) {
        emitFinished({false, TaskError::StartFailed, {}, QByteArrayLiteral("tesseract not found"), {}});
        return;
    }

    const QString language = m_languages.at(m_attempt);
    auto *attempt = ProviderProcessTask::fromProgram(
        QStringLiteral("tesseract"),
        executable,
        {m_imagePath,
         QStringLiteral("stdout"),
         QStringLiteral("-l"),
         language,
         QStringLiteral("--psm"),
         pageSegmentationMode(),
         QStringLiteral("tsv")},
        this);
    m_currentAttempt = attempt;
    connect(attempt, &ProviderTask::finished, this, [this, attempt](const TaskResult &result) {
        attempt->deleteLater();
        if (m_currentAttempt == attempt) {
            m_currentAttempt = nullptr;
        }

        // 1. 成功时把 TSV 转换为标准 tokens JSON 输出
        if (result.ok) {
            emitFinished({true,
                          TaskError::None,
                          tsvToTokensJson(QString::fromUtf8(result.output)),
                          result.errorOutput,
                          {}});
            return;
        }
        if (result.error == TaskError::Timeout) {
            emitFinished(result);
            return;
        }

        // 2. 语言包缺失等失败时换下一个候选语言重试
        m_lastErrorOutput = result.errorOutput;
        ++m_attempt;
        const int remainingMs = m_timeoutMs > 0
            ? m_timeoutMs - static_cast<int>(m_elapsed.elapsed())
            : 0;
        if (m_attempt >= m_languages.size() || (m_timeoutMs > 0 && remainingMs <= 0)) {
            emitFinished({false, TaskError::Failed, {}, m_lastErrorOutput, {}});
            return;
        }
        startAttempt();
    });

    const int remainingMs = m_timeoutMs > 0
        ? std::max(1, m_timeoutMs - static_cast<int>(m_elapsed.elapsed()))
        : 0;
    attempt->start(remainingMs);
}

QByteArray OcrTesseractTask::tsvToTokensJson(const QString &tsv)
{
    const QStringList lines = tsv.split(QLatin1Char('\n'));
    QJsonArray tokens;
    if (!lines.isEmpty()) {
        // 1. 解析表头列名与下标映射
        const QStringList header = lines.first().split(QLatin1Char('\t'));
        QHash<QString, int> columns;
        for (int i = 0; i < header.size(); ++i) {
            columns.insert(header.at(i).trimmed(), i);
        }

        QHash<QString, int> lineIds;
        QHash<QString, int> lineTokenIndexes;
        const auto field = [&columns](const QStringList &row, const QString &name) {
            const int index = columns.value(name, -1);
            return index >= 0 && index < row.size() ? row.at(index) : QString();
        };

        // 2. 只取 level==5 的词级行，按 block/par/line 组合分配行号
        for (int i = 1; i < lines.size(); ++i) {
            const QStringList row = lines.at(i).split(QLatin1Char('\t'));
            if (field(row, QStringLiteral("level")) != QStringLiteral("5")) {
                continue;
            }
            const QString text = field(row, QStringLiteral("text")).trimmed();
            if (text.isEmpty()) {
                continue;
            }

            const QString lineKey = field(row, QStringLiteral("block_num")) + QLatin1Char('/')
                + field(row, QStringLiteral("par_num")) + QLatin1Char('/')
                + field(row, QStringLiteral("line_num"));
            if (!lineIds.contains(lineKey)) {
                lineIds.insert(lineKey, lineIds.size());
                lineTokenIndexes.insert(lineKey, 0);
            }

            bool leftOk = false;
            bool topOk = false;
            bool widthOk = false;
            bool heightOk = false;
            const double left = field(row, QStringLiteral("left")).toDouble(&leftOk);
            const double top = field(row, QStringLiteral("top")).toDouble(&topOk);
            const double width = field(row, QStringLiteral("width")).toDouble(&widthOk);
            const double height = field(row, QStringLiteral("height")).toDouble(&heightOk);
            if (!leftOk || !topOk || !widthOk || !heightOk) {
                continue;
            }

            QJsonObject token;
            token.insert(QStringLiteral("text"), text);
            token.insert(QStringLiteral("box"), QJsonArray{left, top, width, height});
            token.insert(QStringLiteral("line"), lineIds.value(lineKey));
            token.insert(QStringLiteral("index"), lineTokenIndexes.value(lineKey));
            token.insert(QStringLiteral("confidence"),
                         field(row, QStringLiteral("conf")).toDouble());
            tokens.append(token);
            lineTokenIndexes[lineKey] = lineTokenIndexes.value(lineKey) + 1;
        }
    }

    QJsonObject root;
    root.insert(QStringLiteral("backend"), QStringLiteral("tesseract"));
    root.insert(QStringLiteral("tokens"), tokens);
    root.insert(QStringLiteral("errors"), QJsonArray());
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

}  // namespace markshot::providers
