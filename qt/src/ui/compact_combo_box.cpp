#include "ui/compact_combo_box.h"

#include "ui/context_menu.h"
#include "ui/svg_icon.h"

#include <QEnterEvent>
#include <QFontMetrics>
#include <QPainter>

#include <algorithm>

namespace markshot::ui {
namespace {

// 按钮尺寸令牌，颜色取自工具栏配色
constexpr int kHorizontalPadding = 8;
constexpr int kChevronSize = 12;
constexpr int kChevronGap = 6;
constexpr qreal kRadius = 6.0;

}  // namespace

CompactComboBox::CompactComboBox(QWidget *parent)
    : QComboBox(parent)
{
    setAttribute(Qt::WA_Hover, true);
    setFocusPolicy(Qt::NoFocus);
    setCursor(Qt::ArrowCursor);
}

void CompactComboBox::showPopup()
{
    if (count() <= 0) {
        return;
    }
    if (m_menu && m_menu->isVisible()) {
        m_menu->dismiss();
        return;
    }

    // 1. 每次打开时按当前数据重建，选项变化与选中状态始终一致
    if (!m_menu) {
        m_menu = new ContextMenu(this);
        m_menu->setColorScheme(m_toolbarPalette.dark ? ContextMenu::ColorScheme::Dark
                                                     : ContextMenu::ColorScheme::Light);
        connect(m_menu, &ContextMenu::popupFinished, this, [this] { update(); });
    }
    m_menu->clear();
    for (int index = 0; index < count(); ++index) {
        QAction *action = m_menu->addAction(itemIcon(index), itemText(index));
        action->setCheckable(true);
        // 单选状态由当前项决定，菜单使用对勾标记而非原生单选圆点
        action->setChecked(index == currentIndex());
        // 2. 选择后同步当前项，并发出与原生下拉框一致的 activated 信号
        connect(action, &QAction::triggered, this, [this, index] {
            setCurrentIndex(index);
            emit activated(index);
        });
    }

    // 3. 在按钮下方展开，菜单宽度不小于按钮宽度
    m_menu->setMinimumWidth(width());
    m_menu->popupBelow(this);
    update();
}

void CompactComboBox::hidePopup()
{
    if (m_menu) {
        m_menu->dismiss();
    }
    update();
}

QSize CompactComboBox::sizeHint() const
{
    const QFontMetrics metrics(font());
    int textWidth = metrics.horizontalAdvance(currentText());
    for (int index = 0; index < count(); ++index) {
        textWidth = std::max(textWidth, metrics.horizontalAdvance(itemText(index)));
    }
    const int width = kHorizontalPadding * 2 + textWidth + kChevronGap + kChevronSize;
    const int height = std::max(22, metrics.height() + 8);
    return QSize(width, height);
}

QSize CompactComboBox::minimumSizeHint() const
{
    return sizeHint();
}

void CompactComboBox::setToolbarPalette(const ToolbarPalette &palette)
{
    // 1. 记录配色；已创建的菜单同步明暗主题，按钮在下次绘制时生效
    m_toolbarPalette = palette;
    if (m_menu) {
        m_menu->setColorScheme(palette.dark ? ContextMenu::ColorScheme::Dark : ContextMenu::ColorScheme::Light);
    }
    update();
}

void CompactComboBox::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    const bool open = m_menu && m_menu->isVisible();
    const bool enabled = isEnabled();

    // 1. 圆角底板：悬停加亮，展开时使用强调色细边
    const QRectF frame = QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5);
    const ToolbarPalette &colors = m_toolbarPalette;
    painter.setPen(QPen(open ? colors.focusBorder : colors.controlBorder, 1));
    painter.setBrush(m_hovered && enabled ? colors.controlHoverFill : colors.controlFill);
    painter.drawRoundedRect(frame, kRadius, kRadius);

    // 2. 当前选项文本，空间不足时省略尾部
    const QRect chevronRect(width() - kHorizontalPadding - kChevronSize, (height() - kChevronSize) / 2, kChevronSize,
                            kChevronSize);
    const QRect textRect(kHorizontalPadding, 0, chevronRect.left() - kChevronGap - kHorizontalPadding, height());
    painter.setFont(font());
    painter.setPen(enabled ? colors.text : colors.mutedText);
    painter.drawText(textRect, Qt::AlignVCenter | Qt::AlignLeft,
                     QFontMetrics(font()).elidedText(currentText(), Qt::ElideRight, textRect.width()));

    // 3. 统一 SVG 下拉箭头
    const QPixmap chevron = renderSvgIcon(QStringLiteral("chevron-down"), open ? colors.focusBorder : colors.mutedText,
                                          chevronRect.size(), devicePixelRatioF());
    painter.drawPixmap(chevronRect, chevron);
}

void CompactComboBox::enterEvent(QEnterEvent *event)
{
    m_hovered = true;
    update();
    QComboBox::enterEvent(event);
}

void CompactComboBox::leaveEvent(QEvent *event)
{
    m_hovered = false;
    update();
    QComboBox::leaveEvent(event);
}

}  // namespace markshot::ui
