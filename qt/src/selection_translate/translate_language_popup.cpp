#include "selection_translate/translate_language_popup.h"

#include "ui/i18n.h"
#include "ui/svg_icon.h"
#include "ui/theme.h"

#include <QApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPainter>

#include <algorithm>

namespace markshot::selection_translate {

namespace {

/// 弹层内边距
constexpr int kPadding = 4;
/// 标题行高度
constexpr int kTitleHeight = 20;
/// 语言行高度
constexpr int kRowHeight = 26;
/// 当前语言标记的边长
constexpr int kCheckSize = 13;
/// 弹层宽度上下限
constexpr int kMinWidth = 132;
constexpr int kMaxWidth = 240;

}  // namespace

TranslateLanguagePopup::TranslateLanguagePopup(QWidget *parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("translateLanguagePopup"));
    setAttribute(Qt::WA_TranslucentBackground, true);
    setAttribute(Qt::WA_NoSystemBackground, true);
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
    setCursor(Qt::PointingHandCursor);
    setFont(markshot::theme::uiFont(10));
    setAccessibleName(MS_TR("Translate to"));
    hide();
}

void TranslateLanguagePopup::setToolbarPalette(const markshot::ui::ToolbarPalette &palette)
{
    m_palette = palette;
    update();
}

void TranslateLanguagePopup::setLanguages(QVector<markshot::TranslationLanguageOption> options,
                                          const QString &currentValue)
{
    // 1. 当前语言来自配置，可能是用户手填的语言，不在内置列表时补到第一行
    const auto current = std::find_if(options.cbegin(), options.cend(), [&currentValue](const auto &option) {
        return option.value == currentValue;
    });
    if (current == options.cend() && !currentValue.trimmed().isEmpty()) {
        options.prepend({currentValue, currentValue});
    }
    m_options = std::move(options);

    // 2. 键盘选择从当前语言开始，回车即可沿用上次的语言
    m_currentIndex = -1;
    for (int i = 0; i < m_options.size(); ++i) {
        if (m_options.at(i).value == currentValue) {
            m_currentIndex = i;
            break;
        }
    }
    m_activeIndex = std::max(0, m_currentIndex);
    update();
}

void TranslateLanguagePopup::popup(const QRect &geometry)
{
    setGeometry(geometry);
    qApp->installEventFilter(this);
    show();
    raise();
    setFocus(Qt::PopupFocusReason);
    update();
}

void TranslateLanguagePopup::dismiss()
{
    if (!isVisible()) {
        return;
    }
    hide();
    emit closed();
}

QSize TranslateLanguagePopup::sizeHint() const
{
    // 宽度按标题和最长语言名计算，右侧留出当前语言标记
    const QFontMetrics metrics(font());
    int textWidth = metrics.horizontalAdvance(MS_TR("Translate to"));
    for (const markshot::TranslationLanguageOption &option : m_options) {
        textWidth = std::max(textWidth, metrics.horizontalAdvance(option.label));
    }
    const int width = std::clamp(textWidth + kPadding * 2 + 10 + kCheckSize + 18, kMinWidth, kMaxWidth);
    const int rows = static_cast<int>(m_options.size());
    return QSize(width, kPadding * 2 + kTitleHeight + rows * kRowHeight);
}

void TranslateLanguagePopup::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    // 1. 背景与描边沿用工具栏下拉面板的配色
    painter.setPen(QPen(m_palette.popupBorder, 1.0));
    painter.setBrush(m_palette.popupFill);
    painter.drawRoundedRect(QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5), 10.0, 10.0);

    // 2. 标题说明列表用途，使用弱化文字色
    painter.setFont(font());
    painter.setPen(m_palette.mutedText);
    const QRect titleRect(kPadding + 8, kPadding, width() - kPadding * 2 - 16, kTitleHeight);
    painter.drawText(titleRect, Qt::AlignVCenter | Qt::AlignLeft, MS_TR("Translate to"));

    const QIcon check = markshot::ui::makeSvgIcon(QStringLiteral("check"), m_palette.hoverFill);
    for (int i = 0; i < m_options.size(); ++i) {
        const QRect row = rowRect(i);
        // 3. 悬停或键盘选中行绘制浅色底
        if (i == m_activeIndex) {
            painter.setPen(QPen(m_palette.softHoverBorder, 1.0));
            painter.setBrush(m_palette.softHoverFill);
            painter.drawRoundedRect(QRectF(row).adjusted(0.5, 0.5, -0.5, -0.5), 7.0, 7.0);
        }
        // 4. 语言名过长时省略，当前语言在右侧绘制勾选标记
        painter.setPen(m_palette.text);
        const QRect textRect = row.adjusted(8, 0, -(kCheckSize + 14), 0);
        const QString label = painter.fontMetrics().elidedText(m_options.at(i).label, Qt::ElideRight, textRect.width());
        painter.drawText(textRect, Qt::AlignVCenter | Qt::AlignLeft, label);
        if (i == m_currentIndex) {
            const QRect checkRect(row.right() - 8 - kCheckSize, row.top() + (row.height() - kCheckSize) / 2,
                                  kCheckSize, kCheckSize);
            check.paint(&painter, checkRect);
        }
    }
}

void TranslateLanguagePopup::mouseMoveEvent(QMouseEvent *event)
{
    const int index = rowAt(event->position().toPoint());
    if (index >= 0 && index != m_activeIndex) {
        m_activeIndex = index;
        update();
    }
    event->accept();
}

void TranslateLanguagePopup::mousePressEvent(QMouseEvent *event)
{
    event->accept();
}

void TranslateLanguagePopup::mouseReleaseEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        const int index = rowAt(event->position().toPoint());
        if (index >= 0) {
            choose(index);
        }
    }
    event->accept();
}

void TranslateLanguagePopup::keyPressEvent(QKeyEvent *event)
{
    const int count = static_cast<int>(m_options.size());
    switch (event->key()) {
    case Qt::Key_Up:
        if (count > 0) {
            m_activeIndex = (m_activeIndex + count - 1) % count;
            update();
        }
        break;
    case Qt::Key_Down:
    case Qt::Key_Tab:
        if (count > 0) {
            m_activeIndex = (m_activeIndex + 1) % count;
            update();
        }
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

void TranslateLanguagePopup::hideEvent(QHideEvent *event)
{
    qApp->removeEventFilter(this);
    QWidget::hideEvent(event);
}

bool TranslateLanguagePopup::eventFilter(QObject *watched, QEvent *event)
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

    // 2. 只处理控件上的按下事件，弹层自身的事件交给鼠标处理函数
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
    if (QRect(mapTo(window(), QPoint(0, 0)), size()).contains(windowPoint)) {
        return false;
    }
    dismiss();
    return true;
}

QRect TranslateLanguagePopup::rowRect(int index) const
{
    return QRect(kPadding, kPadding + kTitleHeight + index * kRowHeight, width() - kPadding * 2, kRowHeight);
}

int TranslateLanguagePopup::rowAt(const QPoint &position) const
{
    for (int i = 0; i < m_options.size(); ++i) {
        if (rowRect(i).contains(position)) {
            return i;
        }
    }
    return -1;
}

void TranslateLanguagePopup::choose(int index)
{
    if (index < 0 || index >= m_options.size()) {
        return;
    }
    const QString value = m_options.at(index).value;
    dismiss();
    emit languageChosen(value);
}

}  // namespace markshot::selection_translate
