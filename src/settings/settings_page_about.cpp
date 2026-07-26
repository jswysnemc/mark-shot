#include "settings/settings_page_about.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QFormLayout>
#include <QFrame>
#include <QGuiApplication>
#include <QLabel>
#include <QProcessEnvironment>
#include <QVBoxLayout>

#ifndef MARK_SHOT_VERSION
#define MARK_SHOT_VERSION "unknown"
#endif

namespace markshot::settings {
namespace {

/// @brief 读取当前显示服务器类型说明。
/// @return Wayland / X11 / Windows / 未知。
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
    if (!env.value(QStringLiteral("NIRI_SOCKET")).isEmpty()
        && !unique.join(QLatin1Char('/')).contains(QStringLiteral("niri"), Qt::CaseInsensitive)) {
        unique.prepend(QStringLiteral("niri"));
    }
    return unique.isEmpty() ? QStringLiteral("unknown") : unique.join(QStringLiteral(" / "));
}

/// @brief 添加只读信息行。
/// @param form 表单。
/// @param label 标签。
/// @param value 值。
void addReadOnlyInfoRow(QFormLayout *form, const QString &label, const QString &value)
{
    auto *valueLabel = new QLabel(value);
    valueLabel->setTextInteractionFlags(Qt::TextSelectableByMouse | Qt::LinksAccessibleByMouse);
    valueLabel->setOpenExternalLinks(true);
    valueLabel->setWordWrap(true);
    form->addRow(label, valueLabel);
}

}  // namespace

SettingsPageAbout::SettingsPageAbout(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);

    // 1. 版本与运行环境卡片
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
    layout->addStretch();
}

}  // namespace markshot::settings
