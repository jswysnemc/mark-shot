#include "selection_aspect/selection_size_panel.h"

#include "selection_aspect/selection_hud_painter.h"
#include "selection_aspect/selection_ratio_popup.h"
#include "selection_aspect/selection_size_field.h"
#include "ui/i18n.h"

#include <QEvent>
#include <QFontMetrics>
#include <QMouseEvent>
#include <QPainter>
#include <QTimer>
#include <QWheelEvent>

#include <algorithm>

namespace markshot::selection_aspect {
namespace {

using namespace hud_style;

// 宽高之间的乘号
const QString kTimesText = QStringLiteral("×");
// 像素单位
const QString kUnitText = QStringLiteral("px");
// 锁形按钮宽度
constexpr int kLockWidth = 22;
// 比例触发区内部间距
constexpr int kRatioInnerGap = 5;
// 下拉箭头宽度
constexpr int kChevronWidth = 10;

}  // namespace

SelectionSizePanel::SelectionSizePanel(QWidget *parent)
    : QWidget(parent)
{
    // 1. 自绘背景，不抢焦点，避免影响截图窗口快捷键与标注输入
    setAttribute(Qt::WA_NoSystemBackground, true);
    setAutoFillBackground(false);
    setFocusPolicy(Qt::NoFocus);
    setMouseTracking(true);
    setCursor(Qt::ArrowCursor);
    setFont(hudFont());
    setAccessibleName(MS_TR("Selection width") + QStringLiteral(" / ") + MS_TR("Selection height"));

    // 2. 宽、高两个可编辑字段
    m_widthField = new SelectionSizeField(MS_TR("Selection width"), this);
    m_heightField = new SelectionSizeField(MS_TR("Selection height"), this);
    const auto bindField = [this](SelectionSizeField *field, SizeField side, SelectionSizeField *next) {
        connect(field, &SelectionSizeField::valueCommitted, this, [this, side](int value) { commitField(side, value); });
        connect(field, &SelectionSizeField::stepRequested, this, [this, side](int delta) { stepField(side, delta); });
        connect(field, &SelectionSizeField::focusNextRequested, next, &SelectionSizeField::beginEditing);
        connect(field, &SelectionSizeField::editingClosed, this, &SelectionSizePanel::focusReleaseRequested);
    };
    bindField(m_widthField, SizeField::Width, m_heightField);
    bindField(m_heightField, SizeField::Height, m_widthField);

    // 3. 计算固定布局，数值变化时 HUD 宽度保持稳定
    relayout();
    resize(m_contentSize);
}

void SelectionSizePanel::setValues(QSize size, QSize maximum, qreal ratio)
{
    m_size = QSize(std::max(1, size.width()), std::max(1, size.height()));
    m_maximum = QSize(std::max(1, maximum.width()), std::max(1, maximum.height()));
    m_ratio = std::max<qreal>(0.0, ratio);
    m_widthField->setValue(m_size.width(), m_maximum.width());
    m_heightField->setValue(m_size.height(), m_maximum.height());
    update();
}

void SelectionSizePanel::focusWidth()
{
    if (m_ratioPopup) {
        m_ratioPopup->dismiss();
    }
    m_widthField->beginEditing();
}

bool SelectionSizePanel::isInteracting() const
{
    return m_widthField->isEditing() || m_heightField->isEditing() || (m_ratioPopup && m_ratioPopup->isVisible());
}

void SelectionSizePanel::watchObstacle(QWidget *widget)
{
    if (widget && widget != this) {
        widget->installEventFilter(this);
    }
}

void SelectionSizePanel::setToolbarPalette(const ui::ToolbarPalette &palette)
{
    // 1. 输入框、比例弹层与 HUD 本身使用同一组颜色
    m_colors = hud_style::hudColors(palette);
    m_widthField->setHudColors(m_colors);
    m_heightField->setHudColors(m_colors);
    if (m_ratioPopup) {
        m_ratioPopup->setHudColors(m_colors);
    }
    update();
}

QSize SelectionSizePanel::sizeHint() const
{
    return m_contentSize;
}

void SelectionSizePanel::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    paintPanelBackground(painter, QRectF(rect()), m_colors);
    painter.setFont(font());

    // 1. 乘号、单位与分隔线
    painter.setPen(m_colors.muted);
    painter.drawText(m_timesRect, Qt::AlignCenter, kTimesText);
    painter.drawText(m_unitRect, Qt::AlignCenter, kUnitText);
    painter.setPen(Qt::NoPen);
    painter.setBrush(m_colors.separator);
    painter.drawRect(m_separatorRect);

    // 2. 悬停或弹层打开时绘制可点击区域的浅色底
    const bool popupOpen = m_ratioPopup && m_ratioPopup->isVisible();
    painter.setBrush(m_colors.hover);
    if (m_hoverZone == HotZone::Lock) {
        painter.drawRoundedRect(QRectF(m_lockRect), 5.0, 5.0);
    }
    if (m_hoverZone == HotZone::Ratio || popupOpen) {
        painter.drawRoundedRect(QRectF(m_ratioRect), 5.0, 5.0);
    }

    // 3. 锁形图标，锁定时使用强调色
    const bool locked = m_ratio > 0.0;
    const QColor lockInk = locked ? m_colors.accent : m_colors.muted;
    const QRectF lockBox(m_lockRect.center().x() - kGlyphSize / 2.0 + 0.5, m_lockRect.center().y() - kGlyphSize / 2.0 + 0.5,
                         kGlyphSize, kGlyphSize);
    paintLockGlyph(painter, lockBox, locked, lockInk);

    // 4. 比例示意图、名称与下拉箭头
    const QColor ratioInk = locked ? m_colors.accent : m_colors.text;
    const qreal glyphTop = m_ratioRect.center().y() - kGlyphSize / 2.0 + 0.5;
    paintRatioGlyph(painter, QRectF(m_ratioRect.left() + kRatioInnerGap, glyphTop, kGlyphSize, kGlyphSize), m_ratio,
                    ratioInk);
    const int textLeft = m_ratioRect.left() + kRatioInnerGap + kGlyphSize + kRatioInnerGap;
    const QRect textRect(textLeft, m_ratioRect.top(), m_ratioRect.right() - kChevronWidth - kRatioInnerGap - textLeft,
                         m_ratioRect.height());
    painter.setPen(ratioInk);
    painter.drawText(textRect, Qt::AlignVCenter | Qt::AlignLeft, ratioLabel());
    const QRectF chevronBox(m_ratioRect.right() - kChevronWidth - kRatioInnerGap + 1, glyphTop, kChevronWidth,
                            kGlyphSize);
    paintChevronGlyph(painter, chevronBox, m_colors.muted);
}

void SelectionSizePanel::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        switch (zoneAt(event->position().toPoint())) {
        case HotZone::Lock:
            toggleLock();
            break;
        case HotZone::Ratio:
            toggleRatioPopup();
            break;
        case HotZone::None:
            break;
        }
    }
    // HUD 上的点击不穿透到截图画布
    event->accept();
}

void SelectionSizePanel::mouseMoveEvent(QMouseEvent *event)
{
    const HotZone zone = zoneAt(event->position().toPoint());
    if (zone != m_hoverZone) {
        m_hoverZone = zone;
        setCursor(zone == HotZone::None ? Qt::ArrowCursor : Qt::PointingHandCursor);
        setToolTip(zone == HotZone::None ? QString() : MS_TR("Lock aspect ratio"));
        update();
    }
    event->accept();
}

void SelectionSizePanel::leaveEvent(QEvent *event)
{
    if (m_hoverZone != HotZone::None) {
        m_hoverZone = HotZone::None;
        update();
    }
    QWidget::leaveEvent(event);
}

void SelectionSizePanel::wheelEvent(QWheelEvent *event)
{
    event->accept();
}

void SelectionSizePanel::hideEvent(QHideEvent *event)
{
    m_widthField->cancelEditing();
    m_heightField->cancelEditing();
    if (m_ratioPopup) {
        m_ratioPopup->dismiss();
    }
    m_hoverZone = HotZone::None;
    QWidget::hideEvent(event);
}

bool SelectionSizePanel::eventFilter(QObject *watched, QEvent *event)
{
    // 避让控件变化后合并到下一轮事件循环统一重新定位
    const QEvent::Type type = event->type();
    if (watched != this && (type == QEvent::Show || type == QEvent::Hide || type == QEvent::Move || type == QEvent::Resize)
        && !m_layoutPending) {
        m_layoutPending = true;
        QTimer::singleShot(0, this, [this] {
            m_layoutPending = false;
            emit layoutRequested();
        });
    }
    return QWidget::eventFilter(watched, event);
}

void SelectionSizePanel::relayout()
{
    const QFontMetrics metrics(font());
    const QSize fieldSize = m_widthField->sizeHint();
    const int fieldTop = (kHudHeight - fieldSize.height()) / 2;
    int x = kHudPadding;

    // 1. 宽 × 高 px
    m_widthField->setGeometry(QRect(QPoint(x, fieldTop), fieldSize));
    x += fieldSize.width();
    m_timesRect = QRect(x, 0, metrics.horizontalAdvance(kTimesText) + 6, kHudHeight);
    x += m_timesRect.width();
    m_heightField->setGeometry(QRect(QPoint(x, fieldTop), fieldSize));
    x += fieldSize.width();
    m_unitRect = QRect(x, 0, metrics.horizontalAdvance(kUnitText) + 6, kHudHeight);
    x += m_unitRect.width();

    // 2. 分隔线与锁形按钮
    m_separatorRect = QRect(x + 3, 7, 1, kHudHeight - 14);
    x += 7;
    m_lockRect = QRect(x, 3, kLockWidth, kHudHeight - 6);
    x += kLockWidth + 2;

    // 3. 比例触发区按最长名称计算宽度，切换比例时不跳动
    int labelWidth = metrics.horizontalAdvance(MS_TR("Custom"));
    for (const AspectRatioPreset &preset : aspectRatioPresets()) {
        labelWidth = std::max(labelWidth, metrics.horizontalAdvance(markshot::i18n::translate(preset.label)));
    }
    const int ratioWidth = kRatioInnerGap + kGlyphSize + kRatioInnerGap + labelWidth + kRatioInnerGap + kChevronWidth
        + kRatioInnerGap;
    m_ratioRect = QRect(x, 3, ratioWidth, kHudHeight - 6);
    x += ratioWidth + kHudPadding - 2;
    m_contentSize = QSize(x, kHudHeight);
}

SelectionSizePanel::HotZone SelectionSizePanel::zoneAt(const QPoint &position) const
{
    if (m_lockRect.contains(position)) {
        return HotZone::Lock;
    }
    if (m_ratioRect.contains(position)) {
        return HotZone::Ratio;
    }
    return HotZone::None;
}

QString SelectionSizePanel::ratioLabel() const
{
    const int index = presetIndexForRatio(m_ratio);
    if (index < 0) {
        return MS_TR("Custom");
    }
    return markshot::i18n::translate(aspectRatioPresets().at(index).label);
}

void SelectionSizePanel::stepField(SizeField field, int delta)
{
    QSize requested = m_size;
    if (field == SizeField::Width) {
        requested.setWidth(requested.width() + delta);
    } else {
        requested.setHeight(requested.height() + delta);
    }
    emitLinkedSize(requested, field);
}

void SelectionSizePanel::commitField(SizeField field, int value)
{
    QSize requested = m_size;
    if (field == SizeField::Width) {
        requested.setWidth(value);
    } else {
        requested.setHeight(value);
    }
    emitLinkedSize(requested, field);
}

void SelectionSizePanel::emitLinkedSize(QSize requested, SizeField edited)
{
    const QSize next = linkedSize(requested, edited, m_ratio, m_maximum);
    if (next == m_size) {
        // 输入被限制回原值时刷新显示，去掉越界的输入
        setValues(m_size, m_maximum, m_ratio);
        return;
    }
    emit sizeApplied(next);
}

void SelectionSizePanel::toggleLock()
{
    if (m_ratio > 0.0) {
        emit aspectRatioChanged(0.0);
        return;
    }
    // 未锁定时锁定当前选区的宽高比，并保留为自定义比例
    emit aspectRatioChanged(static_cast<qreal>(m_size.width()) / static_cast<qreal>(m_size.height()));
}

void SelectionSizePanel::toggleRatioPopup()
{
    // 1. 首次打开时创建弹层，挂在截图窗口下以便超出 HUD 范围显示
    if (!m_ratioPopup) {
        QWidget *host = parentWidget() ? parentWidget() : this;
        m_ratioPopup = new SelectionRatioPopup(host);
        m_ratioPopup->setHudColors(m_colors);
        connect(m_ratioPopup, &SelectionRatioPopup::ratioSelected, this, &SelectionSizePanel::aspectRatioChanged);
        connect(m_ratioPopup, &SelectionRatioPopup::closed, this, [this] {
            update();
            emit focusReleaseRequested();
        });
    }
    if (m_ratioPopup->isVisible()) {
        m_ratioPopup->dismiss();
        return;
    }

    // 2. 结束字段编辑后在比例触发区下方弹出
    m_widthField->cancelEditing();
    m_heightField->cancelEditing();
    const QWidget *host = m_ratioPopup->parentWidget();
    const QRect anchor(host == this ? m_ratioRect.topLeft() : mapTo(host, m_ratioRect.topLeft()), m_ratioRect.size());
    m_ratioPopup->popup(anchor, m_ratio);
    update();
}

}  // namespace markshot::selection_aspect
