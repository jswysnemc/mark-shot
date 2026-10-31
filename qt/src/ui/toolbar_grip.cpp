#include "ui/toolbar_grip.h"

#include <QEvent>
#include <QPainter>
#include <QSizePolicy>

namespace markshot::ui {
namespace {

// 握柄窄边尺寸，与旧版条形握柄保持一致，避免工具栏整体尺寸变化
constexpr int kGripThickness = 10;
constexpr qreal kDotRadius = 1.25;
constexpr qreal kDotGap = 4.0;
constexpr int kDotRows = 3;
constexpr int kDotColumns = 2;

}

ToolbarGrip::ToolbarGrip(QWidget *parent)
    : QWidget(parent)
{
    // 1. 标记为拖拽握柄，交互光标策略据此保留抓手光标
    setProperty("dragHandle", true);
    setCursor(Qt::OpenHandCursor);
    setAttribute(Qt::WA_Hover, true);
    setOrientation(Qt::Horizontal);
    setToolbarPalette(ToolbarPalette{});
}

void ToolbarGrip::setOrientation(Qt::Orientation orientation)
{
    m_orientation = orientation;
    // 1. 横向工具栏握柄为竖条，纵向工具栏握柄为横条
    if (orientation == Qt::Horizontal) {
        setMinimumSize(0, 0);
        setMaximumSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX);
        setFixedWidth(kGripThickness);
        setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Expanding);
    } else {
        setMinimumSize(0, 0);
        setMaximumSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX);
        setFixedHeight(kGripThickness);
        setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    }
    updateGeometry();
    update();
}

void ToolbarGrip::setToolbarPalette(const ToolbarPalette &palette)
{
    m_dotColor = palette.gripDot;
    m_hoverDotColor = palette.gripDotHover;
    m_disabledDotColor = palette.gripDotDisabled;
    m_hoverFill = palette.gripHoverFill;
    update();
}

void ToolbarGrip::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event)
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    const bool hovered = isEnabled() && underMouse();
    // 1. 悬停时绘制同色系圆角底板，提示可拖动区域
    if (hovered) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(m_hoverFill);
        painter.drawRoundedRect(QRectF(rect()).adjusted(0.5, 1.5, -0.5, -1.5), 3.0, 3.0);
    }

    // 2. 按排列方向绘制 2x3 点阵，纵向工具栏旋转为 3x2
    const int rows = m_orientation == Qt::Horizontal ? kDotRows : kDotColumns;
    const int columns = m_orientation == Qt::Horizontal ? kDotColumns : kDotRows;
    const QColor dotColor = !isEnabled() ? m_disabledDotColor : hovered ? m_hoverDotColor : m_dotColor;
    painter.setPen(Qt::NoPen);
    painter.setBrush(dotColor);
    const QPointF origin(width() / 2.0 - (columns - 1) * kDotGap / 2.0,
                         height() / 2.0 - (rows - 1) * kDotGap / 2.0);
    for (int row = 0; row < rows; ++row) {
        for (int column = 0; column < columns; ++column) {
            painter.drawEllipse(origin + QPointF(column * kDotGap, row * kDotGap), kDotRadius, kDotRadius);
        }
    }
}

bool ToolbarGrip::event(QEvent *event)
{
    // 1. 悬停与启用状态变化时重绘点阵
    switch (event->type()) {
    case QEvent::HoverEnter:
    case QEvent::HoverLeave:
    case QEvent::EnabledChange:
        update();
        break;
    default:
        break;
    }
    return QWidget::event(event);
}

}
