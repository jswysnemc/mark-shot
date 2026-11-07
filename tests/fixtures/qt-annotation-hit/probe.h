// 【标注命中对照】【窗口状态】使用原版标注类型和完整生产几何，不创建可见窗口
#pragma once
#include "shot_window_types.h"
#include "shot_window_ruler_tool.h"
#include "shot_window_ruler_painter.h"
#include "ruler/ruler_geometry.h"
#include "ruler/ruler_painter.h"
#include "line_skeleton_path.h"
#include <QApplication>
#include <QImage>
#include <QLineF>
#include <QTextEdit>
#include <QtMath>
#include <algorithm>
#include <cmath>
#include <limits>

namespace markshot::shot {
using namespace types;
qreal clampedMagnifierScale(qreal);
bool ellipseContainsPoint(QRectF, QPointF, qreal);
}

class ShotWindow {
public:
    using Annotation = markshot::shot::types::Annotation;
    using Tool = markshot::shot::types::Tool;
    using SelectionDrag = markshot::shot::types::SelectionDrag;
    using MagnifierShape = markshot::shot::types::MagnifierShape;
    QImage m_frozenFrame{640, 480, QImage::Format_ARGB32_Premultiplied};
    QRectF m_frozenImageRect{0, 0, 640, 480};
    QRect m_sourceGeometry{0, 0, 640, 480};
    QVector<Annotation> m_annotations;
    Tool m_tool = Tool::Select;
    QTextEdit *m_textEditor = nullptr;
    std::optional<int> m_editingTextAnnotationId;

    /** 【标注命中对照】【对象读取】按id返回持久数组中的只读标注，未知编号返回空指针 */
    const Annotation *annotationById(int id) const { for (const auto &a : m_annotations) if (a.id == id) return &a; return nullptr; }
    /** 【标注命中对照】【窗口投影】按point返回由冻结图像矩形定义的窗口位置 */
    QPointF imageToWidget(QPointF point) const { return m_frozenImageRect.topLeft() + point * annotationSizeScale(true); }
    /** 【标注命中对照】【窗口反解】按point返回同一冻结矩形对应的原图位置 */
    QPointF widgetToImage(QPointF point) const { return (point - m_frozenImageRect.topLeft()) / annotationSizeScale(true); }
    /** 【标注命中对照】【矩形投影】按rect返回完整投影矩形，保留平移和比例 */
    QRectF imageRectToWidget(QRectF value) const { return QRectF(imageToWidget(value.topLeft()), imageToWidget(value.bottomRight())); }
    /** 【标注命中对照】【固定选区】无参数，返回整个原图，用于生产尺子标签布局 */
    QRectF normalizedSelection() const { return {0, 0, 640, 480}; }
    /** 【标注命中对照】【窗口尺寸】无参数，返回探针窗口边界 */
    QRect rect() const { return {0, 0, 640, 480}; }
    /** 【标注命中对照】【未覆盖文字】此探针不采集文字；执行文字测量分支立即终止，参数为生产标注和坐标类型 */
    QRectF textContentRect(const Annotation &, bool) const { qFatal("text is outside this probe's coverage"); }
    qreal annotationSizeScale(bool) const;
    bool annotationSupportsRotation(const Annotation &) const;
    bool annotationSupportsLineControl(const Annotation &) const;
    bool annotationSupportsLineAnchors(const Annotation &) const;
    QRectF annotationUnrotatedBounds(const Annotation &) const;
    QRectF annotationBounds(const Annotation &) const;
    QPointF annotationRotationCenter(const Annotation &, bool) const;
    QPointF annotationRotationHandlePoint(const Annotation &, bool) const;
    QPointF rotatedPoint(QPointF, QPointF, qreal) const;
    QRectF rotatedRectBounds(QRectF, qreal, QPointF) const;
    QRectF magnifierSourceRect(const Annotation &) const;
    QPointF clampedMagnifierCircleCenter(QPointF, qreal) const;
    QRectF magnifierCircleRect(QPointF, qreal) const;
    QRectF clampedMagnifierRect(QRectF) const;
    int lineAnchorPointIndexAt(const Annotation &, QPointF) const;
    SelectionDrag lineAnchorDragAt(const Annotation &, QPointF) const;
    SelectionDrag magnifierDragAt(const Annotation &, QPointF) const;
    SelectionDrag numberDragAt(const Annotation &, QPointF) const;
    SelectionDrag annotationBoundsDragAt(QPointF, QRectF) const;
    SelectionDrag annotationDragAt(QPointF, int) const;
    std::optional<int> annotationAt(QPointF) const;
};
