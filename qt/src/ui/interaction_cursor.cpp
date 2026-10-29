#include "ui/interaction_cursor.h"

#include <QAbstractButton>
#include <QAbstractItemView>
#include <QApplication>
#include <QComboBox>
#include <QCursor>
#include <QEnterEvent>
#include <QEvent>
#include <QLineEdit>
#include <QMouseEvent>
#include <QPlainTextEdit>
#include <QPointer>
#include <QSlider>
#include <QTabBar>
#include <QTextEdit>
#include <QWidget>

namespace markshot::ui {
namespace {

/// @brief 获取当前事件在控件内的位置
/// @param widget 事件目标控件
/// @param event 鼠标、进入或状态事件
/// @return 控件局部位置
QPoint pointerPosition(QWidget *widget, QEvent *event)
{
    if (event->type() == QEvent::MouseMove) {
        return static_cast<QMouseEvent *>(event)->position().toPoint();
    }
    if (event->type() == QEvent::Enter) {
        return static_cast<QEnterEvent *>(event)->position().toPoint();
    }
    return widget->mapFromGlobal(QCursor::pos());
}

/// @brief 只管理标准控件，画布与窗口拖动由所属业务模块决定光标
class InteractionCursorPolicy final : public QObject {
public:
    /// @brief 创建随应用销毁的控件光标策略
    /// @param parent 当前图形应用
    explicit InteractionCursorPolicy(QObject *parent) : QObject(parent) {}

protected:
    /// @brief 同步可操作、禁用、输入和拖动状态，不拦截原事件
    /// @param watched 接收事件的对象
    /// @param event Qt 控件事件
    /// @return 始终返回 false
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        const QEvent::Type type = event->type();
        if (type != QEvent::Polish && type != QEvent::Show && type != QEvent::EnabledChange
            && type != QEvent::MouseButtonPress && type != QEvent::MouseButtonRelease
            && type != QEvent::MouseMove && type != QEvent::Enter && type != QEvent::Leave
            && type != QEvent::Hide && type != QEvent::UngrabMouse) {
            return false;
        }
        auto *widget = qobject_cast<QWidget *>(watched);
        if (!widget) {
            return false;
        }
        if (widget->property("dragHandle").toBool()) {
            if (!widget->isEnabled()) {
                widget->setCursor(Qt::ArrowCursor);
            } else if (type == QEvent::EnabledChange) {
                widget->setCursor(Qt::OpenHandCursor);
            }
            return false;
        }

        // 1. 【界面】【控件光标】可点击控件使用手型，禁用控件恢复箭头
        if (qobject_cast<QAbstractButton *>(widget) || qobject_cast<QComboBox *>(widget)) {
            widget->setCursor(widget->isEnabled() ? Qt::PointingHandCursor : Qt::ArrowCursor);
        } else if (qobject_cast<QLineEdit *>(widget)) {
            widget->setCursor(widget->isEnabled() ? Qt::IBeamCursor : Qt::ArrowCursor);
        } else if (auto *edit = qobject_cast<QTextEdit *>(widget)) {
            // 内联输入框的内边距也属于输入范围，由 textCursorInPadding 标记后同样显示插入光标
            const bool paddingIsText = edit->property("textCursorInPadding").toBool() && edit->isEnabled();
            widget->setCursor(paddingIsText ? Qt::IBeamCursor : Qt::ArrowCursor);
            edit->viewport()->setCursor(edit->isEnabled() ? Qt::IBeamCursor : Qt::ArrowCursor);
        } else if (auto *edit = qobject_cast<QPlainTextEdit *>(widget)) {
            widget->setCursor(Qt::ArrowCursor);
            edit->viewport()->setCursor(edit->isEnabled() ? Qt::IBeamCursor : Qt::ArrowCursor);
        } else if (auto *slider = qobject_cast<QSlider *>(widget)) {
            // 2. 【界面】【滑块拖动】按下时闭合手型，释放、隐藏或禁用时清理拖动状态
            if (type == QEvent::MouseButtonPress && slider->isEnabled()
                && static_cast<QMouseEvent *>(event)->button() == Qt::LeftButton) {
                m_draggingSlider = slider;
            }
            const bool released = type == QEvent::MouseButtonRelease
                && static_cast<QMouseEvent *>(event)->button() == Qt::LeftButton;
            if (released || type == QEvent::Hide
                || type == QEvent::UngrabMouse || !slider->isEnabled()) {
                if (m_draggingSlider == slider) {
                    m_draggingSlider.clear();
                }
            }
            slider->setCursor(!slider->isEnabled() ? Qt::ArrowCursor
                : m_draggingSlider == slider ? Qt::ClosedHandCursor : Qt::OpenHandCursor);
        } else if (auto *tabs = qobject_cast<QTabBar *>(widget)) {
            tabs->setMouseTracking(true);
            const int index = tabs->tabAt(pointerPosition(tabs, event));
            const bool active = type != QEvent::Leave && tabs->isEnabled()
                && index >= 0 && tabs->isTabEnabled(index);
            tabs->setCursor(active ? Qt::PointingHandCursor : Qt::ArrowCursor);
        }

        // 3. 【界面】【列表光标】仅导航等动作列表显示手型，空白与不可选项仍使用箭头
        auto *view = qobject_cast<QAbstractItemView *>(widget);
        if (!view) {
            view = qobject_cast<QAbstractItemView *>(widget->parentWidget());
        }
        if (view && view->property("actionList").toBool()
            && (widget == view || widget == view->viewport())) {
            view->setMouseTracking(true);
            const QPoint point = widget == view->viewport()
                ? pointerPosition(widget, event) : view->viewport()->mapFromGlobal(QCursor::pos());
            const QModelIndex index = view->indexAt(point);
            const auto flags = index.flags();
            const bool active = type != QEvent::Leave && view->isEnabled() && index.isValid()
                && flags.testFlag(Qt::ItemIsEnabled) && flags.testFlag(Qt::ItemIsSelectable);
            view->viewport()->setCursor(active ? Qt::PointingHandCursor : Qt::ArrowCursor);
        }
        return false;
    }

private:
    QPointer<QSlider> m_draggingSlider;
};

}

void installInteractionCursorPolicy(QApplication *application)
{
    if (!application || application->findChild<QObject *>(QStringLiteral("interactionCursorPolicy"))) {
        return;
    }
    auto *policy = new InteractionCursorPolicy(application);
    policy->setObjectName(QStringLiteral("interactionCursorPolicy"));
    application->installEventFilter(policy);
}

}
