#include "display_capture/display_capture_picker.h"

#include "display_capture/display_capture_card.h"
#include "display_capture/display_capture_picker_style.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QFont>
#include <QKeyEvent>
#include <QLabel>
#include <QVBoxLayout>

namespace markshot::display_capture {
namespace {

/**
 * 移除布局中的所有子项。
 * @param layout 需要清空的布局。
 * @return 无返回值。
 */
void clearLayout(QLayout *layout)
{
    if (!layout) {
        return;
    }
    while (QLayoutItem *item = layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            widget->deleteLater();
        }
        delete item;
    }
}

}  // namespace

/**
 * 创建显示器快速截取选择器。
 * @param palette 截图会话的工具栏配色。
 * @param parent 父级窗口。
 */
DisplayCapturePicker::DisplayCapturePicker(const ui::ToolbarPalette &palette, QWidget *parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("displayCapturePicker"));
    setFocusPolicy(Qt::StrongFocus);
    // 1. 【显示器截取】【主题】面板与悬停操作栏跟随截图会话主题
    setStyleSheet(displayCapturePickerStyleSheet(palette));

    auto *rootLayout = new QVBoxLayout(this);
    rootLayout->setContentsMargins(12, 12, 12, 12);
    rootLayout->setSpacing(10);
    m_cardLayout = rootLayout;
}

/**
 * 设置显示器捕获目标。
 * @param targets 可选择的显示器目标列表。
 * @return 无返回值。
 */
void DisplayCapturePicker::setTargets(const QVector<Target> &targets)
{
    clearLayout(m_cardLayout);

    for (int i = 0; i < targets.size(); ++i) {
        auto *card = new DisplayCaptureCard(i, targets.at(i), this);
        connect(card, &DisplayCaptureCard::copyRequested, this, &DisplayCapturePicker::copyRequested);
        connect(card, &DisplayCaptureCard::editRequested, this, &DisplayCapturePicker::editRequested);
        connect(card, &DisplayCaptureCard::saveRequested, this, &DisplayCapturePicker::saveRequested);
        m_cardLayout->addWidget(card);
    }

    if (targets.isEmpty()) {
        auto *empty = new QLabel(MS_TR("No displays found"), this);
        empty->setObjectName(QStringLiteral("displayCaptureSubtitle"));
        empty->setAlignment(Qt::AlignCenter);
        empty->setFont(markshot::theme::uiFont(11, QFont::DemiBold));
        m_cardLayout->addWidget(empty);
    }

    adjustSize();
}

/**
 * 处理选择器键盘输入。
 * @param event 键盘事件。
 * @return 无返回值。
 */
void DisplayCapturePicker::keyPressEvent(QKeyEvent *event)
{
    if (event->key() == Qt::Key_Escape) {
        emit dismissed();
        event->accept();
        return;
    }
    QWidget::keyPressEvent(event);
}

}  // namespace markshot::display_capture
