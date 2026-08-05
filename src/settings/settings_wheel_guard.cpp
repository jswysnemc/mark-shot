#include "settings/settings_wheel_guard.h"

#include <QAbstractSlider>
#include <QAbstractSpinBox>
#include <QApplication>
#include <QComboBox>
#include <QCoreApplication>
#include <QEvent>
#include <QScrollArea>
#include <QScrollBar>
#include <QWidget>
#include <QWheelEvent>

namespace markshot::settings {
namespace {

/// @brief 设置对话框滚轮事件过滤器。
///
/// Qt 默认行为下，滚轮事件会送达鼠标下的控件；QComboBox 与 QAbstractSpinBox
/// 即使未获得键盘焦点也会响应滚轮并修改内容（选中项/数值），用户在设置页
/// 上下滚动翻页时极易误改配置。本过滤器把这类"未聚焦"控件的滚轮事件换算
/// 为对最近的 QScrollArea 滚动条的滚动，让页面按用户意图翻页；找不到滚动
/// 区域时直接吞掉事件，保证控件的值绝不会被悬停滚动篡改。控件聚焦时仍
/// 保留滚轮调整值的能力。
///
/// 滚动换算与 Qt 原生行为逐项对齐（QAbstractSliderPrivate::scrollByDelta）：
/// - 触控板像素增量直接滚动；鼠标角度增量换算为行数；
/// - Ctrl / Shift 修饰下按整页滚动；
/// - 高分辨率滚轮（±30/±60 等非 120 整倍数）按分数累积跨事件，避免
///   被整数除法截断丢失滚动量（Qt 的 offset_accumulated 语义）。
class WheelGuard final : public QObject {
public:
    /**
     * 创建并安装设置窗口滚轮事件过滤器。
     * @param dialog 需要防止滚轮误改值的设置窗口。
     */
    explicit WheelGuard(QWidget *dialog)
        : QObject(dialog)
        , m_dialog(dialog)
    {
        QApplication::instance()->installEventFilter(this);
    }

    /**
     * 从应用程序卸载事件过滤器。
     */
    ~WheelGuard() override
    {
        if (QCoreApplication *app = QCoreApplication::instance()) {
            app->removeEventFilter(this);
        }
    }

    /**
     * 拦截设置窗口内未聚焦数值控件的滚轮事件。
     * @param watched 原始事件目标。
     * @param event 待处理事件。
     * @return 已接管滚轮事件时返回 true。
     */
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        if (event->type() != QEvent::Wheel || !m_dialog) {
            return QObject::eventFilter(watched, event);
        }

        auto *widget = qobject_cast<QWidget *>(watched);
        if (!widget || !isInDialogWindow(widget)) {
            return QObject::eventFilter(watched, event);
        }

        QWidget *control = guardedControl(widget);
        if (!control || controlHasFocus(control)) {
            return QObject::eventFilter(watched, event);
        }

        auto *wheelEvent = static_cast<QWheelEvent *>(event);
        // 1. 手势开始或结束时清空跨事件分数余量
        const Qt::ScrollPhase phase = wheelEvent->phase();
        if (phase == Qt::ScrollBegin || phase == Qt::ScrollEnd) {
            m_verticalRemainder = 0.0;
            m_horizontalRemainder = 0.0;
        }
        // 2. 将事件换算为外层滚动区域的滚动，找不到区域时直接吞掉事件
        if (!redirectToScrollArea(control, wheelEvent)) {
            event->accept();
        }
        return true;
    }

private:
    /// @brief 判断控件是否属于设置对话框所在顶层窗口。
    /// @param widget 事件目标控件。
    /// @return 属于对话框时返回 true。
    bool isInDialogWindow(QWidget *widget) const
    {
        QWidget *window = widget->window();
        return window && m_dialog && window == m_dialog->window();
    }

    /// @brief 从事件目标向上查找应受防护的控件。
    /// @param widget 事件目标控件。
    /// @return 找到的下拉框/数值框/滑块，未找到时返回空指针。
    static QWidget *guardedControl(QWidget *widget)
    {
        for (QWidget *current = widget; current; current = current->parentWidget()) {
            if (qobject_cast<QAbstractSpinBox *>(current) || qobject_cast<QComboBox *>(current)
                || qobject_cast<QAbstractSlider *>(current)) {
                return current;
            }
            // 到达滚动区域仍未命中，说明悬停在普通控件上，无需防护
            if (qobject_cast<QScrollArea *>(current)) {
                return nullptr;
            }
        }
        return nullptr;
    }

    /// @brief 判断控件或其子控件是否持有键盘焦点。
    /// @param control 下拉框/数值框。
    /// @return 持有焦点时返回 true。
    static bool controlHasFocus(QWidget *control)
    {
        const QWidget *focus = QApplication::focusWidget();
        return focus && (control == focus || control->isAncestorOf(focus));
    }

    /// @brief 把滚轮事件换算为对滚动条的滚动，让设置页照常翻页。
    ///
    /// 直接在过滤器内用 QApplication::sendEvent 重新派发事件会在 Qt 的
    /// 手势过滤器中重入（QGestureManager::filterEventThroughContexts 无
    /// 重入保护，Qt 6.11 实测会段错误）。因此这里改为直接调整外层
    /// QScrollArea 的滚动条：行为与 QAbstractSliderPrivate::scrollByDelta
    /// 对齐（像素/角度增量 + 分数累积 + Ctrl/Shift 整页），且不会再次进入
    /// 事件过滤链。
    /// @param control 下拉框/数值框。
    /// @param event 原始滚轮事件。
    /// @return 找到滚动区域并完成滚动时返回 true。
    bool redirectToScrollArea(QWidget *control, const QWheelEvent *event)
    {
        for (QWidget *current = control; current; current = current->parentWidget()) {
            if (auto *area = qobject_cast<QScrollArea *>(current)) {
                // 1. 跨滚动区域时清空余量，避免残留小数造成跳变
                if (area != m_lastArea) {
                    m_verticalRemainder = 0.0;
                    m_horizontalRemainder = 0.0;
                    m_lastArea = area;
                }
                const QPoint pixelDelta = event->pixelDelta();
                const QPoint angleDelta = event->angleDelta();
                const Qt::KeyboardModifiers modifiers = event->modifiers();
                scrollAreaByDelta(area->verticalScrollBar(),
                                  pixelDelta.y(),
                                  angleDelta.y(),
                                  modifiers,
                                  &m_verticalRemainder);
                scrollAreaByDelta(area->horizontalScrollBar(),
                                  pixelDelta.x(),
                                  angleDelta.x(),
                                  modifiers,
                                  &m_horizontalRemainder);
                return true;
            }
        }
        return false;
    }

    /// @brief 按像素/角度增量滚动一条滚动条（与 Qt 原生行为对齐）。
    /// @param bar 需要滚动的滚动条。
    /// @param pixelDelta 触控板像素增量。
    /// @param angleDelta 滚轮角度增量（每格 120）。
    /// @param modifiers 事件修饰键（Ctrl/Shift 时整页滚动）。
    /// @param accumulator 跨事件分数余量累积器。
    static void scrollAreaByDelta(QScrollBar *bar,
                                  int pixelDelta,
                                  int angleDelta,
                                  Qt::KeyboardModifiers modifiers,
                                  qreal *accumulator)
    {
        if (!bar || bar->maximum() == 0) {
            return;
        }
        if (accumulator == nullptr) {
            return;
        }

        // 1. 优先使用角度增量，仅在缺少角度增量时回退像素增量
        // Qt 滚动条忽略 inverted 标记，平台已经在增量中体现自然滚动方向
        if (angleDelta != 0) {
            const qreal offset = static_cast<qreal>(angleDelta) / 120.0;
            int stepsToScroll = 0;

            // 2. 根据修饰键和系统配置选择整页滚动或行滚动
            const bool pageMode = modifiers.testFlag(Qt::ControlModifier)
                || modifiers.testFlag(Qt::ShiftModifier) || QApplication::wheelScrollLines() < 0;
            if (pageMode) {
                // int() 向零截断，并将结果限制在整页步长内
                stepsToScroll =
                    qBound(-bar->pageStep(), static_cast<int>(offset * bar->pageStep()), bar->pageStep());
                *accumulator = 0.0;
            } else {
                // 行滚动跨事件累积分数，避免高分辨率滚轮增量被截断
                const qreal stepsToScrollF =
                    static_cast<qreal>(QApplication::wheelScrollLines()) * offset * bar->singleStep();
                if (*accumulator != 0.0 && (offset / *accumulator) < 0.0) {
                    // 滚动方向反转时丢弃残留余量
                    *accumulator = 0.0;
                }
                *accumulator += stepsToScrollF;
                // int() 向零截断并限制在整页步长内，余量仅保留小数部分
                stepsToScroll = qBound(-bar->pageStep(), static_cast<int>(*accumulator), bar->pageStep());
                *accumulator -= static_cast<int>(*accumulator);
                if (stepsToScroll == 0) {
                    // 累计不足一行时保留余量，滚动边界除外
                    const qreal effectiveOffset =
                        bar->invertedControls() ? -*accumulator : *accumulator;
                    if (effectiveOffset > 0.0 && bar->value() < bar->maximum()) {
                        return;
                    }
                    if (effectiveOffset < 0.0 && bar->value() > bar->minimum()) {
                        return;
                    }
                    *accumulator = 0.0;
                    return;
                }
            }

            // 3. 将 invertedControls 应用于最终步长并更新滚动位置
            if (bar->invertedControls()) {
                stepsToScroll = -stepsToScroll;
            }
            const int previousValue = bar->value();
            bar->setValue(bar->value() + stepsToScroll);
            // 到达滚动边界时丢弃残留余量
            if (bar->value() == previousValue) {
                *accumulator = 0.0;
            }
            return;
        }

        if (pixelDelta != 0) {
            bar->setValue(bar->value() - pixelDelta);
            *accumulator = 0.0;
        }
    }

    /// @brief 当前处理滚轮事件的滚动区域（用于跨区域清空余量）。
    QScrollArea *m_lastArea = nullptr;
    /// @brief 垂直滚动条跨事件分数余量。
    qreal m_verticalRemainder = 0.0;
    /// @brief 水平滚动条跨事件分数余量。
    qreal m_horizontalRemainder = 0.0;
    QWidget *m_dialog = nullptr;
};

}  // namespace

QObject *installSettingsWheelGuard(QWidget *dialog)
{
    return new WheelGuard(dialog);
}

}  // namespace markshot::settings
