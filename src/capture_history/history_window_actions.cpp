#include "capture_history/history_window.h"

#include "annotation_launch.h"
#include "app_config_store.h"
#include "clipboard_image.h"
#include "pinned_window/pinned_image_window.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QBoxLayout>
#include <QCheckBox>
#include <QDialog>
#include <QFileDialog>
#include <QLabel>
#include <QPushButton>
#include <QSignalBlocker>

namespace markshot::history {

void HistoryWindow::copySelected()
{
    if (!m_currentImage.isNull()) {
        showStatus(markshot::copyImageToClipboard(m_currentImage) ? MS_TR("Copied") : MS_TR("Copy failed"));
    }
}

void HistoryWindow::editSelected()
{
    if (!m_currentImage.isNull()) {
        markshot::openImageForAnnotation(m_currentImage, MS_TR("Screenshot History"));
    }
}

void HistoryWindow::pinSelected()
{
    if (m_currentImage.isNull()) {
        return;
    }
    auto *window = new markshot::shot::PinnedImageWindow(m_currentImage);
    window->show();
    window->raise();
    window->activateWindow();
}

void HistoryWindow::saveSelected()
{
    if (m_currentImage.isNull()) {
        return;
    }
    // 1. 【截图历史】【另存为】使用应用样式的文件对话框选择输出文件
    auto *dialog = new QFileDialog(this, MS_TR("Save Screenshot"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->setOption(QFileDialog::DontUseNativeDialog, true);
    dialog->setAcceptMode(QFileDialog::AcceptSave);
    dialog->setFileMode(QFileDialog::AnyFile);
    dialog->setDefaultSuffix(QStringLiteral("png"));
    dialog->setNameFilter(MS_TR("PNG Images (*.png)"));
    dialog->selectFile(selectedFileName());
    const QImage image = m_currentImage;

    // 2. 【截图历史】【另存为】保存选择时的图像快照，不创建重复历史
    connect(dialog, &QFileDialog::accepted, this, [this, dialog, image] {
        const auto files = dialog->selectedFiles();
        if (!files.isEmpty()) {
            showStatus(image.save(files.first(), "PNG") ? MS_TR("Saved to %1").arg(files.first()) : MS_TR("Save failed"));
        }
    });
    dialog->open();
}

void HistoryWindow::removeSelected()
{
    QString error;
    const bool removed = m_store.remove(selectedFileName(), &error);
    refresh();
    showStatus(removed ? MS_TR("Screenshot removed") : MS_TR("History operation failed: %1").arg(error));
}

void HistoryWindow::confirmClear()
{
    // 1. 【截图历史】【清空】先说明删除范围，并将默认操作设为取消
    auto *dialog = new QDialog(this);
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->setWindowTitle(MS_TR("Clear History"));
    dialog->setObjectName(QStringLiteral("extensionPanel"));
    dialog->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *layout = new QVBoxLayout(dialog);
    auto *label = new QLabel(MS_TR("Delete all screenshot history? Saved files will be kept."), dialog);
    label->setWordWrap(true);
    layout->addWidget(label);
    auto *buttons = new QHBoxLayout;
    auto *cancel = new QPushButton(MS_TR("Cancel"), dialog);
    auto *clear = new QPushButton(MS_TR("Clear History"), dialog);
    cancel->setDefault(true);
    buttons->addWidget(cancel);
    buttons->addWidget(clear);
    layout->addLayout(buttons);
    connect(cancel, &QPushButton::clicked, dialog, &QDialog::reject);
    connect(clear, &QPushButton::clicked, dialog, &QDialog::accept);

    // 2. 【截图历史】【清空】确认后只删除历史目录中的截图记录
    connect(dialog, &QDialog::accepted, this, [this] {
        QString error;
        const bool cleared = m_store.clear(&error);
        refresh();
        showStatus(cleared ? MS_TR("History cleared") : MS_TR("History operation failed: %1").arg(error));
    });
    dialog->open();
}

void HistoryWindow::setRecordingEnabled(bool enabled)
{
    QString error;
    if (!markshot::writeAppConfigValue({QStringLiteral("captureHistory"), QStringLiteral("enabled")},
                                      enabled, &error)) {
        const QSignalBlocker blocker(m_enabled);
        m_enabled->setChecked(!enabled);
        showStatus(MS_TR("History operation failed: %1").arg(error));
        return;
    }
    showStatus(enabled ? MS_TR("Screenshot history enabled") : MS_TR("Screenshot history disabled"));
}

}
