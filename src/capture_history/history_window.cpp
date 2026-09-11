#include "capture_history/history_window.h"

#include "app_config_store.h"
#include "shot_window.h"
#include "ui/i18n.h"
#include "ui/icons.h"
#include "ui/theme.h"

#include <QApplication>
#include <QBoxLayout>
#include <QCheckBox>
#include <QEvent>
#include <QGridLayout>
#include <QLabel>
#include <QListWidget>
#include <QPointer>
#include <QPushButton>
#include <QResizeEvent>
#include <QScreen>
#include <QShortcut>
#include <QSignalBlocker>
#include <QSplitter>

namespace markshot::history {
namespace {

/// @brief 创建采用现有图标与面板样式的历史操作按钮
/// @param text 按钮文字
/// @param action 工具图标编号
/// @param parent 所属窗口
/// @return 新建按钮
QPushButton *historyButton(const QString &text, ShotWindow::Action action, QWidget *parent)
{
    auto *button = new QPushButton(markshot::ui::makeToolIcon(action), text, parent);
    button->setIconSize(QSize(16, 16));
    button->setMinimumHeight(30);
    return button;
}

}

HistoryWindow::HistoryWindow()
{
    // 1. 【截图历史】【界面】沿用应用面板样式，并按屏幕可用区域限制初始尺寸
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowTitle(MS_TR("Screenshot History"));
    setObjectName(QStringLiteral("extensionPanel"));
    setStyleSheet(markshot::theme::openWithPanelStyleSheet() + QStringLiteral(
        "QPushButton:disabled { color: rgba(229, 231, 235, 80); }"
        "QCheckBox { color: #E5E7EB; spacing: 8px; }"
        "QCheckBox::indicator { width: 14px; height: 14px; border-radius: 3px;"
        " border: 1px solid #6B7280; background: #111827; }"
        "QCheckBox::indicator:checked { background: #2DD4BF; border-color: #5EEAD4; }"));
    setFont(markshot::theme::uiFont(12));
    const QSize available = screen() ? screen()->availableGeometry().size() - QSize(24, 48) : QSize(800, 520);
    setMinimumSize(QSize(440, 320).boundedTo(available));
    resize(QSize(800, 520).boundedTo(available));

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(12, 10, 12, 10);
    layout->setSpacing(8);
    m_summary = new QLabel(this);
    m_summary->setFont(markshot::theme::uiFont(13, QFont::DemiBold));
    layout->addWidget(m_summary);

    // 2. 【截图历史】【界面】列表与预览可调整宽度，预览始终保持原图比例
    auto *splitter = new QSplitter(Qt::Horizontal, this);
    splitter->setChildrenCollapsible(false);
    m_list = new QListWidget(splitter);
    m_list->setObjectName(QStringLiteral("historyList"));
    m_list->setMinimumWidth(150);
    m_list->setIconSize(QSize(88, 58));
    m_list->setSpacing(3);
    m_list->setSelectionMode(QAbstractItemView::SingleSelection);
    m_list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_list->setStyleSheet(QStringLiteral(
        "QListWidget { background: transparent; border: 0; }"
        "QListWidget::item { padding: 5px; border-radius: 6px; }"
        "QListWidget::item:selected { color: #E5E7EB; background: rgba(45, 212, 191, 35); }"));
    m_preview = new QLabel(splitter);
    m_preview->setObjectName(QStringLiteral("historyPreview"));
    m_preview->setMinimumSize(180, 140);
    m_preview->setAlignment(Qt::AlignCenter);
    m_preview->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Ignored);
    m_preview->setStyleSheet(QStringLiteral(
        "background: rgba(0, 0, 0, 35); border-radius: 8px; padding: 6px;"));
    splitter->addWidget(m_list);
    splitter->addWidget(m_preview);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({250, 526});
    layout->addWidget(splitter, 1);
    connect(splitter, &QSplitter::splitterMoved, this, [this] { updatePreview(); });
    connect(m_list, &QListWidget::currentItemChanged, this, [this] { updateSelection(); });
    connect(m_list, &QListWidget::itemDoubleClicked, this, [this] { editSelected(); });

    // 3. 【截图历史】【操作】复用复制、标注和钉图入口，历史再次复制时不重复入库
    auto *actions = new QGridLayout;
    auto *copy = historyButton(MS_TR("Copy"), ShotWindow::Action::Copy, this);
    auto *edit = historyButton(MS_TR("Edit"), ShotWindow::Action::ToolPen, this);
    auto *pin = historyButton(MS_TR("Pin"), ShotWindow::Action::Pin, this);
    auto *save = historyButton(MS_TR("Save As"), ShotWindow::Action::Save, this);
    auto *remove = historyButton(MS_TR("Delete"), ShotWindow::Action::Clear, this);
    m_clear = historyButton(MS_TR("Clear History"), ShotWindow::Action::Clear, this);
    m_imageActions = {copy, edit, pin, save, remove};
    const QVector<QPushButton *> buttons = {copy, edit, pin, save, remove, m_clear};
    for (int index = 0; index < buttons.size(); ++index) {
        actions->addWidget(buttons.at(index), index / 3, index % 3);
    }
    layout->addLayout(actions);
    copy->setShortcut(QKeySequence::Copy);
    remove->setShortcut(QKeySequence(Qt::Key_Delete));
    connect(copy, &QPushButton::clicked, this, [this] { copySelected(); });
    connect(edit, &QPushButton::clicked, this, [this] { editSelected(); });
    connect(pin, &QPushButton::clicked, this, [this] { pinSelected(); });
    connect(save, &QPushButton::clicked, this, [this] { saveSelected(); });
    connect(remove, &QPushButton::clicked, this, [this] { removeSelected(); });
    connect(m_clear, &QPushButton::clicked, this, [this] { confirmClear(); });

    m_enabled = new QCheckBox(MS_TR("Keep screenshot history"), this);
    m_enabled->setChecked(historyConfigFromRoot(markshot::readAppConfigRoot()).enabled);
    connect(m_enabled, &QCheckBox::toggled, this, [this](bool enabled) { setRecordingEnabled(enabled); });
    layout->addWidget(m_enabled);
    m_status = new QLabel(this);
    m_status->setWordWrap(true);
    m_status->setMinimumHeight(18);
    layout->addWidget(m_status);
    auto *closeShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    connect(closeShortcut, &QShortcut::activated, this, &QWidget::close);
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
        QImage thumbnail = m_store.image(entry.fileName, QSize(88, 58));
        thumbnail.setDevicePixelRatio(1.0);
        const QString label = entry.createdAt.toLocalTime().toString(QStringLiteral("MM-dd HH:mm:ss"))
            + QStringLiteral("\n%1 × %2").arg(entry.imageSize.width()).arg(entry.imageSize.height());
        auto *item = new QListWidgetItem(QIcon(QPixmap::fromImage(thumbnail)), label, m_list);
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

void HistoryWindow::changeEvent(QEvent *event)
{
    QWidget::changeEvent(event);
    if (event->type() == QEvent::ActivationChange && isActiveWindow() && m_list) {
        refresh();
    }
}

void HistoryWindow::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    if (m_preview) {
        updatePreview();
    }
}

void HistoryWindow::showStatus(const QString &text)
{
    m_status->setText(text);
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
