#include "shortcut_config.h"

#include "config_value.h"

#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>

namespace markshot::shortcut {
namespace {

namespace cfg = markshot::config;

/// @brief Returns the default keyboard shortcuts for actions.
/// @return An ActionShortcuts map populated with default key sequences.
ActionShortcuts defaultActionShortcuts()
{
    ActionShortcuts shortcuts{};
    shortcuts[actionIndex(ShotWindow::Action::ToggleCaptureScope)] = QKeySequence(Qt::Key_F);
    shortcuts[actionIndex(ShotWindow::Action::Pin)] = QKeySequence(Qt::CTRL | Qt::Key_P);
    shortcuts[actionIndex(ShotWindow::Action::Copy)] = QKeySequence(Qt::CTRL | Qt::Key_C);
    shortcuts[actionIndex(ShotWindow::Action::Save)] = QKeySequence(Qt::CTRL | Qt::Key_S);
    shortcuts[actionIndex(ShotWindow::Action::Cancel)] = QKeySequence(Qt::Key_Escape);
    shortcuts[actionIndex(ShotWindow::Action::Undo)] = QKeySequence(Qt::CTRL | Qt::Key_Z);
    shortcuts[actionIndex(ShotWindow::Action::Redo)] = QKeySequence(Qt::CTRL | Qt::Key_Y);
    return shortcuts;
}

/// @brief Returns the default keyboard shortcuts for tools.
/// @return A ToolShortcuts map populated with default key sequences.
ToolShortcuts defaultToolShortcuts()
{
    ToolShortcuts shortcuts{};
    shortcuts[toolIndex(ShotWindow::Tool::Move)] = QKeySequence(Qt::Key_V);
    shortcuts[toolIndex(ShotWindow::Tool::Select)] = QKeySequence(Qt::Key_S);
    shortcuts[toolIndex(ShotWindow::Tool::Pen)] = QKeySequence(Qt::Key_P);
    shortcuts[toolIndex(ShotWindow::Tool::Line)] = QKeySequence(Qt::Key_L);
    shortcuts[toolIndex(ShotWindow::Tool::Highlighter)] = QKeySequence(Qt::Key_H);
    shortcuts[toolIndex(ShotWindow::Tool::Rectangle)] = QKeySequence(Qt::Key_R);
    shortcuts[toolIndex(ShotWindow::Tool::Ellipse)] = QKeySequence(Qt::Key_E);
    shortcuts[toolIndex(ShotWindow::Tool::Arrow)] = QKeySequence(Qt::Key_A);
    shortcuts[toolIndex(ShotWindow::Tool::Text)] = QKeySequence(Qt::Key_T);
    shortcuts[toolIndex(ShotWindow::Tool::Number)] = QKeySequence(Qt::Key_N);
    shortcuts[toolIndex(ShotWindow::Tool::Mosaic)] = QKeySequence(Qt::Key_M);
    shortcuts[toolIndex(ShotWindow::Tool::Laser)] = QKeySequence(Qt::Key_G);
    return shortcuts;
}

/// @brief Applies tool-specific shortcuts from a JSON object to a ToolShortcuts map.
/// @param object The JSON object containing tool shortcut mappings.
/// @param shortcuts Pointer to the ToolShortcuts map to be updated.
void applyToolShortcutObject(const QJsonObject &object, ToolShortcuts *shortcuts)
{
    if (!shortcuts) {
        return;
    }
    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        const std::optional<ShotWindow::Tool> tool = toolFromConfigName(it.key());
        const std::optional<QKeySequence> sequence = cfg::keySequenceValue(it.value());
        if (tool.has_value() && sequence.has_value()) {
            (*shortcuts)[toolIndex(*tool)] = *sequence;
        }
    }
}

/// @brief Applies action-specific shortcuts from a JSON object to an ActionShortcuts map.
/// @param object The JSON object containing action shortcut mappings.
/// @param shortcuts Pointer to the ActionShortcuts map to be updated.
void applyActionShortcutObject(const QJsonObject &object, ActionShortcuts *shortcuts)
{
    if (!shortcuts) {
        return;
    }
    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        const std::optional<ShotWindow::Action> action = actionFromConfigName(it.key());
        const std::optional<QKeySequence> sequence = cfg::keySequenceValue(it.value());
        if (action.has_value() && sequence.has_value()) {
            (*shortcuts)[actionIndex(*action)] = *sequence;
        }
    }
}

/// @brief Applies startup-specific shortcut settings from a JSON object to a ShortcutConfig.
/// @param object The JSON object containing startup shortcut settings.
/// @param config Pointer to the ShortcutConfig to be updated.
void applyStartupShortcutObject(const QJsonObject &object, ShortcutConfig *config)
{
    if (!config) {
        return;
    }

    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        const QString key = cfg::normalizedKey(it.key());
        const std::optional<QKeySequence> sequence = cfg::keySequenceValue(it.value());
        if (!sequence.has_value()) {
            continue;
        }
        if (key == QStringLiteral("colorpicker") || key == QStringLiteral("color")
            || key == QStringLiteral("pickcolor")) {
            config->startupColorPicker = *sequence;
        } else if (key == QStringLiteral("ruler") || key == QStringLiteral("measure")) {
            config->startupRuler = *sequence;
        } else if (key == QStringLiteral("codescanner") || key == QStringLiteral("scanner")
                   || key == QStringLiteral("scan") || key == QStringLiteral("barcode")
                   || key == QStringLiteral("qrcode")) {
            config->startupCodeScanner = *sequence;
        } else if (key == QStringLiteral("displaycapture") || key == QStringLiteral("display")
                   || key == QStringLiteral("screen") || key == QStringLiteral("monitor")) {
            config->startupDisplayCapture = *sequence;
        }
    }
}

/// @brief Applies shortcut configuration from a JSON object to a ShortcutConfig.
/// @param object The JSON object containing shortcut mappings.
/// @param config Pointer to the ShortcutConfig to be updated.
void applyShortcutObject(const QJsonObject &object, ShortcutConfig *config)
{
    if (!config || object.isEmpty()) {
        return;
    }

    applyToolShortcutObject(cfg::firstObjectValue(object, {QStringLiteral("tools"),
                                                           QStringLiteral("tool"),
                                                           QStringLiteral("toolShortcuts")}),
                            &config->tools);
    applyActionShortcutObject(cfg::firstObjectValue(object, {QStringLiteral("actions"),
                                                             QStringLiteral("action"),
                                                             QStringLiteral("actionShortcuts")}),
                              &config->actions);
    applyStartupShortcutObject(cfg::firstObjectValue(object, {QStringLiteral("startup"),
                                                              QStringLiteral("startupTools"),
                                                              QStringLiteral("selection")}),
                               config);

    for (auto it = object.constBegin(); it != object.constEnd(); ++it) {
        if (it.value().isObject()) {
            continue;
        }
        const std::optional<QKeySequence> sequence = cfg::keySequenceValue(it.value());
        if (!sequence.has_value()) {
            continue;
        }
        if (const std::optional<ShotWindow::Tool> tool = toolFromConfigName(it.key())) {
            config->tools[toolIndex(*tool)] = *sequence;
            continue;
        }
        if (const std::optional<ShotWindow::Action> action = actionFromConfigName(it.key())) {
            config->actions[actionIndex(*action)] = *sequence;
            continue;
        }
        QJsonObject startupShortcut;
        startupShortcut.insert(it.key(), it.value());
        applyStartupShortcutObject(startupShortcut, config);
    }
}

}  // namespace

int actionIndex(ShotWindow::Action action)
{
    return static_cast<int>(action);
}

int toolIndex(ShotWindow::Tool tool)
{
    return static_cast<int>(tool);
}

std::optional<ShotWindow::Action> actionFromConfigName(QString name)
{
    const QString key = cfg::normalizedKey(std::move(name));
    if (key == QStringLiteral("scope") || key == QStringLiteral("togglecapturescope")
        || key == QStringLiteral("fullscreen")) {
        return ShotWindow::Action::ToggleCaptureScope;
    }
    if (key == QStringLiteral("layout") || key == QStringLiteral("toggletoolbarlayout")) {
        return ShotWindow::Action::ToggleToolbarLayout;
    }
    if (key == QStringLiteral("clear")) return ShotWindow::Action::Clear;
    if (key == QStringLiteral("undo")) return ShotWindow::Action::Undo;
    if (key == QStringLiteral("redo")) return ShotWindow::Action::Redo;
    if (key == QStringLiteral("open") || key == QStringLiteral("openwith")) return ShotWindow::Action::OpenWith;
    if (key == QStringLiteral("extension") || key == QStringLiteral("extensions")) return ShotWindow::Action::Extensions;
    if (key == QStringLiteral("pin")) return ShotWindow::Action::Pin;
    if (key == QStringLiteral("scroll") || key == QStringLiteral("scrollcapture")) return ShotWindow::Action::ScrollCapture;
    if (key == QStringLiteral("ocr") || key == QStringLiteral("ocrcopy")) return ShotWindow::Action::OcrCopy;
    if (key == QStringLiteral("magnifier") || key == QStringLiteral("magnify")
        || key == QStringLiteral("loupe") || key == QStringLiteral("zoom")) {
        return ShotWindow::Action::ToolMagnifier;
    }
    if (key == QStringLiteral("copy")) return ShotWindow::Action::Copy;
    if (key == QStringLiteral("save") || key == QStringLiteral("saveas")) return ShotWindow::Action::Save;
    if (key == QStringLiteral("upload") || key == QStringLiteral("imageupload")
        || key == QStringLiteral("uploadimage") || key == QStringLiteral("host")) {
        return ShotWindow::Action::Upload;
    }
    if (key == QStringLiteral("cancel") || key == QStringLiteral("close") || key == QStringLiteral("escape")) {
        return ShotWindow::Action::Cancel;
    }
    return std::nullopt;
}

std::optional<ShotWindow::Tool> toolFromConfigName(QString name)
{
    const QString key = cfg::normalizedKey(std::move(name));
    if (key == QStringLiteral("move") || key == QStringLiteral("pan") || key == QStringLiteral("hand")) {
        return ShotWindow::Tool::Move;
    }
    if (key == QStringLiteral("select") || key == QStringLiteral("selection") || key == QStringLiteral("cursor")) {
        return ShotWindow::Tool::Select;
    }
    if (key == QStringLiteral("pen")) {
        return ShotWindow::Tool::Pen;
    }
    if (key == QStringLiteral("line")) {
        return ShotWindow::Tool::Line;
    }
    if (key == QStringLiteral("highlighter") || key == QStringLiteral("highlight")) {
        return ShotWindow::Tool::Highlighter;
    }
    if (key == QStringLiteral("rectangle") || key == QStringLiteral("rect")) {
        return ShotWindow::Tool::Rectangle;
    }
    if (key == QStringLiteral("ellipse") || key == QStringLiteral("oval") || key == QStringLiteral("circle")) {
        return ShotWindow::Tool::Ellipse;
    }
    if (key == QStringLiteral("arrow")) {
        return ShotWindow::Tool::Arrow;
    }
    if (key == QStringLiteral("text")) {
        return ShotWindow::Tool::Text;
    }
    if (key == QStringLiteral("number") || key == QStringLiteral("counter")) {
        return ShotWindow::Tool::Number;
    }
    if (key == QStringLiteral("magnifier") || key == QStringLiteral("magnify")
        || key == QStringLiteral("loupe") || key == QStringLiteral("zoom")) {
        return ShotWindow::Tool::Magnifier;
    }
    if (key == QStringLiteral("mosaic") || key == QStringLiteral("blur")) {
        return ShotWindow::Tool::Mosaic;
    }
    if (key == QStringLiteral("laser")) {
        return ShotWindow::Tool::Laser;
    }
    return std::nullopt;
}

ShortcutConfig configuredShortcuts(const QString &configPath)
{
    ShortcutConfig config{defaultActionShortcuts(),
                          defaultToolShortcuts(),
                          QKeySequence(Qt::Key_C),
                          QKeySequence(Qt::Key_R),
                          QKeySequence(Qt::Key_Q),
                          QKeySequence(Qt::Key_D)};

    QFile file(configPath);
    if (!file.exists() || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return config;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return config;
    }

    const QJsonObject root = document.object();
    const QJsonObject annotation = cfg::firstObjectValue(root, QStringLiteral("annotation"));
    for (const QJsonObject &object : {
             cfg::firstObjectValue(root, QStringLiteral("shortcuts")),
             cfg::firstObjectValue(root, QStringLiteral("hotkeys")),
             cfg::firstObjectValue(annotation, QStringLiteral("shortcuts")),
             cfg::firstObjectValue(annotation, QStringLiteral("hotkeys")),
         }) {
        applyShortcutObject(object, &config);
    }
    return config;
}

}  // namespace markshot::shortcut
