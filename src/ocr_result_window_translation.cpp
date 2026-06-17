#include "ocr_result_window.h"

#include "debug_log.h"
#include "shell_command.h"
#include "translation_language_options.h"
#include "ui/i18n.h"

#include <QComboBox>
#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QLineEdit>
#include <QProcess>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QStandardPaths>
#include <QTemporaryFile>
#include <QTextEdit>
#include <QTimer>

namespace markshot::shot {

/// @brief 初始化目标语言下拉框,并绑定选择与手动输入事件。
/// @return 无返回值。
void OcrResultWindow::setupTargetLanguageCombo()
{
    if (!m_targetLanguageCombo) {
        return;
    }

    m_targetLanguageCombo->setObjectName(QStringLiteral("ocrLanguageCombo"));
    m_targetLanguageCombo->setEditable(true);
    m_targetLanguageCombo->setInsertPolicy(QComboBox::NoInsert);
    m_targetLanguageCombo->setFixedWidth(140);
    m_targetLanguageCombo->setToolTip(MS_TR("Target Language"));
    m_targetLanguageCombo->setStyleSheet(QStringLiteral(
        "QComboBox#ocrLanguageCombo {"
        " color: #E5E7EB;"
        " background: transparent;"
        " border: 0;"
        " padding: 5px 8px;"
        " min-height: 18px;"
        "}"
        "QComboBox#ocrLanguageCombo:hover {"
        " background: transparent;"
        "}"
        "QComboBox#ocrLanguageCombo::drop-down {"
        " border: 0;"
        " width: 0;"
        "}"
        "QComboBox#ocrLanguageCombo::down-arrow {"
        " image: none;"
        " width: 0;"
        " height: 0;"
        "}"
        "QComboBox#ocrLanguageCombo QLineEdit {"
        " color: #E5E7EB;"
        " background: transparent;"
        " border: 0;"
        " padding: 0;"
        " selection-background-color: rgba(45, 212, 191, 70);"
        " selection-color: #E5E7EB;"
        "}"
        "QComboBox#ocrLanguageCombo QAbstractItemView {"
        " color: #E5E7EB;"
        " background: #111827;"
        " border: 1px solid rgba(45, 212, 191, 90);"
        " selection-background-color: rgba(45, 212, 191, 70);"
        " selection-color: #E5E7EB;"
        " outline: 0;"
        "}"));

    // 1. 填充内置目标语言选项
    for (const markshot::TranslationLanguageOption &language : markshot::translationLanguageOptions()) {
        m_targetLanguageCombo->addItem(language.label, language.value);
    }
    setTargetLanguageComboValue(m_config.translationTargetLanguage);

    // 2. 下拉选择和手动输入完成后统一走同一条持久化路径
    connect(m_targetLanguageCombo, QOverload<int>::of(&QComboBox::activated), this, [this] {
        applyTargetLanguageFromCombo();
    });
    if (QLineEdit *editor = m_targetLanguageCombo->lineEdit()) {
        connect(editor, &QLineEdit::editingFinished, this, [this] {
            applyTargetLanguageFromCombo();
        });
    }
}

/// @brief 设置目标语言下拉框当前值。
/// @param targetLanguage 实际传递给翻译器的目标语言名称。
/// @return 无返回值。
void OcrResultWindow::setTargetLanguageComboValue(const QString &targetLanguage)
{
    if (!m_targetLanguageCombo) {
        return;
    }

    const QString normalizedTarget = markshot::translationLanguageValueFromText(targetLanguage);
    for (int i = 0; i < m_targetLanguageCombo->count(); ++i) {
        if (m_targetLanguageCombo->itemData(i).toString() == normalizedTarget) {
            m_targetLanguageCombo->setCurrentIndex(i);
            return;
        }
    }
    m_targetLanguageCombo->setEditText(normalizedTarget);
}

/// @brief 返回目标语言下拉框当前实际值。
/// @return 实际传递给翻译器的目标语言名称。
QString OcrResultWindow::currentTargetLanguage() const
{
    if (!m_targetLanguageCombo) {
        return m_config.translationTargetLanguage;
    }

    const QString text = m_targetLanguageCombo->currentText().trimmed();
    const int index = m_targetLanguageCombo->findText(text, Qt::MatchFixedString);
    if (index >= 0) {
        const QString value = m_targetLanguageCombo->itemData(index).toString().trimmed();
        if (!value.isEmpty()) {
            return value;
        }
    }
    return text.isEmpty()
        ? m_config.translationTargetLanguage
        : markshot::translationLanguageValueFromText(text);
}

/// @brief 从下拉框读取目标语言并持久化到应用配置。
/// @return 无返回值。
void OcrResultWindow::applyTargetLanguageFromCombo()
{
    const QString targetLanguage = currentTargetLanguage().trimmed();
    if (targetLanguage.isEmpty() || targetLanguage == m_config.translationTargetLanguage) {
        return;
    }

    const QString previousTargetLanguage = m_config.translationTargetLanguage;
    m_config.translationTargetLanguage = targetLanguage;

    QString error;
    if (markshot::saveTranslationTargetLanguage(targetLanguage, &error)) {
        setTargetLanguageComboValue(targetLanguage);
        return;
    }

    m_config.translationTargetLanguage = previousTargetLanguage;
    setTargetLanguageComboValue(previousTargetLanguage);
    if (!error.isEmpty()) {
        markshot::debugLog("config",
                           "cannot save translation.targetLanguage: %s",
                           error.toUtf8().constData());
    }
}

/// @brief 启动翻译子进程,将 OCR 文本按行转换成翻译输入 token。
/// @return 无返回值。
void OcrResultWindow::startTranslation()
{
    if (!m_editor || !m_translateButton || m_translationProcess) {
        return;
    }
    applyTargetLanguageFromCombo();
    const QString targetLanguage = m_config.translationTargetLanguage.trimmed();

    const QString text = m_editor->toPlainText().trimmed();
    if (text.isEmpty() || targetLanguage.isEmpty()) {
        showToast(MS_TR("No text to translate"));
        return;
    }

    QJsonArray tokens;
    const QStringList lines = text.split(QLatin1Char('\n'));
    int lineIndex = 0;
    for (const QString &rawLine : lines) {
        const QString line = rawLine.trimmed();
        if (line.isEmpty()) {
            ++lineIndex;
            continue;
        }

        QJsonObject object;
        object.insert(QStringLiteral("text"), line);
        object.insert(QStringLiteral("box"),
                      QJsonArray{0, static_cast<double>(lineIndex) * 24.0, 1000.0, 20.0});
        object.insert(QStringLiteral("line"), lineIndex);
        object.insert(QStringLiteral("index"), 0);
        object.insert(QStringLiteral("confidence"), 1.0);
        tokens.append(object);
        ++lineIndex;
    }

    if (tokens.isEmpty()) {
        showToast(MS_TR("No text to translate"));
        return;
    }

    QJsonObject root;
    root.insert(QStringLiteral("targetLanguage"), targetLanguage);
    root.insert(QStringLiteral("tokens"), tokens);

    QTemporaryFile inputFile(
        QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                 ? QDir::tempPath()
                 : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
            .filePath(QStringLiteral("mark-shot-ocr-result-translate-XXXXXX.json")));
    inputFile.setAutoRemove(false);
    if (!inputFile.open()) {
        showToast(MS_TR("Translation failed"));
        return;
    }

    m_translationInputPath = inputFile.fileName();
    inputFile.write(QJsonDocument(root).toJson(QJsonDocument::Compact));
    inputFile.close();

    auto *process = new QProcess(this);
    m_translationProcess = process;
    process->setProcessEnvironment(QProcessEnvironment::systemEnvironment());

    if (!m_config.translationCommand.isEmpty()) {
        QString commandLine = m_config.translationCommand;
        bool replaced = false;
        replaceShellPlaceholder(&commandLine, QStringLiteral("{input}"), m_translationInputPath, &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{inputPath}"), m_translationInputPath, &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{targetLanguage}"), targetLanguage, &replaced);
        replaceShellPlaceholder(&commandLine, QStringLiteral("{config}"), appConfigPath(), &replaced);
        if (!replaced) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(m_translationInputPath);
        }

        markshot::setShellCommand(process, commandLine);
    } else {
        process->setProgram(helperProgramPath(QStringLiteral("mark-shot-translate")));
        process->setArguments({QStringLiteral("--input"),
                               m_translationInputPath,
                               QStringLiteral("--target-language"),
                               targetLanguage,
                               QStringLiteral("--config"),
                               appConfigPath()});
    }

    m_translateButton->setEnabled(false);
    m_translateButton->setText(MS_TR("Translating..."));
    if (m_targetLanguageCombo) {
        m_targetLanguageCombo->setEnabled(false);
    }
    if (m_targetLanguagePopupButton) {
        m_targetLanguagePopupButton->setEnabled(false);
    }

    connect(process, &QProcess::errorOccurred, this, [this, process] {
        if (process == m_translationProcess && process->state() == QProcess::NotRunning) {
            finishTranslation(process, QByteArray());
        }
    });
    connect(process,
            QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this,
            [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
                const QByteArray output = exitStatus == QProcess::NormalExit && exitCode == 0
                    ? process->readAllStandardOutput()
                    : QByteArray();
                finishTranslation(process, output);
            });
    QTimer::singleShot(m_config.translationTimeoutMs, process, [process] {
        if (process->state() != QProcess::NotRunning) {
            process->kill();
        }
    });

    process->start();
}

/// @brief 处理翻译子进程输出。
/// @param process 翻译子进程。
/// @param output 翻译标准输出。
/// @return 无返回值。
void OcrResultWindow::finishTranslation(QProcess *process, const QByteArray &output)
{
    if (process != m_translationProcess) {
        return;
    }

    QStringList translatedLines;
    if (!output.isEmpty()) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonArray tokenArray = document.object().value(QStringLiteral("tokens")).toArray();
            translatedLines.reserve(tokenArray.size());
            for (const QJsonValue &value : tokenArray) {
                if (!value.isObject()) {
                    continue;
                }
                translatedLines.append(value.toObject().value(QStringLiteral("text")).toString().trimmed());
            }
        }
    }

    const QString translatedText = translatedLines.join(QLatin1Char('\n')).trimmed();
    if (!translatedText.isEmpty() && m_editor) {
        m_editor->setPlainText(translatedText);
    } else {
        showToast(MS_TR("Translation failed"));
    }

    finishTranslationCleanup(process);
}

/// @brief 取消正在运行的翻译子进程并清理临时文件。
/// @return 无返回值。
void OcrResultWindow::cancelTranslation()
{
    if (m_translationProcess) {
        disconnect(m_translationProcess, nullptr, this, nullptr);
        if (m_translationProcess->state() != QProcess::NotRunning) {
            m_translationProcess->kill();
        }
        QProcess *process = m_translationProcess;
        m_translationProcess = nullptr;
        process->deleteLater();
    }

    if (!m_translationInputPath.isEmpty()) {
        QFile::remove(m_translationInputPath);
        m_translationInputPath.clear();
    }
    resetTranslationUi();
}

/// @brief 清理已结束的翻译子进程。
/// @param process 翻译子进程。
/// @return 无返回值。
void OcrResultWindow::finishTranslationCleanup(QProcess *process)
{
    m_translationProcess = nullptr;
    if (!m_translationInputPath.isEmpty()) {
        QFile::remove(m_translationInputPath);
        m_translationInputPath.clear();
    }
    resetTranslationUi();
    process->deleteLater();
}

/// @brief 恢复翻译按钮与语言下拉框状态。
/// @return 无返回值。
void OcrResultWindow::resetTranslationUi()
{
    if (m_translateButton) {
        m_translateButton->setEnabled(true);
        m_translateButton->setText(MS_TR("Translate"));
    }
    if (m_targetLanguageCombo) {
        m_targetLanguageCombo->setEnabled(true);
    }
    if (m_targetLanguagePopupButton) {
        m_targetLanguagePopupButton->setEnabled(true);
    }
}

}  // namespace markshot::shot
