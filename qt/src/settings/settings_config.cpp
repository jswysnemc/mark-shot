#include "settings/settings_config.h"

#include <QStringList>

namespace markshot::settings {
namespace {

/// @brief 清理环境变量文本中的行尾空白。
/// @param line 原始文本行。
/// @return 清理后的文本行。
QString trimmedEnvLine(QString line)
{
    return line.trimmed();
}

}  // namespace

QString toolConfigName(ShotWindow::Tool tool)
{
    switch (tool) {
    case ShotWindow::Tool::Move: return QStringLiteral("move");
    case ShotWindow::Tool::Select: return QStringLiteral("select");
    case ShotWindow::Tool::Pen: return QStringLiteral("pen");
    case ShotWindow::Tool::Line: return QStringLiteral("line");
    case ShotWindow::Tool::Highlighter: return QStringLiteral("highlighter");
    case ShotWindow::Tool::Rectangle: return QStringLiteral("rectangle");
    case ShotWindow::Tool::Ellipse: return QStringLiteral("ellipse");
    case ShotWindow::Tool::Arrow: return QStringLiteral("arrow");
    case ShotWindow::Tool::Text: return QStringLiteral("text");
    case ShotWindow::Tool::Number: return QStringLiteral("number");
    case ShotWindow::Tool::Mosaic: return QStringLiteral("mosaic");
    case ShotWindow::Tool::Magnifier: return QStringLiteral("magnifier");
    case ShotWindow::Tool::Laser: return QStringLiteral("laser");
    case ShotWindow::Tool::Marker: return QStringLiteral("marker");
    case ShotWindow::Tool::Ruler: return QStringLiteral("ruler");
    }
    return QStringLiteral("pen");
}

QString actionConfigName(ShotWindow::Action action)
{
    switch (action) {
    case ShotWindow::Action::ToggleCaptureScope: return QStringLiteral("toggleCaptureScope");
    case ShotWindow::Action::ToggleToolbarLayout: return QStringLiteral("toggleToolbarLayout");
    case ShotWindow::Action::Clear: return QStringLiteral("clear");
    case ShotWindow::Action::Undo: return QStringLiteral("undo");
    case ShotWindow::Action::Redo: return QStringLiteral("redo");
    case ShotWindow::Action::OpenWith: return QStringLiteral("openWith");
    case ShotWindow::Action::Extensions: return QStringLiteral("extensions");
    case ShotWindow::Action::Pin: return QStringLiteral("pin");
    case ShotWindow::Action::ScrollCapture: return QStringLiteral("scrollCapture");
    case ShotWindow::Action::OcrCopy: return QStringLiteral("ocrCopy");
    case ShotWindow::Action::Copy: return QStringLiteral("copy");
    case ShotWindow::Action::Save: return QStringLiteral("save");
    case ShotWindow::Action::Upload: return QStringLiteral("upload");
    case ShotWindow::Action::Settings: return QStringLiteral("settings");
    case ShotWindow::Action::Cancel: return QStringLiteral("cancel");
    case ShotWindow::Action::Translate: return QStringLiteral("translate");
    case ShotWindow::Action::ToolMove: return QStringLiteral("toolMove");
    case ShotWindow::Action::ToolSelect: return QStringLiteral("toolSelect");
    case ShotWindow::Action::ToolPen: return QStringLiteral("toolPen");
    case ShotWindow::Action::ToolLine: return QStringLiteral("toolLine");
    case ShotWindow::Action::ToolHighlighter: return QStringLiteral("toolHighlighter");
    case ShotWindow::Action::ToolRectangle: return QStringLiteral("toolRectangle");
    case ShotWindow::Action::ToolEllipse: return QStringLiteral("toolEllipse");
    case ShotWindow::Action::ToolArrow: return QStringLiteral("toolArrow");
    case ShotWindow::Action::ToolText: return QStringLiteral("toolText");
    case ShotWindow::Action::ToolNumber: return QStringLiteral("toolNumber");
    case ShotWindow::Action::ToolMosaic: return QStringLiteral("toolMosaic");
    case ShotWindow::Action::ToolMagnifier: return QStringLiteral("toolMagnifier");
    case ShotWindow::Action::ToolLaser: return QStringLiteral("toolLaser");
    case ShotWindow::Action::ToolMarker: return QStringLiteral("toolMarker");
    case ShotWindow::Action::ToolRuler: return QStringLiteral("toolRuler");
    }
    return QStringLiteral("copy");
}

QString clipboardImageModeName(ClipboardImageMode mode)
{
    switch (mode) {
    case ClipboardImageMode::ImagePng: return QStringLiteral("image/png");
    case ClipboardImageMode::Url: return QStringLiteral("url");
    case ClipboardImageMode::Threshold: return QStringLiteral("threshold");
    }
    return QStringLiteral("image/png");
}

QString envMapToText(const QMap<QString, QString> &values)
{
    QStringList lines;
    for (auto it = values.constBegin(); it != values.constEnd(); ++it) {
        const QString key = it.key().trimmed();
        if (!key.isEmpty()) {
            lines.append(QStringLiteral("%1=%2").arg(key, it.value()));
        }
    }
    return lines.join(QLatin1Char('\n'));
}

QMap<QString, QString> envMapFromText(const QString &text)
{
    QMap<QString, QString> values;
    const QStringList lines = text.split(QLatin1Char('\n'));
    for (const QString &rawLine : lines) {
        const QString line = trimmedEnvLine(rawLine);
        if (line.isEmpty() || line.startsWith(QLatin1Char('#'))) {
            continue;
        }
        const qsizetype separator = line.indexOf(QLatin1Char('='));
        if (separator <= 0) {
            continue;
        }
        const QString key = line.left(separator).trimmed();
        if (!key.isEmpty()) {
            values.insert(key, line.mid(separator + 1));
        }
    }
    return values;
}

}  // namespace markshot::settings
