#include "capture_history/history_window.h"

#include "app_config_store.h"
#include "shot_window.h"
#include "ui/i18n.h"
#include "ui/icons.h"
#include "ui/theme.h"

#include <QBoxLayout>
#include <QCheckBox>
#include <QLabel>
#include <QListWidget>
#include <QPushButton>
#include <QShortcut>
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
    button->setFont(markshot::theme::uiFont(10));
    button->setMinimumHeight(28);
    button->setToolTip(text);
    button->setAccessibleName(text);
    button->setProperty("actionText", text);
    button->setProperty("historyAction", static_cast<int>(action));
    button->setCursor(Qt::PointingHandCursor);
    return button;
}

}

void HistoryWindow::buildLayout()
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(12, 10, 12, 10);
    layout->setSpacing(6);
    m_summary = new QLabel(this);
    m_summary->setFont(markshot::theme::uiFont(10, QFont::DemiBold));
    auto *header = new QHBoxLayout;
    header->addWidget(m_summary, 1);
    auto *close = historyButton(QString(), ShotWindow::Action::Cancel, this);
    close->setObjectName(QStringLiteral("historyCloseButton"));
    close->setProperty("role", QStringLiteral("quiet"));
    close->setFixedSize(32, 32);
    close->setToolTip(MS_TR("Close"));
    close->setAccessibleName(MS_TR("Close"));
    connect(close, &QPushButton::clicked, this, &QWidget::close);
    header->addWidget(close);
    layout->addLayout(header);

    // 2. 【截图历史】【界面】列表与预览可调整宽度，预览始终保持原图比例
    auto *splitter = new QSplitter(Qt::Horizontal, this);
    splitter->setChildrenCollapsible(false);
    m_list = new QListWidget(splitter);
    m_list->setObjectName(QStringLiteral("historyList"));
    m_list->setFont(markshot::theme::uiFont(9));
    m_list->setMinimumWidth(184);
    m_list->setIconSize(QSize(56, 38));
    m_list->setSpacing(2);
    m_list->setUniformItemSizes(true);
    m_list->setTextElideMode(Qt::ElideRight);
    m_list->setSelectionMode(QAbstractItemView::SingleSelection);
    m_list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_preview = new QLabel(splitter);
    m_preview->setObjectName(QStringLiteral("historyPreview"));
    m_preview->setFont(markshot::theme::uiFont(10));
    m_preview->setWordWrap(true);
    m_preview->setMinimumSize(180, 100);
    m_preview->setAlignment(Qt::AlignCenter);
    m_preview->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Ignored);
    splitter->addWidget(m_list);
    splitter->addWidget(m_preview);
    splitter->setStretchFactor(1, 1);
    splitter->setSizes({210, 566});
    layout->addWidget(splitter, 1);
    connect(splitter, &QSplitter::splitterMoved, this, [this] { updatePreview(); });
    connect(m_list, &QListWidget::currentItemChanged, this, [this] { updateSelection(); });
    connect(m_list, &QListWidget::itemDoubleClicked, this, [this] { editSelected(); });

    // 3. 【截图历史】【操作】复用复制、标注和钉图入口，历史再次复制时不重复入库
    m_actions = new QHBoxLayout;
    m_actions->setSpacing(4);
    auto *copy = historyButton(MS_TR("Copy"), ShotWindow::Action::Copy, this);
    auto *edit = historyButton(MS_TR("Edit"), ShotWindow::Action::ToolPen, this);
    auto *pin = historyButton(MS_TR("Pin"), ShotWindow::Action::Pin, this);
    auto *save = historyButton(MS_TR("Save As"), ShotWindow::Action::Save, this);
    auto *remove = historyButton(MS_TR("Delete"), ShotWindow::Action::Clear, this);
    m_clear = historyButton(MS_TR("Clear History"), ShotWindow::Action::Clear, this);
    m_imageActions = {copy, edit, pin, save, remove};
    copy->setProperty("role", QStringLiteral("primary"));
    for (QPushButton *button : m_imageActions) {
        m_actions->addWidget(button);
    }
    layout->insertLayout(1, m_actions);
    copy->setShortcut(QKeySequence::Copy);
    remove->setShortcut(QKeySequence(Qt::Key_Delete));
    connect(copy, &QPushButton::clicked, this, [this] { copySelected(); });
    connect(edit, &QPushButton::clicked, this, [this] { editSelected(); });
    connect(pin, &QPushButton::clicked, this, [this] { pinSelected(); });
    connect(save, &QPushButton::clicked, this, [this] { saveSelected(); });
    connect(remove, &QPushButton::clicked, this, [this] { removeSelected(); });
    connect(m_clear, &QPushButton::clicked, this, [this] { confirmClear(); });

    m_enabled = new QCheckBox(MS_TR("Keep screenshot history"), this);
    m_enabled->setFont(markshot::theme::uiFont(9));
    m_enabled->setChecked(historyConfigFromRoot(markshot::readAppConfigRoot()).enabled);
    connect(m_enabled, &QCheckBox::toggled, this, [this](bool enabled) { setRecordingEnabled(enabled); });
    auto *footer = new QHBoxLayout;
    footer->addWidget(m_enabled);
    footer->addStretch();
    m_clear->setProperty("role", QStringLiteral("quiet"));
    footer->addWidget(m_clear);
    layout->addLayout(footer);
    m_status = new QLabel(this);
    m_status->setProperty("role", QStringLiteral("muted"));
    m_status->setWordWrap(true);
    m_status->hide();
    layout->addWidget(m_status);
    auto *closeShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    connect(closeShortcut, &QShortcut::activated, this, &QWidget::close);
    updateActionLayout();
}

void HistoryWindow::updateActionLayout()
{
    if (!m_actions) {
        return;
    }
    // 1. 【截图历史】【紧凑布局】空间不足时保留图标与提示，不增加操作栏行数
    int requiredWidth = 0;
    for (QPushButton *button : m_imageActions) {
        button->setText(button->property("actionText").toString());
        requiredWidth += button->sizeHint().width();
    }
    requiredWidth += m_actions->spacing() * (m_imageActions.size() - 1);
    if (requiredWidth > width() - 24) {
        for (QPushButton *button : m_imageActions) {
            button->setText(QString());
        }
    }
}

}
