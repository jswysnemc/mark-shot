#include "selection_aspect/selection_ratio_popup.h"

#include "selection_aspect/selection_aspect_ratio.h"
#include "selection_aspect/selection_hud_painter.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPainter>

#include <algorithm>

namespace markshot::selection_aspect {
namespace {

// 弹层内边距
constexpr int kPopupPadding = 4;
// 单行高度
constexpr int kRowHeight = 26;
// 弹层宽度
constexpr int kPopupWidth = 128;
// 弹层与触发区域的间距
constexpr int kPopupGap = 4;

}  // namespace

SelectionRatioPopup::SelectionRatioPopup(QWidget *parent)
    : QWidget(parent)
{
    setAttribute(Qt::WA_TranslucentBackground, true);
    setAttribute(Qt::WA_NoSystemBackground, true);
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
    setCursor(Qt::PointingHandCursor);
    setFont(hud_style::hudFont());
    setAccessibleName(MS_TR("Lock aspect ratio"));
    hide();
}

void SelectionRatioPopup::popup(const QRect &anchor, qreal currentRatio)
{
    // 1. 记录锚点与当前比例，自定义比例时不标记任何预设
    m_anchor = anchor;
    m_currentIndex = presetIndexForRatio(currentRatio);
    m_activeIndex = std::max(0, m_currentIndex);

    // 2. 优先显示在锚点下方，空间不足时翻到上方，并限制在父控件内
    const QSize size = sizeHint();
    const QWidget *host = parentWidget();
    const int hostWidth = host ? host->width() : size.width();
    const int hostHeight = host ? host->height() : size.height();
    int x = std::clamp(anchor.left(), 4, std::max(4, hostWidth - size.width() - 4));
    int y = anchor.bottom() + 1 + kPopupGap;
    if (y + size.height() > hostHeight - 4) {
        y = anchor.top() - kPopupGap - size.height();
    }
    y = std::clamp(y, 4, std::max(4, hostHeight - size.height() - 4));
    setGeometry(QRect(QPoint(x, y), size));

    // 3. 显示并接管键盘，同时监听外部点击
    qApp->installEventFilter(this);
    show();
    raise();
    setFocus(Qt::PopupFocusReason);
    update();
}

void SelectionRatioPopup::setHudColors(const hud_style::HudColors &colors)
{
    m_colors = colors;
    update();
}

void SelectionRatioPopup::dismiss()
{
    if (!isVisible()) {
        return;
    }
    hide();
    emit closed();
}

QSize SelectionRatioPopup::sizeHint() const
{
    const int rows = static_cast<int>(aspectRatioPresets().size());
    return QSize(kPopupWidth, rows * kRowHeight + kPopupPadding * 2);
}

void SelectionRatioPopup::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    hud_style::paintPanelBackground(painter, QRectF(rect()), m_colors);
    painter.setFont(font());

    const QVector<AspectRatioPreset> presets = aspectRatioPresets();
    for (int i = 0; i < presets.size(); ++i) {
        const QRect row = rowRect(i);
        const bool current = i == m_currentIndex;
        // 1. 悬停或键盘选中行绘制浅色底
        if (i == m_activeIndex) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(m_colors.hover);
            painter.drawRoundedRect(QRectF(row), 5.0, 5.0);
        }
        // 2. 比例示意图与名称，当前比例使用强调色
        const QColor ink = current ? m_colors.accent : m_colors.text;
        const int glyphTop = row.top() + (row.height() - hud_style::kGlyphSize) / 2;
        const QRectF glyphBox(row.left() + 8, glyphTop, hud_style::kGlyphSize, hud_style::kGlyphSize);
        hud_style::paintRatioGlyph(painter, glyphBox, presets.at(i).ratio, ink);
        painter.setPen(ink);
        const QRect textRect = row.adjusted(8 + hud_style::kGlyphSize + 8, 0, -26, 0);
        painter.drawText(textRect, Qt::AlignVCenter | Qt::AlignLeft, markshot::i18n::translate(presets.at(i).label));
        // 3. 当前比例绘制选中标记
        if (current) {
            const QRectF checkBox(row.right() - 8 - hud_style::kGlyphSize, glyphTop, hud_style::kGlyphSize,
                                  hud_style::kGlyphSize);
            hud_style::paintCheckGlyph(painter, checkBox, m_colors.accent);
        }
    }
}

void SelectionRatioPopup::mouseMoveEvent(QMouseEvent *event)
{
    const int index = rowAt(event->position().toPoint());
    if (index >= 0 && index != m_activeIndex) {
        m_activeIndex = index;
        update();
    }
    event->accept();
}

void SelectionRatioPopup::mouseReleaseEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        const int index = rowAt(event->position().toPoint());
        if (index >= 0) {
            choose(index);
        }
    }
    event->accept();
}

void SelectionRatioPopup::mousePressEvent(QMouseEvent *event)
{
    event->accept();
}

void SelectionRatioPopup::leaveEvent(QEvent *event)
{
    update();
    QWidget::leaveEvent(event);
}

void SelectionRatioPopup::keyPressEvent(QKeyEvent *event)
{
    const int count = static_cast<int>(aspectRatioPresets().size());
    switch (event->key()) {
    case Qt::Key_Up:
        m_activeIndex = (m_activeIndex + count - 1) % count;
        update();
        break;
    case Qt::Key_Down:
    case Qt::Key_Tab:
        m_activeIndex = (m_activeIndex + 1) % count;
        update();
        break;
    case Qt::Key_Return:
    case Qt::Key_Enter:
    case Qt::Key_Space:
        choose(m_activeIndex);
        break;
    case Qt::Key_Escape:
        dismiss();
        break;
    default:
        break;
    }
    // 弹层打开期间按键不再传给截图窗口
    event->accept();
}

void SelectionRatioPopup::hideEvent(QHideEvent *event)
{
    qApp->removeEventFilter(this);
    QWidget::hideEvent(event);
}

bool SelectionRatioPopup::eventFilter(QObject *watched, QEvent *event)
{
    if (!isVisible()) {
        return false;
    }

    // 1. 弹层获得焦点时屏蔽截图窗口快捷键，按键交给 keyPressEvent
    if (event->type() == QEvent::ShortcutOverride && watched == this) {
        event->accept();
        return true;
    }
    if (event->type() != QEvent::MouseButtonPress && event->type() != QEvent::MouseButtonDblClick) {
        return false;
    }

    // 2. 只处理控件上的按下事件，窗口级重复事件忽略
    auto *widget = qobject_cast<QWidget *>(watched);
    if (!widget || widget == this || isAncestorOf(widget)) {
        return false;
    }
    if (widget->window() != window()) {
        dismiss();
        return false;
    }

    // 3. 点击弹层外部时关闭；同一窗口内的点击被吞掉，避免误触发画布绘制或重新打开弹层
    const auto *mouseEvent = static_cast<QMouseEvent *>(event);
    const QPoint windowPoint = widget->mapTo(window(), mouseEvent->position().toPoint());
    const QRect popupRect(mapTo(window(), QPoint(0, 0)), size());
    if (popupRect.contains(windowPoint)) {
        return false;
    }
    dismiss();
    return true;
}

QRect SelectionRatioPopup::rowRect(int index) const
{
    return QRect(kPopupPadding, kPopupPadding + index * kRowHeight, width() - kPopupPadding * 2, kRowHeight);
}

int SelectionRatioPopup::rowAt(const QPoint &position) const
{
    const int count = static_cast<int>(aspectRatioPresets().size());
    for (int i = 0; i < count; ++i) {
        if (rowRect(i).contains(position)) {
            return i;
        }
    }
    return -1;
}

void SelectionRatioPopup::choose(int index)
{
    const QVector<AspectRatioPreset> presets = aspectRatioPresets();
    if (index < 0 || index >= presets.size()) {
        return;
    }
    const qreal ratio = presets.at(index).ratio;
    dismiss();
    emit ratioSelected(ratio);
}

}  // namespace markshot::selection_aspect
