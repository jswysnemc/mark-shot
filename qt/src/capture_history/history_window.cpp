#include "capture_history/history_window.h"

#include "app_config_store.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QApplication>
#include <QCheckBox>
#include <QEvent>
#include <QLabel>
#include <QListWidget>
#include <QPointer>
#include <QPushButton>
#include <QResizeEvent>
#include <QScreen>
#include <QSignalBlocker>

namespace markshot::history {
HistoryWindow::HistoryWindow()
{
    // 1. 【截图历史】【界面】沿用应用面板样式，并按屏幕可用区域限制初始尺寸
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowTitle(MS_TR("Screenshot History"));
    setObjectName(QStringLiteral("historyWindow"));
    setFont(markshot::theme::uiFont(10));
    const QSize available = screen() ? screen()->availableGeometry().size() - QSize(24, 48) : QSize(800, 520);
    setMinimumSize(QSize(440, 320).boundedTo(available));
    resize(QSize(800, 520).boundedTo(available));

    buildLayout();
    applyTheme();
    refresh();
}

QString HistoryWindow::selectedFileName() const
{
    return m_list->currentItem() ? m_list->currentItem()->data(Qt::UserRole).toString() : QString();
}

void HistoryWindow::refresh()
{
    // 1. 【截图历史】【刷新】保留选中标识并读取存储中的当前内容
    const QString selected = selectedFileName();
    const auto entries = m_store.entries();
    const QSignalBlocker blocker(m_list);
    m_list->clear();
    int selectedRow = 0;
    for (const HistoryEntry &entry : entries) {
        QImage thumbnail = m_store.image(entry.fileName, QSize(56, 38));
        thumbnail.setDevicePixelRatio(1.0);
        const QString label = entry.createdAt.toLocalTime().toString(QStringLiteral("MM-dd HH:mm:ss"))
            + QStringLiteral("\n%1 × %2").arg(entry.imageSize.width()).arg(entry.imageSize.height());
        const QPixmap pixmap = QPixmap::fromImage(thumbnail);
        QIcon thumbnailIcon(pixmap);
        thumbnailIcon.addPixmap(pixmap, QIcon::Selected);
        auto *item = new QListWidgetItem(thumbnailIcon, label, m_list);
        item->setData(Qt::UserRole, entry.fileName);
        item->setToolTip(entry.createdAt.toLocalTime().toString(QStringLiteral("yyyy-MM-dd HH:mm:ss")));
        if (entry.fileName == selected) {
            selectedRow = m_list->count() - 1;
        }
    }

    // 2. 【截图历史】【刷新】同步预览、数量和自动记录开关
    m_list->setCurrentRow(entries.isEmpty() ? -1 : selectedRow);
    m_summary->setText(MS_TR("Screenshot History") + QStringLiteral(" · %1").arg(entries.size()));
    m_clear->setEnabled(!entries.isEmpty());
    const QSignalBlocker enabledBlocker(m_enabled);
    m_enabled->setChecked(historyConfigFromRoot(markshot::readAppConfigRoot()).enabled);
    updateSelection();
}

void HistoryWindow::updateSelection()
{
    showStatus({});
    m_currentImage = m_store.image(selectedFileName());
    for (QPushButton *button : m_imageActions) {
        button->setEnabled(!m_currentImage.isNull());
    }
    updatePreview();
}

void HistoryWindow::updatePreview()
{
    if (m_currentImage.isNull()) {
        m_preview->setPixmap(QPixmap());
        m_preview->setText(m_list->count() == 0 ? MS_TR("No screenshots in history") : MS_TR("Screenshot is no longer available"));
        return;
    }
    const qreal dpr = devicePixelRatioF();
    const QSize target = (m_preview->contentsRect().size() - QSize(12, 12)) * dpr;
    if (target.isEmpty()) {
        return;
    }
    QPixmap preview = QPixmap::fromImage(m_currentImage.scaled(target, Qt::KeepAspectRatio, Qt::SmoothTransformation));
    preview.setDevicePixelRatio(dpr);
    m_preview->setPixmap(preview);
}

bool HistoryWindow::event(QEvent *event)
{
    const bool handled = QWidget::event(event);
    if (m_list && (event->type() == QEvent::ApplicationPaletteChange || event->type() == QEvent::ThemeChange)) {
        applyTheme();
    }
    return handled;
}

void HistoryWindow::changeEvent(QEvent *event)
{
    QWidget::changeEvent(event);
    if (event->type() == QEvent::ActivationChange && isActiveWindow() && m_list) {
        applyTheme();
        refresh();
    }
}

void HistoryWindow::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    updateActionLayout();
    if (m_preview) {
        updatePreview();
    }
}

void HistoryWindow::showStatus(const QString &text)
{
    m_status->setText(text);
    m_status->setVisible(!text.isEmpty());
}

void showHistoryWindow()
{
    static QPointer<HistoryWindow> window;
    if (!window) {
        window = new HistoryWindow;
    } else {
        window->refresh();
    }
    window->show();
    window->raise();
    window->activateWindow();
}

}
