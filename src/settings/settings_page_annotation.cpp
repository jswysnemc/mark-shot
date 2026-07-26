#include "settings/settings_page_annotation.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QColorDialog>
#include <QComboBox>
#include <QFormLayout>
#include <QFrame>
#include <QPushButton>
#include <QSpinBox>
#include <QVBoxLayout>

namespace markshot::settings {
namespace {

/// @brief 返回工具名称的本地化文本。
/// @param tool 工具枚举值。
/// @return 本地化后的工具名称。
QString toolLabel(ShotWindow::Tool tool)
{
    switch (tool) {
    case ShotWindow::Tool::Move: return MS_TR("Move");
    case ShotWindow::Tool::Select: return MS_TR("Select");
    case ShotWindow::Tool::Pen: return MS_TR("Pen");
    case ShotWindow::Tool::Line: return MS_TR("Line");
    case ShotWindow::Tool::Highlighter: return MS_TR("Highlighter");
    case ShotWindow::Tool::Rectangle: return MS_TR("Rect");
    case ShotWindow::Tool::Ellipse: return MS_TR("Ellipse");
    case ShotWindow::Tool::Arrow: return MS_TR("Arrow");
    case ShotWindow::Tool::Text: return MS_TR("Text");
    case ShotWindow::Tool::Number: return MS_TR("Number");
    case ShotWindow::Tool::Mosaic: return MS_TR("Mosaic");
    case ShotWindow::Tool::Magnifier: return MS_TR("Magnifier");
    case ShotWindow::Tool::Laser: return MS_TR("Laser");
    case ShotWindow::ShotWindow::Tool::Marker: return MS_TR("Marker");
    }
    return MS_TR("Pen");
}

}  // namespace

SettingsPageAnnotation::SettingsPageAnnotation(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);

    QFrame *defaultsCard = createSettingsCard(MS_TR("Annotation Defaults"),
                                              MS_TR("Set the initial tools and color used by new capture windows."),
                                              this);
    QFormLayout *defaultsForm = settingsCardForm(defaultsCard);
    m_normalTool = addComboRow(defaultsForm, MS_TR("Region Tool"));
    m_fullscreenTool = addComboRow(defaultsForm, MS_TR("Fullscreen Tool"));
    m_fileTool = addComboRow(defaultsForm, MS_TR("File Tool"));
    populateToolCombo(m_normalTool);
    populateToolCombo(m_fullscreenTool);
    populateToolCombo(m_fileTool);
    m_colorButton = new QPushButton;
    m_colorButton->setCursor(Qt::PointingHandCursor);
    defaultsForm->addRow(MS_TR("Default Color"), m_colorButton);
    connect(m_colorButton, &QPushButton::clicked, this, [this] {
        const QColor selected = QColorDialog::getColor(m_defaultColor, this, MS_TR("Default Color"));
        if (!selected.isValid()) {
            return;
        }
        m_defaultColor = selected;
        updateColorButton();
    });
    layout->addWidget(defaultsCard);

    QFrame *toolbarCard = createSettingsCard(MS_TR("Toolbar Appearance"),
                                             MS_TR("Change compact toolbar sizes used in capture windows."),
                                             this);
    QFormLayout *toolbarForm = settingsCardForm(toolbarCard);
    m_toolbarIconSize = addSpinRow(toolbarForm, MS_TR("Icon Size"), 12, 48, QStringLiteral(" px"));
    m_toolbarFontSize = addSpinRow(toolbarForm, MS_TR("Font Size"), 8, 24, QStringLiteral(" px"));
    layout->addWidget(toolbarCard);
    layout->addStretch();
}

void SettingsPageAnnotation::setConfig(const SettingsConfig &config)
{
    setToolComboValue(m_normalTool, config.annotation.normalTool);
    setToolComboValue(m_fullscreenTool, config.annotation.fullscreenTool);
    setToolComboValue(m_fileTool, config.annotation.fileTool);
    m_defaultColor = config.annotation.defaultColor;
    updateColorButton();
    m_toolbarIconSize->setValue(config.annotation.toolbarIconSize);
    m_toolbarFontSize->setValue(config.annotation.toolbarFontSize);
}

void SettingsPageAnnotation::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->annotation.normalTool = toolComboValue(m_normalTool);
    config->annotation.fullscreenTool = toolComboValue(m_fullscreenTool);
    config->annotation.fileTool = toolComboValue(m_fileTool);
    config->annotation.defaultColor = m_defaultColor;
    config->annotation.toolbarIconSize = m_toolbarIconSize->value();
    config->annotation.toolbarFontSize = m_toolbarFontSize->value();
}

void SettingsPageAnnotation::populateToolCombo(QComboBox *combo)
{
    if (!combo) {
        return;
    }

    for (ShotWindow::Tool tool : {ShotWindow::Tool::Move,
                                  ShotWindow::Tool::Select,
                                  ShotWindow::Tool::Pen,
                                  ShotWindow::Tool::Line,
                                  ShotWindow::Tool::Highlighter,
                                  ShotWindow::Tool::Rectangle,
                                  ShotWindow::Tool::Ellipse,
                                  ShotWindow::Tool::Arrow,
                                  ShotWindow::Tool::Text,
                                  ShotWindow::Tool::Number,
                                  ShotWindow::Tool::Mosaic,
                                  ShotWindow::Tool::Magnifier,
                                  ShotWindow::Tool::Laser,
                                  ShotWindow::Tool::Marker}) {
        combo->addItem(toolLabel(tool), static_cast<int>(tool));
    }
}

void SettingsPageAnnotation::setToolComboValue(QComboBox *combo, ShotWindow::Tool tool)
{
    if (!combo) {
        return;
    }

    const int index = combo->findData(static_cast<int>(tool));
    combo->setCurrentIndex(index >= 0 ? index : 0);
}

ShotWindow::Tool SettingsPageAnnotation::toolComboValue(QComboBox *combo) const
{
    if (!combo) {
        return ShotWindow::Tool::Pen;
    }
    return static_cast<ShotWindow::Tool>(combo->currentData().toInt());
}

void SettingsPageAnnotation::updateColorButton()
{
    const QString colorName = m_defaultColor.isValid()
        ? m_defaultColor.name(QColor::HexRgb).toUpper()
        : QStringLiteral("#FF4D4D");
    m_colorButton->setText(colorName);
    m_colorButton->setStyleSheet(colorButtonStyleSheet(m_defaultColor));
}

}  // namespace markshot::settings
