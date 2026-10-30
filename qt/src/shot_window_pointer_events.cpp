#include "shot_window.h"
#include "startup_ruler/ruler_set.h"

#include <QApplication>
#include <QEnterEvent>
#include <QMouseEvent>
#include <QTimer>
#include <QWheelEvent>

bool ShotWindow::event(QEvent *event)
{
    claimStartupToolFromPeersIfEnabled(event);
    const QEvent::Type type = event->type();
    if (type == QEvent::MouseButtonPress || type == QEvent::MouseButtonDblClick) {
        ++m_pointerInteractionSerial;
    }
    if (type == QEvent::MouseMove) {
        const Qt::MouseButtons buttons = static_cast<QMouseEvent *>(event)->buttons();
        // 1. 【标注】【抓取恢复】先处理丢失的释放事件，避免把无按键悬停继续作为拖动
        if ((m_imagePanning && !buttons.testFlag(Qt::MiddleButton))
            || ((m_dragging || m_startupRulerDragging) && !buttons.testFlag(Qt::LeftButton))) {
            cancelPointerInteraction();
        }
    } else if (type == QEvent::Hide || type == QEvent::WindowDeactivate) {
        cancelPointerInteraction();
        clearWheelPreview();
        m_selectionPointerDetached = false;
    } else if (type == QEvent::UngrabMouse) {
        // 2. 【标注】【抓取恢复】正常释放先提交结果；延后清理仅针对仍未结束的同一次抓取
        const quint64 serial = m_pointerInteractionSerial;
        QTimer::singleShot(0, this, [this, serial] {
            if (serial == m_pointerInteractionSerial) {
                cancelPointerInteraction();
                updateCursor();
            }
        });
    }

    const QPointer<ShotWindow> guard(this);
    const bool handled = QWidget::event(event);
    if (!guard) {
        return handled;
    }

    // 3. 【标注】【光标转移】统一在业务事件处理完成后刷新，覆盖各工具的提前返回分支
    switch (type) {
    case QEvent::MouseButtonPress:
    case QEvent::MouseButtonRelease:
    case QEvent::MouseButtonDblClick:
    case QEvent::MouseMove:
        updatePointerCursor(static_cast<QMouseEvent *>(event)->position());
        break;
    case QEvent::Wheel:
        updatePointerCursor(static_cast<QWheelEvent *>(event)->position());
        break;
    case QEvent::Enter:
        updatePointerCursor(static_cast<QEnterEvent *>(event)->position());
        break;
    case QEvent::Leave:
        clearWheelPreview();
        m_selectionPointerDetached = false;
        updatePointerCursor(QPointF(-1, -1));
        updateImageScrollBars();
        update();
        break;
    case QEvent::Hide:
    case QEvent::WindowDeactivate:
        setCursor(Qt::ArrowCursor);
        break;
    case QEvent::KeyPress:
    case QEvent::KeyRelease:
    case QEvent::Show:
    case QEvent::Resize:
    case QEvent::WindowActivate:
        updateCursor();
        break;
    default:
        break;
    }
    return handled;
}

void ShotWindow::cancelPointerInteraction()
{
    if (!m_dragging && !m_imagePanning && !m_toolbarDragging && !m_startupRulerDragging
        && !m_annotationSelectionBoxActive && !m_draft.has_value() && !m_laserDraft.has_value()) {
        return;
    }

    // 1. 【标注】【交互取消】已完成的标注保持原状，撤去未提交草稿及临时选择框
    m_dragging = false;
    m_imagePanning = false;
    m_toolbarDragging = false;
    if (m_startupRulerDragging) {
        markshot::startup_ruler::dropUnmeasuredLastRuler(m_startupRulers);
    }
    m_startupRulerDragging = false;
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationHistoryCaptured = false;
    m_lineSkeletonDragPointIndex = -1;
    m_annotationSelectionBoxActive = false;
    m_annotationSelectionBox = {};
    m_draft.reset();
    m_laserDraft.reset();
    m_selectionKeyboardAdjusting = false;
    m_selectionPointerDetached = false;
    flushInitialSelectionRepaint();

    // 2. 【标注】【交互取消】握柄可持有独立光标，和画布一起恢复
    for (QWidget *widget : findChildren<QWidget *>()) {
        if (widget->property("dragHandle").toBool()) {
            widget->setCursor(widget->isEnabled() ? Qt::OpenHandCursor : Qt::ArrowCursor);
        }
    }
    updateAnnotationPropertyPanel();
    update();
}
