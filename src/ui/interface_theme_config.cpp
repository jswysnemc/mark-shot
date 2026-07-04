#include "ui/interface_theme_config.h"

#include <QApplication>
#include <QGuiApplication>
#include <QJsonValue>
#include <QStyle>
#include <QStyleHints>

namespace markshot::ui {
namespace {

/**
 * 判断调色板是否接近深色主题。
 * @param palette 系统调色板。
 * @return 深色主题返回 true。
 */
bool paletteIsDark(const QPalette &palette)
{
    const QColor window = palette.color(QPalette::Window);
    const QColor text = palette.color(QPalette::WindowText);
    return window.lightness() <= text.lightness();
}

}  // namespace

UiThemeMode uiThemeModeFromString(const QString &raw)
{
    QString value = raw.trimmed().toLower();
    value.replace(QLatin1Char('_'), QLatin1Char('-'));
    if (value == QStringLiteral("dark")) {
        return UiThemeMode::Dark;
    }
    if (value == QStringLiteral("light")) {
        return UiThemeMode::Light;
    }
    return UiThemeMode::System;
}

QString uiThemeModeName(UiThemeMode mode)
{
    switch (mode) {
    case UiThemeMode::Dark:
        return QStringLiteral("dark");
    case UiThemeMode::Light:
        return QStringLiteral("light");
    case UiThemeMode::System:
        break;
    }
    return QStringLiteral("system");
}

UiThemeMode uiThemeModeFromConfigRoot(const QJsonObject &root)
{
    const QJsonValue uiValue = root.value(QStringLiteral("ui"));
    if (uiValue.isObject()) {
        const QString value = uiValue.toObject().value(QStringLiteral("theme")).toString();
        if (!value.trimmed().isEmpty()) {
            return uiThemeModeFromString(value);
        }
    }

    const QString legacyValue = root.value(QStringLiteral("theme")).toString();
    return uiThemeModeFromString(legacyValue);
}

UiThemeMode effectiveUiThemeMode(UiThemeMode mode, const QPalette &systemPalette)
{
    if (mode == UiThemeMode::Dark || mode == UiThemeMode::Light) {
        return mode;
    }
    return paletteIsDark(systemPalette) ? UiThemeMode::Dark : UiThemeMode::Light;
}

UiThemeMode effectiveUiThemeMode(UiThemeMode mode)
{
    if (mode == UiThemeMode::Dark || mode == UiThemeMode::Light) {
        return mode;
    }

#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    if (const QStyleHints *hints = QGuiApplication::styleHints()) {
        if (hints->colorScheme() == Qt::ColorScheme::Dark) {
            return UiThemeMode::Dark;
        }
        if (hints->colorScheme() == Qt::ColorScheme::Light) {
            return UiThemeMode::Light;
        }
    }
#endif

    if (QApplication::style()) {
        return effectiveUiThemeMode(UiThemeMode::System, QApplication::style()->standardPalette());
    }
    return effectiveUiThemeMode(UiThemeMode::System, QApplication::palette());
}

}  // namespace markshot::ui
