#include "selection_aspect/selection_size_field.h"

#include "selection_aspect/selection_hud_painter.h"

#include <QEnterEvent>
#include <QFocusEvent>
#include <QFontMetrics>
#include <QIntValidator>
#include <QKeyEvent>
#include <QLineEdit>
#include <QMouseEvent>
#include <QPainter>
#include <QWheelEvent>

#include <algorithm>

namespace markshot::selection_aspect {
namespace {

// 字段内容高度
constexpr int kFieldHeight = hud_style::kHudHeight - 6;

/**
 * 读取滚轮事件的方向；部分平台按住 Shift 时把纵向滚动转成横向，此时改用横向分量。
 * @param event 滚轮事件。
 * @return 向上返回 1，向下返回 -1，无滚动分量返回 0。
 */
int wheelDirection(const QWheelEvent *event)
{
    const QPoint pixel = event->pixelDelta();
    const QPoint raw = pixel.isNull() ? event->angleDelta() : pixel;
    const int delta = raw.y() != 0 ? raw.y() : raw.x();
    if (delta == 0) {
        return 0;
    }
    return delta > 0 ? 1 : -1;
}

}  // namespace

SelectionSizeField::SelectionSizeField(const QString &accessibleName, QWidget *parent)
    : QWidget(parent)
{
    // 1. 平时不抢键盘焦点，避免影响截图窗口快捷键
    setFocusPolicy(Qt::NoFocus);
    setCursor(Qt::IBeamCursor);
    setToolTip(accessibleName);
    setAccessibleName(accessibleName);
    setFont(hud_style::hudFont());

    // 2. 行内输入框只在编辑时显示
    m_editor = new QLineEdit(this);
    m_editor->setFont(font());
    m_editor->setAlignment(Qt::AlignCenter);
    m_editor->setStyleSheet(hud_style::hudEditorStyleSheet(m_colors));
    m_editor->setValidator(new QIntValidator(1, 1, m_editor));
    m_editor->setAccessibleName(accessibleName);
    m_editor->setContextMenuPolicy(Qt::NoContextMenu);
    m_editor->installEventFilter(this);
    m_editor->hide();
}

void SelectionSizeField::setHudColors(const hud_style::HudColors &colors)
{
    m_colors = colors;
    m_editor->setStyleSheet(hud_style::hudEditorStyleSheet(m_colors));
    update();
}

void SelectionSizeField::setValue(int value, int maximum)
{
    m_maximum = std::max(1, maximum);
    m_value = std::clamp(value, 1, m_maximum);
    if (auto *validator = qobject_cast<QIntValidator *>(const_cast<QValidator *>(m_editor->validator()))) {
        validator->setRange(1, m_maximum);
    }
    // 编辑中由滚轮调整的值与实际选区保持同步，例如锁定比例或越界被限制时
    if (isEditing() && m_editor->text() == m_syncedText && m_syncedText != QString::number(m_value)) {
        m_syncedText = QString::number(m_value);
        m_editor->setText(m_syncedText);
        m_editor->selectAll();
    }
    update();
}

int SelectionSizeField::value() const
{
    return m_value;
}

void SelectionSizeField::beginEditing()
{
    if (isEditing()) {
        m_editor->selectAll();
        return;
    }
    m_editing = true;
    m_syncedText = QString::number(m_value);
    m_editor->setGeometry(rect());
    m_editor->setText(m_syncedText);
    m_editor->show();
    m_editor->setFocus(Qt::OtherFocusReason);
    m_editor->selectAll();
    update();
}

void SelectionSizeField::cancelEditing()
{
    if (isEditing()) {
        finishEditing();
    }
}

bool SelectionSizeField::isEditing() const
{
    return m_editing;
}

QSize SelectionSizeField::sizeHint() const
{
    const QFontMetrics metrics(font());
    return QSize(metrics.horizontalAdvance(QStringLiteral("00000")) + 10, kFieldHeight);
}

void SelectionSizeField::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    if (isEditing()) {
        return;
    }
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    // 1. 悬停时绘制浅色底，提示可点击编辑
    if (m_hovered) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(m_colors.hover);
        painter.drawRoundedRect(QRectF(rect()), 5.0, 5.0);
    }
    // 2. 居中绘制像素值
    painter.setPen(m_colors.text);
    painter.setFont(font());
    painter.drawText(rect(), Qt::AlignCenter, QString::number(m_value));
}

void SelectionSizeField::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        beginEditing();
    }
    event->accept();
}

void SelectionSizeField::wheelEvent(QWheelEvent *event)
{
    const int direction = wheelDirection(event);
    if (direction != 0) {
        const int step = event->modifiers().testFlag(Qt::ShiftModifier) ? 10 : 1;
        // 1. 编辑中调整输入框数值并立即应用，未编辑时直接请求调整选区
        if (isEditing()) {
            stepEditing(direction * step);
        } else {
            emit stepRequested(direction * step);
        }
    }
    event->accept();
}

void SelectionSizeField::enterEvent(QEnterEvent *event)
{
    m_hovered = true;
    update();
    QWidget::enterEvent(event);
}

void SelectionSizeField::leaveEvent(QEvent *event)
{
    m_hovered = false;
    update();
    QWidget::leaveEvent(event);
}

void SelectionSizeField::resizeEvent(QResizeEvent *event)
{
    m_editor->setGeometry(rect());
    QWidget::resizeEvent(event);
}

bool SelectionSizeField::eventFilter(QObject *watched, QEvent *event)
{
    if (watched != m_editor) {
        return QWidget::eventFilter(watched, event);
    }

    // 1. 编辑中的确认、取消与切换键归输入框处理，不触发截图窗口快捷键
    if (event->type() == QEvent::ShortcutOverride) {
        const int key = static_cast<QKeyEvent *>(event)->key();
        if (key == Qt::Key_Escape || key == Qt::Key_Return || key == Qt::Key_Enter || key == Qt::Key_Tab
            || key == Qt::Key_Backtab || key == Qt::Key_Up || key == Qt::Key_Down) {
            event->accept();
            return true;
        }
        return false;
    }

    // 2. 按键：Enter 提交、Esc 放弃、Tab 提交后切换、上下键微调
    if (event->type() == QEvent::KeyPress) {
        auto *keyEvent = static_cast<QKeyEvent *>(event);
        switch (keyEvent->key()) {
        case Qt::Key_Return:
        case Qt::Key_Enter:
            commitEditing();
            emit editingClosed();
            return true;
        case Qt::Key_Escape:
            cancelEditing();
            emit editingClosed();
            return true;
        case Qt::Key_Tab:
        case Qt::Key_Backtab:
            commitEditing();
            emit focusNextRequested();
            return true;
        case Qt::Key_Up:
        case Qt::Key_Down: {
            const int step = keyEvent->modifiers().testFlag(Qt::ShiftModifier) ? 10 : 1;
            const int current = m_editor->text().toInt();
            const int next = current + (keyEvent->key() == Qt::Key_Up ? step : -step);
            m_editor->setText(QString::number(std::clamp(next, 1, m_maximum)));
            m_editor->selectAll();
            return true;
        }
        default:
            break;
        }
        return false;
    }

    // 3. 输入框上的滚轮直接按步长调整，不依赖事件向父控件传递
    if (event->type() == QEvent::Wheel) {
        wheelEvent(static_cast<QWheelEvent *>(event));
        return true;
    }

    // 4. 点击其他区域导致失焦时提交输入
    if (event->type() == QEvent::FocusOut && isEditing()) {
        commitEditing();
    }
    return false;
}

void SelectionSizeField::commitEditing()
{
    if (!isEditing()) {
        return;
    }
    bool ok = false;
    const int typed = m_editor->text().trimmed().toInt(&ok);
    // 先退出编辑再发信号，避免接收方同步刷新时读到编辑状态
    finishEditing();
    if (!ok) {
        return;
    }
    const int committed = std::clamp(typed, 1, m_maximum);
    m_value = committed;
    update();
    emit valueCommitted(committed);
}

void SelectionSizeField::stepEditing(int delta)
{
    // 1. 以输入框当前文本为基准，文本无效时回退到实际值
    bool ok = false;
    const int current = m_editor->text().trimmed().toInt(&ok);
    const int next = std::clamp((ok ? current : m_value) + delta, 1, m_maximum);
    // 2. 记录程序写入的文本，接收方回写的联动结果会同步到输入框
    m_syncedText = QString::number(next);
    m_editor->setText(m_syncedText);
    m_editor->selectAll();
    if (next != m_value) {
        m_value = next;
        emit valueCommitted(next);
    }
}

void SelectionSizeField::finishEditing()
{
    // 先清除标志，隐藏输入框引发的失焦事件不会再次提交
    m_editing = false;
    m_syncedText.clear();
    m_editor->hide();
    update();
}

}  // namespace markshot::selection_aspect
