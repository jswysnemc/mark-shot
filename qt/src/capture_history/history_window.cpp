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

    m_previewTimer.setSingleShot(true);
    connect(&m_previewTimer, &QTimer::timeout, this, [this] { updatePreview(); });

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
    QSet<QString> names;
    for (const HistoryEntry &entry : entries) names.insert(entry.fileName);
    m_previews.retain(names);
    for (int row = m_list->count() - 1; row >= 0; --row) {
        if (!names.contains(m_list->item(row)->data(Qt::UserRole).toString())) delete m_list->takeItem(row);
    }
    QHash<QString, QListWidgetItem *> items;
    for (int row = 0; row < m_list->count(); ++row) {
        auto *item = m_list->item(row);
        items.insert(item->data(Qt::UserRole).toString(), item);
    }
    int selectedRow = 0;
    for (int row = 0; row < entries.size(); ++row) {
        const HistoryEntry &entry = entries.at(row);
        auto *item = items.value(entry.fileName);
        if (!item) {
            item = new QListWidgetItem;
            item->setData(Qt::UserRole, entry.fileName);
            m_list->insertItem(row, item);
        } else if (m_list->row(item) != row) {
            m_list->insertItem(row, m_list->takeItem(m_list->row(item)));
        }
        // 2. 【截图历史】【增量刷新】未变化条目不替换图标或模型索引，保留选择和滚动位置
        const QString version = HistoryPreviewCache::revision(m_store, entry.fileName);
        if (item->data(Qt::UserRole + 1).toString() != version) {
            const QPixmap pixmap = QPixmap::fromImage(m_previews.image(m_store, entry.fileName, QSize(56, 38)));
            QIcon icon(pixmap);
            icon.addPixmap(pixmap, QIcon::Selected);
            item->setIcon(icon);
            item->setText(entry.createdAt.toLocalTime().toString(QStringLiteral("MM-dd HH:mm:ss"))
                          + QStringLiteral("\n%1 × %2").arg(entry.imageSize.width()).arg(entry.imageSize.height()));
            item->setToolTip(entry.createdAt.toLocalTime().toString(QStringLiteral("yyyy-MM-dd HH:mm:ss")));
            item->setData(Qt::UserRole + 1, version);
        }
        if (entry.fileName == selected) selectedRow = row;
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
    const QString version = HistoryPreviewCache::revision(m_store, selectedFileName());
    if (version == m_selectedRevision && !m_currentImage.isNull()) return;
    m_selectedRevision = version;
    m_currentImage = {};
    m_preview->clear();
    for (QPushButton *button : m_imageActions) button->setEnabled(false);
    if (version.isEmpty()) updatePreview();
    else schedulePreview();
}

void HistoryWindow::schedulePreview()
{
    m_previewTimer.start(30);
}

void HistoryWindow::updatePreview()
{
    m_previewTimer.stop();
    const qreal dpr = devicePixelRatioF();
    const QSize target = (m_preview->contentsRect().size() - QSize(12, 12)) * dpr;
    if (target.isEmpty()) return;
    m_currentImage = m_previews.image(m_store, selectedFileName(), target);
    for (QPushButton *button : m_imageActions) button->setEnabled(!m_currentImage.isNull());
    if (m_currentImage.isNull()) {
        m_preview->setPixmap(QPixmap());
        m_preview->setText(m_list->count() == 0 ? MS_TR("No screenshots in history") : MS_TR("Screenshot is no longer available"));
        return;
    }
    QPixmap preview = QPixmap::fromImage(m_currentImage);
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
        schedulePreview();
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
