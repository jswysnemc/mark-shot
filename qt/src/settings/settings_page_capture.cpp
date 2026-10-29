#include "settings/settings_page_capture.h"

#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QCheckBox>
#include <QComboBox>
#include <QFormLayout>
#include <QFrame>
#include <QVBoxLayout>

namespace markshot::settings {

SettingsPageCapture::SettingsPageCapture(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = createSettingsPageLayout(this);
    QFrame *captureCard = createSettingsCard(MS_TR("Capture"),
                                             MS_TR("Adjust how the frozen screenshot is captured before annotation starts."),
                                             this);
    QFormLayout *form = settingsCardForm(captureCard);
    m_includeCursor = addSwitchRow(form,
                                   MS_TR("Include Cursor"),
                                   MS_TR("Capture the mouse cursor in the frozen image when supported."));
    m_freezeScope = addComboRow(form, MS_TR("Freeze Scope"));
    m_freezeScope->addItem(MS_TR("All Screens"), static_cast<int>(CaptureFreezeScope::AllScreens));
    m_freezeScope->addItem(MS_TR("Cursor Screen"), static_cast<int>(CaptureFreezeScope::CursorScreen));
    QFrame *compatibility = createAdvancedSettingsCard(MS_TR("Capture compatibility"), {}, this);
    QFormLayout *compatibilityForm = settingsCardForm(compatibility);
    m_kdeKwinScreenshot = addSwitchRow(compatibilityForm,
                                       MS_TR("KDE KWin Screenshot"),
                                       MS_TR("Use KWin ScreenShot2 on KDE Wayland when available."));
    m_hideOwnWindows = addSwitchRow(compatibilityForm,
                                    MS_TR("Hide Mark Shot Windows While Capturing"),
                                    MS_TR("Hide own windows from screenshots. Turn off to include them."));
    m_regionDetection = addSwitchRow(compatibilityForm,
                                     MS_TR("Region Detection"),
                                     MS_TR("Analyze image regions for hover selection. Turn off to debug KDE window detection without disabling window recognition."));
    m_regionDetection->setObjectName(QStringLiteral("captureRegionDetection"));
    // 选区内空白处双击执行的快捷动作，省去移动到工具栏按钮的往返
    m_doubleClickAction = addComboRow(form, MS_TR("Double Click Action"),
        MS_TR("Action performed when double clicking an empty area inside the selection."));
    m_doubleClickAction->addItem(MS_TR("Do Nothing"),
                                 static_cast<int>(CaptureDoubleClickAction::None));
    m_doubleClickAction->addItem(MS_TR("Copy And Close"),
                                 static_cast<int>(CaptureDoubleClickAction::Copy));
    m_doubleClickAction->addItem(MS_TR("Save To Default Folder"),
                                 static_cast<int>(CaptureDoubleClickAction::Save));
    m_doubleClickAction->addItem(MS_TR("Save As"),
                                 static_cast<int>(CaptureDoubleClickAction::SaveAs));
    m_doubleClickAction->addItem(MS_TR("Pin To Screen"),
                                 static_cast<int>(CaptureDoubleClickAction::Pin));
    m_doubleClickAction->addItem(MS_TR("Cancel Capture"),
                                 static_cast<int>(CaptureDoubleClickAction::Cancel));
    // 两条工具栏停靠屏幕边缘时位置固定，免去每次截图后寻找菜单（issue #146）
    m_toolbarPosition = addComboRow(form, MS_TR("Toolbar Position"),
        MS_TR("Where the toolbars appear after selecting a region."));
    m_toolbarPosition->addItem(MS_TR("Follow Selection"),
                               static_cast<int>(CaptureToolbarPosition::FollowSelection));
    m_toolbarPosition->addItem(MS_TR("Screen Top"),
                               static_cast<int>(CaptureToolbarPosition::ScreenTop));
    m_toolbarPosition->addItem(MS_TR("Screen Bottom"),
                               static_cast<int>(CaptureToolbarPosition::ScreenBottom));
    m_toolbarPosition->addItem(MS_TR("Screen Right"),
                               static_cast<int>(CaptureToolbarPosition::ScreenRight));
    m_selectionLoupe = addSwitchRow(form,
                                    MS_TR("Selection Loupe"),
                                    MS_TR("Show a magnifier near the cursor and nudge the pointer with arrow keys."));
    layout->addWidget(captureCard);
    layout->addWidget(compatibility);
    layout->addStretch();
}

void SettingsPageCapture::setConfig(const SettingsConfig &config)
{
    m_includeCursor->setChecked(config.capture.includeCursor);
    const int index = m_freezeScope->findData(static_cast<int>(config.capture.freezeScope));
    m_freezeScope->setCurrentIndex(index >= 0 ? index : 0);
    m_kdeKwinScreenshot->setChecked(config.capture.kdeKwinScreenshotEnabled);
    m_hideOwnWindows->setChecked(config.capture.hideOwnWindows);
    const int doubleClickIndex =
        m_doubleClickAction->findData(static_cast<int>(config.capture.doubleClickAction));
    m_doubleClickAction->setCurrentIndex(doubleClickIndex >= 0 ? doubleClickIndex : 0);
    const int toolbarPositionIndex =
        m_toolbarPosition->findData(static_cast<int>(config.capture.toolbarPosition));
    m_toolbarPosition->setCurrentIndex(toolbarPositionIndex >= 0 ? toolbarPositionIndex : 0);
    m_selectionLoupe->setChecked(config.capture.selectionLoupeEnabled);
    m_regionDetection->setChecked(config.capture.regionDetectionEnabled);
}

void SettingsPageCapture::updateConfig(SettingsConfig *config) const
{
    if (!config) {
        return;
    }

    config->capture.includeCursor = m_includeCursor->isChecked();
    config->capture.freezeScope =
        static_cast<CaptureFreezeScope>(m_freezeScope->currentData().toInt());
    config->capture.kdeKwinScreenshotEnabled = m_kdeKwinScreenshot->isChecked();
    config->capture.hideOwnWindows = m_hideOwnWindows->isChecked();
    config->capture.doubleClickAction =
        static_cast<CaptureDoubleClickAction>(m_doubleClickAction->currentData().toInt());
    config->capture.toolbarPosition =
        static_cast<CaptureToolbarPosition>(m_toolbarPosition->currentData().toInt());
    config->capture.selectionLoupeEnabled = m_selectionLoupe->isChecked();
    config->capture.regionDetectionEnabled = m_regionDetection->isChecked();
}

}  // namespace markshot::settings
