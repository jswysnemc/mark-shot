#include "shot_window_module.h"
#include "shot_window_ruler_tool.h"

using namespace markshot::shot;

namespace {

/// @brief 将画面中的缩放方向匹配到最近的系统方向光标
/// @param degrees 以向右为零度、顺时针为正方向的角度
/// @return 水平、垂直或对角方向的缩放光标
Qt::CursorShape resizeCursorForAngle(qreal degrees)
{
    constexpr std::array<Qt::CursorShape, 4> cursors = {
        Qt::SizeHorCursor, Qt::SizeFDiagCursor, Qt::SizeVerCursor, Qt::SizeBDiagCursor,
    };
    const int index = (qRound(degrees / 45.0) % 4 + 4) % 4;
    return cursors.at(index);
}

/// @brief 将选区或标注操作映射为缩放、抓取或控制点光标
/// @param drag 当前命中的操作类型
/// @param pressed 是否正在按住指针执行操作
/// @param rotationDegrees 单个标注相对画面的旋转角度，选区和分组使用零度
/// @return 与操作方向和拖动状态一致的系统光标
Qt::CursorShape cursorForSelectionDrag(types::SelectionDrag drag, bool pressed, qreal rotationDegrees = 0.0)
{
    using Drag = types::SelectionDrag;
    switch (drag) {
    case Drag::Left:
    case Drag::Right:
    case Drag::MagnifierSourceLeft:
    case Drag::MagnifierSourceRight:
        return resizeCursorForAngle(rotationDegrees);
    case Drag::Top:
    case Drag::Bottom:
    case Drag::MagnifierSourceTop:
    case Drag::MagnifierSourceBottom:
        return resizeCursorForAngle(rotationDegrees + 90.0);
    case Drag::TopLeft:
    case Drag::BottomRight:
    case Drag::MagnifierSourceTopLeft:
    case Drag::MagnifierSourceBottomRight:
        return resizeCursorForAngle(rotationDegrees + 45.0);
    case Drag::TopRight:
    case Drag::BottomLeft:
    case Drag::MagnifierSourceTopRight:
    case Drag::MagnifierSourceBottomLeft:
        return resizeCursorForAngle(rotationDegrees + 135.0);
    case Drag::LineControl:
    case Drag::LineStart:
    case Drag::LineEnd:
    case Drag::NumberTip:
    case Drag::AngleVertex:
    case Drag::AngleFirstArm:
    case Drag::AngleSecondArm:
        return Qt::SizeAllCursor;
    case Drag::Move:
    case Drag::Rotate:
    case Drag::MagnifierSource:
    case Drag::MagnifierLens:
    case Drag::NumberBubble:
        return pressed ? Qt::ClosedHandCursor : Qt::OpenHandCursor;
    case Drag::None:
        return Qt::ArrowCursor;
    }
    return Qt::ArrowCursor;
}

}

void ShotWindow::updateCursor()
{
    updatePointerCursor(mapFromGlobal(QCursor::pos()));
}

void ShotWindow::scheduleUiCursorRefresh(QPointF widgetPoint)
{
    QTimer::singleShot(0, this, [this, widgetPoint] {
        if (isVisible() && !m_dragging && hasUsableSelection()) {
            updatePointerCursor(widgetPoint);
        }
    });
}

bool ShotWindow::selectionPointerVisible() const
{
    return m_selectionPointerDetached && m_startupHoverValid && !m_operationBusy
        && (m_mode == Mode::Selecting
            || (canAdjustSelection() && (m_dragging || m_selectionKeyboardAdjusting)));
}

void ShotWindow::updatePointerCursor(QPointF widgetPoint)
{
    updateAnnotationDeleteButtonHoverIfEnabled(widgetPoint);
    if (m_operationBusy) {
        setCursor(Qt::BusyCursor);
        return;
    }
    // 1. 【截图】【光标状态】进行中的抓取保持闭合手型，界面控件优先使用各自光标
    if (m_toolbarDragging || m_imagePanning) {
        setCursor(Qt::ClosedHandCursor);
        return;
    }
    if (!m_dragging && ((m_mode == Mode::Selecting && m_activeRecordingStopHovered)
        || ((m_tool == Tool::Select || m_tool == Tool::Ruler)
            && selectedAnnotationDeleteButtonRect().contains(widgetPoint)))) {
        setCursor(Qt::PointingHandCursor);
        return;
    }
    if (!m_dragging) {
        if (propertyComboPopupVisible()) {
            setCursor(Qt::ArrowCursor);
            return;
        }
        if (QWidget *widget = childAt(widgetPoint.toPoint())) {
            setCursor(widget->cursor());
            return;
        }
    }

    // 2. 【截图】【精确定位】只有画面中实际绘制的软件指针才接管系统光标
    if (selectionPointerVisible()) {
        setCursor(Qt::BlankCursor);
        return;
    }
    if (!m_dragging && !m_frozenImageRect.contains(widgetPoint)) {
        setCursor(Qt::ArrowCursor);
        return;
    }
    if (m_mode == Mode::Selecting) {
        setCursor(captureCrossCursor(devicePixelRatioF()));
        return;
    }
    if (!m_dragging && m_wheelPreview == WheelPreview::ToolSize && wheelPreviewVisible()) {
        setCursor(Qt::BlankCursor);
        return;
    }

    // 3. 【截图】【选区操作】空闲时重新命中当前位置，结束拖动后立即恢复悬停语义
    const QPointF imagePoint = widgetToImage(widgetPoint);
    if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
        const SelectionDrag drag = m_dragging ? m_selectionDrag : selectionDragAt(imagePoint);
        setCursor(cursorForSelectionDrag(drag, m_dragging));
        return;
    }
    if (m_tool == Tool::Select) {
        if (m_dragging && m_annotationSelectionBoxActive) {
            setCursor(captureCrossCursor(devicePixelRatioF()));
            return;
        }
        SelectionDrag drag = m_dragging ? m_annotationDrag : SelectionDrag::None;
        const QVector<int> selectedIds = selectedAnnotationIds();
        const Annotation *selected = selectedIds.size() == 1 ? annotationById(selectedIds.first()) : nullptr;
        if (!m_dragging) {
            if (selectedIds.size() > 1) {
                drag = selectedAnnotationsDragAt(imagePoint);
            } else if (selected) {
                drag = annotationDragAt(imagePoint, selected->id);
            }
            if (drag == SelectionDrag::None && annotationAt(imagePoint).has_value()) {
                drag = SelectionDrag::Move;
            }
        }
        // 4. 【截图】【标注悬停】命中结果只决定光标，不改写正在进行的拖动状态
        setCursor(cursorForSelectionDrag(drag, m_dragging, selected ? selected->rotationDegrees : 0.0));
        return;
    }
    if (m_tool == Tool::Move) {
        setCursor(Qt::ArrowCursor);
        return;
    }
    // 5. 【截图】【尺子】命中尺子控制点或边框时显示调整光标，其余位置用十字开始新的测量
    if (m_tool == Tool::Ruler) {
        qreal rotation =
            annotationSupportsRotation(m_annotationBeforeDrag) ? m_annotationBeforeDrag.rotationDegrees : 0.0;
        const SelectionDrag drag =
            m_dragging ? m_annotationDrag : ShotWindowRulerTool(*this).hoverDragAt(imagePoint, &rotation);
        setCursor(drag == SelectionDrag::None ? captureCrossCursor(devicePixelRatioF())
                                              : cursorForSelectionDrag(drag, m_dragging, rotation));
        return;
    }

    // 6. 【截图】【标注工具】文字在可输入位置使用插入光标，选区外不能输入时用箭头
    if (m_tool == Tool::Text) {
        setCursor(textToolAcceptsPoint(imagePoint) ? Qt::IBeamCursor : Qt::ArrowCursor);
        return;
    }
    // 7. 【截图】【标注工具】其余绘制工具使用统一十字
    setCursor(captureCrossCursor(devicePixelRatioF()));
}
