#include "text_editor_box_chrome.h"

#include <QEvent>
#include <QMouseEvent>
#include <QPainter>
#include <QPen>
#include <QTextCursor>
#include <QTextEdit>
#include <QVariant>

#include <algorithm>

namespace markshot::shot {

namespace {

/// 编辑器上记录输入框背景色的动态属性名
constexpr char kBoxBackgroundProperty[] = "markShotBoxBackground";
/// 输入框虚线边框颜色，与选中标注的强调色一致
const QColor kBoxOutlineColor(45, 212, 191, 210);

}  // namespace

void setTextEditorBoxBackground(QTextEdit *editor, const QColor &backgroundColor)
{
    if (editor) {
        editor->setProperty(kBoxBackgroundProperty, backgroundColor);
    }
}

void paintTextEditorBox(QPainter &painter, const QTextEdit *editor, qreal scale, bool outlined)
{
    if (!editor || !editor->isVisible()) {
        return;
    }
    const QVariant background = editor->property(kBoxBackgroundProperty);
    if (!background.isValid()) {
        return;
    }

    // 1. 背景按提交后的圆角背景绘制，输入时看到的就是成图效果
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    const QRectF box = QRectF(editor->geometry()).adjusted(0.5, 0.5, -0.5, -0.5);
    const qreal radius = 4.0 * scale;
    const QColor backgroundColor = background.value<QColor>();
    if (backgroundColor.alpha() > 0) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(backgroundColor);
        painter.drawRoundedRect(QRectF(editor->geometry()), radius, radius);
    }
    // 2. 虚线边框只在输入时出现，标出输入范围；已有选中框时不再重复描边
    if (!outlined) {
        painter.restore();
        return;
    }
    QPen outline(kBoxOutlineColor, 1.0, Qt::DashLine);
    outline.setCosmetic(true);
    painter.setPen(outline);
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(box, radius, radius);
    painter.restore();
}

bool handleTextEditorBoxPaddingMouse(QTextEdit *editor, QObject *watched, QEvent *event)
{
    if (!editor || !editor->isVisible() || watched != editor || !event) {
        return false;
    }
    const QEvent::Type type = event->type();
    if (type != QEvent::MouseButtonPress && type != QEvent::MouseButtonRelease
        && type != QEvent::MouseButtonDblClick && type != QEvent::MouseMove) {
        return false;
    }

    // 1. 悬停移动照常交给截图窗口，窗口按指针下的控件同步鼠标形状
    const auto *mouseEvent = static_cast<const QMouseEvent *>(event);
    if (type == QEvent::MouseMove && mouseEvent->buttons() == Qt::NoButton) {
        return false;
    }
    // 2. 按在内边距上时把光标放到最近的文字位置，输入框不会因此提交
    if (type == QEvent::MouseButtonPress && mouseEvent->button() == Qt::LeftButton) {
        const QRect viewport = editor->viewport()->geometry();
        const QPoint inside(std::clamp(mouseEvent->position().toPoint().x(), viewport.left(), viewport.right()),
                            std::clamp(mouseEvent->position().toPoint().y(), viewport.top(), viewport.bottom()));
        editor->setTextCursor(editor->cursorForPosition(inside - viewport.topLeft()));
        editor->setFocus(Qt::MouseFocusReason);
    }
    // 3. 内边距上的其余鼠标事件不再传给截图窗口，避免拖动或选中下方标注
    return true;
}

}  // namespace markshot::shot
