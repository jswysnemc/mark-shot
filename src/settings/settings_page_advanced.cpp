#include "settings/settings_page_advanced.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QCheckBox>
#include <QDesktopServices>
#include <QFormLayout>
#include <QFrame>
#include <QGuiApplication>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QProcessEnvironment>
#include <QSpinBox>
#include <QUrl>
#include <QVBoxLayout>

#ifndef MARK_SHOT_VERSION
#define MARK_SHOT_VERSION "unknown"
#endif

namespace markshot::settings {
namespace {

/// @brief 读取当前显示服务器类型说明。
/// @return Wayland / X11 / 未知。
QString displayServerText()
{
    const QString platform = QGuiApplication::platformName().toLower();
    if (platform.contains(QStringLiteral("wayland"))) {
        return QStringLiteral("Wayland");
    }
    if (platform.contains(QStringLiteral("xcb")) || platform.contains(QStringLiteral("windows"))) {
        return platform.contains(QStringLiteral("windows")) ? QStringLiteral("Windows")
                                                           : QStringLiteral("X11");
    }
    const QString session =
        QProcessEnvironment::systemEnvironment().value(QStringLiteral("XDG_SESSION_TYPE")).toLower();
    if (session == QStringLiteral("wayland")) {
        return QStringLiteral("Wayland");
    }
    if (session == QStringLiteral("x11")) {
        return QStringLiteral("X11");
    }
    return platform.isEmpty() ? QStringLiteral("unknown") : platform;
}

/// @brief 读取桌面环境 / 合成器简述。
/// @return 桌面字符串。
QString desktopEnvironmentText()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QStringList parts = {
        env.value(QStringLiteral("XDG_CURRENT_DESKTOP")),
        env.value(QStringLiteral("XDG_SESSION_DESKTOP")),
        env.value(QStringLiteral("DESKTOP_SESSION")),
    };
    QStringList unique;
    for (const QString &part : parts) {
        const QString trimmed = part.trimmed();
        if (!trimmed.isEmpty() && !unique.contains(trimmed, Qt::CaseInsensitive)) {
            unique.append(trimmed);
        }
    }
    if (!env.value(QStringLiteral("NIRI_SOCKET")).isEmpty() && !unique.join(QLatin1Char('/')).contains(QStringLiteral("niri"), Qt::CaseInsensitive)) {
        unique.prepend(QStringLiteral("niri"));
    }
    return unique.isEmpty() ? QStringLiteral("unknown") : unique.join(QStringLiteral(" / "));
}

/// @brief 添加只读信息行。
/// @param form 表单。
/// @param label 标签。
/// @param value 值。
/// @return 值标签。
QLabel *addReadOnlyInfoRow(QFormLayout *form, const QString &label, const QString &value)
{
    auto *valueLabel = new QLabel(value);
    valueLabel->setTextInteractionFlags(Qt::TextSelectableByMouse | Qt::LinksAccessibleByMouse);
    valueLabel->setOpenExternalLinks(true);
    valueLabel->setWordWrap(true);
    form->addRow(label, valueLabel);
    return valueLabel;
}

}  // namespace

SettingsPageAdvanced::SettingsPageAdvanced(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);

    QFrame *aboutCard = createSettingsCard(MS_TR("About"),
                                           MS_TR("Application version and runtime environment."),
                                           this);
    QFormLayout *aboutForm = settingsCardForm(aboutCard);
    addReadOnlyInfoRow(aboutForm, MS_TR("Version"), QStringLiteral(MARK_SHOT_VERSION));
    addReadOnlyInfoRow(aboutForm, MS_TR("Display Server"), displayServerText());
    addReadOnlyInfoRow(aboutForm, MS_TR("Desktop"), desktopEnvironmentText());
    addReadOnlyInfoRow(aboutForm,
                       MS_TR("Project"),
                       QStringLiteral("<a href=\"https://github.com/jswysnemc/mark-shot\">"
                                      "https://github.com/jswysnemc/mark-shot</a>"));
    layout->addWidget(aboutCard);

    QFrame *debugCard = createSettingsCard(MS_TR("Debug"),
                                           MS_TR("Configure diagnostic logging for troubleshooting."),
                                           this);
    QFormLayout *debugForm = settingsCardForm(debugCard);
    m_debugEnabled = addSwitchRow(debugForm, MS_TR("Debug Logging"), MS_TR("Enable debug log output."));
    m_debugLogPath = addTextRow(debugForm, MS_TR("Debug Log Path"), QStringLiteral("~/mark-shot-debug.log"));
    layout->addWidget(debugCard);

    QFrame *windowCard = createSettingsCard(MS_TR("Window Detection"),
                                            MS_TR("Configure the external helper used to detect windows under the selection."),
                                            this);
    QFormLayout *windowForm = settingsCardForm(windowCard);
    m_windowDetectionEnabled = addSwitchRow(windowForm,
                                            MS_TR("Window Detection Enabled"),
                                            MS_TR("Run the configured helper before region selection."));
    m_windowDetectionCommand = addTextRow(windowForm,
                                          MS_TR("Window Detection Command"),
                                          QStringLiteral("mark-shot-window-detection-niri"));
    m_windowDetectionWorkingDirectory = addTextRow(windowForm, MS_TR("Working Directory"), QStringLiteral("~/"));
    m_windowDetectionTimeoutMs = addSpinRow(windowForm, MS_TR("Window Detection Timeout"), 100, 30000, QStringLiteral(" ms"));
    m_windowDetectionEnv = addPlainTextRow(windowForm,
                                           MS_TR("Window Detection Environment"),
                                           QStringLiteral("KEY=value"));
    layout->addWidget(windowCard);

    QFrame *envCard = createSettingsCard(MS_TR("Application Environment"),
                                         MS_TR("Environment variables applied when Mark Shot starts."),
                                         this);
    QFormLayout *envForm = settingsCardForm(envCard);
    m_appEnv = addPlainTextRow(envForm, MS_TR("Application Environment"), QStringLiteral("KEY=value"));
    layout->addWidget(envCard);
    layout->addStretch();
}

void SettingsPageAdvanced::setConfig(const SettingsConfig &config)
{
    m_debugEnabled->setChecked(config.advanced.debugEnabled);
    m_debugLogPath->setText(config.advanced.debugLogPath);
    m_windowDetectionEnabled->setChecked(config.advanced.windowDetectionEnabled);
    m_windowDetectionCommand->setText(config.advanced.windowDetectionCommand);
    m_windowDetectionWorkingDirectory->setText(config.advanced.windowDetectionWorkingDirectory);
    m_windowDetectionTimeoutMs->setValue(config.advanced.windowDetectionTimeoutMs);
    m_windowDetectionEnv->setPlainText(envMapToText(config.advanced.windowDetectionEnv));
    m_appEnv->setPlainText(envMapToText(config.advanced.appEnv));
}

void SettingsPageAdvanced::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->advanced.debugEnabled = m_debugEnabled->isChecked();
    config->advanced.debugLogPath = m_debugLogPath->text().trimmed();
    config->advanced.windowDetectionEnabled = m_windowDetectionEnabled->isChecked();
    config->advanced.windowDetectionCommand = m_windowDetectionCommand->text().trimmed();
    config->advanced.windowDetectionWorkingDirectory = m_windowDetectionWorkingDirectory->text().trimmed();
    config->advanced.windowDetectionTimeoutMs = m_windowDetectionTimeoutMs->value();
    config->advanced.windowDetectionEnv = envMapFromText(m_windowDetectionEnv->toPlainText());
    config->advanced.appEnv = envMapFromText(m_appEnv->toPlainText());
}

}  // namespace markshot::settings
