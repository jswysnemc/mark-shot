// 【尺子窗口对照】【状态提供】冻结尺寸、选区、窗口和投影由场景指定，不创建可见窗口
#pragma once
#include "shot_window_types.h"
#define private public
#include "shot_window_ruler_painter.h"
#undef private
#include "ruler/ruler_geometry.h"
#include "ruler/ruler_painter.h"
#include <QApplication>
#include <QImage>
#include <QLineF>
#include <QTextEdit>
#include <QtMath>
#include <algorithm>
#include <cmath>

class ShotWindow {
public:
    using Annotation = markshot::shot::types::Annotation;
    using Tool = markshot::shot::types::Tool;
    QImage m_frozenFrame;
    QRectF m_frozenImageRect;
    QRect m_sourceGeometry;
    QRectF m_selection;
    QRect windowRect;
    QTextEdit *m_textEditor = nullptr;
    std::optional<int> m_editingTextAnnotationId;

    /** 【尺子窗口对照】【窗口投影】按point返回冻结图像投影中的位置；此状态提供器只支持等比平移 */
    QPointF imageToWidget(QPointF point) const { return m_frozenImageRect.topLeft() + point * annotationSizeScale(true); }
    /** 【尺子窗口对照】【窗口反解】按point返回与投影一致的原图位置 */
    QPointF widgetToImage(QPointF point) const { return (point - m_frozenImageRect.topLeft()) / annotationSizeScale(true); }
    /** 【尺子窗口对照】【窗口边界】无参数，返回场景指定的完整窗口边界 */
    QRect rect() const { return windowRect; }
    /** 【尺子窗口对照】【未覆盖放大镜】参数为标注，执行此分支立即失败 */
    QRectF magnifierSourceRect(const Annotation &) const { qFatal("magnifier outside ruler coverage"); }
    /** 【尺子窗口对照】【未覆盖文字】参数为标注和坐标类型，执行此分支立即失败 */
    QRectF textContentRect(const Annotation &, bool) const { qFatal("text outside ruler coverage"); }

    qreal annotationSizeScale(bool) const;
    QRectF normalizedSelection() const;
    QRectF imageRectToWidget(QRectF) const;
    QRectF annotationUnrotatedBounds(const Annotation &) const;
    QPointF annotationRotationCenter(const Annotation &, bool) const;
    QPointF annotationRotationHandlePoint(const Annotation &, bool) const;
    QPointF rotatedPoint(QPointF, QPointF, qreal) const;
};
